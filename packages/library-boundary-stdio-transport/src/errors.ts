export type LibraryBoundaryStdioTransportErrorCode =
  | "duplicateRequestId"
  | "executeAfterClose"
  | "malformedStdout"
  | "processExit"
  | "spawnFailure"
  | "stdinWriteFailure"
  | "transportError"
  | "unknownRequestId";

export class LibraryBoundaryStdioTransportError extends Error {
  readonly code: LibraryBoundaryStdioTransportErrorCode;
  readonly requestId: string | null;

  constructor(
    code: LibraryBoundaryStdioTransportErrorCode,
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
