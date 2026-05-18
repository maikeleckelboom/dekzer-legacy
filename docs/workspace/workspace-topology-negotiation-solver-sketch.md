# Workspace Topology Negotiation — Solver Sketch

> This document describes one viable implementation family for the negotiation law defined in
`workspace-topology-negotiation-law.md`. Nothing here is architectural canon. The law doc states what must be true. This
> doc shows one way to make it true. Specific formulas, type shapes, and pseudocode may change without violating the law.
>
> Read the law doc first. Do not treat anything in this doc as a constraint on future implementations unless it has been
> explicitly promoted to the law doc.

---

## Participant set construction

The solver operates on a resolved participant set. The participant set is constructed at session creation and frozen for
the duration of the gesture.

The set is derived by walking the same-axis continuation topology rooted at the touched boundary. Starting from the
touched boundary's parent split, collect all siblings sharing the same axis. Assign each a band index based on
topological distance from the touched boundary. Do not cross orthogonal splits.

For each participant, record:

- Band index
- Authored participation preference (the non-CSS, non-flex-grow coefficient)
- Authored min and max fraction constraints
- Which side of the touched boundary it sits on (left or right)

Directional capacity (`canGive`, `canTake`) is **not** stored in the frozen snapshot. It is derived from current preview
sizes at each iteration.

---

## Priority band mechanics

Participants are grouped into bands by distance from the touched boundary.

**Band 0** — immediate neighbors of the touched boundary (left neighbor and right neighbor).
**Band 1** — next same-axis participants outward from Band 0.
**Band N** — same-axis participants at distance N.

In **direct mode**, only Band 0 participates. No other bands are included in the solve.

In **negotiated mode**, all bands are included, with locality attenuation applied by band.

---

## The dominance split

Each solve step splits the delta into two allocations:

```
touchedShare     = α × delta
negotiationShare = (1 − α) × delta
```

Where `α` is the dominance coefficient — an engine-level parameter or authored per split. A value near 1.0 makes Band 0
nearly exclusive. A value near 0.5 produces more balanced co-participation.

**Band 0 absorbs `touchedShare` first**, before `negotiationShare` is distributed. Both allocations run in the same
solver step — this is not sequential exhaustion. But Band 0 gets a privileged, non-negotiable portion that is not
subject to redistribution by the negotiation pool.

`negotiationShare` then distributes across all eligible bands, including Band 0, by weighted economics.

---

## Effective weight

```
effectiveWeight(p) = localityAttenuation(p.band) × p.participationPreference
```

`localityAttenuation` is a monotonically decreasing function over band index. Band 0 has attenuation 1.0. The specific
decay shape (linear, exponential, stepped) is a tuning decision. The constraint is strict monotonic decrease.

Eligibility is binary and gates participation entirely — an ineligible participant is not included in the pool and
receives no share.

---

## Directional capacity

At each solver iteration, for the current preview state:

```
canGive(p) = currentPreviewSize(p) - p.minFrac
canTake(p) = p.maxFrac - currentPreviewSize(p)
```

A participant can absorb a positive delta share only if `canTake(p) > ε`.
A participant can absorb a negative delta share only if `canGive(p) > ε`.

These values change as the preview state evolves. They are not cached.

---

## Solver pseudocode

> **This pseudocode is a candidate shape, not a proof that the desired feel has been achieved.** Matching this code does
> not mean the law is satisfied. The law is satisfied when the visible interaction is correct: touched boundary dominant,
> additional motion constraint-justified, no ambient drift. Treat this as a starting point for implementation, not a
> specification to satisfy by compliance.

