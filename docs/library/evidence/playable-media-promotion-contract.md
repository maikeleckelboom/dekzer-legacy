---
status: accepted
last-reviewed: 2026-06-02
owner: library-store-sqlite
canonical-context:
  - source-file-observations-contract
  - attachment-identity-contract
  - media-probe-observations-contract
  - maintenance-orchestration-contract
  - track-identity-candidate-contract
  - track-identity-decision-contract
scope:
  - playable-media-promotion
  - playable-media-candidates
  - contents-playable-media-profile
---

# Playable Media Promotion Contract

## Purpose

Playable media promotion v0 is the narrow bridge from current source-file evidence to the `readContents` `playableMedia`
row profile. It promotes audio attachments only when there is current BLAKE3 attachment identity and current audio probe
evidence for a present source-file occurrence.

Playable media is the current playable media record for a content attachment, backed by source-file observation evidence.
V0 is audio-only, but the name intentionally avoids "audio only" because the record represents the playable media role
and can later admit video or other playable media kinds deliberately.

This is not canonical track identity, CUE association, playlist identity, preparation readiness, waveform generation,
stems, artwork intelligence, or user-facing metadata intelligence. It is eligible input evidence for the later
track-identity-candidate layer, which remains non-canonical and reversible.

## Durable Target

The durable target is `playable_media`.

Rules:

- One candidate row exists per `content_attachments.attachment_id`.
- The row stores the evidence source file, the accepted evidence basis fingerprint, media kind, and basic probe fields.
- The row is evidence-backed attachment promotion only. It must not write source inventory, attachment identity,
  track-identity candidate/decision rows, preparation rows, waveform/stem artifacts, playlists, or product UI state.
- Current contents row identity for this policy is the `playable_media` row plus revalidated source-file,
  attachment, and `source_file_observations` provenance.

## Eligibility

A source-file occurrence is eligible only when all of these are true:

- The source is usable for maintenance.
- The source file is present, `file_class = audio`, and `file_kind = audio`.
- `source_file_observations` is current for the exact source-file basis: source id, relative path, size, mtime, and presence state.
- The current observations contain `content_hash_algorithm = blake3` and a non-empty hash value.
- `source_file_attachment_links` currently links the source file to a `content_attachments` row with the same hash.
- `source_file_observations.media_kind = audio`.
- At least one probe field is present: MIME type, duration, sample rate, channels, bit depth, or codec.

When multiple current source files point at the same attachment, promotion stores one candidate for the attachment and
chooses a deterministic representative by lowercased relative path and source-file id.

## Contents Read Behavior

`readContents` with `{ kind: 'playableMedia', mediaKinds: [...] }` reads only promoted
`playable_media` rows. At read time it
revalidates the current scoped source-file row, attachment link, attachment hash, and source observations. Stale or out-of-scope
candidate rows are omitted rather than returned as degraded product rows.

There is no fallback from plain present audio/video `source_files` to playable-media rows. A complete scope with no
promoted candidates is an authoritative empty playable-media result.

The row origin exposed through the boundary for these rows is `playableMedia`. Plain source-file fallback rows
are not emitted by the v0 playable-media query path.

## Source Maintenance Integration

Source maintenance runs promotion after hashing, attachment materialization, and media probing:

1. BLAKE3 source-file hash evidence.
2. Attachment materialization from current BLAKE3 observations.
3. Audio media probe observations.
4. Playable media promotion from current attachments and probe evidence.
5. Track identity candidate production from exact playable media content evidence.
6. Track identity decision production from active exact-content candidates.
7. Maintained snapshot invalidation.

The command accepts an optional `promotionLimit`, applies backend bounds, reports promotion summary counts, and reports
remaining promotion candidates. A bounded maintenance unit is allowed to finish partial when more promotion candidates
remain.
Promotion candidate reads and remaining-candidate counts are SQL-bounded/source-scoped. Promotion must not build a full
source-wide candidate vector merely to take the requested limit. Deterministic representative selection for multiple
source files pointing at one attachment remains lowercased relative path, then source-file id.

Track identity candidate production has its own optional `identityCandidateLimit`, and decision production has its own
optional `identityDecisionLimit`. Playable-media promotion does not create or refresh candidates or decisions directly.

## Non-Goals

Playable media promotion v0 does not:

- promote video files;
- parse CUE sheets or pair CUE with audio;
- infer canonical tracks, releases, performances, artwork roles, or metadata identity;
- create preparation, waveform, stems, playlist, or product-contents projection records outside
  `playable_media`;
- drain all candidates synchronously;
- expose local filesystem paths to the renderer.

## Next Gate

Future work may add video-capable probing and promotion, CUE parse observations, richer playable identity, canonical
track identity, or preparation integration. Those layers must remain separate from attachment identity and from v0
evidence-backed playable-media observations.
