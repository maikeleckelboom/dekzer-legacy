# Workspace Engine Spec

## Status

Accepted working foundation.

This document captures the current final implementation direction for the Dekzer workspace engine foundation as
established through architecture review.

It exists to prevent loss of design decisions across chat, repo resets, or prototype churn.

---

## Purpose

Define the canonical foundation for the new workspace engine inside the current empty Dekzer scaffold.

This spec covers:

- product and ownership goals
- repo/package shape
- internal domain boundaries
- public contract surface
- core data model decisions
- runtime behavior rules
- immediate implementation sequence
- explicit rejections and deferred work

This is the implementation foundation to build from.

---

## Product Position

Dekzer is a performance instrument, not a generic tool bench.

That means the workspace must optimize for:

- deterministic behavior
- stable muscle memory
- explicit structural change
- recoverability
- runtime-owned participation and visibility
- renderer as projection only

The workspace is a configured instrument room, not a casual drag-anywhere docking environment.

---

## External References

### Adobe

Adobe remains a behavior reference for shared-boundary resize interaction only.

Useful behavior references:

- shared-boundary resize across multiple groups
- junction-aware resize interaction feel

Rejected as room-level law:

- ambient free-form docking
- floating panels as first-class room behavior
- casual topology mutation through drop zones

Adobe is evidence for interaction feel, not architectural authority.

### Archive Workspace Layout Engine

The archive remains:

- behavior oracle
- interaction oracle
- visual feel oracle
- acceptance reference

The archive is never implementation authority.

Do not port archive ownership structure, DOM assumptions, or internal code paths into the new engine.

---

## Existing Repo Context

Current repo root is an empty Dekzer scaffold.

Current tree shape is effectively:

- `apps/desktop` exists and boots
- `packages/*` is empty and reserved
- `docs/workspace/*` contains the current law docs

This means the next work is greenfield implementation inside the scaffold, not migration patching of an already-built
workspace engine.

---

## Canonical Package Split

Use exactly two packages.

```text
packages/
  workspace/
  workspace-vue/
```

### `packages/workspace`

Headless workspace authority package.

Owns:

- authored input types
- compiled IR
- runtime topology state
- runtime layout state
- drag sessions
- negotiation
- projection
- runtime orchestration

### `packages/workspace-vue`

Thin Vue presentation adapter.

Owns:

- rendering projected frames
- pointer capture plumbing
- keyboard wiring
- focus and presentation edge behavior
- forwarding user signals into runtime

It does not own topology, layout law, participant resolution, or DOM-derived geometry authority.

---

## Internal Domain Structure

Inside `packages/workspace/src`, use internal domains rather than many public packages.

```text
src/
  contract/
  language/
  compiler/
  topology/
  layout/
  projection/
  runtime/
```

These are domains, not package boundaries.

### contract

Public types crossing package boundaries.

### language

Authored workspace DSL and validation.

### compiler

Canonical compiled workspace IR and structural lineage.

### topology

Runtime-owned workspace structure and policy axes.

### layout

Committed and preview geometry behavior, sessions, and negotiation.

### projection

Projection from authority state into renderable frame.

### runtime

Orchestration only.

---

## Dependency Law

Internal dependencies must flow like this:

```text
language -> contract
compiler -> language + contract

topology -> contract
layout -> contract
projection -> contract

runtime -> compiler + topology + layout + projection + contract
```

Disallowed:

- projection mutating topology
- layout mutating topology directly
- topology importing projection
- Vue importing internal workspace domain files
- renderer deriving layout authority from DOM geometry

---

## Core Ownership Stack

The canonical ownership stack is:

```text
AuthoredTopology
  -> CompiledLayout
  -> RuntimeTopologyState
  -> RuntimeLayoutState
  -> PreviewState
  -> RealizedGeometry
```

### AuthoredTopology

What the user configured.

### CompiledLayout

Canonical structural IR with deterministic lineage.

### RuntimeTopologyState

Owns:

- structural presence
- visibility policy
- participation eligibility
- budget policy
- slot placement
- topology generation
- topology mutation results

### RuntimeLayoutState

Owns:

- committed boundary/child geometry state
- preview geometry state
- drag sessions
- negotiated/direct solve state

### PreviewState

Transient solve output only.

Preview is never committed state.

### RealizedGeometry

Renderer projection only.

DOM is not authority.

---

## Room-Level Product Law

### Floating panels

Floating panels are not first-class room-level behavior.

### Docking

Ambient free-form docking is rejected as a room-level workspace law.

### Topology mutation

Topology mutation must be explicit.

Structural changes occur through declared workspace commands, not casual drag-over docking affordances.

### Performance interaction

Topology mutation is separated from normal performance interaction.

### Local internal docking

A bounded host-local docking model may exist later inside a specific component contract.

It does not weaken room-level law.

---

## Public API Surface

Keep the public API of `@dekzer/workspace` small.

Expected public exports:

- `compileWorkspace(...)`
- `createWorkspaceRuntime(...)`
- `WorkspaceRuntime`
- `WorkspaceFrame`
- `WorkspaceViewport`
- `WorkspaceCommand`
- `WorkspaceBoundaryRef`
- `WorkspaceJunctionRef`
- `WorkspaceSlotRef`
- `BoundaryDragMode`

