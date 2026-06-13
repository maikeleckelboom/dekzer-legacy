# Library Workstation Design Doctrine

Status: canonical design doctrine for Library Workstation V0
Scope: Dekzer desktop library workstation, source admission, source-aware browsing, scope/projection/view control, contents, inspector, preview lane, and future Hot Table boundary
Non-goal: this document does not define deck DSP, mixer behavior, waveform rendering internals, external adapter schemas, or final Prepared Room UX

## Purpose

Dekzer is only as reliable as its library substrate. The Library Workstation is the first surface that proves that reliability to the user.

The workstation must make library state legible without collapsing different concepts into one generic browser. A DJ must always be able to answer:

- where the music came from
- what material boundary is active
- how that material is being interpreted
- how the result is being rendered
- what object is selected
- what can be inspected or acted on
- what is only being previewed
- what is loaded or armed for performance
- what layout region is being edited

The Library Workstation is not a deck UI with a library panel attached. It is a source-health-aware, scope-driven, projection-rendered music substrate interface.

## Core product law

The Library Workstation must not lie about ownership, provenance, readiness, or object type.

A source is not a track.
A source file is not a playable media object.
A folder is not a source unless it has been admitted as a source origin.
A playlist is not a source.
A crate is not a source.
A smart list is not a source.
A browse root is not automatically a source.
An active scope is not automatically a selected source row.
A preview slot is not a loaded deck.
A workspace region is not library state.

Every surface must declare which concept it represents.

## Information architecture chain

The canonical chain is:

```text
source origins
→ browse roots
→ active scope
→ projection
→ view mode
→ contents
→ primary selection
→ inspector/action target
→ preview slot / loaded slot
→ workspace region
```

This chain is the design review checklist. Any proposed UI element must fit one of these concepts or be rejected until its role is clear.

## Canonical concepts

| Concept                 | Meaning                                                                                                            | Owns                                                                                    | Does not own                                                 |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Source origin           | A provenance-bearing origin known to Dekzer                                                                        | origin identity, source type, availability, health, admission state                     | browse taxonomy, playlists, track identity                   |
| Browse root             | A user entry point into library material or workflow objects                                                       | activation intent, default scope/projection/filter preset                               | provenance authority                                         |
| Active scope            | The material boundary currently being queried                                                                      | included sources/material set, scope identity, scope hierarchy level                    | row rendering, inspector context                             |
| Projection              | The structural interpretation applied over the active scope                                                        | result family such as tracks, artists/albums, source inventory, collection objects      | source origin identity, view layout, health filter state     |
| Collection object       | A playlist, crate, smart list, or collection group row exposed by the `Collection Objects` projection              | object identity, collection type, membership query contract, ordering/grouping metadata | source provenance, source health, track identity             |
| History object          | A history period, session, performance log, or performed-set reference exposed by the `History Objects` projection | event/session identity, time range, ordering, history membership contract               | live slot runtime, source provenance, current playback state |
| View mode               | The renderer form for a projection                                                                                 | list, columns, tree, covers, grid, compact density                                      | membership semantics                                         |
| Contents                | The result field for active scope, projection, filters, search, sort, and cursor                                   | rows, retained rows, pending rows, empty state display                                  | durable membership authority                                 |
| Primary selection       | The selected inspectable object                                                                                    | inspector context trigger, action target                                                | active scope authority                                       |
| Inspector/action target | Details and actions for primary selection                                                                          | context-specific metadata, readiness, actions                                           | active scope selection                                       |
| Preview slot            | The collapsed evaluative slot lane for pre-load work                                                               | audition/evaluation state, preview waveform, targetable load actions                    | deck runtime ownership                                       |
| Loaded slot             | A committed performance slot/deck/hot-table item                                                                   | loaded/armed/playing state, runtime slot identity                                       | source provenance                                            |
| Workspace region        | A layout/topology region selected in edit mode                                                                     | pane role, constraints, parking, placement                                              | library membership, source state                             |

## Source origins

A source origin is an entity that explains provenance and availability. It answers: where does this material come from, and can Dekzer currently rely on it?

Examples:

```text
Local source
  Music
  DJ Pool
  Samples
  External SSD

External library adapter
  rekordbox Library
  Traktor Collection
  Serato Crates
  VirtualDJ Database
  Engine Library

Cloud or sync adapter
  Dropbox DJ Library
  Google Drive Sets

Device or volume
  USB drive
  SD card
  standalone hardware library
```

A source origin can be ready, scanning, incomplete, offline, missing, blocked, failed, or hidden after removal. These states are source state. They must not be represented as fake browse roots or fake track rows.

### Source panel responsibility

The Sources panel shows provenance-bearing origins and source groups. It also exposes admission actions.

Canonical structure:

