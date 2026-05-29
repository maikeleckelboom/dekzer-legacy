---
status: candidate
doctrine-version: 0.1
last-reviewed: 2026-05-29
owner: renderer-substrate-boundary
canonical-context:
  - product-doctrine-shortened
  - source-root-scan-admission-contract
  - library-tree-frame-stability-contract
  - first-slice-substrate-map
scope:
  - source-hierarchy-read-boundary
  - hierarchy-row-model
  - hierarchy-pagination-model
  - hierarchy-coverage-and-empty-results
  - renderer-hierarchy-cache-permissions
  - hierarchy-versus-contents-reads
  - hierarchy-versus-scan-admission
  - source-file-visibility-vocabulary
---

# Source Hierarchy Contract

## Status and scope

This document defines the source hierarchy read boundary as its own contract, separate from contents-pane reads and scan
admission. It governs how the renderer reads literal source hierarchy children, what rows those reads produce, how
pagination works, what coverage and empty-result semantics mean, what the renderer may cache, and how hierarchy reads
differ from contents reads.

This contract is implementation-grounded. It describes the current `readLiteralHierarchyChildren` boundary and its
consumers, not a future ideal.

## Contract role

The source hierarchy contract exists because hierarchy reads and contents reads are separate systems with different
pagination strategies, different row models, different coverage semantics, and different renderer controllers.
Scan admission is a third system. They must not blur.

| System                    | Owns                                                       | Must not own                                  |
|---------------------------|------------------------------------------------------------|-----------------------------------------------|
| Source hierarchy reads    | Immediate children of a hierarchy directory or source root. | Recursive selected contents.                  |
| Contents reads            | Flat/recursive file rows for a selected scope.             | Hierarchy structure, directory nesting.       |
| Scan admission            | Traversal, candidate gate, media inventory.                | Renderer projection of hierarchy or contents. |

## Row model

Source hierarchy rows represent a literal filesystem-source tree projected into the renderer. The current row model uses
`LiteralHierarchyNode`:

| Field                        | Meaning                                                             |
|------------------------------|---------------------------------------------------------------------|
| `node_kind`                  | `directory`, `file`, `source_root`.                                 |
| `source_id`                  | Substrate source identity.                                          |
| `source_directory_id`        | Directory identity, when the row is a directory node.               |
| `source_file_id`             | File identity, when the row is a file node.                         |
| `parent_source_directory_id` | Parent directory identity. `null` for source root children.         |
| `relative_path`              | Path relative to the source root.                                   |
| `display_name`               | Name for display.                                                   |
| `media_class`                | `audio`, `video`, `image`, `unsupported`, or `null`.                |
| `presence_state`             | `present`, `missing`, `removed`, `pending`.                         |
| `size_bytes`                 | File size, if known.                                                |
| `modified_at_ns`             | Modification timestamp, if known.                                   |
| `has_child_directories`      | Whether the directory has child directories.                        |
| `has_primary_media_descendant` | Whether the directory subtree contains primary media.              |
| `has_image_media_descendant` | Whether the directory subtree contains image media.                 |
| `dir_scan_state`             | `pending`, `scanning`, `complete`, `blocked`, `failed`, or `null`.  |

Rows are grouped into a `LiteralHierarchyWindow` per page:

| Field                    | Meaning                                                    |
|--------------------------|------------------------------------------------------------|
| `entry_point`            | Source or source location identity.                        |
| `parent_source_directory_id` | Which directory's children are being read. `null` for root. |
| `offset`                 | Page offset (zero-based).                                  |
| `limit`                  | Page size.                                                 |
| `total_rows`             | Total child rows known for this parent under current policy. |
| `rows`                   | Page of `LiteralHierarchyNode` rows.                       |
| `coverage`               | `LiteralHierarchyCoverage` for this window.                |

### Row kinds in current implementation

| Row kind       | Represents                                                   | Expandable |
|----------------|--------------------------------------------------------------|------------|
| `source_root`  | Registered source root.                                      | Yes        |
| `directory`    | Source directory within a source.                            | Yes        |
| `file`         | Source file (admitted candidate or inventory file).          | No         |

