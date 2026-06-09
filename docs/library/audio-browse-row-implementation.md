---
status: implemented-v0
doctrine-version: 0.2
last-reviewed: 2026-06-06
owner: library-substrate-boundary
canonical-context:
  - contents-browse-policy
  - decisions/library-contents-read-boundary
  - tree-selection-contents-contract
  - media-relevant-file-inventory-contract
scope:
  - audio-browse-row-v0-boundary
  - contents-read-model-decision
---

# Audio Browse Row Implementation

## Status

Implemented V0.

This document resolves the V0 authority and implementation boundary for the audio browse row read-model concept. The
implemented V0 uses `{ kind: 'audioBrowse' }` in the profile-specific `ContentsReadPolicy` union under the existing
`readContents` boundary. It reuses the existing contents file-row payload shape and does not introduce a dedicated
generated audio row type.

## Decision

Audio browse row V0 is implemented as the filter-free `audioBrowse` policy variant under `readContents`.

The implementation keeps the existing
contents result envelope, scope model, scope depth model, coverage model, and cursor pagination. V0 rows use the current
contents file-row payload shape with the V0 field subset below. The product/read-model concept is audio browse row; it
is not a canonical track, not a tree row, and not a separate browse endpoint.

The smallest correct V0 field set is the current source-file audio row authority that already crosses the contents
boundary:

- row id;
- source id;
- source file id;
- parent directory id;
- display label;
- file name;
- source-relative path;
- file class;
- file kind;
- presence;
- optional availability state, preserving current absence for source-file-backed rows;
- optional updated timestamp;
- result state;
- result scope;
- result policy;
- scope depth;
- scopeCoverage;
- required service-owned `hasPolicyOmittedRows`;
- cursor.

Normalized extension, source display label, source-location provenance, source-file row version, and container/codec are
not V0 fields.

## Current Row Authority Inventory

