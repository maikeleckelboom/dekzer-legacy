---
status: accepted
last-reviewed: 2026-06-03
owner: library-substrate-boundary
canonical-context:
  - source-lifecycle-backend-contract-gap
  - media-relevant-file-inventory-contract
  - library-contents-read-boundary
  - primary-media-promotion-contract
  - track-identity-candidate-contract
  - track-identity-decision-contract
  - track-identity-decision-authority-contract
scope:
  - asset-identity
  - primary-media-authority
  - cue-association
  - content-hash-placement
  - track-identity-decisions
  - deferred-preparation-surfaces
---

# Media Identity Schema Authority

## Purpose

This document owns the current authority decision for media identity surfaces that sit beyond source lifecycle and
source-file inventory. It decides what is canonical now, what is intentionally dormant, and what was stale enough to
remove before observed file facts are broadened.

Reading order:

1. `docs/library/source-lifecycle-backend-contract-gap.md` owns source lifecycle and source readiness surfaces.
2. `docs/library/media-relevant-file-inventory-contract.md` owns source-file inventory, contents browse policy, CUE
   inventory admission, image-file inventory behavior, and explicit inventory admission.
3. This document owns asset identity, `primaryMedia`, CUE association, hashing placement, and deferred preparation
   surfaces.

## Current Canonical Surfaces

| Surface                                                                                                  | Authority                                                                                                                                                             | Current role                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| -------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `source_files`                                                                                           | SQLite + `SourceFilesAuthorityTx`                                                                                                                                     | Canonical attachment inventory row for observed source files. Classification is path-derived and provisional.                                                                                                                                                                                                                                                                                                                                                          |
| `readContents` default policy                                                                            | Renderer boundary + service/store read model                                                                                                                          | Current product contents path: recursive `{ kind: 'audioBrowse' }` rows for audio-only V0 main contents browse.                                                                                                                                                                                                                                                                                                                                                        |
| `SourceFacts` observed-file evidence                                                                     | Observed source facts substrate                                                                                                                                       | Optional accepted source-file evidence attached to a `source_file_id` and copied file basis. Current evidence includes BLAKE3 content hash and audio media probe facts. It is not attachment identity and is not required by default contents reads.                                                                                                                                                                                                                   |
| `content_attachments` / `source_file_attachment_links`                                                   | SQLite + `SqliteDurableStore::materialize_attachments_for_source` called by bounded service maintenance                                                               | Current Rust/store/service attachment identity foundation from current BLAKE3 `SourceFacts` evidence. Not track identity, not `primaryMedia`, not CUE pairing. Exposed only through the narrow attachment identity read boundary.                                                                                                                                                                                                                                      |
| `primary_media_candidates`                                                                               | SQLite + `SqliteDurableStore::promote_primary_media_for_source` called by bounded service maintenance                                                                 | Current evidence-backed primary-media v0 projection target. One row per attachment with current BLAKE3 attachment identity and current audio probe evidence. Not canonical track identity, not browser-row authority, not CUE pairing.                                                                                                                                                                                                                                 |
| `track_identity_candidates` / `track_identity_candidate_members` / `track_identity_candidate_evidence`   | SQLite + `SqliteDurableStore::produce_track_identity_candidates_for_source` called by bounded service maintenance                                                     | Current exact evidence candidate foundation from current `primary_media_candidates`. Groups exact current BLAKE3 primary-media content evidence with provenance. Not canonical track identity, not user identity, not semantic recording matching.                                                                                                                                                                                                                     |
| `track_identity_decisions` / `track_identity_decision_evidence` / `track_identity_decision_source_scope` | SQLite + `SqliteDurableStore::produce_track_identity_decisions_for_source` called by bounded service maintenance, plus explicit accept/reject/defer decision commands | Current backend-owned decision authority over candidates. System maintenance may accept active exact-content candidates. Explicit user commands may accept, reject, or defer one candidate. Effective-decision precedence is backend-owned and candidate-level. Decision evidence is copied provenance, not live cascade authority. Decision source scope is copied provenance used for source-scoped visibility, never caller-supplied. Not canonical track identity. |
| `readContents` `primaryMedia` policy                                                                     | Store read model + boundary protocol                                                                                                                                  | Current evidence-backed primary-media row profile. Reads only promoted `primary_media_candidates` revalidated against current scoped source files, attachment links, content attachments, and `SourceFacts`. No source-file fallback.                                                                                                                                                                                                                                  |
| `source_media` write/read guards                                                                         | Store filesystem guard                                                                                                                                                | Current guardrail for source-media read-only operations. CUE parsing operation names are reserved, not current parsing.                                                                                                                                                                                                                                                                                                                                                |
| `Playlists` / `PlaylistEntries`                                                                          | Boundary service + store                                                                                                                                              | Live service/protocol/store surface. Desktop Main/Preload do not expose playlist writes yet. Playlist membership still targets `library_asset_id`.                                                                                                                                                                                                                                                                                                                     |

