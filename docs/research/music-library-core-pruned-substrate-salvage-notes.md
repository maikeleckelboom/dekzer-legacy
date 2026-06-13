# Music Library Core Pruned Substrate Salvage Notes

## Status

This note preserves the useful findings from the `music-library-core` repository review.

The active Dekzer repository has already absorbed the useful first-slice library substrate direction. This note is not a
request to restore `music-library-core`, copy its code, or re-open old compatibility paths.

The value is narrower:

- identify the last useful pre-pruning snapshot
- preserve the substrate concepts that were removed or hidden during cleanup
- decide what Dekzer should rebuild later for beatgrid, waveform, key, loudness, cue, and preparation analysis

## Short verdict

The current `music-library-core` `main` branch is useful as confirmation of first-slice direction, but the remaining new
value is in the pruned analysis/artifact substrate that existed before the April 23 cleanup.

The best commit anchor found for the full pre-pruning substrate is:

00269f4223853d657374c42b872a55cc0d699be4
fix(library-sqlite): enforce work lifecycle invariants and rename canonical authority tests
2026-04-22 20:33:37 +01:00

This appears to be the last useful pre-pruning anchor found through the GitHub connector before the cleanup sequence
begins. Locally, verify the exact parent of the first pruning commit with:

git rev-parse 5792afa9cca2382eb7da1d2dd4a8be81de4c7c1b^

If that parent is different from 00269f4223853d657374c42b872a55cc0d699be4, prefer the exact parent as the archival
inspection point. The connector could not reliably enumerate branch refs or parent metadata, so the local parent check
should be treated as the final authority.

The first pruning commit found is:

5792afa9cca2382eb7da1d2dd4a8be81de4c7c1b
refactor(library-sqlite): remove legacy product exports and quarantine old compatibility paths
2026-04-23 00:22:12 +01:00

That commit starts the visible cleanup path: old areas become crate-internal, public exports are reduced, compatibility
paths are quarantined, and obsolete projection/publication domains begin disappearing.

## Timeline worth preserving

### Foundation of analysis/artifact substrate

c32ed50be22dbd8f4522da9b5036df2185a8817d
rename durable prep family to frozen analysis ontology

This marks the old transition from preparation-shaped vocabulary into a more general analysis ontology.

ebbb4b4af33622d8e8854361d50cc581abf5c6a7
refactor(library): recut artifact storage and retention surfaces for analysis durability

This is the early artifact-retention design pass.

5bdebea8d16bee0a89b2765507ce2069bdb720bb
refactor(library): finish claimant-owned artifact retention law

This is relevant because artifact retention should be owned by the claimant/use case, not by casual result rows.

c4543b30355d18a0b49ee804959b478b104d17d0
Add analysis execution seam and typed artifact materialization handoff

This is one of the most important pre-pruning commits. It contained the analysis execution path, typed artifact
materialization, artifact roles, media types, and execution outcome handling.

cb2811d82daef793488f20673249c6658fb92b4e
analysis: persist durable artifacts before marking execution success

This is a core invariant for Dekzer.

50c4ac7bff7c70a421db492219593820076117c1
library-sqlite: bind analysis artifact file cleanup to outer write transaction outcome

This is relevant for artifact-file lifecycle and crash-safe cleanup.

72d6475d75575e58a401132fceea351e5d7b9f3d
Add superseded artifact retirement and cleanup queue handling

This is relevant for re-analysis, imported grids, user edits, and artifact replacement.

1c2b6bdca71ecbf883e850e53609b871453c6e68
Add artifact storage cleanup queue drain worker

This is relevant for durable cleanup, not best-effort deletion.

cbc241343d30f388cc8483af06ac9d934ce98be6
feat(storage): add lawful app-owned artifact file store for large analysis payloads

This is relevant for beatgrid and waveform payload storage policy.

### Product-facing enrichment behavior

c16218dc5a70de0a5eb6b231e6c1f00a4393e183
feat(library): surface inventory-backed browse rows immediately and enrich them progressively through probe and
analysis

e175202820e17e473336029323cb5bc22db1b5fc
feat(library): surface inventory-backed browse rows immediately and enrich them progressively through probe and
analysis

These are important product behavior commits. They reinforce the Dekzer rule that rows become visible from
hierarchy/inventory first, then gain probe and analysis information later.

