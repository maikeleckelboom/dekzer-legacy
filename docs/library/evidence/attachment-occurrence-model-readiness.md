---
status: implemented-review
last-reviewed: 2026-06-12
owner: library-substrate-boundary
canonical-context:
  - attachment-identity-contract
  - media-probe-observations-contract
  - source-integrity-read-model-contract
  - media-identity-schema-authority
scope:
  - attachment-occurrence-evidence
  - occurrence-readiness
  - source-file-occurrence-status
---

# Attachment Occurrence Model Readiness

## Verdict

A-5 is implemented as a narrow substrate read-model slice and ready for substrate review.

The implementation should not add a durable occurrence table. Current durable occurrence observations already exist as
`source_files` rows linked to `content_attachments` by `source_file_attachment_links`. A-5 should make those observations
readable with source/path/status evidence and deterministic bounds.

Primary motivation: local library correctness. The same audio/content may appear in multiple places because users copy
folders, attach backup drives, rotate external drives, or temporarily lose access to known sources. A-5 must preserve
offline occurrences, keep unavailable from collapsing into absent, expose relocation candidates as evidence only, and
avoid unsafe claims about what content represents or what should happen to it.

Secondary motivation: future room compatibility. Prepared Room, Performed Room, and RT Flight Deck references will later
need identity, availability, provenance, source isolation, stable references, and evidence/decision boundaries. Those
future concepts are architectural compatibility targets, not A-5 product scope.

## Repo Evidence

- `docs/library/evidence/attachment-identity-contract.md` is accepted and states that
  `source_file_attachment_links` is the canonical source-file occurrence table for the attachment identity pass.
- `docs/library/evidence/media-probe-observations-contract.md` is accepted and stores probe observations in
  `source_file_observations`, not on attachments or tracks.
- `docs/library/health/source-integrity-read-model-contract.md` is accepted and documents the source-scoped
  `readSourceIntegrity` boundary.
- `crates/library-store-sqlite/migrations/20260502000000_substrate_baseline.sql` contains `source_files`,
  `content_attachments`, and `source_file_attachment_links` with indexes on `source_file_id` and `attachment_id`.
- `crates/library-store-sqlite/src/read_models/attachment_identity.rs` already reads attachment-to-source-file links
  with computed current/stale link status.

## Occurrence Meaning

An occurrence is a substrate observation, not a product or library-management decision.

A source-file occurrence is the existing `source_files` row: the durable source-relative inventory observation with source id,
relative path, file kind/class, size, mtime, and presence state.

An attachment occurrence is that source-file occurrence when it is connected to a durable content identity through
`source_file_attachment_links.attachment_id -> content_attachments.attachment_id`.

A-5 occurrence grouping is a derived read over those existing observations:

- `content_attachments` owns exact BLAKE3 content identity.
- `source_file_attachment_links` owns the current materialized source-file-to-attachment relation.
- `source_files` owns the source-relative path and presence observations for the occurrence.
- source lifecycle and source integrity own availability and health context.

A-5 is therefore a read boundary and derived read query. It is not a new durable row, not a materialized projection, and
not a second source-file inventory model.

## Ownership Map

| Concept | Current durable owner | A-5 stance |
| --- | --- | --- |
| Source registration | `sources`, `source_state`, `SourcesAuthorityTx`, and the `RegisterLocalRoot` boundary | Reuse only. Do not create occurrence-owned source records. |
| Source locator/path identity | `source_locators`, `SourceLocatorsAuthorityTx`, source lifecycle reads, and `source_locations` for accepted sub-roots | Reuse source and path evidence. Do not infer relocation acceptance. |
| Source file inventory | `source_files`, `source_directories`, `SourceFilesAuthorityTx`, scan/finalization code | A source-file occurrence is this row. |
| Attachment/content identity | `content_attachments`, `source_file_attachment_links`, and `materialize_attachments_for_source` | Group by `attachment_id` / BLAKE3 content identity. |
| Probe observations | `source_file_observations` plus accepted inspection artifacts and media probe maintenance | Expose only as evidence availability if needed. Do not move probe observations onto occurrences. |
| Source health/integrity | `readSourceIntegrity`, source lifecycle, scan coverage, source maintenance snapshot | Use to mark unavailable, missing, blocked, stale, or incomplete evidence without hiding rows. |
| Occurrence grouping | Derived query over `source_file_attachment_links`, `content_attachments`, and `source_files` | No durable owner beyond existing tables. |
| Occurrence read model | Existing attachment identity read path, especially `readAttachmentSourceFiles` | Extend in place for occurrence evidence rather than adding occurrence-interpretation commands. |
| Future user decisions | A-6 user decision pattern. Existing track identity decisions govern candidates only. | Absent from A-5. Do not write decision rows. |
| Renderer projection | Existing desktop attachment identity forwarding only | Renderer may project later but must not own occurrence authority. |

## Evidence Versus Decision

A-5 may expose evidence that the same attachment/content identity appears through multiple source-file occurrences.

A-5 must not decide:

