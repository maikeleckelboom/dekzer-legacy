---
status: candidate
doctrine-version: 0.2
last-reviewed: 2026-06-06
owner: renderer-substrate-boundary
canonical-context:
  - frame-stability-contract
  - browse-policy
  - read-boundary-contract
  - source-hierarchy-contract
scope:
  - tree-selection-authority
  - contents-scope-derivation
  - contents-panel-read-model
  - contents-refresh-continuity
  - selection-invalidation
---

# Library Tree Selection Contents Contract

## Core Law

Selecting a tree navigation row creates a contents scope. It does not transfer tree authority to the contents panel.

The tree owns navigation rows: sources, source locations, directories, and hierarchy state/action rows. The contents
panel owns audio/file row presentation for the selected scope. The tree does not push row data into contents, and
contents does not read from the tree cache.

## Ownership Boundaries

| Owner                      | Owns                                                                         | Must not own                                               |
| -------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Tree controller            | Expanded/collapsed state, selection state, hierarchy row cache.              | Contents rows, contents read state.                        |
| Selection model            | Selected navigation row identity and derived contents scope.                 | Tree expansion, contents rendering.                        |
| Contents panel             | Contents rows, pagination, loading and refresh presentation.                 | Tree structure, hierarchy node identity, browse authority. |
| Contents policy/read model | Policy discriminant, variant filters, scopeDepth, ordering, cursor identity. | Renderer-local sort/filter authority.                      |
| Substrate                  | Hierarchy and contents projection reads.                                     | Renderer state of either panel.                            |

Renderer projection may display labels, icons, state rows, and actions for rows returned by the contents read. It must
not sort, filter, fan out hierarchy children, or synthesize rows to invent browse authority.

## Selection Produces Scope

Current selectable navigation rows derive contents scopes as follows:

| Tree row            | Derived contents scope                                                                       |
| ------------------- | -------------------------------------------------------------------------------------------- |
| Source row          | Media-relevant source-file inventory under that source, subject to contents policy.          |
| Source-location row | Media-relevant source-file inventory under that source location, subject to contents policy. |
| Directory row       | Media-relevant source-file inventory under that directory, subject to contents policy.       |
| State/action row    | No contents scope unless its action explicitly loads a page or branch.                       |

The scope is an identity tuple, not a row copy and not a path string. It includes the selected source or directory
identity plus the contents policy and scope depth that shape the read.

## Contents Read Contract

The contents panel issues an independent read:

```text
readContents(scope, policy, scopeDepth, limit, cursor)
```

Tree selection establishes scope only. The initial **Audio** workflow filter supplies `audioBrowse`:

| Field         | Initial Audio value |
| ------------- | ------------------- |
| `policy.kind` | `audioBrowse`       |
| `scopeDepth`  | recursive           |

Switching to **Media** supplies `playableMediaBrowse`. Switching to **All Files** supplies an explicit
`sourceFileInventory` policy. None of these filter changes alter tree containment or selection.

Contents rows are media-relevant source-file rows unless a future explicit policy says otherwise. They are not
canonical tracks and do not decide track identity, duplicate resolution, CUE association, artwork role, or analysis
readiness.

## Contents Panel States

| State          | Meaning                                                                                                 | Required behavior                                                                    |
| -------------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `no_selection` | No selectable navigation row is selected.                                                               | Clear contents intentionally.                                                        |
| `loading`      | A scope is set, no prior accepted rows exist, and the first read is pending past the display threshold. | Show stable loading state. Do not flash empty.                                       |
| `ready`        | Rows loaded for the current policy and scope.                                                           | Render returned rows normally.                                                       |
| `partial`      | Some rows are known, but coverage is incomplete.                                                        | Show known rows with incomplete-coverage indication.                                 |
| `empty`        | Coverage is complete and no rows exist under the current policy and scope.                              | Show stable empty state.                                                             |
| `blocked`      | The selected scope is blocked or inaccessible.                                                          | Show blocked/inaccessible state, not empty.                                          |
| `refreshing`   | A newer read is pending while prior accepted rows are retained.                                         | Keep prior rows visible and mark refresh only when the pending threshold is crossed. |
| `unavailable`  | The source containing the selected scope is unavailable.                                                | Keep scope selection and show unavailable/degraded row state when rows are retained. |

Retained rows are perception continuity. They are not data authority. Replacement data must come from an accepted
contents read, and stale responses must be rejected by request identity, scope, policy, scopeDepth, cursor, and boundary
validation.

## Refresh And Invalidation

Maintained snapshot invalidations and scan events do not carry replacement rows. Renderer refresh planning coalesces the
affected visible/current work, then asks Main for authoritative reads.

Current scoped refresh behavior:

| Input                         | Refresh planning                                                                                                                   |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `navigationRows` invalidation | Refresh navigation rows and visible source lifecycle state.                                                                        |
| `contents` invalidation       | Refresh current contents, clear contents warm snapshots, and refresh visible source lifecycle state.                               |
| Event gap recovery            | Refresh active first-slice projections by bounded policy and clear contents warm snapshots.                                        |
| Source scan events            | Refresh visible source lifecycle state; contents and tree rows update through maintained snapshot invalidations or explicit reads. |

Warm contents prefetch is runtime warmth only. A warm snapshot may satisfy a matching request before it expires, but it
does not become durable truth, does not broaden policy, and is cleared by relevant invalidations or generation changes.

Pending state is user-perception behavior. Fast reads that complete before the threshold do not need visible pending
chrome; slow reads must not erase valid prior rows while pending.

## Source Add, Scan, And Remove

Source registration and scan update the browser through authoritative reads and maintained snapshot invalidations. The
renderer may request refreshes, but source identity, scan coverage, contents rows, and hierarchy rows come from the
boundary/store path.

Source removal is intentional clearing:

1. Clear the selected node.
2. Clear contents.
3. Clear expanded tree state for that source.
4. Persist the empty view state.

Retained prior rows must not prove removed source visibility after an accepted refresh or explicit removal result.

## Panel Containment

Contents rows must be browsable inside the library panel. The contents table owns its internal scroll region and must
not require app-level overflow to reach rows.

This is an acceptance rule, not a CSS implementation contract. Any implementation may choose different layout
mechanics, but the resulting browser must keep tree and contents browsing contained within the library panel bounds.

## Non-Goals

This contract does not define track detail panels, deck loading, search-scoped contents reads, renderer-side
filter/sort controls, or future audio browse fields and surfaces beyond the implemented V0 contents profile.
