import type {
  CommandOutcome,
  CommandReply,
  CommandRequest
} from "@dekzer/library-boundary-contract";

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from "./errors.js";
import type { LibraryBoundaryCommandExecutor } from "./transport.js";

type FamilyReply<Family extends CommandReply["type"]> = Extract<
  CommandReply,
  { type: Family }
>["payload"];

type ReplyVariant<Reply> = Reply extends { type: infer Variant extends string }
  ? Variant
  : never;

type ReplyPayload<Reply, Variant extends string> = Reply extends {
  type: Variant;
  payload: infer Payload;
}
  ? Payload
  : never;

export type LibraryBoundaryCommandReplyVariant<
  Family extends CommandReply["type"]
> = ReplyVariant<FamilyReply<Family>>;

export type LibraryBoundaryCommandReplyPayload<
  Family extends CommandReply["type"],
  Variant extends string
> = ReplyPayload<FamilyReply<Family>, Variant>;

export async function executeLibraryBoundaryCommand<
  Family extends CommandReply["type"],
  Variant extends LibraryBoundaryCommandReplyVariant<Family>
>(
  executor: LibraryBoundaryCommandExecutor,
  request: CommandRequest,
  expectedFamily: Family,
  expectedVariant: Variant
): Promise<LibraryBoundaryCommandReplyPayload<Family, Variant>> {
  const reply = unwrapCommandOutcome(
    await executeCommand(executor, request)
  );

  if (reply.type !== expectedFamily) {
    throw new LibraryBoundaryReplyMismatchError({
      expectedFamily,
      expectedVariant,
      actualFamily: reply.type,
      actualVariant: reply.payload.type
    });
  }

  const familyReply = reply.payload as FamilyReply<Family>;
  if (familyReply.type !== expectedVariant) {
    throw new LibraryBoundaryReplyMismatchError({
      expectedFamily,
      expectedVariant,
      actualFamily: reply.type,
      actualVariant: familyReply.type
    });
  }

  return (
    familyReply as unknown as {
      payload: LibraryBoundaryCommandReplyPayload<Family, Variant>;
    }
  ).payload;
}

async function executeCommand(
  executor: LibraryBoundaryCommandExecutor,
  request: CommandRequest
): Promise<CommandOutcome> {
  try {
    return await executor.execute(request);
  } catch (cause) {
    throw new LibraryBoundaryTransportError(cause);
  }
}

function unwrapCommandOutcome(outcome: CommandOutcome): CommandReply {
  if (outcome.type === "error") {
    throw new LibraryBoundaryProtocolError(outcome.payload.error);
  }

  return outcome.payload.reply;
}
