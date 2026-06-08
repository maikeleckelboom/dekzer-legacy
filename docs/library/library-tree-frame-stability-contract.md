---
status: candidate
ratification-target: library-tree-frame-stability-v1
doctrine-version: 0.3
last-reviewed: 2026-05-28
owner: renderer-substrate-boundary
canonical-context:
  - product/product-doctrine
  - source-root-scan-admission-contract
  - source-hierarchy-contract
  - first-slice-substrate-map
scope:
  - library-tree-rendering
  - hierarchy-projection-reads
  - renderer-branch-state
  - child-row-caching
  - visible-frontier-prefetch
  - scan-progress-invalidation
---

# Library Tree Frame Stability Contract

## Core laws

A tree expansion must never clear a valid visible branch before the replacement projection is ready.

A tree expansion read is a substrate projection read. It is never a filesystem traversal.

This contract exists because local SQLite reads can be fast and still produce visible flicker when renderer state is
cleared before the next projection arrives. The visible failure happens between renderer commits, not inside SQLite.

Tree expansion must feel stable under fast reads, slow reads, scan updates, partial hierarchy discovery, source
relocation, and stale responses.

## Product promise

When a user expands a library tree row, Dekzer must keep the visible structure honest and stable.

The UI may show loading, partial, blocked, excluded, or unknown states. It must not briefly pretend a valid branch is
empty merely because a projection read is pending.

## Scope

This document governs the renderer and substrate boundary for source hierarchy tree expansion.

Future implementation stages are marked explicitly. Concepts such as batch reads, child summaries,
visible-frontier reads, and targeted scan invalidation by parent IDs are future architecture.

It covers:

| Area                   | Included                                                                   |
| ---------------------- | -------------------------------------------------------------------------- |
| Tree row identity      | Stable renderer keys grounded in substrate-assigned hierarchy node IDs.    |
| Expansion behavior     | Immediate expansion without clearing valid visible children.               |
| Child projection reads | Reads from persisted hierarchy/projection state, not filesystem traversal. |
| Child summaries        | Count, completeness, coverage, and chevron affordance state.               |
| Renderer cache         | Non-authoritative cache for branch children and summaries.                 |
| Cache invalidation     | Targeted vs full clear rules.                                              |
| Stale responses        | Request epoch/revision guards.                                             |
| Prefetch               | Bounded visible-frontier prefetch only.                                    |
| Scan progress updates  | Affected parent invalidation without whole-tree thrash.                    |

## Non-goals

This document does not define source scanning, source-root admission, media candidate admission, audio inspection, CUE
segmentation, collection organization, search, or virtualized infinite pagination.

Tree expansion consumes already-persisted hierarchy and projection state. If a branch is unscanned, queued, partial,
inaccessible, or excluded, the tree renders that state. It does not scan the filesystem on click.

## Required concepts

| Concept          | Meaning                                                                                                                   |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Hierarchy node   | Substrate-owned persisted node representing a source root, source location, directory, or hierarchy state/action subject. |
| Tree row         | Renderer projection of a navigation hierarchy node under the current product policy.                                      |
| Row key          | Stable renderer key derived from substrate-assigned node identity, never from path alone.                                 |
| Expansion state  | Renderer/session-owned affordance state: expanded or collapsed.                                                           |
| Child state      | Projection-owned state describing what is known about a row's children.                                                   |
| Coverage state   | Substrate-owned scan coverage for the hierarchy under a node.                                                             |
| Projection epoch | Version/revision representing the projection policy or materialized row view.                                             |
| Scan epoch       | Version/revision representing hierarchy discovery progress.                                                               |
| Read epoch       | Epoch values captured when a child read request is issued.                                                                |
| Visible frontier | Expanded and near-visible tree branches currently relevant to the viewport.                                               |
| Branch cache     | Renderer-side non-authoritative cache of child rows and summaries.                                                        |

## Node identity law

Tree row keys must be grounded in substrate-assigned node identity.

Do not use filesystem paths as renderer row keys.

Paths are display claims and current resolution claims. They can change when a drive remounts under another letter, a
source root relocates, a cloud provider changes a local path, a junction resolves differently, or a folder is renamed.

Correct row identity:

| Identity source                      |      Allowed as row key? | Reason                                                         |
| ------------------------------------ | -----------------------: | -------------------------------------------------------------- |
| Substrate-assigned hierarchy node ID |                      Yes | Stable for the persisted node and independent of current path. |
| Opaque substrate UUID                |                      Yes | Stable and intentionally non-semantic.                         |
| Database autoincrement node ID       | Yes, if scoped correctly | Stable within the local substrate database.                    |
| Filesystem path                      |                       No | Path is a resolution/display claim, not identity.              |
| Row index                            |                       No | Reorders destroy DOM stability.                                |
| Display label                        |                       No | Labels collide and change.                                     |

Required rule:

**Tree row keys must be substrate-assigned node identifiers, never filesystem paths.**

Tree node IDs used as renderer row keys are substrate-assigned node identifiers. Filesystem paths are display claims.
Row key stability survives drive remounting and source root relocation only when node IDs are substrate-assigned and
preserved by the hierarchy substrate.

## Ownership boundaries

| Owner                    | Owns                                                         | Must not own                           |
| ------------------------ | ------------------------------------------------------------ | -------------------------------------- |
| Source scanner           | Discovery, scan coverage, skipped/inaccessible observations. | Renderer expansion state.              |
| Library substrate        | Persisted hierarchy node identity and source coverage.       | DOM realization.                       |
| Projection read model    | Tree rows, child summaries, policy-shaped row output.        | Filesystem traversal.                  |
| Renderer tree controller | Expanded/collapsed state, pending read state, cache, focus.  | Durable hierarchy meaning.             |
| DOM/Vue realization      | Rendering stable keyed rows.                                 | Identity, source truth, scan coverage. |

Expansion state is immediate and renderer-owned. Child truth and coverage are substrate/projection-owned.

## Tree row shape

A tree row projection must carry enough state for stable rendering.

| Field           | Meaning                                                                                        |
| --------------- | ---------------------------------------------------------------------------------------------- |
| nodeId          | Substrate-assigned hierarchy node identity.                                                    |
| rowKey          | Stable renderer key derived from nodeId and row-scope where needed.                            |
| parentNodeId    | Parent node identity, if any.                                                                  |
| label           | Display label.                                                                                 |
| pathDisplay     | Optional display path or current resolution claim.                                             |
| kind            | source_root, source_location, directory, read_state, action, etc.                              |
| childState      | unknown, queued, loading, ready, empty, partial, blocked, inaccessible, excluded.              |
| coverageState   | unscanned, queued, scanning, partial, complete, inaccessible, excluded.                        |
| knownChildCount | Number of known child rows under current policy.                                               |
| isCountComplete | Whether knownChildCount is final for the current coverage and policy.                          |
| projectionEpoch | Projection epoch used to produce this row.                                                     |
| scanEpoch       | Scan epoch visible to this row.                                                                |
| reasonCodes     | Optional reason codes for blocked, partial, inaccessible, excluded, degraded, or stale states. |

Counts must be honest. A directory with 14 known children during partial coverage renders as `14+`, not `14`.

Renderer rule:

| `isCountComplete` | Render                   |
| ----------------- | ------------------------ |
| `true`            | Exact count: `14`        |
| `false`           | Provisional count: `14+` |

Never display a provisional count as final.

## Child states

| State        | Meaning                                                                                | UI behavior                                                                        |
| ------------ | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| unknown      | Children are not loaded into the renderer and no more precise work state is available. | Show chevron if summary says possible children; on expand show stable loading row. |
| queued       | Scan or projection work has admitted this branch but has not traversed/read it yet.    | Show discovered/pending state if the work queue is queryable.                      |
| loading      | A child projection read is in flight.                                                  | Keep prior children or show branch-local loading row.                              |
| ready        | Children are loaded and count is complete under current policy.                        | Render children normally.                                                          |
| empty        | Coverage is complete and no children exist under current policy.                       | Render stable empty state if expanded.                                             |
| partial      | Some children are known, but coverage or projection is incomplete.                     | Render known children plus partial marker/count suffix.                            |
| blocked      | A hard condition prevents child access.                                                | Render blocked row state with reason.                                              |
| inaccessible | Permissions, mount, provider, or source state prevents listing.                        | Render inaccessible state with reason.                                             |
| excluded     | Scan policy deliberately excluded this subtree.                                        | Render skipped/excluded state if visible.                                          |

