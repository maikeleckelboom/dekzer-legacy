import { fileURLToPath } from "node:url";

import type {
  CommandOutcome,
  CommandRequest,
  ProtocolError
} from "@dekzer/library-boundary-contract";

import {
  createStdioCommandEnvelope,
  LibraryBoundaryStdioProcessExitError,
  LibraryBoundaryStdioTransport,
  LibraryBoundaryStdioTransportError
} from "../src/index.js";

const fixtureServerPath = fileURLToPath(
  new URL("./stdioFixtureServer.js", import.meta.url)
);

function createPlaylistRequest(displayName: string): CommandRequest {
  return {
    type: "playlistWrite",
    payload: {
      type: "createPlaylist",
      payload: { displayName }
    }
  };
}

function createTransport(
  requestIdFactory = sequentialRequestIdFactory()
): LibraryBoundaryStdioTransport {
  return new LibraryBoundaryStdioTransport({
    serverBinaryPath: process.execPath,
    serverArgs: [fixtureServerPath],
    userDataPath: process.cwd(),
    environment: "development",
    requestIdFactory
  });
}

function sequentialRequestIdFactory(): () => string {
  let next = 0;
  return () => `request-${++next}`;
}

async function validatesRequestEnvelopeCreation(): Promise<void> {
  const request = createPlaylistRequest("Envelope");
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

async function validatesSuccessResponseResolution(): Promise<void> {
  const transport = createTransport();
  try {
    const outcome = await transport.execute(createPlaylistRequest("Success"));

    equal(outcome.type, "success", "success response resolves execute");
    equal(
      successPlaylistId(outcome),
      "1",
      "success outcome payload is preserved"
    );
  } finally {
    await transport.close();
  }
}

async function validatesProtocolErrorOutcomeIsPreserved(): Promise<void> {
  const transport = createTransport();
  try {
    const outcome = await transport.execute(
      createPlaylistRequest("protocol-error")
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

async function validatesTransportErrorRejects(): Promise<void> {
  const transport = createTransport();
  try {
    const error = await rejects(
      () => transport.execute(createPlaylistRequest("transport-error")),
      LibraryBoundaryStdioTransportError,
      "transportError envelope rejects execute"
    );

    equal(error.code, "transportError", "typed stdio error code is exposed");
    equal(error.requestId, "request-1", "transport error carries requestId");
  } finally {
    await transport.close();
  }
}

async function validatesMalformedStdoutDoesNotCrash(): Promise<void> {
  const transport = createTransport();
  try {
    const error = await rejects(
      () => transport.execute(createPlaylistRequest("malformed")),
      LibraryBoundaryStdioTransportError,
      "malformed stdout rejects the in-flight request"
    );
    equal(error.code, "malformedStdout", "malformed stdout has typed error");

    const outcome = await transport.execute(createPlaylistRequest("Success"));
    equal(outcome.type, "success", "transport continues after malformed stdout");
  } finally {
    await transport.close();
  }
}

async function validatesBlankStdoutLinesAreIgnored(): Promise<void> {
  const transport = createTransport();
  try {
    const outcome = await transport.execute(createPlaylistRequest("blank"));

    equal(successPlaylistId(outcome), "10", "blank stdout line is ignored");
  } finally {
    await transport.close();
  }
}

async function validatesStderrDiagnosticsAreForwarded(): Promise<void> {
  const diagnostics: string[] = [];
  const transport = new LibraryBoundaryStdioTransport({
    serverBinaryPath: process.execPath,
    serverArgs: [fixtureServerPath],
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
    const outcome = await transport.execute(createPlaylistRequest("stderr"));

    equal(successPlaylistId(outcome), "11", "stderr does not become protocol");
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
    () => transport.execute(createPlaylistRequest("after-close")),
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
      () => transport.execute(createPlaylistRequest("exit-pending")),
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
    const first = transport.execute(createPlaylistRequest("concurrent-first"));
    const second = transport.execute(createPlaylistRequest("concurrent-second"));

    equal(successPlaylistId(await first), "1", "first request gets first reply");
    equal(
      successPlaylistId(await second),
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
    const delayed = transport.execute(createPlaylistRequest("delayed"));
    const error = await rejects(
      () => transport.execute(createPlaylistRequest("duplicate")),
      LibraryBoundaryStdioTransportError,
      "duplicate generated requestId rejects"
    );

    equal(error.code, "duplicateRequestId", "duplicate requestId code is stable");
    equal(successPlaylistId(await delayed), "12", "first request still resolves");
  } finally {
    await transport.close();
  }
}

async function validatesConcurrentWritesUseCompleteJsonLines(): Promise<void> {
  const transport = createTransport();
  try {
    const outcomes = await Promise.all(
      Array.from({ length: 12 }, (_, index) =>
        transport.execute(createPlaylistRequest(`bulk-${index + 1}`))
      )
    );

    deepEqual(
      outcomes.map(successPlaylistId),
      Array.from({ length: 12 }, (_, index) => String(index + 1)),
      "concurrent writes arrive as complete routed JSON lines"
    );
  } finally {
    await transport.close();
  }
}

function successPlaylistId(outcome: CommandOutcome): string {
  must(outcome.type === "success", "expected success outcome");
  const reply = outcome.payload.reply;
  must(reply.type === "playlistWrite", "expected playlistWrite reply");
  must(reply.payload.type === "createPlaylist", "expected createPlaylist reply");
  return reply.payload.payload.playlistId;
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
await validatesSuccessResponseResolution();
await validatesProtocolErrorOutcomeIsPreserved();
await validatesTransportErrorRejects();
await validatesMalformedStdoutDoesNotCrash();
await validatesBlankStdoutLinesAreIgnored();
await validatesStderrDiagnosticsAreForwarded();
await validatesCloseRejectsLaterExecute();
await validatesProcessExitRejectsPending();
await validatesConcurrentResponsesRouteByRequestId();
await validatesDuplicateGeneratedRequestIdRejectsBeforeWrite();
await validatesConcurrentWritesUseCompleteJsonLines();
