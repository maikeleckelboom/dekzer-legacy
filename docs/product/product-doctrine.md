# Dekzer Product Doctrine

> Dekzer V0 is a minimal, reliable, local-first DJ application foundation.

Dekzer's long-term product model includes spatial performance memory, but V0 is local DJ basics first: sources,
indexing, browsing, search/filter/sort, clear availability states, deterministic reads, and stable renderer projection.
The post-deletion baseline has no legacy fallback path: removed asset, browser, preparation, capability, playlist, and
segment surfaces are not current product authority, aliases, fallback surfaces, or implementation targets.

---

## 1. Status and Authority

| Field         | Value                                                                                                                                                     |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Status**    | Product doctrine                                                                                                                                          |
| **Scope**     | Defines the V0 product target, long-term product model, and present-day substrate obligations implied by both                                              |
| **Not**       | A UI specification. An implementation specification. A domain object schema. A VR/AR roadmap.                                                             |
| **Authority** | This document owns product phasing, product laws, and roadmap pressure. The Prepared Room canonical foundations document owns the future room domain model. |

---

## 2. V0 Product Target

V0 is a serious local DJ foundation, comparable in basic product shape to mature local-library DJ software, but with a
narrower first scope.

V0 must provide:

- local source registration;
- local scan/indexing;
- source and folder browsing;
- descendant media scope by default;
- search, filter, and sort;
- clear missing and offline states;
- deterministic backend reads;
- stable renderer projection;
- later basic preparation and local performance workflow.

V0 explicitly excludes:

- hardware support;
- streaming services;
- product-backed Prepared Room UI;
- audience, social, AR, VR, or hardware room surfaces.

## 3. Long-Term Product Thesis

**Spatial performance memory** is Dekzer's future durable model of a DJ's prepared musical environment, live
commitments, alternative paths, runtime events, and post-set interpretation. It preserves not only what was played, but
where it lived in the prepared context, what alternatives existed, what changed during performance, and what evidence
explains the result.

Future room-based concepts are architectural compatibility targets, not V0 product scope. Current substrate work
supports them by preserving identity, availability, provenance, stable references, and evidence versus decision
boundaries, but the first product bar remains a minimal, reliable local DJ workflow.

Future waveform, analysis, workspace, hardware, Prepared Room, RT Flight Deck, and performance surfaces must project the
current local-first substrate. They must not introduce parallel schema authority or revive deleted substrate models.

---

## 4. Future Product Direction

Dekzer's long-term direction includes spatial performance memory: the ability to prepare, perform, revisit, inspect, and
evolve a DJ set as a durable room-shaped performance artifact.

- **Prepared Room** is a future preparation environment.
- **Performed Room** is a future performance-history artifact.
- **RT Flight Deck** is a future runtime diagnostic evidence domain.
- **Audience-attended rooms, AR/VR, and hardware room projection** are future projection surfaces.
- **The local-first substrate** is the current product foundation and the reason those later concepts can remain honest.

---

## 5. Present-Day Mandate

Dekzer builds local DJ basics on a correct substrate first.

Dekzer should feel immediately local-first: local filesystem entry points make likely music locations discoverable
without silently indexing them. Explicit admission and explicit scan protect users from broad-root mistakes. Local
browser candidates are not sources.

Library scanning. Canonical track identity. Source registration. Browse scopes. Contents reads. Media filtering. Event
pump. Lifecycle reads. Stable references. Deterministic renderer projection.

Current implementation authority is narrower than the long-term product model: source files, source_file_observations, content
attachments, source-file attachment links, playable-media observations, track-identity candidates, track-identity
decisions, source lifecycle/integrity, contents/search/filter, and navigation. Future product surfaces consume this
substrate when their contracts exist; they do not own it.

Prepared Room, Performed Room, RT Flight Deck, audience-attended rooms, AR/VR, and hardware room projection apply
architectural pressure now only where they preserve identity, availability, provenance, source isolation, stable
references, evidence versus decision boundaries, or future runtime diagnostic integrity. They are not current product
scope and must not be the primary justification for every backend slice.

No product-backed Prepared Room UI until every substrate admission gate in §12 is satisfied. Concept mockups and
research prototypes may exist only as non-authoritative exploration and must not introduce production state paths.

---

## 6. Future Performance Stack

