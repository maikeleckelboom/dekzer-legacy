# Dekzer — Hardware Screen & Performance Instance Doctrine

**Status:** FUTURE COMPATIBILITY DOCTRINE — Synthesis pending split
**Date:** June 2026
**Origin:** Hardware UX innovation session, critique, and correction synthesis.

---

## Document Scope and Authority

This is a bridge doctrine. It captures three distinct layers of product thinking in one place because they were
developed together. They must eventually be split into separate documents. Until then, every section carries an
authority marker:

- **[ACTIONABLE]** — Concrete enough for a future hardware-support slice after hardware enters product scope. Not V0.
- **[ARCHITECTURAL-DIRECTION]** — Must shape design decisions now but not fully implemented yet. Shape the data model
  even if UI ships later.
- **[FUTURE-SCOPE]** — Valid product vision. Not implementation input. Do not implement from this document.

Current product doctrine supersedes older hardware-first language in this document: Dekzer V0 has no hardware support,
no audience-attended rooms, no AR/VR, no hardware room projection, and no product-backed Prepared Room UI. Hardware,
audience, room projection, and performed-room concepts are future architectural compatibility targets only.

**Do not pass this document to Codex or other agents as V0 implementation input.** Future hardware implementation must
first receive an explicit product slice that revalidates the relevant sections against the then-current substrate.

The three eventual split targets:

1. `docs/product/hardware-decision-ui-doctrine.md` — Future hardware screen, input model, first-slice spec, row design,
   trust model. ACTIONABLE sections become active only after hardware enters product scope.
2. `docs/product/active-performance-instance-doctrine.md` — Active Performance Instance, event log, room operations,
   performed record, lifecycle. ARCHITECTURAL-DIRECTION sections.
3. `docs/product/audience-attended-room-doctrine.md` — Audience projection, visibility contracts,
   chat/reactions/history. FUTURE-SCOPE sections.

---

## Core Law [ACTIONABLE]

Dekzer hardware screens are **performance decision instruments**, not miniature desktop browsers.

A hardware screen answers one question at a time:

> **What can I safely do next?**

The list exists. The list is not the product idea. The list is one projection inside a larger decision model.
Compressing a desktop library UI onto a small rectangle is the mistake every other hardware screen makes. Dekzer does
not do this.

---

## The Three-Layer Separation

### Layer 1 — Hardware Decision UI [ACTIONABLE]

Small-screen, performance-stress UX. Deterministic input rules, trust-first row design, finger-aware layout. Does not
require audience layer or complex substrate. It is not part of Dekzer V0.

### Layer 2 — Performance Runtime Model [ARCHITECTURAL-DIRECTION]

Active Performance Instance, room movement operations, provenance events, performed history, event log. Must be designed
early. Cannot be retrofitted later. UI ships later; the event container shape cannot wait.

### Layer 3 — Audience-Attended Room [FUTURE-SCOPE]

Public presence, chat, reactions, replay, visibility contracts, social memory. Do not build the UI now. Do not let it
contaminate current local DJ foundation work. Preserve the event model shape for future runtime design.

---

## Canonical Live Objects [ARCHITECTURAL-DIRECTION]

### Active Performance Instance

**The canonical live object. Not the Prepared Room.**

The Active Performance Instance is the container of **event continuity** for a performance. It owns:

- Current room context (the room the performance currently inhabits)
- Reference to the Performance Event Log
- Room movement operation history (provenance)
- Source incident records
- Session-scoped played-state

**It does not own live deck state.** The audio-critical deck runtime owns live deck state. The Active Performance
Instance records and projects deck-relevant performance events and snapshots. These are distinct authority boundaries.
Conflating them entangles the event model with the real-time engine.

A performance may inhabit, switch between, borrow from, fork, overlay, or receive external injections from rooms. The
Active Performance Instance is what continues unbroken when the DJ moves. It is never destroyed mid-event.

### Performance Event Log [ARCHITECTURAL-DIRECTION]

The durable observation store for a performance. Every room operation, track commitment, source incident, deck snapshot, and
visibility change produces an event appended to this log. Append-only. Never mutated.