```text
SOURCES

  Local Sources
    Music                         ● Ready
    DJ Pool                       ● Ready
    External SSD                  ⚠ Offline

  External Libraries
    rekordbox Library             ● Ready
    Traktor Collection            ● Ready

  Cloud / Sync
    Dropbox DJ Library            ⚠ Needs attention

  Devices
    USB Drive                     ● Ready

  + Add Source

SOURCE HEALTH
  1 source offline
```

Rules:

- `+ Add Source` is an action, not a source row.
- Unadmitted folders shown during admission are candidates, not sources.
- A registered source origin may remain visible when offline, missing, blocked, or failed.
- Source health belongs next to source origins and in source-health summaries.
- Playlists, crates, smart lists, history, genres, artists, keys, BPM, and missing-file predicates do not belong under Sources unless they are being shown as children of an explicitly selected external library adapter with preserved provenance.
- Grouping or labeling sources by offline status in the Sources panel is allowed when it describes source availability. Showing a static `Offline Sources` node as if it were a provenance-bearing source origin is not allowed.

## Browse roots

Browse roots are entry points into material or workflow objects. They answer: how does the user want to enter the library?

Examples:

```text
All Music
Recently Added
Recently Played
Top Rated
Unplayed
Not Analysed
Missing Files
Playlists
Crates
Smart Lists
History
Prepared Rooms (post-V0, reserved)
Performed Sets (post-V0, reserved)
```

Browse roots are not source origins. Activating a browse root produces an explicit browse activation. That activation may set scope, projection, health filters, sort, or a workflow-object selection. It must not rely on implicit UI convention.

### Browse root activation contract

The first implementation must treat these as the canonical V0 browse roots:

| Browse root        | Scope effect                                                                                                                        | Projection effect    | Filter/sort effect                 | Notes                                                                                                                                                                         |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `All Music`        | set `All Admitted Music`                                                                                                            | `Tracks`             | clear health filters, default sort | UI label may be `All Music`; doctrine name is `All Admitted Music`.                                                                                                           |
| `Recently Added`   | preserve current compatible musical scope; if none or current scope is inventory-only, use `All Admitted Music`                     | `Tracks`             | sort or filter by added date       | This is a browse preset, not a source.                                                                                                                                        |
| `Recently Played`  | preserve current compatible musical scope; if none or current scope is inventory-only, use `All Admitted Music`                     | `Tracks`             | sort or filter by last-played date | This is a browse preset.                                                                                                                                                      |
| `Top Rated`        | preserve current compatible musical scope; if none or current scope is inventory-only, use `All Admitted Music`                     | `Tracks`             | rating filter or rating sort       | This is a browse preset.                                                                                                                                                      |
| `Unplayed`         | preserve current compatible musical scope; if none or current scope is inventory-only, use `All Admitted Music`                     | `Tracks`             | play-count filter                  | This is a browse preset.                                                                                                                                                      |
| `Not Analysed`     | preserve current compatible musical scope; if none or current scope is inventory-only, use `All Admitted Music`                     | `Tracks`             | analysis-state filter              | This is a health/preparation preset, not a structural projection.                                                                                                             |
| `Missing Files`    | preserve current compatible musical scope; if none or current scope is inventory-only, use `All Admitted Music`                     | `Tracks`             | missing/offline health filter      | Canonical track-facing browse preset. A diagnostic Source Inventory path may be exposed as an explicit secondary action or mode toggle, not as an implicit projection switch. |
| `Playlists`        | no concrete media scope until a playlist is selected                                                                                | `Collection Objects` | none                               | Selecting a playlist sets scope to that playlist membership and projection to `Tracks`.                                                                                       |
| `Crates`           | no concrete media scope until a crate is selected                                                                                   | `Collection Objects` | none                               | Selecting a crate sets scope to that crate membership and projection to `Tracks`.                                                                                             |
| `Smart Lists`      | no concrete media scope until a smart list is selected                                                                              | `Collection Objects` | none                               | Selecting a smart list sets scope to evaluated smart-list membership and projection to `Tracks`.                                                                              |
| `History`          | no concrete media scope until a history period/set is selected                                                                      | `History Objects`    | default chronological sort         | Selecting a history object sets scope to that event/set membership and projection to `Tracks`.                                                                                |
| `Source Inventory` | requires an exact source or source group; if no compatible source is active, open an explicit source-selection or admission chooser | `Source Inventory`   | inventory filters                  | Diagnostic, not default musical browse. It must not silently choose a source.                                                                                                 |

The table is authoritative for V0. `Prepared Rooms` and `Performed Sets` are reserved post-V0 browse roots. They must not appear as V0 activation targets until their workflow-object contracts define the same four fields: scope effect, projection effect, filter/sort effect, and notes. Future browse roots must declare those fields before implementation.

A compatible musical scope is any active scope that can answer a `Tracks` projection over playable media. Source Inventory-only contexts, candidate admission contexts, and zero-source state are not compatible musical scopes; browse presets that preserve scope must fall back to `All Admitted Music` or to the nearest exact admitted source when such a fallback is explicitly chosen by the UI.

