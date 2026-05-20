import type {
  LibraryBoundaryStdioRemoteErrorCode
} from "./generated/stdioEnvelope.js";

export type {
  LibraryBoundaryStdioRemoteErrorCode
} from "./generated/stdioEnvelope.js";

export type LibraryBoundaryStdioLocalErrorCode =
  | "duplicateRequestId"
  | "executeAfterClose"
  | "malformedStdout"
  | "processExit"
  | "spawnFailure"
  | "stdinWriteFailure"
  | "unknownRequestId";

export class LibraryBoundaryStdioTransportError extends Error {
  readonly kind = "localTransportError";
  readonly source = "typescriptStdioTransport";
  readonly code: LibraryBoundaryStdioLocalErrorCode;
  readonly requestId: string | null;

  constructor(
    code: LibraryBoundaryStdioLocalErrorCode,
    message: string,
    options?: {
      readonly requestId?: string | null;
      readonly cause?: unknown;
    }
  ) {
    super(message, { cause: options?.cause });
    this.name = new.target.name;
    this.code = code;
    this.requestId = options?.requestId ?? null;
  }
}

export class LibraryBoundaryStdioRemoteTransportError extends Error {
  readonly kind = "remoteTransportError";
  readonly source = "rustStdioServer";
  readonly remoteCode: LibraryBoundaryStdioRemoteErrorCode;
  readonly remoteMessage: string;
  readonly requestId: string | null;

  constructor(
    remoteCode: LibraryBoundaryStdioRemoteErrorCode,
    remoteMessage: string,
    options?: {
      readonly requestId?: string | null;
      readonly cause?: unknown;
    }
  ) {
    super(remoteMessage, { cause: options?.cause });
    this.name = new.target.name;
    this.remoteCode = remoteCode;
    this.remoteMessage = remoteMessage;
    this.requestId = options?.requestId ?? null;
  }
}

export class LibraryBoundaryStdioProcessExitError extends LibraryBoundaryStdioTransportError {
  readonly exitCode: number | null;
  readonly signal: NodeJS.Signals | null;

  constructor(exitCode: number | null, signal: NodeJS.Signals | null) {
    super(
      "processExit",
      `library boundary stdio process exited unexpectedly: code=${exitCode ?? "null"} signal=${signal ?? "null"}`
    );
    this.exitCode = exitCode;
    this.signal = signal;
  }
}
