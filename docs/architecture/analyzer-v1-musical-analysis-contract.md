---
status: contract
owner: product-architecture
review: required-before-implementation
---

# Analyzer V1 Musical Analysis Contract

## Baseline Inspected

This contract was written after Analyzer V0 technical-facts work reached commit
`d6ba9d715c14b46a09fc14024d5c01f6af62f811`.

Read-first repo files:

- `docs/architecture/analyzer-v0-contract.md`
- `crates/analyzer-core/Cargo.toml`
- `crates/analyzer-core/src/lib.rs`
- `crates/analyzer-core/tests/wav_fixtures.rs`
- `Cargo.toml`
- `crates/library-store-sqlite/Cargo.toml`

External sources inspected:

- `https://docs.rs/stratum-dsp/latest/stratum_dsp/`
- `https://docs.rs/stratum-dsp/latest/stratum_dsp/fn.analyze_audio.html`
- `https://github.com/HLLMR/stratum-dsp`

## Executive Decision

Analyzer V1 is the first Dekzer musical-analysis layer.

Analyzer V1 must support:

- BPM detection;
- key detection;
- beatgrid / beat tracking.

`stratum-dsp` is the preferred V1 musical-analysis adapter candidate. It is not a generic fallback and it is not yet a
committed runtime dependency.

The intended shape is:

```text
Dekzer-owned V1 contract
-> Dekzer-owned adapter boundary
-> stratum-dsp execution
-> Dekzer-owned result normalization, validation, basis, and artifact policy
```

`stratum-dsp` may compute candidate musical-analysis results. Dekzer owns whether those results become advisory facts,
durable artifacts, workflow projections, or rejected evidence.

## Candidate Summary

Current public metadata observed:

- crate: `stratum-dsp`
- Rust module: `stratum_dsp`
- observed version: `1.0.0`
- license: `MIT OR Apache-2.0`
- advertised scope: DJ-oriented BPM detection, key detection, and beat tracking
- main API: `analyze_audio(&samples, sample_rate, AnalysisConfig::default())`
- expected audio input: mono normalized `f32` samples
- normal dependencies include `log`, `rayon`, `rustfft`, `serde`, `serde_json`, and `symphonia`
- optional dependency includes `ort`
- upstream repository: `HLLMR/stratum-dsp`

The upstream README advertises BPM confidence scoring, key notation including numerical DJ notation, beat grid generation,
normalization and silence trimming, batch processing, benchmark claims, and validation claims over a 155-track DJ corpus.
Those claims are useful adoption evidence. They are not Dekzer product facts until Dekzer reproduces validation under a
Dekzer-owned corpus and basis policy.

## Non-Negotiable Boundary

Analyzer V1 does not rewrite Analyzer V0.

Analyzer V0 remains:

- self-owned;
- WAV-only for slice 1;
- technical sample facts only;
- independent of `stratum-dsp`;
- independent of Symphonia in `analyzer-core`;
- independent of persistence, UI, and generated boundary output.

Analyzer V1 may introduce a new musical-analysis adapter crate or module later, but it must not let `stratum-dsp` own:

- target selection;
- playable-media ownership;
- content identity;
- source-file provenance;
- canonical JSON;
- artifact manifests;
- SQLite persistence;
- boundary protocol semantics;
- renderer/UI semantics;
- whether an upstream result becomes authoritative product evidence.

## Current Contract Conflicts With Analyzer V0

`stratum-dsp` must not be added to `analyzer-core` V0.

Reasons:

- Analyzer V0 excludes BPM, key, and beatgrid.
- Analyzer V0 forbids Symphonia in `analyzer-core`.
- `stratum-dsp` depends on Symphonia by default.
- Analyzer V0 forbids optional ML, ONNX Runtime, native, and model-file dependency paths.
- `stratum-dsp` has an optional `ort` dependency path.
- Analyzer V0 computes deterministic `f64` technical sample facts.
- `stratum-dsp` appears to consume mono normalized `f32` samples.
- Analyzer V0 has no musical-analysis persistence, artifact kind, basis fingerprint, or manifest contract.

These conflicts are not a rejection of `stratum-dsp`. They mean V1 needs a separate adapter boundary and validation path.

## V1 Adapter Architecture

The future V1 adapter must preserve Dekzer ownership:

1. Dekzer selects the target.
2. Dekzer resolves bytes and provenance.
3. Dekzer decodes audio or receives decoded PCM from a reviewed decode path.
4. Dekzer applies a reviewed channel mixdown policy.
5. Dekzer applies a reviewed sample-rate and normalization policy.
6. The adapter calls `stratum-dsp` with the prepared mono normalized `f32` input.
7. The adapter maps upstream output into Dekzer-owned result structs.
8. Dekzer records confidence, warnings, policy version, adapter version, upstream crate version, and feature flags.
9. Dekzer validation decides whether the result is advisory, accepted evidence, rejected evidence, or blocked.
10. Only a later persistence contract may decide what becomes durable.

The adapter must not expose raw upstream structs as Dekzer product contracts. It must translate them into explicit Dekzer
vocabulary.

## V1 Planning Result Vocabulary

Planning vocabulary for future result structs:

- `bpm`
- `bpm_confidence`
- `tempo_candidates`
- `tempo_range`
- `tempo_warning`
- `key`
- `key_notation`
- `key_confidence`
- `key_warning`
- `beatgrid`
- `beat_positions`
- `first_beat_position`
- `downbeat_position`
- `grid_stability`
- `grid_warning`
- `analysis_confidence`
- `analysis_policy`
- `adapter_key`
- `adapter_version`
- `upstream_crate_name`
- `upstream_crate_version`
- `upstream_feature_flags`

