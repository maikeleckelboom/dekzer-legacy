import type {
  LibraryBoundaryEvent,
  MaintainedSnapshotEvent,
  MaintainedSnapshotInvalidation,
  MaintainedSnapshotRevision,
  MaintainedSnapshotScope,
  MaintainedSnapshotScopeRevision
} from "@dekzer/library-boundary-contract";

import { LibraryBoundaryClient } from "./client.js";
import {
  LibraryBoundarySessionDrainInProgressError,
  LibraryBoundarySessionInvalidMaxEventsError,
  LibraryBoundarySessionStateError,
  type LibraryBoundarySessionState
} from "./errors.js";
import type { LibraryBoundaryTransport } from "./transport.js";

export const LIBRARY_BOUNDARY_LISTENER_ERROR_RETENTION_LIMIT = 32;

export type LibraryBoundaryInvalidationBatch = {
  readonly invalidations: readonly MaintainedSnapshotInvalidation[];
  readonly changedScopes: readonly MaintainedSnapshotScope[];
  readonly changedRevisions: readonly MaintainedSnapshotScopeRevision[];
  readonly lastSeenRevisions: ReadonlyMap<
    MaintainedSnapshotScope,
    MaintainedSnapshotRevision
  >;
};

export type LibraryBoundaryInvalidationListener = (
  batch: LibraryBoundaryInvalidationBatch
) => void;

export type LibraryBoundaryListenerErrorReport = {
  readonly error: unknown;
  readonly batch: LibraryBoundaryInvalidationBatch;
  readonly listener: LibraryBoundaryInvalidationListener;
};

export type LibraryBoundaryListenerErrorDrain = {
  readonly reports: readonly LibraryBoundaryListenerErrorReport[];
  readonly droppedReportCount: number;
  readonly retainedReportCount: number;
  readonly retentionLimit: number;
};

export type LibraryBoundaryListenerErrorHandler = (
  report: LibraryBoundaryListenerErrorReport
) => void;

export type LibraryBoundarySessionOptions = {
  readonly onListenerError?: LibraryBoundaryListenerErrorHandler;
};

export type LibraryBoundaryInvalidationSubscription = {
  unsubscribe(): void;
};

export class LibraryBoundarySession {
  readonly client: LibraryBoundaryClient;
  readonly #transport: LibraryBoundaryTransport;

  private readonly onListenerError: LibraryBoundaryListenerErrorHandler | null;
  private readonly listeners = new Set<LibraryBoundaryInvalidationListener>();
  private readonly listenerErrors: LibraryBoundaryListenerErrorReport[] = [];
  private droppedListenerErrorCount = 0;
  private readonly lastSeenRevisionByScope = new Map<
    MaintainedSnapshotScope,
    MaintainedSnapshotRevision
  >();
  private currentState: LibraryBoundarySessionState = "ready";
  private inFlightDrain: Promise<LibraryBoundaryInvalidationBatch> | null =
    null;
  private closePromise: Promise<void> | null = null;

  constructor(
    transport: LibraryBoundaryTransport,
    options: LibraryBoundarySessionOptions = {}
  ) {
    this.#transport = transport;
    this.onListenerError = options.onListenerError ?? null;
    this.client = new LibraryBoundaryClient(transport);
  }

  get state(): LibraryBoundarySessionState {
    return this.currentState;
  }

  getLastSeenRevision(
    scope: MaintainedSnapshotScope
  ): MaintainedSnapshotRevision | null {
    return this.lastSeenRevisionByScope.get(scope) ?? null;
  }

  snapshotLastSeenRevisions(): ReadonlyMap<
    MaintainedSnapshotScope,
    MaintainedSnapshotRevision
  > {
    return new Map(this.lastSeenRevisionByScope);
  }

  takeListenerErrors(): LibraryBoundaryListenerErrorDrain {
    const reports = [...this.listenerErrors];
    const droppedReportCount = this.droppedListenerErrorCount;
    this.listenerErrors.length = 0;
    this.droppedListenerErrorCount = 0;
    return {
      reports,
      droppedReportCount,
      retainedReportCount: reports.length,
      retentionLimit: LIBRARY_BOUNDARY_LISTENER_ERROR_RETENTION_LIMIT
    };
  }

  subscribeInvalidations(
    listener: LibraryBoundaryInvalidationListener
  ): LibraryBoundaryInvalidationSubscription {
    this.requireReady("subscribe to invalidations");
    this.listeners.add(listener);

    let subscribed = true;
    return {
      unsubscribe: () => {
        if (!subscribed) {
          return;
        }

        subscribed = false;
        this.listeners.delete(listener);
      }
    };
  }