The Performance Event Log is the canonical source of truth for:

- What happened and in what order
- Which room operations were performed and with what provenance
- What tracks were played and when
- What source incidents occurred
- What the Performed Room record contains

### Prepared Room

A planned musical environment. One possible context the Active Performance Instance may inhabit. Not the live event
container. A Prepared Room is a performance environment, not a prison.

### Performed Room [ARCHITECTURAL-DIRECTION]

A **materialized projection** of the Performance Event Log, not a separately maintained record. The Performed Room is
derived from the event log — it is the log rendered as a human-legible performance record.

Because it is a projection, correcting or reprocessing the Performed Room requires only reprocessing the event log. The
log is authoritative. The Performed Room is not stored as a peer object to the log.

Projection includes: rooms inhabited, operations performed, tracks played and skipped, source incidents, forks, borrows,
external injections. Not a playlist. A performance memory.

### Audience Room [FUTURE-SCOPE]

A visibility-governed projection of the Active Performance Instance. Attaches to the event, not to any fixed room. The
audience follows the performance; they do not own it. The DJ's runtime authority is never delegated to audience
presence. The audience sees the performance story. The DJ sees the cockpit.

---

## Formal Room Operations [ARCHITECTURAL-DIRECTION]

These are **provenance events appended to the Performance Event Log**, not UI labels. Each operation must be recorded
with its full schema. Without schema enforcement, "Room Borrow" and "Room Switch" remain prose.

Do not model requested, cancelled, failed, and committed operations as one ambiguous event shape. Introduce lifecycle
event kinds:

- `RoomOperationRequested`
- `RoomOperationCommitted`
- `RoomOperationCancelled`
- `RoomOperationFailed`

Only `RoomOperationCommitted` events may change the active room context. `RoomOperationCancelled` and
`RoomOperationFailed` events may be logged, but they must not be presented as successful provenance operations. They
record that an operation was attempted and did not complete; they do not materialize room context change.

### Minimum Operation Schema

Every room operation records the following fields:

| Field                   | Type                                                                                                | Required    | Description                                                                                |
| ----------------------- | --------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------ |
| `operation_id`          | UUID                                                                                                | Yes         | Unique identifier for this event                                                           |
| `performance_id`        | UUID                                                                                                | Yes         | Parent Active Performance Instance                                                         |
| `timestamp_ms`          | i64                                                                                                 | Yes         | Unix epoch milliseconds                                                                    |
| `actor`                 | `Dj \| System \| Integration`                                                                       | Yes         | Initiator of the operation                                                                 |
| `kind`                  | `RoomOperationRequested \| RoomOperationCommitted \| RoomOperationCancelled \| RoomOperationFailed` | Yes         | Lifecycle event kind                                                                       |
| `operation_type`        | `Switch \| Borrow \| Fork \| Overlay \| Injection`                                                  | Yes         | Operation type                                                                             |
| `source_room_id`        | UUID?                                                                                               | Conditional | Room material was sourced from (Borrow, Fork, Overlay)                                     |
| `target_room_id`        | UUID?                                                                                               | Conditional | Room switched or forked into (Switch, Fork)                                                |
| `track_ids`             | Vec\<UUID\>                                                                                         | Conditional | Tracks involved (Borrow, Injection)                                                        |
| `track_trust_snapshots` | Vec<TrustVectorSnapshot>                                                                            | Yes         | Trust state of each involved track at operation time                                       |
| `visibility_class`      | VisibilityClass                                                                                     | Yes         | Visibility assigned at operation time                                                      |
| `note`                  | String?                                                                                             | No          | Optional DJ or system annotation                                                           |
| `resulting_context`     | RoomContextSnapshot                                                                                 | Conditional | Active room context after operation completes. Required only for `RoomOperationCommitted`. |
| `failure_reason`        | OperationFailureKind?                                                                               | Conditional | Failure reason if kind is `RoomOperationFailed`                                            |

### Room Switch

The Active Performance Instance changes its primary room context. Hot Table, Nearby Reserve, Shadow Paths, and candidate
suggestions now derive from the new room.