The app and Vue layer must not import internal domain files directly.

---

## Public Contract Decisions

### Refs

Use structural ids for slots, boundaries, and junctions.

Refs are small wrappers around ids.

### Frame contract

The renderer receives a projected frame, not raw solver state.

The frame contains:

- slot presentations
- boundary presentations
- junction presentations

Do not reduce the public renderer contract to a `Map<SlotId, PixelRect>`.

### Naming

Use precise presentation field names.

Accepted:

- `isDragged`
- `hasKeyboardFocus`
- `participation`

Rejected:

- `isActive`

"Active" is a naming lie and must not appear in the presentation contract.

---

## Structural Identity Law

### Deterministic ids

Compiled structural ids must be deterministic and path-derived.

Do not use module-level counters.

Examples:

- `split:root`
- `split:root/1`
- `boundary:root:0`
- `boundary:root/1:0`

This guarantees:

- reproducible compile output
- stable tests
- structurally derived lineage

---

## Compiled Layer Decisions

### Do not leak authored size type directly

The compiled layer must not expose `AuthoredChildSize` directly as its canonical size type.

The compiled layer gets its own size form.

### Compiled child size

Use a compiled child size type that is independent from authored DSL and can carry normalized information.

At minimum it must provide a place for:

- size kind
- fixed px if applicable
- weight if applicable
- min px if applicable
- max px if applicable

This prevents authored DSL leakage into the canonical IR.

### Junctions

Do not emit fake junctions.

Until real structural junction membership exists, compiled junction output should remain empty.

---

## Initial Layout Law

### No default 0.5 boundary initialization

Initial layout must not default every boundary to `0.5`.

That is visibly wrong and violates authored sizing.

### Viewport-aware initialization

Initial committed layout state must be derived from compiled child sizing plus viewport.

Reason:

- fixed px sizes depend on viewport size
- weighted and bounded sizes must be resolved relative to available space

Therefore:

- compiled layer stores normalized child sizing inputs
- runtime layout initialization derives initial committed fractions or child extents using the viewport

### Initialization algorithm

For each split:

1. reserve fixed px children first
2. compute remaining available main-axis space
3. distribute remaining space to weighted/bounded children by weight
4. clamp to min/max where applicable
5. derive cumulative boundary positions from child extents

This must happen before first render.

---

## Runtime Topology Law

Runtime topology state owns these axes separately:

- structural presence
- visibility policy
- participation eligibility
- budget policy
- slot placement

These are independent axes.

Do not collapse them into one state enum.

### Budget policy

Budget policy remains a first-class runtime concern.

Accepted policies:

- retain
- release

Retain/release is separate from visibility and separate from participation eligibility.

---

## Runtime Layout Law

Runtime layout state owns:

- committed geometry state
- preview geometry state
- drag sessions
- direct mode behavior
- negotiated mode behavior

### Preview/commit split

Preview must remain separate from committed state.

Never mutate committed state during preview drag.

### Active session freezing

Boundary drag sessions must freeze the current topology generation at start.

If topology generation changes during drag, the active drag session is invalidated.

That is a hard law.

---

## Drag Mode Law

Two modes exist:

- direct
- negotiated

### Direct mode

Strict local behavior.

Only the touched local boundary relationship negotiates.

### Negotiated mode

Same-axis participants may join from the first delta.

The touched boundary remains dominant.

### Runtime-owned mode selection

Mode selection belongs to runtime, not solver internals and not renderer policy.

### Initial practical signaling mechanism

For the first slice, runtime may accept a `requestedMode` signal in `beginBoundaryDrag(...)`.

This is allowed only as a user-signal translation path.

Meaning:

- Vue may translate a modifier key into `requestedMode`
- runtime still resolves the effective mode
- runtime may ignore or override the request

This keeps ownership correct while allowing both modes to be exercised from day one.

### Explicit rejection

Do not put drag mode on the Vue component prop surface as behavioral authority.

---

## Negotiation Law

### Current scope

The intended final model is topology-native same-axis continuation.

### Immediate reality

An early same-parent-split-only implementation is acceptable only as a temporary slice, provided it is treated as debt
and not mistaken for the final law.

### Final target

Negotiated participation must eventually resolve over same-axis continuation topology, not only same-parent-split
siblings.

### Dominance

The touched boundary is dominant.

Dominance is not a hardcoded scalar coefficient.

Dominance means first claim on available directional capacity.

### Capacity-first law

Replace hardcoded dominance coefficients with directional capacity solving.

The correct long-term solver must:

- compute directional give/take capacity
- allow the touched band to consume capacity first
- spill pressure only when local capacity is saturated
- propagate only for constraint-justified reasons

### Constraint enforcement

Negotiated mode must honor:

- fixed size
- min px
- max px
- bounded ranges
- frozen eligibility
- excluded eligibility
- retain/release budget semantics when implemented

### Temporary warning

A boundary-centric negotiated solver may exist as a stepping stone, but it is not the final architecture.

