import { fileURLToPath } from "node:url";

import {
  createLibraryBoundaryClient,
  LibraryBoundaryProtocolError
} from "@dekzer/library-boundary-client";
import type {
  CommandOutcome,
  CommandRequest,
  ProtocolError
} from "@dekzer/library-boundary-contract";

import {
  createStdioCommandEnvelope,
  isLibraryBoundaryStdioReadyEnvelope,
  libraryBoundaryStdioReadyEnvelopeType,
  libraryBoundaryStdioReadyServer,
  libraryBoundaryStdioRemoteErrorCodes,
  LibraryBoundaryStdioRemoteTransportError,
  LibraryBoundaryStdioProcessExitError,
  LibraryBoundaryStdioTransport,
  LibraryBoundaryStdioTransportError,
  parseStdioResponseEnvelope
} from "../src/index.js";

const fixtureServerPath = fileURLToPath(
  new URL("./stdioFixtureServer.js", import.meta.url)
);

function registerLocalRootRequest(requestedPath: string): CommandRequest {
  return {
    type: "libraryRoots",
    payload: {
      type: "registerLocalRoot",
      payload: { requestedPath }
    }
  };
}

function createTransport(
  requestIdFactory = sequentialRequestIdFactory(),
  startupMode = "ready",
  closeTimeoutMs?: number
): LibraryBoundaryStdioTransport {
  return new LibraryBoundaryStdioTransport({
    serverBinaryPath: process.execPath,
    serverArgs: [fixtureServerPath, startupMode],
    userDataPath: process.cwd(),
    environment: "development",
    closeTimeoutMs,
    requestIdFactory
  });
}

function sequentialRequestIdFactory(): () => string {
  let next = 0;
  return () => `request-${++next}`;
}

async function validatesRequestEnvelopeCreation(): Promise<void> {
  const request = registerLocalRootRequest("Envelope");
  const envelope = createStdioCommandEnvelope("request-1", request);

  deepEqual(
    envelope,
    {
      type: "command",
      requestId: "request-1",
      request
    },
    "request envelope keeps requestId outside the generated CommandRequest"
  );
}

async function validatesGeneratedReadyEnvelopeContract(): Promise<void> {
  equal(
    libraryBoundaryStdioReadyEnvelopeType,
    "ready",
    "generated ready envelope type is stable"
  );
  equal(
    libraryBoundaryStdioReadyServer,
    "libraryBoundaryStdio",
    "generated ready server is stable"
  );

  const ready = {
    type: "ready",
    server: "libraryBoundaryStdio"
  };
  equal(
    isLibraryBoundaryStdioReadyEnvelope(ready),
    true,
    "generated ready type guard accepts valid ready"
  );

  deepEqual(
    parseStdioResponseEnvelope(JSON.stringify(ready)),
    ready,
    "parser returns valid ready envelope"
  );
}

async function validatesMalformedReadyRejects(): Promise<void> {
  const error = throws(
    () => parseStdioResponseEnvelope(JSON.stringify({
      type: "ready",
      server: "wrongServer"
    })),
    "malformed ready is rejected"
  );

  equal(
    error.message,
    "ready response envelope has invalid stdio ready shape",
    "malformed ready failure is explicit"
  );

  const readyWithRequestId = throws(
    () => parseStdioResponseEnvelope(JSON.stringify({
      type: "ready",
      server: "libraryBoundaryStdio",
      requestId: "not-allowed"
    })),
    "ready with requestId is rejected"
  );

  equal(
    readyWithRequestId.message,
    "ready response envelope has invalid stdio ready shape",
    "ready must not carry requestId"
  );
}

async function validatesReadyResolvesOnlyAfterReadyEnvelope(): Promise<void> {
  const transport = createTransport(sequentialRequestIdFactory(), "delayed-ready");
  try {
    let isReady = false;
    void transport.ready.then(() => {
      isReady = true;
    });

    await sleep(15);
    equal(isReady, false, "ready remains pending before ready envelope");
    await transport.ready;
    equal(isReady, true, "ready resolves after ready envelope");
  } finally {
    await transport.close();
  }
}

