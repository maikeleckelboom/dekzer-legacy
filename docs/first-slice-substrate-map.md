---
status: candidate
doctrine-version: 0.1
last-reviewed: 2026-05-29
owner: product-architecture
canonical-context:
  - product-doctrine-shortened
  - source-root-scan-admission-contract
  - source-hierarchy-contract
  - library-tree-frame-stability-contract
scope:
  - first-slice-authority-map
  - first-slice-path
  - first-slice-inclusions
  - first-slice-exclusions
  - substrate-owner-roles
  - read-boundary-owners
  - renderer-responsibilities
  - provisional-and-deferred-work
---

# First Slice Substrate Map

## Status and scope

This is the first-slice authority map. It defines the exact first serious Dekzer product slice and prevents agents
from jumping to deck runtime, Prepared Room UI, smart crates, column browser, or broad filter systems too early.

It does not own any read boundary, scan contract, or icon doctrine directly. It declares which contracts are active in
the first slice, which owners hold each substrate surface, and which concepts are explicitly not yet implemented.

## First slice path

The first slice path is:

```
local source root registration
  → source/root classification and scan admission
  → persisted source hierarchy and attachment/media inventory
  → hierarchy reads and contents reads
  → renderer browser projection
```

Each step from left to right represents a product-capable slice. Steps to the right may already be partially implemented
even when steps to the left are still incomplete.

## Included capabilities

The first slice includes the following capabilities, whether fully implemented or partially built:

| Capability                            | Current implementation reality                                          |
|---------------------------------------|-------------------------------------------------------------------------|
| Local source root registration        | Active. Source registration and basic root state exist.                 |
| Source root identity slots            | Provisional path-only handling; durable identity slots exist in schema. |
| Root classification                   | Defined in scan admission contract; partial implementation.             |
| Scan admission                        | Defined in scan admission contract; not yet fully implemented.          |
| Persisted literal source hierarchy    | Active. SQLite `source_directories`, `source_files` with scan coverage. |
| Attachment/media inventory            | Attachment data model active via promotion; scan-integrated inventory stage deferred. |
| Source hierarchy read boundary        | Active. `readLibraryTreeChildren` with offset/limit pagination.         |
| Contents read boundary                | Active. Single parameterized `contentsRead` with cursor pagination.     |
| Renderer tree projection              | Active. Tree with root, directory, file rows under library tree row admission. |
| Renderer contents pane projection     | Active. Contents table with `loadContentsPage` cursor pagination.       |
| Renderer library browser shell        | Active. Panel, source toolbar, browser, split pane.                     |

## Explicit exclusions

The first slice explicitly excludes the following. None of these are first-slice product authority:

| Excluded area                         | Why excluded                                              |
|---------------------------------------|-----------------------------------------------------------|
| Deck runtime                          | Beyond current substrate scope.                           |
| RT Flight Deck                        | Future architecture boundary only.                        |
| Prepared Room UI                      | Future surface.                                           |
| Smart crates / playlists              | Future organization domain.                               |
| Full user-defined filters             | Not first slice. Library tree row admission is product/boundary-owned, not renderer-configurable. |
| Column browser                        | Phase 3 in implementation sequence, not current.          |
| Source relocation UX                  | Beyond root identity slots; defined in lifecycle contract. |
| Broad icon expansion                  | First minimal icon set only.                              |
| Full analysis / prep dashboards       | Future preparation surface.                               |
| Hardware / controller integration     | Future devices domain.                                    |
| Search                                | Future surface.                                           |
| Organization (crates/playlists/etc.) | Future surface.                                           |
| Imports / exports / sleeves / history | Beyond first slice.                                       |

## Active substrate owners

Each domain surface in the first slice has an owning layer:

| Owner                   | Owns                                                      | Must not own                              |
|-------------------------|-----------------------------------------------------------|-------------------------------------------|
| Rust / SQLite substrate | Durable source identity, hierarchy nodes, scan coverage.  | Renderer state, DOM, expansion state.     |
| Boundary protocol       | Typed command/event shapes, generated TS contracts.       | Domain logic, business rules.             |
| Boundary service        | Command dispatch, store orchestration, validation.        | Presentation, UI decisions.               |
| Shared TS types         | Types shared between main and renderer. No logic.         | Domain state, persistence.                |
| Main IPC                | Host-side wiring; Rust service → renderer bridge.         | Renderer state, projection logic.         |
| Renderer                | Tree projection, contents projection, view state, cache.  | Durable substrate state, source scanning. |

Renderers may cache projection/read state, but do not own substrate state. Rust/SQLite own durable substrate state.

Hierarchy reads do not own recursive selected contents. Contents reads do not own source scanning. Scan admission does
not equal media inventory. Empty rows do not automatically mean nothing exists.

## Active read boundaries

Two distinct read boundaries are active in the first slice:

| Read boundary   | Pagination model                  | Action names                          | Owns                              |
|-----------------|-----------------------------------|---------------------------------------|-----------------------------------|
| Hierarchy read  | Offset-based (`offset`/`limit`)   | Tree `loadMore` / `loadChildren`      | Library tree children admitted by the product/boundary surface. |
| Contents read   | Cursor-based (`cursor`/`cursor`)  | Contents pane `loadContentsPage`      | Selected scope file rows.         |

