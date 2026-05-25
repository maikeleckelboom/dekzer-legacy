# Library to Deck Performance Boundary

_Canonical authority boundary document. Defines the ownership domains that separate library substrate,
deck runtime, performance session, and broadcast projection. Governs all future readiness, scheduler,
deck, performance, and broadcast implementation._

---

## Purpose

Dekzer's library substrate, deck runtime, performance session, and broadcast projection are separate
authority domains. Each owns distinct state, distinct invariants, and distinct mutation paths.

This document does not implement deck runtime.
This document defines the ownership boundary that readiness, scheduler, deck, and broadcast work must obey.

The missing boundary is:

```
library_item -> readiness_target -> deck_load_request -> loaded_deck_item
  -> performance_session_event -> broadcast_projection
```

The current library substrate work is grounded: source access, scan coverage, selected contents,
media visibility policy, and source-file browsing. Role classification is defined as the next
product layer. Deck state, transition events, track identity, and performance session projection
exist now as architectural boundaries only. Those boundaries must be recorded before readiness
and scheduler work goes deeper.

---

## Core Law

```
A library item does not become a deck item.
Deck load creates a runtime binding owned by the deck authority.
```

Readiness does not load the deck.
Scheduler does not own deck state.
Broadcast does not observe mutable library rows directly.
Renderer does not coordinate the transition.

---

## Entity Chain

```
source_file
  discovered filesystem inventory

library_item
  durable product-addressable object derived from source inventory, embedded streams,
  generated assets, or user-created objects

item_role
  product meaning such as performance_item, visual_asset, artwork_candidate,
  companion_metadata

readiness_target
  explicit target capability such as audio_deck_load, video_deck_load,
  visual_output, artwork_attachment, or future deck/output targets

readiness_proof
  bounded answer that says whether the item may be used for that target now,
  or what blocks it

deck_load_request
  control-plane command requesting that a specific library item be loaded into a
  specific deck target

loaded_deck_item
  live runtime binding owned by the deck authority, referencing a library item
  and runtime resources

deck_runtime_state
  live state such as loaded item, playhead, tempo, pitch, cue/loop state,
  sync participation, transport state, resource handles, and invalidation state

performance_session_event
  durable or replayable event emitted from runtime actions: load, unload, play,
  cue, seek, loop, transition, mix, route progress, deck handoff

broadcast_projection
  externally publishable projection of performance/session state; not the owner
  of library or deck authority
```

---

## Authority Boundaries

### Source Access Authority

Owns:

- `source_locations` rows and source location lifecycle where the lifecycle concern is
  access or location scoped
- Mount and access state
- Source access issue classification
- Source availability as source-level access state

Does not own:

- Library item identity
- Roles
- Deck readiness
- Deck runtime
- Scan inventory facts

### Scanner / Inventory Authority

Owns:

- `source_files` rows as discovered filesystem inventory
- `source_directories` rows
- `source_scan_state` rows, scan phase
- `presence_state`
- Directory scan state, directory affordance facts
- Source-file provisional `media_class` used only as inventory evidence and browse
  visibility input
- Discovery timestamps and observation facts

Does not own:

- Library item identity
- Role meaning
- Deck-load eligibility
- Readiness answers
- Deck state

Rejection cases for this authority:

- `source_files.availability_state` is treated as deck or runtime availability
- `source_files.media_class` or `presence_state` is used as readiness proof or
  deck-load eligibility

### Library Item Authority

Owns:

- `library_items` rows
- `item_origins`
- `file_identities` or equivalent identity binding
- Stable product-addressable identity

Does not own:

- Source access state or mount state
- Scan phase or discovery facts
- Deck runtime
- Performance events

### Role Classification Authority

Owns:

- `library_item_roles` rows, including classifier-originated and user-originated
  role decisions
- `library_item_role_events` rows

Does not own:

- Deck state
- Readiness execution
- Scheduler work identity
- Library item identity mutation

### Readiness Authority

Owns:

- `item_readiness` rows (one per `library_item_id, target`)
- Target-specific readiness answers: proof that the item may be used for that target
- Readiness re-evaluation requests (may request scheduler work; does not own work item rows)
- Invalidation in response to substrate events (source change, probe update, role change,
  policy change)

Reads:

- Library evidence (roles, origins, source availability, media streams)
- Probe facts, analysis artifacts
- Target policy

Does not own:

- Deck runtime or loaded deck item creation
- Library identity mutation
- Performance session event creation
- Readiness work execution (the scheduler runs readiness work; the readiness evaluator
  owns the output rows)
