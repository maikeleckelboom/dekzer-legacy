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
---

# Source Hierarchy Contract

## Status and scope

This document defines the source hierarchy read boundary as its own contract, separate from contents-pane reads and scan
admission. It governs how the renderer reads literal source hierarchy children, what rows those reads produce, how
pagination works, what coverage and empty-result semantics mean, what the renderer may cache, and how hierarchy reads
differ from contents reads.

This contract is implementation-grounded. It describes the current `ReadLibraryTreeChildren` boundary and its
consumers, not a future ideal.

## Contract role

The source hierarchy contract exists because the systems below are separate with different
pagination strategies, different row models, different coverage semantics, and different controllers.
They must not blur.

### Literal source hierarchy

Persisted substrate/read-model semantics. The Rust/SQLite store owns the durable source identity,
hierarchy nodes, and scan coverage. This is the authoritative source of truth for what exists on disk.

### Library tree children

Product-facing renderer read surface. The `ReadLibraryTreeChildren` command returns admitted rows
through the library tree product boundary. Row admission is an internal product-boundary concern;
the command exposes no visibility or policy parameter to the renderer.

### Contents read

Selected-scope file rows. Contents reads return flat or recursive file rows for a selected scope,
using cursor-based pagination. Contents reads must not use hierarchy pagination state.

### Scan admission

Traversal, candidate gate, media inventory. Scan admission walks the filesystem, admits or rejects
candidates, records observations, and updates scan coverage. It does not project hierarchy or contents
to the renderer.

## Row model

Source hierarchy rows represent a literal filesystem-source tree projected into the renderer. The current row model uses
`LibraryTreeNode`:

| Field                        | Meaning                                                             |
|------------------------------|---------------------------------------------------------------------|
| `node_kind`                     | `directory`, `file`.                                                |
| `source_id`                     | Substrate source identity.                                          |
| `source_directory_id`           | Directory identity, when the row is a directory node.               |
| `source_file_id`                | File identity, when the row is a file node.                         |
| `parent_source_directory_id`    | Parent directory identity. `null` when the row's parent is the window root. |
| `relative_path`                 | Path relative to the source root.                                   |
| `display_name`                  | Name for display.                                                   |
| `media_class`                   | `audio`, `video`, `image`, `unsupported`, `none`, or absent.        |
| `presence_state`                | `present`, `missing`, `removed`.                                    |
| `size_bytes`                    | File size, if known.                                                |
| `modified_at_ns`                | Modification timestamp, if known.                                   |
| `updated_at_ms`                 | Last update timestamp.                                              |
| `has_child_directories`         | Whether the directory has child directories, if directory.          |
| `child_row_state`               | Directory expandability signal. `unknown`, `hasChildRows`, `noChildRows`. Directory-only. |
| `directory_primary_media_state` | Primary media descendant state for directories. Tagged union; see below. |
| `directory_image_media_state`   | Image media descendant state for directories. Tagged union; see below. |
| `directory_scan_state`          | Scan state for directories. Enum; see below.                        |

### Tagged-union directory fields

The `directory_primary_media_state`, `directory_image_media_state`, and `directory_scan_state` fields are
protocol-level representations derived from store-level raw fields at mapping time:

| Protocol field                     | Variants                                                                      | Store-level source                        |
|------------------------------------|-------------------------------------------------------------------------------|-------------------------------------------|
| `directory_primary_media_state`    | `Unknown`, `HasPrimaryMediaDescendants`, `NoPrimaryMediaDescendants`          | `has_primary_media_descendant` bool + `dir_scan_state` |
| `directory_image_media_state`      | `Unknown`, `HasImageMediaDescendants`, `NoImageMediaDescendants`              | `has_image_media_descendant` bool + `dir_scan_state` |
| `directory_scan_state`             | `Pending`, `Scanning`, `Complete`, `Failed`, `Blocked`                        | `dir_scan_state` string                   |

At the store layer, `has_primary_media_descendant` and `has_image_media_descendant` are `Option<bool>` and
`dir_scan_state` is `Option<String>`. The service mapping converts these into protocol-level tagged-union
and enum values: if the boolean is `true`, the state is `Has*Descendants`; if the scan is `Complete` and
the boolean is not `true`, the state is `No*Descendants`; otherwise `Unknown`. The `directory_scan_state`
maps from the store string to the protocol enum. All three fields are absent on file nodes.

Directory scan pending (`directory_scan_state = Pending`) belongs to `directory_scan_state` only;
`presence_state` may only describe `present`, `missing`, or `removed`.

Rows are grouped into a `LibraryTreeWindow` per page:

| Field                    | Meaning                                                    |
|--------------------------|------------------------------------------------------------|
| `entry_point`            | Source or source location identity.                        |
| `parent_source_directory_id` | Which directory's children are being read. `null` for root. |
| `offset`                 | Page offset (zero-based).                                  |
| `limit`                  | Page size.                                                 |
| `total_rows`             | Total child rows known for this parent under current policy. |
| `rows`                   | Page of `LibraryTreeNode` rows.                            |
| `coverage`               | `LibraryTreeCoverage` for this window.                     |

### Row kinds in current implementation

| Row kind     | Represents                                                   | Expandable |
|--------------|--------------------------------------------------------------|------------|
| `directory`  | Source directory within a source.                            | Yes        |
| `file`       | Source file (admitted candidate or inventory file).          | No         |

`primary_media`, `segment`, `companion`, `blocked`, `excluded`, and `unsupported` are tree-projection concepts
(surface by row profile) but the literal hierarchy row kind from the substrate is `file` or `directory`.