- safe deletion;
- preferred copy;
- relocation acceptance;
- merge;
- split;
- canonical track identity;
- source-file preference;
- destructive action.

Rows returned by A-5 are evidence rows. Any future action or preference needs a separate user decision record and must
not be encoded into occurrence status.

## Interpretation Vocabulary

These are possible interpretations over occurrence evidence. They are not product decisions and must not appear as
action recommendations.

| Interpretation | Evidence-only definition |
| --- | --- |
| exact duplicate | Multiple available occurrences with the same attachment/content identity. |
| relocation candidate | One or more unavailable prior occurrences and one or more available occurrences with matching identity, where path/source context may suggest movement. |
| offline occurrence | Occurrence is known but currently unavailable because the source, locator, or path is unavailable. |
| backup/copy occurrence | Available occurrence with the same identity in a different source or backup-like location, without deleting, preferring, or merging either occurrence. |

The read boundary should expose enough evidence for these interpretations, but it should not return product commands
such as "delete", "merge", "accept relocation", or "prefer".

## Source Health Contribution

Source Integrity / Collection Health affects occurrence reads by adding availability and trust context. It does not own
occurrence identity.

Unavailable must not collapse into absent:

- a known source that is unmounted, missing, blocked, or otherwise unavailable still contributes known occurrence rows;
- a source-file row with `presence_state = 'missing'` or `presence_state = 'removed'` is not the same as no row;
- a stale attachment link is evidence of a previously materialized relation and must be reported as stale evidence, not
  filtered away by default;
- source-level blocked/missing/unavailable states must be represented as occurrence status or source status fields.

A-5 reads should combine separate status evidence rather than reduce everything to one healthy/unhealthy boolean:

- attachment link status: current or stale;
- source-file presence state: present, missing, or removed;
- source availability state: reuse the source lifecycle / `SourceIntegrityAvailabilityState` vocabulary where the
  boundary needs a public enum;
- source coverage state when needed to explain incomplete source evidence.

## Minimum Read Boundary

The minimum backend boundary is the attachment-to-source-file evidence read, extended in place:

- Current command owner: `readAttachmentSourceFiles`.
- Required identity parameter: `attachmentId`.
- Optional bound: `limit`, preserving the existing backend limit validation.
- Pagination: if A-5 needs pageable results beyond the existing one-page bound, add an opaque backend-issued `cursor`
  and `nextCursor`. Cursor identity must include version, `attachmentId`, page size, and ordered position. It must use
  keyset order over `(source_id, source_file_id)` and reject mismatched cursors explicitly.
- Sort: deterministic by `source_id ASC, source_file_id ASC` unless a later contract replaces the complete cursor
  identity.

Returned occurrence evidence fields should include:

- attachment identity: `attachmentId`, `contentHashAlgorithm`, `contentHashValue`;
- source-file identity: `sourceFileId`, `sourceId`, and, if exposed, `sourceFileAttachmentLinkId` for provenance;
- source/path fields: source display name or class when already available, source-file `relativePath`, `name`, and
  `parentSourceDirectoryId` if useful for navigation;
- inventory fields: `fileKind`, `fileClass`, `presenceState`, size, mtime, and updated timestamps;
- evidence fields: attachment link status, link timestamps, current BLAKE3 observation availability, and probe availability
  summary if needed;
- status fields: source availability, source coverage/integrity summary when needed, and stale-evidence state;
- counts: total occurrences, current-link occurrences, stale-link occurrences, available occurrences, unavailable known
  occurrences, missing occurrences, blocked occurrences, removed occurrences, distinct source count, effective limit,
  and remaining count.

Implemented A-5 shape:

- `readAttachmentSourceFiles` remains the single attachment-to-source-file occurrence read.
- Each `sourceFileLinks[]` row is one linked `source_files` row and includes `sourceFileAttachmentLinkId`,
  `sourceFileId`, `sourceId`, source display/class context, source-file `name`, `relativePath`,
  `parentSourceDirectoryId`, `sizeBytes`, `mtimeNs`, `fileKind`, `fileClass`, `presenceState`, lifecycle-derived
  `sourceMountStatus`, `sourceAccessState`, `sourceAccessIssueKind`, `sourceScanPhase`, `sourceAvailabilityState`,
  `hasCurrentBlake3Observation`, `linkStatus`, and derived `occurrenceStatus`.
- `sourceAvailabilityState` reuses `SourceIntegrityAvailabilityState` vocabulary but is locally derived in this read
  from persisted lifecycle and scan-state observations. It is not a nested `readSourceIntegrity` result and must not imply
  source-failure or maintenance evaluation.
- `occurrenceStatus` is source/path availability only: `available`, `sourceUnavailable`, `sourceMissing`,
  `sourceBlocked`, `fileMissing`, `fileRemoved`, or `unknown`. Content-evidence freshness remains separate in
  `linkStatus`.
- The reply includes a derived `summary` with total, available, unavailable, current-link, stale-link, distinct-source,
  and multiple-occurrence counts.
