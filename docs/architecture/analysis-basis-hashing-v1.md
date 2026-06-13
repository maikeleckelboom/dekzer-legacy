# Analysis Basis Hashing V1

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define how analysis artifacts compute a stable basis hash so waveform, stem, beatgrid, loudness, and future artifacts can be invalidated deterministically.

## Decision

V1 basis hashing uses canonical JSON encoded as UTF-8 and hashed with SHA-256 unless the current repo already has a stronger canonical hashing utility accepted by human review.

Canonical JSON rules:

- object keys sorted lexicographically,
- no insignificant whitespace,
- stable array order,
- IDs serialize as stable strings, not lossy numbers or process-local objects,
- include `basis_schema_version` as an explicit top-level field,
- UTF-8 encoding,
- finite numbers only,
- no locale-dependent formatting,
- no timestamps unless they are part of durable input identity,
- no paths unless path identity is explicitly the subject of the artifact.

Hash:

    basis_hash = sha256(canonical_json_utf8)

Canonical hash fixtures must prove that equivalent object key ordering produces the exact same canonical JSON bytes and
the exact same hash. Fixture names should make the equivalence explicit, such as
`basis_equivalent_key_order_a.json`, `basis_equivalent_key_order_b.json`, and
`basis_equivalent_key_order.sha256`.

## Basis includes

For waveform artifacts, basis includes:

- target identity using current vocabulary,
- target kind, such as `playable_media_full_mix` or future `audio_component`,
- `playable_media_id` for V0 full-mix targets,
- `content_attachment_id` or accepted content identity,
- input content hash,
- input evidence/provenance ids needed to explain bytes,
- source range identity if the target is a subrange,
- decoder key/version/profile,
- resampler key/version/profile,
- analysis sample rate,
- channel/downmix strategy,
- waveform engine key/version,
- artifact format version,
- column schema key,
- spectral profile version for mix schemas,
- transient profile version,
- alignment transform hash for stems/subranges,
- upstream artifact dependency hashes where relevant.

For stem-derived artifacts, basis also includes:

- stem set id or durable stem-set identity,
- stem member id or role/ordinal identity,
- stem source kind,
- separation artifact id and basis hash if generated,
- separation model key/version/hash if generated,
- dedicated stem content identity if supplied by files,
- alignment profile id and transform hash.

## Basis excludes

Basis excludes:

- parent mix peak absolute value,
- stem set peak absolute value,
- component peak measured output value,
- UI color,
- renderer theme,
- waveform surface orientation,
- storage path,
- descriptor URL,
- tile last accessed timestamp,
- cache state,
- progress state,
- work lease owner,
- user-visible label strings.

## Normalization decision

Measured peak values are not basis inputs.

The waveform payload is normalized by deterministic analysis over the input signal. The measured component peak is an output of that analysis and is recorded in the manifest. It is not an external input to the basis.

Manifest fields may include:

- component_peak_abs,
- parent_mix_peak_abs if known,
- stem_set_peak_abs if known.

Only component_peak_abs is required to interpret the artifact’s own normalized signal facts. Parent and stem-set peaks are rendering hints used for comparable amplitude policies.

Updating `parent_mix_peak_abs` or `stem_set_peak_abs` must not invalidate a stem waveform artifact.

## Rendering hint model

Amplitude policy is renderer context:

- isolated detail: display each component at its own normalized detail,
- mix comparable: scale by component peak versus parent mix peak if known,
- stem set comparable: scale by component peak versus stem set peak if known,
- user gain: renderer/session scale.

If a comparable hint is unavailable, the renderer falls back to isolated detail until the hint arrives. Arrival of the hint updates presentation only.

## Dependency ordering

Stem waveform analysis must not be forced to wait for mix waveform analysis solely to get parent mix peak. If parent mix peak is required for a UI mode, it is obtained as metadata/hint later.

Actual hard dependencies remain:

- content bytes available,
- decoder profile available,
- alignment transform available for stems/subranges,
- upstream separation/playback artifact available when the waveform input is generated output.

## Source occurrence law

The basis may record which source occurrence supplied bytes for diagnostics, but the artifact remains owned by the accepted target and content identity. If another source occurrence supplies the same accepted content identity, the artifact remains valid.

## Stale and supersession rules

An artifact becomes stale when a basis input changes:

- content hash changes,
- decoder/resampler profile changes,
- engine version changes,
- artifact format version changes,
- column schema key changes,
- spectral/transient profile changes,
- alignment transform changes,
- upstream artifact basis changes.

An artifact does not become stale when:

- renderer theme changes,
- UI color changes,
- storage path changes while artifact identity remains valid,
- parent/stem-set normalization hint arrives,
- tile cache last-access timestamp changes.