```
Cold Archive
  → Nearby Reserve
    → Prepared Room
      → Hot Table
        → Live Path / Shadow Paths
```

The archive is huge. The room is prepared. The table is hot. The path is live.

This ordering is a future product law for room and performance features. It does not change when those features arrive,
but it is not the V0 product entry point.

_This document repeats the stack for product framing. The canonical domain definitions remain in the Prepared Room
foundations document._

---

## 7. Future Prepared Room Model

### Definition

A Prepared Room is a reusable prepared environment for a performance. It contains:

- **Zones** — spatial working regions; they carry placement, readiness, and intention but do not imply linear playback
  order
- **Nearby Reserve spec** — criteria for reachable material outside the staged room, scoped to the room context; not a
  global collection
- **Planned path structure** — candidate paths and pre-performance route sketches; not committed Live Path events
- **Grouped memory anchors** — eventually including Sleeves when the Sleeve domain exists
- **Room layout and metadata**
- **Session attachment points** — including Hot Table context during use; Hot Table runtime state is volatile and
  session-local, not part of the reusable room definition

A Prepared Room is intention. It does not contain performance events, committed path events, or runtime trace evidence.
Those belong to the performance instance and the performed artifact.

**Hard law:** A room surface that cannot preserve stable references, scope context, and unavailable items is not a
Prepared Room. A surface that additionally claims performed-room history must also preserve historical snapshots. Either
failure makes it a mockup.

**Hard law:** Prepared crates reduce search; they do not forbid archive access. The room is porous.

**Hard law:** Search results may be pulled into a room, zone, reserve, or Hot Table context. Search must not replace the
active room context as a global mode switch.

### What a Prepared Room Is Not

- Not a playlist.
- Not a queue.
- Not a graph canvas.
- Not automatic curation output.
- Not a renamed crate screen.
- Not a generic mind map.
- Not a global search mode.
- Not a replacement for the Cold Archive.

---

## 8. Future Spatial Mental Mapping

**Core product thesis:** DJs maintain a multidimensional spatial model of their prepared music. Energy, pressure, genre,
era, key, BPM, readiness, danger, familiarity, transition shape, crowd state, and contextual availability are all active
simultaneously, all spatially weighted.

Existing software mostly represents this through flat lists, playlists, sidebars, tabs, and search results. Those
surfaces can support preparation, but they do not preserve the DJ's spatial structure as a first-class product model.

Dekzer models the spatial structure instead of flattening it.

The Prepared Room, its zones, its reserve, and its paths are the product surface for that model. The Performed Room is
how that spatial structure persists across time.

---

## 9. Future Performed Room Lifecycle

**Central law:** Prepared Room is intention. Room Performance Instance is execution. Performed Room is evidence.

### Prepared Room Definition

The reusable prepared environment before a performance. Contains zones, reserve criteria, planned path structure,
grouped memory anchors, notes, and layout. May be opened, edited, and reused across multiple performances.

### Room Performance Instance

One actual performance run using a Prepared Room as its starting state.

A Room Performance Instance has its own stable identity and references the Prepared Room definition it started from.
Multiple performance instances may originate from the same Prepared Room.

It records the set as it unfolds: Live Path commits, Shadow Path promotions, Hot Table pulls, runtime events. These
committed path events are distinct from the planned path structure held in the reusable room.

### Performed Room Artifact

The preserved historical container after the performance closes. Contains:

- The prepared state at the start of performance — captured as a historical snapshot, not a live pointer to the mutable
  Prepared Room definition. This snapshot must not resolve through the current mutable room. It must remain stable even
  after the reusable room is later edited.
- The actual Live Path taken
- Deviations from planned paths
- Shadow Paths that were promoted
- Material staged but not played
- RT Flight Deck trace links (see §10)
- Post-set annotations

**Hard law:** A performed room is historical evidence. Editing the next room must not mutate the performed room.

**Hard law:** A Prepared Room can be reused. A Performed Room cannot be edited into the past. New preparation happens
through derivation or annotation, not historical mutation.

### Next-Room Derivation

A later Prepared Room may be derived from a Performed Room. The derivation creates a new room with its own editable
definition. The derived room records provenance linking back to the performed artifact but does not mutate it.

### Trace Retention

RT Flight Deck trace retention policy may determine how much runtime evidence is kept. Pruned or unavailable trace
segments degrade evidence detail but must not destroy the Performed Room artifact.

