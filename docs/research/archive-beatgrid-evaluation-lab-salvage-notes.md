# Archive Beatgrid Evaluation Lab Salvage Notes

## Status

This note preserves the useful findings from reviewing `archive-beatgrid-evaluation-lab`.

The repo is valuable as a beatgrid artifact, fixture, diagnostics, and evaluation reference. It should not be treated as
the future Dekzer analyzer implementation.

The future Dekzer analyzer should be rebuilt inside Dekzer’s current Rust/SQLite ownership model. The old lab should
sharpen the artifact contract, evaluation harness, fixture strategy, and failure-mode vocabulary.

## Primary inspection branch

Use the `dev` branch first.

The repo default is `main`, but the latest meaningful beatgrid evaluation work landed on `dev`. The branch comparison
showed `dev` had substantial additional work relative to `main`, including the BeatTimeline contract, WASM support,
fixture-run v2 contracts, decision visualization, fixture generation/checking, real-music harness, and evaluation/report
tooling.

Local verification command sequence:

git fetch origin --prune
git log --all --date=iso --decorate --oneline --max-count=80
git log origin/dev --date=iso --decorate --oneline --max-count=40
git log origin/main --date=iso --decorate --oneline --max-count=40
git diff --stat origin/main...origin/dev

Treat `dev` as the first salvage branch unless local inspection proves another branch is newer.

## Core verdict

Adopt the harness concepts, not the analyzer.

The repo’s useful center is:

1. sample-index fixture references
2. reference-window evaluation behavior
3. deterministic event matching
4. BeatTimeline-style artifact encoding
5. diagnostics explaining tempo and phase decisions
6. tempo-family candidate reporting
7. fixture generation and tiering
8. native/WASM/TypeScript artifact equivalence checks
9. decision visualization and reporting

The analyzer code is a useful lab specimen, especially for synthetic fixtures, but it is not production architecture.

## What to adopt

### 1. BeatTimeline-style artifact

The repo has a compact BeatTimeline v1 artifact with:

- sample rate
- beats per bar
- grid0 sample
- period in samples
- optional downbeat phase
- lock/confidence fields
- binary section encoding
- SHA-256 payload validation

Adopt the idea of a compact, versioned, hash-checked beat timeline artifact.

Do not adopt the exact representation unchanged.

For Dekzer, this must become:

- grid origin anchor sample, not ambiguous grid0 wording
- samples-per-beat as rational or fixed-point, not rounded integer period
- optional bar/downbeat anchor, separate from grid origin
- optional downbeat confidence, not an implied observation
- provenance for analyzer version, decoded audio basis, imported/manual/edit source, and reanalysis policy
- representation kind: rigid, segmented, flexible, unresolved
- future support for explicit beat sample positions when fixed period is not enough

Important correction:

A fixed grid can use anchor sample plus rational samples-per-beat. A flexible grid must store beat sample positions. A
segmented grid must store segment anchors and per-segment samples-per-beat.

### 2. Beatgrid result and diagnostics should be separate linked artifacts

The repo’s BeatgridArtifactV1 contains beat seconds, downbeat seconds, lock state, diagnostics, and BeatTimeline bytes.

That was fine for the lab, but Dekzer should separate concerns:

- Beatgrid artifact: compact runtime/playback representation
- Beatgrid diagnostics artifact: explanation, candidates, config, margins, novelty/evidence, lock clauses
- Beatgrid projection summary: lightweight browser/UI read model

Diagnostics should be durable enough for review and QA, but not required on the hot path.

### 3. Diagnostics are product-quality material

The diagnostics model is worth preserving as a design pattern. It captures:

- audio observations
- input identity and optional content hash
- config and config hash
- tempo scan candidates
- phase alignment candidates
- selected grid
- stability counters
- lock thresholds and lock clauses

This is exactly the kind of evidence trail Dekzer needs for a professional analyzer.

Adopt this structure, but rename and harden it:

- avoid banned vocabulary
- bound large arrays such as novelty values
- keep method ids stable
- record analyzer version and model version when applicable
- record fixture/report provenance separately from runtime artifact metadata

### 4. Sample-index evaluation

The evaluation crate matches reference and predicted events as sample indices. It computes counts, precision, recall,
F1, median error, p95 error, and matched deltas.

Adopt this directly as evaluation doctrine:

- reference beats are sample indices
- predicted beats are sample indices
- tolerances are sample windows
- seconds/BPM are projections for display and reports
- deterministic matching is required

This applies to beat, downbeat, bar anchor, segment boundary, and imported-grid comparison.

