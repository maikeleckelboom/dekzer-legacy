---
status: contract
owner: product-architecture
review: required-before-implementation
---

# Analyzer V0 Contract

## Baseline Inspected

This contract was written against `dev` at `5a4cbcf476ae33d841d53aebb5bbad628e1b1476`.

The current repo has source-file inspection, BLAKE3 content evidence, media probing, attachment materialization,
playable-media promotion, exact-content identity candidates, exact-content identity decisions, and generic `work_*`
artifact tables. It does not have an analyzer crate, analyzer CLI, analyzer persistence schema, analyzer boundary
protocol, waveform artifact schema, stem artifact schema, or renderer analyzer surface.

This document is the Analyzer V0 doctrine/specification gate. It authorizes no implementation by itself.

## Product Boundary

Dekzer Analyzer V0 is a local-first, headless, deterministic audio analyzer contract.

The first useful proof is CLI/test oriented. It analyzes one local audio file and emits a deterministic JSON result that
can be checked by golden fixtures. It is not a renderer feature, not Library Workstation polish, not a playback engine,
not a persistence migration, and not a generated boundary contract.

Analyzer V0 answers one question: given one supported local WAV file and one fixed analyzer policy, what deterministic
technical sample facts are reported?

Analyzer V0 must be usable without:

- Electron renderer code;
- normal JSON IPC from renderer to Rust;
- SQLite writes;
- generated TypeScript boundary output;
- network access;
- cloud, catalog, or streaming providers;
- real-time playback.

## First-Slice Input Policy

Slice 1 is WAV-only.

The only authorized first-slice input path is hound-generated fixtures and hound-readable WAV input. The first analyzer
core must not depend on Symphonia.

Symphonia is already present in `library-store-sqlite` for current source-file media probing. That does not authorize
Symphonia for analyzer-core. Symphonia's MPL-2.0 license requires a separate product/legal/dependency review before
analyzer-core may depend on it.

The rule is:

```text
Analyzer V0 slice 1 remains WAV-only until the Symphonia license decision is explicitly ratified.
```

Slice 1 accepts only integer PCM WAV that the selected hound-based implementation can read deterministically. IEEE float
WAV, compressed WAV codecs, RF64/WAVE64, multi-container audio, malformed WAV, and non-WAV inputs are rejected.

## Analyze-One-File JSON Envelope

The analyze-one-file command returns exactly one JSON envelope. This is contract vocabulary, not generated boundary
protocol.

Successful envelope:

```text
{
  "envelope_schema": "dekzer.analyzer.analyze_one_file.v0",
  "status": "completed",
  "payload_hash": "sha256:<64 lowercase hex chars>",
  "payload": {
    "payload_schema": "dekzer.analyzer.technical_sample_facts.v0",
    "analyzer": {
      "engine_key": "dekzer_analyzer",
      "engine_version": "<semver-or-test-version>",
      "policy_key": "technical_sample_facts_v0",
      "policy_version": "0"
    },
    "basis": {
      "basis_schema": "dekzer.analyzer_basis.v0",
      "basis_hash": "sha256:<64 lowercase hex chars>",
      "input_policy": "wav_hound_slice1",
      "decoder_key": "hound",
      "decoder_version": "<crate-version>",
      "normalization_policy": "integer_pcm_full_scale_v0",
      "numeric_policy": "f64_fixed_order_decimal9_v0",
      "facts_schema": "technical_sample_facts_v0",
      "content_hash_algorithm": "blake3",
      "content_hash_value": "<64 lowercase hex chars>",
      "byte_length": 0,
      "sample_rate_hz": 44100,
      "channel_count": 2,
      "bits_per_sample": 16,
      "sample_format": "pcm_integer",
      "frame_count": 0
    },
    "facts": {
      "frame_count": 0,
      "channel_count": 2,
      "sample_peak": 0.000000000,
      "peak_dbfs": null,
      "rms": 0.000000000,
      "rms_dbfs": null,
      "crest_factor": null,
      "dc_offset": 0.000000000,
      "clipping_count": 0,
      "clipping_ratio": 0.000000000,
      "stereo_balance": null,
      "stereo_phase_correlation": null
    }
  },
  "failure": null
}
```

Failed envelope:

