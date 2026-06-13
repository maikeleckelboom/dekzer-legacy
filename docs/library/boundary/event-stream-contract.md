---
status: accepted
doctrine-version: 0.1
last-reviewed: 2026-06-09
owner: renderer-substrate-boundary
canonical-context:
  - first-slice-substrate-map
  - source-hierarchy-contract
  - frame-stability-contract
  - electron-boundary-spine
scope:
  - boundary-event-model
  - cursor-only-event-reading
  - event-ring-semantics
  - session-bounded-events
  - gap-recovery
  - scan-event-family
  - maintained-snapshot-invalidation
  - event-parser-contract
  - live-scan-progress-boundary
---

# Library Boundary Event Stream Contract

## Core law

**Service boundary events are cursor-only. There is no read-pending, drain, events-changed, or
service subscription model.**

The Rust library boundary event stream is a polled, session-bounded event ring. Desktop Main reads
service events via `ReadAfter` with an `eventSequence` cursor, then forwards typed event batches to
renderer consumers. There is no durable audit log, no event replay archive, and no Rust service-side
push/subscription mechanism.

## Event model

The boundary event model uses exactly one read command:

```
ReadLibraryBoundaryEventsAfter(lastSeenEventSequence, maxEvents)
  → ReadLibraryBoundaryEventsAfterReply(events, latestEventSequence,
      earliestRetainedSequence, gapDetected)
```

### Cursor semantics