| Surface                   | Current authority                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Store read model          | `StoreContentsFileRow` owns `id`, `source_id`, `source_file_id`, `parent_directory_id`, `label`, `relative_path`, `file_name`, `file_class`, `file_kind`, `presence`, `availability_state`, `primary_media`, `updated_at`, and internal `relative_path_browse_sort_key`. Source-file profile SQL returns source-file facts and `NULL` primary-media fields.                                                                                                                 |
| Rust boundary protocol    | `ContentsReadRequest` owns `scope`, `policy`, `scopeDepth`, `limit`, and `cursor`. `ContentsResult` owns `state`, echoed `scope`, echoed `policy`, echoed `scopeDepth`, `rows`, `scopeCoverage`, required `hasPolicyOmittedRows`, optional `nextCursor`, and optional `detail`. `ContentsFileRow` owns the current row fields listed above.                                                                                                                                 |
| Generated TS contract     | `packages/library-boundary-contract` mirrors the Rust protocol. `ContentsReadPolicy` is the `playableMediaBrowse`, `audioBrowse`, `sourceFileInventory`, or `primaryMedia` union. No dedicated generated `AudioBrowseRow` type exists.                                                                                                                                                                                                                                      |
| TS boundary client        | `LibraryBoundaryClient.readContents` sends the existing `snapshotRead/contentsRead` command and expects the existing `contents` reply. It adds no row authority.                                                                                                                                                                                                                                                                                                            |
| Desktop main adapter      | `apps/desktop/src/main/libraryContents/read.ts` normalizes scope, policy, scopeDepth, limit, and cursor, maps generated `ContentsFileRow` into shared `ContentsFileRow`, and rejects invalid image/unsupported primary-media combinations. It adds no browse-field authority.                                                                                                                                                                                               |
| Renderer boundary adapter | `apps/desktop/src/renderer/library/boundary/contentsRead.ts` currently uses `{ kind: 'playableMediaBrowse' }` as its hard-coded fallback request. The policy union also accepts `{ kind: 'audioBrowse' }`. Product filter wiring must replace the hard-coded fallback so initial **Audio** maps to `audioBrowse` and **Media** maps to `playableMediaBrowse`. The controller owns warm snapshots, retained rows, delayed pending display, and pagination accumulation only. |
| Renderer projection       | `contents/projection.ts` maps rows to `ContentRow` display rows with `id`, `label`, `presence`, `detail`, `icon`, `fileClass`, and `availabilityState`. Its relative-path fallback and file-class labels are presentation, not durable field authority.                                                                                                                                                                                                                     |
| Table display             | `contents/table.vue` displays `Name` and `Details` columns. It does not display extension, source label, source-location provenance, row version, container, or codec.                                                                                                                                                                                                                                                                                                      |
| Cursor fields             | Cursor identity is store-owned and binds version, scope, the full profile-specific policy, scopeDepth, and row order position. `audioBrowse` has a distinct policy identity from `sourceFileInventory` and reuses the source-file order position: `relative_path_browse_sort_key`, `relative_path`, and `source_file_id`.                                                                                                                                                   |
| Coverage fields           | `ContentsScopeCoverage` owns `state`, `subtreeCoverageComplete`, `emptyResultAuthoritative`, and optional `detail`; incomplete, scanning, blocked, failed, unavailable, and missing-location cases must not become authoritative empty results.                                                                                                                                                                                                                             |
| Scope fields              | Contents scope supports source, source location, and directory. Directory scope carries `sourceId` and `sourceDirectoryId`; source-location scope carries `sourceLocationId`. Rows do not carry per-row source-location provenance.                                                                                                                                                                                                                                         |
| Stable row identity       | Current source-file rows use `source-file:{source_file_id}`. Primary-media rows may use primary-media or library-asset identities, but those are not V0 audio browse row identities.                                                                                                                                                                                                                                                                                        |
| Display fields            | Current display authority is `label`, `fileName`, `relativePath`, `fileClass`, `fileKind`, `presence`, and optional `availabilityState`.                                                                                                                                                                                                                                                                                                                                    |
| Provenance fields         | Current per-row provenance is `sourceId`, `sourceFileId`, `parentDirectoryId`, and `relativePath`. Scope-level provenance is in `ContentsResult.scope`; coverage-level provenance is in `ContentsScopeCoverage`.                                                                                                                                                                                                                                                            |
| State fields              | Row state is `presence`, optional `availabilityState`, and optional `updatedAtMs`. Result state is `ContentsResult.state`, scopeCoverage, detail, and cursor.                                                                                                                                                                                                                                                                                                               |

## Field Authority Matrix

