---
status: accepted
last-reviewed: 2026-05-31
owner: library-substrate-boundary
canonical-context:
  - media-relevant-file-inventory-contract
  - media-identity-schema-authority
scope:
  - observed-file-facts
  - content-hash-placement
  - source-file-evidence
---

# Observed File Facts Contract

## Purpose

Observed file facts are durable evidence produced by reading or probing a source file. They are attached to a
`source_file_id` and a captured file basis. They are not product identity, track identity, attachment identity, artwork
intelligence, waveform state, stems state, or preparation readiness.

Source files remain attachment inventory. Observed facts describe what was observed about a file row at a point in
time; they do not promote that file into playable media.

## Current Storage

`SourceFacts` is the accepted observed-file-facts table for the current substrate. It stores the accepted
`source_inspection` fact for a `source_file_id`. The row includes:

- `source_file_id`
- `fact_kind`
- `basis_fingerprint`
- `basis_source_id`
- `basis_relative_path`
- `basis_size_bytes`
- `basis_mtime_ns`
- `basis_presence_state`
- `observed_at_ms`
- optional `content_hash_algorithm` and `content_hash_value`
- media/probe summary fields such as `media_kind`, `mime_type`, duration, sample rate, channels, bit depth, and codec
- `accepted_artifact_id`

The basis columns are copied from `source_files` when the accepted facts are committed. A read compares the copied
basis with the current `source_files` row and returns `current` only when source id, relative path, size, mtime, and
presence still match. Otherwise the fact is `stale`.

## Content Hash Policy

Content hash evidence belongs here because hashing reads file bytes. Hash values must always be stored with an explicit
algorithm tag.

SHA-256 appears only in tests and fixtures as explicit evidence. It is not declared the product content-hash algorithm.

BLAKE3 source-file hashing is implemented in `library-store-sqlite` as a narrow store-side evidence job. The job accepts
a `source_file_id` plus a caller-resolved filesystem path, streams file bytes through BLAKE3, and commits accepted
`SourceFacts` through the existing inspect-source artifact and observed-facts authority path. The algorithm string is
`blake3`, and the value is the canonical lowercase hex digest.

BLAKE3 is content evidence, not identity by itself. The hash job does not create `LibraryAssets`,
`LibraryAssetAttachments`, `SourceSegmentSets`, `SourceSegments`, track rows, attachment identity, or CUE-to-audio
pairing.

The job captures the current `source_files` basis before reading bytes and re-reads it before committing. If the basis
changed while hashing was in flight, the job rejects the commit with a typed basis-change result instead of recording
evidence that could appear current for the wrong row state. Missing or unreadable filesystem paths fail as typed IO
errors and do not create `SourceFacts`.

`LibraryAssets.equivalence_fingerprint` is not a content hash. It must not be copied into observed file facts as hash
evidence.

## CUE Ownership

A CUE file owns future CUE parse observations on its own `source_file_id`. An adjacent audio file owns future
audio/container/probe observations on its own source-file row. This contract does not infer CUE-to-audio pairing,
does not parse CUE sheets, and does not create segment, attachment, or track rows from CUE evidence.

## Future Work

Future work remains separate:

- production scheduling/path-resolution integration for the BLAKE3 hash job
- media/container probing
- CUE parsing
- CUE-to-audio association evidence
- attachment identity
- track identity
- artwork intelligence
- waveform, stems, and preparation jobs