### 5. Reference-window evaluation

The repo has evaluation-window behavior that can either clip predictions to the annotated/reference window or count
outside predictions as false positives.

The legacy identifier uses banned wording. Do not carry that spelling into Dekzer.

Adopt the behavior with new names:

- ReferenceWindowPolicy
- EvalWithReferenceWindow
- reference_samples
- annotation_window
- pred_outside_reference_window
- CountOutsideAsFp
- ClipToReferenceWindow

Why this matters:

Many real fixtures only have partial annotations. The harness must distinguish:

- wrong inside the evaluated region
- extra predictions outside the annotated region
- clipped evaluation for limited-region fixtures
- strict outside-as-false-positive evaluation for full-region fixtures

### 6. Fixture sidecars

The fixture model is useful. It stores:

- fixture id
- sample rate
- audio relative path
- reference beats as samples
- reference downbeats as samples
- declared BPM
- bars and beats per bar
- lead-in samples
- noise level
- jitter
- swing/offbeat delay
- dropout ratio
- BPM ramp
- acceptance overrides
- notes

Adopt the sidecar idea.

Rename legacy fields before they enter Dekzer. Use annotation/reference terminology, not banned terminology.

Future Dekzer fixtures should support:

- exact synthetic references
- partial annotation windows
- downbeat uncertainty
- harmonic tempo ambiguity
- drift and rigidity classification
- imported grid comparisons
- local-only real-music fixtures with ignored audio files
- fixture notes explaining expected ambiguity or abstention

### 7. Tempo-family candidates

The repo models tempo candidates as families with:

- family id
- base BPM
- ratio set
- BPM members
- score
- normalized score
- evidence peaks

That is useful. It avoids pretending tempo is a single scalar too early.

Adopt candidate families as analyzer output and diagnostics.

Do not make family selection the same as display BPM selection. The analyzer can identify a family while the UI/display
layer chooses a perceptual or product-facing level with confidence and provenance.

### 8. Failure-mode tactics

The repo contains several tactical ideas that should be preserved as notes, not doctrine:

- active-window selection for lead-in silence
- multi-resolution autocorrelation
- peak sharpness penalty for jitter/broad peaks
- subdivision support for half-time rejection
- ghost-beat penalty
- fast-tempo support for 180+ BPM material
- rigidity confidence separate from periodicity confidence
- phase margin and tempo margin as separate lock clauses

These are useful failure-mode learnings. They should inform the next analyzer design, but not hard-code the product
architecture.

### 9. Native/WASM/TypeScript artifact equivalence

The dev branch includes evidence of:

- BeatTimeline contract spec/schema
- Rust WASM crate
- TypeScript decoder
- WASM equivalence script

Adopt the equivalence habit later.

Rust owns analysis in Dekzer, but compact artifact projection may eventually cross Rust, TypeScript, and perhaps
WebAssembly boundaries. When that happens, equivalence tests are valuable.

## What not to adopt

Do not adopt the analyzer implementation as the future analyzer.

Reasons:

- It is explicitly a temporary synthetic-click analyzer.
- It is rigid-grid oriented.
- It uses integer period samples, which can introduce long-track drift for non-integer periods.
- It treats downbeat phase too simplistically.
- It does not model segmented or flexible grids as first-class representations.
- It does not carry enough provenance for imported/manual/analyzer-edited artifacts.
- Diagnostics can include large novelty arrays without an obvious production bounding policy.
- Some heuristic tuning is fixture-driven and should not become product doctrine.

Do not adopt the old vocabulary directly.

Existing lab identifiers that contain banned terms must be renamed before any concept enters Dekzer docs or code.

Do not adopt seconds as authority.

The lab stores beats_audio_s and downbeats_audio_s for compatibility/reporting. Dekzer’s durable artifact should use
sample coordinates. Seconds are display/report projections.

## Specific corrections before Dekzer adoption

### Replace period_samples

Old:

period_samples: u64

Dekzer target:

samples_per_beat as rational/fixed-point value
or a numerator/denominator pair bound to the decoded sample clock

Reason:

At 44.1 kHz and 128 BPM, one beat is 20671.875 samples. Rounding to integer samples accumulates drift.

### Split grid origin and downbeat anchor

Old:

grid0_sample plus optional downbeat_phase

Dekzer target:

- grid_origin_anchor_sample
- grid_origin_anchor_provenance
- bar_anchor_sample or bar_phase if known
- bar_anchor_confidence
- downbeat_status: unresolved, inferred, imported, user_set, locked
- edit_anchor records for user corrections

