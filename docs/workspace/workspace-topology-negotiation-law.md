# Workspace Topology Negotiation — Law

> This document states architectural law only. It does not describe a solver algorithm or propose specific type shapes. Those belong in a separate solver-sketch document. If you are reading this to understand what the system must do, you are in the right place. If you are reading this to understand one way to implement it, read this first, then read the sketch.

---

## Opening law

Dragging a boundary does not resize two panes and only later spill outward. In negotiated mode, it runs a same-axis topology negotiation from the first delta. The touched boundary is dominant, but not exclusive. Direct mode is the only strict local mode.

---

## What this is not

Every wrong implementation started from one of these. State them first.

**Not local-pair-only with late spill.**
A boundary drag does not exclusively resize the two panels it sits between and then, only after one hits a limit, begin affecting others. In negotiated mode, the participant set is resolved at session creation and is available from the first delta. Availability does not mean visible motion. Actual motion for any non-touched participant still requires a constraint-justified reason. The difference from the wrong model is that no participant must wait for full exhaustion of its neighbor before it can move — not that all participants visibly move immediately.

**Not equalized motion.**
Additional participants do not all move the same amount, or at the same time, or for the same reason. Motion is weighted, attenuated by locality, and bounded per participant.

**Not arbitrary group motion.**
Negotiated mode does not mean every pane in a row moves whenever any boundary moves. Every additional moving pane must have a local, same-axis, constraint-justified reason to move. See Constraint-Justified Propagation below.

**Not DOM-driven.**
The DOM does not determine layout. The DOM realizes layout. Nothing upstream of the renderer reads DOM geometry at any point during a drag. Nothing downstream owns layout state.

**Not one resize law.**
Two modes exist. They are not the same mode with a parameter. Direct mode is strict local: only the immediate neighbors of the touched boundary participate. Negotiated mode allows same-axis continuation. The runtime selects the mode at session creation. The solver does not decide the mode.

---

## The two modes

### Direct mode

Only the immediate neighbors of the touched boundary participate. No other participant moves, regardless of topology. This is the strict local mode. It is not a degenerate case of negotiated mode. It is a separate law.

### Negotiated mode

Eligible same-axis participants may begin participating from the first delta. The touched boundary remains dominant. Participation is weighted and bounded, not binary and not equalized. Actual motion for any non-touched participant requires a constraint-justified reason. See Constraint-Justified Propagation.

---

## Constraint-justified propagation

This is the most commonly misunderstood part of the system. State it precisely.

Negotiated resize is not ambient multi-pane motion. A non-touched participant moves only when there is a same-axis, structural, constraint-driven reason for pressure to continue through it.

In practice: a participant farther from the touched boundary begins moving when the closer participant on the same pressure path has reached or is currently bounded by its directional give/take limit, so pressure continues across the next same-axis boundary.

Every visible propagation step must be explainable boundary-by-boundary as a contiguous same-axis chain of local constraint transfers. If a pane moves and there is no such explanation, the behavior is wrong.

**Eligibility is broader than motion.** A participant may be eligible for negotiation without visibly moving at a given delta. Actual movement is determined by the current pressure path and current directional capacities, not by membership in the participant set alone.

Propagation is never ambient. It is always constraint-justified.

---

## Same-axis scope

Participation is resolved over the same-axis continuation topology rooted at the touched boundary.

Orthogonal branches do not join a negotiated resize. An orthogonal crossing is not a far hop in the same graph. It is outside the graph. If a future mode allows orthogonal participation, that is a separate, explicitly defined mode with its own law. It does not extend the current negotiated mode.

---

## Participation factors

Each participant's contribution is determined by independent factors:

**Eligibility** — binary. Resolved by the runtime topology layer. Either a participant is in the negotiated set or it is not.

**Locality attenuation** — a decreasing function of topological distance along the same-axis continuation. Distance 0 (immediate neighbors of the touched boundary) has full attenuation. Attenuation decreases strictly with distance. The exact decay function is an implementation choice, not architectural law.

**Participation preference** — an authored per-slot preference that sets relative strength among participants at the same locality. Analogous in spirit to a flex-grow coefficient but scoped to topology negotiation specifically. Do not import CSS flex semantics.

**Directional capacity** — how much a participant can currently give or take in the required direction. This is **not** a scalar multiplier on the weight. It is a directional bound applied separately during solving. It must be recomputed against the current preview state on each solver iteration, not frozen at session creation.

Collapsing directional capacity into the weight formula is a common mistake. The weight determines how much of the available delta a participant bids to absorb. Capacity determines how much it can absorb given where it currently is. These are separate concerns.

---

## Dominance without exclusivity

The touched boundary is dominant. This means it absorbs more than its proportional share of delta. It does not mean it absorbs all of it.

Dominance is a structural guarantee, not an emergent consequence of higher weight. The solver must enforce it explicitly. How it enforces dominance is an implementation detail. That the touched pair receives a privileged, non-negotiable share of each delta step is law.