Audience-visible label: `"The room changed. The set entered a new chapter."`

### Room Borrow

The Active Performance Instance uses material from another room without changing the current context. The borrowed track
carries provenance in the event log. The current room remains active.

Audience-visible label: `"A track was brought in from another room."`

### Room Fork

The current performance direction branches into a new live path. The original room plan is preserved. A new branch is
created live. Turns improvisation into durable structure.

Audience-visible label: `"The set forked into a new direction."`

### Room Overlay

Another room becomes visible as an alternate comparison layer without committing. DJ-private by default. Audience
visibility is determined by the assigned visibility class.

### External Injection

A track enters the performance from outside all prepared rooms. Dekzer never punishes improvisation. Records it
honestly.

Audience-visible label: `"An unplanned track entered the performance."`

---

## Future Hardware First-Slice Specification [ACTIONABLE]

Do not start with: room radar, audience rooms, Shadow Path editing on hardware, Live Path visualization, Smart Browse
modes, reason graphs.

These require substrate that does not yet exist.

### Row Data — Tiered by Substrate Availability

Future hardware rows may render only fields whose substrate exists. Fields without substrate render as absent, not
inferred. The UI must never display an estimated or invented value where the substrate is unavailable.

**Available now — no new substrate required:**

- Title (if extractable from file metadata)
- Artist (if extractable from file metadata)
- Duration (if extractable)
- Source availability state (file present vs. missing — from cached source observations)
- Currently-loaded indicator (which deck this track is loaded on, if any)

Row rendering consumes cached source observations. It must not perform blocking filesystem checks on the hot browse path.
The renderer does not stat files during scroll.

**Available after analysis pipeline exists:**

- BPM delta from currently playing deck
- Key compatibility indicator
- Grid confidence (feeds the `GRID_UNCERTAIN` trust dimension)
- Cue readiness (feeds the `CUE_INCOMPLETE` trust dimension)

**Future — requires models not yet in scope:**

- Energy movement direction (requires energy classification model)
- Known Transition indicator (requires transition history or inference model)
- Safe Next scoring (requires full substrate contract — see Substrate Dependency Gate)
- Reason graph ("why am I seeing this")
- Path membership (Live Path, Shadow Path)

**Unknown is a valid display state. Invented safety is not.**

### Trust Vector Model [ACTIONABLE]

Readiness is not a single enum. States are not mutually exclusive. A track can be `LOCAL_VERIFIED` and `CUE_INCOMPLETE`
simultaneously. A track can be `EXTERNAL_REF` and `MISSING` simultaneously. A single flat glyph cannot model this
without lying.

Model readiness as a **four-dimension trust vector:**

The trust vector is raw state, not global product logic. Severity is feature-contextual. Each browse mode or feature
defines which trust states block eligibility. For example, `GRID_UNCERTAIN` may block `SAFE_NEXT` but not basic playback;
`UNANALYZED` may still be usable in `RECOVER` mode. The vector itself does not dictate policy; the consuming feature does.

#### Dimension 1 — Source Availability

| State            | Meaning                                                           |
| ---------------- | ----------------------------------------------------------------- |
| `LOCAL_VERIFIED` | File present on local disk, path confirmed                        |
| `EXTERNAL_REF`   | Reference points to an external library (Rekordbox, Serato, etc.) |
| `SLOW`           | Accessible but read speed is degraded                             |
| `MISSING`        | File path cannot be resolved                                      |
| `UNKNOWN`        | Not yet checked                                                   |

#### Dimension 2 — Analysis State

| State            | Meaning                                                |
| ---------------- | ------------------------------------------------------ |
| `ANALYZED`       | BPM, key, grid data present with acceptable confidence |
| `GRID_UNCERTAIN` | Analysis exists but grid confidence is below threshold |
| `UNANALYZED`     | No analysis data                                       |
| `UNKNOWN`        | Analysis state not yet determined                      |

#### Dimension 3 — Preparation State

| State            | Meaning                              |
| ---------------- | ------------------------------------ |
| `CUE_READY`      | Required cue points present          |
| `CUE_INCOMPLETE` | Cue points absent or partial         |
| `UNKNOWN`        | Preparation state not yet determined |