A grid can be musically useful while bar/downbeat position is unresolved.

### Add representation kind

Dekzer beatgrid artifacts need an explicit representation kind:

- rigid
- soft
- segmented
- flexible
- unresolved

Rigid can use one anchor and one samples-per-beat value.

Segmented uses segment anchors and per-segment samples-per-beat.

Flexible stores explicit beat sample positions.

Unresolved stores evidence and warnings without pretending a usable grid exists.

### Add provenance

Every beatgrid artifact needs provenance:

- decoded audio identity
- source file / track / segment basis
- analyzer id
- analyzer version
- config hash
- model version, when applicable
- imported source, when applicable
- manual edit lineage, when applicable
- prior artifact id, when superseding
- accepted/superseded/retired state

### Separate artifact payload and projection summary

The browser does not need the whole beatgrid payload.

Use:

- durable artifact metadata
- artifact payload or blob pointer
- projection summary for browser/readiness
- diagnostics artifact for review and QA

## Suggested future Dekzer artifact shape

This is not a schema. It is the conceptual target.

BeatgridArtifact:

- artifact id
- artifact version
- representation kind
- basis id
- decoded sample rate
- decoded duration samples
- grid origin anchor
- samples-per-beat rational/fixed-point for rigid grids
- segment list for segmented grids
- explicit beat samples for flexible grids
- optional bar/downbeat anchor
- confidence summary
- warning list
- provenance
- created at
- accepted/superseded/retired state

BeatgridDiagnosticsArtifact:

- linked artifact id
- analyzer config
- config hash
- tempo candidate families
- phase candidates
- selected grid decision
- lock clauses
- stability data
- residual stats
- bounded onset/tempogram evidence
- failure or abstention reasons

BeatgridProjectionSummary:

- artifact id
- readiness state
- representation kind
- display BPM
- confidence class
- bar/downbeat known flag
- warnings
- last analyzed at
- source/provenance label

## Fixture strategy to adopt

Tier A:

- deterministic synthetic clicks
- exact sample references
- wide BPM points from 30 to 300
- no ambiguity expected

Tier B:

- jitter
- swing
- dropout
- lead-in silence
- offbeat emphasis
- harmonic ambiguity
- tempo ramps
- weak downbeat

Tier C:

- curated local real music
- ignored audio files
- checked-in sidecars only
- annotated partial windows allowed
- genre/pathology tags
- human notes required

External comparison:

- imported Serato/Rekordbox/Traktor/Engine/VirtualDJ/Lexicon/djay grids
- external grids are provenance inputs
- disagreement is reported, never silently overwritten

## Design law for the next beatgrid slice

Do not start by writing a better BPM detector.

Start by defining the beatgrid artifact substrate:

1. analysis basis
2. analysis request
3. work item / run
4. artifact metadata
5. beatgrid payload
6. diagnostics payload
7. accepted/superseded state
8. reference fixture sidecars
9. evaluation harness
10. projection summary

Then build the first analyzer behind that contract.

## Local follow-up checks

Run these checks locally when returning to the repo:

git fetch origin --prune
git checkout dev
cargo test --workspace
cargo run -p xtask -- verify
git diff --stat main...dev

Inspect these files first:

- crates/dekzer-beatgrid/src/beat_timeline.rs
- crates/dekzer-beatgrid/src/artifact.rs
- crates/dekzer-beatgrid/src/diagnostics.rs
- crates/dekzer-beatgrid/src/analysis.rs
- crates/dekzer-beatgrid/src/tempo_scan.rs
- crates/dekzer-eval/src/matching.rs
- crates/dekzer-fixtures/src/lib.rs
- contracts/beat_timeline/v1/spec.md
- contracts/fixture_run/v2/spec.md
- xtask/src/fixtures_gen.rs
- xtask/src/fixtures_check.rs
- xtask/src/verdict.rs
- xtask/src/decision_viz/mod.rs
- xtask/src/real_music.rs

## Final adoption decision

The repo is worth preserving as beatgrid harness prior art.

Adopt:

- BeatTimeline artifact idea
- sample-index references
- reference-window evaluation
- deterministic event matching
- diagnostics as explanation
- tempo-family candidates
- fixture sidecars
- fixture tiering
- decision reports
- equivalence checks

Reject:

- production use of the analyzer implementation
- integer-only period samples
- fixed-grid-only worldview
- implicit downbeat confidence
- banned vocabulary
- unbounded diagnostics payloads
- renderer-owned analysis state

The value is not the detector. The value is the shape around the detector.
