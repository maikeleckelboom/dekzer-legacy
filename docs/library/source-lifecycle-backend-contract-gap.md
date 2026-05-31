---
status: architecture-inventory
doctrine-version: 0.1
last-reviewed: 2026-05-31
owner: library-boundary-service
canonical-context:
  - source-root-scan-admission-contract
  - library-substrate-e2e-flow-diagrams
  - source-lifecycle-visible-state-contract
scope:
  - backend-contract-inventory
  - contract-gap
---

# Source lifecycle — backend contract inventory & gap

Status: architecture inventory / partially implemented contract gap

Purpose
-------
This note inventories the durable facts and runtime signals owned by the Rust/SQLite substrate and the
boundary service, then defines the exact backend contract gap for a backend-owned source lifecycle/readiness
surface.

Current owners (durable & runtime)
----------------------------------

- Rust/SQLite substrate: `crates/library-store-sqlite` — owns durable tables and read-models (see `sources`,
  `source_state`, `source_scan_state`, `source_directories`, `source_files`, and the `literal_hierarchy` read model).
- Boundary service: `crates/library-boundary-service` — owns host exposure and snapshot/read mapping (see
  `LibraryBoundaryService` in `service.rs` and protocol mappings in `snapshot_read_protocol.rs`).
- Renderer projection: `apps/desktop/src/renderer/.../sourceReadiness.ts` — temporary renderer-owned projection
  state over currently exposed backend facts and live scan signals. It is not durable authority and not a UI taxonomy.

Current exposed contracts (surface)
-----------------------------------

- `libraryRoots.readLocalRoots` — coarse root identity + availability (`LibraryBoundaryService::read_local_roots`,
  `apps/desktop/src/shared/libraryRoots/readLocalRoots.ts`).
- `snapshotRead.readSourceLifecycle` — dedicated backend-owned source lifecycle read keyed by `sourceId`.
  It returns typed semantic source facts from `sources`, `source_state`, and `source_scan_state`, or `null`
  for not found. Desktop Main maps `null` to a typed `notFound` result and validates invalid `sourceId`
  requests before they cross the host boundary.
- `snapshotRead.readLibraryTreeChildren` — windowed literal-hierarchy reads with per-window coverage
  (`read_literal_hierarchy_children` ⇒ `crates/library-store-sqlite::read_models::literal_hierarchy`).
- `snapshotRead.readNavigationRows`, `snapshotRead.readContents`, and related snapshot reads — navigation rows and
  selected-contents reads provide scoped authoritative rows.
- Event stream: the service boundary event stream is cursor/read-after based and owned by the boundary service.
  Desktop Main owns polling this stream and delivers typed batches to renderer subscribers. Events publish scan
  lifecycle and maintained snapshot invalidation signals, but they are not a durable per-source lifecycle contract.

Durable / store facts currently available
----------------------------------------

- `sources` table: canonical `source_id`, `source_class`, `identity_key`, `display_name`, `is_user_visible`.
- `source_locators`: locator kinds (`absolute_path`, `removable_volume`), canonical path, relation to `source_id`.
- `source_state`: `mount_status`, `access_state`, `access_issue_kind`, `effective_path`, `observed_volume_label`,
  `access_checked_at`, `last_seen_at`, `updated_at`.
- `source_scan_state`: `scan_phase` (idle, scanning, complete, partial, blocked, failed), scan timestamps,
  `scan_issue_kind`, `error_detail`, `updated_at`.
- `source_directories` / `source_files`: per-directory `presence_state`, `dir_scan_state`,
  `has_primary_media_descendant`,
  and file presence/media metadata used by literal-hierarchy reads.
- Read-models: the literal-hierarchy read path performs an authoritative join of `sources`, `source_state`, and
  `source_scan_state` to compute a `SourceReadiness`-like structure (see `load_source_readiness` in
  `crates/library-store-sqlite/src/read_models/literal_hierarchy.rs`).

Runtime / event facts currently available
----------------------------------------