| Field                           | Decision      | Authority                     | Reason                                                                                                                                                          | Implementation impact                                                                                  |
| ------------------------------- | ------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Row id                          | current       | current contract              | `ContentsFileRow.id` is generated from the current store row identity.                                                                                          | Preserve `source-file:{source_file_id}` identity for source-file-backed audio browse rows.             |
| Source id                       | current       | current contract              | `ContentsFileRow.sourceId` and `StoreContentsFileRow.source_id` already cross the boundary.                                                                     | Preserve unchanged.                                                                                    |
| Source file id                  | current       | current contract              | `ContentsFileRow.sourceFileId` and `StoreContentsFileRow.source_file_id` already cross the boundary.                                                            | Preserve unchanged; do not treat it as a canonical track id.                                           |
| Parent directory id             | current       | current contract              | `ContentsFileRow.parentDirectoryId` maps from `source_files.parent_source_directory_id`.                                                                        | Preserve unchanged.                                                                                    |
| Display label                   | current       | current store/read model      | Store contents labels fall back from title to file name to relative path; source-file rows currently use file name unless empty.                                | Preserve store-owned label; renderer may display it only.                                              |
| File name                       | current       | current contract              | `ContentsFileRow.fileName` maps from `source_files.name`.                                                                                                       | Preserve unchanged.                                                                                    |
| Source-relative path            | current       | current contract              | `ContentsFileRow.relativePath` maps from `source_files.relative_path` and is current row location provenance.                                                   | Preserve unchanged.                                                                                    |
| File class                      | current       | current contract              | `ContentsFileRow.fileClass` maps from `source_files.file_class`; V0 policy/profile must return audio rows only.                                                 | Enforce audio browse profile parity with current audio source-file reads.                              |
| File kind                       | current       | current contract              | `ContentsFileRow.fileKind` maps from `source_files.file_kind`.                                                                                                  | Preserve as file-kind authority, not container or codec.                                               |
| Presence                        | current       | current contract              | `ContentsFileRow.presence` maps from `source_files.presence_state`.                                                                                             | Preserve present/missing/removed behavior.                                                             |
| Availability state              | current       | current contract              | The field exists on `ContentsFileRow`; source-file profile currently returns it absent.                                                                         | Keep optional and absent unless store/read-model authority supplies it; do not synthesize in renderer. |
| Updated timestamp               | current       | current store/read model      | `ContentsFileRow.updatedAtMs` maps from `StoreContentsFileRow.updated_at`.                                                                                      | Preserve as timestamp, not a row version.                                                              |
| Result state                    | current       | current contract              | `ContentsResult.state` already carries ready, empty, partial, unavailable, missing, blocked, failed, policy-conflict, and cursor-invalid outcomes.              | Reuse unchanged.                                                                                       |
| Scope echo                      | current       | current contract              | `ContentsResult.scope` echoes source, source-location, or directory scope.                                                                                      | Reuse unchanged.                                                                                       |
| Policy echo                     | current       | current contract              | `ContentsResult.policy` echoes the profile-specific policy union.                                                                                               | Preserve the `audioBrowse` discriminant in cursor identity and echo behavior.                          |
| ScopeDepth                      | current       | current contract              | `ContentsResult.scopeDepth` echoes immediate or recursive.                                                                                                      | Reuse unchanged.                                                                                       |
| ScopeCoverage                   | current       | current contract              | `ContentsResult.scopeCoverage` owns completeness and authoritative-empty semantics.                                                                             | Reuse unchanged.                                                                                       |
| Cursor                          | current       | current store/read model      | `nextCursor` is store-owned and binds scope, policy, scopeDepth, and row ordering position.                                                                     | Preserve the distinct `audioBrowse` policy identity without mixing pages across policy variants.       |
| Normalized extension            | promote later | proposed read-model authority | Useful as a convenience derived from persisted file name/path, but not currently a boundary field. It must be named extension, not format, container, or codec. | Exclude from V0. Add only in a later store/read-model-owned slice with tests.                          |
| Source display label            | defer         | none                          | Current row payload does not carry source display labels, and current selected-scope UI can display scope title outside the row.                                | Exclude from V0; add only if a later product column requires it.                                       |
| Source-location id              | defer         | none                          | Current rows do not carry per-row location membership, and overlapping accepted locations make naive inference ambiguous.                                       | Exclude from V0; add only with explicit location-membership authority and tie-break rules.             |
| Source-location label           | defer         | none                          | Depends on source-location provenance, which is not current per-row authority.                                                                                  | Exclude from V0.                                                                                       |
| Source-file row version         | defer         | none                          | `source_files` has `updated_at` but no row-version concept. Navigation and primary-media surfaces have row versions; source-file rows do not.                   | Exclude from V0; do not map `updatedAtMs` as a version.                                                |
| Container                       | reject        | none                          | Source-file rows have no explicit container evidence. File kind and extension are not container authority.                                                      | Do not add to V0.                                                                                      |
| Codec                           | reject        | none                          | Codec appears only on primary-media/evidence-backed summaries, not current source-file audio browse rows.                                                       | Do not add to V0.                                                                                      |
| Title                           | reject        | none                          | Optional primary-media summary fields are not reliable source-file browse-row fields.                                                                           | Do not add to V0.                                                                                      |
| Artist                          | reject        | none                          | Optional primary-media summary fields are not reliable source-file browse-row fields.                                                                           | Do not add to V0.                                                                                      |
| Album                           | reject        | none                          | Optional primary-media summary fields are not reliable source-file browse-row fields.                                                                           | Do not add to V0.                                                                                      |
| Duration                        | reject        | none                          | Duration is evidence/analysis-adjacent and not current source-file browse-row authority.                                                                        | Do not add to V0.                                                                                      |
| BPM                             | reject        | none                          | Analysis fact, not browse-row identity.                                                                                                                         | Do not add to V0.                                                                                      |
| Musical key                     | reject        | none                          | Analysis fact, not browse-row identity.                                                                                                                         | Do not add to V0.                                                                                      |
| Waveform                        | reject        | none                          | Analysis artifact, not browse-row identity.                                                                                                                     | Do not add to V0.                                                                                      |
| Artwork                         | reject        | none                          | Role decision, not browse-row identity.                                                                                                                         | Do not add to V0.                                                                                      |
| CUE association                 | reject        | none                          | Association/segmentation decision, not browse-row identity.                                                                                                     | Do not add to V0.                                                                                      |
| Canonical track id              | reject        | none                          | Track identity decision, not browse-row identity.                                                                                                               | Do not add to V0.                                                                                      |
| Duplicate or same-song key      | reject        | none                          | Identity resolution, not browse-row identity.                                                                                                                   | Do not add to V0.                                                                                      |
| Analysis readiness              | reject        | none                          | Preparation must remain grouped by fact, structure, artifact, work, outcome, and satisfaction concepts.                                                         | Do not add a flat readiness field to V0.                                                               |
| Stems state                     | reject        | none                          | Preparation/analysis domain, not browse-row identity.                                                                                                           | Do not add to V0.                                                                                      |
| Tags                            | reject        | none                          | Organization domain, not browse-row identity.                                                                                                                   | Do not add to V0.                                                                                      |
| Notes                           | reject        | none                          | Organization domain, not browse-row identity.                                                                                                                   | Do not add to V0.                                                                                      |
| Crates                          | reject        | none                          | Organization domain, not browse-row identity.                                                                                                                   | Do not add to V0.                                                                                      |
| Sleeves                         | reject        | none                          | Organization domain, not browse-row identity.                                                                                                                   | Do not add to V0.                                                                                      |
| Routes                          | reject        | none                          | Organization domain, not browse-row identity.                                                                                                                   | Do not add to V0.                                                                                      |
| Cloud or streaming availability | reject        | none                          | Not current local source-file browse authority.                                                                                                                 | Do not add to V0.                                                                                      |

