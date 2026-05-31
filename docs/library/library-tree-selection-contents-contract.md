---
status: candidate
doctrine-version: 0.1
last-reviewed: 2026-05-28
owner: renderer-substrate-boundary
canonical-context:
  - library-tree-frame-stability-contract
  - library-row-profile-contract
  - library-contents-read-boundary
  - recursive-selected-contents-rule
  - source-hierarchy-contract
scope:
  - tree-selection-authority
  - contents-scope-derivation
  - contents-panel-read-model
  - selection-invalidation
---

# Library Tree Selection → Contents Contract

## Core law

**Selecting a tree node creates a contents scope. It does not transfer tree authority to the contents panel.**

The tree owns hierarchy. The contents panel owns track rows for a selected scope.
They are different projections of different questions. The tree does not push its
row data into the contents panel. The contents panel reads independently using
the scope the selection produced.

## Ownership boundaries

| Owner           | Owns                                                            | Must not own                             |
|-----------------|-----------------------------------------------------------------|------------------------------------------|
| Tree controller | Expanded/collapsed state, selection state, hierarchy row cache. | Contents rows, contents read state.      |
| Selection model | Selected node identity, derived contents scope.                 | Tree expansion, contents rendering.      |
| Contents panel  | Contents rows, pagination, sort, filter, loading state.         | Tree structure, hierarchy node identity. |
| Substrate       | Both hierarchy and contents projection reads.                   | Renderer state of either panel.          |

## Selection produces a scope, not a row copy

When a tree node is selected, the selection model derives a contents scope from
the selected node's substrate identity.

```
selectedNodeId → contentsScope(kind, id, policy)
```

Scope kinds:

| Tree node kind  | Derived contents scope                                                           |
|-----------------|----------------------------------------------------------------------------------|
| `source_root`   | Media-relevant source-file inventory under this source, subject to contents policy. |
| `directory`     | Media-relevant source-file inventory under this directory, subject to contents policy (recursive or not). |
| `primary_media` | Single track. Contents panel shows track detail or adjacent context.             |
| `segment`       | Single segment. Contents panel shows segment detail or parent disc context.      |
| `blocked`       | Blocked scope. Contents panel shows blocked state, not empty.                    |
| `excluded`      | Excluded scope. Contents panel shows excluded state, not empty.                  |

The scope is a substrate concept, not a path string. It survives source relocation
if the node identity is stable.

## Contents read is independent

The contents panel issues its own read using the derived scope. It does not
receive its rows from the tree cache.

```
readContents(scope, rowProfile, sortPolicy, filterPolicy, limit, cursor)
```

The contents read carries its own epoch guards:

| Guard field            | Meaning                                     |
|------------------------|---------------------------------------------|
| requestId              | Renderer-generated unique request identity. |
| scopeNodeId            | Selected node identity.                     |
| rowProfile             | Row profile active for contents.            |
| sortPolicy             | Sort order at time of issue.                |
| filterPolicy           | Active filter at time of issue.             |
| scanEpochAtIssue       | Scan epoch when request was issued.         |
| projectionEpochAtIssue | Projection epoch when request was issued.   |

Stale response handling mirrors the tree: a response whose guard no longer
matches current state is silently discarded without clearing existing rows.

## Contents panel states

| State          | Meaning                                                             | UI behavior                                                           |
|----------------|---------------------------------------------------------------------|-----------------------------------------------------------------------|
| `no_selection` | No tree node is selected.                                           | Panel shows empty/prompt state.                                       |
| `loading`      | Scope is set; first page read is in flight; no rows yet.            | Show stable loading state. Do not flash empty.                        |
| `partial`      | Some rows are loaded; scan coverage for scope is incomplete.        | Show known rows with partial indicator. Never hide known rows.        |
| `ready`        | Rows loaded; coverage is complete for the current policy and scope. | Render normally.                                                      |
| `empty`        | Coverage is complete; no rows exist under current policy and scope. | Show stable empty state.                                              |
| `blocked`      | The selected scope is blocked or inaccessible.                      | Show blocked/inaccessible state with reason. Do not show empty.       |
| `excluded`     | The selected scope is excluded by scan policy.                      | Show excluded state. Do not show empty.                               |
| `stale`        | A new read is in flight; prior rows are still valid.                | Keep prior rows visible; show refresh indicator if duration warrants. |
| `unavailable`  | The source containing the selected scope is currently unavailable.  | Keep scope selection; show unavailable state for rows.                |

A contents panel must never briefly show empty because a read is in flight.
The `loading` state applies only when no prior row data exists for this scope.

## Partial contents behavior

When a selected directory has incomplete scan coverage, the contents panel shows
what is known and indicates partial state. It does not wait for complete coverage.

The partial indicator must be honest:

| Coverage state | Contents panel behavior                                         |
|----------------|-----------------------------------------------------------------|
| `unscanned`    | Show `loading` or `unknown` state; do not show empty.           |
| `scanning`     | Show known rows (may be zero) with scanning/partial indicator.  |
| `partial`      | Show known rows with `partial` indicator and provisional count. |
| `complete`     | Show all rows with final count.                                 |
| `inaccessible` | Show inaccessible state with reason.                            |
| `excluded`     | Show excluded state.                                            |

## Scan update invalidation for selected scope

When a scan event arrives, the contents panel must check whether the event
affects the currently selected scope.

```
if affectedParentNodeIds.includes(selectedNodeId) or affects scope subtree:
  mark contents stale
  re-read if panel is visible
  keep existing rows until replacement arrives
```

The contents panel does not re-read on every scan chunk. Only chunks that affect
the selected scope trigger a contents refresh.

## Selection survival under source state changes

| Source condition           | Contents panel behavior                                                          |
|----------------------------|----------------------------------------------------------------------------------|
| Source remains mounted     | Normal operation.                                                                |
| Source becomes unavailable | Keep scope selection; show `unavailable` state for rows. Do not clear selection. |
| Source relocates (same ID) | Scope remains valid; re-read may be needed if paths changed.                     |
| Source removed by user     | Clear selection; show `no_selection` state.                                      |

Selection is not cleared by source unavailability. The DJ's intent to view that
scope is preserved even when the source is temporarily offline.

## First-page law

The contents panel reads the first page only on initial scope load. It does not
pre-read subsequent pages. Pagination is user-initiated or scroll-triggered.

This prevents a scope change from silently issuing hundreds of reads across a
large directory.

## Contents panel does not own browse hierarchy

The contents panel reads a flat or policy-shaped list of tracks for a scope.
It does not re-implement hierarchy traversal. If a "show all tracks recursively"
scope is selected, the recursion decision belongs to the browse policy passed to
the read, not to the contents panel iterating subdirectories.

## Non-goals

This contract does not define track detail panels, now-playing context, drag
targets inside the contents panel, or search-scoped contents reads. Those are
governed by downstream contracts once this coupling is stable.