### Browse root examples

```text
Click All Music
→ SCOPE: All Music
→ PROJECTION: Tracks
→ VIEW: last compatible view or List
→ filters: cleared unless user pins them
```

```text
Click Missing Files while SCOPE is Music
→ SCOPE: Music
→ PROJECTION: Tracks
→ VIEW: current compatible view
→ filters: Missing
```

```text
Click Playlists
→ SCOPE: unchanged or no concrete media scope
→ PROJECTION: Collection Objects
→ VIEW: List or Columns
→ contents: playlist objects
```

```text
Click playlist Main Set
→ SCOPE: Playlist: Main Set
→ PROJECTION: Tracks
→ VIEW: List or Columns
→ contents: tracks in Main Set
```

A browse root can create a query. It does not become provenance.

## Active scope

Active scope is the material boundary currently queried by contents and projections. It is not a visual label only. It must be modeled explicitly.

Scope levels:

| Scope level            | Example UI label    | Doctrine meaning                                                                                                                        |
| ---------------------- | ------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Single source          | `Music`             | one source origin                                                                                                                       |
| Source group           | `All Local Sources` | all source origins in a source group                                                                                                    |
| Admitted aggregate     | `All Music`         | all music admitted into Dekzer's library substrate                                                                                      |
| Cross-origin aggregate | `All Sources`       | all known origins allowed by the current cross-origin policy, including adapters, devices, cloud, or unavailable origins when requested |

The user-facing label can stay simple. The model must remain precise.

`All Music` is acceptable as UI copy when the doctrine name is `All Admitted Music`. The model must not forget whether it means native admitted material, a local-source group, or every known origin including cloud/adapters.

### Scope dropdown hierarchy

The SCOPE control should expose the hierarchy. It must not be a passive label.

Example:

```text
SCOPE

  Current source
    Music
    DJ Pool
    External SSD

  Source groups
    All Local Sources
    All External Libraries
    All Cloud Sources
    All Devices

  Library aggregates
    All Music

  Cross-origin
    All Sources
```

The structural groupings remain present even when some entries are unavailable. Individual entries may be disabled, annotated, or hidden only when the product explicitly decides that hiding is less confusing than showing an unavailable item. Group headings must remain stable so the user learns the scope hierarchy.

### Zero-source state

The workstation must handle first-run and post-removal states explicitly.

When there are zero admitted sources:

```text
SOURCES
  + Add Source

SCOPE
  No sources

PROJECTION
  Tracks, disabled or placeholder

CONTENTS
  Zero-source add-source state.
```

Rules:

- Do not show `No tracks` for zero-source state.
- Do not default to a fake `All Music` scope when there is no admitted material.
- Do not show a selected source row.
- Primary selection is empty unless the user selects the `+ Add Source` action or an admission candidate.
- Source Inspector is not shown for a nonexistent source.
- The primary action is source admission.

## Sources to scope sync contract

The Sources panel and the SCOPE control are synchronized views over related state. They are not independent selectors.

Primary direction:

```text
Sources selection → active scope
```

Secondary direction:

```text
SCOPE override → Sources visual state
```

### Exact-source behavior

When the user clicks an individual source row:

```text
Click Sources > Local Sources > Music
→ active scope = source: Music
→ Sources row Music is selected
→ SCOPE shows Music
→ primary selection = source: Music
→ inspector context = SOURCE
→ contents query uses source: Music
```

When the user selects an exact source from SCOPE:

```text
Select SCOPE > Music
→ active scope = source: Music
→ Sources reveals and selects Music
→ primary selection = source: Music unless a retained track selection remains explicitly active
→ contents query uses source: Music
```

A source row is selected only when active scope is exactly that source, or when the user explicitly selects that source for inspection.

### Source-group behavior

When the user clicks a source group that is interactive:

```text
Click Sources > Local Sources
→ active scope = group: All Local Sources
→ Sources group Local Sources is active
→ no child source row is selected
→ SCOPE shows All Local Sources
→ primary selection = source group: Local Sources
→ inspector context = SOURCE
→ contents query uses All Local Sources
```

A source group may use the Source Inspector family, but the header must disclose the group nature, for example:

```text
SOURCE
Local Sources
3 ready, 1 offline
```

### Aggregate-scope behavior

When the user changes SCOPE to an aggregate that does not map to one source row:

```text
Select SCOPE > All Music
→ active scope = aggregate: All Admitted Music
→ no individual source row is selected
→ Sources may show an aggregate active marker
→ contents query uses All Admitted Music
```

```text
Select SCOPE > All Sources
→ active scope = aggregate: All Sources
→ no individual source row is selected
→ Sources may show a global aggregate active marker
→ contents query uses All Sources
```

The Sources panel may highlight containing groups, show active-scope badges, or display a subtle aggregate state. It must not fake selection of `Local Library`, `Music`, or any other single source when the active scope is broader than that source.

### Selection retention during scope changes

