---
status: spike-evidence
owner: product-architecture
review: required-before-adoption
---

# stratum-dsp Adapter Spike Results

## Inspection Ref

Inspected repository ref: `6bfafb2a0177871405097fa6637cac9b342f233c`.

This note records evidence from the isolated `crates/analyzer-musical-stratum` spike. It does not authorize persistence,
renderer UI, generated boundary contracts, migrations, CLI workflow, or product authority for musical-analysis output.

## Dependency Shape

The spike crate is a workspace member at `crates/analyzer-musical-stratum`.

Its exact direct dependency declaration is:

```toml
stratum-dsp = { version = "=1.0.0", default-features = false }
```

Current `cargo tree -p analyzer-musical-stratum` shows:

```text
analyzer-musical-stratum
-> stratum-dsp v1.0.0
   -> log
   -> rayon
   -> rustfft
   -> serde
   -> serde_json
   -> symphonia v0.5.5
```

`cargo tree -i stratum-dsp` shows `stratum-dsp v1.0.0` is pulled only by
`analyzer-musical-stratum`.

## Symphonia Lines

`stratum-dsp v1.0.0` pulls `symphonia v0.5.5` transitively, even with the spike dependency using
`default-features = false`.

The existing workspace media-probe line remains:

```toml
symphonia = { version = "0.6.0", default-features = false, features = ["aac", "aiff", "alac", "flac", "isomp4", "mp3", "ogg", "pcm", "vorbis", "wav"] }
```

That line belongs to `crates/library-store-sqlite/Cargo.toml`. `Cargo.lock` currently contains both `symphonia 0.5.5`
and `symphonia 0.6.0`.

## ML Runtime Check

No `ort`, `ort-sys`, `onnxruntime`, `onnxruntime-sys`, `tract-core`, or `tract-onnx` package entries were found in
`Cargo.lock`.

The adapter basis records:

- `default=[]`
- `default-features=false`
- `ml=disabled`
- `ort=disabled`

## Fixture Behavior

Silence fixture:

- input: four seconds of mono normalized `f32` zeros at 44,100 Hz;
- current pinned stratum-dsp 1.0.0 spike observation: silence produced a positive BPM candidate around `40.0`
  with `bpm_confidence = 0.0`; this is adoption evidence, not a Dekzer correctness requirement;
- adapter behavior: status remains `Inconclusive`;
- warnings include the non-authoritative spike warning, silent input, and low BPM confidence.

120 BPM pulse fixture:

- input: sixteen seconds of synthetic mono click pulses at 120 BPM and 44,100 Hz;
- upstream behavior observed: BPM candidate around `118.6` with low BPM confidence;
- adapter behavior: status remains advisory spike evidence, not product authority;
- warnings keep low-confidence behavior visible.

The current tests also include a 90 BPM pulse fixture. It is not treated as a correctness proof; it exists to ensure
surprising or low-confidence pulse output remains non-authoritative and warning-bearing.

## Current Status

Current status: spike evidence only.

The adapter is limited to controlled mono normalized `f32` fixtures. It does not decode source files, write artifacts,
persist results, generate boundary protocol, drive UI, or modify Analyzer V0.

## Blockers Before Adoption

- Dekzer-owned validation corpus and acceptance policy do not exist yet.
- Duplicate Symphonia lines require license, security, and maintenance review.
- The current pinned stratum-dsp 1.0.0 spike showed silence can produce a positive upstream BPM candidate with zero
  confidence; treat this as an adoption concern, not a Dekzer invariant.
- Pulse fixtures currently produce low-confidence tempo evidence; they are not correctness proof.
- Rayon and parallelism determinism need review beyond one-process fixture repeatability.
- Decoder, mixdown, resampling, and normalization policies are not validated outside controlled fixtures.
- Key and beatgrid validation are not covered by a real corpus or trusted reference comparison.
- Optional ML / `ort` remains disabled and out of scope unless separately ratified.
- Persistence, artifact basis fingerprints, manifest schema, and boundary protocol are separately gated.

## Next Validation Requirements

- Expand synthetic tempo fixtures for 90, 120, 128, half-tempo, double-tempo, 2/3, 3/2, triplet feel, weak percussion,
  long intro, long outro, and drift cases.
- Add synthetic key and no-tonality fixtures where musically meaningful.
- Validate beat positions and grid stability against known fixture timings without promoting them to product facts.
- Run repeated analyses across process boundaries and CI environments.
- Compare against at least one trusted external reference on a curated local DJ corpus.
- Keep lockfile and cargo-tree gates for adapter isolation, duplicate Symphonia lines, and absence of optional ML/native
  runtime packages.
- Define a Dekzer-owned validation profile before any result can leave spike evidence status.