async function validatesExecuteBeforeReadyWaitsForReadiness(): Promise<void> {
  const transport = createTransport(sequentialRequestIdFactory(), "delayed-ready");
  try {
    const outcomePromise = transport.execute(registerLocalRootRequest("Success"));

    await sleep(15);
    await transport.ready;
    const outcome = await outcomePromise;

    equal(
      successRegisteredRootId(outcome),
      "1",
      "execute before ready waits and then sends the command"
    );
  } finally {
    await transport.close();
  }
}

async function validatesProcessExitBeforeReadyRejectsReady(): Promise<void> {
  const transport = createTransport(
    sequentialRequestIdFactory(),
    "exit-before-ready"
  );
  try {
    const error = await rejects(
      () => transport.ready,
      LibraryBoundaryStdioProcessExitError,
      "process exit before ready rejects ready"
    );

    equal(error.exitCode, 8, "pre-ready exit code is exposed");
  } finally {
    await transport.close();
  }
}

async function validatesMalformedStdoutBeforeReadyRejectsReady(): Promise<void> {
  const transport = createTransport(
    sequentialRequestIdFactory(),
    "malformed-before-ready"
  );
  try {
    const error = await rejects(
      () => transport.ready,
      LibraryBoundaryStdioTransportError,
      "malformed stdout before ready rejects ready"
    );

    equal(error.code, "malformedStdout", "pre-ready malformed stdout is local");
  } finally {
    await transport.close();
  }
}

async function validatesSuccessResponseResolution(): Promise<void> {
  const transport = createTransport();
  try {
    const outcome = await transport.execute(registerLocalRootRequest("Success"));

    equal(outcome.type, "success", "success response resolves execute");
    equal(
      successRegisteredRootId(outcome),
      "1",
      "success outcome payload is preserved"
    );
  } finally {
    await transport.close();
  }
}

async function validatesLocalBrowseEntryPointSnapshotResponse(): Promise<void> {
  const transport = createTransport();
  try {
    const client = createLibraryBoundaryClient(transport);
    const reply = await client.readLocalBrowseEntryPoints(null);

    equal(reply.status, "complete", "local browse entry point read status is preserved");
    equal(
      reply.entries[0]?.identity.entryPointKind,
      "music",
      "local browse entry point fixture routes through stdio transport"
    );
  } finally {
    await transport.close();
  }
}

async function validatesLocalBrowseItemSnapshotResponse(): Promise<void> {
  const transport = createTransport();
  try {
    const client = createLibraryBoundaryClient(transport);
    const reply = await client.readLocalBrowseItems({
      entryPointKind: "music",
      resolvedRootPath: "C:\\Users\\DJ\\Music",
      resolvedParentPath: "C:\\Users\\DJ\\Music",
      offset: 0,
      limit: 50,
      profile: "audio"
    });

    equal(reply.status, "complete", "local browse item read status is preserved");
    equal(
      reply.items[0]?.itemKind,
      "mediaFile",
      "local browse item fixture routes through stdio transport"
    );
    deepEqual(
      reply.items[0]?.availableOperations,
      [
        {
          kind: "requestSourceAdmission",
          requestKind: "parentDirectory",
          resolvedPath: "C:\\Users\\DJ\\Music"
        }
      ],
      "local browse item admission operation is preserved"
    );
  } finally {
    await transport.close();
  }
}

async function validatesProtocolErrorOutcomeIsPreserved(): Promise<void> {
  const transport = createTransport();
  try {
    const outcome = await transport.execute(
      registerLocalRootRequest("protocol-error")
    );
    const expected: ProtocolError = {
      type: "invalidRequest",
      payload: { detail: "fixture protocol error" }
    };

    deepEqual(
      outcome,
      {
        type: "error",
        payload: { error: expected }
      } satisfies CommandOutcome,
      "protocol error outcome remains a CommandOutcome"
    );
  } finally {
    await transport.close();
  }
}

async function validatesClientTurnsProtocolErrorOutcomeIntoClientError(): Promise<void> {
  const transport = createTransport();
  try {
    const client = createLibraryBoundaryClient(transport);
    const error = await rejects(
      () => client.registerLocalRoot({ requestedPath: "protocol-error" }),
      LibraryBoundaryProtocolError,
      "client converts protocol error outcome into LibraryBoundaryProtocolError"
    );
    const expected: ProtocolError = {
      type: "invalidRequest",
      payload: { detail: "fixture protocol error" }
    };

    deepEqual(
      error.protocolError,
      expected,
      "client layer preserves protocol error payload"
    );
  } finally {
    await transport.close();
  }
}

