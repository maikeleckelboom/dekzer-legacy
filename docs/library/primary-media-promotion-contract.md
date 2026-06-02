---
status: accepted
last-reviewed: 2026-06-02
owner: library-store-sqlite
canonical-context:
  - observed-file-facts-contract
  - attachment-identity-foundation-contract
  - media-probe-observations-contract
  - source-maintenance-orchestration-contract
  - track-identity-candidate-contract
scope:
  - primary-media-promotion
  - primary-media-candidates
  - contents-primary-media-profile
---

# Primary Media Promotion Contract

## Purpose

Primary media promotion v0 is the narrow bridge from current source-file evidence to the `readContents` `primaryMedia`
row profile. It promotes audio attachments only when there is current BLAKE3 attachment identity and current audio probe
evidence for a present source-file occurrence.

This is not canonical track identity, CUE association, playlist identity, preparation readiness, waveform generation,
stems, artwork intelligence, or user-facing metadata intelligence. It is eligible input evidence for the later
track-identity-candidate layer, which remains non-canonical and reversible.

## Durable Target

The durable target is `primary_media_candidates`.

Rules:

- One candidate row exists per `content_attachments.attachment_id`.
- The row stores the evidence source file, the accepted evidence basis fingerprint, media kind, and basic probe fields.
- The row is evidence-backed attachment promotion only. It must not write `LibraryAssets`, `LibraryAssetAttachments`,
  `SourceSegmentSets`, `SourceSegments`, `LibraryBrowserRows`, tracks, prep rows, waveforms, stems, playlists, or playlist
  entries.
- `LibraryBrowserRows` remains a browser projection over legacy asset rows. It is not primary-media v0 authority.

## Eligibility

A source-file occurrence is eligible only when all of these are true:

- The source is usable for maintenance.
- The source file is present, `media_class = audio`, and `file_kind = audio`.
- `SourceFacts` is current for the exact source-file basis: source id, relative path, size, mtime, and presence state.
- The current facts contain `content_hash_algorithm = blake3` and a non-empty hash value.
- `source_file_attachment_links` currently links the source file to a `content_attachments` row with the same hash.
- `SourceFacts.media_kind = audio`.
- At least one probe field is present: MIME type, duration, sample rate, channels, bit depth, or codec.

When multiple current source files point at the same attachment, promotion stores one candidate for the attachment and
chooses a deterministic representative by lowercased relative path and source-file id.

## Contents Read Behavior

`readContents` with `rowProfile: primaryMedia` reads only promoted `primary_media_candidates` rows. At read time it
revalidates the current scoped source-file row, attachment link, attachment hash, and source facts. Stale or out-of-scope
candidate rows are omitted rather than returned as degraded product rows.

There is no fallback from plain present audio/video `source_files` to primary-media rows. A complete scope with no
promoted candidates is an authoritative empty primary-media result.

The row origin exposed through the boundary for these rows is `primaryMediaCandidate`. Legacy `libraryAsset` and
`sourceFile` origins are not emitted by the v0 primary-media query path.

## Source Maintenance Integration

Source maintenance runs promotion after hashing, attachment materialization, and media probing:

1. BLAKE3 source-file hash evidence.
2. Attachment materialization from current BLAKE3 facts.
3. Audio media probe observations.
4. Primary media promotion from current attachments and probe evidence.
5. Track identity candidate production from current evidence-backed primary-media candidates.
6. Maintained snapshot invalidation.

The command accepts an optional `promotionLimit`, applies backend bounds, reports promotion summary counts, and reports
remaining promotion candidates. A bounded maintenance unit is allowed to finish partial when more promotion candidates
remain.

Track identity candidate production has its own optional `identityCandidateLimit`. Primary-media promotion does not
create or refresh those candidates directly.

## Non-Goals

Primary media promotion v0 does not:

- promote video files;
- parse CUE sheets or pair CUE with audio;
- infer canonical tracks, releases, performances, artwork roles, or metadata identity;
- create preparation, waveform, stems, playlist, or browser-row records;
- drain all candidates synchronously;
- expose local filesystem paths to the renderer.

## Next Gate

Future work may add video-capable probing and promotion, CUE parse observations, richer playable identity, canonical
track identity, or preparation integration. Those layers must remain separate from attachment identity and from v0
evidence-backed primary-media candidates.
