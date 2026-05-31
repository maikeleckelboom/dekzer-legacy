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

Status: architecture inventory / contract gap

Purpose
-------
This note inventories the durable facts and runtime signals owned by the Rust/SQLite substrate and the
boundary service, then defines the exact backend contract gap for a backend-owned source lifecycle/readiness
surface. This is a documentation-only pass: it does not implement the contract.

Current owners (durable & runtime)
----------------------------------

- Rust/SQLite substrate: `crates/library-store-sqlite` — owns durable tables and read-models (see `sources`,
  `source_state`, `source_scan_state`, `source_directories`, `source_files`, and the `literal_hierarchy` read model).
- Boundary service: `crates/library-boundary-service` — owns host exposure and snapshot/read mapping (see
  `LibraryBoundaryService` in `service.rs` and protocol mappings in `snapshot_read_protocol.rs`).
- Renderer projection: `apps/desktop/src/renderer/.../sourceReadiness.ts` — a temporary renderer-owned projection
  derived from currently exposed reads and events; presentational only.

Current exposed contracts (surface)
-----------------------------------

- `libraryRoots.readLocalRoots` — coarse root identity + availability (`LibraryBoundaryService::read_local_roots`,
  `apps/desktop/src/shared/libraryRoots/readLocalRoots.ts`).
- `snapshotRead.readLibraryTreeChildren` — windowed literal-hierarchy reads with per-window coverage
  (`read_literal_hierarchy_children` ⇒ `crates/library-store-sqlite::read_models::literal_hierarchy`).
- `snapshotRead.readNavigationRows`, `snapshotRead.readContents`, and related snapshot reads — navigation rows and
  selected-contents reads provide scoped authoritative rows.
- Event stream: `libraryBoundaryEvents.readAfter` — publishes scan lifecycle events (
  start/progress/completed/failed/blocked)
  but events are signals, not a durable per-source contract.

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

Renderer-only inference currently performed
-----------------------------------------
The renderer projects a collapsed `SourceReadiness` (kinds like `registered`, `scanning`, `rescanRunning`,
`ready`, `empty`, `unavailable`, `blocked`, `failed`) by combining:

- `projection.bindingsById` (browser projection)
- `readLocalRoots` (coarse availability)
- snapshot read `source` read-models (when available)
- scan progress events mapped by `rootId` (scan progress states)

This projection is intentionally an application-side, presentational inference over multiple substrate surfaces.
It synthesizes a small set of presentation-ready kinds from substrate facts and events.

What the backend does NOT yet expose (the gap)
----------------------------------------------

- There is no dedicated backend-owned, authoritative, per-source collapsed lifecycle/readiness read surface (for
  example `readSourceLifecycle` keyed by `sourceId` or `rootId`).
- `readLocalRoots` exposes root identity and a coarse `availability` only; it does not expose raw lifecycle fields
  such as `mount_status`, `access_state`, `scan_phase`, `scan_issue_kind`, last-scan timestamps, or an
  authoritative whole-source coverage summary.
- `readLibraryTreeChildren` is a windowed read and its `coverage` is scoped to the requested entry point/parent
  directory; it is not intended as a global per-source lifecycle contract.
- Events communicate activity and progress but are append-only signals and do not, by themselves, constitute an
  authoritative collapsed lifecycle surface.

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

Candidate backend contract shapes (do not choose prematurely)
-----------------------------------------------------------

Option A — Enrich `readLocalRoots` with backend-owned raw lifecycle fields
-------------------------------------------------------------------------

What it would own

- Extend the `ReadLocalRootsReply` to include raw substrate fields per-root (authoritative, schema-facing):
  - `mount_status` (enum: `unknown`|`mounted`|`unmounted`|...)
  - `access_state` (enum: `accessible`|`missing`|`blocked`|`unknown`)
  - `access_issue_kind` (nullable string enum)
  - `scan_phase` (enum: `idle`|`scanning`|`complete`|`partial`|`blocked`|`failed`)
  - `scan_issue_kind` (nullable string enum)
  - `last_scan_started_at_ms`, `last_scan_finished_at_ms`, `last_successful_scan_at_ms`
  - `last_seen_at_ms` / `updated_at_ms`

What it must NOT own

- Presentation mappings (for example `ready`/`rescanRunning`/`empty`), UI-only labels, or any synthesized
  children/rows. It must not claim whole-window row coverage beyond coarse whole-source summary fields.

