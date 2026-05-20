import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { randomUUID } from "node:crypto";
import { createInterface, type Interface } from "node:readline";

import type {
  CommandOutcome,
  CommandRequest
} from "@dekzer/library-boundary-contract";
import type { LibraryBoundaryTransport } from "@dekzer/library-boundary-client";

import {
  createStdioCommandEnvelope,
  parseStdioResponseEnvelope,
  serializeStdioCommandEnvelope,
  type StdioRemoteTransportErrorEnvelope
} from "./envelope.js";
import {
  LibraryBoundaryStdioProcessExitError,
  LibraryBoundaryStdioRemoteTransportError,
  LibraryBoundaryStdioTransportError
} from "./errors.js";

export type LibraryBoundaryStdioEnvironment =
  | "development"
  | "production";

export type LibraryBoundaryStdioDiagnostic = {
  readonly stream: "stderr" | "stdout";
  readonly line: string;
};

export type LibraryBoundaryStdioTransportOptions = {
  readonly serverBinaryPath: string;
  readonly userDataPath: string;
  readonly environment: LibraryBoundaryStdioEnvironment;
  readonly serverArgs?: readonly string[];
  readonly cwd?: string;
  readonly diagnostics?: (diagnostic: LibraryBoundaryStdioDiagnostic) => void;
  readonly requestIdFactory?: () => string;
};

type TransportState = "ready" | "closing" | "closed" | "failed";
type TransportLifecycleState = "starting" | TransportState;

type PendingRequest = {
  readonly requestId: string;
  readonly resolve: (outcome: CommandOutcome) => void;
  readonly reject: (reason: unknown) => void;
};