- Scan lifecycle events published by the boundary service (`SourceScanStarted`, `SourceScanProgressed`,
  `SourceScanCompleted`, `SourceScanBlocked`, `SourceScanFailed`, `SourceScanCancelled`) with `root_id`, `scan_run_id`,
  `phase`, basic counters, and optional `detail` (see `service.rs` and `session_events`).
- Maintained snapshot revision invalidations used to drive projection refresh.

Renderer lifecycle projection currently performed
-------------------------------------------------
The renderer projects a collapsed `SourceReadiness` (kinds like `registered`, `scanning`, `rescanRunning`,
`ready`, `empty`, `unavailable`, `blocked`, `failed`) by combining:

- `projection.bindingsById` (browser projection)
- `readLocalRoots` (coarse availability)
- snapshot read `source` read-models (when available)
- scan progress events mapped by `rootId` (scan progress states)

This projection is intentionally an application-side lifecycle projection over multiple substrate surfaces.
It normalizes currently exposed facts into renderer state until a backend-owned lifecycle read surface exists.

What the backend now exposes
----------------------------

- `readSourceLifecycle` is the dedicated backend-owned source lifecycle read surface keyed by `sourceId`.
- The record currently exposes `sourceId`, `sourceClass`, `isUserVisible`, `mountStatus`, `accessState`,
  optional `accessIssueKind`, `scanPhase`, optional `scanIssueKind`, scan timestamps, `lastSeenAtMs`, and
  `updatedAtMs`.
- The record does not expose presentation readiness labels such as `ready`, `empty`, `rescanRunning`, badges,
  copy, tones, or child rows.
- Known user-visible sources remain readable when unavailable because the read uses durable source lifecycle
  rows, not renderer projection state.
- Known sources also remain readable if `source_state` or `source_scan_state` side rows are absent. The read is
  anchored on `sources` and left-joins lifecycle side rows. Missing access/mount facts are returned as
  `mountStatus: unknown` and `accessState: unknown`; missing scan facts are returned as `scanPhase: idle`.
  `notFound` is reserved for an absent `sources` row.

What remains a backend contract gap
-----------------------------------

- `readLocalRoots` exposes root identity and a coarse `availability` only; it does not expose backend-owned
  lifecycle fields such as `mount_status`, `access_state`, `scan_phase`, `scan_issue_kind`, last-scan timestamps,
  or an authoritative whole-source coverage summary.
- `readLibraryTreeChildren` is a windowed read and its `coverage` is scoped to the requested entry point/parent
  directory; it is not intended as a global per-source lifecycle contract.
- Events communicate activity and progress but are append-only signals and do not, by themselves, constitute an
  authoritative collapsed lifecycle surface.
- Renderer lifecycle hydration is implemented through a dedicated non-visual source lifecycle read owner. It reads
  `readSourceLifecycle({ sourceId })` for current browser source rows plus selected/expanded source rows, stores
  `sourceLifecycleBySourceId`, deduplicates in-flight reads per source, and preserves the last known lifecycle
  record when refresh returns `notFound`, `hostUnavailable`, or `readFailed`.
- Whole-source coverage summary remains omitted. It should only be added if it can be computed without duplicating
  branch/window hierarchy semantics or implying authoritative `empty` state without proof.
- Locator/root identity remains outside `readSourceLifecycle` for now; `readLocalRoots` continues to own local-root
  identity and path exposure.

Why `readLibraryTreeChildren` is NOT the collapsed source lifecycle contract
----------------------------------------------------------------------

- `readLibraryTreeChildren` is a scoped, paginated read for a specific entry point and parent directory; its
  `coverage` value describes the read window and may be `emptyResultAuthoritative` only with respect to that
  window. It is not a canonical whole-source lifecycle summary.
- Using `readLibraryTreeChildren` as a lifecycle API conflates two concerns: windowed row pagination and source-level
  lifecycle. It is also awkward for clients that need lifecycle facts without a particular parent-directory context.

Why `readLocalRoots` is currently too small
------------------------------------------