- Expensive inline evaluation (must enqueue work through the scheduler for anything
  beyond a bounded synchronous check)

Rules:

- Readiness is target-specific. Global `deckReady` is rejected. Deck loading needs an
  explicit target such as `audio_deck_load` or `video_deck_load`. Do not use generic
  deck readiness when the real question is readiness for a specific runtime target.
- Readiness answers must be bounded.
- Pending readiness may request scheduler work.
- Readiness evaluation must not perform expensive work inline.
- Readiness must not own deck state.
- Readiness must not create loaded deck items.

### Scheduler

Owns:

- Work enqueue, admission, selection, leasing, priority computation, execution order,
  budget enforcement, cancellation flow
- `work_items` rows, `scan_runs` rows
- Work identity idempotence
- Resource budget group management
- Foreground interest expiry

Reads:

- Substrate tables for scheduling decisions
- Foreground interest signals, explicit user requests, deck load requests (via control
  plane)

Does not own:

- Readiness output rows, probe results, role assignments, source observations
- Deck state, loaded deck items
- Performance session events
- Broadcast projection
- Work output tables (workers write outputs only through the owning authority)

Rules:

- A deck load creates foreground interest or an explicit readiness request.
- Scheduler may promote readiness/probe/hash/resource work in response.
- Scheduler must not mutate deck state.
- Scheduler must not emit performance events.
- Scheduler must not treat queued/deferred/requested as scan phase or deck state.
- Scheduler work completion may update readiness-owned rows or artifacts through the
  owning authority.

### Deck Runtime Authority

Owns:

- Deck slots and deck identity
- Loaded deck item instances
- Transport state (play, pause, stop)
- Playhead position
- Tempo, pitch, key adjustment
- Cue points, loop in/out, loop state
- Sync participation state
- Runtime resource bindings (decoder handles, audio graph nodes, buffer references)
- Runtime invalidation (resource loss, decode failure, buffer underrun)

Does not own:

- Library items, roles, origins
- Source file inventory
- Probe results or analysis artifacts (reads them, does not write them)
- Readiness rows (reads them, does not write them)
- Broadcast projection
- Scheduler work items

Rules:

- Creates `loaded_deck_item` from a `deck_load_request` only after readiness policy
  allows it.
- References library identity by `library_item_id`. Does not copy or mutate library
  truth.
- Runtime settings (tempo, pitch, cues, loops) may diverge from library metadata.
- Runtime cue/loop/playhead/tempo state belongs to deck runtime, not library item.
- If the source disappears after load, deck runtime decides whether the loaded resource
  remains playable, degraded, blocked, or invalidated based on resource ownership and
  buffering state.
- Library source unavailability does not automatically rewrite historical performance
  events.
- Resource handles are issued by the resource or media authority at the runtime resource
  boundary. Deck Runtime owns bindings to handles, not the durable media artifacts or
  source access policy that produced them. Deck Runtime must not become file access
  authority, decoder authority, or resource policy authority as a side effect of owning
  handle bindings.
- The exact schema, persistence strategy, and Rust types are future work.

### Performance Session Authority

Owns:

- Performance timeline and event log
- Persisted or replayable events: load, unload, play, cue, seek, loop, transition, mix,
  route progress, deck handoff
- Session identity and session lifecycle

Does not own:

- Source scanning or source file inventory
- Media classification or role assignment
- Deck transport internals (references deck state; does not own playhead, cue, or tempo)
- Library item identity
- Broadcast projection

Rules:

- May reference `library_item_id` and `loaded_deck_item_id` in events.
- If a `performance_session_event` references a loaded deck item, the referenced identity
  must be stable within the performance session and event log. Volatile runtime handles
  must not be used as durable event identity.
- Performance events may capture event-time display or metadata snapshots, or
  projection-version references. Such snapshots are historical evidence and are not
  library authority. Library metadata changing after the fact must not alter historical
  event records.
- Events are durable or replayable facts about what happened at runtime.
- Performance session does not own the runtime state that produced the events.
- A load event records that a deck load occurred; it does not perform the load.

### Broadcast Projection Authority

Owns:

- Publishable external/session projection
- Policy for what facts are published externally

Consumes:

- Performance session events
- Deck runtime projection (current loaded items, deck state summaries)
- Privacy-filtered, versioned library item projection (title, artist, album, artwork,
  metadata); not mutable library tables directly

Does not own:

- Library item identity
- Deck runtime state
- Source file inventory
- Readiness rows
- Performance session raw event storage

Rules:

- Broadcast projection is projection, not authority.
- It does not own deck transport.
- It does not own library item identity.
- It does not infer track identity from filenames.
- It consumes privacy-filtered library item projection or event-time metadata snapshots,
  not mutable library tables directly. This is required because broadcast has privacy and
  publication policy that library tables do not enforce.
- It may publish:
  - current loaded item
  - deck state summary
  - transition/mix state
  - session timeline excerpt
  - artwork/metadata projection
  - readiness/degraded state when relevant
- It must not publish raw source paths unless an explicit privacy policy allows it.
- It subscribes to projections, not to raw source file rows.

### Renderer

Owns:

- Visual presentation of projection results

May:

- Request deck load (via control plane)
- Show readiness state
- Present deck state (via deck runtime projection)
- Present broadcast/session projection (via broadcast projection)

Does not own:

- Library substrate tables or direct queries
- Deck state mutation
- Readiness evaluation
- Scheduler work creation
- Broadcast policy
- Performance session event creation

Rules:

- Does not infer readiness from `source_files.media_class`.
- Does not mutate deck state directly.
- Does not coordinate library-to-deck transition itself.
- Does not construct fallback rows from raw filesystem inventory.
- Remains requester and presenter, not authority.

---

## Load Transition

### Flow

```
1. User or controller requests deck load for a library_item_id and target deck.
2. Control plane validates request shape.
3. Readiness Authority returns ready, pending, blocked, unavailable, or unsupported
   for the target.
4. If ready, Deck Runtime Authority creates or replaces a loaded_deck_item binding.
   If pending, control plane enqueues readiness work via scheduler and returns pending.
   If blocked, returns blocked + reason to UI. No work created.
   If unavailable, returns unavailable to UI.
   If unsupported, returns unsupported (target type not applicable).
5. Deck Runtime Authority obtains or receives resource handles needed for runtime use.
6. Performance Session Authority records load event.
7. Broadcast Projection Authority may project the load event and current deck/session
   state using privacy-filtered projection.
```

### Distinctions

- `readiness_target` answers whether load may proceed.
- `deck_load_request` asks for a runtime binding.
- `loaded_deck_item` is runtime state, not durable library truth.
- `performance_session_event` records what happened.
- `broadcast_projection` publishes selected facts externally.

### Rules

- Deck load never performs probe, analysis, or readiness evaluation inline.
- Deck load never creates `work_items` rows directly.
- Deck load never waits unboundedly for work to complete.
- The query path (synchronous readiness check) and request path (via control plane)
  defined in `work-scheduling.md` remain canonical for the library-to-readiness portion
  of the load transition. This document adds the deck-runtime-side boundary.

---

## Loaded Deck Item

A loaded deck item is runtime-owned state. It is not durable library truth.

Conceptual shape (exact schema is future work):

```
loaded_deck_item_id       runtime-scoped identity
deck_id or deck_slot_id   which deck holds this binding
library_item_id           reference to library item (foreign authority)
readiness_proof_ref       reference to readiness target/proof or readiness epoch
source_lineage            which source, file identity, and probe result produced this item
resource_handles          audio/video/visual handles as needed by the runtime target
loaded_at                 when the binding was created
runtime_epoch             generation counter for this binding
invalidation_state        current runtime validity
invalidation_reason       reason if invalidated
```

Rules:

- A loaded deck item references a library item; it is not the library item.
- Runtime settings may diverge from library metadata.
- Runtime cue/loop/playhead/tempo state belongs to deck runtime, not library item.
- If the source disappears after load, deck runtime decides whether the loaded resource
  remains playable, degraded, blocked, or invalidated based on resource ownership and
  buffering state.
- Library source unavailability does not automatically rewrite historical performance
  events.
- The exact schema, persistence strategy, and Rust types are future work.

---

## Readiness Targets

Readiness targets must be explicit and target-specific.

A readiness target has one canonical domain identity. Durable storage uses the canonical
target key. Boundary or protocol naming may transform representation style, but must not
create a second target identity.

Canonical targets:

```
audio_deck_load
video_deck_load
visual_output
artwork_attachment
waveform_preview
broadcast_metadata_projection
```

Protocol representation examples (same semantic targets, not separate identities):

```
audioDeckLoad        is the protocol representation of audio_deck_load
videoDeckLoad        is the protocol representation of video_deck_load
visualOutput         is the protocol representation of visual_output
```

### Readiness Target Taxonomy

Readiness targets belong to target families. The `readiness_target` field is not a
junk drawer. Every target belongs to one of:

```
runtime_load
  audio_deck_load
  video_deck_load

output_projection
  visual_output
  broadcast_metadata_projection

attachment_projection
  artwork_attachment
  waveform_preview
```

Analysis jobs such as `waveform_analysis` or `beatgrid_analysis` produce evidence and
artifacts. They are not automatically readiness targets. `waveform_analysis` is a work
domain; `waveform_preview` is a consumption target derived from that work. These are
distinct: an analysis job completing does not make `waveform_preview` ready without a
readiness evaluation step that consumes the artifact.

This distinction matters for scheduler design. Analysis jobs and readiness targets are
related but not the same thing.

### Naming Alignment

This document and `media-role-classification.md` agree on canonical readiness target keys:

```
audio_deck_load
video_deck_load
visual_output
artwork_attachment
waveform_preview
broadcast_metadata_projection
```

Analysis work kinds such as `waveform_analysis` and `beatgrid_analysis` are analysis
work domains or artifacts, not readiness targets. Analysis completion may produce
evidence, but readiness still requires an explicit readiness evaluation step.

### Rules

- Readiness is target-specific. A music video may be `audio_deck_load: ready` and
  `video_deck_load: ready` simultaneously. An audio-only track may be
  `audio_deck_load: ready` and `video_deck_load: unsupported`.
- Global `deckReady` is rejected. Deck loading needs an explicit target such as
  `audio_deck_load` or `video_deck_load`. Do not overload `audio_deck`,
  `audioDeckLoad`, and `deckReady` as competing concepts.
- Readiness may depend on role, probe facts, source availability, resource availability,
  analysis artifacts, and target policy.
- Readiness answers must be bounded. Expensive work is enqueued, not performed inline.
- Readiness must not create loaded deck items.
- Readiness row shape is owned by the readiness authority. Adding new targets does not
  make readiness own deck state.

---

## Scheduler Interaction

The scheduler owns when work runs. Deck state, performance events, and broadcast
projection are not scheduler concerns.

Rules:

- A deck load can create foreground interest or an explicit readiness request.
- Scheduler may promote readiness/probe/hash/resource work in response.
- Scheduler must not mutate deck state.
- Scheduler must not emit performance events.
- Scheduler must not treat queued/deferred/requested as scan phase or deck state.
- Scheduler work completion may update readiness-owned rows or artifacts through the
  owning authority.
- Foreground interest expires. A deck load request creates transient urgency, not
  permanent scheduler priority. See `work-scheduling.md` for foreground interest TTL.

---

## Source Loss After Load

Source availability belongs to the source access authority. Loaded resource validity
belongs to the deck runtime authority.

Possible loaded deck item runtime states:

```
playable            resource is usable
degraded            resource is usable with known limitations (e.g. source lost but
                    buffer holds)
resourceUnavailable source is gone and buffer is insufficient
invalidated         binding is no longer valid (source replaced, identity changed)
unloaded            binding was explicitly removed by deck authority
```

Rules:

- Source authority reports source availability. Library source unavailability does not
  directly mutate deck runtime.
- Source availability changes invalidate or degrade readiness answers for affected targets
  according to target policy. A loaded or buffered item may remain playable while future
  loads of the same item become unavailable. Readiness invalidation is not always total;
  the appropriate response depends on the target and whether the resource is buffered.
- Deck runtime detects resource loss through its own handles. It does not poll
  `source_files.availability`.
- Historical performance events remain historical facts. A source that disappears does not
  erase the fact that it was loaded and played.
- Broadcast projection must surface degraded/unavailable state when relevant, but must not
  rewrite the past. Broadcast does not publish raw source mutation as performance
  authority.
- Exact runtime state machine, transition rules, and invalidation protocol are future work.

---

## Broadcast Projection Boundary

Broadcast is projection, not authority. It consumes performance/session/deck/library
projections and publishes selected external facts.

Rules:

- Broadcast projection does not own deck transport.
- Broadcast projection does not own library item identity.
- Broadcast projection does not infer track identity from filenames.
- Broadcast projection must not publish raw source paths unless an explicit privacy policy
  allows it.
- Broadcast projection consumes privacy-filtered, versioned library item projection or
  event-time metadata snapshots. It does not consume mutable library tables directly.
  Library metadata that changes after a broadcast event must not alter the published
  history of that event.
- Broadcast projection subscribes to projections, not to raw `source_files` or
  `library_items` rows.
- The exact schema, protocol, and projection pipeline are future work.
- Broadcast policy (what is publishable, under what conditions, to which destinations) is
  a separate product decision. This document defines only the ownership boundary.

