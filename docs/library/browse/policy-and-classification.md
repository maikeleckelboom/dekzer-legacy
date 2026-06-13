---
status: accepted
doctrine-version: 0.1
last-reviewed: 2026-06-09
owner: library-browser-architecture
canonical-context:
  - background-root-scan-lifecycle-diagrams
  - electron-boundary-spine
  - library-browser-representation-contract
  - library-browser-representation-composition
scope:
  - browse-policy-and-classification
  - filter-registry
  - content-row-projection
  - interpretation-links
  - cue-and-companion-file-rules
  - facet-projection
  - row-identity
  - navigation-readiness-probe-independence
---

# Browse Policy and Classification

---

## 1. Problem statement

Dekzer's library presents two surfaces to the user: a stable browse-scope tree on the left, and a content pane on the
right that shows playable or inspectable items scoped to the selected node. These surfaces have different relationships
to the active browse filter and to the underlying source files. Without a clear ownership model, the filter,
classification, interpretation, and facet systems bleed into each other and produce either a fragile implementation or a
god object.

This document establishes the canonical separation of concerns, the complete vocabulary, the built-in filter registry,
the CUE and companion-file rules, the facet behaviour contract, and the relationship to the navigation readiness probe.
No code
may be written against the filter registry, content read policy, or facet projection without this document reaching
canon status.

---

## 2. Governing principles

Four sentences govern this entire domain. They define ownership and prevent scope creep.

> **Source files are classified.**
> **Relationships are interpreted.**
> **Browse filters project rows.**
> **Facets summarise known content.**

Each sentence implies a separate owner. None of the four may absorb the responsibilities of another.

A fifth invariant governs the tree specifically:

> **Tree containment is filter-agnostic. Tree content facets and the content pane are filter-aware.**

Disclosure state, `hasChildren`, and `leaf` are structural observations. They do not change when the active filter changes.
Folder facets and content pane rows do change.

---

## 3. Ownership model

Eight concerns are separated. Each has a single owning authority.

### 3.1 Source structure

**Owner:** Scan / hierarchy authority (navigation readiness probe and background scan hierarchy pass).

Source structure answers questions about browse-scope hierarchy. For local filesystem sources, child browse scopes are
child directory scopes. Future external-library adapters may produce non-directory child scopes under their own adapter
contracts.

- What child browse scopes exist under this source root?
- For local filesystem sources: what child directory scopes exist under this node?
- Does this node have child scopes (`hasChildren`) or not (`leaf`)?
- Is this node blocked, failed, or still probing?

Source structure does **not** answer:

- What file classes exist in this directory?
- What media kinds are present?
- What interpreted content can be produced?

**The navigation readiness probe produces early source-structure observations for newly admitted local filesystem sources.**
Background scan may later extend or refresh those observations. Source structure does not depend on classification,
interpretation, browse policy, or facet projection.

**The first visible source-structure window must be committed without waiting for classification or interpretation.**
Classification and interpretation may run concurrently after source admission, but they must not block the first
hierarchy window. See §11 for the full probe relationship.

### 3.2 Source-file observations

**Owner:** Scan / source indexer.

Source-file observations are filesystem-level observations about a file as it exists on disk. They are the shared foundation
that classification, identity, and interpretation all reference by `sourceFileId`. None of those authorities owns these
fields.

| Field           | Description                                                            |
| --------------- | ---------------------------------------------------------------------- |
| `sourceFileId`  | Stable library-database identifier for this file                       |
| `path`          | Resolved filesystem path                                               |
| `size`          | File size in bytes                                                     |
| `mtime`         | Modification timestamp                                                 |
| `volumeFileId`  | Volume-level inode or file ID, if available, for fast change detection |
| `accessibility` | `accessible` / `permissionDenied` / `unavailable`                      |

Source-file observations are recorded at discovery time and updated when the scanner detects a change. Classification, identity
hashing, and interpretation read these observations by reference; they do not duplicate them.

### 3.3 Source-file classification

**Owner:** Scan / classifier.

For every source file discovered, the classifier produces a classification record keyed by `sourceFileId`:

| Field                 | Description                                                                                      |
| --------------------- | ------------------------------------------------------------------------------------------------ |
| `fileClass`           | What kind of file this is (see §4.1)                                                             |
| `mediaKind`           | What playable experience it contributes to, if directly inferable from the file alone (see §4.2) |
| `companionRole`       | Whether the file is a companion to another file, if directly inferable                           |
| `classificationState` | `pending` / `classified` / `failed` / `blocked`                                                  |

Classification references source-file observations (§3.2) by `sourceFileId` and may later reference identity observations (§3.4), but
owns neither. **BLAKE3 hash completion does not gate classification.** A file may be classified before hashing is
complete. The classifier reads extension, MIME type, and light content probing only.

Classification is performed once per observed source-file version and persisted until invalidated by source-file observation
changes or classifier-version changes. It is not recomputed on each browse query. Switching the active filter does not
trigger reclassification.

### 3.4 Identity and evidence observations

**Owner:** Scan / identity worker.

Identity observations support duplicate detection, change detection, and interpretation validation. They are computed
independently of and asynchronously from classification.

| Field             | Description                                                |
| ----------------- | ---------------------------------------------------------- |
| `sourceFileId`    | Foreign key to source-file observations (§3.2)             |
| `hashState`       | `pending` / `computing` / `complete` / `failed`            |
| `blake3Hash`      | BLAKE3 content hash, populated once `hashState = complete` |
| `hashGeneratedAt` | Timestamp when the hash was last successfully computed     |

Identity observations update without blocking or invalidating existing classification records.

### 3.5 Interpretation links

**Owner:** Scan / interpreter.

The interpreter reads classification records and persists relationships between source files and between source files
and interpreted content assets. These relationships are **not** computed at query time. They are persisted observations with
their own lifecycle.

Examples of interpretation links:

- A `cueSheet` file points to an `audioFile` target → may produce a `cueBackedDisc` asset
- A `playlistFile` references a set of source paths → may produce a resolved playlist/import relationship in a future
  playlist/import contract
- An `artworkImage` is associated with a track or album candidate
- A `cueSheet` target is missing → broken link
- A `cueSheet` target is present but the cue file fails to parse → broken link

