# Prepared Room — Canonical Foundations

**Status:** Foundational canon. Domain model and substrate constraints are locked. UX shape and implementation detail of
future layers are intentionally deferred.
**Purpose:** Establish the domain model firmly enough that current library substrate work is built compatibly. Not a
full product spec. Not a UI specification.

---

## The Problem This Names

All current DJ software digitizes the objects of DJing — records, crates, playlists, queues. None of them models the
performance situation: the staged environment, the proximity layers, the temporary surfaces, the uncommitted branches.

Dekzer's opportunity is to make the DJ's preparation visible, spatial, and performable. That requires a domain model
that current software has never needed. The library substrate must be built to accommodate it from day one, even though
most of the surface is future work.

---

## The Five-Layer Stack

These are canonical terms. Do not rename them informally. Do not collapse layers.

```
Cold Archive
    → Nearby Reserve
        → Prepared Room
            → Hot Table
                → Live Path  /  Shadow Paths
```

**Cold Archive** — everything the DJ owns. The complete indexed library across all registered sources. This is what the
current library substrate is building. It can be large, messy, archival, and inconsistent. No assumptions about
organization quality.

**Nearby Reserve** — music that is not staged in the room but close enough to pull during the set. Not a permanent
collection. A context-scoped view into the Cold Archive, defined per Prepared Room session. The reserve is reachable
without losing room context.

**Prepared Room** — a saved, session-specific performance environment. Not the whole archive. The intentional subset and
spatial arrangement the DJ prepares before a set. Has identity, persists between sessions, and can be cloned, archived,
and reused.

**Hot Table** — the temporary surface for items in hand. Volatile. Session-scoped only. Tracks pulled from crates to
compare, preview, park, or reject. Does not persist as a room artifact. Resets when the session ends.

**Live Path / Shadow Paths** — the committed and near-committed performance direction, and the uncommitted alternative
branches. The Live Path is the actual sequence of the set as it unfolds. Shadow Paths are viable alternatives the DJ
maintains without committing to. Either can be promoted, discarded, or previewed.

---

## Domain Object Model

### Cold Archive

Not a new domain object. The Cold Archive is the existing library collection — all indexed tracks and files across all
registered sources. It is the backing store that all upper layers reference.

Substrate responsibility: the Cold Archive must produce stable, canonical, source-path-independent identifiers for every
indexed item. Upper layers (CrateZones, Hot Table, Live Path) hold references to Cold Archive items. Those references
must survive source remounts, file moves, and source unavailability.

### Nearby Reserve

A scoped, context-local view into the Cold Archive. It is associated with a specific Prepared Room and exists only in
that context. It is not a global or persistent collection.

A Nearby Reserve is defined by membership criteria that may be: manually curated (user-added items), computed (BPM
neighborhood, key compatibility, style cluster, previously used in similar sets), or both. It does not copy items — it
references Cold Archive items by stable identifier.

Properties:

- `associatedRoomId` — the Prepared Room this reserve serves
- `membershipCriteria` — curated refs, computed rules, or hybrid
- `members: ColdArchiveRef[]` — resolved reference list at read time

The Nearby Reserve is browsable. It must respect content policy at its intake boundary (what is eligible for the
reserve) but not re-filter already-placed manual entries. Browse of the reserve produces a result scoped to
`nearbyReserve` context, not `coldArchive` context.

### Prepared Room

The core domain object. A Prepared Room is a saved, named performance environment. It is not a playlist. It is not a
session state blob. It is a first-class persistent object with its own identity.

Properties:

- `roomId: RoomId` — stable, opaque, unique
- `name: string` — user-assigned (e.g., "Thunderdome Livestream Room")
- `zones: CrateZone[]` — ordered list of crate zones
- `nearbyReserve: NearbyReserveSpec`
- `livePath: LivePath`
- `shadowPaths: ShadowPath[]`
- `readinessState: RoomReadinessState`
- `spatialLayout: RoomLayoutMetadata`
- `provenance: RoomProvenance` — created when, from what context, associated with which event

A Prepared Room does not own its track items. It holds references. If a referenced item becomes unavailable, the room
reflects that unavailability without losing the reference.

### CrateZone

A working region of possible moves within a Prepared Room. Not a playlist.

A playlist says: here is an ordered sequence to play through. A CrateZone says: here is a working region of tracks that
belong near each other for tonight. The DJ may use some, skip others, branch, return, or improvise.

Properties:

- `zoneId: ZoneId`
- `roomId: RoomId` — parent room
- `name: string` (e.g., "Industrial Storm")
- `intentionLabel?: string` (e.g., "violent closers", "crowd reset")
- `bpmRange?: {min: number, max: number}`
- `members: ZoneMember[]`

ZoneMember:

- `ref: ColdArchiveRef | SleeveRef` — reference to a track or sleeve
- `position: ZonePosition` — spatial position within the zone (front/back, index, or spatial coordinate)
- `readiness?: ZoneReadiness` — ready-now / maybe-later / reviewed / pass

CrateZones reference Cold Archive items. They do not copy or re-index them. Content policy does not re-filter CrateZone
members at read time — the DJ placed them there deliberately. Policy filtering happens only at the Cold Archive and
Nearby Reserve intake boundary.

### Hot Table

The temporary surface for items in active consideration. Volatile. Session-local.

The Hot Table is not persisted as a permanent room artifact. It may carry a crash-recovery snapshot (so the session can
be restored after an unexpected close), but semantically it resets between sessions.

Properties (runtime, not persisted):

- `slots: HotTableSlot[]` — fixed maximum slot count (implementation defines)
- `associatedRoomId?: RoomId` — the room context it is operating within, if any

HotTableSlot:

- `slotId: SlotId` (A, B, C, D, E or equivalent)
- `ref?: ColdArchiveRef | SleeveRef` — item currently in this slot; null if empty
- `slotState: HotTableSlotState` — pulled / previewing / parked / maybe-next / rejected-for-now

The Hot Table is a product layer, not a library layer. The library substrate must not model the Hot Table. The
substrate's responsibility is to provide fast random-access reads for the items the Hot Table holds references to.

### Live Path

The committed or near-committed performance direction. Persisted. Grows during the set.

The Live Path is a directed graph, not a list. Nodes are playable positions; edges are transition objects. A node may be
resolved (a specific Cold Archive item is committed) or unresolved (a sleeve or candidate pool from which the DJ will
choose at performance time).

Properties:

- `pathId: PathId`
- `roomId: RoomId`
- `nodes: LivePathNode[]`
- `edges: TransitionObject[]`
- `cursor: LivePathNodeId` — which node is currently live / just played
- `committedFrontier: LivePathNodeId` — how far ahead is actually decided

LivePathNode:

- `nodeId: LivePathNodeId`
- `kind: 'resolved' | 'unresolved'`
- `ref?: ColdArchiveRef` — present if kind is resolved
- `candidatePool?: SleeveRef | ZoneRef` — present if kind is unresolved (DJ picks at play time)

A Live Path node being unresolved is a first-class valid state. It means "I know I want something from this zone or
sleeve here, but I will choose the specific track in the moment." The product must present this state honestly, not
treat it as an error or a gap.

The Live Path is not the same as history. History records what happened. The Live Path is the planned and executing
performance direction, including future nodes that haven't played yet.

### Shadow Path

An uncommitted alternative branch off the Live Path. Not promoted to the main path.

Properties:

- `pathId: ShadowPathId`
- `attachmentPoint: LivePathNodeId` — which node in the Live Path this branches from
- `nodes: LivePathNode[]`
- `edges: TransitionObject[]`
- `previewState: ShadowPathPreviewState` — not-previewed / previewed-in-headphones

Shadow Paths can be promoted to the Live Path (replacing the main sequence from the attachment point forward) or
discarded. They must not automatically affect the content pane or room state — a Shadow Path is a planning artifact, not
an active navigation state.

### Transition Object

A first-class entity representing the connection between two nodes in the Live Path or a Shadow Path. Not decoration.

Properties:

- `transitionId: TransitionId`
- `from: LivePathNodeId`
- `to: LivePathNodeId`
- `entryPhrase?: BarCount`
- `exitPhrase?: BarCount`
- `energyDelta?: number`
- `bpmStrategy?: BpmStrategy`
- `eqPlan?: EQPlan`
- `filterPlan?: FilterPlan`
- `fxPlan?: FXPlan`
- `stemPlan?: StemPlan`
- `keyMove?: { from: CamelotKey, to: CamelotKey, compatibility: HarmonicCompatibility }`
- `confidence?: ConfidenceLevel`
- `notes?: string`

An empty TransitionObject — one with no fields filled in — is valid. An empty transition object is better than no
transition object, because it reserves the slot and can later be auto-populated from analysis or user input. The product
must not require a non-empty TransitionObject for a Live Path to be valid.

