---
status: proposal
owner: product-architecture
review: required-before-implementation
---

# Waveform And Stems Current Baseline Discovery

## Current branch and HEAD observed

Observed on 2026-06-13:

| Field  | Value                                      |
| ------ | ------------------------------------------ |
| Branch | `dev`                                      |
| HEAD   | `66c0c2fb1f254022bbe9d99bf9d6b7bc23e6a4b2` |

## Actual files inspected

Required read-first files inspected:

- `README.md`
- `docs/product/product-doctrine.md`
- `docs/product/first-slice-substrate-map.md`
- `docs/decisions/substrate-implementation-discipline.md`
- `docs/library/source/root-scan-admission-contract.md`
- `docs/library/evidence/media-identity-schema-authority.md`
- `docs/library/evidence/playable-media-promotion-contract.md`
- `docs/library/identity/track-identity-candidate-contract.md`
- `docs/library/identity/track-identity-decision-contract.md`
- `crates/library-store-sqlite/migrations/20260502000000_substrate_baseline.sql`
- `crates/library-store-sqlite/src/schema/mod.rs`
- `crates/library-store-sqlite/src/schema/baseline.rs`
- `crates/library-boundary-service/src/source_maintenance.rs`
- `package.json`
- `Cargo.toml`

Additional files inspected to ground boundary, work, and delivery facts:

- `docs/library/source/maintenance-orchestration-contract.md`
- `docs/decisions/work-scheduling.md`
- `docs/decisions/electron-boundary-spine.md`
- `docs/library/source/background-root-scan-lifecycle-diagrams.md`
- `crates/library-domain/src/work.rs`
- `crates/library-domain/src/artifact.rs`
- `crates/library-boundary-protocol/src/contract.rs`
- `crates/library-boundary-protocol/src/commands/mod.rs`
- `crates/xtask/src/commands/boundary_contract.rs`
- `packages/library-boundary-contract/manifest.json`
- `crates/library-store-sqlite/src/store/work_items.rs`
- `crates/library-store-sqlite/src/store/artifacts.rs`
- `crates/library-store-sqlite/src/authority/work/work_items.rs`
- `crates/library-store-sqlite/src/authority/work/work_runs.rs`
- `crates/library-store-sqlite/src/authority/work/artifacts.rs`
- `apps/desktop/src/main/library/boundary/commandRegistry.ts`

## Search results summary

Repo searches excluded `node_modules`, `target`, and `.git`.

Present terms:

- `waveform`, `stem`, `stems`, `analysis`, `preparation`, and `prep` occur mostly in product doctrine, future
  architecture, historical salvage notes, and rejection/non-goal lists.
- `work_items`, `work_runs`, `work_artifacts`, `work_artifact_inline_payloads`,
  `work_artifact_file_store_entries`, and `work_artifact_claims` are current SQLite/schema/store vocabulary.
- `ipcMain.handle` is present in `apps/desktop/src/main/library/boundary/commandRegistry.ts` for current control-plane
  IPC handlers.

Absent exact or targeted concepts:

- `audio_component` / `audio_components`
- `analysis_work_items`, `analysis_work_runs`, `analysis_artifacts`
- `waveform_column`, `waveform_tile`, `waveform_delivery`
- `seqlok`, `seqlock`, `hot plane`
- `DeckAudioClockSnapshot`
- `custom protocol`, `protocol.handle`, `protocol.register`, `registerFileProtocol`, `registerSchemesAsPrivileged`
- `OffscreenCanvas`, `WebGPU`, `GPUCanvasContext`, `navigator.gpu`

## Actual current substrate tables relevant to waveform, stems, and analysis

The current baseline table set relevant to future waveform/stem/analysis work is:

| Table                                  | Current role                                                                                                                                  |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `source_files`                         | Durable source-relative file inventory with file kind/class, presence, size, mtime, and path ordering.                                        |
| `source_file_observations`             | Current accepted source-file evidence. It carries basis fields, optional BLAKE3 content hash, media probe fields, and `accepted_artifact_id`. |
| `content_attachments`                  | Content identity by BLAKE3 hash. One row per content hash.                                                                                    |
| `source_file_attachment_links`         | Occurrence link from a source file to an attachment. It stores `source_id` and `file_kind`, with triggers preventing source-id drift.         |
| `playable_media`                       | Evidence-backed playable-media row, currently audio-only, one row per attachment. It records an evidence source file and probe fields.        |
| `track_identity_candidates`            | Non-canonical exact playable-media content candidate groups keyed by current BLAKE3 evidence.                                                 |
| `track_identity_candidate_members`     | Candidate membership over `playable_media`, attachments, and evidence source files.                                                           |
| `track_identity_candidate_evidence`    | Candidate evidence provenance over playable media, attachment, source-file link, source file, source, BLAKE3, and probe artifact.             |
| `track_identity_decisions`             | Reversible/supersedable decisions over one candidate. Automatic decisions are exact-content decisions only.                                   |
| `track_identity_decision_source_scope` | Copied source-scope provenance for decision visibility.                                                                                       |
| `track_identity_decision_evidence`     | Copied decision evidence snapshots.                                                                                                           |
| `work_items`                           | Current narrow machine work queue over `source_file` and `projection_domain` subjects only.                                                   |
| `work_runs`                            | Current run records for claimed work items, with adapter key/version and outcome.                                                             |
| `work_artifacts`                       | Current artifact records for work runs, with narrow artifact kinds and inline/file-store storage.                                             |
| `work_artifact_inline_payloads`        | Inline artifact payload storage.                                                                                                              |
| `work_artifact_file_store_entries`     | File-store artifact payload metadata.                                                                                                         |
| `work_artifact_claims`                 | Active/released claims on artifacts.                                                                                                          |

Not present in the current baseline:

- canonical track rows
- durable audio component rows
- analysis-specific work tables
- waveform tables
- stem tables
- browser-row waveform fields
- preparation facet tables

## Current source/media identity substrate summary

The current source/media identity substrate is layered:

1. `source_files` record source-relative file occurrences and present/missing/removed state.
2. `source_file_observations` store accepted evidence from byte-reading work: current BLAKE3 hash evidence and current
   audio probe fields.
3. `content_attachments` materialize content identity from current BLAKE3 observations.
4. `source_file_attachment_links` connect source-file occurrences to attachments.
5. `playable_media` promotes current audio attachments only when current attachment identity and audio probe evidence
   exist.
6. `track_identity_candidates` group current playable-media evidence by exact BLAKE3 content, but are not tracks.
7. `track_identity_decisions` classify candidates under a decision source, but do not create semantic identity.

`playable_media` is the current V0 analysis input layer for full-mix waveform planning. `content_attachments` provide
the content identity basis, and source-file attachment links provide occurrence routes to bytes. Candidate and decision
rows may help product scoping and coalescing later, but they are not the ownership target for V0 waveform artifacts.

## Current boundary/protocol/generated-contract structure

The Rust protocol crate owns command and event DTOs in `crates/library-boundary-protocol`.

Current command families are:

- `LibraryBoundaryEvents`
- `LibraryRoots`
- `SourceFileHash`
- `SourceMaintenance`
- `TrackIdentityDecisions`
- `SnapshotRead`

`crates/library-boundary-protocol/src/contract.rs` generates the TypeScript contract and JSON schema. `crates/xtask`
exports/checks generated artifacts into `packages/library-boundary-contract`, whose manifest marks it
`generatedOnly: true`.

The desktop app routes current library control-plane calls through `ipcMain.handle` in
`apps/desktop/src/main/library/boundary/commandRegistry.ts`. The main process delegates to the library boundary host,
which uses the stdio transport/client path over the Rust boundary service. There is no current waveform, analysis
artifact, binary tile, or custom resource delivery command.

## Current work/artifact substrate reality

The current `work_*` substrate is real but narrow.

`work_items` currently admits:

- `subject_kind`: `source_file`, `projection_domain`
- `work_kind`: `inspect_source_file`, `rebuild_projection`
- `priority_class`: `urgent`, `interactive`, `background`
- `state`: `queued`, `leased`, `completed`, `blocked`, `failed`, `canceled`

It has `leased_until` but no lease owner, lease id, enqueue sequence, priority order, compute backend, target/facet key,
artifact dependency relation, output-artifact pointer, resource budget, or durable scheduler lane.

`work_runs` records the work item, adapter key/version, start/finish times, outcome, failure kind, and error detail.

`work_artifacts` currently admits:

- `subject_kind`: `source_file`, `projection_domain`
- `artifact_kind`: `inspection_result`, `projection_snapshot`
- `storage_kind`: `inline_payload`, `file_store`

Artifacts have media type, payload hash, adapter key/version, and basis fingerprint. There is no current artifact DAG,
artifact supersession model, artifact state model, format manifest, storage manifest beyond file-store root/path/bytes,
or analysis-specific artifact family.

`source_file_observations.accepted_artifact_id` references `work_artifacts`, so hash/probe observations already route
accepted evidence through the current artifact authority. That is the strongest reason to extend the current work/artifact
substrate instead of creating a parallel analysis authority.

## Whether any waveform, stem, or analysis contract already exists

No current waveform/stem/analysis contract exists in the active schema or generated boundary.