async function validatesRemoteTransportErrorCodeParsing(): Promise<void> {
  deepEqual(
    [...libraryBoundaryStdioRemoteErrorCodes],
    [
      "invalidFrame",
      "invalidEnvelope",
      "invalidRequestId",
      "invalidCommandRequest",
      "serverPanic",
      "stdinReadFailure"
    ],
    "generated remote code list is the exact Rust stdout envelope code set"
  );

  for (const code of libraryBoundaryStdioRemoteErrorCodes) {
    const envelope = parseStdioResponseEnvelope(JSON.stringify({
      type: "transportError",
      requestId: "request-1",
      error: {
        code,
        message: `message for ${code}`
      }
    }));

    must(
      envelope.type === "transportError",
      "expected parsed transport error envelope"
    );
    equal(envelope.error.code, code, `${code} parses as a remote error code`);
  }
}

async function validatesUnknownRemoteTransportErrorCodePolicy(): Promise<void> {
  const error = throws(
    () => parseStdioResponseEnvelope(JSON.stringify({
      type: "transportError",
      requestId: "request-1",
      error: {
        code: "futureRemoteCode",
        message: "unknown future code"
      }
    })),
    "unknown remote transport error code is rejected by the parser"
  );

  equal(
    error.message,
    "transportError response error.code is not a known Rust stdio remote error code",
    "unknown remote code policy is explicit"
  );
}

async function validatesTransportErrorRejects(): Promise<void> {
  const transport = createTransport();
  try {
    const error = await rejects(
      () => transport.execute(registerLocalRootRequest("transport-error")),
      LibraryBoundaryStdioRemoteTransportError,
      "remote transportError envelope rejects execute"
    );

    equal(error.kind, "remoteTransportError", "remote error kind is explicit");
    equal(error.source, "rustStdioServer", "remote error source is explicit");
    equal(error.remoteCode, "invalidEnvelope", "remote code is preserved");
    equal(
      error.remoteMessage,
      "fixture transport error",
      "remote message is preserved"
    );
    equal(error.requestId, "request-1", "transport error carries requestId");
  } finally {
    await transport.close();
  }
}

async function validatesUnknownRemoteTransportErrorCodeRejectsAsMalformedStdout(): Promise<void> {
  const transport = createTransport();
  try {
    const error = await rejects(
      () => transport.execute(registerLocalRootRequest("unknown-remote-transport-code")),
      LibraryBoundaryStdioTransportError,
      "unknown remote code rejects as local malformed stdout"
    );

    equal(error.code, "malformedStdout", "unknown remote code is a local parse failure");
  } finally {
    await transport.close();
  }
}

async function validatesMalformedStdoutDoesNotCrash(): Promise<void> {
  const transport = createTransport();
  try {
    const error = await rejects(
      () => transport.execute(registerLocalRootRequest("malformed")),
      LibraryBoundaryStdioTransportError,
      "malformed stdout rejects the in-flight request"
    );
    equal(error.code, "malformedStdout", "malformed stdout has typed error");

    const outcome = await transport.execute(registerLocalRootRequest("Success"));
    equal(outcome.type, "success", "transport continues after malformed stdout");
  } finally {
    await transport.close();
  }
}

async function validatesDuplicateReadyAfterReadyIsLifecycleFailure(): Promise<void> {
  const transport = createTransport();
  try {
    await transport.ready;
    const error = await rejects(
      () => transport.execute(registerLocalRootRequest("duplicate-ready")),
      LibraryBoundaryStdioTransportError,
      "duplicate ready rejects the in-flight request"
    );

    equal(error.code, "malformedStdout", "duplicate ready is malformed stdout");

    const laterError = await rejects(
      () => transport.execute(registerLocalRootRequest("Success")),
      LibraryBoundaryStdioTransportError,
      "duplicate ready fails the transport for future execute calls"
    );
    equal(laterError.code, "executeAfterClose", "failed transport rejects later execute");
  } finally {
    await transport.close();
  }
}