### Sleeve

A navigation and grouping object. Not the durable playable unit. Not an album in the traditional library sense.

A Sleeve is a visual memory anchor that can represent: a release, a compilation, a disc, a CUE-split parent, a remaster
set, a user-authored stack, a crate slice, or a release series entry. It contains one or more candidate playable items (
Cold Archive references) plus its own visual identity (artwork).

Behaviors:

- Inspect: see the sleeve identity, metadata, and contained item summary
- Open/explode: reveal contained playable items as selectable candidates
- Collapse: return to sleeve identity

A Sleeve can appear in: CrateZone members, Hot Table slots, Nearby Reserve, Live Path nodes (as an unresolved choice).
Wherever a ColdArchiveRef is valid, a SleeveRef must also be valid — the system must not assume every position holds a
single resolved track.

Properties:

- `sleeveId: SleeveId`
- `visualIdentity: SleeveVisualIdentity` — artwork, label color, visual marker
- `members: ColdArchiveRef[]` — contained playable items
- `metadata: SleeveMetadata` — release info, year, label, edition, notes

The library substrate must be designed to group tracks into sleeve-like aggregations (by release, by CUE parent, by
user-authored grouping). A flat-item-only data model cannot support Sleeve without a breaking schema change later.

---

## Product Laws

These invariants must hold across all implementations, layers, and future feature work.

**The archive is huge. The room is prepared. The table is hot. The path is live.**

1. A Prepared Room is not the archive. It is an intentional, bounded performance environment.
2. A Nearby Reserve is reachable but not staged. It never replaces the Cold Archive; it narrows it.
3. A Hot Table is volatile. It does not persist as a room artifact.
4. A Live Path is not a Shadow Path. Only one Live Path is the committed direction.
5. A Live Path is not history. History records what happened. The Live Path plans what will happen.
6. A Sleeve is not the playable item. A sleeve is a navigation and grouping object.
7. Search must not erase spatial context. Search results arrive as external pulls into the current context, not as a
   global mode switch.
8. Prepared crates reduce search; they do not forbid it. The room surface is always porous.
9. Content policy applies at intake boundaries (Cold Archive reads, Nearby Reserve reads). It does not re-filter inside
   a Prepared Room scope.
10. An item may appear in multiple layers simultaneously. Its presence in the Hot Table does not remove it from a
    CrateZone. Its promotion to the Live Path does not remove it from the Nearby Reserve.
11. An unresolved Live Path node is valid. The product must never treat an unresolved node as an error or a missing-data
    condition.
12. An empty Transition Object is valid. The presence of a TransitionObject does not require any field to be filled.
13. Track references in upper layers (CrateZone, Hot Table, Live Path) survive source unavailability. The room reflects
    unavailability; it does not drop the reference.

---

## Substrate Constraints

These are the specific obligations the current library substrate implementation must satisfy. They must not be violated
or deferred. Each one represents a design decision that, if made wrongly now, requires a breaking change to undo later.

### Constraint 1 — Canonical, source-path-independent track identity

Every indexed item in the Cold Archive must have a stable, durable identifier that does not depend on its source path,
filename, or mount point.

CrateZones, Hot Table slots, Live Path nodes, and Nearby Reserve membership all hold references to Cold Archive items.
If those references are source-path-dependent, every reference breaks when a source is remounted, a file is moved, or a
drive is renamed. Recovery from that state requires either re-linking (expensive, error-prone) or loss of prepared work.

The identifier must be stable under: source remount, file rename, folder reorganization within the same source, and
temporary source unavailability. It must be computed from content-level identity (file hash, or combination of reliable
metadata fields) rather than path.

### Constraint 2 — Browse scope must be context-instantiable, not a global singleton

The Prepared Room requires multiple simultaneous independent browse contexts: one for the Cold Archive (accessed from
Nearby Reserve browsing or global search), one for the active CrateZone being inspected, and potentially one per visible
zone panel. These contexts must be able to coexist without overwriting each other's state.

The current browse architecture must not be built as a global singleton browse state. Each browse context must be an
instantiable object with its own scope, selection state, scroll position, and content pane content. Context switching
must preserve the state of non-active contexts.

This is the substrate requirement behind the "search must not erase spatial context" product law.

### Constraint 3 — Content policy is applied at scope boundaries, not universally

