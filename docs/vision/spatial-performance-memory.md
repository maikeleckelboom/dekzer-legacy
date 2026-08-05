# Vision: spatial performance memory

**This document describes future direction. None of it is implemented.** Nothing here is a current roadmap commitment,
and nothing here should be cited to justify a backend decision on its own. For what Dekzer actually does today, read
[the product overview](../product.md) and [the roadmap](../roadmap.md).

It exists because the substrate being built now was chosen with a specific long-term product in mind, and a reader who
does not know what that product is will find some of the current constraints hard to justify.

## The thesis

DJs do not hold their prepared music as a list. They hold it as a space.

Energy, genre, era, key, tempo, readiness, familiarity, danger, transition shape, and crowd fit are all active at once
and all spatially weighted. A DJ knows that a particular record lives near the end of the set, next to two others that
work as alternatives, one of which is risky. That structure is real, it is built over hours of preparation, and it is
what actually gets used under pressure.

Existing software flattens it. Playlists, crates, tabs, sidebars, and search results can support preparation, but they
do not preserve the structure. When the set is over, what remains is a play history: a flat list of what happened, with
none of the context that explains why.

Spatial performance memory is the proposal that this structure should be a first-class, durable, inspectable product
object. Not a visualization layer over a list, but a model with persistence, identity, and history.

## The layers

Five terms, in order. They are not interchangeable and each answers a different question.

```text
Cold Archive
  -> Nearby Reserve
    -> Prepared Room
      -> Hot Table
        -> Live Path / Shadow Paths
```

**Cold Archive** is everything the DJ owns, indexed across all registered sources. Large, messy, and archival, with no
assumption of tidiness. This is what the current library substrate is building.

**Nearby Reserve** is material not staged in the room but close enough to reach during a set. It is a context-scoped
view into the archive defined per room, not a second permanent collection.

**Prepared Room** is a saved, named performance environment: the intentional subset and spatial arrangement prepared
before a set. It has its own identity, persists between sessions, and can be cloned and reused. It holds references to
archive items rather than copies of them.

**Hot Table** is the volatile surface for material in hand: pulled, previewed, parked, or rejected. It is session-local
and does not persist as part of the room.

**Live Path and Shadow Paths** are the committed performance direction and the uncommitted alternatives beside it. The
Live Path is a graph rather than a list, because a node may be resolved to a specific item or left unresolved as "I
want something from this zone here, and I will choose in the moment". An unresolved node is a valid state, not a gap.

## Intention, execution, evidence

The central distinction, and the one that shapes the substrate:

**A Prepared Room is intention.** It is the environment prepared beforehand. It can be edited and reused.

**A performance instance is execution.** One actual run that started from a Prepared Room. It has its own identity and
references the room it started from. Several instances may originate from the same room. It records the set as it
unfolds: path commits, promotions, pulls, and runtime events.

**A Performed Room is evidence.** It is the preserved historical container after the set closes. It holds the prepared
state as a snapshot taken at start time, the path actually taken, deviations from plan, alternatives that were
promoted, material staged but never played, and post-set annotations.

Two consequences follow, and they are why this matters to schema work now rather than later.

The historical snapshot must not resolve through the current mutable room. If it is stored as a pointer, editing next
week's preparation silently rewrites last week's history. It has to be captured, not referenced.

A Performed Room cannot be edited into the past. A later room may be derived from a performed one, recording provenance
back to it, but derivation creates a new editable object and leaves the original intact.

## Runtime evidence

Runtime evidence belongs to a distinct domain from the performance history that links to it.

The proposal is a runtime diagnostic layer that answers narrow questions: what was the system state at a given moment,
what surrounded a deviation from the plan, what coincided with or interrupted a transition, what checkpoints exist in
the timeline. It owns trace content. A Performed Room holds references into it.

Two constraints keep it honest. Runtime evidence records what happened and does not infer musical intention, which is
the job of the performance history and any post-set annotation over it. And a missing or pruned trace degrades the
detail available about a set without invalidating the Performed Room itself, because retention policy must never be
able to destroy history.

## Projection surfaces

Immersive and overlay surfaces are the most visible part of this idea and the least important one.

If room, path, and history exist as durable models, spatial surfaces become projections over them: an immersive view of
a prepared room, or an overlay of room and path context onto a physical setup. The value is in the model. The surface
is a way of reading it.

One rule governs any such surface, and it is the reason to state this at all rather than leave it unsaid: a projection
surface may read, render, annotate, and issue commands through existing product boundaries. It may not own canonical
room state, path state, or performance history. A surface that starts owning state has become a second product model,
and there is no way back from that without a rewrite.

The same rule applies to hardware. Controller and hardware integration would project the same model. Neither is on the
current roadmap, and neither justifies current backend work.

## What this asks of the substrate now

Most of this vision imposes no obligation on current work. A small part of it does, and only because those constraints
are hard to retrofit.

**Identity must be able to become path-independent.** References held by a future room have to survive remounts,
renames, and reorganization. This is why current work separates byte identity from occurrence and refuses to treat a
path as a durable identifier.

**Unavailability must not collapse into absence.** A referenced item on an unplugged drive has to remain a reference in
an unavailable state. A DJ must not lose prepared work because a backup drive was not connected at launch.

**Browse scope must be instantiable rather than global.** A room implies several independent browse contexts open at
once without overwriting each other.

**Result models must be extensible to grouped items.** A flat-item-only result cannot later represent a group with its
own identity containing several candidate items without a breaking change.

That is the whole list. It is short on purpose. These constraints are also defensible for the current local library on
their own terms, which is the test any of them had to pass to be honored now.

Everything else here waits until the local library foundation is trustworthy, because a room surface built on a library
that cannot yet answer what it has is a mockup.
