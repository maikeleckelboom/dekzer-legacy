# CUE Sheet and Subrange Analysis Model V1

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define how future CUE sheet or long-file material can produce playable analysis targets without treating proximity or filenames as automatic truth.

This document does not implement CUE parsing.

## Problem

A CD rip or long continuous mix may produce:

- one large audio source file,
- one companion CUE sheet,
- many intended track-like subranges.

Dekzer needs waveform, beatgrid, key, loudness, and future stem analysis to operate in track-local coordinates even when the bytes come from a subrange of a larger content attachment.

## Core decision

A future playable unit may represent a source range of a larger content attachment.

The analysis target uses track-local sample coordinates. Local sample `0` maps to parent decoded sample
`source_range_start_sample`; local sample `n` maps to parent decoded sample `source_range_start_sample + n`.

    local_sample_zero = source_range_start_sample
    local_sample_n = source_range_start_sample + n

Waveform tiles, cue markers, beatgrid points, and playhead positions for that playable unit use local coordinates starting at zero.

## Future target relationship

A future source-range playable target depends on:

- parent content attachment,
- source range start sample,
- source range duration samples,
- CUE/provenance evidence if accepted,
- user correction if edited,
- decoder profile and sample-rate basis.

This must not create canonical track identity by accident. It is an analyzable/playable source-range target.

## Artifact dependency kind

Reserve dependency kind:

    source_range

A `source_range` dependency records:

- parent content attachment id/hash,
- start sample in parent decoded timebase,
- duration samples,
- range basis/provenance id,
- accepted correction version if user adjusted.

## Waveform coordinates

Waveform artifacts for subrange targets use local coordinates:

- tile column 0 starts at local sample 0,
- track length equals range duration,
- LOD bucket math uses local sample positions,
- final bucket clips to subrange duration.

The artifact manifest must retain the parent source range dependency so regeneration can locate the original bytes.

## CUE proximity law

A CUE file near an audio file is evidence, not automatic pairing truth.

Automatic pairing must remain proposed/inspectable because:

- filenames may not match,
- multiple audio files may sit near one CUE,
- CUE encoding may be wrong,
- pregap/index data may be ambiguous,
- a long mix may have user-edited split points,
- the user may prefer the single long-file identity over split ranges.

## User correction law

User correction must be possible for:

- pairing audio file to CUE file,
- track title/artist metadata,
- start/end offsets,
- pregap handling,
- hidden tracks or bonus segments,
- merged/split ranges.

Corrections become provenance for the source-range target. They must not mutate the original content attachment.

## Analysis implications

Artifacts that depend on a source range include the range basis in their artifact basis hash.

Changing a source range start/end invalidates artifacts for that range:

- waveform,
- beatgrid,
- loudness,
- key,
- phrase markers,
- future stem/playback artifacts.

Changing display metadata only does not invalidate signal artifacts unless it changes the accepted range or audio bytes.

## Stop conditions

Do not implement CUE parsing until:

- source_range dependency shape is accepted,
- local coordinate model is accepted,
- artifact basis hashing includes range basis,
- UI correction/provenance semantics are planned,
- long-file test corpus exists.