`hasPolicyOmittedRows` and policy filtering (`audioBrowse` for **Audio**, `playableMediaBrowse` for **Media**) apply at
Cold Archive and Nearby Reserve browse boundaries. They do not apply when reading the contents of a CrateZone or a
Prepared Room's internal structure.

The architecture must make the policy application point a configurable parameter of the browse request, not a hardwired
behavior of the browse service. A request with scope `coldArchive` or `nearbyReserve` applies the active policy. A
request with scope `crateZone` or `preparedRoom` does not re-filter — the DJ placed those items deliberately.

### Constraint 4 — The browse result model must accommodate grouped items

The current browse result model must be designed as extensible to support Sleeve: a group with its own stable identity
that contains N playable candidate items. A flat-item-only result model cannot accommodate Sleeve without a breaking
schema change.

This does not mean implementing Sleeve now. It means: the result item type must be a discriminated union or extensible
type that can accommodate `singleItem` and `groupedItem` variants. The wire format, generated types, and renderer
contract must all be designed with this extension point reserved.

A single item and a sleeve item must be renderable from the same result list without a renderer-level type rupture.

### Constraint 5 — Track references must be extractable and storable independently of the browse result

When a track appears in a browse result, its stable identifier must be extractable from the result and storable in an
external object (CrateZone member, Hot Table slot, Live Path node) without requiring the full browse result to remain in
memory.

The browse result is a read artifact. The stable identifier is a persistent reference artifact. These must be different
types with explicit separation at the API boundary. The browse result must not be the only place the identifier lives.

### Constraint 6 — The tree scope model must support non-archive roots

The left-panel tree is currently built for Cold Archive sources and filesystem roots. Eventually, Prepared Rooms and
CrateZones must be browsable as tree roots in the same panel — not as filesystem paths, but as domain object scopes.

The tree architecture must not assume all roots are source/filesystem-based. The root node type must be extensible to
accommodate domain object roots (rooms, zones) that are resolved from the database layer rather than the source scan
layer.

This is not implementation work for now. It is an architectural constraint on the current tree root registration and
scope resolution path: do not hardwire it to source/filesystem identity.

### Constraint 7 — Source unavailability must not drop upper-layer references

When a source becomes unavailable (drive ejected, network source offline, source deleted), any Cold Archive items from
that source that are referenced in upper layers (CrateZones, Live Path, Hot Table) must remain as references in an
unavailable state — not silently dropped.

The substrate must model source-unavailability as a state on the item reference, not as item deletion. The upper layers
will render referenced-but-unavailable items differently than absent items. This distinction is critical for the
Prepared Room — a DJ must not lose their prepared work because their backup drive wasn't plugged in when they opened the
app.

---

## Deferred Work

The following are explicitly not being specified now. They are named here so future work knows they are expected and
knows what the foundation supports.

**CrateZone spatial layout engine** — the visual arrangement of zone members (front row / back row / left / right),
memory landmarks, and zone-relative coordinate system. Deferred. The domain model reserves `spatialLayout` fields
without specifying the layout schema.

**Nearby Reserve membership computation** — the algorithm for auto-populating the reserve from BPM neighborhood, key
compatibility, style clusters, or prior-set history. Deferred. The domain model defines the reserve as criteria-based
without specifying the criteria engine.

**Live Path graph rendering and performance-mode simplification** — how the node graph is presented during preparation
vs. during live performance. The graph is the correct data model; how it collapses into a simpler linear view for
high-pressure performance is a UX decision for later.

**Transition Object auto-population** — deriving entry/exit phrase suggestions, energy delta estimates, and harmonic
compatibility from analysis data, so TransitionObjects are not empty by default. Deferred. The schema is locked; the
inference engine is not.

**Shadow Path promotion mechanics** — the exact interaction contract for promoting a Shadow Path to the Live Path,
including what happens to nodes already played past the branch point. Deferred.

**Memory Landmarks** — visual spatial markers (color-coded danger crates, era markers, shelf positions). Deferred. The
domain model acknowledges them as room-scoped annotation objects; the schema is not yet defined.

**Sleeve authoring** — user creation of custom sleeves (grouping tracks not from the same release). Deferred. The Sleeve
domain object and SleeveRef type are defined; the authoring interface is not.

**Playable-media browse policy** — the eventual default policy that includes audio and video (see Browse Policy
Integrity document). Deferred to its own slice.

---

## North Star

> Dekzer makes the DJ's preparation visible, spatial, and performable.

> The archive is huge. The room is prepared. The table is hot. The path is live.