**Interpretation link lifecycle:**

| State        | Meaning                                                            |
| ------------ | ------------------------------------------------------------------ |
| `unresolved` | Link identified, resolution not yet attempted                      |
| `resolved`   | Link fully resolved; interpreted asset is valid                    |
| `broken`     | Link exists; target is missing, unreadable, or parse-invalid       |
| `ambiguous`  | Multiple resolution candidates exist; cannot auto-resolve          |
| `stale`      | Previously resolved; source file has changed since last resolution |
| `ignored`    | Link explicitly ignored by policy or user action                   |

The interpreter must not create content rows visible to the renderer. It persists link observations. The content projection
authority reads those observations and decides which rows to emit.

### 3.6 Content row projection

**Owner:** Content projection / read model (library-read-kernel).

The content projection reads classification records and interpretation links and produces the `ContentRow` objects that
the content pane renders. The row universe and the row types produced depend on the active browse policy (§3.7).

Content rows are projection outputs. They may be computed on read or materialized by a read model, but their authority
derives from source-file observations, classification records, and interpretation links. A materialized read model is a valid
performance optimization; it does not change the architectural owner or alter the truth of which layer is authoritative.

### 3.7 Browse policy / filter registry

**Owner:** Read / projection (library-read-kernel, browse session parameter).

A browse policy is the complete description of what the content pane renders for the currently selected scope. It is a
session-level parameter, not a global singleton (see §8).

A browse policy is defined by four fields:

```
BrowsePolicy {
  id:                   PolicyId               // internal identifier
  label:                String                 // user-facing label
  rowUniverse:          RowUniverse            // which set of rows is the input
  predicate:            ContentPredicate       // which rows pass the filter
  interpretationPolicy: InterpretationPolicy   // how to handle interpreted vs raw
  persistenceBehavior:  PersistenceBehavior    // workflow or inspection
}
```

The built-in filter registry is defined in §5.

### 3.8 Facet projection

**Owner:** Facet projection authority (library-read-kernel, post-classification pass).

Facet projection reads classification records for a scope (including descendants when the browse model is
descendant-aware) and produces `ScopeFacet` summaries that the tree renderer uses to annotate folder nodes.

Every `ScopeFacet` must carry the following fields:

| Field               | Description                                                              |
| ------------------- | ------------------------------------------------------------------------ |
| `coverage`          | `directOnly` or `descendants` — which files were counted                 |
| `completeness`      | `unknown` / `partial` / `complete` — how much of the scope is classified |
| `classifiedCount`   | Number of files for which classification is complete                     |
| `unclassifiedCount` | Number of files discovered but not yet classified                        |
| `blockedCount`      | Number of files that could not be accessed or classified                 |
| `generatedAt`       | Timestamp / classification epoch at which this facet was last produced   |

A `ScopeFacet` without `coverage` and `completeness` is invalid and must not be consumed by the renderer.

Facets are **not** produced by the navigation readiness probe. Initial facet state is `unknown` or `probing`. Facets are
updated as classification catches up.

**V0 workflow filter facets use `descendants` coverage.** `directOnly` coverage is permitted only in diagnostic or
raw-inventory contexts, not in the default musical browse model. A scope whose playable files are nested below the
selected folder must reflect those files in its facets.

Facet visual prominence is active-filter-aware. Facet content is not. See §9.

---

## 4. Vocabulary

### 4.1 File class

`FileClass` is the literal kind of a source file, inferred from extension, MIME type, and light content probing. It
describes the file as it exists on disk, independent of any interpretation.

| Value          | Meaning                           | Example extensions                                        |
| -------------- | --------------------------------- | --------------------------------------------------------- |
| `audioFile`    | Directly playable audio file      | `.mp3` `.flac` `.wav` `.aiff` `.ogg` `.m4a` `.opus` `.wv` |
| `videoFile`    | Directly playable video file      | `.mp4` `.mkv` `.avi` `.mov` `.wmv` `.webm`                |
| `cueSheet`     | CUE sheet file                    | `.cue`                                                    |
| `playlistFile` | Playlist or tracklist file        | `.m3u` `.m3u8` `.pls` `.xspf`                             |
| `artworkImage` | Image file used as cover art      | `.jpg` `.jpeg` `.png` `.webp`                             |
| `textDocument` | Plain text, log, or metadata text | `.txt` `.nfo` `.log` `.md`                                |
| `unknownFile`  | Unrecognised or unclassifiable    | —                                                         |

`FileClass` is stable for the current observed source-file version/epoch. If source-file observations change (file replaced in
place, extension changed, cloud placeholder hydrated), classification may be invalidated and recomputed. `FileClass`
does not change when the active browse policy changes.

### 4.2 Media kind

`MediaKind` describes what playable experience a file or interpreted asset contributes to. It is an attribute of
content, not of raw files. A raw `cueSheet` has no `MediaKind` by itself; only the resolved `cueBackedDisc` asset it may
produce carries `MediaKind.audio`.

Direct playable source files still receive a `MediaKind` when that playable experience is directly inferable from the
file alone. A `.flac`, `.wav`, or `.mp3` file that is later identified as the target of a CUE sheet still has
`fileClass = audioFile` and may contribute `MediaKind.audio` as a direct playable asset. Classification does not hide
that file.

The content projection hides a CUE target audio file under `preferInterpreted` only after a resolved or stale CUE
interpretation emits a `cueBackedDisc` row. If the CUE link is `unresolved`, `ambiguous`, or `broken`, no interpreted
playable row is emitted in workflow filters, and the direct audio file may remain visible as a normal
`playableAudioAsset` unless a later problem or repair surface explicitly chooses a different presentation.

Each `MediaKind` value carries registry metadata:

| Metadata field                   | Description                                                  |
| -------------------------------- | ------------------------------------------------------------ |
| `playable`                       | Whether this kind contributes to a playback experience       |
| `includedInMediaFilterByDefault` | Whether this kind is included in the built-in `media` filter |
| `defaultWorkflowFilter`          | Which built-in filter primarily surfaces this kind           |

The built-in `media` filter predicate is: `mediaKind ∈ {kinds where includedInMediaFilterByDefault = true}`.