## Boundary Shape

### Selected: profile-specific policy under existing `readContents`

The implementation uses the `ContentsReadPolicy` union and keeps the existing `ContentsReadRequest` and
`ContentsResult` envelope and row payload.

This is the smallest correct boundary because it preserves:

- cursor compatibility by extending the existing cursor identity model with a distinct profile kind;
- scopeCoverage compatibility by reusing `ContentsScopeCoverage`;
- source, source-location, and directory scope compatibility;
- descendant-scope contents browse behavior;
- source-file audio parity without making source-file rows canonical tracks;
- generated contract locality: one enum/profile addition instead of a second command family;
- renderer surface impact: keep the existing contents controller, pagination, and table projection while the profile
  changes at the boundary;
- one browse path for contents-pane reads.

### Rejected: new contents row kind under existing result shape

A row union or new row kind would force renderer and generated-contract branching before V0 has fields that require a
separate payload. It also invites a type-level product concept before the read-model authority is meaningfully distinct.

### Rejected: new dedicated read endpoint

A dedicated endpoint would duplicate scope, scope depth, scopeCoverage, cursor, validation, IPC, and client surfaces already
owned by `readContents`. There is no ownership reason for a new command.

### Rejected: no new boundary yet

Keeping only source-file rows leaves the audio-only read-model concept expressed as raw inventory policy.
`audioBrowse` provides the backend-owned policy for the initial **Audio** workflow filter.
`playableMediaBrowse` provides the backend-owned policy for the separate **Media** filter.

