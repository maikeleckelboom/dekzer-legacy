# Topology modeling law

**Governs:** slot representation, slot identity, continuity law, feature-to-slot assignment, implementation rejection
rules.
**Does not govern:** optical law, topology family catalog, or size-class substitution.
**See also:** `visual-workspace-doctrine.md`, `responsive-topology-matrix.md`.

---

## Status

This document is the canonical topology modeling law for Dekzer. It defines how topology is represented in the system,
how slots are kept stable, and how features assign to slots.

## Purpose

Topology must be modeled explicitly. Slot identity must survive breakpoints, mode changes, docking changes, and
workspace rearrangement without losing runtime state or triggering unnecessary remount.

## Naming conventions

These names are not interchangeable:

- **module rail** is the shell-stable rail region when the product keeps a durable rail across topology families
- **dominant field** is a topology slot; **dominant** is the slot class it belongs to
- **horizon lane** is the topology slot; **commit horizon surface**, **route summary surface**, or similar are assigned
  contents
- **object row** is a topology slot; **object host** is the slot class describing how it behaves

Use one name per level of ownership. Do not blur shell region, topology slot, slot class, and assigned surface.

## Core statement

A topology slot is a stable workspace attachment point. It is not a component and not a DOM node.

Slot identity is fixed. Breakpoint changes and mode changes re-solve participation, placement, containment mode,
visibility mode, and realization strategy. They do not replace the workspace with a different conditional render tree.

Features do not own layout. Features declare where they can legally live through surface assignment.

## Stable slot identity

Responsive substitution, mode changes, docking, and workspace rearrangement do not redefine slot existence as the
primary mechanism.

Slots are stable workspace identities. Runtime topology decides how they participate. Geometry decides placement.
Realization follows solved geometry.

This law exists for correctness first and performance second. It protects continuity, focus, interaction state, and
heavy surface stability.

## Responsive slot law

Responsive behavior re-solves participation and arrangement over stable slot identities. It does not unmount one
workspace truth and mount a different one.

It changes: which slots participate, how they are arranged, how they are contained, how they are realized.

It does not rebuild interaction state as a side effect of breakpoint changes.

## Continuity law for heavy surfaces

Heavy surfaces — waveforms, deck views, graph canvases, analysis canvases — must survive topology changes as the same
workspace-owned surface.

When a heavy surface grows, shrinks, moves, docks, or is promoted into a larger field:

- stable slot identity remains
- runtime state remains
- solved rect changes
- projection density adapts
- realization updates incrementally
- expensive resources stay retained where practical

This prevents state resets at breakpoints, focus loss during topology changes, pointer-session loss during responsive
substitution, and expensive mount-unmount churn.

### Waveform continuity example

A waveform does not become a different thing when its slot rect changes. Playhead, zoom, scroll, anchor, and selection
state remain stable. New solved geometry is delivered to the same surface. Detail level adapts to the new rect.
Destructive remount is avoided where possible.

## Three-layer ownership model

### Layer 1: shell regions

Shell regions are stable across the product and are not authored per mode.

Typical shell regions: titlebar, module rail or equivalent durable product navigation region, optional global status
region only when it is product-wide and mode-stable.

Shell regions are few by design. If a region is mode-authored or topology-authored, it is not shell.

### Layer 2: topology slots

Topology slots are stable workspace attachment points that participate in layout solve, responsive substitution,
continuity, and geometry negotiation.

Typical slots: dominant field, authority upper, authority lower, mid support A, mid support B, mid support C, horizon
lane, object row, contents field, inspector, graph field, operator field, verification field.

Concrete product slot examples include `browseRoots`, `contentsField`, `selectionInspector`, `performanceField`, and
`lowerBrowseSupport`.

A slot should be modeled only when it can independently affect: participation, geometry, visibility mode, responsive
substitution, continuity, or feature placement.

Do not model a slot merely because a designer can draw a box around it.

### Layer 3: surface assignments

Surface assignment answers what content currently occupies a slot.

Examples:

- dominant field → perform surface
- dominant field → route canvas surface
- contents field → library contents surface
- inspector → track inspector surface
- horizon lane → commit horizon surface
- object row → candidate objects surface

Topology says where something can live. Surface assignment says what currently lives there. These are separate ownership
layers.

Projection switches such as List, Tree, Columns, and Covers are view/projection state inside the same Contents slot.
They are not topology mutation and do not create new room-level slots.

