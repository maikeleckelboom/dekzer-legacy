---
status: accepted
last-reviewed: 2026-05-31
owner: library-substrate-boundary
canonical-context:
  - source-lifecycle-backend-contract-gap
  - media-relevant-file-inventory-contract
  - library-contents-read-boundary
scope:
  - asset-identity
  - primary-media-authority
  - cue-association
  - content-hash-placement
  - deferred-preparation-surfaces
---

# Media Identity Schema Authority

## Purpose

This document owns the current authority decision for media identity surfaces that sit beyond source lifecycle and
source-file inventory. It decides what is canonical now, what is intentionally dormant, and what was stale enough to
remove before observed file facts are broadened.

Reading order:

1. `docs/library/source-lifecycle-backend-contract-gap.md` owns source lifecycle and source readiness surfaces.
2. `docs/library/media-relevant-file-inventory-contract.md` owns source-file inventory, default contents admission, CUE
   inventory admission, and image-file inventory behavior.
3. This document owns asset identity, `primaryMedia`, CUE association, hashing placement, and deferred preparation
   surfaces.

## Current Canonical Surfaces

| Surface | Authority | Current role |
| --- | --- | --- |
| `source_files` | SQLite + `SourceFilesAuthorityTx` | Canonical attachment inventory row for observed source files. Classification is path-derived and provisional. |
| `readContents` default policy | Renderer boundary + service/store read model | Current product contents path: recursive `sourceFile` rows for audio, video, image, and admitted unsupported CUE sheets. |
| `SourceFacts.content_hash` | Observed source facts substrate | Optional accepted source-fact evidence attached to a `source_file_id`. It is not attachment identity and is not required by default contents reads. |
| `source_media` write/read guards | Store filesystem guard | Current guardrail for source-media read-only operations. CUE parsing operation names are reserved, not current parsing. |
| `Playlists` / `PlaylistEntries` | Boundary service + store | Live service/protocol/store surface. Desktop Main/Preload do not expose playlist writes yet. Playlist membership still targets `library_asset_id`. |

The default desktop renderer path does not request `primaryMedia`. `apps/desktop/src/renderer/library/boundary/contentsRead.ts`
requests `rowProfile: sourceFile` with audio, video, image, and unsupported media classes.

## Intentionally Dormant Or Transitional Surfaces

| Surface | Status | Blocker / owner |
| --- | --- | --- |
| `LibraryAssets` | Live transitional substrate, not current file identity authority | `equivalence_fingerprint` is caller-provided. Attachment identity requires observed file facts and durable bytes-derived evidence. |
| `LibraryAssetAttachments` / `SourceSegmentSets` / `SourceSegments` | Dormant future shape | Segment and attachment identity are not ratified. Future owner: observed facts / segmentation promotion. |
| `LibraryBrowserRows` | Live maintained projection over `LibraryAssets` | Used by library browser, playlist-scoped browser reads, and promoted `primaryMedia` summaries. Not current default contents authority. |
| `primaryMedia` row profile | Intentionally dormant projection shape with live protocol/read-model support | Correct long-term playable/performance projection. Blocked on attachment identity, observed file facts, and playable media evidence. |
| Waveform, stems, prep readiness summaries | Live generated/service/store projection fields, dormant in desktop UI | Future preparation owner. They summarize capability/projection state and are not canonical media identity. |
| `CapabilitySpecs`, `CapabilityDependencies`, `PrepPolicies`, `PrepAssignments`, `ResolvedLibraryAssetPrepTargets` | Live preparation substrate | Scheduler/prep substrate exists, but observed facts and attachment identity are not complete enough to make it product authority. |

## Delete-Now Surfaces Removed

The uncompiled `crates/library-store-sqlite/src/authority/media/*` scaffold was removed in this pass. It referenced
legacy tables such as `media_assets`, `file_media_latest`, `releases`, `media_sets`, and `entity_asset_links` that are
not present in the current baseline schema and was not included from `authority/mod.rs`. Its local SHA-256 inspection
code was therefore not current product hashing authority.

## `primaryMedia` Decision

Decision: **B. Intentionally dormant projection shape.**

Evidence:

- Store read model support exists in `read_models/contents.rs`.
- Boundary protocol and generated TypeScript expose `ContentsRowProfile.primaryMedia` and `PrimaryMediaSummary`.
- Desktop Main validates and maps the profile, but the renderer default requests `sourceFile`.
- `primaryMedia` rows are present-file scoped and reject image/unsupported media classes.
- Promoted rows depend on `LibraryBrowserRows`, `LibraryAssetAttachments`, `SourceSegments`, and `SourceSegmentSets`.
- Unpromoted fallback rows are derived directly from present audio/video `source_files` and are marked with
  `origin: sourceFile`.

Therefore `primaryMedia` must not be treated as current track identity. It remains the long-term playable/performance
projection shape, blocked by attachment identity, observed facts, and playable media evidence.

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

Answers:

| Question | Decision |
| --- | --- |
| What creates it? | Callers of `ResolveLibraryAssetPromotionInput` / `MintOrReuseLibraryAssetInput`. |
| What inputs derive it? | Not specified by current authority; tests use values such as `eq:track-a`. |
| Is it bytes-derived? | Not proven. |
| Is an algorithm named? | No. |
| Stable across moves? | Not guaranteed. |
| Stable across metadata-only filesystem changes? | Not guaranteed. |
| Stable across same-content duplicates? | Not guaranteed. |
| Current product authority? | Live transitional asset key, not attachment identity. |
| Can it support attachment identity? | No. It is opaque and not proven to be durable bytes-derived evidence. |

## Content Hash / BLAKE3 Placement

`SourceFacts.content_hash` exists and can store accepted observed-file evidence. Current tests and fixtures use
`sha256:` examples, but there is no live default product path that computes BLAKE3 or uses a content hash as attachment
identity. BLAKE3 is absent.

Future placement:

- Content hashing belongs to observed file facts / file evidence because hashing reads bytes.
- Attachment identity consumes durable hash/evidence later.
- Track identity must not rely on path identity.
- Attachment identity should not re-read files merely to discover content identity if observed facts already owns
  hashing.
- BLAKE3 selection, migration, and coexistence with any SHA-256 evidence require a separate observed-facts design.

## Boundary Between Identity Layers

| Layer | Owns | Does not own |
| --- | --- | --- |
| Source files | Durable source-relative file inventory, path-derived file kind/media class, presence, size, mtime | Playability, track identity, attachment identity, artwork role |
| Observed file facts | Future/partial evidence from reading bytes or probing containers | Product row admission or user-facing track identity by itself |
| Attachments | Future durable relation between media evidence and playable/asset identity | Path proximity guesses |
| Track identity | Future semantic musical/performance identity | Source-file row identity or playlist membership alone |
| Preparation | Capability targets, artifacts, readiness summaries | File identity or content-addressed attachment identity |

## Next Implementation Gate

The next gate is an observed file facts pass that defines byte-reading ownership, hash algorithm policy, basis
fingerprints, idempotence, and how source facts feed attachment identity. It must not add track tables, CUE pairing,
artwork intelligence, playlist UI, prep facets, or waveform generation as part of the same change.
