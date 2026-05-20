import type {
  CommandOutcome,
  CommandRequest
} from "@dekzer/library-boundary-contract";

import {
  isLibraryBoundaryStdioRemoteErrorCode,
  type LibraryBoundaryStdioRemoteErrorCode
} from "./generated/stdioEnvelope.js";

export type {
  LibraryBoundaryStdioRemoteErrorCode
} from "./generated/stdioEnvelope.js";

export type StdioCommandEnvelope = {
  readonly type: "command";
  readonly requestId: string;
  readonly request: CommandRequest;
};

export type StdioCommandOutcomeEnvelope = {
  readonly type: "commandOutcome";
  readonly requestId: string;
  readonly outcome: CommandOutcome;
};

export type StdioRemoteTransportErrorEnvelope = {
  readonly type: "transportError";
  readonly requestId: string | null;
  readonly error: {
    readonly code: LibraryBoundaryStdioRemoteErrorCode;
    readonly message: string;
  };
};

export type StdioResponseEnvelope =
  | StdioCommandOutcomeEnvelope
  | StdioRemoteTransportErrorEnvelope;

export function createStdioCommandEnvelope(
  requestId: string,
  request: CommandRequest
): StdioCommandEnvelope {
  return {
    type: "command",
    requestId,
    request
  };
}

export function serializeStdioCommandEnvelope(
  envelope: StdioCommandEnvelope
): string {
  return JSON.stringify(envelope);
}

export function parseStdioResponseEnvelope(line: string): StdioResponseEnvelope {
  const parsed = JSON.parse(line) as unknown;
  if (!isRecord(parsed)) {
    throw new Error("stdio response envelope must be a JSON object");
  }

  if (parsed.type === "commandOutcome") {
    if (!isNonEmptyString(parsed.requestId)) {
      throw new Error("commandOutcome response requestId must be a non-empty string");
    }
    if (!isCommandOutcome(parsed.outcome)) {
      throw new Error("commandOutcome response outcome is invalid");
    }

    return {
      type: "commandOutcome",
      requestId: parsed.requestId,
      outcome: parsed.outcome
    };
  }

  if (parsed.type === "transportError") {
    if (
      parsed.requestId !== null &&
      !isNonEmptyString(parsed.requestId)
    ) {
      throw new Error(
        "transportError response requestId must be a non-empty string or null"
      );
    }
    if (!isRecord(parsed.error)) {
      throw new Error("transportError response error must be an object");
    }
    if (!isNonEmptyString(parsed.error.code)) {
      throw new Error("transportError response error.code must be a string");
    }
    if (!isLibraryBoundaryStdioRemoteErrorCode(parsed.error.code)) {
      throw new Error(
        "transportError response error.code is not a known Rust stdio remote error code"
      );
    }
    if (typeof parsed.error.message !== "string") {
      throw new Error("transportError response error.message must be a string");
    }

    return {
      type: "transportError",
      requestId: parsed.requestId,
      error: {
        code: parsed.error.code,
        message: parsed.error.message
      }
    };
  }

  throw new Error("stdio response envelope type is invalid");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isCommandOutcome(value: unknown): value is CommandOutcome {
  if (!isRecord(value)) {
    return false;
  }

  if (value.type === "success") {
    return isRecord(value.payload) && isRecord(value.payload.reply);
  }

  if (value.type === "error") {
    return isRecord(value.payload) && isRecord(value.payload.error);
  }

  return false;
}