## V0 Acceptance Criteria

- `readContents` accepts `{ kind: 'audioBrowse' }`; incompatible class combinations are unrepresentable.
- The initial **Audio** workflow requests audio browse rows with recursive source, source-location, and directory scopes.
- The separate **Media** workflow requests `playableMediaBrowse`.
- V0 audio browse rows match `sourceFileInventory.fileClasses = ['audio']` results for the same scope, scopeDepth, limit, and cursor,
  including row order and absence/presence of optional fields.
- Audio browse rows never carry primary-media summary fields, canonical track identity, preparation summaries, waveform,
  stems, BPM, key, duration, artwork, CUE association, tags, notes, crates, sleeves, routes, or cloud/streaming state.
- Cursor identity includes the audio browse profile and cannot mix source-file, primary-media, and audio-browse pages.
- Coverage, required `hasPolicyOmittedRows`, blocked, failed, scanning, incomplete, source-unavailable, and
  location-missing semantics remain service-owned.
- MP4/video rows never enter `audioBrowse`; an MP4-only resolved scope returns zero rows with
  `hasPolicyOmittedRows: true`.
- Source-file rows are not added back to tree navigation.
- Renderer does not filter, sort, or derive authoritative audio browse fields.

## V0 Test Coverage

- Store read-model tests proving audio browse parity with `sourceFileInventory` audio recursive and immediate
  reads for source, source-location, and directory scopes.
- Store cursor tests proving audio browse cursor page two, invalid cursor profile mismatch, policy mismatch, scopeDepth
  mismatch, scope mismatch, and no duplicate/gap behavior.
- Store scope coverage tests proving incomplete, blocked, failed, unavailable, and missing-location cases keep current
  scope coverage semantics.
- Boundary protocol serialization tests for the policy discriminant and cursor-invalid behavior.
- Boundary service mapping tests proving profile mapping and row parity.
- Generated contract checks proving TS/schema include the new row-profile kind and no new dedicated row type.
- Desktop main adapter tests proving request normalization, generated-contract mapping, error mapping, and no
  renderer-side field derivation.
- Renderer boundary tests proving cursor pagination and request-key identity for every policy. Workflow-filter tests
  remain required when the current hard-coded fallback is replaced by product filter wiring.
- Renderer projection tests proving current row display parity and no filtering/sorting authority.

## Contract Generation

The implementation updates Rust protocol first, then regenerates `packages/library-boundary-contract` from the Rust
source of truth. Generated TS and JSON schema expose the profile-specific `ContentsReadPolicy` union and mechanical
manifest hashes. No dedicated audio row type, dedicated endpoint, or source hierarchy contract change exists.

## Renderer Surface

The renderer currently handles request-key identity for every policy profile but still hard-codes
`playableMediaBrowse` as its fallback request. That is implementation state, not product doctrine. Product filter wiring
must map initial **Audio** to `audioBrowse` and **Media** to `playableMediaBrowse`.
Projection and table code render returned rows and do not derive extension, source label,
source-location provenance, row version, container, or codec. Source-file rows remain available for explicit
non-default inventory/diagnostic modes.

## Unresolved Gaps

- Normalized extension is useful later, but it needs store/read-model authority and must remain an extension field, not
  format, container, or codec.
- Source display label is not a V0 row field. A later product column should decide whether scope title is sufficient or
  whether row-level source labels are required.
- Source-location provenance is not safe to infer per row because accepted locations can overlap. A later slice would
  need explicit membership authority and deterministic tie-break rules.
- Source-file row version does not exist. `updatedAtMs` is a timestamp and must not be described as a row version.
- Container and codec are rejected for source-file audio browse rows until explicit media evidence authority is chosen.