`primary_media`, `segment`, `companion`, `blocked`, `excluded`, and `unsupported` are tree-projection concepts
(surface by row profile) but the literal hierarchy row kind from the substrate is `file` or `directory`.

## Read boundary

The hierarchy read boundary is a single command:

```
ReadLiteralHierarchyChildren(entryPoint, parentSourceDirectoryId?, offset, limit, sourceFileVisibility?)
  → ReadLiteralHierarchyChildrenReply(window: LiteralHierarchyWindow?)
```

This boundary is implemented through:

| Layer                         | Location                                                              |
|-------------------------------|-----------------------------------------------------------------------|
| Rust protocol                 | `crates/library-boundary-protocol/src/commands/snapshot_reads.rs`     |
| Rust service                  | `crates/library-boundary-service/src/snapshot_read_protocol.rs`       |
| Rust store                    | `crates/library-store-sqlite/src/read_models/literal_hierarchy.rs`    |
| TS boundary contract          | `packages/library-boundary-contract/index.ts`                         |
| Main IPC                      | `apps/desktop/src/main/libraryHierarchy/readChildren.ts`              |
| Shared TS types               | `apps/desktop/src/shared/libraryHierarchy/readChildren.ts`            |
| Renderer hierarchy controller | `apps/desktop/src/renderer/library/boundary/hierarchyRead.ts`         |

The hierarchy read returns immediate children only. It does not recurse. Recursion into subdirectories requires separate
reads at the renderer level, triggered by user expansion actions.

## Pagination model

Hierarchy pagination uses offset-based pagination, not cursor-based pagination:

| Pagination aspect | Hierarchy reads                    | Contents reads                    |
|-------------------|------------------------------------|------------------------------------|
| Pagination token  | `offset` (integer)                 | `cursor` (encoded string)          |
| Next-page signal  | `nextOffset` (renderer-computed)   | `nextCursor` (store-computed)       |
| Load-more action  | `loadMore` (tree) / `loadChildren` | `loadContentsPage`                 |
| Pagination state  | Renderer computes from `totalRows` | Store encodes cursor from row tuple |

The renderer computes `nextOffset` as `rows.length` when `rows.length < totalRows`. The renderer stores this in
`LoadedChildren.nextOffset` and uses it in subsequent `loadMore` requests via `moreReadRequest()`.

Load-more in the tree:

```
User clicks load-more row
  → tree projection emits { kind: 'loadMore', state }
  → panel dispatches to hierarchy read controller
  → controller issues ReadLiteralHierarchyChildren with target.offset
  → result is appended to existing children via appendHierarchyChildrenWindow()
```

Load-children on expand:

```
User expands directory
  → tree projection emits { kind: 'loadChildren', state }
  → panel calls requestNodeChildren / requestDirectoryChildren
  → controller issues ReadLiteralHierarchyChildren with offset: 0
  → result replaces (or initializes) the branch's children via loadedChildrenFromWindow()
```

## Coverage and empty-result semantics

Every hierarchy window carries a `LiteralHierarchyCoverage`:

| Field                      | Meaning                                                                 |
|----------------------------|-------------------------------------------------------------------------|
| `state`                    | `Complete`, `Pending`, `Scanning`, `Blocked`, `Failed`, `SourceUnavailable`, `LocationMissing`. |
| `recursiveScopeComplete`   | Whether all descendant directories have been enumerated.                |
| `emptyResultAuthoritative` | Whether the result of zero rows is definitive.                          |
| `detail`                   | Optional human-readable detail for diagnosis.                           |

Rules for `emptyResultAuthoritative`:

| Condition                                                     | `emptyResultAuthoritative` |
|---------------------------------------------------------------|----------------------------|
| Coverage is `Complete` AND `totalRows == 0`                   | `true`                     |
| Coverage is `Complete` AND `totalRows > 0`                    | `false` (rows exist)       |
| Coverage is `Scanning`, `Pending`, `Blocked`, or `Failed`     | `false`                    |
| Coverage is `SourceUnavailable` or `LocationMissing`          | `false`                    |