Source roots are navigation entities, not hierarchy child rows. They connect to hierarchy reads through the
`entry_point` on `LibraryTreeWindow` and the `ReadRoot` / `EntryPoint` on `ChildWindow` in shared TS.
The window's `parent_source_directory_id = null` signals that the returned rows are root-level children.

### Field naming across layers

snake_case names are Rust/protocol field names (e.g., `directory_primary_media_state`). Generated TypeScript
receives camelCase equivalents (e.g., `directoryPrimaryMediaState`). Shared renderer types use post-mapping
app-facing names: `ChildWindow` for the window, `ChildRow` for the row, `NodeKind` for the kind, and
`Presence` for the presence state.

## Read boundary

The hierarchy read boundary is a single command:

```
ReadLibraryTreeChildren(entryPoint, parentSourceDirectoryId?, offset, limit)
  → ReadLibraryTreeChildrenReply(window: LibraryTreeWindow?)
```

This boundary is implemented through:

| Layer                         | Location                                                              |
|-------------------------------|-----------------------------------------------------------------------|
| Rust protocol                 | `ReadLibraryTreeChildrenRequest`, `ReadLibraryTreeChildrenReply`, `LibraryTreeWindow`, `LibraryTreeNode`, `LibraryTreeCoverage`, `ChildRowState` |
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
  → controller issues ReadLibraryTreeChildren with target.offset
  → result is appended to existing children via appendHierarchyChildrenWindow()
```

Load-children on expand:

```
User expands directory
  → tree projection emits { kind: 'loadChildren', state }
  → panel calls requestNodeChildren / requestDirectoryChildren
  → controller issues ReadLibraryTreeChildren with offset: 0
  → result replaces (or initializes) the branch's children via loadedChildrenFromWindow()
```

## Coverage and empty-result semantics

Every hierarchy window carries a `LibraryTreeCoverage`:

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

## Stale response behavior

The renderer hierarchy controller preserves existing children while a new read is in flight. Key behaviors:

| Scenario                                        | Renderer behavior                                            |
|-------------------------------------------------|--------------------------------------------------------------|
| Read in flight; prior children exist            | Keep existing children; mark branch refreshing.              |
| Read completes with new data                    | Replace branch children.                                     |
| Read fails; prior children still valid          | Keep prior children; do not flash empty.                     |
| Window does not match expected offset/entry     | Window is discarded.                                         |

Window validation (`isExpectedWindow`) checks: `offset`, `parentDirectoryId`, `entryPoint`,
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
| Result shape      | `LibraryTreeWindow`.                              | `ContentsResult`.                       |
| Coverage shape    | `LibraryTreeCoverage`.                            | `ContentsCoverage`.                     |
| Profile parameter | None (internal row admission).                    | `ContentsReadPolicy` (row profile + media classes). |
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
| Admits or rejects candidates.      | Applies internal library tree row admission policy.       |
| Records observations.              | Returns rows and coverage.                            |
| Updates scan coverage.             | Consumes coverage state already persisted.            |
| Produces `source_files` inventory. | Projects inventory as hierarchy rows.                 |

A directory that has not been scanned shows `coverage.state = Pending` or `Scanning` in the hierarchy read result.
The hierarchy read does not trigger a scan to fill in the gap. That is the scan admission system's responsibility.

## Product boundary vocabulary

The library tree children boundary uses `ReadLibraryTreeChildren` as the product-facing command. Library tree row
admission is an internal product-boundary concern: the renderer does not choose a visibility or policy parameter.

`ChildRowState` (`unknown`, `hasChildRows`, `noChildRows`) is the product-surface signal for directory expandability.
The renderer must use `childRowState` to determine whether a directory row should show an expand affordance, rather
than deriving expandability from `directoryPrimaryMediaState` or `directoryImageMediaState`.

The current product policy admits audio/video rows and excludes image-only files/directories from the library tree.

## Non-goals

This contract does not:

- Make `rowAdmission` canonical as a renderer-facing parameter (it is an internal product-boundary concern).
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
| Hierarchy reads use offset/limit pagination, not cursor pagination.      | Protocol shape; `ReadLibraryTreeChildrenRequest` uses `offset`. |
| Contents reads use cursor pagination, not offset pagination.             | Protocol shape; `ContentsReadRequest` uses `cursor`.       |
| Hierarchy `loadMore` / `loadChildren` must not use `loadContentsPage`.   | Renderer dispatch in `panel.vue`.                         |
| Contents `loadContentsPage` must not use `loadMore` / `loadChildren`.    | Renderer dispatch in `panel.vue`.                         |
| `emptyResultAuthoritative = false` when coverage is not `Complete`.      | Store `literal_hierarchy.rs` coverage computation.        |
| Zero rows with incomplete coverage must not be rendered as "empty."      | Tree projection respects `emptyResultAuthoritative`.      |
| Renderer must not clear visible children while a hierarchy read is pending. | Frame stability contract.                              |
| Hierarchy cache is not an authoritative contents source.                 | Contents reads are independent of hierarchy controller.   |
| Hierarchy reads never trigger filesystem traversal.                      | Reads are from persisted SQLite state.                    |
| Library tree children command exposes no visibility or policy parameter. | `ReadLibraryTreeChildren` has no visibility/policy arg.   |
| Renderer must not derive expandability from `directoryPrimaryMediaState` or `directoryImageMediaState`; use `childRowState` instead. | Product boundary contract. |