The hierarchy read returns `LibraryTreeWindow` rows with `totalRows`, `nextOffset`, and a
`LibraryTreeCoverage` containing `emptyResultAuthoritative`. In shared TS, the post-mapping
equivalents are `ChildWindow` for the window and `ChildRow` for individual child rows. The contents read returns `ContentsResult`
rows with `nextCursor` and `ContentsCoverage` containing `emptyResultAuthoritative`.

These two boundaries must not share pagination state, cursor/offset tokens, cache entries, or row accumulators.
Hierarchy pagination (`loadChildren`/`loadMore`) and contents pagination (`loadContentsPage`) are distinct.

## Active renderer responsibilities

The renderer owns:

| Surface        | Controller location                                 | Owns                                                |
|----------------|-----------------------------------------------------|-----------------------------------------------------|
| Hierarchy tree | `renderer/library/boundary/hierarchyRead.ts`        | Expansion state, branch cache, row projection.      |
| Contents pane  | `renderer/library/boundary/contentsRead.ts`         | Pagination state, row accumulation, sort.           |
| Selection      | `renderer/library/selection/`                       | Selected node → contents scope derivation.          |
| View state     | `renderer/library/viewState/`                       | Expanded node IDs, split position, session state.   |

The renderer does not own scan scheduling, scan state authority, durable source identity, or media classification
at the substrate level.

## Current implementation reality

The following are true of the current repo as of this writing:

| Fact                                                                 | Observation                              |
|----------------------------------------------------------------------|------------------------------------------|
| `sourceFileVisibility` was implementation debt and has been removed from renderer-facing contracts. | Library tree row admission is now a product/boundary surface concern. |
| Library tree row admission is owned by the product/boundary surface, not the renderer. | Renderer no longer chooses visibility or policy. |
| Tree `loadMore` feeds `nextOffset` into subsequent hierarchy reads.   | Active pagination.                      |
| Contents `loadContentsPage` feeds `nextCursor` into subsequent reads.  | Active pagination.                      |
| `emptyResultAuthoritative` exists in hierarchy and contents coverage.  | Active coverage field.                  |
| `HierarchyCoverage` states include `Complete`, `Pending`, `Scanning`, `Blocked`, `Failed`, `SourceUnavailable`, `LocationMissing`. | Active protocol. |
| Cursor docs are already repaired and describe active contents cursor pagination. | Do not treat as stale. |

## Provisional vocabulary

The provisional `sourceFileVisibility` vocabulary has been removed. Library tree row admission is now an internal product-boundary concern, not a renderer-facing parameter.

## Canonical vocabulary

The following names are canonical first-slice vocabulary:

| Canonical name          | Location / owner                              | Meaning                                       |
|-------------------------|-----------------------------------------------|-----------------------------------------------|
| `readLibraryTreeChildren` | Rust protocol → TS contract            | Product-facing hierarchy read command.        |
| `LibraryTreeWindow` | Rust protocol → TS contract                   | Single page of hierarchy children.            |
| `LibraryTreeCoverage` | Rust protocol → TS contract                 | Coverage for a hierarchy window.              |
| `ContentsReadRequest`   | Rust protocol → TS contract                   | Single contents read request.                 |
| `ContentsResult`        | Rust protocol → TS contract                   | Single contents read result page.             |
| `ContentsReadPolicy`    | Shared TS → Rust service                      | Policy parameter for contents reads.          |
| `ContentsScope`         | Shared TS → Rust service                      | Scope parameter for contents reads.           |
| `ContentsRowProfile`    | Shared TS → Rust store                        | `sourceFile` or `primaryMedia` row profile.   |
| `StoreLiteralHierarchyWindow` | Rust store                            | Store-level hierarchy window.                 |
| `StoreLiteralHierarchyCoverage` | Rust store                          | Store-level hierarchy coverage.               |


## Deferred work

The following work is acknowledged but deferred beyond the first slice:

| Deferred work                              | Why deferred                                       |
|--------------------------------------------|----------------------------------------------------|
| Full scan admission implementation         | Contract exists; implementation is ongoing.         |
| Durable root identity resolver             | Schema slots exist; path-only handling is current.  |
| Column browser                             | Phase 3 in implementation sequence.                 |
| Search, organization, prep, imports, etc.  | Phases 4–10 in implementation sequence.             |

## Rejection cases

A change fails this map if it does any of the following:

| Failure                                                                 | Why rejected                                                         |
|-------------------------------------------------------------------------|----------------------------------------------------------------------|
| Implements deck runtime, Prepared Room, or RT Flight Deck as first slice | Explicitly excluded.                                                 |
| Adds smart crates, playlists, or broad filter systems                    | Explicitly excluded.                                                 |
| Exposes `sourceFileVisibility` or renderer-chosen policy in the library tree product boundary | Visibility/policy is an internal row admission concern, not a renderer parameter. |
| Blurs hierarchy reads with contents reads                                | Distinct pagination models, owners, and result shapes.               |
| Claims hierarchy cache is an authoritative contents source               | Contents reads own selected scope; hierarchy owns tree children only. |
| Treats empty hierarchy rows as meaning nothing exists                    | `emptyResultAuthoritative` must be false when coverage is incomplete. |
| Rolls search, imports, exports, or organization into first slice scope   | Explicitly deferred.                                                 |
| Treats repaired cursor docs as stale                                     | Cursor docs are already current.                                     |