This vocabulary is planning language. It does not freeze generated protocol types, SQLite columns, artifact kinds, or JSON
schema.

## Basis Requirements

A V1 musical-analysis basis must include:

- target kind and target id, once persistence is introduced;
- content hash / attachment provenance, once persistence is introduced;
- selected source-file occurrence provenance, once byte access is source-backed;
- decoder policy;
- channel mixdown policy;
- sample-rate policy;
- normalization policy;
- mono conversion policy;
- `stratum-dsp` crate version;
- `stratum-dsp` feature flags;
- `stratum-dsp` analysis config;
- Dekzer adapter version;
- preprocessing policy;
- deterministic execution policy;
- whether Rayon or parallelism is enabled, disabled, or irrelevant for this path;
- whether ML / `ort` is disabled;
- validation profile used to accept, reject, or mark results advisory.

If any basis field cannot be recorded deterministically, the result must remain non-durable advisory output.

## Dependency And Feature Gates

Before code adoption, these gates must pass:

1. License confirmation for `stratum-dsp` and normal transitive dependencies.
2. Symphonia MPL-2.0 decision.
3. Duplicate Symphonia-line review, because Dekzer currently uses Symphonia directly and `stratum-dsp` depends on a
   Symphonia line as well.
4. Rayon and parallelism determinism review.
5. `serde` / `serde_json` review to ensure upstream serialization does not own Dekzer canonical JSON policy.
6. `ort` / ML path must remain disabled and out of scope unless separately ratified.
7. No native, ONNX Runtime, model-file, network, or runtime-download dependency may enter the V1 default path.
8. Feature flags must be explicit and recorded in basis.
9. Cargo dependency placement must not put `stratum-dsp` in `analyzer-core` V0.

A future implementation may use a separate adapter crate such as `analyzer-musical-stratum` or a similarly precise name.
That crate must depend on the V1 adapter contract, not on product UI needs.

## Determinism And Validation Requirements

Dekzer must validate V1 results before treating them as product facts.

Required validation classes:

- synthetic tempo fixtures;
- synthetic key fixtures where musically meaningful;
- synthetic beatgrid fixtures;
- curated local DJ track corpus;
- comparison against at least one trusted external reference;
- half-tempo cases;
- double-tempo cases;
- 2/3 and 3/2 tempo confusion cases;
- triplet feel;
- weak percussion;
- long intros;
- long outros;
- live tempo drift;
- ambiguous key;
- percussive/no-tonality tracks;
- low-confidence cases;
- unsupported/short/silent audio cases.

Validation output must classify results as:

- accepted;
- advisory;
- rejected;
- blocked;
- inconclusive.

No V1 result may create automatic cue, phrase, transition, or performance-path claims.

## Persistence Deferral

Analyzer V1 may describe future persisted musical-analysis artifacts, but this document does not authorize persistence.

Before persistence, these contracts must be reviewed:

- work subject or musical-analysis target contract;
- artifact kind contract;
- basis fingerprint contract;
- manifest schema;
- target ownership review;
- migration plan;
- boundary protocol shape;
- artifact retention and supersession policy;
- failure and advisory-state policy.

Future ownership should follow the existing Analyzer direction:

```text
playable_media_id
  + attachment_id/content_hash
  + component_role=full_mix
  + analyzer_basis_fingerprint
  -> work_run
  -> musical-analysis artifact manifest
  -> file-backed artifacts if needed
```

`source_file_id` remains byte-access provenance, not durable artifact ownership.

## Rejection Cases

Reject any future V1 implementation that:

- adds `stratum-dsp` directly to `analyzer-core` V0;
- weakens Analyzer V0 stop rules;
- enables optional `ort` / ML features by default;
- treats upstream BPM, key, or beatgrid output as canonical without Dekzer validation;
- writes musical-analysis facts to SQLite before contracts are approved;
- adds renderer UI before headless validation exists;
- hides upstream dependency versions or feature flags from basis;
- lets upstream JSON or serde output become Dekzer canonical JSON;
- uses source-file occurrence identity as durable musical-analysis ownership;
- claims cue, phrase, or transition intelligence from BPM/key/beatgrid alone.

## Stop Rules For Future Implementation Prompts

Stop before implementation if a prompt would:

- modify `analyzer-core` to depend on `stratum-dsp` before adapter contract approval;
- add `stratum-dsp` without Symphonia/MPL review;
- enable optional ML or `ort`;
- treat V1 musical-analysis output as canonical product facts without Dekzer validation;
- start renderer UI work before headless/CLI validation;
- start persistence before artifact, basis, manifest, target, migration, and boundary contracts;
- add generated boundary output before protocol review;
- add SQLite migrations before persistence review;
- route large analysis payloads through normal JSON IPC;
- depend on cloud, streaming providers, or network analysis.

When a future implementation prompt conflicts with this contract, implementation must stop and request a contract update
instead of writing code.

## Recommendation

Adopt this decision path:

1. Keep Analyzer V0 technical facts self-owned.
2. Treat `stratum-dsp` as the preferred V1 musical-analysis adapter candidate.
3. Run a no-persistence adapter spike only after this contract is reviewed.
4. Keep optional ML disabled.
5. Keep persistence, UI, and generated boundary work behind separate contracts.
6. Build Dekzer-owned validation before promoting BPM, key, or beatgrid results into product workflows.

Suggested first implementation after this contract:

```text
Create a no-persistence V1 adapter spike that depends on stratum-dsp in a separate adapter crate, feeds controlled mono
normalized f32 fixtures into analyze_audio, maps BPM/key/beatgrid output into Dekzer-owned structs, disables optional ML,
and records upstream version/config/feature information in a typed basis object. Do not touch SQLite, generated boundary,
renderer UI, or Analyzer V0 technical facts.
```
