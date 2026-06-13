# Waveform Column V1

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define the proposed production V1 binary column payload for Dekzer waveform artifacts.

This document proposes replacing the older prototype 50-byte high-detail layout with alignment-safe production layouts:

- `mix_spectral_microtrace_v1`: 32-byte base columns, 52-byte high-detail columns.
- `stem_amp_microtrace_v1`: 20-byte base columns, 40-byte high-detail columns.

The extra two bytes in high-detail columns exist so JavaScript workers can decode i16 fields through aligned typed-array or structure-of-arrays paths without odd-offset i16 reads.

## LOD tiers

All LOD tiers are independently addressable.

| LOD | Bucket samples | Duration at 48 kHz | Mix column bytes | Stem column bytes | Persistence policy                               |
| --- | -------------: | -----------------: | ---------------: | ----------------: | ------------------------------------------------ |
| 0   |           8192 |       about 170 ms |               32 |                20 | persistent preview/overview                      |
| 1   |           2048 |        about 42 ms |               32 |                20 | persistent overview                              |
| 2   |            512 |      about 10.7 ms |               32 |                20 | persistent deck standard                         |
| 3   |            128 |       about 2.7 ms |               52 |                40 | generated for deck/edit/high-detail use          |
| 4   |             32 |      about 0.67 ms |               52 |                40 | on-demand/cache by default, especially for stems |

Column count:

    column_count = ceil(track_length_samples / bucket_samples)

The final partial bucket uses the Last Bucket Law below.

## Last Bucket Law

For each column:

    column_start = column_index * bucket_samples
    true_count = min(bucket_samples, track_length_samples - column_start)
    true_range = [column_start, column_start + true_count)

For the final partial column, metrics are computed only from true samples. Padding may exist only to preserve deterministic fixed-width payloads. Padding must not create nonzero spectral energy, transient flux, min/max samples, peaks, RMS, or extrema. Renderers clip final columns to the true track duration.

## Endianness and serialization

- All numeric fields are little-endian.
- No implicit struct padding.
- Field order is exact.
- Float fields must be finite before serialization.
- Normalized scalar fields are clamped before serialization.
- Signed sample fields are clamped to [-1, 1].
- Quantized sample values use the symmetric i16 range -32767 to +32767.
- Never emit -32768.

## Mix base payload, 32 bytes

Schema key: `mix_spectral_microtrace_v1`

| Offset | Field          | Type | Meaning                                     |
| -----: | -------------- | ---- | ------------------------------------------- |
|      0 | rms            | f32  | RMS amplitude, normalized [0, 1]            |
|      4 | peak           | f32  | Peak absolute amplitude, normalized [0, 1]  |
|      8 | energy_low     | f32  | Low-band energy, normalized [0, 1]          |
|     12 | energy_mid     | f32  | Mid-band energy, normalized [0, 1]          |
|     16 | energy_high    | f32  | High-band energy, normalized [0, 1]         |
|     20 | transient_flux | f32  | Transient whiteness/flux, normalized [0, 1] |
|     24 | min_sample     | f32  | Signed minimum sample, normalized [-1, 1]   |
|     28 | max_sample     | f32  | Signed maximum sample, normalized [-1, 1]   |

## Mix high-detail payload, 52 bytes

Used only for LOD 3 and LOD 4.

| Offset | Field          | Type   | Meaning                                      |
| -----: | -------------- | ------ | -------------------------------------------- |
|      0 | rms            | f32    | Same as base                                 |
|      4 | peak           | f32    | Same as base                                 |
|      8 | energy_low     | f32    | Same as base                                 |
|     12 | energy_mid     | f32    | Same as base                                 |
|     16 | energy_high    | f32    | Same as base                                 |
|     20 | transient_flux | f32    | Same as base                                 |
|     24 | min_sample     | f32    | Same as base                                 |
|     28 | max_sample     | f32    | Same as base                                 |
|     32 | mt_samples[6]  | i16[6] | Endpoint-inclusive signed microtrace samples |
|     44 | argmin_pos     | u16    | Position of true minimum within bucket       |
|     46 | min_val        | i16    | Quantized value at argmin_pos                |
|     48 | argmax_pos     | u16    | Position of true maximum within bucket       |
|     50 | max_val        | i16    | Quantized value at argmax_pos                |

## Stem base payload, 20 bytes

Schema key: `stem_amp_microtrace_v1`

Stem columns omit spectral color by default because stem color is renderer/theme context, not artifact payload.

| Offset | Field          | Type | Meaning                                     |
| -----: | -------------- | ---- | ------------------------------------------- |
|      0 | rms            | f32  | RMS amplitude, normalized [0, 1]            |
|      4 | peak           | f32  | Peak absolute amplitude, normalized [0, 1]  |
|      8 | transient_flux | f32  | Transient whiteness/flux, normalized [0, 1] |
|     12 | min_sample     | f32  | Signed minimum sample, normalized [-1, 1]   |
|     16 | max_sample     | f32  | Signed maximum sample, normalized [-1, 1]   |

