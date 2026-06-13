# Library Browser Surface Contract

**Status:** proposal, accepted for implementation

## Purpose

Define the applied product contract for Dekzer's library browsing surface.

This document governs the user-facing mental model and ownership split for library browse surfaces. It sits beside
`docs/library/boundary/browser-workspace-surface-contract.md`, which owns workspace topology handoff, surface bounds,
viewport hints, and geometry exchange. Query mechanics remain owned by the contents and browse contracts.

## Applied Surface Model

The library surface is one continuous field with this applied structure:

```text
Library substrate -> Browse roots -> Active scope -> Projection -> Contents -> Selection -> Inspector / Preview / Actions
```

These are not sibling product domains. They are stages in one library workflow.

## Definitions

### Library substrate

The backend-owned state that records sources, source lifecycle, media evidence, accepted playable media, collection
membership, preparation evidence, and durable work/artifact state. It is not a renderer layout and not a workspace slot.

### Browse roots

The user's entry points into the substrate. Browse roots expose sources, collections, and workflow roots as selectable
scope origins. They do not own the contents rows they reveal.

### Active scope

The currently selected library scope. Its identity includes:

- selected browse root or scope
- projection mode
- filters
- browse profile
- scope depth policy

### Projection

The presentation mode used to read the active scope. Projection changes how contents are represented; it does not
change durable library state.

Legal projection modes:

- List
- Tree
- Columns
- Covers

### Contents

The main browsable result field for the active scope. Contents are read from substrate-owned facts through the accepted
contents/read boundary and browse policy contracts. Contents may present tracks, folders, collections, crates, or other
allowed row universes depending on active scope and policy.

### Selection

The active subject inside the current contents projection. Selection is library/renderer presentation state until an
explicit backend command commits a durable decision.

### Inspector / Preview / Actions

The support region for selected-subject metadata, artwork, preview, readiness, and allowed commands. It supports the
active selection and must not become a third sibling authority domain beside browse and contents.

## Sources And Collections

Sources are provenance-bearing origins only:

- local folders
- devices
- volumes
- cloud/adapters
- external libraries

Sources explain where material came from, what is reachable, and what lifecycle state applies. Sources do not become
playlists, crates, or semantic track ownership.

Collections are user or system workflow objects:

- playlists
- crates
- smart lists
- history
- workflow objects

Collections may reference media or scopes, but they do not replace source provenance.

## Ownership

| Owner              | Owns                                                                                  | Must not own                                                                   |
| ------------------ | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Backend/store      | Substrate state, source lifecycle, durable evidence, accepted objects, work artifacts. | Renderer presentation state, workspace placement, local projection cosmetics.  |
| Renderer           | Presentation state, active projection UI state, selection affordance, visual density. | Durable library facts, source lifecycle, artifact ownership.                   |
| Workspace topology | Placement, resize, visibility, docking/parking of the library surface.                | Source meaning, selected scope, contents query mechanics, projection meaning.  |
| Projection mode    | How the active scope is represented inside Contents.                                  | Durable library state, source identity, collection membership, backend facts.  |

## Perform Bench And Library Bench

In Perform Bench, the browse/library surface is subordinate support. It helps load, inspect, and act on material without
competing with decks, mixer, transport, timing, or performance readiness.

In Library Bench, Contents is the primary working surface. Browse roots and Inspector support navigation and focused
decision-making, but the contents field owns first-glance attention.

## Descendant-Media Default

Selecting a folder or source defaults to media-relevant descendant tracks as the musical scope. This makes a source or
folder selection behave like a useful music scope rather than a literal file-manager inventory.

Literal direct-child inventory is an explicit diagnostic/source-inventory mode. It is not the default musical browse
behavior.

## Readiness States

Library surfaces must distinguish these readiness states without presenting false emptiness:

- indexing
- missing
- blocked
- offline
- stale
- partial
- ready

These states describe source, scope, or contents readiness. They do not imply renderer failure and must not be collapsed
into one empty state.

## Drag Taxonomy

| Drag type               | Meaning                                                                    |
| ----------------------- | -------------------------------------------------------------------------- |
| Content drag            | Dragging a track, crate item, folder scope, or collection item as content. |
| Component-local drag    | Reordering or resizing inside one component, such as table columns.        |
| Workspace topology drag | Moving, swapping, parking, or resizing workspace-level surface slots.      |

Content drag and component-local drag must not mutate room-level topology. Workspace topology drag is governed by
workspace law and is unavailable during performance interaction unless an explicit edit/customize mode allows it.

## Generated UI Acceptance Notes

Generated UI mockups may be useful as visual exploration. They are not canonical product authority.

`Browser / Library / Details` as sibling labels is misleading because it treats browse, contents, and inspection as
three equal product domains. The preferred applied structure is:

```text
Browse roots / Contents / Inspector
```

Use that structure when reviewing library UI proposals, especially generated or exploratory screens.

## Cross References

- `docs/library/boundary/browser-workspace-surface-contract.md` owns workspace surface identity and topology handoff.
- `docs/library/contents/read-boundary-contract.md` owns parameterized contents reads.
- `docs/library/contents/selected-scope-depth-rule.md` owns selected-scope depth semantics.
- `docs/library/browse/policy-and-classification.md` owns browse policy and row-universe semantics.
- `docs/library/browse/representation-contract.md` owns representation classes and provenance.
- `docs/workspace/topology-families.md` owns Perform Bench and Library Bench topology families.