**Dominance must be observable in the visible interaction, not only in internal accounting.** A solver where Band 0 is theoretically privileged but the felt result looks like equalized group motion has not satisfied this law. The test is behavioral: a user watching a resize gesture must see the touched boundary move more than any other boundary. If that is not true, the dominance model is wrong regardless of what the math says.

In direct mode, dominance is total. Only Band 0 participates.  
In negotiated mode, dominance is partial. Band 0 receives a guaranteed dominant portion. The remainder distributes across all eligible participants by weighted economics.

---

## Capacity is directional and recomputed

A participant's directional capacity — how much it can give or take in the required direction — is not a static session constant.

**Freeze at session creation:** structural participant membership, mode, locality model, participation preferences.

**Recompute each solver iteration:** directional capacity, derived from the current preview baseline, not the session baseline.

If you freeze capacity at session creation, clamping behavior becomes incorrect as the preview state evolves during the gesture. The authored min/max constraints are stable and can be frozen. The instantaneous available give and take cannot.

---

## Topology generation changes during active drag

A drag session freezes its participant snapshot, dominance model, and mode at session creation.

If a topology-generation change occurs during an active drag — a slot becomes absent, eligibility changes, DnD modifies structural membership, a slot hides — the session is cancelled or terminated against its original topology generation. It is not silently rebound to the new topology mid-drag.

Rebinding mid-drag is not a convenience feature. It is a correctness hazard. The participant snapshot was frozen against a specific generation. Applying it to a different topology produces undefined negotiation behavior.

The response to a mid-drag topology change is session termination, not silent adaptation.

---

## Pane state — five independent axes

A pane is governed by independent runtime/topology axes: structural presence, resize-negotiation participation, realization/visibility, budget retention/release, and placement in topology. Fixed-size panes, hidden panes, and drag-reorganized panes are not special DOM cases. They are first-class topology/runtime states.

Never represent pane state as a single enum. There are five independent axes.

**1. Structural presence** — is this slot part of the current topology at all?  
A structurally absent slot does not exist in the model. A structurally present slot exists in the model regardless of whether it is currently visible, realized, or actively participating.

**2. Negotiation participation** — can it currently take part in resize negotiation?  
A slot can be negotiable, frozen/pinned, or excluded. A fixed-size pane is not special CSS. It is a topology participant whose size negotiation is disabled or constrained to a fixed band. Its fixed size is enforced by the runtime topology layer, not by DOM attributes.

**3. Realization policy** — what does the renderer currently produce for this slot?  
Whether it is fully rendered, collapsed to zero, shown as a placeholder, or allocated but not mounted. A hidden slot is not necessarily absent from topology. It may remain structurally present while not currently realized.

**4. Budget policy** — when not realized, does it retain its size allocation?  
A slot hidden with retain-budget keeps its fraction in the layout model. Other participants do not absorb it. A slot hidden with release-budget gives its fraction back to the negotiation pool. These are different product behaviors with different consequences for the resize engine.

**5. Placement** — where in the topology does this slot currently live?  
Placement is a topology fact, not a DOM fact. Dragging a pane to a new location produces a topology mutation that assigns it a new structural position. The renderer reprojects from the mutated topology. DOM adjacency is never the authority for placement.

### Valid combinations

These five axes are independent. A pane may simultaneously be:

- `present` + `frozen` + `realized` — present, fixed size, visible
- `present` + `negotiable` + `collapsed` + `release` — participates when visible, releases budget when hidden
- `present` + `frozen` + `placeholder` + `retain` — reserves space as a placeholder without realizing content
- `absent` — not in the model at all
- `present` + `negotiable` + `realized` + placed at a new topology position after DnD

No single label covers these combinations. Collapsing any two axes into one enum forces exceptions and special cases the first time a combination arises that the label cannot express.

### Consequence for the resize engine

The resize engine must read pane state from the runtime topology layer, not from DOM conditions or inferred visual state. Specifically:

- A frozen/pinned pane must not negotiate size, regardless of what its DOM element reports
- A hidden pane with release-budget must not hold space in the negotiation pool
- A hidden pane with retain-budget may still affect size allocation of its neighbors
- A pane temporarily excluded during a DnD gesture must not participate in resize negotiation for the duration of that gesture
- A pane reinserted at a new topology location gets a new participation context derived from its new structural position

The resize engine, visibility system, and DnD system are connected through runtime/topology authority. Not through the DOM.

The specific value sets for realization policy and budget policy are **not yet frozen product law**. The separation principle is law. The exact values are working vocabulary.

---

## DnD reorganization

DnD does not rearrange DOM siblings. It mutates topology and then reprojects realization.

When a pane is dragged to a new location:
1. The topology mutation is computed — new slot placement, new boundary positions, new structural relationships
2. The topology generation increments
3. Any active drag session (resize) is terminated against the old generation
4. The new topology is projected to a new renderer frame
5. The renderer realizes the new frame