Active scope and primary selection are separate state axes.

When scope changes:

- retain a selected track only if it remains valid for the new query
- clear a selected track if it is outside the new active scope or excluded by the new projection/filters
- retain selected source only if the new active scope is exactly that source or that source remains explicitly selected for inspection
- clear selected source when the scope becomes an aggregate that does not correspond to that source
- do not fabricate a selected source for aggregate scopes
- do not switch inspector context because of hover or focus movement

A row identity is valid for a given query when the row belongs to the active scope, satisfies the active projection, and is not excluded by active filters or search. Otherwise it is invalid and primary selection must be cleared.

Control-bar changes are controlling state changes. They do not automatically create a fake inspectable object.

## Projection

Projection is the structural interpretation applied to the active scope. It answers: what kind of result family is being requested from this material boundary?

Canonical V0 projection families:

```text
Tracks
Artists / Albums
Albums
Genres
Keys
BPM Ranges
Collection Objects
History Objects
Source Inventory
```

`Collection Objects` returns collection/workflow containers such as playlists, crates, smart lists, and collection groups. Selecting one of those objects creates a concrete membership scope and usually switches projection to `Tracks`.

`History Objects` returns history containers such as history periods, sessions, performance logs, or performed-set references. Selecting one of those objects creates a concrete historical membership scope and usually switches projection to `Tracks` with chronological ordering.

Not every browse root is a projection. `Recently Added`, `Top Rated`, `Unplayed`, `Not Analysed`, and `Missing Files` are browse presets or health filters over a projection, usually `Tracks`. They must not be implemented as separate projection families unless a future contract gives them distinct result shape and semantics.

Projection is not view mode. `Tracks` can be rendered as list, columns, covers, or another view. `Genres` can be rendered as columns or tree. `Source Inventory` can be rendered as a diagnostic tree or list, but it remains source inventory, not musical browse.

Projection must be visible in the control bar:

```text
SCOPE: All Music
PROJECTION: Tracks
VIEW: Columns
```

or:

```text
SCOPE: Music
PROJECTION: Tracks
FILTER: Missing
VIEW: List
```

or:

```text
SCOPE: External SSD
PROJECTION: Source Inventory
VIEW: Tree
```

### Projection rules

- Projection does not change provenance.
- Projection does not admit sources.
- Projection does not own row readiness.
- Projection must be part of contents query identity.
- Projection changes may clear or retain primary selection according to row identity validity.
- Projection-specific columns and facets are allowed, but they must not change source identity.
- Browse presets may set projection and filters together, but implementation must store those effects explicitly.

## View mode

View mode is the renderer form. It answers: how is the projection being displayed?

Examples:

```text
List
Columns
Tree
Covers
Grid
Compact
```

Rules:

- View mode must not change active scope.
- View mode must not change source provenance.
- View mode must not change track membership.
- View mode may change navigation affordances, row grouping, visible columns, and density.
- View mode may require different read models only when the projection contract declares it.
- Switching view mode should retain active scope, projection, search, filters, and selected row when possible.

Column view is first-class. It is not decoration. It exists because Dekzer's model needs to express source scope, browse roots, facets, and final contents without pretending they are one tree.

## Column view navigator contract

Column view is a contents-area renderer. It is not the Sources panel.

Column 1 is the active scope/projection navigator. It shows the first navigable dimension for the current active scope and projection. Its rows are not automatically source origins, browse roots, or files. They are navigation choices declared by the active projection.

Column 1 may show different row families depending on the active projection:

| Active projection    | Column 1 represents                                                                           | Selecting Column 1 row                             |
| -------------------- | --------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `Tracks`             | track rows for the active scope/query                                                         | selects a track row                                |
| `Artists / Albums`   | artists, or source origins first when scope is aggregate and source-first columns are enabled | narrows column path to artist or source            |
| `Albums`             | albums for the active scope/query                                                             | narrows column path to album, then track rows      |
| `Genres`             | genres                                                                                        | narrows column path to genre                       |
| `Keys`               | musical keys                                                                                  | narrows column path to key                         |
| `BPM Ranges`         | BPM buckets                                                                                   | narrows column path to BPM bucket, then track rows |
| `Collection Objects` | playlists, crates, smart lists, or collection groups                                          | selects object or narrows to object group          |
| `History Objects`    | history periods, sessions, or performed-set references                                        | selects history object or narrows path             |
| `Source Inventory`   | source directories or adapter inventory roots                                                 | narrows literal inventory path                     |

Pinned facets or browse presets may be introduced later, but they are not the default V0 `Tracks` column behavior. If they are added, the projection contract must declare whether they apply a filter, change projection, or narrow a column path.

Rules:

- Column 1 is inside the contents area, not under Sources.
- Column 1 rows may change projection, apply a filter preset, or narrow a column path only when the projection contract declares that behavior.
- Column 1 must not silently admit a source.
- Column 1 must not turn a browse root into provenance.
- Column 1 must not treat health presets as source origins.
- Selecting a final media row in any column path sets primary selection to TRACK.
- Selecting a source-origin row inside a source-first column path may set primary selection to SOURCE, but it must not replace the Sources panel contract.