async function validatesUnknownResponseRequestIdRejectsPending(): Promise<void> {
  const transport = createTransport();
  try {
    const error = await rejects(
      () => transport.execute(registerLocalRootRequest("unknown-response-request-id")),
      LibraryBoundaryStdioTransportError,
      "unknown response requestId rejects pending execute"
    );

    equal(error.code, "unknownRequestId", "unknown response requestId is local");
    equal(error.requestId, "missing-request", "unknown response requestId is exposed");
  } finally {
    await transport.close();
  }
}

async function validatesBlankStdoutLinesAreIgnored(): Promise<void> {
  const transport = createTransport();
  try {
    const outcome = await transport.execute(registerLocalRootRequest("blank"));

    equal(successRegisteredRootId(outcome), "10", "blank stdout line is ignored");
  } finally {
    await transport.close();
  }
}

async function validatesStderrDiagnosticsAreForwarded(): Promise<void> {
  const diagnostics: string[] = [];
  const transport = new LibraryBoundaryStdioTransport({
    serverBinaryPath: process.execPath,
    serverArgs: [fixtureServerPath, "ready"],
    userDataPath: process.cwd(),
    environment: "development",
    requestIdFactory: sequentialRequestIdFactory(),
    diagnostics: (diagnostic) => {
      if (diagnostic.stream === "stderr") {
        diagnostics.push(diagnostic.line);
      }
    }
  });

  try {
    const outcome = await transport.execute(registerLocalRootRequest("stderr"));

    equal(successRegisteredRootId(outcome), "11", "stderr does not become protocol");
    deepEqual(
      diagnostics,
      ["fixture diagnostic"],
      "stderr line is forwarded as diagnostics"
    );
  } finally {
    await transport.close();
  }
}

async function validatesCloseRejectsLaterExecute(): Promise<void> {
  const transport = createTransport();
  await transport.close();

  const error = await rejects(
    () => transport.execute(registerLocalRootRequest("after-close")),
    LibraryBoundaryStdioTransportError,
    "execute after close rejects"
  );

  equal(error.code, "executeAfterClose", "execute-after-close code is stable");
  await transport.close();
}

async function validatesProcessExitRejectsPending(): Promise<void> {
  const transport = createTransport();
  try {
    const error = await rejects(
      () => transport.execute(registerLocalRootRequest("exit-pending")),
      LibraryBoundaryStdioProcessExitError,
      "unexpected process exit rejects pending execute"
    );

    equal(error.exitCode, 7, "exit code is exposed");
  } finally {
    await transport.close();
  }
}

async function validatesConcurrentResponsesRouteByRequestId(): Promise<void> {
  const transport = createTransport();
  try {
    const first = transport.execute(registerLocalRootRequest("concurrent-first"));
    const second = transport.execute(registerLocalRootRequest("concurrent-second"));

    equal(successRegisteredRootId(await first), "1", "first request gets first reply");
    equal(
      successRegisteredRootId(await second),
      "2",
      "second request gets second reply even when response arrives first"
    );
  } finally {
    await transport.close();
  }
}

async function validatesDuplicateGeneratedRequestIdRejectsBeforeWrite(): Promise<void> {
  const transport = createTransport(() => "duplicate-request");
  try {
    const delayed = transport.execute(registerLocalRootRequest("delayed"));
    const error = await rejects(
      () => transport.execute(registerLocalRootRequest("duplicate")),
      LibraryBoundaryStdioTransportError,
      "duplicate generated requestId rejects"
    );

    equal(error.code, "duplicateRequestId", "duplicate requestId code is stable");
    equal(successRegisteredRootId(await delayed), "12", "first request still resolves");
  } finally {
    await transport.close();
  }
}

async function validatesConcurrentWritesUseCompleteJsonLines(): Promise<void> {
  const transport = createTransport();
  try {
    const outcomes = await Promise.all(
      Array.from({ length: 12 }, (_, index) =>
        transport.execute(registerLocalRootRequest(`bulk-${index + 1}`))
      )
    );

    deepEqual(
      outcomes.map(successRegisteredRootId),
      Array.from({ length: 12 }, (_, index) => String(index + 1)),
      "concurrent writes arrive as complete routed JSON lines"
    );
  } finally {
    await transport.close();
  }
}