#### Dimension 4 — Provenance State

| State           | Meaning                                                     |
| --------------- | ----------------------------------------------------------- |
| `INTERNAL`      | Belongs to the active prepared room                         |
| `BORROWED`      | From another room via a Borrow operation                    |
| `INJECTED`      | External injection — outside all rooms                      |
| `DUPLICATE_REF` | Multiple source files match; provenance ambiguous           |
| `EXTERNAL_ONLY` | Exists only as an external library reference; no local file |

#### Derived State: PERFORMANCE_READY

`PERFORMANCE_READY` is computed, not stored as a peer dimension. It is stricter than playable.

```
PLAYABLE = (source == LOCAL_VERIFIED)

PERFORMANCE_READY = (source == LOCAL_VERIFIED)
                 && (analysis == ANALYZED)
                 && (preparation == CUE_READY)
```

`PERFORMANCE_READY` is displayed as a positive compound indicator only when all three conditions pass. `PLAYABLE` is a
weaker guarantee: the file is present and verifiable, but analysis or cue preparation may still be incomplete.

#### Display Rule

Blocking states win visually: `MISSING > SLOW > UNKNOWN` within source; `GRID_UNCERTAIN > UNANALYZED > UNKNOWN` within
analysis. Every row exposes a trust summary. Blocking states are displayed prominently. Orthogonal provenance and
preparation states appear as secondary indicators. A row must never display `PERFORMANCE_READY` when any blocking state is present in
any dimension.

### Fixed Inspection Strip [ACTIONABLE]

A reserved screen area showing the selected or touched candidate's detail independent of scroll position and finger
occlusion. Not a tooltip. A permanent strip. This is the primary structural fix for the hand-covering-content problem
visible in reference hardware.

The inspection strip shows: title, artist, trust vector summary, armed load target, and all available analysis data for
the candidate.

### Persistent Armed Load Target — Future Hardware Scope [ACTIONABLE]

Always visible on screen when hardware support exists. Examples: Deck A, Deck B, Preview (headphone cue monitoring, if
hardware supports). General rule: Deck 1..N where supported, plus Preview.

**Hot Table and Shadow Path are not first-slice hardware load targets.** They require path and prepared-room substrate
that does not yet exist. Including them in the first hardware slice creates a dependency on infrastructure that is not
available.

A separate, explicit UI act changes the armed target. No implicit or state-dependent interpretation of the load button.
No accidental commitment.

#### Hardware Profile Distinction

Not all hardware shares a single load button model. The armed target behavior must adapt:

| Profile                   | Armed Target Behavior                                                                                               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **Shared load button**    | Persistent armed target must always be visible. Explicit DJ action required to change it.                           |
| **Per-deck load buttons** | Armed target is physically encoded by the button pressed. Screen shows commit destination as confirmation feedback. |
| **Touch-primary**         | Explicit target selection or confirmation gesture required before commit.                                           |

The encoder-primary focus state machine below assumes shared-load or touch-primary hardware. Per-deck hardware may
simplify or eliminate the global armed target.

### Encoder-Primary Focus State Machine [ACTIONABLE]

Swipe gestures are **not core interactions**. They are unreliable under performance stress (gloves, sweat, pressure).
Swipe may exist as an optional accelerator. It must never be the only path to a critical action.

Primary state machine:

```
Encoder turn     → list scroll mode (encoder selects row, touch inspects)
Touch row        → inspect mode (row detail expands to inspection strip)
Load button      → commit selected candidate to armed deck target
Load hold        → preview to headphones (if hardware supports)
Encoder push     → reserved (future: pin candidate — substrate required)
```

Every state has one deterministic primary action. This state machine must be formally documented before any hardware
screen implementation begins.

### Hardware Control ↔ Screen Co-Ownership [ACTIONABLE]

The screen is context-reactive to the physical control surface. Rules are deterministic, not emergent.