Example musical path:

```text
SCOPE: All Local Sources
PROJECTION: Artists / Albums
VIEW: Columns

Column 1: Source origins
Column 2: Artists
Column 3: Albums
Column 4: Tracks
```

Example aggregate artist path:

```text
SCOPE: All Music
PROJECTION: Artists / Albums
VIEW: Columns

Column 1: Artists
Column 2: Albums
Column 3: Tracks
```

Example health path:

```text
SCOPE: All Music
PROJECTION: Tracks
FILTER: Missing
VIEW: Columns

Column 1: Affected source origins
Column 2: Affected folders or groups
Column 3: Affected tracks
```

`Health roots` is not a canonical concept. Use `health preset`, `health filter`, or `affected source origins` depending on the actual row family.

## Contents

Contents is the primary result field. It answers: what rows are inside the current active scope, projection, filters, search, sort, and cursor?

Contents is not a source panel. Contents does not own provenance. Contents displays provenance columns and readiness states supplied by substrate read models.

Contents query identity must include at minimum:

```text
active scope
projection
search text
filters
sort
cursor identity
readiness/coverage identity when relevant
```

View mode may be included when the view requests a different row shape, but view mode must not silently change membership semantics.

### Retained rows lifecycle

Retained rows are rows held in renderer state during a pending query transition to preserve visual continuity. They are not query results for the new read until the substrate response confirms them.

Rules:

- Retained rows are created only when an existing contents result remains visible while a replacement query is pending.
- Retained rows are cleared when the new query result arrives.
- Retained rows are cleared immediately when the new active scope makes the row identity invalid.
- Retained rows must not participate in substrate reads.
- Retained rows must not prove membership.
- Retained rows must not keep primary selection alive if row identity validity fails for the new query.
- Retained rows may display a pending or stale visual treatment if needed.

### Contents health filters

Health filters belong in the contents header. They are predicates over the active query, not primary navigation and not sources.

Canonical compact set:

```text
All
Ready
Preparing
Offline
Missing
Blocked
Not Analysed
```

The visible set may be reduced in V0:

```text
All
Ready
Offline
Missing
```

But the model must remain expandable.

`Preparing` is a row-level readiness filter for playable media that is known to the library substrate but not yet fully ready because scan, hashing, probing, analysis, evidence evaluation, or promotion is still pending. Source-level scanning remains source or scope health; it must not be collapsed into a row-level `Preparing` filter unless concrete affected media rows exist.

Rules:

- Health tabs filter contents rows inside the current active scope and projection.
- Health tabs must not appear under Sources.
- Health tabs must not rewrite active scope.
- Active health filters must be visible, including filter count badges.
- Empty results must disclose whether the empty result is authoritative.

### Empty states

An empty contents result must not say only `No tracks`.

It must distinguish at least:

```text
No tracks in this scope
Still indexing
Source offline
Source missing
Source blocked
Scan failed
Coverage incomplete
Filtered out by active filters
Projection has no matching rows
No audio tracks found in this view
Zero-source add-source state
```

The zero-source add-source state is rendered through the contents empty-state surface, but it is selected before a normal contents query is attempted. It must use admission-oriented copy such as `Add a source to start building your library`, not `No tracks`.

If coverage is incomplete, the UI must say that. If a source is offline, the UI must say that. If an audio profile excludes video or companion files, the UI must say that.

## Primary selection

Primary selection is the selected inspectable object. It answers: what object are details and actions about?

Primary selection is distinct from active scope, projection, view mode, keyboard focus, hover, and retained rows.

Allowed primary selection object types for Library Workstation V0:

```text
track or playable media row
source origin or source group
slot or preview lane item
workspace region in edit mode
```

Rules:

- Hover never changes primary selection.
- Keyboard focus alone does not change primary selection.
- Control-bar changes do not automatically fabricate primary selection.
- Retained rows may stay visible during pending reads, but a retained row must not remain selected if its identity is invalid for the new query.
- Primary selection must carry enough identity for the inspector to verify it still belongs to the active context.

## Inspector context contract

Primary selection object type determines inspector context.

| Primary selection                          | Inspector context           | Header label |
| ------------------------------------------ | --------------------------- | ------------ |
| Track or playable media row                | Library selection inspector | `TRACK`      |
| Source origin or source group              | Source inspector            | `SOURCE`     |
| Preview slot, hot-table slot, or deck slot | Slot inspector              | `SLOT`       |
| Workspace region in edit mode              | Workspace inspector         | `WORKSPACE`  |

The inspector header must label the current context type explicitly. It must not only say `INSPECTOR`.

Examples:

```text
TRACK
Molecules
Yotto
Ready
```

```text
SOURCE
External SSD
Offline
```