---

## 10. Future RT Flight Deck Relationship

The RT Flight Deck is a separate evidence domain. It provides runtime trace evidence that links into Performed Room
artifacts. The Performed Room holds references. The RT Flight Deck owns the trace content.

It is not decorative telemetry. It answers specific questions:

- What was the system state at a given moment in the performance?
- What runtime evidence surrounded a path deviation?
- What runtime event coincided with, interrupted, or affected this transition?
- What checkpoints exist in the set timeline?

**Hard law:** The RT Flight Deck must not infer musical intention. It records runtime evidence. The Performed Room and
post-set annotation layers interpret that evidence.

**Hard law:** A missing trace must not invalidate the Performed Room artifact.

---

## 11. Future VR/AR Projection Law

_Market note: spatial DJ surfaces already exist. This proves the interaction category is real; it does not define
Dekzer's product model._

Dekzer's opportunity is not to reproduce a booth in space, but to project its room/path/history model into spatial
surfaces once the core product is mature.

VR may project an immersive room. AR may overlay room/path/runtime context onto a physical performance setup. Both
remain projections over the same canonical model.

### Laws

1. VR/AR is a projection surface. It is not a second product model.
2. Desktop/local-first remains the canonical substrate. Spatial surfaces project it.
3. VR/AR must not introduce new canonical room state, path state, deck state, or performance history state.
4. VR/AR may read, project, annotate, and command through approved product boundaries only.

Law 3 exists to prevent a spatial prototype from becoming a second canonical authority. Any surface that begins owning
state has left its lane.

---

## 12. Substrate Admission Gates

**Product-backed Prepared Room work is blocked by the gates applicable to the slice being built. Any slice that claims
performed-room or performance-history behavior must also satisfy the Performance and History gates.**

If an applicable gate is unmet, a Prepared Room UI built on top of it is a facade.

### Library Substrate Gates

| Gate                          | Requirement                                                                                                                                                                                                                                                                                    |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Item identity**             | Track identity is canonical and source-path-independent. Survives drive remounts, path renames, source unavailability, and source unregistration without dropping upper-layer references. Intentional deletion, unregistration, missing drive, and identity recovery carry distinct semantics. |
| **Scope instantiation**       | Browse scopes are instantiable, not global singletons. Multiple concurrent instances with independent policy are supported.                                                                                                                                                                    |
| **Policy boundaries**         | Filtering and media policy are applied at scope boundaries. They do not contaminate global state.                                                                                                                                                                                              |
| **Grouped results**           | Scope results carry grouping structure compatible with future Sleeve rendering.                                                                                                                                                                                                                |
| **Stable refs**               | Every item a scope yields carries a stable reference that survives the scope being closed.                                                                                                                                                                                                     |
| **Reference status**          | A stable reference resolves to an explicit status: available, unavailable, missing, replacement candidate, or unresolved. Unavailable must not collapse into absent.                                                                                                                           |
| **Context-preserving search** | Search pulls results into the current room or zone context without replacing the active room scope as a global mode switch.                                                                                                                                                                    |
| **Pagination and windowing**  | Room zones and reserves support bounded reads. No assumption that a zone or reserve fits in memory unwindowed.                                                                                                                                                                                 |

### Room Domain Gates

| Gate                         | Requirement                                                                                                                                                                  |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Non-filesystem roots**     | The tree root abstraction does not assume filesystem. Room zones and virtual groupings must be supportable as future roots.                                                  |
| **Versioned room snapshots** | The room domain supports immutable capture of room start-state for performance history. This snapshot does not resolve through the current mutable Prepared Room definition. |
| **Boundary-owned mutation**  | Room membership, path commits, and performance events are mutated through explicit commands, not renderer-side state patches.                                                |

### Performance and History Gates

| Gate                                | Requirement                                                                                                                                                  |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Performance instance identity**   | A Room Performance Instance carries its own stable identity, independent of the reusable Prepared Room. Multiple instances may originate from the same room. |
| **Source unavailability isolation** | A missing or disconnected source does not invalidate references held by a Prepared Room, a Performed Room artifact, or a grouped memory anchor.              |

---

## 13. Ownership Boundaries