**Empty rows do not casually mean inaccessible, unscanned, unsupported, blocked, filtered, pending, or absent.**
Zero rows with `emptyResultAuthoritative = false` means the current coverage state cannot confirm emptiness.
Only `emptyResultAuthoritative = true` means the scope was fully scanned and definitively contains zero rows.

## Renderer cache permissions

The renderer hierarchy controller (`hierarchyRead.ts`) maintains non-authoritative in-memory state:

| Renderer state          | What it holds                              | Authoritative? |
|-------------------------|--------------------------------------------|----------------|
| `sourceReadStates`      | `Map<nodeId, SourceState>`                 | No             |
| `directoryReadStates`   | `Map<nodeId, DirectoryState>`              | No             |
| `LoadedChildren`        | `{ rows, totalRows, coverage, nextOffset }` | No             |

The renderer may cache:

- Already-loaded children for expanded directories.
- Child row lists, total counts, and coverage.
- Expansion state (which rows are expanded).

The renderer must not:

- Infer that a directory is empty from a missing or never-loaded cache entry.
- Invent children, readiness, or media classification.
- Use hierarchy cache as an authoritative contents source for the contents pane.
- Assume `emptyResultAuthoritative` is true when coverage is not `Complete`.

When `sourceFileVisibility` changes, the renderer may replay existing state by re-requesting children under the new
visibility. The current implementation supports this through `setSourceFileVisibility(visibility, { replayNodeIds })`.
The replay reads each affected branch under the new visibility and replaces the cached children. It must not clear
children before the new read completes (frame stability rule).

## Stale response behavior

The renderer hierarchy controller preserves existing children while a new read is in flight. Key behaviors:

| Scenario                                        | Renderer behavior                                            |
|-------------------------------------------------|--------------------------------------------------------------|
| Read in flight; prior children exist            | Keep existing children; mark branch refreshing.              |
| Read completes with new data                    | Replace branch children.                                     |
| Read fails; prior children still valid          | Keep prior children; do not flash empty.                     |
| Visibility changes during read                  | Expectation: response is validated against current visibility before acceptance. |
| Window does not match expected offset/visibility | Window is discarded.                                         |

Window validation (`isExpectedWindow`) checks: `offset`, `parentDirectoryId`, `entryPoint`, `sourceFileVisibility`,
`nodes.length <= limit`, `offset + nodes.length <= totalRows`, and non-empty when offset < totalRows.

## Relationship to contents reads

Hierarchy reads and contents reads are separate systems. The following table summarizes the critical differences:

| Axis              | Hierarchy read                                   | Contents read                           |
|-------------------|--------------------------------------------------|-----------------------------------------|
| Question answered | "What children are under this directory?"         | "What files are in this selected scope?" |
| Row granularity   | Directories and files (immediate children).       | Files only (flat or recursive).          |
| Pagination        | Offset-based.                                     | Cursor-based.                           |
| Next-page signal  | `nextOffset` (integer).                           | `nextCursor` (encoded string).          |
| Load action       | Tree `loadMore` / `loadChildren`.                 | Pane `loadContentsPage`.                |
| Result shape      | `LiteralHierarchyWindow`.                         | `ContentsResult`.                       |
| Coverage shape    | `LiteralHierarchyCoverage`.                       | `ContentsCoverage`.                     |
| Profile parameter | `sourceFileVisibility`.                           | `ContentsReadPolicy` (row profile + media classes). |
| Recursion         | Not supported; children are immediate.            | Supported via `ContentsRecursion`.       |
| Renderer owner    | `boundary/hierarchyRead.ts`.                      | `boundary/contentsRead.ts`.             |

A contents read must never use hierarchy cache as its data source. A hierarchy read must never use contents pagination
state. The existence of a directory in the hierarchy tree does not prove the directory contains media files. A contents
read for that directory answers that question separately.

## Relationship to scan admission

The hierarchy read boundary consumes already-persisted hierarchy state. It does not perform filesystem traversal, scan
admission, or media candidate inspection.

| Scan admission system              | Hierarchy read system                                 |
|------------------------------------|-------------------------------------------------------|
| Walks the filesystem.              | Reads persisted SQLite rows.                          |
| Admits or rejects candidates.      | Filters rows by `sourceFileVisibility` profile.       |
| Records observations.              | Returns rows and coverage.                            |
| Updates scan coverage.             | Consumes coverage state already persisted.            |
| Produces `source_files` inventory. | Projects inventory as hierarchy rows.                 |