Queued is distinct from unknown and loading. If the current job/work infrastructure cannot expose queued state in the
first slice, unknown may temporarily cover it, but the contract must preserve the distinction.

## Expansion behavior

On expand:

1. Mark the row expanded immediately.
2. Look up cached child rows and child summary for the row key.
3. If cached rows are valid for the current epochs and policy, render them immediately.
4. If cached rows exist but are stale, keep them visible and mark the branch refreshing unless the policy changed
   incompatibly.
5. If no cached rows exist, render a stable branch-local loading row.
6. Issue a child projection read with captured scan/projection epochs.
7. Accept the response only if its guard still matches current state.
8. Patch only the target branch.
9. Schedule bounded visible-frontier prefetch.

Forbidden behavior:

| Forbidden behavior                                | Why                                              |
| ------------------------------------------------- | ------------------------------------------------ |
| Clearing children before a read returns           | Creates empty-frame flicker.                     |
| Collapsing a branch while loading                 | Lies about user intent.                          |
| Rebuilding the whole tree for one branch response | Destroys DOM stability.                          |
| Keying rows by array index                        | Causes DOM churn on insert/remove.               |
| Keying rows by path                               | Breaks under relocation/remount.                 |
| Treating unknown as empty                         | Hides incomplete discovery.                      |
| Triggering filesystem traversal from expand click | Couples UI interaction to source scan mechanics. |

## Child projection reads (partial: single-parent reads active; batch/summary/frontier reads are future)

The substrate should expose child projection reads that are shaped for branch patching and batching.

Current read surface:

| Read                                      | Purpose                         |
| ----------------------------------------- | ------------------------------- |
| readChildren(parentNodeId, offset, limit) | Read child rows for one parent. |

Future recommended read surfaces:

| Read                                                                               | Purpose                                                      |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| readChildrenBatch(parentNodeIds, rowProfile, policy, limit)                        | Batch multiple branch reads into one boundary call.          |
| readChildSummary(parentNodeIds, rowProfile, policy)                                | Read chevron/count/coverage summary without full child rows. |
| readVisibleFrontier(rootNodeId, expandedNodeIds, viewportHint, rowProfile, policy) | Read the currently visible and near-visible branch frontier. |

Child summary shape:

| Field            | Meaning                                                                 |
| ---------------- | ----------------------------------------------------------------------- |
| parentNodeId     | Parent being summarized.                                                |
| knownChildCount  | Number of known child rows for the current row policy.                  |
| isCountComplete  | True only when count is final for current coverage/policy.              |
| hasKnownChildren | True when knownChildCount > 0.                                          |
| mayHaveChildren  | True when coverage or type suggests children may appear later.          |
| coverageState    | complete, partial, unscanned, queued, scanning, inaccessible, excluded. |
| scanEpoch        | Scan epoch used for summary.                                            |
| projectionEpoch  | Projection epoch used for summary.                                      |
| reasonCodes      | Optional explanation for non-ready states.                              |

A partial count must render as provisional, such as 14+.

## Request guards and stale responses (partial: request guards active; projection policy key and sort policy are future)

Every child read request must carry the projection context under which it was issued.

Request guard fields (current):

| Field                  | Meaning                                         |
| ---------------------- | ----------------------------------------------- |
| requestId              | Renderer-generated unique request identity.     |
| parentNodeId           | Branch being read.                              |
| scanEpochAtIssue       | Scan epoch known when request was issued.       |
| projectionEpochAtIssue | Projection epoch known when request was issued. |
| cacheGenerationAtIssue | Renderer cache generation for this policy/root. |

Request guard fields (future — not active in current implementation):

| Field               | Meaning                                       |
| ------------------- | --------------------------------------------- |
| rowProfile          | Row profile/filter policy for the request.    |
| sortPolicy          | Sort order in effect when request was issued. |
| projectionPolicyKey | Filter/media policy key for the request.      |

Response acceptance rule:

A response is accepted only when the current projection context, projection epoch, and relevant scan epoch still match
the request guard, or when the response explicitly declares which affected parent IDs remain valid under the newer
epoch.

If a scan chunk lands between request issue and response arrival and affects the requested parent, the response is
stale. The renderer must ignore it and re-request the branch.

If an unrelated scan chunk lands elsewhere, the response may still be accepted if affected-parent invalidation proves
this parent was not touched.

Stale response handling must be silent and stable: do not flash empty rows, do not collapse, and do not replace current
children with older data.

## Renderer branch cache (future: projectionPolicyKey is future)

The renderer may maintain a non-authoritative branch cache.

Cache key (current):

| Component        | Reason                                          |
| ---------------- | ----------------------------------------------- |
| sourceRootNodeId | Separate source roots.                          |
| parentNodeId     | Branch identity.                                |
| projectionEpoch  | Guards against incompatible projection changes. |

Cache key (future — not active in current implementation):

| Component           | Reason                                                                                                                                                    |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| rowProfile          | Different row shapes may produce different children.                                                                                                      |
| sortPolicyKey       | Different sort orders produce different row order. A cache entry produced under one sort policy must not satisfy a request under a different sort policy. |
| projectionPolicyKey | Filter/media policy affect child rows.                                                                                                                    |

Sort policy is not part of `rowProfile`, but it is part of the cache key and request identity. They are different axes:
profile governs which node kinds appear; sort governs their order.

Cache value:

| Field           | Meaning                              |
| --------------- | ------------------------------------ |
| children        | Last accepted child rows.            |
| summary         | Last accepted child summary.         |
| scanEpoch       | Scan epoch used when accepted.       |
| projectionEpoch | Projection epoch used when accepted. |
| sortPolicyKey   | Sort policy in effect when accepted. |
| loadedAt        | Local time for eviction heuristics.  |
| refreshing      | Whether a newer read is in flight.   |

The cache is not authority. It may be discarded. It must not invent children or readiness.

## Cache invalidation

Do not treat all epoch changes as full cache clears.

| Event                                                | Correct invalidation scope                                                                                                                                                      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source root identity changes                         | Full clear for that root. Node identities may no longer refer to the same substrate branch.                                                                                     |
| Source root becomes unavailable or unmounted         | Keep last-known rows for that root; mark branch degraded/inaccessible. Do not clear.                                                                                            |
| Source root relocation with preserved node identity  | Retain cache if node identities and projection epoch remain valid; refresh display paths.                                                                                       |
| Projection policy/filter changes                     | Full clear for that policy key because child shape/order may change.                                                                                                            |
| Sort order changes                                   | Re-read affected branch order; keep existing rows visible until replacement arrives. The branch must not go empty between the old order being stale and the new order arriving. |
| Projection epoch changes globally                    | Full clear unless affected-parent metadata is available.                                                                                                                        |
| Scan epoch advances with affected parent IDs         | Targeted invalidation for affected parents and ancestors whose summaries changed.                                                                                               |
| Scan epoch advances without affected parent metadata | Conservative targeted refresh for visible branches; avoid full clear if possible.                                                                                               |
| Branch-specific read failure                         | Mark that branch stale/failed; do not clear siblings.                                                                                                                           |
| Manual refresh of one branch                         | Invalidate that branch only.                                                                                                                                                    |

Scan updates invalidate affected parent IDs, not the entire tree. Cache invalidation must follow the same rule.

## Scan progress integration (partial implementation)

Scan progress must not thrash the visible tree.

Current implementation reality:

- Cursor-only boundary events exist (`ReadAfter` with `eventSequence` cursors).
- `SourceScanEvent` and `MaintainedSnapshotInvalidated` are distinct event families.
- Desktop Main polls events and forwards batches; the renderer updates scan progress state per root
  from Main-delivered events.
- Scan events do not currently provide `affectedParentNodeIds` for targeted invalidation.
- Event gaps (`gapDetected`) trigger authoritative snapshot refresh/recovery, not user re-add/rescan blame.
- True live scan progress requires background scan jobs (not yet implemented; `runRootScan` is synchronous).
- The renderer consumes scan events through the boundary events controller but does not author scan
  progress.

Future scan event fields (not yet implemented):