## Minimal topology schema

Every topology slot carries these axes. Keep them distinct.

### Slot identity

A stable name used to preserve continuity across topology changes. Examples: `dominantField`, `horizonLane`,
`authorityUpper`, `inspector`.

### Structural presence

Whether the slot belongs to the active topology family at all.

Example: authority stack workbench includes `horizonLane` structurally. Focused instrument view does not.

### Participation

Whether a structurally present slot is currently part of the active layout solve. A slot may be structurally present
while not participating.

### Containment mode

How the slot is currently hosted. Values: docked, overlay, drawer, sheet, tray, parked.

### Visibility mode

How the user currently encounters the slot. Values: visible, collapsed, hidden but summonable, preview-only.

### Realization strategy

How much implementation weight is currently kept alive. Values: realized, virtualized, retained hidden, unrealized.

DOM realization is downstream. It does not own whether the slot exists.

### Surface assignment

What instrument or feature surface currently occupies the slot. A slot stays stable while its assigned surface changes
by mode.

A single `isVisible` boolean cannot represent parked, overlay, summonable, retained, and participating states correctly.
Use explicit axes.

## Slot classes

Every slot belongs to a slot class. Classes prevent topology from devolving into a bag of unrelated names.

### Dominant

Owns first-glance attention and the primary instrument of the screen. Only one dominant slot wins at a time. Must not be
visually equaled by support slots.

### Authority

Anchors durable navigation, hierarchy, or structural context. Supports orientation and collection structure. Must not
grow to compete with the dominant field.

### Support

Provides contextual or operational assistance to the dominant field. Remains subordinate in tone and emphasis. Must not
accumulate equal visual weight with the dominant slot.

### Inspector

Provides focused detail, editing, or metadata support for the currently active subject. Stable and quieter than the main
field. Does not compete with the active instrument.

### Lane

Provides thin horizon-oriented structure such as commit, route, action, or live timeline context. Is a workspace pane,
not shell chrome, when it carries mode-owned operational content. Must remain thin and linear.

### Object host

Presents contained secondary objects such as candidates, sleeves, route objects, or compare surfaces. May use card
treatment because it hosts contained objects, not main field partitions.

## Feature assignment rules

### Rule 1

A feature prefers occupying an existing legal slot before inventing a new one.

### Rule 2

Create a new slot only when the region needs independent: geometry ownership, participation behavior, responsive
substitution behavior, continuity semantics, or visibility and containment behavior.

### Rule 3

If a feature only changes what is shown in an existing pane, it is a new surface assignment, not a new topology slot.

Examples:

- inspector → stems plan surface is not a new slot
- inspector → transition detail surface is not a new slot
- horizon lane → route summary surface is not a new slot
- adding a real horizon lane where none existed may justify a new slot

### Rule 4

A screen review must answer: what is the dominant instrument, which slots are support-only, which surfaces are
persistent, which surfaces are summonable, what topology family is active. If those answers are not obvious, the
composition is not ready.

## Example: authority stack workbench modeling

Shell regions: titlebar, module rail.

Topology slots: authorityUpper, authorityLower, dominantField, midSupportA, midSupportB, midSupportC, horizonLane,
objectRow.

Surface assignments:

- authorityUpper → hierarchy surface
- authorityLower → collection surface
- dominantField → waveform workbench surface
- midSupportA → lower browse support surface
- midSupportB → selection inspector surface
- midSupportC → preparation surface
- horizonLane → commit horizon surface
- objectRow → candidate objects surface

This keeps topology readable even while the actual product features evolve.

## Rejection rules

Reject any implementation where:

- slot existence is inferred from DOM presence
- breakpoints swap entire workspace truths through conditional app trees
- visibility is represented as a single overloaded boolean
- docking and overlay behavior are owned by components instead of runtime topology
- heavy surfaces reset identity during geometry change
- state resets occur at breakpoints
- focus or pointer-session is lost during topology changes
- a new slot is invented for a change that is only a new surface assignment

## Review discipline

Every new screen or topology proposal answers these three questions immediately:

1. what is the dominant instrument
2. which slots are support only
3. which surfaces are persistent versus summonable

That is the minimum discipline required to preserve clarity while integrating features.

## See also

- `visual-workspace-doctrine.md` — visual product law governing surface appearance
- `topology-families.md` — legal topology families and their slot compositions
- `responsive-topology-matrix.md` — which slots participate at each size class