| Field                      | Meaning                                                                                                                                                                                                                                                                                   |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lastSeenEventSequence`    | The last event sequence the caller has already observed. `None` means start from the beginning.                                                                                                                                                                                           |
| `latestEventSequence`      | The next cursor the consumer should pass in the subsequent `ReadAfter` call. This is the event sequence of the last event delivered in this reply, or the previously supplied `lastSeenEventSequence` when no new events were returned, or `None` when no events have been published yet. |
| `earliestRetainedSequence` | The earliest sequence still retained in the event ring buffer.                                                                                                                                                                                                                            |
| `gapDetected`              | True when the caller's `lastSeenEventSequence` is older than the earliest retained event, meaning events were compacted away.                                                                                                                                                             |

`latestEventSequence` is the **next cursor** position, not a "highest available event" guarantee. It may
equal the caller's input `lastSeenEventSequence` when no new events have been published since the last read.

### Session-bounded event ring

Events are stored in a bounded in-memory ring buffer (default: 256 events). When the ring is full, oldest
events are dropped. The event ring is session-scoped: it is created on service start, populated in-memory,
and discarded on service shutdown. There is no durable event log retained across restarts.

This means:

- The renderer cannot rely on event history surviving a service restart.
- A fresh session starts with no events and an empty ring.
- `earliestRetainedSequence` advances as old events are compacted.

### Forbidden event concepts

The following concepts do not exist in the current event model and must not appear in docs as active:

| Forbidden term             | Why absent                                                                     |
| -------------------------- | ------------------------------------------------------------------------------ |
| `ReadPending`              | Superseded by cursor-only `ReadAfter`.                                         |
| `readPending`              | Superseded by cursor-only `ReadAfter`.                                         |
| `eventsChanged`            | Superseded. No change-list or diff model exists.                               |
| `drain`                    | No drain/batch flush concept exists.                                           |
| service subscription model | Service events are polled through `ReadAfter`, not pushed by the Rust service. |
| durable audit log          | Events are in-memory session-only.                                             |
| user re-add blame          | Gaps are internal recovery conditions, not user-facing faults.                 |

## Event families

Two distinct event families exist in the current implementation:

### SourceScanEvent

Describes scan job lifecycle events for a source root.

| Field                | Meaning                                                                                                                             |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `eventSequence`      | Monotonic event sequence.                                                                                                           |
| `occurredAtMs`       | Wall-clock timestamp.                                                                                                               |
| `kind`               | `sourceScanStarted`, `sourceScanProgressed`, `sourceScanCompleted`, `sourceScanFailed`, `sourceScanBlocked`, `sourceScanCancelled`. |
| `rootId`             | Source root identity.                                                                                                               |
| `scanRunId`          | Scan run identity.                                                                                                                  |
| `phase`              | `scanning`, `blocked`, `interrupted`.                                                                                               |
| `directoriesVisited` | Cumulative directories visited.                                                                                                     |
| `filesVisited`       | Cumulative files visited.                                                                                                           |
| `filesDiscovered`    | Cumulative files admitted to inventory.                                                                                             |
| `mediaCandidates`    | Cumulative media candidates detected.                                                                                               |
| `queuedWorkItems`    | Cumulative work items queued from this scan.                                                                                        |
| `detail`             | Optional human-readable detail.                                                                                                     |

### MaintainedSnapshotInvalidated

Signals that a maintained snapshot scope has been invalidated and consumers must
reread snapshot data authoritatively.

| Field           | Meaning                                             |
| --------------- | --------------------------------------------------- | ------- |
| `eventSequence` | Monotonic event sequence.                           |
| `occurredAtMs`  | Wall-clock timestamp.                               |
| `invalidation`  | `{ scope: MaintainedSnapshotScope, revision: string | null }` |

Snapshot scopes:

| Scope            | Invalidated reads                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `navigationRows` | `ReadNavigationRows`, `LoadNavigationRow`, `LoadNavigationRowByStableKey`.                                                                             |
| `contents`       | `ReadLibraryTreeChildren`, `ReadSourceLifecycle`, `ReadSourceIntegrity`, `ReadSourceMaintenance`, `ReadTrackIdentityReviewCandidates`, `ContentsRead`. |

`SourceScanEvent` and `MaintainedSnapshotInvalidated` are distinct families. They are emitted through
the same event ring and share the same `eventSequence` space. They are not subtypes of each other.

## Gap recovery

### Gap detection

A gap is detected when the caller's `lastSeenEventSequence + 1` is less than the `earliestRetainedSequence`.
The reply sets `gapDetected = true`. A gap means events were compacted away from the ring buffer while
the renderer was not polling (or polling too slowly).

### Gap recovery behavior

When `gapDetected` is true, Desktop Main's Boundary Event Pump reports the gap and renderer consumers schedule the
authoritative projection rereads:

1. The renderer marks its cached hierarchy/contents/projection state as potentially stale.
2. The renderer issues authoritative snapshot rereads for visible and selected scopes.
3. The renderer does **not** tell the user to re-add a source or rescan.
4. The renderer does **not** blame the user for the gap.
5. The renderer acknowledges the gap via `acknowledgedGap()` to dismiss the recovery indicator.

Gap recovery is an internal consistency mechanism. It is not a product-facing error surface
and must not surface as user-visible "source failed" messaging.

### Event gaps are not user source faults

An event gap arises from the ring buffer compacting away events the renderer missed. It does not
mean a source root failed, a scan crashed, or the user must take action. The correct response is
authoritative reread, not user blame.

## Event parser contract

### Shared parser location

The boundary event parser is at `apps/desktop/src/shared/library/boundary/eventParser.ts`.
It is the single point of event shape validation shared between all consumers.

### Parser rules

- Every parsed event is `AppBoundaryEvent`: `{ type: 'sourceScanEvent', payload: AppSourceScanEvent }` or `{ type: 'maintainedSnapshotInvalidated', payload: AppMaintainedSnapshotInvalidatedEvent }` or `{ type: 'unsupported', payload: unknown }`.
- Unknown or malformed event types return `unsupported`.
- Fields with invalid types (non-numeric `eventSequence`, non-string `rootId`, etc.) return `unsupported`.
- The `eventSequence` field must be a non-negative safe integer (as number or string).
- The `occurredAtMs` field must be a non-negative safe integer.
- `rootId` and `scanRunId` must be non-empty strings.
- `phase` must be one of `scanning`, `blocked`, `interrupted`.
- `detail` is coerced to string or null.

### Empty event reply

An empty `events` array in the reply means no new events since the last cursor. It is not a failure.
Empty events with `gapDetected = false` means the event stream is up to date and there are no new
events to deliver. Empty events with `gapDetected = true` means events were lost and recovery is needed.

### Event read failure

Event read failure is explicit: the IPC call throws or returns an error envelope. A successful IPC
call returns the reply even with zero events. Zero events does not mean failure.

## True live scan progress

### Current state

`runRootScan` is synchronous. The current Rust service executes the root scan on the blocking command
path. Scan events (`SourceScanStarted`, `SourceScanProgressed`, `SourceScanCompleted`, etc.) are
published from within the synchronous scan execution. There is no background scan job lifecycle.

### Current scan event limitations

- Scan events carry cumulative counters (directories visited, files visited, etc.) but are published
  synchronously within the command path, not asynchronously from a background worker.
- Scan events do not provide `affectedParentNodeIds` for targeted renderer cache invalidation.
  The renderer currently uses event-family signals (`SourceScanCompleted` → refresh affected root)
  rather than fine-grained parent-ID invalidation.
- `SourceScanProgressed` is emitted at coarse intervals during the synchronous scan but is not
  a true streaming progress signal from an independent background job.

### Next implementation frontier

True live scan progress requires background scan job lifecycle:

1. `StartRootScan(rootId)` returns a `scanRunId` quickly without blocking.
2. The scan runs outside the blocking command path.
3. Progress events stream while the scan is running.
4. Terminal events (completed, failed, blocked, interrupted) terminate the scan run.
5. `MaintainedSnapshotInvalidated` events trigger authoritative rereads of affected snapshot scopes.

This is not yet implemented. The current docs must not claim live progress is active.

## Desktop delivery and renderer event consumption

Desktop Main owns `BoundaryEventPump` v1
(`apps/desktop/src/main/library/boundary/eventPump.ts`). The pump starts when the library boundary
host is started and at least one renderer subscriber exists. It polls `ReadAfter`, owns
`lastSeenEventSequence`, detects `gapDetected`, and forwards batches through typed IPC.

The Boundary Event Pump is the only scheduling owner for `ReadAfter` or a future `WaitForEventsAfter` transport.
Renderer code consumes forwarded publications and must not schedule either service event-read operation.

The renderer consumes Main-delivered batches through the boundary events controller
(`apps/desktop/src/renderer/library/boundary/boundaryEvents.ts`).

### Consumer behavior

- Subscribes to Main-delivered boundary event batches.
- Accumulates scan progress state per root from `SourceScanEvent` payloads.
- Exposes `MaintainedSnapshotInvalidated` as normal invalidation signals.
- Detects `gapDetected` and sets `recoveryNeeded`.
- Exposes `acknowledgedGap()` to dismiss the recovery indicator.
- Rejects `unsupported` event types silently (does not surface them to the user).
- Does not author scan progress — it reflects events published by the substrate.
- Does not poll `ReadAfter` and does not own an event cursor.

Normal maintained snapshot invalidation is separate from gap recovery. `navigationRows` invalidation
refreshes navigation rows. `contents` invalidation refreshes maintained source tree/status,
current contents, and track identity candidate snapshot reads through authoritative reads while
preserving previous visible rows during the pending refresh.

### Renderer must not

| Forbidden renderer behavior                           | Why                                                            |
| ----------------------------------------------------- | -------------------------------------------------------------- |
| Author scan progress                                  | Scan progress is published by the substrate, not the renderer. |
| Create or inject boundary events                      | Only the boundary service publishes events.                    |
| Treat event gap as user source fault                  | Gap recovery is internal consistency, not user-visible error.  |
| Surface `SourceScanEvent` as user-facing notification | Scan events feed renderer state, not user toasts.              |

## Non-goals

This contract does not:

- Define real-time push events or WebSocket streaming.
- Govern event replay or durable event logs.
- Define scan admission policy or scan job lifecycle (see `source-root-scan-admission-contract.md`).
- Define source lifecycle visible state (see `source-lifecycle-visible-state-contract.md`).
- Govern deck runtime events or performance session events (those are future architecture).
- Implement background scan jobs (this is the next implementation frontier, not part of this doc pass).

## Invariants

| Invariant                                                                                        | Enforcement                                                                                              |
| ------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| Service boundary events are cursor-only (`ReadAfter`). No `ReadPending`/`drain`/`eventsChanged`. | Protocol shape in `session_events.rs`.                                                                   |
| `latestEventSequence` is the next cursor, not "highest available event."                         | Service computes from last delivered or previously-supplied sequence.                                    |
| Empty events reply means no events, not failure.                                                 | Service returns empty vec with valid cursor fields.                                                      |
| Event read failure is explicit.                                                                  | Main pump forwards a failed delivery payload; renderer boundary events controller sets `lastReadFailed`. |
| Malformed event payloads are rejected by shared parser.                                          | `eventParser.ts` returns `unsupported` for invalid shapes.                                               |
| Event gap triggers authoritative snapshot reread, not user re-add/rescan blame.                  | Renderer sets `recoveryNeeded` on `gapDetected`.                                                         |
| `SourceScanEvent` and `MaintainedSnapshotInvalidated` are distinct families.                     | Protocol enum in `session.rs` — two separate variants.                                                   |
| The event ring is session-bounded, not a durable audit log.                                      | In-memory `VecDeque` with `MAX_STORED_EVENTS = 256`.                                                     |
| Renderer consumes events but does not author scan progress.                                      | Boundary events controller reflects substrate-published events delivered by Main.                        |
| `SourceScanCancelled` is terminal for a `scanRunId`; `SourceScanCompleted` must not follow.      | Service publishes cancellation from the scan job error path and does not re-publish completed.           |
| True live scan progress requires background scan jobs (not yet implemented).                     | `runRootScan` is synchronous; no background job lifecycle exists.                                        |