**Adding a new `MediaKind` value does not automatically extend the built-in `media` filter.** The
`includedInMediaFilterByDefault` flag must be set to `true` deliberately. This prevents future playable kinds from
silently appearing in the primary DJ browse surface.

**V0 MediaKind registry:**

| Value   | Meaning                                     | playable | includedInMediaFilterByDefault | defaultWorkflowFilter |
| ------- | ------------------------------------------- | -------- | ------------------------------ | --------------------- |
| `audio` | Contributes to an audio playback experience | true     | true                           | `audio`               |
| `video` | Contributes to a video playback experience  | true     | true                           | `video`               |

Future values (not yet registered): `stems`, `karaoke`, `spatial`, `multichannel`. Each must opt in explicitly.

### 4.3 Content row kind

`ContentRowKind` describes the type of row the content pane renders. It is a projection concept, not a classification
concept.

`ContentRowKind` and representation kind are separate axes. `ContentRowKind` describes the rendered row form inside a
browse surface. Representation kind, as defined by [`representation-contract.md`](representation-contract.md), describes ownership, provenance, legal
actions, and the row's relationship to musical material. Content rows must carry both axes when they cross the read
boundary. They are not competing taxonomies.

Example: a `playableAudioAsset` emitted from Local Files is a playable row form inside the raw source representation. A
future Collection projection may emit a row with a similar playable form, but it is a Collection representation and must
not inherit Local Files semantics. A `cueBackedDisc` emitted from a Local Files audio workflow remains source-derived
unless a later Collection authority explicitly admits and re-projects it as canonical material.

| Value                | Meaning                                                                                            |
| -------------------- | -------------------------------------------------------------------------------------------------- |
| `playableAudioAsset` | A directly playable audio file row                                                                 |
| `playableVideoAsset` | A directly playable video file row                                                                 |
| `cueBackedDisc`      | An interpreted disc image produced from a resolved CUE + audio relationship                        |
| `cueTrack`           | An individual track within a cue-backed disc (reserved; not modelled as a separate row kind in V0) |
| `companionFile`      | A companion file shown as a file row (CUE sheet, playlist, etc.)                                   |
| `rawSourceFile`      | Any source file shown as a raw inventory row                                                       |

`cueTrack` is reserved for a future release when the disc image model is ready to expose internal track structure —
including timing, titles, indexes, gaps, and waveform segments — as clean independently loadable rows. Do not partially
model cue tracks in V0.

Blocked, unresolved, broken, stale, and ambiguous conditions are row state or related interpretation annotations, not
separate V0 row kinds. A `companionFile` or `rawSourceFile` row may carry source accessibility, classification state,
and related interpretation state fields, but it remains a companion or raw inventory row. A playable row may carry
`interpretationReadiness` and `loadEligibility` when it depends on interpretation.

### 4.4 Row universe

`RowUniverse` defines which population of rows a browse policy draws from.

| Value                       | Meaning                                                                                                                                                                                                                                                                                                      |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `interpretedPlayableAssets` | The set of playable content rows produced through classification and interpretation-aware projection. It includes directly playable source files and retained or resolved interpreted relationships according to policy. Prioritises canonical playable representations. Raw files are not directly visible. |
| `rawSourceFiles`            | The set of source files as classified. Interpreted assets are not additionally inserted. No file is hidden behind a derived representation.                                                                                                                                                                  |

### 4.5 Interpretation policy

`InterpretationPolicy` describes how the content projection handles cases where both an interpreted asset and its
constituent source files could appear.

| Value               | Meaning                                                                                                                                                                                                                                            |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `preferInterpreted` | Show the canonical interpreted row. Hide constituent raw files that are fully accounted for by a resolved or stale interpretation. Example: show `cueBackedDisc`, hide the raw `.cue` and its audio target only after that interpreted row exists. |
| `rawOnly`           | Show raw source files. Do not insert interpreted asset rows. Constituent files appear as raw rows regardless of whether an interpretation exists.                                                                                                  |

### 4.6 Browse policy

A named quadruple of `(rowUniverse, predicate, interpretationPolicy, persistenceBehavior)` that defines what the content
pane renders for the selected scope. Built-in policies are registered in §5. Custom policies (V2+) follow the same
structure.

### 4.7 Persistence behavior

| Value              | Meaning                                                                                   |
| ------------------ | ----------------------------------------------------------------------------------------- |
| `workflowFilter`   | Active policy survives session close. Restored on next launch.                            |
| `inspectionFilter` | Active policy is session-only. On close, the last persisted `workflowFilter` is restored. |

### 4.8 Interpretation readiness and related interpretation state

`InterpretationReadiness` is an attribute of interpreted playable content rows. It communicates the trust level of the
row's underlying interpretation link and is derived from the interpretation link lifecycle state (§3.5) at projection
time.

| Value        | Meaning                                                                                            | Visible to user                                           |
| ------------ | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `verified`   | Interpretation link is `resolved`; source files are unchanged since last resolution                | No — normal state                                         |
| `stale`      | Link was `resolved` but a source file has changed since last resolution                            | Yes — stale indicator on row                              |
| `broken`     | Link is `broken`; interpreted playable row is not produced in workflow filters                     | Yes only in surfaces that explicitly render problem state |
| `unresolved` | Link identified but not yet resolved; interpreted playable row is not produced in workflow filters | N/A                                                       |
| `ambiguous`  | Multiple resolution candidates; interpreted playable row is not produced in workflow filters       | N/A                                                       |

For raw inspection rows (`companionFile`, `rawSourceFile`), the raw file itself is not "verified" or "broken" as
playable content. Its related interpretation link may be resolved, stale, broken, unresolved, or ambiguous. That
relationship is exposed as a **related interpretation state annotation**, not as playback readiness.

Example: a raw `.cue` row in `companionFiles` may show `relatedInterpretationState = broken`, while its
`loadEligibility` remains `nonLoadable`.

### 4.9 Load eligibility

`LoadEligibility` is an attribute of a content row projection that describes whether the row may be loaded for playback
or preparation.