- `unavailableOccurrenceCount` is the non-available occurrence count over all linked rows. `distinctSourceCount` counts
  distinct `source_id` values only. `hasMultipleOccurrences` means multiple linked source-file rows, not a duplicate-song
  or preferred-copy decision.
- The read preserves the existing bounded one-page limit contract (`effectiveLimit` plus `remainingSourceFileLinks`);
  it does not introduce cursor pagination in this slice.

Availability limitation: A-5 does not perform filesystem access checks. Source availability is the strongest honest
state available from persisted `source_state` / `source_scan_state` observations plus `source_files.presence_state`. If
lifecycle state is absent or stale, the read reports the persisted status rather than collapsing the occurrence row into
absence.

The boundary must not expose product-facing track-equivalence terms, destructive-action terms, movement-confirmation
terms, or preferred-copy terms.

Invalidation scope:

- A-5 is an explicit snapshot read in the first implementation.
- It must not attach occurrence truth to navigation invalidations.
- The read is invalidated by writes to source lifecycle/state, source-file inventory, `source_file_observations`, `content_attachments`,
  or `source_file_attachment_links`.
- A maintained occurrence invalidation scope is not required for the substrate slice. If a future UI needs live
  subscription semantics, add a dedicated source/attachment-scoped invalidation contract in a later slice.

## Schema Stance

A-5 needs:

- no new durable table;
- no materialized read model;
- a derived read query over existing tables;
- no schema change for correctness in the first implementation.

Justification:

- `source_files` already owns occurrence identity and source-relative path observations.
- `source_file_attachment_links` already owns the source-file-to-attachment relation and has an index on
  `attachment_id`.
- `content_attachments` already owns BLAKE3 content identity and uniqueness.
- Source health is already read from source lifecycle / source integrity; copying it into an occurrence table would
  duplicate authority and create staleness.

A composite index such as `(attachment_id, source_id, source_file_id)` is allowed only if the implementation adds cursor
pagination and query-plan evidence shows the current `attachment_id` index is insufficient. That would be a performance
index, not a new occurrence representation.

## Schema Guard

Do not introduce a second occurrence authority, aliases, compatibility wrappers, or duplicate read paths for A-5.
Attachment occurrence evidence must stay a derived read over `source_file_attachment_links`, `content_attachments`,
`source_files`, source lifecycle state, and current `source_file_observations`.

Current non-occurrence tables keep their own roles:

- `playable_media`: playable-media record; one row per attachment in v0, not per source-file
  occurrence.
- `track_identity_candidates`, `track_identity_candidate_members`, and `track_identity_candidate_evidence`: exact
  playable-media observation grouping; not A-5 occurrence grouping.
- `track_identity_decisions` and related decision evidence tables: candidate decision authority; not occurrence
  preference or relocation decisions.

## Non-Goals

A-5 explicitly rejects:

- UI work;
- renderer-owned occurrence authority;
- track-level same-song semantics;
- track identity;
- automatic removal or merge;
- source-file preference;
- relocation confirmation;
- future room reference handling;
- CUE association;
- broad search/filter polish.

## Implementation Acceptance Bar

Smallest safe implementation slice:

- `crates/library-store-sqlite/src/read_models/attachment_identity.rs`: enrich the existing attachment-to-source-file
  read query with source-file path/status/source health evidence.
- `crates/library-store-sqlite/src/store/attachment_identity_reads.rs`: keep the store read as a read-only wrapper.
- `crates/library-boundary-protocol/src/commands/snapshot_reads.rs`: update Rust protocol types and tests, then generate
  the TypeScript contract through the repo command.
- `crates/library-boundary-service/src/service.rs` and
  `crates/library-boundary-service/src/snapshot_read_protocol.rs`: validate input and map store rows without running
  maintenance.
- `packages/library-boundary-client` and `apps/desktop/src/**/attachmentIdentity`: update generated-consumer adapters
  only as needed for the changed generated contract. Do not hand-edit generated contract files.

Minimal tests:

- store read returns multiple current present source-file occurrences for one attachment with deterministic order and
  counts;
- store read returns missing/offline/blocked known occurrences instead of filtering them out;
- stale attachment links remain visible as stale evidence;
- boundary service read does not hash, probe, materialize, promote, publish invalidations, or run maintenance;
- protocol serialization covers any new fields and cursor-invalid behavior if cursor pagination is added;
- existing desktop forwarding tests are updated only to preserve boundary forwarding, not UI behavior.

Validation command for the implementation slice:

```powershell
cargo test -p library-store-sqlite attachment_identity
cargo test -p library-boundary-protocol attachment
cargo test -p library-boundary-service attachment
pnpm run library:contract:check
```

Stop conditions:

- stop if implementation cannot preserve unavailable/missing occurrence rows without adding a second durable occurrence
  table;
- stop if attachment identity or source-file inventory cannot supply the grouping evidence;
- stop if the read starts needing preferred-copy, merge, relocation acceptance, or track identity semantics;
- stop if the renderer must own grouping authority;
- stop if generated contract files would need hand edits.