The correct internal solving model trends toward child extents and capacity, not naked boundary-fraction arithmetic.

---

## Projection Law

Projection owns rendered geometry.

### Integer snapping

Projection must own integer pixel snapping.

Fractions belong to layout solving.

Exact integer pixel rects belong to projection output.

### Exact fill guarantee

Projection must guarantee:

- total realized child extents plus gutters equals the exact viewport extent
- no sub-pixel seams
- no accidental light bleed gaps or overlaps

Rounding remainders must be explicitly assigned.

### Structural sharing goal

Projection should eventually support structural sharing or pooling so unchanged presentation objects are reused across
frames.

This is a performance goal after correctness.

Correctness comes first.

### Renderer contract

The renderer consumes projected geometry only.

The DOM does not determine topology, participation, or hit-region authority.

---

## Vue Adapter Law

`workspace-vue` is a thin adapter.

It may own:

- pointer capture
- keyboard plumbing
- focus management
- rendering projected frames
- translating physical user signals into runtime requests

It must not own:

- topology
- participant resolution
- drag policy
- hit region authority
- DOM-derived structure inference

### Reactivity rule

All UI state that affects rendering must be reactive.

Do not use plain variables for drag session state if the UI depends on them.

Example requirement:

- active boundary drag session must be stored in a reactive ref in Vue
- computed styles depending on it must read the ref value

---

## Desktop App Role

`apps/desktop` is not the workspace engine.

It is:

- desktop shell
- viewport source
- content host
- place that mounts `workspace-vue`

Do not let `apps/desktop` become layout authority.

---

## Minimal First Implementation Sequence

### Phase 0

Create the two new packages:

- `@dekzer/workspace`
- `@dekzer/workspace-vue`

### Phase 1

Implement the first honest slice:

- authored contract
- validation
- compiled structural IR with deterministic ids
- topology state
- viewport-aware initial layout state
- runtime shell
- projected frame
- Vue root host rendering projected frame

### Phase 2

Direct boundary drag:

- begin/update/commit/cancel
- preview separate from committed
- topology generation freeze
- strict local direct mode

### Phase 3

Negotiated boundary drag first slice:

- runtime-resolved participant snapshot
- touched-boundary dominance
- joined boundary projection state
- no fake same-parent-split permanence

### Phase 4

Topology mutation commands:

- move
- swap
- split
- remove
- generation increments
- active drag invalidation

### Phase 5

Same-axis continuation compiler/runtime work:

- same-axis lineage graph
- cross-split same-axis participant resolution

### Phase 6

Projection hardening:

- integer snapping
- exact fill guarantees
- structural sharing/pooling

### Phase 7

Junctions and later 2D work:

- real junction membership
- projected junctions
- separate junction drag law

---

## Immediate Implementation Priorities

These are the highest-priority code decisions to lock before writing too much more:

1. deterministic structural ids
2. compiled child sizing type separate from authored DSL
3. viewport-aware initial layout derivation
4. reactive Vue drag session state
5. runtime-level mode resolution via `requestedMode` bridge
6. no fake junctions
7. precise presentation naming

---

## Tests That Must Exist Early

### Direct mode remains strict local

A boundary drag in direct mode must not mark joined participants.

### Preview separate from committed

Cancel must restore committed geometry.

### Topology generation invalidates active drag

Structural mutation must terminate stale boundary drag sessions.

### Initial authored sizing honored on first render

A weighted 1:2:1 row must not render as equal thirds before first drag.

### Runtime mode path test

Both direct and negotiated must be exercisable through runtime begin-drag signaling.

---

## Explicit Rejections

Do not do any of these:

- ambient room-level docking
- first-class floating panels at room level
- DOM-derived layout authority
- fake junction registration
- module-level ID counters
- equalized negotiated motion as universal law
- hardcoded dominance coefficient as the long-term model
- Vue prop authority over drag mode
- another disposable proof-surface prototype
- archive internal code porting

---

## Deferred But Planned

Deferred means not implemented yet, not rejected.

Deferred items:

- same-axis continuation across nested topology
- true capacity-first negotiated solve
- retain/release budget effect on negotiation pool
- integer-snapped projection implementation
- structural sharing or pooling in projection
- real junction membership and projection
- separate 2D junction-drag law
- Rust migration of the authority core

---

## Rust Strategy

Rust is not the first move.

TS-first remains correct because the current problem is ownership and law proofing, not raw throughput.

Later, if the architecture hardens, the inside of `packages/workspace` can migrate to Rust or WASM behind the same
public contract.

That future migration target would be:

- topology
- layout
- projection
- runtime orchestration

`workspace-vue` stays at the presentation edge.

---

## Final Working Position

This is the production foundation direction.

The architecture is accepted.

The implementation must now proceed inside the current Dekzer scaffold with:

- two packages
- one headless authority center
- one thin Vue adapter
- deterministic structural ids
- viewport-aware initial layout
- runtime-owned drag mode resolution
- explicit topology mutation
- projection-owned geometry
- renderer as realization only

Build this exact thing.

Do not start over again.