| Value         | Meaning                                                                                                                                                   |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `allowed`     | Row may be loaded normally                                                                                                                                |
| `warning`     | Row may be loaded, but with an explicit stale-readiness indicator; the playback and preparation system must acknowledge the stale state before proceeding |
| `blocked`     | Row may not be loaded until re-resolution succeeds                                                                                                        |
| `nonLoadable` | Row is an inspection or inventory row; it cannot enter playback or preparation                                                                            |

Derivation from `InterpretationReadiness` for interpreted playable rows:

| InterpretationReadiness | LoadEligibility |
| ----------------------- | --------------- |
| `verified`              | `allowed`       |
| `stale`                 | `warning`       |
| `broken`                | `blocked`       |

This derivation table applies only to emitted interpreted playable rows. `unresolved` and `ambiguous` links do not
produce interpreted playable rows in workflow filters, so no playable-row `LoadEligibility` is derived for them. A
`broken` interpretation maps to `blocked` only in a surface that explicitly emits a broken interpreted playable/problem
row; this table does not permit broken interpreted rows to appear in normal workflow filters.

`LoadEligibility` is a projection field. It must be supplied explicitly to the renderer. The renderer must not derive it
from visual state.

`LoadEligibility` applies only to rows that can enter playback or preparation. Non-playable inspection rows (
`companionFile`, `rawSourceFile`) carry `nonLoadable`, even when they annotate a related interpretation link as
resolved, stale, broken, unresolved, or ambiguous. The renderer must not treat absence of `loadEligibility` as
equivalent to `allowed`.

---

## 5. Built-in filter registry

Five filters ship as built-ins. They are the complete default set.

| User label      | Internal ID      | Row universe                | Predicate                                                                   | Interpretation policy | Persistence        |
| --------------- | ---------------- | --------------------------- | --------------------------------------------------------------------------- | --------------------- | ------------------ |
| Audio           | `audio`          | `interpretedPlayableAssets` | `mediaKind = audio`                                                         | `preferInterpreted`   | `workflowFilter`   |
| Video           | `video`          | `interpretedPlayableAssets` | `mediaKind = video`                                                         | `preferInterpreted`   | `workflowFilter`   |
| Media           | `media`          | `interpretedPlayableAssets` | `mediaKind ∈ {kinds registered with includedInMediaFilterByDefault = true}` | `preferInterpreted`   | `workflowFilter`   |
| Companion Files | `companionFiles` | `rawSourceFiles`            | `fileClass ∈ {cueSheet, playlistFile}`                                      | `rawOnly`             | `inspectionFilter` |
| All Files       | `allSourceFiles` | `rawSourceFiles`            | all file classes pass                                                       | `rawOnly`             | `inspectionFilter` |

**Default active filter:** `audio`. Dekzer's V0 slice is audio-first. Showing video, companion, and raw files by default
makes the application feel like a file manager rather than a DJ library.

**Built-in activation contract:** All five built-ins are active registry members in the browser-flow arc. Audio, Video,
Media, Companion Files, and All Files each carry a distinct filter identity. Activating a built-in filter sets the
library browse session's active filter id and requires a re-keyed contents read for the selected scope and selected
depth. A contents read, retained snapshot, cursor, stale-response guard, and verified-empty result must include the
active filter identity through the concrete read policy/filter state used for that filter.

| Active filter   | Required read policy/filter semantics                                                                                                                            |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Audio           | Interpreted playable rows where `mediaKind = audio`; V0 protocol activation uses `audioBrowse`.                                                                  |
| Video           | Interpreted playable rows where `mediaKind = video`; identity is distinct from Audio and Media.                                                                  |
| Media           | Interpreted playable rows where `mediaKind` is in the registered media default set; V0 uses `{audio, video}` and protocol activation uses `playableMediaBrowse`. |
| Companion Files | Raw companion rows where `fileClass ∈ {cueSheet, playlistFile}`; identity is distinct from All Files.                                                            |
| All Files       | Raw source-file inventory rows with no predicate filtering; no interpreted assets are inserted.                                                                  |

The **Media** filter is not optional in the activated browser-flow contract. Dropping Media from filter wiring, treating
Media as Audio, or treating Media as All Files is a contract violation.

**Custom filters** are a V2+ feature. The architecture supports them as user-named browse policy records following the
same schema. No special-casing is required.

### 5.1 Policy notes

**`audio` and `media`:** The predicate is over `MediaKind`, not over `FileClass`. A `.cue` + `.flac` pair resolved into
a `cueBackedDisc` asset carries `MediaKind.audio` and passes the `audio` predicate. The raw `.cue` and `.flac` files do
not appear because `interpretationPolicy = preferInterpreted` hides constituents fully accounted for by a resolved
interpretation.

**`media`:** The predicate is `mediaKind ∈ {kinds registered with includedInMediaFilterByDefault = true}`. In V0 that
set is `{audio, video}`. Adding a new `MediaKind` does not extend this set automatically. The
`includedInMediaFilterByDefault` flag must be set deliberately on the new value.

**Media is playable media only.** The `media` policy must never be used as a synonym for `allSourceFiles`, raw
inventory, or "everything in the folder".

**`companionFiles`:** The predicate covers `cueSheet` and `playlistFile`. Artwork images (`artworkImage`) are excluded
by default (see §7). Future additions (`.log`, `.nfo`) require an explicit predicate update.

**`allSourceFiles`:** No predicate filtering. Every file class passes. No file is hidden. No interpreted asset is
inserted. This is raw inventory mode.

> **All Files is raw inventory, not a problems view.** It may annotate known interpretation and classification states on
> its rows, but it is not the owner of problem triage, repair workflows, or source health. A future Library Problems
> surface owns those concerns. Problem-specific UX — repair buttons, batch fix actions, triage flows — must not be added
> to All Files. Those belong to the Library Problems surface when it exists.

---

## 6. CUE and companion-file rules

### 6.1 Canonical CUE rule

> **A CUE file is a raw source file. A resolved CUE relationship may produce an interpreted audio content asset. The
> active browse policy determines which representation is visible.**

This rule must not be violated. It prevents duplicate rows in normal DJ browsing and prevents silent file hiding in raw
inventory mode.

### 6.2 Per-filter CUE behaviour