export class LibraryBoundaryStdioTransport
  implements LibraryBoundaryTransport
{
  readonly ready: Promise<void>;
  readonly #child: ChildProcessWithoutNullStreams;
  readonly #diagnostics?: (diagnostic: LibraryBoundaryStdioDiagnostic) => void;
  readonly #pending = new Map<string, PendingRequest>();
  readonly #requestIdFactory: () => string;
  readonly #stdout: Interface;
  readonly #stderr: Interface;
  readonly #pendingEmptyResolvers = new Set<() => void>();
  #readyReject!: (reason: unknown) => void;
  #readyResolve!: () => void;
  #readySettled = false;
  #closePromise: Promise<void> | null = null;
  #exitPromise: Promise<void>;
  #state: TransportLifecycleState = "starting";
  #writeQueue: Promise<void> = Promise.resolve();

  constructor(options: LibraryBoundaryStdioTransportOptions) {
    this.#diagnostics = options.diagnostics;
    this.#requestIdFactory = options.requestIdFactory ?? randomUUID;
    this.#child = spawn(
      options.serverBinaryPath,
      [
        ...(options.serverArgs ?? []),
        "--user-data-path",
        options.userDataPath,
        "--env",
        options.environment
      ],
      {
        cwd: options.cwd,
        stdio: ["pipe", "pipe", "pipe"]
      }
    );

    this.#stdout = createInterface({
      input: this.#child.stdout,
      crlfDelay: Infinity
    });
    this.#stderr = createInterface({
      input: this.#child.stderr,
      crlfDelay: Infinity
    });
    this.ready = new Promise((resolve, reject) => {
      this.#readyResolve = resolve;
      this.#readyReject = reject;
    });
    void this.ready.catch(() => undefined);
    this.#exitPromise = this.#createExitPromise();

    this.#stdout.on("line", (line) => this.#handleStdoutLine(line));
    this.#stderr.on("line", (line) => {
      this.#diagnostics?.({ stream: "stderr", line });
    });
  }

  async execute(request: CommandRequest): Promise<CommandOutcome> {
    if (this.#state !== "starting" && this.#state !== "ready") {
      throw new LibraryBoundaryStdioTransportError(
        "executeAfterClose",
        `library boundary stdio transport cannot execute while ${this.#state}`
      );
    }

    await this.ready;

    if (this.#state !== "ready") {
      throw new LibraryBoundaryStdioTransportError(
        "executeAfterClose",
        `library boundary stdio transport cannot execute while ${this.#state}`
      );
    }

    const requestId = this.#requestIdFactory();
    if (this.#pending.has(requestId)) {
      throw new LibraryBoundaryStdioTransportError(
        "duplicateRequestId",
        `duplicate stdio requestId generated: ${requestId}`,
        { requestId }
      );
    }

    const envelope = createStdioCommandEnvelope(requestId, request);
    const line = `${serializeStdioCommandEnvelope(envelope)}\n`;
    const pending = this.#createPendingRequest(requestId);
    this.#pending.set(requestId, pending);

    this.#writeQueue = this.#writeQueue
      .catch(() => undefined)
      .then(() => this.#writeLine(line));
    void this.#writeQueue.catch((cause) => {
      this.#rejectPending(
        requestId,
        new LibraryBoundaryStdioTransportError(
          "stdinWriteFailure",
          "failed to write request to library boundary stdio process",
          { cause, requestId }
        )
      );
    });

    return await pendingPromise(pending);
  }

  close(): Promise<void> {
    if (this.#closePromise !== null) {
      return this.#closePromise;
    }

    if (this.#state === "closed") {
      return Promise.resolve();
    }

    const wasStarting = this.#state === "starting";
    this.#state = "closing";
    if (wasStarting) {
      this.#rejectReady(
        new LibraryBoundaryStdioTransportError(
          "closedBeforeReady",
          "library boundary stdio transport closed before the Rust server became ready"
        )
      );
    }
    this.#closePromise = (async () => {
      await this.#waitForPendingRequests();
      if (!this.#child.stdin.destroyed) {
        this.#child.stdin.end();
      }
      await this.#exitPromise;
      this.#stdout.close();
      this.#stderr.close();
      if (this.#state !== "failed") {
        this.#state = "closed";
      }
    })();

    return this.#closePromise;
  }

  #createPendingRequest(requestId: string): PendingRequest {
    let resolve: ((outcome: CommandOutcome) => void) | null = null;
    let reject: ((reason: unknown) => void) | null = null;
    const promise = new Promise<CommandOutcome>((innerResolve, innerReject) => {
      resolve = innerResolve;
      reject = innerReject;
    });
    const pending = {
      requestId,
      resolve: (outcome: CommandOutcome) => {
        if (resolve === null) {
          throw new Error("pending request resolve was not initialized");
        }
        resolve(outcome);
      },
      reject: (reason: unknown) => {
        if (reject === null) {
          throw new Error("pending request reject was not initialized");
        }
        reject(reason);
      }
    };

    pendingPromises.set(pending, promise);
    return pending;
  }

  #handleStdoutLine(line: string): void {
    if (line.trim().length === 0) {
      return;
    }

    let envelope;
    try {
      envelope = parseStdioResponseEnvelope(line);
    } catch (cause) {
      this.#diagnostics?.({ stream: "stdout", line });
      this.#rejectForMalformedStdout(cause);
      return;
    }

    if (envelope.type === "ready") {
      this.#handleReadyEnvelope();
      return;
    }

    if (this.#state === "starting") {
      this.#diagnostics?.({
        stream: "stdout",
        line: "stdio process wrote a non-ready envelope before readiness"
      });
      this.#fail(
        new LibraryBoundaryStdioTransportError(
          "malformedStdout",
          "library boundary stdio process wrote non-ready stdout before readiness"
        )
      );
      return;
    }

    if (envelope.type === "commandOutcome") {
      const pending = this.#pending.get(envelope.requestId);
      if (pending === undefined) {
        this.#rejectForUnknownRequestId(envelope.requestId);
        return;
      }

      this.#pending.delete(envelope.requestId);
      pending.resolve(envelope.outcome);
      this.#notifyPendingEmpty();
      return;
    }

    this.#handleTransportErrorEnvelope(envelope);
  }

  #handleReadyEnvelope(): void {
    if (this.#state !== "starting") {
      this.#diagnostics?.({
        stream: "stdout",
        line: "duplicate stdio ready envelope"
      });
      this.#fail(
        new LibraryBoundaryStdioTransportError(
          "malformedStdout",
          "library boundary stdio process wrote duplicate ready envelope"
        )
      );
      return;
    }

    this.#state = "ready";
    this.#resolveReady();
  }

  #handleTransportErrorEnvelope(
    envelope: StdioRemoteTransportErrorEnvelope
  ): void {
    const error = new LibraryBoundaryStdioRemoteTransportError(
      envelope.error.code,
      envelope.error.message,
      { requestId: envelope.requestId }
    );

    if (envelope.requestId !== null) {
      if (this.#rejectPending(envelope.requestId, error)) {
        return;
      }

      this.#rejectForUnknownRequestId(envelope.requestId);
      return;
    }

    this.#rejectAllPending(error);
  }

  #rejectForUnknownRequestId(requestId: string): void {
    this.#diagnostics?.({
      stream: "stdout",
      line: `unknown stdio response requestId: ${requestId}`
    });
    this.#rejectAllPending(
      new LibraryBoundaryStdioTransportError(
        "unknownRequestId",
        `library boundary stdio process wrote a response for unknown requestId: ${requestId}`,
        { requestId }
      )
    );
  }

  #rejectForMalformedStdout(cause: unknown): void {
    const error = new LibraryBoundaryStdioTransportError(
      "malformedStdout",
      "library boundary stdio process wrote malformed stdout",
      { cause }
    );

    if (this.#state === "starting") {
      this.#fail(error);
      return;
    }

    if (this.#pending.size === 1) {
      const [requestId] = this.#pending.keys();
      this.#rejectPending(requestId, error);
      return;
    }

    this.#rejectAllPending(error);
  }

  #writeLine(line: string): Promise<void> {
    if (this.#state !== "ready") {
      throw new LibraryBoundaryStdioTransportError(
        "executeAfterClose",
        `library boundary stdio transport cannot write while ${this.#state}`
      );
    }

    return new Promise((resolve, reject) => {
      this.#child.stdin.write(line, "utf8", (error) => {
        if (error) {
          reject(error);
          return;
        }

        resolve();
      });
    });
  }

  #createExitPromise(): Promise<void> {
    return new Promise((resolve) => {
      let settled = false;
      const settle = (): void => {
        if (settled) {
          return;
        }

        settled = true;
        resolve();
      };

      this.#child.once("error", (cause) => {
        this.#fail(
          new LibraryBoundaryStdioTransportError(
            "spawnFailure",
            "failed to spawn library boundary stdio process",
            { cause }
          )
        );
        settle();
      });
      this.#child.once("exit", (exitCode, signal) => {
        const exitError = new LibraryBoundaryStdioProcessExitError(
          exitCode,
          signal
        );
        this.#rejectReady(exitError);
        if (this.#pending.size > 0) {
          this.#rejectAllPending(exitError);
        }

        if (this.#state !== "closing") {
          this.#state = "failed";
        }

        settle();
      });
    });
  }

  #fail(error: LibraryBoundaryStdioTransportError): void {
    this.#state = "failed";
    this.#rejectReady(error);
    this.#rejectAllPending(error);
  }

  #resolveReady(): void {
    if (this.#readySettled) {
      return;
    }

    this.#readySettled = true;
    this.#readyResolve();
  }

  #rejectReady(error: Error): void {
    if (this.#readySettled) {
      return;
    }

    this.#readySettled = true;
    this.#readyReject(error);
  }

  #rejectPending(requestId: string, error: Error): boolean {
    const pending = this.#pending.get(requestId);
    if (pending === undefined) {
      return false;
    }

    this.#pending.delete(requestId);
    pending.reject(error);
    this.#notifyPendingEmpty();
    return true;
  }

  #rejectAllPending(error: Error): void {
    for (const [requestId, pending] of this.#pending) {
      this.#pending.delete(requestId);
      pending.reject(error);
    }
    this.#notifyPendingEmpty();
  }

  #waitForPendingRequests(): Promise<void> {
    if (this.#pending.size === 0) {
      return Promise.resolve();
    }

    return new Promise((resolve) => {
      this.#pendingEmptyResolvers.add(resolve);
    });
  }

  #notifyPendingEmpty(): void {
    if (this.#pending.size !== 0) {
      return;
    }

    for (const resolve of this.#pendingEmptyResolvers) {
      resolve();
    }
    this.#pendingEmptyResolvers.clear();
  }
}

const pendingPromises = new WeakMap<PendingRequest, Promise<CommandOutcome>>();

function pendingPromise(pending: PendingRequest): Promise<CommandOutcome> {
  const promise = pendingPromises.get(pending);
  if (promise === undefined) {
    throw new Error("pending request promise was not initialized");
  }

  return promise;
}

export function createLibraryBoundaryStdioTransport(
  options: LibraryBoundaryStdioTransportOptions
): LibraryBoundaryStdioTransport {
  return new LibraryBoundaryStdioTransport(options);
}

export const spawnLibraryBoundaryStdioTransport =
  createLibraryBoundaryStdioTransport;