- `readLocalRoots` returns `rootId`, `canonicalPath`, and a coarse `availability` derived from
  `source_state.access_state`.
  It lacks the richer, authoritative substrate fields that are needed to build a stable renderer projection (e.g.
  `mount_status`, `scan_phase`, `scan_issue_kind`, `last_scan_*` timestamps, `is_user_visible`).

Why navigation rows should not quietly absorb lifecycle state
-----------------------------------------------------------

- Navigation rows are a presentation-oriented projection. If lifecycle semantics are absorbed into the navigation
  projection without an explicit backend contract, different clients may interpret or duplicate lifecycle logic
  inconsistently. A backend-owned read surface or explicit enrichment of navigation rows (with clear semantics)
  avoids ad-hoc duplication and inconsistent UX.

Product invariants (non-negotiable)
----------------------------------

- Known source does not disappear when filesystem resolution is unavailable. The substrate's durable identity must
  remain queryable and visible (`is_user_visible`) until the user explicitly removes or forgets the source.
- Source-level lifecycle (mount/access/scan) is separate from branch-level refresh and contents window reads. Branch
  refresh remains a branch-owned operation.
- Events signal reads and active scan progress; events do not synthesize children nor act as durable source truth.
- Source lifecycle refresh remains separate from hierarchy branch refresh. Maintained `navigationRows` and
  `libraryBrowser` invalidations trigger targeted lifecycle rereads for known/visible source ids. Delivered scan
  events also trigger targeted lifecycle rereads for named visible roots, but active scan progress is still only
  runtime scan-phase immediacy and not durable truth. Live scan progress must not override backend-owned
  mount/access barriers.

Candidate backend contract shapes (do not choose prematurely)
-----------------------------------------------------------

Option A — Enrich `readLocalRoots` with backend-owned semantic lifecycle fields
-----------------------------------------------------------------------------

What it would own

Field names below describe substrate facts, not final protocol spelling. A boundary contract must expose typed
semantic fields using the project's contract naming conventions.

- Extend the reply to include per-root lifecycle fields:
  - `mountStatus` (typed enum: `unknown` | `mounted` | `unmounted` | …)
  - `accessState` (typed enum: `accessible` | `missing` | `blocked` | `unknown`)
  - `accessIssueKind`, optional typed enum
  - `scanPhase` (typed enum: `idle` | `scanning` | `complete` | `partial` | `blocked` | `failed`)
  - `scanIssueKind`, optional typed enum
  - `lastScanStartedAtMs`, `lastScanFinishedAtMs`, `lastSuccessfulScanAtMs`
  - `lastSeenAtMs`, `updatedAtMs`

What it must NOT own

- It must not own application-level readiness labels such as `ready` or `rescanRunning`, UI-only labels, or any
  synthesized children/rows. `empty` may only appear as a backend lifecycle result if an authoritative
  whole-source coverage calculation proves it. It must not claim whole-window row coverage beyond coarse
  whole-source summary fields.

Advantages

- Minimal surface change (enriches an already-visible root list).
- Gives renderer authoritative lifecycle facts to compute a consistent projection.

Risks

- Expands the purpose of `readLocalRoots` (could be overloading the root-list surface).
- Requires careful versioning and client handling of the enlarged reply shape.

Why avoid presentation vocabulary

- The backend should expose authoritative enums and timestamps; presentation classification and friendly wording
  remain renderer responsibilities.

Renderer ownership afterward

- Renderer still owns the application-level `SourceReadiness` projection where it combines backend lifecycle facts
  with local runtime inputs such as in-flight scan progress. It should not duplicate backend-owned lifecycle
  inference once the backend contract exists.

Option B — Add dedicated `readSourceLifecycle` (per-source read surface)
------------------------------------------------------------------

What it owns

- The implemented snapshot read surface is keyed by `sourceId` and returns an authoritative, backend-owned contract
  describing per-source lifecycle facts:
  - `sourceId`, `sourceClass`, `isUserVisible`
  - `mountStatus`, `accessState`, optional `accessIssueKind`
  - `scanPhase`, optional `scanIssueKind`, `lastScanStartedAtMs`, `lastScanFinishedAtMs`,
    `lastSuccessfulScanAtMs`
  - `lastSeenAtMs`, `updatedAtMs`