async function validatesCloseTimeoutRejectsPendingAndAwaitsChildExit(): Promise<void> {
  const transport = createTransport(sequentialRequestIdFactory(), "ready", 50);
  await transport.ready;

  const pending = transport.execute(registerLocalRootRequest("never-respond"));
  void pending.catch(() => undefined);
  await sleep(15);

  const closeError = await rejects(
    () => transport.close(),
    LibraryBoundaryStdioTransportError,
    "close timeout rejects instead of hanging on a pending request"
  );
  equal(closeError.code, "closeTimeout", "close timeout code is explicit");

  const pendingError = await rejects(
    () => pending,
    LibraryBoundaryStdioTransportError,
    "close timeout rejects the pending request"
  );
  equal(pendingError.code, "closeTimeout", "pending request sees close timeout");
}

function successRegisteredRootId(outcome: CommandOutcome): string {
  must(outcome.type === "success", "expected success outcome");
  const reply = outcome.payload.reply;
  must(reply.type === "libraryRoots", "expected libraryRoots reply");
  must(reply.payload.type === "registerLocalRoot", "expected registerLocalRoot reply");
  const payload = reply.payload.payload;
  must(payload.type === "registered", "expected registered root reply");
  return payload.payload.rootId;
}

function must(condition: unknown, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

function equal<T>(actual: T, expected: T, message: string): void {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${String(expected)}, got ${String(actual)}`);
  }
}

function deepEqual(actual: unknown, expected: unknown, message: string): void {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`${message}: expected ${expectedJson}, got ${actualJson}`);
  }
}

function sleep(milliseconds: number): Promise<void> {
  return new Promise((resolve) => {
    globalThis.setTimeout(resolve, milliseconds);
  });
}

function throws(action: () => unknown, message: string): Error {
  try {
    action();
  } catch (error) {
    if (error instanceof Error) {
      return error;
    }

    throw new Error(`${message}: threw unexpected value ${String(error)}`);
  }

  throw new Error(`${message}: action did not throw`);
}

async function rejects<ErrorType extends Error>(
  action: () => Promise<unknown>,
  errorClass: new (...args: never[]) => ErrorType,
  message: string
): Promise<ErrorType> {
  try {
    await action();
  } catch (error) {
    if (error instanceof errorClass) {
      return error;
    }

    throw new Error(`${message}: rejected with unexpected error ${String(error)}`);
  }

  throw new Error(`${message}: promise resolved`);
}

await validatesRequestEnvelopeCreation();
await validatesGeneratedReadyEnvelopeContract();
await validatesMalformedReadyRejects();
await validatesReadyResolvesOnlyAfterReadyEnvelope();
await validatesExecuteBeforeReadyWaitsForReadiness();
await validatesProcessExitBeforeReadyRejectsReady();
await validatesMalformedStdoutBeforeReadyRejectsReady();
await validatesSuccessResponseResolution();
await validatesLocalBrowseEntryPointSnapshotResponse();
await validatesLocalBrowseItemSnapshotResponse();
await validatesProtocolErrorOutcomeIsPreserved();
await validatesClientTurnsProtocolErrorOutcomeIntoClientError();
await validatesRemoteTransportErrorCodeParsing();
await validatesUnknownRemoteTransportErrorCodePolicy();
await validatesTransportErrorRejects();
await validatesUnknownRemoteTransportErrorCodeRejectsAsMalformedStdout();
await validatesMalformedStdoutDoesNotCrash();
await validatesDuplicateReadyAfterReadyIsLifecycleFailure();
await validatesUnknownResponseRequestIdRejectsPending();
await validatesBlankStdoutLinesAreIgnored();
await validatesStderrDiagnosticsAreForwarded();
await validatesCloseRejectsLaterExecute();
await validatesProcessExitRejectsPending();
await validatesConcurrentResponsesRouteByRequestId();
await validatesDuplicateGeneratedRequestIdRejectsBeforeWrite();
await validatesConcurrentWritesUseCompleteJsonLines();
await validatesCloseTimeoutRejectsPendingAndAwaitsChildExit();