| Field                  | Meaning                                                |
| ---------------------- | ------------------------------------------------------ |
| scanEpoch              | New scan epoch.                                        |
| affectedParentNodeIds  | Parents whose child lists changed or may have changed. |
| affectedSummaryNodeIds | Nodes whose counts/coverage changed.                   |
| coverageChanges        | Partial/complete/blocked/excluded transitions.         |
| reasonCodes            | Optional explanation for visible state updates.        |

Renderer behavior:

1. Update current scan epoch.
2. Mark affected cached branches stale.
3. If an affected branch is visible or expanded, schedule refresh.
4. If not visible, do nothing until user navigates there.
5. Preserve existing visible children while refresh is pending.

## Prefetch policy

Prefetch is an optimization after frame stability exists. It must not hide a clearing bug.

Allowed prefetch targets:

| Trigger                           | Prefetch target                                                      |
| --------------------------------- | -------------------------------------------------------------------- |
| Root opened                       | First-level child summary and visible child rows.                    |
| Row expanded                      | Expanded row's children and summary for immediate child directories. |
| Keyboard focus moves              | Nearby sibling summaries.                                            |
| Hover/focus on chevron            | That row's child summary or children if cheap.                       |
| Scan chunk affects visible parent | Refresh affected visible branch.                                     |

Stopping rules:

| Rule                 | Requirement                                                                                         |
| -------------------- | --------------------------------------------------------------------------------------------------- |
| Depth bound          | Prefetch extends at most one level below the deepest visible row in the current viewport.           |
| Viewport bound       | Prefetch never crosses into subtrees outside the current viewport's expansion depth.                |
| Count bound          | Prefetch at most N child summaries or rows per unit, where N is policy-defined.                     |
| Work budget          | Prefetch yields under the same scheduler as normal branch reads.                                    |
| No recursive cascade | Prefetched children do not recursively trigger more prefetch unless they become visible or focused. |
| No scan trigger      | Prefetch reads projection state only. It does not start filesystem traversal.                       |

This prevents one expanded row from quietly crawling the whole collection.

Default bounds (adjustable by configuration):

| Limit                                    | Default |
| ---------------------------------------- | ------: |
| Prefetch depth below deepest visible row |       1 |
| Max prefetch parent rows per tick        |       8 |
| Max prefetched child rows per parent     |      64 |
| Max total prefetched rows per cycle      |     256 |
| Hover prefetch delay                     |  120 ms |
| Loading indicator delay                  |   80 ms |

The loading indicator delay prevents spinner flash on fast local reads. A read completing in under 80 ms must not show a
loading indicator at all.

## Renderer commit discipline

Branch updates must be patched, not rebuilt wholesale.

Required renderer behavior:

| Rule                | Requirement                                                          |
| ------------------- | -------------------------------------------------------------------- |
| Stable keys         | Use substrate node IDs for row keys.                                 |
| Branch patching     | Replace children only under the target parent.                       |
| State retention     | Preserve existing branch rows while refresh is pending.              |
| Request dedupe      | Coalesce duplicate reads for the same cache key.                     |
| Request batching    | Batch parent reads issued in the same UI tick.                       |
| Out-of-order safety | Ignore stale responses by request guard.                             |
| Local pending state | Show loading/refreshing inside the branch, not as global tree churn. |
| No empty interframe | Never commit an empty child list solely because a read is pending.   |

## Loading vs refreshing

`loading` and `refreshing` are distinct states with distinct rendering rules.

| Condition                                        | Correct state                                                                                     |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| No prior child projection exists; read in flight | `childState = loading`; show branch-local loading row.                                            |
| Prior children exist; newer read in flight       | `childState` remains `ready` or `partial`; cache `refreshing = true`; keep existing rows visible. |
| Read completes with new data                     | Patch branch; clear `refreshing`.                                                                 |
| Read fails with prior data still valid           | Keep prior rows; mark branch `stale`; do not set `loading`.                                       |

**Never use `loading` to mean "refreshing existing rows."** A branch that has prior children must never briefly show
empty because a new read was issued.

## Drag stability

A tree row participating in an active drag operation is frozen from the perspective of the cache and DOM keying system.