408ba0b353bb2868ba55cbd72cfe449b6d25951a
feat(library): add selected-item urgent acquisition and analysis escalation for immediate DJ use

This is the strongest product feature to salvage conceptually. It recognizes that the selected or soon-to-be-used track
needs priority over background enrichment.

### Last pre-pruning hardening

0b7ca1b999addd7e231689f9180bed08e7a80878
fix(sqlite): restore baseline metadata singleton to greenfield schema

00269f4223853d657374c42b872a55cc0d699be4
fix(library-sqlite): enforce work lifecycle invariants and rename canonical authority tests

The 00269f commit is the best known pre-pruning inspection anchor because it hardens work lifecycle invariants after the
substrate was already present.

### Pruning and cleanup sequence

5792afa9cca2382eb7da1d2dd4a8be81de4c7c1b
refactor(library-sqlite): remove legacy product exports and quarantine old compatibility paths

120a6652e086becd58cd91679316c62182e82693
refactor(library-sqlite): remove internal legacy replay subjects and center store on navigation and playable-browser
projections

321fb8cb29ead1073a2dd3804290279f1ec44cef
refactor(library-sqlite): port root and discovery substrate onto canonical source tables and remove legacy root file
location state

8ce1d9f676f4a8e728870d6ebcb4a50a1a014e13
refactor(library-sqlite): port artifact persistence onto generic artifact tables and remove analysis-owned file-store
control

73602a33cb85bc500e96ca59eceff298194054c0
library-sqlite: remove legacy materialization and compat read models

82292864d714e1bfa6f4e9896427a697da8c416d
chore(cutover): remove dead desktop staging and restore maintained cargo check

c9973fd34b8625e7c0d0e99a1d6531af082ce64e
refactor(cutover): remove compatibility projection surface from maintained crates

The cleanup sequence had good reasons. It removed compatibility baggage and surface confusion. Do not undo it. Use the
pre-pruning commit only as an inspection point for substrate ideas.

## What the pre-pruning substrate appears to have contained

### 1. Work item lifecycle

The old substrate had a machine-work model with:

- queued work
- claiming/leasing
- active work runs
- completion
- failure
- blocking
- retry/abandon implications
- lifecycle invariants

The 00269f hardening is especially important because it appears to reject invalid state transitions, such as completing
a work item while a run is still open, or completing work without a successful run.

Adopt this law:

A work item cannot become completed unless its latest execution/run has completed successfully and the required
artifacts are durable.

### 2. Work run / execution boundary

The old system distinguished a work item from a run/execution.

That distinction matters. A work item is the requested unit of work. A run is one concrete attempt by a specific
adapter/version at a specific time.

Dekzer should keep this separation.

A beatgrid request may have multiple attempts over time:

- first analyzer version fails
- second analyzer version succeeds
- user requests reanalysis
- imported grid is compared
- manual edit creates a derived accepted artifact

Those are not the same thing.

### 3. Typed artifact materialization

The old system had typed artifact roles and media types. It included roles like beatgrid and key result in older
analysis materialization work.

Dekzer should rebuild this as:

- artifact family
- artifact kind
- artifact role
- media type
- producer/adapter identity
- basis fingerprint
- payload hash
- creation time
- payload location
- accepted/retired state

Beatgrid should not be a scalar column. It should be an artifact family.

### 4. Artifact persistence before completion

This is the most important invariant.

The old system explicitly moved toward:

- create/run analysis
- materialize artifact
- persist artifact
- only then mark work complete

Dekzer should adopt this as a hard rule.

No analysis job may report complete while its artifact is only in memory, only in the renderer, only in a temporary
file, or not yet accepted.

### 5. Superseded artifact retirement and cleanup

The old system recognized that artifacts have lifecycle beyond creation.

Dekzer needs this for:

- reanalysis after analyzer upgrades
- user-corrected beatgrids
- imported Serato/Rekordbox/Traktor grids
- waveform format changes
- corrected decoder profiles
- changed source-file identity
- stale basis fingerprints
- failed writes or abandoned payloads

An artifact should not disappear casually. It should become superseded, retired, invalidated, or cleaned through an
explicit path.

### 6. App-owned artifact file store

The old system had a storage direction for large analysis payloads.

Dekzer should support two payload modes:

- inline payload for small summaries
- app-owned artifact file/blob for larger payloads

Beatgrid V0 may fit inline, but waveform overviews, detailed waveforms, stems summaries, and long flexible beat maps may
not.

The schema should not force every payload through JSON rows.

### 7. Progressive enrichment of inventory-backed rows

The old browser work had the right user-facing direction:

- source inventory appears first
- probe data arrives later
- analysis artifacts arrive later
- browser projection reports readiness honestly

This is already part of Dekzer doctrine. Keep it.

The analyzer should not be a gate for browsing.

### 8. Selected-item urgency

The old selected-item urgent target is worth adopting as product behavior.

Dekzer needs a Rust-owned priority lane where user intent can raise priority for the selected, loaded, armed, or
inspected track.

Renderer intent:

The user is focused on this source file or accepted track.

Rust response:

Prioritize acquisition, probe, waveform, beatgrid, key, and readiness work according to product rules.

This should not be a renderer-owned scheduler.

### 9. Segment-aware analysis basis

The old analysis-basis work suggests the design should eventually support segment-aware analysis.

Dekzer should not assume one file equals one beatgrid forever.

Future basis targets may include:

- whole decoded source file
- accepted track identity
- file segment
- cue-defined region
- long recording section
- imported external grid basis
- manually edited derived basis

For V0, analyze whole local audio files or accepted tracks. But do not design a schema that makes segment-aware
artifacts impossible.

## What Dekzer should adopt

### Adopt now as doctrine

1. Analysis results are durable artifacts, not fields.
2. Analysis work has requests, work items, runs, artifacts, and acceptance.
3. Completion requires durable artifact persistence.
4. A work item and a work run are different concepts.
5. Beatgrid is an artifact family, not a BPM column.
6. Artifact replacement requires explicit supersession/retirement.
7. Large payload storage must be a first-class policy.
8. Selected-track urgency is product behavior and belongs in Rust-owned scheduling.
9. Browser rows appear before enrichment and report readiness honestly.
10. Analysis basis identity must leave room for segments and imported/manual provenance.

### Adopt later as implementation substrate

Design a generic analysis/artifact substrate with:

- analysis family registry
- analysis basis
- analysis request
- machine work item
- work run
- adapter identity and version
- artifact metadata
- artifact payload storage
- accepted artifact pointer
- superseded artifact retirement
- cleanup queue
- progress/events
- failure and retry policy
- selected-target priority lane

Do this before implementing the serious beatgrid analyzer. A throwaway beatgrid table will create churn.

### Keep out

Do not restore:

- compatibility projection surfaces
- old public product exports
- old desktop staging
- old replay subjects
- old direct renderer/desktop analysis control
- compatibility wrappers
- old vocabulary
- one-off beatgrid state outside the generic artifact path

## Suggested Dekzer doc follow-up

Create or update a Dekzer doc later:

docs/analysis/analysis-artifact-substrate.md

That doc should define:

- ownership
- durable tables
- artifact lifecycle
- accepted artifact semantics
- payload storage policy
- cleanup contract
- work scheduling
- selected-target urgency
- renderer projection contract
- beatgrid as first artifact family

Do not include algorithm details in that doc. Algorithm design belongs in a separate beatgrid analyzer design doc.

## Suggested first implementation slice later

A good first analysis substrate slice is not “build beatgrid analyzer.”

It is:

1. add analysis family enum/registry with beatgrid as the first family
2. add analysis basis identity for source file V0
3. add analysis request/work item/work run tables
4. add artifact metadata and inline payload storage
5. add accepted artifact pointer
6. add minimal cleanup state
7. add one fake/test adapter that materializes a deterministic tiny beatgrid artifact
8. prove completion cannot occur before artifact persistence
9. expose a small Rust service read for accepted beatgrid summary
10. keep renderer as observer only

After that, the real beatgrid analyzer can plug in as an adapter.

## Final note

The pre-pruning `music-library-core` code is not authority. It is prior-art.

The commit to inspect first is:

00269f4223853d657374c42b872a55cc0d699be4

Then inspect backward for substrate construction and forward into the pruning commits to understand what was
intentionally removed.

Dekzer should not resurrect the old branch. Dekzer should rebuild the analysis/artifact substrate cleanly with the
current Rust/SQLite ownership model.