```text
{
  "envelope_schema": "dekzer.analyzer.analyze_one_file.v0",
  "status": "failed",
  "payload_hash": null,
  "payload": null,
  "failure": {
    "failure_kind": "unsupported_sample_format",
    "message": "integer PCM WAV is required for analyzer V0 slice 1",
    "retryable": false
  }
}
```

`payload_hash` is the SHA-256 hash of the canonical UTF-8 JSON bytes of the `payload` object only. The hash field itself
is not part of the hash input. Failed runs do not produce an analysis payload hash.

Paths, timestamps, host names, source ids, thread counts, and process-local details must not enter the canonical
payload. If a CLI later reports local diagnostic paths, those paths must live outside the hashed payload.

## Deterministic Basis Fields

The headless one-file basis for slice 1 includes only deterministic byte, decoder, policy, and WAV technical fields:

- `basis_schema`;
- `input_policy`;
- `decoder_key`;
- `decoder_version`;
- `normalization_policy`;
- `numeric_policy`;
- `facts_schema`;
- `content_hash_algorithm = blake3`;
- `content_hash_value`;
- `byte_length`;
- `sample_rate_hz`;
- `channel_count`;
- `bits_per_sample`;
- `sample_format`;
- `frame_count`.

The basis excludes:

- absolute path;
- source root path;
- `source_file_id`;
- `source_id`;
- `playable_media_id`;
- local clock timestamps;
- machine identity;
- OS name/version;
- CPU feature detection;
- worker count;
- cache state;
- renderer state;
- labels or display names.

For the headless V0 proof, `basis_hash = sha256(canonical_json_utf8(basis_without_basis_hash))`.

For future persistence, this headless basis is not automatically sufficient. The persisted basis fingerprint contract
must be reviewed before schema or boundary work starts.

## Canonical JSON Rules

Canonical JSON is required for stable basis and payload hashes.

Rules:

- Output is UTF-8 with no BOM.
- Objects sort keys lexicographically by UTF-8 byte order.
- No insignificant whitespace is emitted.
- All object keys are lower_snake_case ASCII.
- Enum and status strings are lower_snake_case ASCII.
- Schema identifiers are stable lowercase ASCII strings.
- All fields defined by the active schema are present.
- Absence is represented as JSON `null`, not by omitting keys.
- Unknown extension keys are forbidden unless the schema string changes.
- Arrays preserve the schema-defined semantic order. Channel arrays, if introduced later, must be ordered by zero-based
  channel index.
- Integers are base-10 JSON numbers with no leading plus sign and no leading zeroes except `0`.
- Floating-point reported facts are finite JSON numbers in fixed decimal notation with exactly 9 digits after the decimal
  point.
- Floating-point canonicalization rounds to nearest at the ninth decimal place, ties to even.
- Negative zero serializes as `0.000000000`.
- Scientific notation is forbidden for reported facts.
- `NaN`, `Infinity`, and `-Infinity` are forbidden. Undefined numeric-domain results use `null`.
- Strings use normal JSON escaping for quotation mark, reverse solidus, and control characters. Non-control Unicode is
  emitted as UTF-8, but Analyzer V0 schema keys and enum values are ASCII only.

The canonical payload hash input bytes are exactly:

```text
canonical_json_utf8(payload_object)
```

No transport wrapper, terminal newline, log prefix, pretty-print whitespace, or `payload_hash` field is part of the
payload hash input.

## Numeric And Sample Policy

All sample traversal is deterministic:

- read frames in file order;
- read channels in file channel order;
- use one fixed reduction order;
- do not parallelize accumulation unless the implementation proves the same reduction tree and byte-identical output.

Intermediate computation uses `f64` for normalized samples and accumulators. Integer counts use integer types large
enough for the supported file size.

### Integer PCM normalization

Signed integer PCM with `n` valid bits is normalized as:

```text
normalized = sample_integer / 2^(n - 1)
```

This maps the most negative sample to `-1.0` and the most positive sample to `1.0 - 2^(1 - n)`.

Unsigned integer PCM, if accepted by the hound reader for the WAV bit depth, is centered before scaling:

```text
centered = raw_unsigned - 2^(n - 1)
normalized = centered / 2^(n - 1)
```

The valid bit depth, not the storage container width, defines `n`. For example, 24-bit PCM stored in a 32-bit integer
container uses `n = 24`.

### Float samples

Float WAV is rejected in slice 1 with `failure_kind = unsupported_sample_format`.