A directory that has not been scanned shows `coverage.state = Pending` or `Scanning` in the hierarchy read result.
The hierarchy read does not trigger a scan to fill in the gap. That is the scan admission system's responsibility.

## Current vocabulary debt

The hierarchy read boundary uses `SourceFileVisibility` as a protocol-level parameter. This is current implementation
vocabulary, not final product doctrine:

| Current term            | What it actually does                      | Notes                                   |
|-------------------------|--------------------------------------------|-----------------------------------------|
| `sourceFileVisibility`  | Selects which media classes appear in hierarchy children. | Provisional naming debt.             |
| `Performance`           | Show audio and video children.             | Filters by `media_class IN ('audio','video')`. |
| `PerformanceAndImages`  | Show audio, video, and image children.     | Filters by `media_class IN ('audio','video','image')`. |

This vocabulary is present in all layers:

- Rust protocol: `SourceFileVisibility` enum
- Rust store: `browse_media::SourceFileVisibility` and `source_file_visibility_predicate_sql`
- TS boundary contract: generated from Rust protocol
- Shared TS types: `shared/libraryHierarchy/readChildren.ts`
- Renderer controller: `boundary/hierarchyRead.ts` with `setSourceFileVisibility()`
- Renderer tree projection: `tree/projection.ts` consumes visibility

Do not rename `sourceFileVisibility` in this task. It is current naming debt, not product doctrine.
A future vocabulary cleanup may rename it to something more descriptive of its actual role (hierarchy child
media-class filter), but the first slice stabilizes the current names first.

## Non-goals

This contract does not:

- Make `rowAdmission` canonical (not an implemented concept).
- Make `browsePolicy` canonical (renderer folder concept only).
- Rename `sourceFileVisibility`.
- Claim source hierarchy owns recursive selected contents.
- Claim hierarchy cache is an authoritative contents source.
- Claim scan admission is fully implemented.
- Blur media inventory with scan admission.
- Describe cursor docs as stale (cursor docs are already repaired).
- Govern source lifecycle visible state (see `source-lifecycle-visible-state-contract.md`).
- Govern tree selection → contents coupling (see `library-tree-selection-contents-contract.md`).
- Govern row profile definitions (see `library-row-profile-contract.md`).

## Invariants

The following invariants hold for the source hierarchy read boundary:

| Invariant                                                                | Enforcement                                               |
|--------------------------------------------------------------------------|-----------------------------------------------------------|
| Hierarchy reads use offset/limit pagination, not cursor pagination.      | Protocol shape; `ReadLiteralHierarchyChildrenRequest` uses `offset`. |
| Contents reads use cursor pagination, not offset pagination.             | Protocol shape; `ContentsReadRequest` uses `cursor`.       |
| Hierarchy `loadMore` / `loadChildren` must not use `loadContentsPage`.   | Renderer dispatch in `panel.vue`.                         |
| Contents `loadContentsPage` must not use `loadMore` / `loadChildren`.    | Renderer dispatch in `panel.vue`.                         |
| `emptyResultAuthoritative = false` when coverage is not `Complete`.      | Store `literal_hierarchy.rs` coverage computation.        |
| Zero rows with incomplete coverage must not be rendered as "empty."      | Tree projection respects `emptyResultAuthoritative`.      |
| Renderer must not clear visible children while a hierarchy read is pending. | Frame stability contract.                              |
| Hierarchy cache is not an authoritative contents source.                 | Contents reads are independent of hierarchy controller.   |
| `sourceFileVisibility` change replays children under new visibility.     | Renderer `setSourceFileVisibility` with `replayNodeIds`.  |
| Hierarchy reads never trigger filesystem traversal.                      | Reads are from persisted SQLite state.                    |
| `rowAdmission` is not an implemented hierarchy concept.                  | Must not be made canonical accidentally.                  |
| `browsePolicy` is a renderer folder, not a canonical contract.           | Must not be made canonical accidentally.                  |