```text
SLOT
Preview Lane
Armed for Deck A
```

```text
WORKSPACE
Contents Pane
Edit Mode
```

### Library selection inspector

Shown when a track or playable media row is the primary selection.

It answers:

```text
Can I trust and evaluate this item before loading it?
```

It should show:

- artwork
- title, artist, album
- source and provenance
- readiness
- file path or origin reference
- format, bitrate, sample rate, channels
- BPM and key
- duration
- cue and loop overview
- waveform overview
- tags
- analysis state
- missing/offline/blocked reasons
- load actions routed through slot dispatch

The waveform overview is an inspector display for evaluating the selected track. It is not the Preview Lane and does not own preview slot state. The Preview Lane's preview waveform is slot-state-owned and may become interactive over time.

### Source inspector

Shown when a source origin or source group is the primary selection.

It answers:

```text
What is the state of this origin or source group?
```

It should show:

- source type
- path, device, adapter, or cloud origin
- admission state
- availability
- health summary
- scan coverage
- media counts
- missing counts
- blocked counts
- unsupported counts
- last scan
- last successful read
- maintenance actions
- removal or hide actions when allowed

A source group may use the Source Inspector family, but it must disclose that it is a group and not one source origin.

### Slot inspector

Shown when a preview slot, hot-table slot, or deck slot is the primary selection.

It answers:

```text
What is loaded, previewed, armed, or playing here?
```

It should show:

- slot identity
- slot role
- loaded or previewed media
- readiness
- target deck or route
- playback/evaluation state
- cue/loop state
- key/BPM lock state when applicable
- load, clear, arm, park, or replace actions when supported by the slot role

### Workspace inspector

Shown only in edit mode when a workspace region is selected.

It answers:

```text
What layout surface am I editing?
```

It should show:

- region identity
- surface role
- visibility
- resize constraints
- parking state
- allowed splits, moves, swaps, or removals
- affected workspace preset

Normal library selection must not enter Workspace Inspector. Edit mode must be explicit.

## Preview lane and Hot Table contract

The Library Workstation V0 bottom strip is not a disposable preview widget. It is the collapsed first expression of the future Hot Table.

Canonical relationship:

```text
Preview lane V0
= single-slot collapsed expression of the Hot Table slot lane

Full Hot Table
= expanded multi-slot expression of the same slot identity model

Perform slot strip
= performance-oriented expression of the same slot identity and dispatch model
```

Rules:

- V0 preview lane must have slot identity.
- V0 preview lane must use the same slot dispatch path that future Hot Table and Perform Workstation use.
- Load actions in the track inspector must route through slot dispatch.
- Previewing a track must not imply that it is loaded into a deck.
- Loading to Deck A/B must not bypass slot dispatch.
- Clearing preview state must not clear loaded deck state.
- Slot selection may switch inspector context to SLOT.
- Track selection may update preview candidate state, but it must not automatically commit a loaded slot.

### Required V0 preview lane behavior

The minimal V0 bottom strip should support:

```text
one preview slot
selected track identity
artwork
title / artist
preview waveform
cue markers when available
readiness badge
load target actions
clear action
slot selection
```

V0 exposes `preview`, `load`, and `clear` actions. `arm` is reserved in the shared dispatch vocabulary but is not exposed in Library Workstation V0 unless a real armed-slot state already exists. Do not create a fake armed state for the sake of UI symmetry.

The lane may visually look simple, but internally it must be modeled as a slot lane.

### Shared dispatch model

Every load path must use the same conceptual dispatch:

```text
source object identity
→ selected or previewed media identity
→ target slot identity
→ load intent
→ readiness checks
→ dispatch result
```

Dispatch intent vocabulary:

```text
preview
load
clear
arm
replace
park
```

Library Workstation may only expose a subset in V0. The dispatch model must not be split into separate library and perform paths.

## Source admission and candidate media evidence

Admission is the process that turns a user-chosen origin into a registered source. It is not the same as browsing an already admitted source.

During admission, Dekzer may show candidate media evidence. This is a preflight read that helps the user decide whether a candidate folder should become a source.

Candidate media evidence is not a source.
Candidate media evidence is not the preview lane.
Candidate media evidence is not a playable slot.
Candidate media evidence is not durable library membership.
Candidate media evidence must be replaced by source-backed reads after admission.

Candidate media evidence is cleared at registration time. Evidence rows are not promoted. If the user registers a source but defers scanning, source-backed views show the registered source with not-scanned, uncovered, or pending state, not stale candidate evidence. Confirmed media rows appear only after source-backed reads produce them.

Rules:

- `+ Add Source` opens admission.
- Local folders shown before registration are candidates.
- Candidate folders must not be labeled as admitted sources.
- Candidate media evidence may show that media exists under a candidate folder.
- Source Inventory may show literal files and folders for diagnostic purposes after admission.
- Musical browse defaults to descendant playable media after admission.
- Literal direct-child inventory is diagnostic, not the default DJ browse mode.