Required behavior during active drag:

| Rule                                          | Requirement                                                                                                  |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Drag source row key stability                 | The dragged row's node ID and DOM key must not change while the drag is in progress.                         |
| Branch refresh does not remove drag source    | A cache invalidation or branch patch must not remove or re-key the active drag source row.                   |
| Sibling updates are allowed                   | Non-dragged siblings may update normally.                                                                    |
| Drag resolves or cancels before branch clears | If the drag source row's branch must clear (policy change, root identity change), defer until drag resolves. |

UX result: any future draggable tree row must remain stable even if a scan chunk arrives and refreshes the parent branch
mid-gesture. The current navigation-only tree does not expose audio rows as drag sources.

## Source unavailable behavior

A known source does not disappear merely because its current filesystem resolution is unavailable.

| Source condition       | Tree behavior                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------------ |
| Mounted and accessible | Render normally.                                                                           |
| Unmounted / ejected    | Keep root row and last-known child structure; render in `inaccessible` presentation.       |
| Relocation in progress | Keep root row; update display path when new resolution is confirmed.                       |
| Blocked (permissions)  | Keep root row; render `blocked` with reason code.                                          |
| Partially accessible   | Keep root row; mark affected subtrees `inaccessible`; accessible branches render normally. |
| Removed by user        | Remove from tree. User-initiated removal is the only case where the row disappears.        |

Ejecting a USB drive must not erase the source from the visible tree. The DJ's mental map of their library is preserved
in the substrate and must be reflected in the UI. The source row communicates unavailability, not absence.

Fix frame stability before prefetch.

Recommended order (items marked "(future)" are not current implementation targets):

1. Ensure tree rows use substrate-assigned node IDs as renderer keys.
2. Add explicit childState, coverageState, knownChildCount, and isCountComplete to tree row/summary projections.
3. Change expand behavior so it never clears current children while a read is pending.
4. Patch one branch at a time instead of rebuilding the whole visible tree.
5. Add request guards with requestId, scanEpochAtIssue, projectionEpochAtIssue. (future: rowProfile, projectionPolicyKey)
6. Add stale-response rejection.
7. Add branch cache keyed by parent node ID. (future: row profile and projectionPolicyKey)
8. Add targeted cache invalidation from affected parent IDs. (future: affected-parent metadata not yet provided by scan events)
9. Add batch child-summary/children reads for visible branches. (future)
10. Add bounded visible-frontier prefetch. (future)
11. Add queued child state if the scan/work queue can expose it. (future)

Prefetch before frame stability is fixed is caching a bug.

## Acceptance criteria

### Expansion stability

| Scenario                             | Required result                                                             |
| ------------------------------------ | --------------------------------------------------------------------------- |
| User expands cached row              | Children appear in the same frame from cache.                               |
| User expands uncached row            | Parent remains expanded and stable; branch-local loading row appears.       |
| Read takes multiple frames           | Existing children stay visible or loading row stays stable.                 |
| Read fails                           | Branch shows failed/blocked/inaccessible state without collapsing siblings. |
| Response returns out of order        | Stale response is ignored.                                                  |
| Scan updates same parent during read | Response is rejected or branch re-requests under new epoch.                 |

### Identity stability

| Scenario                                               | Required result                                              |
| ------------------------------------------------------ | ------------------------------------------------------------ |
| Source root remounts with preserved substrate identity | Row keys remain stable where node identities are preserved.  |
| Directory display path changes                         | Row key does not change solely because display path changed. |
| Sibling order changes                                  | Vue/DOM preserves rows by node ID, not index.                |

### Child summary honesty

| Scenario                                             | Required result                                        |
| ---------------------------------------------------- | ------------------------------------------------------ |
| Directory has 14 known children and partial coverage | Renderer shows provisional count such as 14+.          |
| Directory has 14 children and complete coverage      | Renderer shows final count 14.                         |
| Directory is excluded                                | Renderer shows excluded/skipped state, not empty.      |
| Directory is inaccessible                            | Renderer shows inaccessible state, not empty.          |
| Directory is queued but not scanned                  | Renderer shows queued if available, otherwise unknown. |

### Cache and invalidation