| Domain                 | Owns                                                                                                                                                                                                     |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Library substrate**  | Canonical item identity, source state, media eligibility, browse scopes, stable item references                                                                                                          |
| **Room domain**        | Prepared Room definitions, zones, reserve specifications, planned transition objects, room layout metadata, references to grouped anchors in room context — not Sleeve identity, membership, or metadata |
| **Performance domain** | Performance instances, Live Path commits, Shadow Path promotions, committed transition events and actual transition outcomes, set timeline                                                               |
| **RT Flight Deck**     | Runtime traces, incident evidence, frozen moments, checkpoint and replay evidence, runtime evidence around transitions                                                                                   |
| **Renderer**           | Projection, interaction, and ephemeral view state only (hover, focus, drag gesture, viewport, transient selection, open popovers, local layout measurement)                                              |

**Hard law:** The renderer does not own canonical room state, path history, or runtime evidence. It reads and projects.
Authority flows from domain owners through approved boundaries.

**Hard law:** Any renderer state required to reopen, replay, audit, or preserve a room is not renderer-owned.

**Hard law:** Room spatial structure is domain meaning. Renderer layout is projection. Do not confuse the two.

**Transition split:** The Room domain owns planned transition objects. The Performance domain owns committed transition
events and actual transition outcomes. The RT Flight Deck owns runtime evidence around those transitions.

Any component that reaches outside its owned column is a boundary violation.

---

## 14. Future Prepared Room First Slice

This is the first shippable Prepared Room slice after its gates are satisfied. It is not Dekzer V0 and must not be
implemented before the local DJ foundation is reliable.

**The future first room slice must deliver:**

- Create a room.
- Add zones to the room.
- Place stable item references into zones. Membership model must be Sleeve-compatible; no Sleeve UI is required.
- Remove an item reference from a zone without deleting the underlying archive item.
- Open a room-scoped browse context without replacing Cold Archive browse state or another active room/zone browse
  context.
- Persist minimal zone member placement or ordering across app restart.
- Persist room layout and membership across app restart.
- Show unavailable referenced items without dropping them from the room.

**The future first room slice explicitly excludes:**

- Persisted Hot Table history. Hot Table in this slice is session-local only; no Hot Table state survives app restart
  beyond an optional crash-recovery snapshot.
- Live performance mode.
- Live Path graph.
- AR/VR projection.
- Automatic recommendations or curation.
- RT Flight Deck UI.
- Performed Room artifact generation.
- Sleeve UI.

The first room slice is the room at rest. It must be correct before any performance concerns are layered on top.

---

## 15. Non-Goals

**Prepared Room:**

- Prepared Room is not a canvas-first feature.
- Prepared Room is not a playlist replacement with a different skin.
- Prepared Room is not automatic AI curation.
- Prepared Room is not a generic mind map.
- Prepared Room is not a global search mode.
- Prepared Room is not a replacement for the Cold Archive.

**Performed Room:**

- Performed Room is not a flat play history list.
- Performed Room is not editable history.

**Hot Table:**

- Hot Table is not persistent room membership.

**RT Flight Deck:**

- RT Flight Deck is not decorative telemetry.

**VR/AR:**

- VR/AR is not a separate application model.
- VR/AR is not the reason to build the room model.

**Renderer:**

- The renderer is not the owner of room, path, or performance history.

**Spatial performance memory:**

- Spatial performance memory is not a visualization feature. It is a product and domain model with persistence, runtime,
  and projection consequences.

---

## 16. Product Promise

Dekzer is a local-first, inspectable DJ library and preparation foundation.

Ledger means durable record, provenance, change history, evidence, auditability, reversibility, and accountable state.
The product must help DJs know what they have, what is ready, what changed, and what will work tonight.

**Design flag:** Will it work tonight?

The performance surface is downstream of the substrate. Decks, waveforms, effects, controller workflows, and live
surfaces must not become the first place the system discovers whether music is safe.

---

## 17. Identity, Claims, and Evidence

The word "track" must not collapse distinct identity layers. Source-file observations, byte identity, media
attachments, recording identity, collection track identity, analysis baselines, preparation artifacts, export
projections, and runtime use are separate owners.

**Hard law:** A claim belongs to one identity layer and one evidence basis. Moving a claim between layers requires an
explicit interpretation or decision record.

Many important values are claims rather than plain observations. Observed, inferred, imported, suggested, edited, accepted,
verified, invalidated, rejected, superseded, and exported states are not interchangeable.