Current docs mention waveform, stems, and analysis as future architecture pressure, non-goals, historical salvage, or
rejection cases. `docs/docs-authority-map.md` explicitly frames `docs/decisions/library-preparation-substrate.md` as
historical future-architecture notes and says future preparation, analysis, and waveform work must be re-contracted from
the post-deletion substrate.

The current source-file observation and media-probe contracts explicitly say probing does not decode packets into
waveform buffers and does not generate analysis artifacts.

## Whether Seqlok/hot-plane substrate exists in this repo

No Seqlok/seqlock or hot-plane substrate was found. Exact searches for `seqlok`, `seqlock`, and `hot plane` returned no
results.

Future product docs mention Hot Table, but that is not a hot-plane runtime substrate and must not be treated as one.

## Whether Electron custom protocol infrastructure exists

No Electron custom protocol infrastructure was found.

Absent targeted infrastructure includes:

- `protocol.handle`
- `protocol.register`
- `registerFileProtocol`
- `registerSchemesAsPrivileged`
- exact `custom protocol` wording

Current Electron boundary infrastructure is IPC-based. `docs/decisions/electron-boundary-spine.md` reserves a future
`ResourcePlane` for waveform data, artwork, and analysis blobs, but it explicitly says large binary payload routing is
pending ResourcePlane definition. Current app code has no custom privileged app protocol.

## Current source maintenance limits and pipeline order

`crates/library-boundary-service/src/source_maintenance.rs` defines these bounded defaults:

| Phase                               | Default limit |
| ----------------------------------- | ------------- |
| BLAKE3 hash maintenance             | `8`           |
| Media probe maintenance             | `4`           |
| Attachment materialization          | `4`           |
| Attachment materialization maximum  | `128`         |
| Playable-media promotion            | `4`           |
| Track identity candidate production | `4`           |
| Track identity decision production  | `4`           |

The current maintenance order is:

1. Hash source files with BLAKE3.
2. Publish maintained snapshot invalidations.
3. Materialize attachments.
4. Publish maintained snapshot invalidations.
5. Probe source-file media.
6. Publish maintained snapshot invalidations.
7. Rebuild search/filter index for the source.
8. Promote playable media.
9. Publish maintained snapshot invalidations.
10. Produce track identity candidates.
11. Publish maintained snapshot invalidations.
12. Produce track identity decisions.
13. Publish maintained snapshot invalidations.

Runtime maintenance scheduling is in-memory service state. Pending source ids are deduped and drained in ascending
source-id order. Active source ids prevent duplicate same-source runs. There is no durable scheduler loop and no hidden
full-source drain.

## Stale assumptions from older prompts that must be replaced

Older planning must be replaced where it assumes:

- a primary-media-era target layer exists;
- source-file evidence rows are facts rather than current observations;
- exact-content candidates are canonical tracks;
- automatic exact-content decisions are semantic identity;
- waveform ownership can be attached to browser rows, prep rows, asset rows, or source-file occurrences;
- source traversal is the same as analysis;
- renderer IPC can carry large binary payloads;
- Seqlok/hot-plane runtime substrates already exist;
- Electron custom protocol delivery already exists;
- prep rows are a current target;
- analysis-specific work tables exist.

## Stale terminology translation

This table is only a stale-terminology map. The right column is the vocabulary later prompts should use.

| Stale term                                       | Current repo translation                                                                                                                                                               |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `primary_media_candidate_id`                     | Use `playable_media_id` for the V0 full-mix analysis subject, with `attachment_id` and BLAKE3 basis as provenance.                                                                     |
| `primary_media_candidate_id + attachment_id`     | Use `playable_media_id` as the subject plus `attachment_id` as content identity provenance. Occurrence byte access comes through `source_file_attachment_links`, not target ownership. |
| `primaryMedia`                                   | Use `playableMedia` only when naming generated/boundary row shapes, and `playable_media` for schema/store vocabulary.                                                                  |
| `primary_media_facts`                            | Use `source_file_observations` for accepted evidence and `content_attachments` / `source_file_attachment_links` for attachment identity.                                               |
| `SourceFacts`                                    | Use `source_file_observations`; they are observations with a basis, not timeless facts.                                                                                                |
| `source_file_facts`                              | Use `source_file_observations`.                                                                                                                                                        |
| `prep / preparation`                             | Use future preparation facets only after a new contract. Do not create prep rows in the waveform arc.                                                                                  |
| `library asset / browser row waveform ownership` | Use backend-owned analysis artifacts targeted at `playable_media`; browser rows only project summaries after contracts exist.                                                          |

## Analysis and waveform target vocabulary decision

### 1. V0 current input evidence layer for full-mix waveform analysis