## Stem high-detail payload, 40 bytes

Used only for LOD 3 and LOD 4.

| Offset | Field          | Type   | Meaning                                      |
| -----: | -------------- | ------ | -------------------------------------------- |
|      0 | rms            | f32    | Same as base                                 |
|      4 | peak           | f32    | Same as base                                 |
|      8 | transient_flux | f32    | Same as base                                 |
|     12 | min_sample     | f32    | Same as base                                 |
|     16 | max_sample     | f32    | Same as base                                 |
|     20 | mt_samples[6]  | i16[6] | Endpoint-inclusive signed microtrace samples |
|     32 | argmin_pos     | u16    | Position of true minimum within bucket       |
|     34 | min_val        | i16    | Quantized value at argmin_pos                |
|     36 | argmax_pos     | u16    | Position of true maximum within bucket       |
|     38 | max_val        | i16    | Quantized value at argmax_pos                |

## Microtrace extraction

For a bucket of B samples, six endpoint-inclusive sample positions are:

    pos[k] = floor(k * (B - 1) / 5 + 0.5), for k = 0..5

This guarantees:

- first sample represented,
- last sample represented,
- boundary continuity anchors between adjacent columns.

Pinned extrema are computed over the true sample range only:

- `argmin_pos` points to the true minimum sample within the bucket,
- `argmax_pos` points to the true maximum sample within the bucket,
- extrema positions use u16 for alignment and future schema safety.

## Quantization law

For normalized signed samples:

    clamped = clamp(normalized, -1.0, 1.0)
    rounded_magnitude = floor(abs(clamped) * 32767.0 + 0.5)
    i16_value = sign(clamped) * rounded_magnitude

Then clamp to -32767 through +32767. Never emit -32768.

If a track or component is silent, use 1.0 as the divisor for quantization while recording the honest measured peak as 0.0 in the manifest.

## Amplitude facts

Amplitude facts are PCM-derived, not FFT-derived:

- peak
- rms
- min_sample
- max_sample
- local_delta_peak contribution to transient_flux

The signed envelope guard uses min_sample/max_sample. Renderers must not substitute symmetric ±peak when drawing high-detail signed envelopes.

## Spectral profile for mix columns

Mix columns use a fixed spectral analysis profile:

- FFT size: 2048 samples
- hop size: 512 samples
- window: Hann
- analysis rate: 48 kHz for V1 unless a later decoder profile explicitly changes it
- low band: 20–250 Hz
- mid band: 250–4000 Hz
- high band: 4000–20000 Hz

Spectral color is pooled from spectral frames, not tiny PCM windows. LOD 3 and 4 use nearest-frame assignment when no spectral frame center falls inside the bucket.

## Transient preservation

For LOD 2, 3, and 4, compute local transient score directly from PCM:

    local_delta_peak = max(abs(x[i] - x[i - 1])) over the true column range

The first sample of a column uses the previous global sample, not zero, except for global sample zero. This preserves attacks that cross column boundaries.

Final transient value:

    transient_flux = max(pooled_spectral_flux, normalized_local_delta_peak)

LOD 0 and LOD 1 receive transient values through downsampling, not PCM re-analysis.

## Downsampling laws

LOD 0 and LOD 1 are downsampled from LOD 2.

For mix columns:

- peak: max
- rms: RMS-pool
- energy_low/mid/high: RMS-pool
- transient_flux: max
- min_sample: min
- max_sample: max

For stem columns:

- peak: max
- rms: RMS-pool
- transient_flux: max
- min_sample: min
- max_sample: max

LOD 3 and LOD 4 are direct-from-PCM outputs. Do not derive LOD 3 or 4 from lower-resolution tiers.

## Renderer expansion contract

Renderers may decode columns into a structure-of-arrays cache. The binary format remains compact; the worker/GPU representation may be expanded for performance.

The artifact carries no stem role color. Stem color is provided by waveform surface context.

## Validation requirements

Stem column schemas are documented so the format does not block future stem lanes. They are not implied as the first
implementation path; the first implementation may be full-mix only until stem target, alignment, and playback contracts
are accepted.

The isolated spike for this schema must include exact fixture names:

- `silence_48k_10s_mono.wav`
- `single_sample_spike_center_48k.wav`
- `single_sample_spike_lod_boundary_48k.wav`
- `boundary_spike_pair_48k.wav`
- `asymmetric_clipped_burst_48k.wav`
- `stereo_phase_inverted_bass_48k.wav`
- `transient_click_train_48k.wav`
- `long_fixture_10min_48k.wav`

The isolated spike for this schema must prove:

- deterministic binary output for repeated runs,
- no amplitude guard violation,
- no spline overshoot outside min/max guard,
- single-sample spike localization at every LOD,
- transient_flux nonzero at every LOD for spike fixtures,
- asymmetric clipped burst follows min/max guard,
- boundary spike pair does not invent a smooth loop,
- artifact byte sizes match schema math,
- LOD4 size confirms on-demand/cache policy.