The UI must preserve the difference between:

```text
unadmitted local folder
registered source origin
source directory
source file
attachment
playable media
track identity
```

## Source inventory versus musical browse

Dekzer must support both musical browse and literal inventory, but they are not the same view.

Musical browse answers:

```text
What playable music is inside this scope?
```

Source inventory answers:

```text
What filesystem or adapter objects exist under this source?
```

Default admitted-library browsing uses musical scope. Selecting a source or folder should show media-relevant descendant tracks by default. Literal direct child files belong in Source Inventory or diagnostics.

Rules:

- Do not show folders as empty when descendant playable media exists.
- Do not require enrichment before basic hierarchy is visible.
- During incomplete indexing, show unknown or indexing state rather than hiding branches as empty.
- Do not put tracks inside the source tree.
- Tracks belong in Contents.
- Source Inventory may include non-playable files, sidecars, unsupported files, blocked files, and missing paths.

## Health and readiness placement

Health has multiple levels. Each level must appear in the right place.

| Health level          | Placement                                  | Example                                |
| --------------------- | ------------------------------------------ | -------------------------------------- |
| Source health         | Sources panel and Source Inspector         | External SSD offline                   |
| Scope health          | SCOPE control or contents header summary   | All Local Sources has 1 offline source |
| Row readiness         | Contents table/status column               | track Ready, Offline, Missing          |
| Filtered health query | Contents filter tabs or health browse root | Missing Files                          |
| Maintenance health    | Source Inspector or maintenance panel      | rescan required, evidence backlog      |

Rules:

- Missing Files as a browse root is allowed.
- Missing Files as a source is not allowed.
- Offline as a row status is allowed.
- Grouping or labeling source origins by offline status is allowed when it describes availability.
- A static `Offline Sources` node must not be shown as if it were a provenance-bearing source origin.
- Source health summary must remain available even when contents filters are not open.
- Readiness state must not be hidden inside settings.

## State ownership map

The Library Workstation must avoid duplicated hidden state.

| State                               | Owner                                                      | Notes                                                                       |
| ----------------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------------- |
| Source origins and source lifecycle | library substrate                                          | renderer reads and projects                                                 |
| Source health and scan coverage     | library substrate                                          | renderer may summarize but not invent                                       |
| Playable media membership           | library substrate                                          | contents cannot derive membership from visible rows                         |
| Search and filter membership        | library substrate                                          | renderer sends query identity                                               |
| Active scope                        | library workstation controller                             | Sources and SCOPE write through this controller                             |
| Source panel visual state           | renderer projection from active scope and source selection | aggregate scope must not fake exact row selection                           |
| Projection                          | library workstation controller                             | drives read model/query shape                                               |
| View mode                           | library workstation controller for state; renderer applies | affects presentation, not membership                                        |
| Contents rows                       | renderer projection over substrate reads                   | retained rows are visual continuity only                                    |
| Primary selection                   | workstation selection controller                           | drives inspector context                                                    |
| Inspector context                   | derived from primary selection                             | do not store as independent authority unless cached with selection identity |
| Preview slot                        | slot lane runtime/controller                               | collapsed Hot Table expression                                              |
| Loaded slots                        | performance slot runtime                                   | library dispatch commands target it                                         |
| Workspace regions                   | workspace topology/runtime                                 | not library substrate                                                       |

## Required event behavior

| Event                                  | Required behavior                                                                                                                                                             |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Click exact source row                 | set active scope to that source, select source row, show SOURCE inspector                                                                                                     |
| Click source group                     | set active scope to group, mark group active, show SOURCE inspector for group                                                                                                 |
| Select SCOPE exact source              | set active scope to source, reveal/select matching source row                                                                                                                 |
| Select SCOPE aggregate                 | set active scope to aggregate, clear exact source row selection, do not fake a source                                                                                         |
| Select browse root                     | apply declared scope/projection/filter intent from the browse root activation contract                                                                                        |
| Change projection                      | keep scope, change query interpretation, validate selection                                                                                                                   |
| Change view mode                       | keep scope/projection/query membership, change rendering                                                                                                                      |
| Change health filter                   | keep scope/projection, filter contents rows, show active filter state                                                                                                         |
| Select track row                       | set primary selection to track, show TRACK inspector                                                                                                                          |
| Select preview/slot item               | set primary selection to slot, show SLOT inspector                                                                                                                            |
| Enter edit mode and select region      | set primary selection to workspace region, show WORKSPACE inspector                                                                                                           |
| Hover row                              | do not change primary selection or inspector context                                                                                                                          |
| Source goes offline                    | preserve source identity, update health/readiness, do not show authoritative empty state                                                                                      |
| Source reconnects or comes back online | re-validate source identity, update health to available, re-query active scope if it includes this source, do not fabricate track rows before rescan or coverage confirmation |
| Source removed/hidden                  | clear invalid scope/selection, choose explicit fallback, do not silently show stale rows                                                                                      |
| Zero admitted sources                  | show `+ Add Source`, show SCOPE as `No sources`, show add-source empty state, do not show fake `All Music`                                                                    |
| Load to Deck A/B                       | dispatch through shared slot load path, do not direct-load from library UI                                                                                                    |