- Imported observations do not silently become native authority.
- Machine recomputation does not overwrite accepted user decisions.
- Conflicts remain inspectable evidence until explicitly resolved.
- User acceptance chooses product-facing authority without erasing competing history.
- Preparation artifacts bind to the media or analysis basis they were created against.

Evidence grades are also distinct. Filename or extension declarations, byte inspection, computed analysis, secondary
verification, dry-run testing, export completion, and confirmation on the actual target prove different things.

---

## 18. Readiness and Trust

Readiness is a target-specific projection from current evidence, not a stored belief and not a property invented by the
renderer.

A readiness projection may be ready, verified, degraded, stale, suspect, unverified, blocked, unknown, or failed. The
exact vocabulary belongs to the relevant domain contract, but every projection must identify its target, evidence
basis, and invalidation conditions.

**Hard law:** A visual defect is a defect. A silent readiness lie is a trust failure.

The system must surface uncertainty during preparation. Source loss, media drift, changed analysis basis, stale
compatibility knowledge, unsupported formats, and unresolved identity must not wait until deck load to become visible.

---

## 19. Local Sovereignty and External Knowledge

The user's library and preparation state remain locally authoritative. Dekzer does not require remote indexing of the
library and does not send library contents, paths, preparation state, or play history to external services without
explicit consent.

External compatibility knowledge is versioned, updateable, cacheable, and pinnable. A gig may freeze the catalog
version used for its verdicts. Catalog changes make affected projections stale; they do not silently rewrite trusted
state.

Cloud sync, catalog lookup, external analysis, backup, and collaboration are additive services. Revoking them must not
invalidate local authority.

---

## 20. Performance Mode, Recovery, and Jobs

Performance mode is a substrate commitment, not a cosmetic UI mode. While active, automatic catalog application,
destructive migration, unapproved analysis rewrites, and background mutation of accepted preparation state are
forbidden. Passive observation may continue, but consequential change must remain staged and visible.

Operations that modify accepted preparation state require a recovery story. Import sessions, batch edits, scans,
analysis rewrites, export snapshots, preflight reports, catalog pins, and database backups must be attributable and
recoverable at the appropriate boundary.

Jobs are substrate actors. They must be bounded, observable, cancelable where cancellation is safe, attributable to
their outputs, resumable when the work model supports it, and diagnostic on failure. Jobs produce evidence, claims,
artifacts, or projections. They do not silently resolve ambiguity or overwrite accepted work.

---

## 21. Browse and Filter Doctrine

The product default active workflow filter is **Audio**.

- **Audio** shows audio workflow content.
- **Media** shows playable media, including audio and video where supported.
- **All Files** shows raw source inventory. It is not interpreted content and not a problems view.
- **Companion Files** is the user-facing label for admitted non-primary files associated with media workflows.

Product filter state, contents read policy, scope depth, and source inventory classification are separate concepts.
Protocol names such as `audioBrowse` and `playableMediaBrowse` may remain where they identify implemented policy
variants. They do not define product doctrine by themselves.

---

## 22. Modeling Laws

- Do not treat files as tracks.
- Do not treat byte identity as recording or track identity.
- Do not treat imported metadata as native authority.
- Do not store important preparation values without provenance.
- Do not flatten candidates, accepted observations, rejected observations, stale observations, and exported observations into one state.
- Do not treat compatibility as a boolean.
- Do not hide drift.
- Do not let runtime surfaces own durable product meaning.
- Do not apply consequential background changes without attribution and recovery.
- Do not collapse manual collections, query collections, snapshots, ordered sequences, request queues, performance
  paths, and export projections into one generic organization object.

---

## 23. Summary

Dekzer begins as a minimal, reliable local DJ workflow: register local sources, scan and index them, browse folders and
descendant media, search/filter/sort, show missing and offline states clearly, read deterministically, and project stable
renderer state.

That foundation later enables Prepared Rooms: durable spatial environments for preparation, performance, alternatives,
and post-set memory.

A performed room preserves not only the played set, but the prepared context, live decisions, deviations, transition
evidence, and runtime history. Future AR/VR surfaces project this model. They do not define a second one.

The substrate work is not preliminary scaffolding. It is part of the product's value: the reason rooms, references,
histories, and future spatial projections can be trusted.