The DOM is never consulted to determine where a pane belongs. The DOM is never mutated directly to achieve a new arrangement. Topology is the authority. The DOM follows.

DnD drop targets, insertion zones, and placement previews are projected affordances, not DOM-derived geometries. They are part of the renderer frame, produced by the runtime, consumed by the renderer.

---

## Boundary identity vs boundary projection

A boundary has two separate representations. They must not be merged.

**Boundary identity** is structural. It is assigned at compile or runtime-init time and does not change as geometry changes. A drag session attaches to a boundary's identity. Identity does not carry pixel positions.

**Boundary projection** is geometric. It is recomputed on every projection pass. It carries pixel positions, extents, junction intersections, and hit-region geometry. The renderer consumes projected boundaries. Nothing upstream uses projected geometry as identity.

The visual line is not the boundary. The visual line is a realization artifact. The boundary exists independently of where it is currently drawn.

---

## Interaction affordances are projected, not discovered

The renderer does not derive hit regions from visual positions. It does not reconstruct junction affordances from DOM geometry. It does not infer which boundaries are interactive from layout structure.

All interaction affordances — boundary hit regions, junction handles, DnD targets, reveal surfaces, placeholder regions — are projected by the runtime and handed to the renderer as part of the renderer frame. The renderer acts on them. It does not produce them.

This applies to current affordances and to future ones. If a new interaction surface is added to the product, it is projected by the runtime, not discovered by the renderer.

---

## Ownership boundaries

| Concern | Owner |
|---|---|
| Slot structural presence | Runtime topology layer |
| Negotiation eligibility | Runtime topology layer |
| Realization policy | Runtime topology layer |
| Budget policy | Runtime topology layer |
| Slot placement in topology | Runtime topology layer |
| DnD topology mutation | Runtime topology layer |
| Committed live sizes | Runtime layout layer |
| Committed boundary baselines | Runtime layout layer |
| Transient preview sizes during drag | Drag session / solver |
| Pixel rects and projected geometry | Projection pass |
| DnD drop targets and insertion zones | Projection pass |
| Rendering and interaction affordances | Renderer, from projected frame only |
| DOM structure | Renderer, from projected frame only |
| DOM adjacency / sibling order | Not an authority. Ever. |

The runtime topology layer and runtime layout layer are separate owners. Topology eligibility is not a layout concern. Layout sizes are not a topology concern. Merging them into a single live-state object is how topology authority silently dissolves into layout authority.

---

## What the renderer receives

The renderer receives a complete projected frame containing:

- Slot presentations: pixel rect and realization policy per slot
- Boundary presentations: identity reference, hit region, visual region, gutter region, active state, participation state
- Junction presentations: identity, hit region, visual region, boundary membership

The renderer does not receive the topology. It does not receive the layout state. It does not receive the drag session. It receives a projected frame and acts on it.

`Map<SlotId, PixelRect>` is not the renderer contract. That is a prototype convenience. The real contract is the full projected frame described above.

---

## Common false positives

The following do **not** prove the law has been implemented correctly:

- Changing the default drag mode from direct to negotiated
- Making more panes move during a drag gesture
- Lowering pane minima so propagation starts earlier
- Adding visual highlight to neighboring panes or boundaries
- Rewriting the solver to match one candidate pseudocode

The law is satisfied only when additional motion is same-axis, topology-owned, constraint-justified, and visibly subordinate to the touched boundary's dominant role. If any of the above changes were applied but those four conditions are not verifiable, the implementation is not correct — it is more active.

---

## Checklist

Use this when reviewing an implementation or a proposed change.

- [ ] Does a boundary drag in negotiated mode ever move a non-touched participant without a constraint-justified same-axis reason?
- [ ] Does any non-touched participant move in direct mode?
- [ ] Does the solver read DOM geometry at any point during a drag?
- [ ] Is directional capacity frozen at session creation rather than recomputed against current preview state?
- [ ] Is pane state represented as a single enum rather than independent axes?
- [ ] Does boundary identity carry pixel positions?
- [ ] Does the renderer derive hit regions or junction affordances from its own DOM or geometry?
- [ ] Does a mid-drag topology change silently rebound the active session to the new topology?
- [ ] Does the renderer receive anything other than the projected frame?
- [ ] Does a fixed-size pane enforce its size via DOM attributes or CSS rather than through negotiation eligibility?
- [ ] Does a hidden pane's budget behavior depend on DOM mount state rather than explicit budget policy?
- [ ] Does DnD mutate DOM sibling order directly rather than producing a topology mutation first?
- [ ] Are DnD drop targets or insertion zones derived from DOM geometry rather than projected by the runtime?
- [ ] Does the resize engine read pane state from DOM conditions rather than from the runtime topology layer?

All answers must be **no**. A yes is a law violation.