## Rejection cases

Reject any design or implementation that does any of the following:

- shows `Local Files` as a peer of `All Music` when it means admitted local source material
- shows `Local Library` as a source row when it means an aggregate scope
- places playlists, crates, smart lists, or history under Sources as generic source rows
- places artists, albums, genres, labels, key, or BPM under Sources as if they were provenance
- represents `Ready`, `Offline`, or `Missing` as source rows instead of filters or status
- selects an individual source row when active scope is broader than that source
- treats the SCOPE control as a passive label
- lets Sources and SCOPE drift into independent state
- changes inspector context on hover
- hides inspector context behind a generic `INSPECTOR` title
- treats source inventory as the default musical browse view
- calls an unadmitted folder a source
- calls candidate media evidence a preview lane, preview slot, or source
- calls a source file a track before it has playable media or track identity semantics
- shows `No tracks` when coverage is incomplete, source is offline, filters exclude results, or no sources exist
- lets view mode change membership semantics
- lets renderer infer playable membership from visible rows
- creates a Library-only load path separate from perform slot dispatch
- builds the V0 preview strip as a disposable component unrelated to Hot Table
- treats preview slot and loaded deck as the same state
- lets workspace edit state own library state

## Acceptance bar for Library Workstation V0

Library Workstation V0 is acceptable only when the following are true:

- Sources panel shows provenance-bearing origins and source health.
- `+ Add Source` is an action, not a tree node.
- Zero-source state shows source admission affordance and does not claim `No tracks`.
- SCOPE exposes single-source, source-group, admitted-aggregate, and cross-origin levels.
- Clicking a source drives active scope.
- Changing SCOPE updates Sources visual state without fake source selection.
- Browse roots have explicit activation contracts for scope, projection, filters, and sort.
- The control bar always discloses active SCOPE, PROJECTION, VIEW, and active filters when present.
- Column view supports at minimum a source/facet/media path over admitted local material: Source origin → Artist → Album → Track rows.
- Contents shows rows for active scope, projection, filters, search, sort, and cursor.
- Retained rows are cleared according to the retained-row lifecycle and never prove membership.
- Contents health filters are visible and scoped.
- Empty states distinguish no results from zero-source, indexing, offline, missing, blocked, failed, incomplete, and filtered-out states.
- Source reconnection re-validates identity and re-queries affected active scopes without fabricating confirmed rows.
- Primary selection drives TRACK, SOURCE, SLOT, or WORKSPACE inspector context.
- Row identity validity is checked when scope, projection, filters, search, or source availability changes.
- Inspector header always labels context explicitly.
- Track inspector supports pre-load evaluation.
- Source inspector exposes health, coverage, and bounded maintenance actions.
- Preview lane exists as a single-slot collapsed Hot Table expression.
- V0 preview lane exposes preview/load/clear only unless real armed-slot state exists.
- Load actions route through shared slot dispatch.
- Literal source inventory is available as diagnostic mode, not as default musical browse.
- Renderer retains visual continuity without owning substrate membership.

## Design review checklist

Use this checklist for every mockup, implementation prompt, and agent report.

1. What source origin does this surface represent, if any?
2. What browse root is active, if any?
3. What exact browse-root activation effects were applied?
4. What active scope is being queried?
5. What projection is applied?
6. What view mode renders it?
7. What contents identity defines the rows?
8. Are any rows retained, and what ends retention?
9. What object is the primary selection?
10. Is the selected row identity valid for the current query?
11. What inspector context is shown?
12. What actions are valid for that selection?
13. Is this preview state or loaded slot state?
14. Is this normal mode or workspace edit mode?
15. Does any label imply false provenance?
16. Does any empty state hide readiness or coverage?
17. Does any load action bypass slot dispatch?
18. Does any renderer state duplicate substrate authority?

If any answer is unclear, the design is not ready for implementation.

## First implementation target

The first target is:

```text
Library Workstation
with source-health-aware Sources,
explicit SCOPE / PROJECTION / VIEW control bar,
column-capable browse surface,
contents health filters,
context-labeled inspector,
and a single-slot preview lane built as Hot Table V0.
```

Perform mode can sequence later, but the preview and load boundary must be architected now through shared slot identity and dispatch. The Library Workstation must not invent a temporary load model that the Perform Workstation later replaces.

The product direction is clear:

```text
Sources express provenance.
Browse roots express entry intent.
Scope expresses material boundary.
Projection expresses interpretation.
View expresses rendering.
Contents expresses results.
Primary selection drives inspection.
Preview is collapsed Hot Table.
Loaded slots belong to performance.
Workspace regions belong to topology.
```