| Active filter   | CUE file                       | Associated audio file                                                                    | Interpreted disc asset                                      |
| --------------- | ------------------------------ | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| Audio           | Not visible                    | Hidden only after a resolved or stale `cueBackedDisc` exists; otherwise visible as audio | Visible as `cueBackedDisc` (`MediaKind.audio`) when emitted |
| Video           | Not visible                    | Not visible                                                                              | Not visible (disc is audio kind)                            |
| Media           | Not visible                    | Hidden only after a resolved or stale `cueBackedDisc` exists; otherwise visible as audio | Visible as `cueBackedDisc` (`MediaKind.audio`) when emitted |
| Companion Files | Visible as `companionFile` row | Not visible                                                                              | Not visible                                                 |
| All Files       | Visible as `rawSourceFile` row | Visible as `rawSourceFile` row                                                           | Not visible (raw inventory does not insert derived assets)  |

### 6.3 Broken, stale, and unresolved CUE

A CUE sheet whose interpretation link is `broken`, `unresolved`, or `ambiguous` produces no `cueBackedDisc` asset and is
not visible in workflow filters.

The associated direct audio file remains governed by its own classification while interpretation is unresolved,
ambiguous, or broken. `preferInterpreted` hides that audio file only when a resolved or stale interpretation actually
emits a `cueBackedDisc`. This prevents music from disappearing while relationship interpretation catches up or fails.

A CUE sheet whose interpretation link is `stale` produces a `cueBackedDisc` row with `interpretationReadiness = stale`
and `loadEligibility = warning`. The disc remains visible in the `audio` and `media` filters to preserve continuity. It
is not hidden because a source file changed. The row carries an explicit stale indicator and is not considered
verified-ready until background re-resolution succeeds.

| Link state   | Audio filter                   | Companion Files                         | All Files       | Interpreted-row readiness     | Raw-row load eligibility |
| ------------ | ------------------------------ | --------------------------------------- | --------------- | ----------------------------- | ------------------------ |
| `resolved`   | disc visible                   | raw CUE visible (annotated resolved)    | raw CUE visible | `verified` on `cueBackedDisc` | `nonLoadable`            |
| `stale`      | disc visible (stale indicator) | raw CUE visible (annotated stale)       | raw CUE visible | `stale` on `cueBackedDisc`    | `nonLoadable`            |
| `broken`     | not visible                    | raw CUE visible with error affordance   | raw CUE visible | no interpreted row            | `nonLoadable`            |
| `unresolved` | not visible                    | raw CUE visible with probing affordance | raw CUE visible | no interpreted row            | `nonLoadable`            |
| `ambiguous`  | not visible                    | raw CUE visible with warning affordance | raw CUE visible | no interpreted row            | `nonLoadable`            |

Raw CUE rows in `companionFiles` and `allSourceFiles` expose the link state as a related interpretation annotation. They
do not become playable rows and do not receive `allowed`, `warning`, or `blocked` playback eligibility.

A future Library Problems surface will surface broken and ambiguous links as actionable issues. That surface is out of
scope for this document.

### 6.4 Playlist companion-file rules

Playlist companion-file interpretation is deferred beyond V0. Under `companionFiles`, raw playlist files (`.m3u`,
`.m3u8`, `.pls`,
`.xspf`) are visible as `companionFile` rows. Under `allSourceFiles`, they appear as `rawSourceFile` rows.

Whether resolved playlist files produce browse scopes, content rows, or imported library objects is owned by a separate
playlist and import contract. **Playlist files must not appear as interpreted content rows in the `audio` or `media`
filter in V0.** This is not an oversight to fill in later; it is a deliberate boundary. Showing an `.m3u` as a "playable
asset" row mixed into track results is wrong regardless of how it is resolved.

---

## 7. Artwork rule

Artwork images (`.jpg`, `.jpeg`, `.png`, `.webp` and similar `artworkImage` file class members) are classified and
linked by the interpreter but are **not visible in any default browse filter except `allSourceFiles`**.

| Filter          | Artwork image visible?             |
| --------------- | ---------------------------------- |
| Audio           | No                                 |
| Video           | No                                 |
| Media           | No                                 |
| Companion Files | No (excluded by default predicate) |
| All Files       | Yes, as `rawSourceFile`            |

**Rationale:** Cover art files sitting beside audio files are a storage implementation detail, not a DJ browse target.
Including them in `companionFiles` would make that filter noisy for the primary use case. Classification and
interpretation still proceed: the `artworkImage` file class is assigned, and the artwork link to its associated track or
album candidate is persisted. A future artwork or metadata inspection surface can read those observations. The browse filter
layer simply does not surface artwork in any default filter other than raw inventory.

Custom filters (V2+) may include `artworkImage` in their predicate if a user explicitly constructs one.

---

## 8. Filter state and persistence

### 8.1 Browse policy is a session parameter

The active browse policy is a parameter on the library browse session context. It is not an application-global
singleton. V0 has one library browse session shared across all tree and content pane views, so the practical effect in
V0 is identical to a global. The architecture must not encode it as a global.

**V0 behaviour:** One active browse policy applies to all content pane reads. The tree is never affected by this
parameter (structural observations are filter-agnostic).

**Future behaviour:** Multiple browse panes (e.g., deck-locked browsing) each carry their own browse session context
with an independent active policy. No refactor is required if the session parameter model is implemented correctly in
V0.

### 8.2 Persistence rules

| Scenario                                                                                | Behaviour                                                     |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Dekzer closes while a workflow filter is active (`audio`, `video`, `media`)             | That filter is restored on next launch                        |
| Dekzer closes while an inspection filter is active (`companionFiles`, `allSourceFiles`) | The last persisted workflow filter is restored on next launch |
| No workflow filter has ever been explicitly chosen                                      | `audio` is restored on next launch (factory default)          |
| User switches from an inspection filter back to a workflow filter during a session      | New workflow filter becomes the persisted filter immediately  |

**Persistence scope:** The filter is persisted at the library browse session / user-profile level. It is not persisted
per source root. One filter state applies to the entire library browse session. The exact storage key and store location
are implementation details.

### 8.3 Filter switch behaviour invariants

The following must hold on every filter switch:

- No filesystem probe or scan is triggered.
- No tree node changes expansion state.
- No tree node changes disclosure affordance (`hasChildren` / `leaf`).
- No tree node is removed or added.
- Selection, scroll position, and focus do not change.
- A content read is scheduled immediately using the new policy.
- The content read is re-keyed because active filter identity is part of contents identity.
- The renderer retains the last accepted content snapshot until the new read completes (see §12.1, criterion 5).
- Facet visual prominence re-weights synchronously from the last accepted facet snapshot when enough facet data already
  exists. If new facet data is required, the prior accepted facet presentation is retained and marked pending without
  implying that the scope is empty, failed, or complete.
- The empty state message in the content pane is filter-specific (see §9.4).

---

## 9. Tree and facet behaviour

### 9.1 Tree containment invariants

These invariants hold regardless of the active browse policy:

1. A node's `hasChildren` or `leaf` state is determined solely by the existence of child scope directories. It is never
   determined by the presence or absence of content matching the active filter.

2. A node's expansion state (collapsed / expanded) does not change when the filter switches.

3. A node's position in the tree, its sort order, and its parent–child relationships do not change when the filter
   switches.

4. Row selection state does not change when the filter switches.

### 9.2 Facet model

A `ScopeFacet` is a summary of known content kinds present in a scope (and its descendants, when the browse model is
descendant-aware). Facets are produced by the facet projection authority after classification data is available. They
are **not** produced by the navigation readiness probe.

**V0 workflow filter facets use `descendants` coverage.** `directOnly` is reserved for diagnostic or raw-inventory
modes, not the default musical browse model. A parent folder whose tracks are nested one level deeper must show audio
facets reflecting those descendants; selecting the folder must not appear empty while tracks exist below.

**Facets are advisory:** Facet summaries are projection outputs. They must not be used as authority for content read
results, scan completeness, or source structure. A facet indicating zero audio items does not permit skipping a content
read for that scope.

Every `ScopeFacet` must carry the fields defined in §3.8. A facet without `coverage` and `completeness` is invalid and
must not be consumed by the renderer.

**Facet lifecycle per node:**

| Phase                                            | Facet state                                                               |
| ------------------------------------------------ | ------------------------------------------------------------------------- |
| Before first-window commit                       | `unscanned` — no facet affordance shown                                   |
| After first-window commit, before classification | `probing` — neutral/subdued affordance                                    |
| After partial classification                     | Facets for classified content appear; unscanned content remains `probing` |
| After full classification                        | Facets are complete for known content                                     |

**Reopened-source handling:** For a source previously scanned and committed to the database, persisted facet summaries
with a valid `generatedAt` epoch may be shown immediately at reopen without resetting to `probing`. The lifecycle above
applies to newly admitted sources. Do not force a previously scanned source back through `probing` state when its
persisted facets are fresh enough to be trusted.

Facets update progressively as classification catches up. These updates must not change disclosure state, row sort
order, or selection state.

### 9.3 Facet prominence rules

Facet visual prominence is active-filter-aware. **ScopeFacet data is filter-agnostic; the rendered subset, ordering, and
prominence of facet indicators are filter-aware.** The rule is:

> **Always emphasise content matching the active workflow filter. Show non-active content indicators only when they are
> meaningful enough to explain the scope or guide the user. Meaningfulness is a projection threshold, not a
> classification
> rule.**

**Prominence weights by scenario:**

| Scenario                                        | Audio facet               | Video facet                    | Companion facet |
| ----------------------------------------------- | ------------------------- | ------------------------------ | --------------- |
| Folder has audio, active filter = Audio         | Prominent                 | —                              | —               |
| Folder has audio + video, active filter = Audio | Audio prominent           | Video subdued (if meaningful)  | —               |
| Folder has only video, active filter = Audio    | —                         | Video subdued                  | —               |
| Folder has no content matching active filter    | — (content pane is empty) | Other kinds subdued if present | —               |

**Significance threshold:** A content kind facet indicator for a non-active filter is shown only when it clears a
significance threshold. The threshold is a configurable projection parameter, not a hardcoded rule. Threshold inputs
include:

- Absolute count of items of that kind
- Proportion of that kind relative to total known items in the scope
- Whether the active filter has zero matching items in the scope (a zero-match scope gives non-active facets higher
  utility as navigation aids)

A folder with 4 000 audio files and 1 video file must not display a prominent video facet in Audio mode. A folder with 0
audio files and 1 video file should display a subdued video facet in Audio mode, because the facet explains why the
content pane is empty.

**Facet prominence and inspection filters:** When the active filter is `companionFiles` or `allSourceFiles`, facet
prominence weighting reverts to neutral — all known content kinds at equal weight. These filters are diagnostic
surfaces; the active-filter-weighted emphasis model does not apply to them.

**Immediate re-weighting:** Immediate means a synchronous renderer/projection pass over already accepted facet data. It
must not trigger a scan, classifier pass, or blocking facet query. If a filter switch needs facet data that is not yet
accepted, the renderer keeps the previous accepted facet presentation and marks it pending. Pending facet presentation
must not look empty, failed, or complete.

### 9.4 Empty state

When the content pane has zero rows for the selected scope under the active filter, it must display a filter-specific
empty state message only after the contents readiness state proves verified empty. These labels are outputs of the
state machine, not copy polish.

| Active filter   | Empty state message                 |
| --------------- | ----------------------------------- |
| Audio           | "No audio items in this scope."     |
| Video           | "No video items in this scope."     |
| Media           | "No playable media in this scope."  |
| Companion Files | "No companion files in this scope." |
| All Files       | "No files in this scope."           |

Verified-empty labels are legal only when selected scope identity, active filter identity, selected depth, coverage, and
row count all prove absence for the active contents identity. Pending, incomplete, blocked, failed, missing,
unavailable, stale, cursor-invalid, unsupported-selection, and retained-pending states must not use these labels.

The empty state must never say "This folder is empty" without qualification when the folder contains content that is
hidden by the active filter.

---

## 10. Row identity

Row identity rules prevent selection bugs on filter switches, CUE state changes, and re-resolution events. They must be
established before any implementation that renders a scrollable content list.