| Physical control   | Screen shifts to                      |
| ------------------ | ------------------------------------- |
| Encoder turn       | List focus mode                       |
| Touch screen       | Direct inspect mode                   |
| Load button        | Commit to armed target                |
| EQ / stem controls | Stem and track structure view         |
| Loop controls      | Phrase and loop affordance            |
| Pitch fader moved  | Tempo relationship, BPM delta display |
| Cue button         | Cue map for loaded track              |

### Waveform Integration Principle [ACTIONABLE]

Track inspection must be able to expose phrase, cue, and transition affordances without replacing the browse context
entirely. Waveform display and browse list compete for the same pixels on hardware. The layout must have a defined
arbitration rule before implementation.

Unresolved questions drive ADR 2 (see below): mini-waveform in inspection strip; cue readiness exposed on waveform view;
safe entry/exit window marking; whether the waveform belongs to the selected candidate, the loaded deck, or both.

---

## Substrate Dependency Gate [ACTIONABLE]

No hardware screen concept becomes product doctrine until its substrate contract names: required data, substrate owner,
unavailable behavior, partial-data behavior, and what the UI must never do. Every future "smart" browse mode has a
dependency chain. If the dependency does not exist, the feature does not exist.

## Future Dependency Matrix [ARCHITECTURAL-DIRECTION]

The substrate contracts below are not yet actionable. They shape design decisions now and define the boundary between
what is buildable and what remains future scope.

| Feature                                 | Required substrate                                                                                                                  | Substrate owner                                            | Unavailable behavior                                            | Partial-data behavior                                                          | Must never                                                                                          |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------- |
| **Safe Next mode**                      | BPM relation, key relation, local file verification, cue readiness, grid confidence, energy model, session played-state, deck state | Library backend + analysis pipeline + session runtime      | Mode hidden or disabled. Fallback to `UNSCOPED_LIBRARY` browse. | Track may appear in generic browse but not as a Safe Next candidate.           | Invent safety. Show a track as Safe Next when substrate is absent or confidence is below threshold. |
| **"Why am I seeing this?"**             | Typed reason data emitted alongside each candidate by the suggestion engine                                                         | Library suggestion engine                                  | Feature hidden. No reason display.                              | Show only confirmed reasons. Do not pad with inferred reasons.                 | Generate post-hoc natural language from the display layer. Fabricate provenance.                    |
| **Live Path / Shadow Path on hardware** | Route/path data structures, candidate membership per path, commitment state, branch/fork operations                                 | Performance runtime + library backend                      | Hardware shows browse only. No path UI rendered.                | Show path as read-only if path data exists but live editing is unavailable.    | Mark a track as "on path" without confirmed path membership data.                                   |
| **Performed Room replay**               | Durable performance event log, continuous from session start                                                                        | Active Performance Instance / event store                  | No replay capability. Replay UI must not appear.                | Replay available for captured portion only; show coverage boundary explicitly. | Synthesize replay events not present in the log.                                                    |
| **Audience-attended room**              | Visibility contracts, public projection layers, event continuity, moderation model                                                  | Performance runtime + visibility layer + platform services | No audience features. No stubs or placeholders.                 | N/A — all-or-nothing at platform layer.                                        | Grant audience any authority over DJ runtime state.                                                 |

---

## Browse Mode Taxonomy [ARCHITECTURAL-DIRECTION / FUTURE-SCOPE]

These modes are the correct product framing. They are not V0.

`EXTERNAL` has been removed from this taxonomy. It is a provenance dimension, not a decision intent. Tracks from outside
all prepared rooms appear in any browse mode; they are marked by their provenance dimension (`INJECTED`,
`EXTERNAL_ONLY`), not isolated into a separate mode.

`UNSCOPED_LIBRARY` replaces `EXTERNAL` as the general-collection browse mode.