What it must NOT own

- It must not synthesize child rows, create pagination windows, or include UI-only labels. It must not subsume
  branch/contents read semantics.

Advantages

- A clear, single authoritative surface for per-source lifecycle facts. Cleaner separation of concerns and easier
  client contracts.

Remaining risks

- Needs careful update and invalidation semantics so lifecycle reads stay coherent with events and projection seeds.
- Future whole-source coverage summary must not duplicate or contradict hierarchy-window coverage.

Why avoid presentation vocabulary

- Expose typed semantic enums and timestamps only; the renderer maps these to presentation kinds.

Renderer ownership afterward

- Renderer still owns the application-level `SourceReadiness` projection where it combines backend lifecycle facts
  with local runtime inputs such as in-flight scan progress. It should not duplicate backend-owned lifecycle
  inference once the backend contract exists.

Implementation acceptance bar
-----------------------------
Current implementation provides demonstrable evidence (tests + code) that:

1. Registered known source remains visible when filesystem resolution is unavailable (durable `is_user_visible` +
   read surface returns the source record).
2. `unavailable` / `blocked` / `failed` are emitted only from authoritative substrate facts (derived from
   `source_state` and `source_scan_state` and not guessed from events alone).
3. A `completed` scan does not synthesize children; completed-only state must not be used to fabricate rows.
4. Branch refresh semantics remain branch-owned and are not automatically promoted to source lifecycle changes.
5. Renderer source readiness can consume backend lifecycle facts as primary durable source truth while preserving
   active scan progress for scan-phase runtime immediacy only. Backend mount/access barriers remain authoritative.
6. Missing lifecycle side rows do not produce `notFound`; they produce a known-source lifecycle record with typed
   unknown/default lifecycle facts.

Invalidation semantics discovered in this pass:

- Root lifecycle mutations through `SourceLifecycleTx` call `sync_root_projection_state`, which reseeds navigation
  and library-browser projections; subsequent event reads publish maintained snapshot invalidations for those
  revisions.
- Public `upsert_source_state` reseeds navigation and library-browser projections. Public
  `upsert_source_scan_state` reseeds library-browser projections.
- Discovery `start_scan_session` updates `source_scan_state` to `scanning` inside discovery materialization without
  directly publishing a maintained invalidation at scan start. The already-delivered scan event is therefore used as
  the supported renderer trigger for targeted scan-related lifecycle refresh; it does not synthesize durable
  lifecycle state.

Still future:

- `empty` should be emitted only when an authoritative whole-source coverage calculation or per-window
  `emptyResultAuthoritative` proves there are zero rows. `readSourceLifecycle` does not emit `empty`.
- A future active-run identity can be added if callers need to correlate durable `scanPhase` with a specific
  runtime scan run; the current minimal read exposes durable `scanPhase` without conflating it with branch refresh.
- Renderer hydration of `readSourceLifecycle` records is implemented for visible/selected/expanded source rows.

Docs / authority map
--------------------
This note is an inventory and contract gap. If the authority map is present, register this doc as an inventory /
contract-gap entry under source/scan contracts so implementers can find the gap and next steps.

Current preference
------------------

- Prefer Option B, a dedicated source lifecycle read surface.
- Option B is implemented for the minimal source lifecycle substrate contract.
- Use Option A only for future local-root-list enrichment if a concrete caller needs root-list hydration.
- Reason: source lifecycle is not merely local-root hydration. Future source classes should not be forced through
  `readLocalRoots`.

Follow-up work
--------------

- Add a whole-source coverage summary only if the semantics can be proved without duplicating branch/window reads.
- Add source locator/root identity only when the lifecycle caller needs it and the source/root mapping remains clean.
- Add a narrower lifecycle invalidation scope only if future clients need lifecycle refresh independent of current
  maintained projection invalidations and delivered scan events.