Float acceptance requires a later contract because NaN, infinity, denormal, over-full-scale, and clipping semantics must
be reviewed before float samples can enter deterministic analyzer output.

### dBFS reference

The dBFS reference is normalized full scale:

```text
0 dBFS == absolute normalized amplitude 1.0
dbfs = 20 * log10(value)
```

If `value == 0`, the dBFS field is `null`. Analyzer V0 must not emit `-Infinity`.

### Clipping threshold

For integer PCM, a sample is clipped if its original decoded integer value is exactly the minimum or maximum value for
the valid bit depth. For unsigned PCM, a sample is clipped if the raw value is exactly `0` or `2^n - 1`.

`clipping_count` counts individual samples, not frames.

`clipping_ratio = clipping_count / (frame_count * channel_count)`. If there are zero samples, `clipping_ratio` is
`0.000000000`.

### Stereo edge cases

`stereo_balance` and `stereo_phase_correlation` are defined only for exactly two channels.

For mono and for more than two channels, both fields are `null`.

For silence or near-zero-energy stereo input, both fields are `null`. The near-zero threshold is:

```text
channel_rms < 1e-12
```

## Technical Sample Facts For Slice 1

Slice 1 may compute only these facts:

| Field                      | Definition                                                                                               |
| -------------------------- | -------------------------------------------------------------------------------------------------------- |
| `frame_count`              | Number of complete sample frames in the WAV data.                                                        |
| `channel_count`            | Number of channels declared by the accepted WAV input.                                                   |
| `sample_peak`              | Maximum absolute normalized sample across all channels and frames.                                       |
| `peak_dbfs`                | `20 * log10(sample_peak)`, or `null` when `sample_peak == 0`.                                            |
| `rms`                      | Square root of the mean squared normalized sample across all channels and frames.                        |
| `rms_dbfs`                 | `20 * log10(rms)`, or `null` when `rms == 0`.                                                            |
| `crest_factor`             | `sample_peak / rms`, or `null` when `rms == 0`.                                                          |
| `dc_offset`                | Arithmetic mean of normalized samples across all channels and frames.                                    |
| `clipping_count`           | Count of samples at the integer full-scale minimum or maximum for the bit depth.                         |
| `clipping_ratio`           | `clipping_count / total_sample_count`, with zero samples producing `0.000000000`.                        |
| `stereo_balance`           | For stereo only, `(rms_right - rms_left) / (rms_right + rms_left)`, or `null` for near-zero energy.      |
| `stereo_phase_correlation` | For stereo only, `sum(left * right) / sqrt(sum(left^2) * sum(right^2))`, or `null` for near-zero energy. |

The first implementation slice must explicitly defer:

- waveform bucket summaries;
- spectral summaries;
- loudness, LUFS, and true peak;
- BPM;
- key;
- beatgrid;
- phrase/cue suggestions;
- stems;
- ML/AI;
- GPU;
- realtime playback;
- renderer UI;
- SQLite persistence.

## Fixture Generation Policy

Golden fixtures are generated, deterministic, and auditable. Fixture audio files and expected JSON facts must record:

- generator name;
- generator version;
- PRNG algorithm when random/noise fixtures are used;
- explicit PRNG seed when random/noise fixtures are used;
- sample rate;
- channel count;
- bit depth;
- sample format;
- duration or exact frame count;
- waveform formula or fixture recipe;
- expected facts;
- tolerance per expected fact.

Deterministic non-random fixtures are preferred for slice 1: silence, constant DC, sine, impulse, hard-clipped samples,
left-only stereo, right-only stereo, in-phase stereo, and polarity-inverted stereo.

If fixed-seed noise fixtures are introduced, the only authorized PRNG for this contract is `splitmix64_v1` with default
seed `0x44454b5a45525f30`. Any other noise PRNG or seed requires a contract update. The fixture metadata must still
record the actual seed used.

Expected fact tolerances must be tight and explicit. Integer facts use exact equality. Floating-point facts use absolute
tolerances unless a fixture explains why relative tolerance is required.

## Failure Model

Analyzer V0 failures are typed and deterministic. A failure returns a failed envelope with `payload = null`,
`payload_hash = null`, and one `failure` object.

Allowed initial failure kinds:

- `input_not_found`;
- `input_not_file`;
- `input_unreadable`;
- `unsupported_container`;
- `unsupported_codec`;
- `unsupported_sample_format`;
- `unsupported_bit_depth`;
- `malformed_wav`;
- `decode_error`;
- `basis_changed`;
- `numeric_domain_error`;
- `internal_error`.

No failed run may emit partial facts as if they were authoritative. No failed run may write analyzer artifacts. Future
worker integration may map failures to work-run outcomes, but that mapping is deferred with persistence.

## Durable Target Model

Future persisted analyzer artifacts target playable media, not source-file occurrences.

Current durable roles:

- `source_file` is byte-access provenance only.
- `content_attachments` provide BLAKE3 content identity.
- `source_file_attachment_links` provide occurrence routes to bytes.
- `playable_media` is the durable artifact owner for future audio analyzer artifacts.
- `track_identity_candidates` are not analyzer artifact owners.
- `track_identity_decisions` are not analyzer artifact owners.
- Track identity candidates and decisions are not canonical tracks.

The future full-mix ownership chain is:

```text
playable_media_id
  + attachment_id/content_hash
  + component_role=full_mix
  + analyzer_basis_fingerprint
  -> work_run
  -> analysis artifact manifest
  -> file-backed artifacts later
```

`source_file_id` may appear in run provenance to explain which occurrence supplied bytes. It must not be the durable
owner of analyzer artifacts.

If one occurrence disappears and another current `source_file_attachment_links` row can route to the same attachment
content, the target identity remains the playable media plus content/basis identity. Occurrence availability is not
artifact ownership.

## Artifact Ownership Model

Analyzer artifacts must eventually be recorded as analysis artifacts owned by the target model above. The future artifact
manifest must describe:

- target identity;
- content identity;
- component role;
- basis fingerprint;
- analyzer engine key/version;
- decoder policy;
- normalizer policy;
- facts schema or artifact schema;
- payload hash;
- file-backed artifact entries when payloads are too large for inline storage;
- run provenance, including the source-file occurrence used to read bytes.

This pass does not add the artifact kind, manifest schema, file layout, artifact DAG, claim policy, or migration.

## Persistence Deferral

Analyzer persistence is explicitly deferred.

This pass does not authorize:

- SQLite migrations;
- generated boundary contract changes;
- analyzer persistence;
- new Rust analyzer crates;
- CLI code;
- fixtures;
- dependencies;
- renderer UI changes;
- Library Workstation implementation changes.

Before persistence, these contracts must be reviewed:

- new work subject or analysis target contract;
- new artifact kind contract;
- basis fingerprint contract;
- artifact manifest contract;
- migration plan;
- boundary protocol shape.

## Out Of Scope

Out of scope for Analyzer V0 slice 1:

- non-WAV decoding;
- Symphonia in analyzer-core;
- float WAV acceptance;
- waveform buckets or tiles;
- spectral features;
- loudness, LUFS, and true peak;
- BPM, key, beatgrid, phrase, and cue detection;
- stem separation or stem playback;
- ML or AI inference;
- GPU compute;
- realtime playback;
- renderer UI;
- Library Workstation polish;
- normal JSON IPC for large analysis data;
- SQLite persistence;
- generated TypeScript boundary output;
- cloud, catalog, streaming, or network-backed analysis.

## Stop Rules For Future Implementation Prompts

Stop before implementation if a prompt would:

- touch renderer UI;
- polish Library Workstation surfaces;
- add migrations before the persistence contract is reviewed;
- add generated boundary output before Rust protocol review;
- route large waveform or spectral data through normal JSON IPC;
- use `source_file_id` as durable analyzer artifact owner;
- treat track identity candidates as canonical tracks;
- treat automatic track identity decisions as canonical tracks;
- add BPM, key, beatgrid, stems, ML, GPU, or realtime playback;
- add GPL, LGPL, AGPL, non-commercial, or unclear dependencies;
- add Symphonia to analyzer-core before the MPL-2.0 decision is explicitly ratified;
- claim LUFS or true-peak compliance without fixture/reference validation;
- depend on cloud or streaming providers;
- write analyzer facts into SQLite before the work subject, artifact kind, basis fingerprint, manifest, migration, and
  boundary protocol contracts are reviewed.

When a future implementation prompt conflicts with this contract, the implementation must stop and request a contract
update instead of writing code.