| Mode               | Intent                                                                                                                                       | Substrate status                               |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `NEARBY`           | Compatible candidates in current musical context                                                                                             | Requires energy + key/BPM model                |
| `PREPARED`         | Hot Table and Reserve from active room                                                                                                       | Requires Prepared Room sync                    |
| `SAFE_NEXT`        | Analyzed, local, key/BPM compatible, cued, unplayed, deck-state aware                                                                        | Requires full substrate contract above         |
| `RECOVER`          | Known-safe tracks, playable now, regardless of musical fit                                                                                   | Requires local verification + readiness state  |
| `ENERGY_UP`        | Higher energy classification                                                                                                                 | Requires energy model                          |
| `ENERGY_DOWN`      | Lower energy classification                                                                                                                  | Requires energy model                          |
| `KNOWN_TRANSITION` | Documented or inferred transition compatibility with current track                                                                           | Requires transition history or inference model |
| `SHADOW_PATH`      | Alternate routes prepared before the gig                                                                                                     | Requires path data structures                  |
| `RESERVE`          | Prepared but not on Live Path                                                                                                                | Requires Prepared Room + path membership       |
| `UNSCOPED_LIBRARY` | Full collection browse without requiring active room membership. Results still carry provenance relative to the active room when one exists. | Available now                                  |

`UNSCOPED_LIBRARY` is the only mode requiring no new substrate. All other modes have explicit dependency chains that
must be satisfied before the mode ships.

---

## Future: Audience-Attended Room [FUTURE-SCOPE]

Do not build this UI now. Do not let it contaminate current local DJ foundation work or future first-slice hardware
decisions.

The architectural requirement it proves: **the Active Performance Instance must be a durable event container from day
one.** If only "current room" is modeled today, event continuity, room movement provenance, replay indexing, and
visibility projection must be retrofitted later. That is not acceptable.

Design the event container now. Build the audience UI later.

### Audience Interaction Principles

- Audience can: attend, react, chat, save moments, follow the played path, submit requests (with DJ-controlled admission
  gate).
- Audience cannot: own room context, influence deck state, override DJ authority, access private layers.
- Room movement is narrated to the audience with public labels. The DJ's control surface is never exposed.

### Audience-Visible Performance Events

```
"The artist opened another room."
"A track was brought in from another room."
"The set forked into a new direction."
"An unplanned track entered the performance."
```

Improvisation is part of performance. Dekzer does not hide it. It labels it honestly.

---

## Visibility Contracts [ARCHITECTURAL-DIRECTION]

Every room object and operation carries a visibility class. This class is recorded in the Performance Event Log at
operation time and is immutable for that event record.

**Default visibility is `PRIVATE_DJ`.** Audience visibility is never inferred from performance participation. Public
archive is an explicit publish action, not automatic history.

| Class                  | Scope                                                                                                      |
| ---------------------- | ---------------------------------------------------------------------------------------------------------- |
| `PRIVATE_DJ`           | Shadow Paths, Hot Table, source health, uncommitted next tracks, private notes, RT Flight Deck diagnostics |
| `VISIBLE_AFTER_PLAYED` | Individual track and transition detail; released per-track as each track enters played state               |
| `VISIBLE_DURING`       | Current room chapter, current track, played path, room switch events                                       |
| `VISIBLE_AFTER_SET`    | Full performed timeline, room movement history, transition annotations                                     |
| `COLLABORATOR`         | Extended access per explicit permission grant                                                              |
| `TICKETED_AUDIENCE`    | Curated projection during performance                                                                      |
| `PUBLIC`               | Post-set archive, highlights, summary                                                                      |

The Performed Room inherits the strictest visibility class of all contributing events by default. Visibility may only be
**relaxed** (widened) by an explicit post-set publish action. Visibility is never tightened post-hoc — events are
recorded at operation-time visibility and that record is immutable.

A post-set publish action appends a new visibility or publication event to the Performance Event Log. It never mutates
the original event record. Relaxation is append-only, not edit-in-place.

---

## Product Laws

1. **Hardware compresses trust, fit, readiness, and provenance. It never invents them.**
2. **Active Performance Instance is the live continuity object. Rooms are contexts, not containers of the event.**
3. **A room is a performance environment, not a prison. The artist may switch, borrow, fork, overlay, or inject without
   restriction.**
4. **The Performed Room is a materialized projection of the Performance Event Log. It records what actually happened,
   including all deviations.**
5. **Audience attaches to a visibility-governed projection of the active performance. Audience never owns runtime state.**
6. **No concept becomes product doctrine without a named substrate contract, runtime operation, and failure behavior.**

