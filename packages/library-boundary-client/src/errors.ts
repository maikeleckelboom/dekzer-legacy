import type {
  CommandReply,
  ProtocolError
} from "@dekzer/library-boundary-contract";

export class LibraryBoundaryClientError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = new.target.name;
  }
}

export class LibraryBoundaryProtocolError extends LibraryBoundaryClientError {
  readonly protocolError: ProtocolError;

  constructor(protocolError: ProtocolError) {
    super(`Library boundary protocol error: ${protocolError.type}`);
    this.protocolError = protocolError;
  }
}

export type LibraryBoundaryReplyMismatch = {
  readonly expectedFamily: CommandReply["type"];
  readonly expectedVariant: string;
  readonly actualFamily: CommandReply["type"] | string;
  readonly actualVariant: string;
};

export class LibraryBoundaryReplyMismatchError extends LibraryBoundaryClientError {
  readonly expectedFamily: CommandReply["type"];
  readonly expectedVariant: string;
  readonly actualFamily: CommandReply["type"] | string;
  readonly actualVariant: string;

  constructor(mismatch: LibraryBoundaryReplyMismatch) {
    super(
      `Library boundary reply mismatch: expected ${mismatch.expectedFamily}/${mismatch.expectedVariant}, received ${mismatch.actualFamily}/${mismatch.actualVariant}`
    );
    this.expectedFamily = mismatch.expectedFamily;
    this.expectedVariant = mismatch.expectedVariant;
    this.actualFamily = mismatch.actualFamily;
    this.actualVariant = mismatch.actualVariant;
  }
}

export class LibraryBoundaryTransportError extends LibraryBoundaryClientError {
  constructor(cause: unknown) {
    super("Library boundary transport failed while executing a command", {
      cause
    });
  }
}

export type LibraryBoundarySessionState =
  | "ready"
  | "closing"
  | "closed"
  | "failed";

export class LibraryBoundarySessionStateError extends LibraryBoundaryClientError {
  readonly operation: string;
  readonly state: LibraryBoundarySessionState;

  constructor(operation: string, state: LibraryBoundarySessionState) {
    super(`Library boundary session cannot ${operation} while ${state}`);
    this.operation = operation;
    this.state = state;
  }
}

export class LibraryBoundarySessionDrainInProgressError extends LibraryBoundaryClientError {
  constructor() {
    super("Library boundary session already has an event drain in progress");
  }
}

export class LibraryBoundarySessionInvalidMaxEventsError extends LibraryBoundaryClientError {
  readonly maxEvents: number;

  constructor(maxEvents: number) {
    super(
      "Library boundary session pumpEvents maxEvents must be a positive safe integer"
    );
    this.maxEvents = maxEvents;
  }
}