Advantages

- Minimal surface change (enriches an already-visible root list).
- Gives renderer authoritative raw facts to compute a consistent projection.

Risks

- Expands the purpose of `readLocalRoots` (could be overloading the root-list surface).
- Requires careful versioning and client handling of the enlarged reply shape.

Why avoid presentation vocabulary

- The backend should expose authoritative enums and timestamps; presentation classification and friendly wording
  remain renderer responsibilities.

Renderer ownership afterward

- Renderer still owns the final presentational `SourceReadiness` projection (human-friendly messages, derived kinds),
  but it should source its primary authoritative inputs from `readLocalRoots` (enriched) rather than heuristics or
  event-only inference.

Option B — Add dedicated `readSourceLifecycle` (per-source read surface)
------------------------------------------------------------------

What it would own

- A new snapshot read surface keyed by `sourceId` or `rootId` returning an authoritative, backend-owned contract
  describing per-source lifecycle facts, for example:
  - `source_id`, `root_id`, `source_class`
  - `mount_status`, `access_state`, `access_issue_kind`
  - `scan_phase`, `scan_issue_kind`, `last_scan_*` timestamps
  - an optional whole-source `coverage_summary` (computed from `source_directories` counts) consisting of
    `coverage_state`, `recursive_scope_complete`, `empty_result_authoritative`, `detail`.

What it must NOT own

- It must not synthesize child rows, create pagination windows, or include UI-only labels. It must not subsume
  branch/contents read semantics.

Advantages

- A clear, single authoritative surface for per-source lifecycle facts. Cleaner separation of concerns and easier
  client contracts.

Risks

- Requires new protocol types and service methods; more work to implement and roll out.
- Needs careful update and invalidation semantics so lifecycle reads stay coherent with events and projection seeds.

Why avoid presentation vocabulary

- Expose raw enums and timestamps only; the renderer maps these to presentation kinds.

Renderer ownership afterward

- Renderer continues to own presentation mapping and any UX phrasing, but can stop duplicating lifecycle inference.
  Renderer should convert authoritative enums into the `SourceReadiness` kinds used by the UI.

Implementation acceptance bar (future contract must prove)
------------------------------------------------------
Future implementation must provide demonstrable evidence (tests + code) that:

1. Registered known source remains visible when filesystem resolution is unavailable (durable `is_user_visible` +
   read surface returns the source record).
2. `unavailable` / `blocked` / `failed` are emitted only from authoritative substrate facts (derived from
   `source_state` and `source_scan_state` and not guessed from events alone).
3. A `completed` scan does not synthesize children; completed-only state must not be used to fabricate rows.
4. `empty` is emitted only when an authoritative whole-source coverage calculation or per-window
   `emptyResultAuthoritative`
   proves there are zero rows.
5. Active scan state is represented distinctly from branch refresh; the service surface must return `scan_phase` and
   `scan_run_id` or equivalent without conflating refresh events.
6. Branch refresh semantics remain branch-owned and are not automatically promoted to source lifecycle changes.
7. Renderer no longer duplicates backend-owned lifecycle inference once the backend contract is available; the renderer
   consumes the new read surface as primary source of truth.

Docs / authority map
--------------------
This note is an inventory and contract gap. If the authority map is present, register this doc as an inventory /
contract-gap entry under source/scan contracts so implementers can find the gap and next steps.

Implementation prompt (for later work)
------------------------------------
If the team decides to implement a backend contract, the follow-up work should include:

- Protocol: add a new snapshot/read protocol type (or enrich `ReadLocalRootsReply`) and version it.
- Store read: add a substrate read in `crates/library-store-sqlite` to return the raw lifecycle fields and (optionally)
  whole-source coverage summary computed the same way `literal_hierarchy` does for whole-source anchors.
- Service: implement mapping in `snapshot_read_protocol.rs` and a handler in `service.rs` that returns the new type.
- Tests: add integration tests asserting the acceptance bar above (visibility under unavailability, authoritative
  empty, no synthesized children, separation of scan vs refresh).
- Renderer: migrate `deriveSourceReadiness` to prefer backend-owned raw fields for authoritative classification and
  reduce duplicated heuristics.

Do not implement changes in this pass. Use this note as the specification for the next implementation step.