The V0 current input evidence layer is `playable_media`, backed by:

- `content_attachments` for BLAKE3 content identity;
- `source_file_attachment_links` and `source_files` for current source-file occurrences that can provide bytes;
- `source_file_observations` for the current BLAKE3 and audio probe basis;
- `work_artifacts` for accepted hash/probe evidence provenance.

`track_identity_candidates` and `track_identity_decisions` are not the V0 waveform target. They may later help decide
which product-facing rows request or share analysis, but they are not canonical track identity and must not own waveform
artifacts.

### 2. Replacement for the old two-field target

The old two-field target named in the stale terminology table is replaced by:

- subject: `playable_media`
- subject id: `playable_media_id`
- content provenance: `attachment_id`, `content_hash_algorithm = blake3`, `content_hash_value`
- byte-access provenance: selected `source_file_attachment_link_id` and `source_file_id` used for the run
- basis: a future explicit waveform basis fingerprint derived from the current playable-media, attachment, source-file
  observation, decoder, and policy inputs

The selected source-file occurrence is runtime/provenance for reading bytes. It is not the owner of the artifact.

### 3. V0 full-mix waveform target name for later prompts

Later prompts should call the V0 full-mix waveform target:

- `playable_media_full_mix_waveform_target_v0`

Decision-level fields should use:

- `waveform_subject_kind = playable_media`
- `waveform_subject_id = playable_media_id`
- `waveform_component_role = full_mix`
- `waveform_basis_fingerprint`
- `waveform_artifact_kind = waveform_tile_set`

These names are planning vocabulary until a reviewed contract freezes them. They are not current schema.

### 4. Correct future role of audio components

The repo is not ready for a durable audio component table now.

For this arc, audio components should be a future derived contract layer above `playable_media` and track-identity
decision context, not a new durable table. The minimal future-safe placeholder vocabulary is:

- `waveform_component_role = full_mix` for V0 full-mix artifacts;
- future `waveform_component_role = stem:<stem_key>` only after a stem alignment/playback contract exists;
- `component_basis_fingerprint` as future planning vocabulary for the evidence and transformation basis.

Do not introduce durable component rows just because waveform and stems want a shared abstraction. A durable component
model must wait until the repo has a reviewed stem/alignment contract and a clear relation to playback bundles,
analysis artifacts, and identity decisions.

### 5. Preserving occurrence-provided bytes without occurrence ownership

Source bytes are occurrence-provided: an analyzer must read from an available source-file occurrence linked to the
target attachment. The waveform artifact owner is still `playable_media_id` plus `waveform_component_role = full_mix`
and its basis fingerprint.

If one occurrence disappears, another current occurrence for the same attachment can provide bytes for recomputation.
That keeps occurrence availability out of artifact ownership while preserving exact provenance for each run.

### 6. Avoiding accidental canonical track identity

The target does not use a candidate id or decision id as the artifact subject. A candidate can be stale, a decision can
be automatic, and neither means semantic track identity.

Later prompts must not create a canonical track table, treat candidates as tracks, or treat automatic exact-content
decisions as user-accepted semantic identity. Waveform artifacts can be shared or discovered through exact attachment
identity, but that is byte identity and analysis basis reuse, not track identity.

### 7. Exact names later prompts should use

Use these names in the next implementation-safe prompts:

- `playable_media`
- `playableMedia`
- `playable_media_id`
- `attachment_id`
- `content_attachments`
- `source_file_attachment_links`
- `source_file_observations`
- `work_items`
- `work_runs`
- `work_artifacts`
- `waveform_subject_kind = playable_media`
- `waveform_component_role = full_mix`
- `playable_media_full_mix_waveform_target_v0`
- `waveform_basis_fingerprint`
- `waveform_artifact_kind = waveform_tile_set`

Do not use stale primary-media-era target names as doctrine vocabulary.

## Stop conditions before implementation

Stop before implementation if any of these are true:

- Human review has not approved the target vocabulary and work/artifact substrate direction.
- A prompt would add migrations before the substrate decision is reviewed.
- A prompt would add generated contract output before the Rust protocol shape is reviewed.
- A prompt would add renderer UI, analyzer code, stem playback code, or protocol handlers in the same slice as schema
  design.
- A prompt would create prep rows, browser-row waveform fields, legacy asset rows, or source-occurrence-owned waveform
  fields.
- A prompt would create canonical track identity or treat current candidate/decision rows as canonical tracks.
- A prompt needs Seqlok/hot-plane or custom protocol infrastructure but the current repo still lacks those substrates.
- A prompt tries to route binary waveform tiles through normal JSON IPC.
- Current schema files contradict a planning document. Trust the current schema and stop for a new discovery pass.
