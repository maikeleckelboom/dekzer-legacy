import type {
  CommandOutcome,
  CommandRequest
} from "@dekzer/library-boundary-contract";

export interface LibraryBoundaryCommandExecutor {
  execute(request: CommandRequest): Promise<CommandOutcome>;
}

export interface LibraryBoundaryTransport
  extends LibraryBoundaryCommandExecutor {
  close?(): Promise<void> | void;
}