  pumpEvents(maxEvents: number): Promise<LibraryBoundaryInvalidationBatch> {
    this.requireReady("pump events");
    if (!Number.isSafeInteger(maxEvents) || maxEvents <= 0) {
      throw new LibraryBoundarySessionInvalidMaxEventsError(maxEvents);
    }
    if (this.inFlightDrain !== null) {
      throw new LibraryBoundarySessionDrainInProgressError();
    }

    const drain = this.drainEvents(maxEvents);
    this.inFlightDrain = drain;
    void drain.then(
      () => {
        if (this.inFlightDrain === drain) {
          this.inFlightDrain = null;
        }
      },
      () => {
        if (this.inFlightDrain === drain) {
          this.inFlightDrain = null;
        }
      }
    );
    return drain;
  }

  async close(): Promise<void> {
    if (this.currentState === "closed") {
      return;
    }
    if (this.closePromise !== null) {
      return this.closePromise;
    }

    this.currentState = "closing";
    this.closePromise = this.closeSession();
    return this.closePromise;
  }

  private async closeSession(): Promise<void> {
    try {
      await this.inFlightDrain?.catch(() => undefined);
      await this.#transport.close?.();
      this.listeners.clear();
      this.currentState = "closed";
    } catch (error) {
      this.currentState = "failed";
      this.closePromise = null;
      throw error;
    }
  }

  private async drainEvents(
    maxEvents: number
  ): Promise<LibraryBoundaryInvalidationBatch> {
    let batch: LibraryBoundaryInvalidationBatch;
    try {
      const reply = await this.client.readAfterBoundaryEvents({
        lastSeenEventSequence: null,
        maxEvents
      });
      batch = this.collectInvalidations(reply.events);
    } catch (error) {
      if (this.currentState !== "closing") {
        this.currentState = "failed";
      }
      throw error;
    }

    this.fanoutInvalidations(batch);
    return batch;
  }

  private collectInvalidations(
    events: readonly LibraryBoundaryEvent[]
  ): LibraryBoundaryInvalidationBatch {
    const changedByScope = new Map<
      MaintainedSnapshotScope,
      MaintainedSnapshotInvalidation
    >();

    for (const event of events) {
      if (event.type !== "maintainedSnapshotInvalidated") {
        continue;
      }

      const envelope = event.payload as MaintainedSnapshotEvent;
      const accepted = this.acceptInvalidation(envelope.invalidation);
      if (accepted === null) {
        continue;
      }

      const pending = changedByScope.get(accepted.scope);
      if (pending?.revision === null) {
        continue;
      }
      if (accepted.revision === null) {
        changedByScope.set(accepted.scope, accepted);
        continue;
      }
      if (
        pending === undefined ||
        pending.revision === null ||
        compareRevisions(accepted.revision, pending.revision) > 0
      ) {
        changedByScope.set(accepted.scope, accepted);
      }
    }

    const invalidations = Array.from(changedByScope.values());
    return {
      invalidations,
      changedScopes: invalidations.map((invalidation) => invalidation.scope),
      changedRevisions: invalidations.flatMap((invalidation) =>
        invalidation.revision === null
          ? []
          : [{ scope: invalidation.scope, revision: invalidation.revision }]
      ),
      lastSeenRevisions: this.snapshotLastSeenRevisions()
    };
  }

  private acceptInvalidation(
    invalidation: MaintainedSnapshotInvalidation
  ): MaintainedSnapshotInvalidation | null {
    if (invalidation.revision === null) {
      return invalidation;
    }

    const lastSeen = this.lastSeenRevisionByScope.get(invalidation.scope);
    if (
      lastSeen !== undefined &&
      compareRevisions(invalidation.revision, lastSeen) <= 0
    ) {
      return null;
    }

    this.lastSeenRevisionByScope.set(invalidation.scope, invalidation.revision);
    return invalidation;
  }

  private fanoutInvalidations(batch: LibraryBoundaryInvalidationBatch): void {
    if (batch.invalidations.length === 0) {
      return;
    }

    for (const listener of Array.from(this.listeners)) {
      try {
        listener(batch);
      } catch (error) {
        this.reportListenerError({ error, batch, listener });
      }
    }
  }

  private reportListenerError(
    report: LibraryBoundaryListenerErrorReport
  ): void {
    if (
      this.listenerErrors.length ===
      LIBRARY_BOUNDARY_LISTENER_ERROR_RETENTION_LIMIT
    ) {
      this.listenerErrors.shift();
      this.droppedListenerErrorCount += 1;
    }
    this.listenerErrors.push(report);

    if (this.onListenerError === null) {
      return;
    }

    try {
      this.onListenerError(report);
    } catch {
      return;
    }
  }

  private requireReady(operation: string): void {
    if (this.currentState !== "ready") {
      throw new LibraryBoundarySessionStateError(operation, this.currentState);
    }
  }
}

function compareRevisions(
  left: MaintainedSnapshotRevision,
  right: MaintainedSnapshotRevision
): number {
  const leftValue = BigInt(left);
  const rightValue = BigInt(right);
  if (leftValue === rightValue) {
    return 0;
  }

  return leftValue > rightValue ? 1 : -1;
}