The default desktop renderer path does not request `primaryMedia`. `apps/desktop/src/renderer/library/boundary/contentsRead.ts`
requests `{ kind: 'audioBrowse' }` for the V0 main contents browse.

## Intentionally Dormant Or Transitional Surfaces

| Surface                                                                                                           | Status                                                                 | Blocker / owner                                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LibraryAssets`                                                                                                   | Live transitional substrate, not current attachment identity authority | `equivalence_fingerprint` is caller-provided and remains non-authoritative for content identity. Current attachment identity uses `content_attachments`. |
| `LibraryAssetAttachments` / `SourceSegmentSets` / `SourceSegments`                                                | Dormant future segment/promotion shape                                 | Segment promotion remains separate from BLAKE3 attachment identity. These tables are not used by attachment materialization.                             |
| `LibraryBrowserRows`                                                                                              | Live maintained projection over `LibraryAssets`                        | Used by library browser and playlist-scoped browser reads. Not current default contents authority and not `primaryMedia` v0 authority.                   |
| Waveform, stems, prep readiness summaries                                                                         | Live generated/service/store projection fields, dormant in desktop UI  | Future preparation owner. They summarize capability/projection state and are not canonical media identity.                                               |
| `CapabilitySpecs`, `CapabilityDependencies`, `PrepPolicies`, `PrepAssignments`, `ResolvedLibraryAssetPrepTargets` | Live preparation substrate                                             | Scheduler/prep substrate exists, but observed facts and attachment identity are not complete enough to make it product authority.                        |

## Delete-Now Surfaces Removed

The uncompiled `crates/library-store-sqlite/src/authority/media/*` scaffold was removed in this pass. It referenced
legacy tables such as `media_assets`, `file_media_latest`, `releases`, `media_sets`, and `entity_asset_links` that are
not present in the current baseline schema and was not included from `authority/mod.rs`. Its local SHA-256 inspection
code was therefore not current product hashing authority.

## `primaryMedia` Decision

Decision: **current evidence-backed v0 projection.**

Evidence:

- Store read model support exists in `read_models/contents.rs` and reads `primary_media_candidates`.
- Boundary protocol and generated TypeScript expose `ContentsReadPolicy.primaryMedia` and `PrimaryMediaSummary`.
- Desktop Main validates and maps the policy, while the renderer default requests `audioBrowse`.
- `primaryMedia` rows are present-file scoped and reject image/unsupported media classes.
- Promoted rows depend on current `primary_media_candidates`, `source_file_attachment_links`, `content_attachments`, and
  current `SourceFacts`.
- Unpromoted source-file fallback rows are intentionally removed. Plain scanned source files do not surface as
  `primaryMedia`.

Therefore `primaryMedia` must not be treated as current track identity. It is now a narrow playable-media candidate
projection backed by current attachment identity and audio probe evidence. The detailed eligibility and non-goals are
owned by `docs/library/primary-media-promotion-contract.md`.

## Track Identity Candidate Decision

Decision: **current evidence-backed candidate foundation, non-canonical.**

Evidence:

- Store schema contains `track_identity_candidates`, `track_identity_candidate_members`, and
  `track_identity_candidate_evidence`.
- Store production is backend-owned, source-scoped, and bounded.
- Production consumes only current `primary_media_candidates` rows revalidated against current source files, attachment
  links, `content_attachments`, BLAKE3 `SourceFacts`, and audio probe facts.
- Duplicate current exact BLAKE3 primary-media evidence groups into one active candidate with preserved source-file and
  attachment provenance.

Therefore track identity candidates may say only that current evidence-backed primary-media candidates appear equivalent
by exact content evidence. They must not create canonical tracks, user decisions, CUE associations, metadata
reconciliation, prep surfaces, playlist/crate/sleeve rows, waveform/stem authority, or renderer-owned grouping. The
detailed status, grouping, and stale/current rules are owned by
`docs/library/track-identity-candidate-contract.md`.

## Track Identity Decision Foundation

Decision: **current backend-owned decision foundation over candidates, non-canonical.**

Evidence:

- Store schema contains `track_identity_decisions`, `track_identity_decision_evidence`, and
  `track_identity_decision_source_scope`.
- V0 decision production is backend-owned, source-scoped, and bounded.
- Production creates current `accepted` decisions with `decision_source = system_exact_content_v0`.
- Explicit decision commands create `accepted`, `rejected`, or `deferred` user decisions with
  `decision_source = user_local_v0`.
- Decision evidence snapshots retain copied provenance ids and do not cascade from live candidate, evidence, source, or
  attachment rows.
- The read model exposes backend-owned effective-decision precedence: current user decisions win over system decisions,
  and current user reject/defer blocks system accept from being effective.
- Production consumes only `active` exact-content candidates with current candidate evidence and skips candidates that
  already have a current system exact-content decision or a current user blocking decision.
- Decision evidence snapshots preserve candidate, member, candidate evidence, primary-media candidate, attachment,
  source-file link, source-file, BLAKE3, basis fingerprint, and probe artifact provenance.

Therefore track identity decisions may say only that a backend-owned decision source accepted, rejected, deferred, or
superseded a candidate under a recorded basis. The v0 automatic accepted decision means accepted exact-content candidate;
it must not be read as canonical track identity, same-song semantic identity, metadata reconciliation, CUE association,
prep readiness, playlist membership, waveform/stem authority, artwork intelligence, or renderer-owned grouping. The
detailed status, provenance, and supersession rules are owned by
`docs/library/track-identity-decision-contract.md`.

## CUE Association Decision

CUE files are current source-file inventory rows only: `fileKind = cueSheet`, `mediaClass = unsupported`.

Future CUE model:

- The CUE source-file row owns future CUE parse observations.
- The audio source-file row owns future audio/container/probe observations.
- CUE-to-audio pairing is future evidence/association work.
- Pairing must not be inferred from path proximity.
- CUE parse observations must not be attached to a FLAC/audio row by convenience.
- Future association must represent ambiguity, missing audio files, multiple candidate audio files, and user
  correction.

No CUE parsing, FLAC/CUE pairing, or track split inference is implemented by this decision.

## `equivalence_fingerprint` Decision

`LibraryAssets.equivalence_fingerprint` is a live uniqueness key for minting or reusing `LibraryAssets`, but its
derivation is not owned by the table or authority method. `LibraryAssetsAuthorityTx::mint_or_reuse_library_asset`
accepts an opaque string and stores it as-is. `ResolveLibraryAssetPromotionTx` forwards the same caller-provided value
and also uses it as a projection rebuild basis.

Attachment identity does not read or write `equivalence_fingerprint`. It is not copied into `content_attachments`, does
not satisfy BLAKE3 evidence, and is not a content identity authority after the attachment foundation pass.

Answers:

| Question                                        | Decision                                                                         |
| ----------------------------------------------- | -------------------------------------------------------------------------------- |
| What creates it?                                | Callers of `ResolveLibraryAssetPromotionInput` / `MintOrReuseLibraryAssetInput`. |
| What inputs derive it?                          | Not specified by current authority; tests use values such as `eq:track-a`.       |
| Is it bytes-derived?                            | Not proven.                                                                      |
| Is an algorithm named?                          | No.                                                                              |
| Stable across moves?                            | Not guaranteed.                                                                  |
| Stable across metadata-only filesystem changes? | Not guaranteed.                                                                  |
| Stable across same-content duplicates?          | Not guaranteed.                                                                  |
| Current product authority?                      | Live transitional asset key, not attachment identity.                            |
| Can it support attachment identity?             | No. It is opaque and not proven to be durable bytes-derived evidence.            |

## Content Hash / BLAKE3 Placement

`SourceFacts` stores optional `content_hash_algorithm` and `content_hash_value` evidence. Current tests and fixtures
use explicit `sha256` examples, and the store now also owns a narrow BLAKE3 source-file hash evidence job.

Current placement:

- Content hashing belongs to observed file facts / file evidence because hashing reads bytes.
- The BLAKE3 job lives in `library-store-sqlite`. Production admission resolves filesystem paths in the backend from
  durable source state plus `source_files.relative_path`, then streams bytes and commits through the inspect-source
  artifact plus `SourceFacts` authority path.
- The current root path authority is `source_state.effective_path`, falling back to `source_locators.absolute_path` for
  absolute-path sources. Source-location subpath-scoped admission remains future; source-file path resolution itself is
  root plus `source_files.relative_path`.
- BLAKE3 evidence is stored as `content_hash_algorithm = 'blake3'` with a lowercase hex digest value.
- A basis change between the pre-hash source-file read and pre-commit source-file read rejects the commit.
- Admission is bounded, deterministic, and limited to present media-relevant source-file inventory: audio, video, image,
  and CUE sheet rows. Current BLAKE3 facts are skipped; missing, stale, absent, or non-BLAKE3 hash facts are candidates.
- Attachment identity now consumes current BLAKE3 durable hash evidence through the store-owned
  `materialize_attachments_for_source(source_id, limit)` path, called by one bounded service-owned maintenance unit
  after scan-triggered or manual source hash maintenance.
- Primary-media promotion consumes current attachment identity plus current audio probe evidence through
  `promote_primary_media_for_source(source_id, limit)`, called by the same bounded service-owned maintenance unit after
  hashing, attachment materialization, and probing.
- Track identity candidate production consumes current primary-media candidates through
  `produce_track_identity_candidates_for_source(source_id, limit)`, called by the same bounded service-owned maintenance
  unit after primary-media promotion. The v0 evidence key is exact BLAKE3 content evidence and remains a reversible
  candidate grouping, not canonical track identity.
- Track identity decision production consumes active exact-content candidates through
  `produce_track_identity_decisions_for_source(source_id, limit)`, called by the same bounded service-owned maintenance
  unit after candidate production. The v0 system decision source is `system_exact_content_v0` and remains a reversible
  or supersedable decision record, not canonical track identity. Explicit user decision commands use `user_local_v0` and
  supersede prior current user decisions for the same candidate without letting maintenance override user reject/defer.
  Decision evidence snapshots retain copied provenance ids and do not cascade from live candidate/evidence/source rows.
- `content_attachments` owns attachment hash authority. `source_file_attachment_links` stores the source-file occurrence
  relation and derives exposed link hash values from the joined attachment row; it does not store a duplicate hash copy.
- Track identity must not rely on path identity.
- Attachment identity should not re-read files merely to discover content identity if observed facts already owns
  hashing.
- Production service exposure exists as the bounded `hashSourceFilesBlake3` boundary command. The boundary service also
  owns a narrow scan-completion maintenance trigger that runs a small bounded BLAKE3 maintenance unit for the completed
  source after the terminal scan event and maintained invalidations are published. It does not synchronously drain all
  remaining candidates from a large source. Broader production scheduler policy remains separate integration work.

## Media Probe Placement

Media probe observations v0 also belong to `SourceFacts`. The store-owned adapter
`dekzer.source_file_media_probe.symphonia` version `1` probes supported audio files for basic media/container facts and
commits through the same inspect-source work/run/artifact authority path. Probe commits preserve current BLAKE3 evidence
only when the existing row basis is current. BLAKE3 commits preserve current probe fields under the same basis rule.

The probe job does not create attachment identity, track identity, CUE pairing, waveform data, prep rows, playlist rows,
or renderer UI. Video source files return typed unsupported outcomes in v0 until a video-capable adapter is selected.
CUE sheets remain source-file companion metadata rows and are not parsed by media probing.

## Boundary Between Identity Layers

| Layer                     | Owns                                                                                                 | Does not own                                                                                |
| ------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Source files              | Durable source-relative file inventory, path-derived file kind/media class, presence, size, mtime    | Playability, track identity, attachment identity, artwork role                              |
| Observed file facts       | Future/partial evidence from reading bytes or probing containers                                     | Product row admission or user-facing track identity by itself                               |
| Attachments               | Current durable bytes-identity relation from current BLAKE3 evidence to source-file occurrence links | Path proximity guesses, track identity, CUE pairing, prep readiness, browser rows           |
| Primary media             | Evidence-backed playable-media candidate projection from current attachments and audio probe facts   | Canonical track identity, CUE pairing, browser-row authority                                |
| Track identity candidates | Reversible exact evidence candidate grouping from current primary-media candidates                   | Canonical track identity, user decisions, semantic recording matching, CUE pairing          |
| Track identity decisions  | Reversible/supersedable decisions over candidates with preserved provenance                          | Canonical track rows, semantic identity by hash alone, CUE pairing, metadata reconciliation |
| Track identity            | Future semantic musical/performance identity                                                         | Source-file row identity, exact hash candidate alone, or playlist membership alone          |
| Preparation               | Capability targets, artifacts, readiness summaries                                                   | File identity or content-addressed attachment identity                                      |

## Next Implementation Gate

The next gate is collection health / source integrity work, video-capable probe adapter selection,
source-location-scoped admission, broader scheduler policy, CUE parse observations, user correction commands, or a later
canonical track identity layer that consumes exact candidate evidence plus additional evidence and user decisions.
Follow-on work must not conflate the attachment foundation, media probe evidence, primary-media candidates, track
identity candidates, or track identity decisions with canonical track tables, CUE pairing, artwork intelligence,
playlist UI, prep facets, or waveform generation.