```ts
function solve(
  session: FrozenDragSession,
  preview: PreviewSolveResult,
  deltaPx: number,
  totalPx: number
): PreviewSolveResult {
  const deltaFrac = deltaPx / totalPx
  const sizes = clone(preview.negotiatedSizes)

  const touchedShare = session.dominanceCoeff * deltaFrac
  const negotiatedShare = (1 - session.dominanceCoeff) * deltaFrac

  // Band 0 absorbs touched share
  distribute(sizes, session.band0Left, +touchedShare, sizes)
  distribute(sizes, session.band0Right, -touchedShare, sizes)

  // All bands share the negotiated remainder
  distribute(sizes, session.allLeft, +negotiatedShare, sizes)
  distribute(sizes, session.allRight, -negotiatedShare, sizes)

  return {negotiatedSizes: sizes}
}

function distribute(
  sizes: SizeMap,
  participants: ResolvedParticipant[],
  delta: number,
  currentSizes: SizeMap    // recomputed capacity source
): void {
  let remaining = delta

  while (Math.abs(remaining) > 1e-9) {
    const eligible = participants.filter(p =>
      remaining > 0
        ? canTake(p, currentSizes) > 1e-9
        : canGive(p, currentSizes) > 1e-9
    )
    if (!eligible.length) break

    const totalWeight = eligible.reduce((s, p) => s + p.effectiveWeight, 0)
    let absorbed = 0

    for (const p of eligible) {
      const share = remaining * (p.effectiveWeight / totalWeight)
      const before = currentSize(sizes, p)
      const next = clamp(before + share, p.minFrac, p.maxFrac)
      setSize(sizes, p, next)
      absorbed += next - before
    }

    if (Math.abs(absorbed) < 1e-12) break   // fully constrained, no progress
    remaining -= absorbed
  }
}
```

The solver does not read `RuntimeTopologyState` or `RuntimeLayoutState` directly. It receives a frozen participant
snapshot and a current preview state. It returns a new preview state. That is its entire contract.

---

## Candidate type shapes

These are working sketches. They reflect the current understanding of what the solver needs. They may change.

```ts
// Stable identity — assigned at compile/runtime-init
type BoundaryRef = {
  id: BoundaryId
  axis: Axis
}

// Geometric projection — recomputed each projection pass
type ProjectedBoundary = {
  ref: BoundaryRef
  hitRegion: PixelRect
  visualRegion: PixelRect
  gutterRegion: PixelRect
  junctions: JunctionId[]
  isActive: boolean
  participating: boolean
}

// Renderer frame — complete projected surface
type RendererFrame = {
  slots: Map<SlotId, SlotPresentation>
  boundaries: Map<BoundaryId, BoundaryPresentation>
  junctions: Map<JunctionId, JunctionPresentation>
}

// Frozen at session creation
type FrozenDragSession = {
  boundary: BoundaryRef
  mode: 'direct' | 'negotiated'
  dominanceCoeff: number
  band0Left: ResolvedParticipant[]
  band0Right: ResolvedParticipant[]
  allLeft: ResolvedParticipant[]   // all bands, ordered outward
  allRight: ResolvedParticipant[]
}

// One participant in the solve pool
type ResolvedParticipant = {
  splitId: SplitId
  index: number           // position in parent's sizes array
  band: number
  effectiveWeight: number           // attenuation × participationPreference
  minFrac: number           // authored constraint, stable
  maxFrac: number           // authored constraint, stable
  // canGive / canTake are NOT stored here — derived from current preview each iteration
}
```

---

## Layer stack (implementation view)

```
AuthoredTopology
  └─ compile() → CompiledLayout

CompiledLayout                        immutable during session
  ├─ node graph
  ├─ authored sizes (baseline fractions)
  └─ authored constraints

RuntimeTopologyState                  topology-owned
  ├─ structural presence per slot
  ├─ negotiation eligibility per slot
  ├─ realization policy per slot
  └─ budget policy per slot

RuntimeLayoutState                    layout-owned
  ├─ committed sizes (fractions per split)
  └─ committed boundary baselines

FrozenDragSession                     created at pointer-down, immutable
  └─ (see type above)

PreviewSolveResult                    transient per pointer-move
  └─ negotiated sizes: Map<SplitId, number[]>

RendererFrame                         produced by projection pass
  └─ slots, boundaries, junctions
```

---

## What is not decided here

- Exact locality attenuation decay function (linear vs exponential vs stepped)
- Exact dominance coefficient value or whether it is globally authored or per-split
- Exact value sets for realization policy and budget policy enums
- Whether `negotiationShare` uses the same band participants as the touched-share pool or a separately filtered set
- Junction drag semantics (four-way handle behavior is not yet fully specified)

These will be decided when the law requires it. Until then, they are open.