---

## Rejection Cases

The following designs are invalid and must be rejected in code review:

```
library_item carries playhead, deck slot, cue state, or loaded runtime status
source_files.media_class is used as deck-load eligibility or readiness proof
source_files.availability_state is treated as deck or runtime availability
readiness writes deck state
scheduler writes deck state
renderer loads deck by mutating store rows
broadcast subscribes directly to source_files or library_items as performance authority
broadcast consumes mutable library tables directly without privacy-filtered projection
performance session events depend on mutable filename/path as primary identity
performance_session_event uses a volatile runtime handle as durable event identity
deck load performs probe/analysis inline
global deckReady replaces target-specific readiness
queued/deferred/requested are added to source scan phase
library_items carries a deck_id or deck_slot_id column
item_readiness includes transport state, playhead, or tempo
work_items owns deck runtime state, stores loaded deck state, or treats a deck_id column
  as deck authority
source_files has a loaded_in_deck flag
loaded_deck_item is stored as a library item row or owned by any library authority
broadcast projection writes to library_items or item_readiness
renderer creates a loaded_deck_item by calling a library mutation
performance_session_event references source_files.source_file_id as primary identity
source access state, mount state, or source availability is owned by Library Item Authority
scan inventory facts or presence_state are owned by Source Access Authority
role decisions are owned by Scanner/Inventory Authority or Library Item Authority
deck runtime owns source access policy or becomes file access authority through handle
  binding
analysis job completion (waveform_analysis, beatgrid_analysis) is treated as automatic
  readiness target fulfillment without a readiness evaluation step
readiness_target keys disagree between this document and media-role-classification.md
```

A future scheduler, admission, or request row may reference a deck-load request,
foreground-interest source, or target deck if the reference is explicitly
non-authoritative and does not mutate deck state.

---

## What This Document Does Not Decide

This document defers to future work:

- Exact deck runtime schema and persistence strategy
- Audio engine architecture and graph topology
- Real-time hot lane layout and scheduling
- Controller mapping and hardware integration
- Streaming platform integration
- Broadcast monetization and product policy
- Waveform/probe implementation details
- Scheduler table implementation details
- Final deck UI and workspace surface assignments
- Performance session event schema and storage
- Broadcast projection protocol and wire format


---

## Alignment With Existing Decisions

This document extends, not replaces, the following:

- `media-role-classification.md`: defines library item identity, roles, classification
  pipeline, and readiness as target-specific usability. This document adds the deck-side
  boundary that consumption is not mutation. Both documents agree on canonical readiness
  target keys; see Naming Alignment in Readiness Targets above.

- `work-scheduling.md`: defines deck load semantics (query path + request path), work
  lanes, scheduler ownership. This document adds the deck runtime authority that receives
  the load after readiness approves it.

- `source-access-and-scan-coverage.md`: defines source access authority. This document
  adds the rule that source unavailability after load is a deck runtime decision, not a
  library mutation, and that source availability changes invalidate or degrade readiness
  answers according to target policy rather than always fully invalidating.

- `recursive-selected-contents-rule.md`: defines how tree selection projects recursive
  primary media. This document adds the rule that contents projection does not load decks
  and does not own performance session scope.

- `source-locations-lifecycle-contract.md`: defines source location lifecycle. This
  document adds no new requirements on source locations.

---

## Acceptance Bar for Future Work

Any future readiness, scheduler, deck, performance session, or broadcast implementation
must prove:

- Library substrate authority is split: source access, scanner/inventory, library item
  identity, and role classification are separate owners with separate rejection cases
- Library authority remains separate from deck runtime authority
- Readiness is target-specific and does not load decks
- Readiness targets belong to a named target family; analysis jobs are not readiness
  targets
- Readiness target keys are consistent with `media-role-classification.md`
- Deck load creates runtime binding, not library mutation
- Resource handles are issued at the runtime resource boundary; deck runtime owns
  bindings, not source access policy
- Performance session records runtime events using stable session-scoped identity;
  volatile runtime handles are not used as durable event identity
- Performance event-time metadata snapshots are historical evidence, not library authority
- Broadcast projection reads privacy-filtered, versioned projections or event-time
  snapshots, not raw library tables
- Source availability changes invalidate or degrade readiness answers according to target
  policy; readiness invalidation is not always total
- No stored visibility flag, deck state column, or loaded flag exists on library items or
  source files
- Renderer remains requester/presenter, not authority
- Scheduler does not write to output tables owned by other authorities
