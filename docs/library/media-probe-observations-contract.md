---
status: accepted
last-reviewed: 2026-06-01
owner: library-substrate-boundary
canonical-context:
  - observed-file-facts-contract
  - media-identity-schema-authority
  - media-relevant-file-inventory-contract
scope:
  - media-probe-observations
  - source-file-evidence
  - probe-admission
---

# Media Probe Observations Contract

## Purpose

Media probe observations are durable evidence produced by reading a present source file enough to identify basic
container/audio facts. They are attached to `source_file_id` and a copied source-file basis through `SourceFacts`.

Probe facts are not track identity, attachment identity, `primaryMedia`, artwork intelligence, waveform state, stems
state, preparation readiness, playlist state, CUE association, or UI status.

## Storage And Merge Policy

`SourceFacts` remains the accepted current observed-file-facts row for media probe v0. No separate observation ledger is
introduced in this gate.

Because `SourceFacts` is one accepted row per `source_file_id`, evidence jobs must merge compatible current facts instead
of replacing unrelated evidence with nulls:

- A media probe commit preserves current content hash evidence only when the previous `SourceFacts` row is current for
  the same `source_files` basis.
- A BLAKE3 hash commit preserves current probe fields only when the previous `SourceFacts` row is current for the same
  `source_files` basis.
- Stale hash evidence is never carried forward by a probe commit.
- Stale probe fields are never carried forward by a hash commit.

The basis comparison remains source id, relative path, size, mtime, and presence state. If the file basis changes between
probe read and commit, the job rejects the commit with a typed basis-changed outcome.

## Probe Owner And Adapter

The probe job is owned by `library-store-sqlite`. The renderer does not resolve paths or trigger probing directly.

Path resolution reuses the backend source-file resolver used by BLAKE3 hashing: source lifecycle state supplies the root,
`source_files.relative_path` supplies the source-relative file path, and canonicalized paths must stay under the
canonical source root.

The v0 adapter is `dekzer.source_file_media_probe.symphonia` version `1`, implemented with the pure Rust Symphonia
metadata/container reader. Symphonia is used for header/container probing; the job does not decode packets into waveform
buffers and does not generate audio analysis artifacts.

The accepted artifact is an `inspection_result` inline JSON payload. Tool identity is represented by `WorkRuns` and
`Artifacts` adapter key/version plus the artifact payload.

## Supported Fields

V0 writes the fields Symphonia can honestly expose for supported audio files:

- `media_kind`
- `mime_type`
- `duration_ms`
- `sample_rate_hz`
- `channels`
- `bit_depth`
- `codec`

The current v0 implementation supports audio probing. Video source files are admitted as media-probe candidates but
return a typed unsupported outcome until a video-capable no-native-runtime adapter is selected. Images are not admitted
for media probe v0. CUE sheets are not admitted and are not parsed.

MIME/container values are best-effort v0 labels derived from the source-relative extension after Symphonia has accepted
the file as a supported audio stream. They are evidence summary fields, not identity.

## Unsupported And Failure Behavior

Missing, unavailable, blocked, unreadable, unsupported, invalid-relative-path, root-escape, and basis-changed cases return
typed outcomes. They do not write fake probe facts and do not erase source-file inventory.

Unsupported companion metadata such as CUE sheets is not parsed and is not paired to adjacent audio. A CUE source-file row
will own future CUE parse observations on its own row in a separate gate.

## Non-Goals

Media probe v0 does not:

- promote attachments into `primaryMedia`;
- create `LibraryAssets`, `LibraryAssetAttachments`, `content_attachments`, or `source_file_attachment_links`;
- infer track identity;
- parse CUE sheets or associate CUE files with audio;
- generate waveform data;
- create stems, prep readiness, playlists UI, crates, sleeves, chips, badges, or renderer UI.

## Next Gate

The next gate after media probe v0 should decide one narrow follow-up: video-capable probe adapter selection,
source-location-scoped admission, broader scheduler policy, collection health/source integrity, CUE parse observations,
or a later promotion layer from attachment identity to playable media identity.