1. **Raw source file rows** are identified by `(sourceFileId, rowUniverse)`. A `sourceFileId` in the
   `interpretedPlayableAssets` universe does not produce a raw row, and vice versa.

2. **Direct playable asset rows** (`playableAudioAsset`, `playableVideoAsset`) projected from a plain source file use
   this priority order: if a materialized playable asset id exists, that id is the row identity; otherwise the row id is
   derived from `sourceFileId` plus `ContentRowKind` plus `MediaKind`. They must not reuse the `rawSourceFile` row id
   from the `rawSourceFiles` universe. They must never reuse one row id across different `ContentRowKind` values. This
   prevents Audio-filter row identity and All Files row identity from aliasing on the same physical file.

3. **Interpreted asset rows** are identified by interpretation asset id or interpretation link id — not by any
   constituent source file id.

4. **`cueBackedDisc` identity** must remain stable for the lifetime of the resolved interpretation link. A disc row must
   not change its id while the underlying link state is `resolved` or `stale`, even as the stale indicator is added or
   removed.

5. **Switching the active filter must not reuse the same row id for semantically different row kinds.** If a `.cue` file
   appears as `companionFile` under `companionFiles` and the same file's resolved disc appears as `cueBackedDisc` under
   `audio`, those are two distinct row ids in two distinct row universes.

6. **The same physical file may appear in different row universes** — for example, as a `playableAudioAsset` in
   `interpretedPlayableAssets` and as a `rawSourceFile` in `rawSourceFiles` — but must never appear as duplicate
   competing rows within the same row universe.

7. **Row ids must not encode filter-specific state.** An id valid for the `audio` filter is absent under
   `companionFiles`; it does not change or alias between filters.

8. **Content snapshots carry scope and policy identity.** Every content snapshot must include the selected scope
   identity and the `browsePolicyId` used to produce it. The renderer must reject or ignore a snapshot whose scope
   identity, browse policy id, request token, or continuity token does not match the currently accepted request.

---

## 11. Relation to the navigation readiness probe

The navigation readiness probe (defined in [`background-root-scan-lifecycle-diagrams.md`](../source/background-root-scan-lifecycle-diagrams.md)) is the priority-lane operation
that discovers immediate child scopes after source admission. Its relationship to this document's model is strict.

**The probe answers only structural questions:**

- What child browse scopes exist under this node?
- For local filesystem sources: what child directory scopes exist under this node?
- Does this child have its own child scopes (`hasChildren`) or not (`leaf`)?
- Is a child blocked (permission denied), failed, or still probing?

**The probe does not answer:**

- What file classes are present in this directory?
- What media kinds are available?
- What interpreted content exists or could be produced?
- What facet values apply to this scope?

**The probe's output feeds only source structure (§3.1).** Classification (§3.3), identity (§3.4), interpretation (
§3.5), content row projection (§3.6), and facet projection (§3.8) receive no input from the probe and do not affect its
scheduling or completion.

**The probe is filter-agnostic by law.** It must not read, reference, or consult the active browse policy, any built-in
filter id, or any `BrowsePolicy` record. A code dependency from probe code to the browse policy registry is a contract
violation. The probe must compile and execute correctly in a build where the browse policy registry does not exist.

**Facets after the probe:** Immediately after the first child window is committed by the probe, newly admitted source
nodes have `facetState = probing`. Previously scanned source nodes with valid persisted facets may display those facets
immediately (see §9.2). Classification and facet projection then run as background work, progressively updating facet
state through the normal boundary push path. These updates do not re-trigger the probe.

---

## 12. Acceptance criteria

These criteria must be satisfied before any implementation against this document is considered correct.

### 12.1 Filter switch behaviour

1. Switching the active browse policy must not trigger any filesystem probe, scan restart, or classification work.
   Measure probe-start timestamps before and after a filter switch; no new probe must appear.

2. Switching the active browse policy must not change the expansion state of any tree node.

3. Switching the active browse policy must not change the `hasChildren` or `leaf` state of any tree node.

4. Switching the active browse policy must not add, remove, or reorder nodes in the tree.

5. On filter switch, a content read must be scheduled immediately. The renderer must retain the last accepted content
   snapshot and mark it pending until a new accepted snapshot arrives. Pending state must not look empty, failed, or
   complete. It must not clear the pane to empty or show an unqualified loading screen before the new read completes. If
   the selected scope has no previously loaded snapshot (first view of this scope under this filter), the pane may show
   a
   scoped loading state — not a blank pane or a misleading empty-state message.

6. Filter switch handling must preserve selected scope, expansion state, tree containment, disclosure affordances,
   scroll position, and focus. It may change content rows, retained content state, omission metadata, verified-empty
   eligibility, and facet prominence.

### 12.2 CUE and companion-file behaviour

7. Under the `audio` filter: a scope containing a resolved `cueBackedDisc` asset with
   `interpretationReadiness = verified` must show that disc row in the content pane. The raw `.cue` file and its
   associated raw audio file must not appear as separate rows.

8. Under the `audio` filter: a scope containing a `cueBackedDisc` with `interpretationReadiness = stale` must show that
   disc row with a visible stale indicator. `loadEligibility` must be `warning`. The row must remain visible while
   background re-resolution is running and must not be removed until re-resolution completes.

9. Under the `audio` filter: a scope containing an unresolved, ambiguous, or broken `.cue` file must produce no
   `cueBackedDisc` row. The raw `.cue` must not appear. A directly playable associated audio file may remain visible as
   a
   `playableAudioAsset` unless it is hidden by a resolved or stale `cueBackedDisc` projection.

10. Under the `companionFiles` filter: a raw `.cue` file must appear as a `companionFile` row, regardless of whether an
    interpretation link exists.

11. Under the `companionFiles` filter: a broken `.cue` file must appear with an error affordance indicating the related
    broken interpretation state. Its `loadEligibility` must be `nonLoadable`.

12. Under the `allSourceFiles` filter: a raw `.cue` file must appear as a `rawSourceFile` row. Its associated audio file
    must also appear as a `rawSourceFile` row. No `cueBackedDisc` row must appear. Raw rows must carry
    `loadEligibility = nonLoadable` even when they annotate a related interpretation state.