---

## Open Questions — ADRs Required

Each item below is a named gap requiring a separate ADR before any implementation that depends on it.

**ADR 1 — Prepared Room Sync Model**
How is a Prepared Room transported to hardware? USB export (CDJ model), local network discovery, or cloud sync? These
carry different trust and reliability profiles. The "is my prepared world here with me?" promise depends entirely on
this answer. Must cover: partial sync behavior, stale room detection, missing file behavior, and sync failure states (
see also ADR 11).

**ADR 2 — Waveform Integration on Hardware**
The hardware screen must arbitrate between browse/list mode and waveform display. Open questions: can inspection show a
mini-waveform in the strip? Can row focus expose cue readiness on the waveform? Can Safe Next identify entry/exit
windows? Does the waveform belong to the selected candidate, the loaded deck, or both? What is the layout arbitration
rule when they compete for pixels?

**ADR 3 — Multi-Deck Browse Context**
Does each deck maintain an independent browse and filter context? If Deck A is mid-phrase at 127 BPM and Deck B is
preloaded, does that affect Safe Next candidate scoring? It should. Define the dependency model between deck state and
browse context.

**ADR 4 — Live Path Assignment on Hardware**
Can a DJ create or modify Shadow Paths live on hardware, or is path assignment desktop-only? This determines whether
hardware browse is read-only relative to the path model and whether the encoder push reservation above has a V1 target.

**ADR 5 — Active Performance Instance Lifecycle**
When does the instance start? When Dekzer opens for a session, on first track played, or on an explicit "start set"
action? When does it end? These answers determine event log coverage boundaries and replay fidelity.

**ADR 6 — Performance Event Log Schema**
Define all event families. Minimum required families: deck events (load, eject, play, pause, cue set), room operations (
the five formal operations), source incidents (file missing, slow, verification failure), session events (start, end,
pause, resume), visibility changes, and annotations. Define field schema for each family. This ADR is a prerequisite for
the Performed Room materialization layer.

**ADR 7 — Runtime Authority Boundary**
Formal boundary definition: what owns live deck state (audio engine), what owns event capture (Active Performance
Instance), what owns materialization (Performed Room projection layer), and what owns durable record (Performance Event
Log). No authority boundary ambiguity is acceptable here. The real-time engine and event model must not entangle.

**ADR 8 — Trust Vector Model Finalization**
Adopt the four-dimension trust vector from this document as the canonical readiness model. Defines: confidence
thresholds for `GRID_UNCERTAIN`, required cue point definition for `CUE_READY`, `PERFORMANCE_READY` derivation logic and caching
strategy, display priority rules for blocking vs. secondary states, and how the trust vector is serialized into the
operation schema's `TrustVectorSnapshot`.

**ADR 9 — Hardware Profile Model**
How do interaction rules, state machines, and armed target behavior differ across: encoder-primary/touch-secondary (
SC6000-class), touch-primary (Rane/Denon-class), per-deck load buttons, shared load button, and screenless or
limited-screen devices? Each profile may require a distinct interaction rule set. The hardware abstraction layer must
expose a profile type that the screen logic consumes.

**ADR 10 — Visibility Publish Workflow**
Who can relax visibility after the set? What relaxation actions are auditable? What steps are irreversible? What
portions of the Performed Room remain permanently private regardless of publish action? Define the post-set publish flow
and its invariants.

**ADR 11 — Prepared Room Sync Failure States**
What does the hardware screen show when a room is partially synced, stale, contains missing files, has missing analysis,
or has unresolved external references? Hardware must never show false confidence about room readiness. Define the
degraded-room display states, their visual treatment, and their escalation path to the DJ.

---

_This document preserves future hardware and performance compatibility constraints. It is not V0 implementation input.
Hardware screen work requires a dedicated future product slice before ACTIONABLE sections become implementation
authority. Performance runtime work references ARCHITECTURAL-DIRECTION sections only after the runtime slice is in
scope. FUTURE-SCOPE sections require their own dedicated doctrine before any implementation._
