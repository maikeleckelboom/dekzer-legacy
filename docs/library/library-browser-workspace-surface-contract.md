---
status: candidate
doctrine-version: 0.1
last-reviewed: 2026-05-28
owner: workspace-library-boundary
canonical-context:
  - product-doctrine-shortened
  - library-tree-frame-stability-contract
  - library-row-profile-contract
  - library-tree-selection-contents-contract
  - source-lifecycle-visible-state-contract
  - workspace-topology-negotiation-law
  - workspace-layout-contract (TODO: not yet written)
scope:
  - library-browser-surface-identity
  - workspace-topology-handoff
  - geometry-and-viewport-hints
  - internal-layout-presets
  - authority-partition
---

# Library Browser Workspace Surface Contract

## Core laws

1. **The Library Browser is a workspace surface. Its rows are not workspace topology.**
2. **Workspace topology owns placement, sizing, visibility, and surface composition.**
3. **Library projection owns source, row, selection, contents, track, segment, and readiness meaning.**
4. **Renderer realization owns frame-stable painting inside the surface.**
5. **Topology may provide geometry and viewport hints. It must not own scan state, row authority, source identity, or
   selection.**

## The boundary

The workspace topology slot containing the Library Browser is an opaque container
from the library browser's perspective. The library browser is an opaque tenant
from the topology's perspective.

They exchange exactly two things:

| Direction          | What is exchanged                                           |
| ------------------ | ----------------------------------------------------------- |
| Topology → Library | Surface bounds, viewport dimensions, visibility state.      |
| Library → Topology | Preferred minimum dimensions, surface identity for routing. |

Nothing else crosses the boundary. The topology does not know what sources,
directories, tracks, or row profiles exist. The library browser does not know
what slot it occupies, how many panels surround it, or what the workspace
layout algorithm decided.

## Ownership partition

| Owner              | Owns                                                                                               | Must not own                                                                             |
| ------------------ | -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Workspace topology | Surface existence, placement, sizing, visibility, tab/split/dock composition.                      | Scan state, row authority, source identity, selection.                                   |
| Library Browser    | Tree state, contents state, selection, source visible projection/presentation state, branch cache. | Surface placement, workspace slot identity, layout geometry, source lifecycle authority. |
| Library substrate  | Hierarchy node IDs, source records, source lifecycle state, track identity, preparation evidence.  | DOM realization, renderer state.                                                         |
| Renderer           | Frame-stable painting of tree and contents rows inside the surface bounds.                         | Source meaning, hierarchy authority, source lifecycle.                                   |

## Surface identity

The Library Browser is registered in workspace topology as a named surface kind:

```
surfaceKind: library_browser
```

A workspace may contain at most one Library Browser surface in v1. Multiple
instances are a future concern and not governed here.

The Library Browser surface is identified by kind, not by slot position. If the
topology moves the Library Browser to a different slot, the library browser's
internal state (tree expansion, selection, scroll position, branch cache) is
preserved. Slot changes are geometry changes, not identity changes.

## What topology provides

Topology provides the Library Browser with its rendered bounds:

| Provided value  | Meaning                                                     |
| --------------- | ----------------------------------------------------------- |
| `surfaceBounds` | Available width and height for the surface.                 |
| `isVisible`     | Whether the surface is currently visible in the workspace.  |
| `viewportHint`  | Optional hint: compact, standard, or expanded presentation. |

## What Library provides

The Library Browser declares its identity and advisory geometry needs to topology.
These are not authoritative over layout. Topology may use them for placement
negotiation, but they do not let the Library Browser choose its own slot, split,
dock, or tab position.

| Provided value               | Meaning                                                                     |
| ---------------------------- | --------------------------------------------------------------------------- |
| `surfaceKind`                | Stable workspace surface kind: `library_browser`.                           |
| `preferredMinDimensions`     | Advisory minimum width/height for usable browser presentation.              |
| `preferredDefaultDimensions` | Optional advisory default size for first placement. Topology may ignore it. |

**Topology may use these values for layout negotiation, but they are not authority over workspace topology.**

If topology cannot satisfy `preferredMinDimensions`, it proceeds with the available
space and the Library Browser adapts using its internal layout presets. The
browser does not refuse to render because it received less space than preferred.

The Library Browser uses `isVisible` to pause non-critical work (prefetch,
background projection reads) when the surface is hidden. It does not stop
scan progress or preparation work on visibility changes.

The Library Browser uses `surfaceBounds` and `viewportHint` to select an
internal layout preset and to size the visible frontier for prefetch bounds.

## Internal layout presets

The Library Browser manages its own internal layout. Topology provides bounds;
the browser decides how to fill them.

| Preset                         | When used                              | Regions visible                           |
| ------------------------------ | -------------------------------------- | ----------------------------------------- |
| `navigator_only`               | Surface width below compact threshold. | Source tree only.                         |
| `navigator_contents`           | Standard surface width.                | Source tree + contents browser.           |
| `navigator_contents_inspector` | Expanded surface width or user pinned. | Source tree + contents + inspector panel. |

The compact threshold, standard threshold, and inspector pin state are
Library Browser preferences, not topology configuration. Topology does not
know which preset is active.

Region proportions (tree width, inspector width) are Library Browser state.
They are persisted independently of workspace layout state.

## Viewport hints and prefetch

When topology reports `isVisible = false`, the Library Browser:

- Pauses visible-frontier prefetch entirely.
- Does not issue new branch reads for off-screen refreshes.
- Continues to accept scan events and mark cache entries stale.
- Resumes prefetch when `isVisible` returns to `true`.

When `surfaceBounds` changes (resize, split drag), the Library Browser:

- Recalculates the visible frontier for prefetch bounds.
- Does not clear the branch cache.
- Does not re-read branches solely because the surface grew or shrank.

## What topology must not own

| Forbidden topology ownership   | Why                                                                     |
| ------------------------------ | ----------------------------------------------------------------------- |
| Selected tree node             | Selection is library state. Layout changes must not reset it.           |
| Expanded tree nodes            | Expansion is library state. Slot moves must not collapse the tree.      |
| Row profile or sort policy     | Projection policy is library state. Topology changes must not reset it. |
| Source visible states          | Source lifecycle is library state. Surface hide/show must not eject.    |
| Branch cache                   | Cache is library renderer state. Surface resize must not clear it.      |
| Contents panel scroll position | Scroll is library state. Slot moves must not reset pagination.          |

A workspace topology change (slot move, split resize, tab switch) must not
produce any of the failure modes described in the frame-stability rejection table.

## Reconnection after hide/show

When the Library Browser surface becomes visible after being hidden:

1. Restore the branch cache as-is. It may be stale.
2. Re-validate visible branches against current epochs.
3. Refresh stale visible branches following targeted invalidation rules.
4. Resume prefetch for the visible frontier.
5. Do not flash the tree to empty on restore.

The tree and contents panel must look the same after show as they did before
hide, modulo any scan progress that arrived while hidden.

## Panel containment

The Library Browser must keep row browsing inside its own surface bounds. Contents rows must be reachable through the
contents region inside the library panel and must not require app-level overflow or workspace scrolling to browse the
table.

This is an acceptance rule between workspace topology and the Library Browser surface. It does not require a specific
CSS class, grid, or table implementation.

## Non-goals

This contract does not define workspace routing, slot assignment algorithms,
multi-surface composition, or how the Library Browser surface is opened or
closed. Those are governed by workspace topology contracts.

It does not define the internal layout system within the Library Browser regions.
Internal split proportions and region visibility are Library Browser state
governed by the selection-contents contract and the frame-stability contract.