### 12.3 Artwork behaviour

13. Under `audio`, `video`, `media`, and `companionFiles` filters: artwork image files (`.jpg`, `.png`, and similar
    `artworkImage` file class members) must not appear in the content pane.

14. Under `allSourceFiles`: artwork image files must appear as `rawSourceFile` rows.

### 12.4 Facet behaviour

15. Immediately after the navigation readiness probe commits the first child window for a **newly admitted** source, all
    visible folder facets must be in `probing` or `unscanned` state. No facet content may appear before classification
    data is available.

16. For a **previously scanned source** at reopen: persisted facets may be displayed immediately, provided their
    `coverage` field matches the current scope/projection policy and their `generatedAt` epoch is within the valid
    retention window. Facets whose `coverage` does not match the current policy (for example, `directOnly` facets when
    the current policy requires `descendants`) must be treated as invalid and regenerated. Previously scanned sources
    must not be reset to `probing` when their persisted facets are epoch-valid and coverage-compatible.

17. Facet state transitions must not change disclosure state, sort order, or selection state of any tree node.

18. A folder with only video content and 0 audio content must not show a prominent audio facet when the active filter is
    `audio`.

19. A folder with 0 audio content and non-zero video content must show a subdued video facet when the active filter is
    `audio`, provided the video count clears the significance threshold.

### 12.5 Persistence behaviour

20. Closing Dekzer while `audio`, `video`, or `media` is active must restore that filter on next launch.

21. Closing Dekzer while `companionFiles` or `allSourceFiles` is active must restore the last persisted workflow filter
    on next launch, not the inspection filter.

22. The active browse policy must be stored as a session-context parameter. It must not be read or written as a
    process-global variable.

### 12.6 Navigation probe independence

23. The navigation readiness probe must complete its structural enumeration without reading or consulting active browse
    policy state. A code dependency from probe code to the browse policy registry, to any `BrowsePolicy` record, or to
    any filter-specific predicate is a contract violation. The probe must compile and execute correctly in a build where
    the browse policy registry does not exist.

24. The navigation readiness probe must not produce classification records, interpretation links, content row
    projections, identity observations, or facet values.

### 12.7 Row identity

25. Two content rows produced for the same physical file under different row universes must have distinct row ids that
    do not alias.

26. A `cueBackedDisc` row id must remain stable while the underlying interpretation link is in `resolved` or `stale`
    state. It must not change when the stale indicator is added or removed.

27. Switching the active filter must not reuse the same row id for a different row kind. Reuse of a row id across
    different `ContentRowKind` values is a contract violation.

28. A `playableAudioAsset` or `playableVideoAsset` row produced by direct projection from a source file must not share
    its row id with the `rawSourceFile` row for the same source file. Direct playable asset row ids must derive from a
    projected identity space distinct from the raw row id.

29. A content snapshot must carry the scope identity and `browsePolicyId` used to produce it. The renderer must reject
    or ignore snapshots whose scope identity, policy id, request token, or continuity token does not match the currently
    accepted request.

### 12.8 Renderer constraints

30. The renderer must not infer `ScopeFacet` values by counting visible content rows.

31. The renderer must not infer `hasChildren` or `leaf` from the presence or absence of content rows.

32. The renderer must not infer `MediaKind` or `FileClass` from file extensions visible in row display names.

33. The renderer must not infer `interpretationReadiness`, `relatedInterpretationState`, or `loadEligibility` from row
    appearance alone. These must be supplied as explicit projection fields on each content row when applicable.

34. The renderer must consume and display the projection state it is supplied. It must not derive or override structural
    or classification observations from any other source.

### 12.9 Source guards

35. Navigation readiness probe modules must not import the browse policy registry, filter registry, built-in filter
    definitions, or filter-specific predicates.

36. Renderer modules must not import file-extension classification helpers to infer `FileClass`, `MediaKind`, facets, or
    content row eligibility.

37. Facet renderer modules must not import content row reducers or count visible content rows to infer `ScopeFacet`
    values.

38. Content read code must receive the active `browsePolicyId` as an explicit request/session parameter. It must not
    read process-global browse policy state directly.

---

## 13. Open questions

Questions are divided by whether they block a specific implementation surface. Decisions already taken are reflected in
the document body and are not listed here.

### 13.1 Deferred companion contracts

The following contracts are deliberately deferred. They are not open decisions for this document, and implementations
must not infer their final behavior from placeholders here.

| Contract                                                      | Deferred scope                                                                                                                                                                                                  |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cue-backed-disc-image-model.md`                              | Internal track structure, timing, track identity, index/gap handling, waveform segment projection, and independently loadable cue-track rows.                                                                   |
| Bulk reclassification / classifier-version migration contract | How thousands of classification records are invalidated, retained, refreshed, and visually represented during classifier-version upgrades without lying to the renderer or dropping accepted state prematurely. |

### 13.2 Open — not blocking classification, filter, or content read

| #   | Question                                                                                                                     | Relevant section |
| --- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| Q2  | Should broken interpretation links surface in a dedicated "Library Problems" view, and if so, what authority owns that view? | §6.3             |

### 13.3 Must resolve before implementing the related surface

| #   | Question                                                                                                                                                                                                                                                                       | Relevant section | Blocks                          |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------- | ------------------------------- |
| Q1  | What is the exact significance threshold formula for non-active-filter facet indicators? Candidate inputs are defined in §9.3; the formula is a product decision.                                                                                                              | §9.3             | Facet UI implementation         |
| Q4  | What is the confirmation policy when a stale `cueBackedDisc` row with `loadEligibility = warning` is sent to a deck — does the playback path proceed immediately (the stale indicator is informational) or does it require an explicit user acknowledgement before proceeding? | §4.9, §6.3       | Stale-row playback path         |
| Q6  | What is the caching and invalidation strategy for descendant-coverage facet epochs — how are coverage completeness, partial descendant classification, and epoch mismatches represented and handled efficiently?                                                               | §3.8, §9.2       | Facet projection implementation |

---

_This document is implementation-gate canon. No changes to the filter registry, classification taxonomy, interpretation
lifecycle, or facet projection contract may be made without a documented revision to this file and review by the
architectural owner._