| Scenario                                 | Required result                                                              |
| ---------------------------------------- | ---------------------------------------------------------------------------- |
| Scan epoch advances for unrelated parent | Current visible branch is not cleared.                                       |
| Scan epoch advances for visible parent   | Branch is marked stale and refreshed without empty-frame flicker.            |
| Projection policy changes                | Cache for that policy is cleared and rows reload under stable loading state. |
| Source root identity changes             | Cache for that root is cleared.                                              |
| Source root relocates with same identity | Cache may be retained if node IDs remain valid; display path refreshes.      |

### Prefetch bounds

| Scenario                                     | Required result                                                     |
| -------------------------------------------- | ------------------------------------------------------------------- |
| User expands one deep branch                 | Prefetch does not cascade into unrelated roots or whole collection. |
| Artist folder has hundreds of subdirectories | Prefetch uses a bounded first-N policy and work budget.             |
| Prefetch misses                              | Manual expansion still uses frame-stable loading behavior.          |

## Rejection cases

A change fails this contract if it does any of the following:

| Failure                                                      | Why rejected                                                    |
| ------------------------------------------------------------ | --------------------------------------------------------------- |
| Clears branch children before replacement projection arrives | Causes empty-frame flicker.                                     |
| Uses filesystem path as renderer row key                     | Breaks stability under relocation/remount.                      |
| Uses array index as row key                                  | Breaks stability under insert/remove/reorder.                   |
| Starts filesystem traversal from tree expansion              | Couples UI interaction to scan mechanics.                       |
| Treats unknown/partial/inaccessible/excluded as empty        | Lies about substrate coverage.                                  |
| Clears full tree cache on every scan epoch                   | Causes avoidable flicker and destroys useful cache.             |
| Accepts stale child response after affected scan update      | Shows old children in a newer hierarchy world.                  |
| Displays partial child count as final                        | Misrepresents coverage.                                         |
| Lets prefetch recursively crawl beyond visible frontier      | Recreates broad-root scanning pressure at the projection layer. |
| Rebuilds whole tree for one branch update                    | Causes DOM churn and focus loss.                                |

## Review checklist

Before accepting a tree-related change, ask:

| Question                                                                                                                 | Required answer                        |
| ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------- |
| Are row keys substrate-assigned node IDs?                                                                                | Yes.                                   |
| Are paths treated as display/resolution claims only?                                                                     | Yes.                                   |
| Does expand keep valid children visible while pending?                                                                   | Yes.                                   |
| Does a tree expansion read avoid filesystem traversal?                                                                   | Yes.                                   |
| Are unknown, queued, loading, ready, empty, partial, blocked, inaccessible, and excluded distinct enough for this slice? | Yes, or queued is explicitly deferred. |
| Do child summaries report knownChildCount and isCountComplete?                                                           | Yes.                                   |
| Do read requests carry epoch/revision guards?                                                                            | Yes.                                   |
| Are stale responses ignored?                                                                                             | Yes.                                   |
| Is scan invalidation targeted by affected parent IDs where possible?                                                     | Yes.                                   |
| Is prefetch bounded to the visible frontier?                                                                             | Yes.                                   |
| Does the renderer patch branches instead of rebuilding the whole tree?                                                   | Yes.                                   |

## Ratification note

This contract is ratifiable once the current tree projection and renderer controller names are mapped to the concepts
above.

Amendments 1–8 (node ID grounding, targeted cache invalidation, stale response guards, partial count shape, prefetch
bounds, loading vs refreshing, drag stability, and source unavailable behavior) are required for ratification. They
affect frame behavior directly and are not optional polish.

This contract intentionally does not govern:

- Tree row admission and contents-row separation (see `source-hierarchy-contract`,
  `library-tree-selection-contents-contract`, and `contents-policy-shape`)
- Tree selection → contents coupling (see `library-tree-selection-contents-contract`)
- Contents row presentation (see `library-tree-selection-contents-contract` and `library-contents-browse-policy`)
- Source lifecycle visible states (see `source-lifecycle-visible-state-contract`)

It does require the first implementation to stop empty-frame flicker by preserving branch state, using stable substrate
node IDs, guarding stale responses, and treating tree reads as projection reads rather than filesystem traversal.
