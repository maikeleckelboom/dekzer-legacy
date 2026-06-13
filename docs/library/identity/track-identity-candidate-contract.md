---
status: accepted
last-reviewed: 2026-06-02
owner: library-store-sqlite
canonical-context:
  - observed-file-observations-contract
  - attachment-identity-contract
  - media-probe-observations-contract
  - playable-media-promotion-contract
  - maintenance-orchestration-contract
  - track-identity-decision-contract
scope:
  - track-identity-candidates
  - exact-playable-media-content-evidence
  - candidate-group-provenance
---

# Track Identity Candidate Contract

## Purpose

Track identity candidates are the first durable backend-owned foundation for saying that current evidence-backed
`playableMedia` candidates appear to represent the same exact playable item candidate.

They are not canonical tracks. They are not user-facing track identity. They do not make a semantic recording decision.
They preserve reversible evidence and provenance so the track identity decision layer and later identity layers can
decide whether exact duplicate playable content, metadata, CUE associations, user choices, or other observations should
become canonical track identity.

## Durable Target

The durable tables are:

- `track_identity_candidates`
- `track_identity_candidate_members`
- `track_identity_candidate_evidence`

The internal candidate kind for v0 is `exact_playable_media_content`.

The v0 evidence basis is `current_playable_media_exact_blake3`. It means the candidate was produced from a current
`playable_media` row, current attachment identity, current BLAKE3 source-file observations, and current audio probe
evidence. It does not mean "same song" beyond exact current content evidence.

## Status

`track_identity_candidates.status` is:

- `active`: at least one current member still validates against current playable-media, attachment, source-file, hash, and
  probe evidence.
- `stale`: the durable candidate remains as historical evidence, but no current member validates. Stale candidates must
  not be used as current identity output.
- `superseded`: reserved for a future reversible replacement flow. It is not used by v0 production.

Evidence rows also have computed read status:

- `current`: the evidence source file, attachment link, attachment hash, BLAKE3 observations, and probe observations still validate.
- `stale`: one or more of those inputs no longer validates.

## Relation To Existing Layers

`source_files` remain the default contents surface. Candidate production never removes source-file inventory rows and
never changes default `readContents` behavior.

`source_file_observations` remain observed evidence. Candidate production consumes only current BLAKE3 and audio probe observations; stale
observations cannot create or refresh candidates.

`content_attachments` and `source_file_attachment_links` remain attachment identity. Candidate production consumes
current links and attachment hash equality but does not re-read files or own hash authority.

`playable_media` remain the playable-media input. Candidate production consumes only current promoted audio
playable-media observations and revalidates them before producing or refreshing candidate rows.

## Sufficient Evidence

V0 production may create or refresh a candidate only when all of these are true:

- The input `playable_media` row is current.
- The evidence source file is present audio source-file inventory.
- `source_file_observations` is current for the exact source-file basis.
- `source_file_observations.content_hash_algorithm = blake3` and the hash value is non-empty.
- A current `source_file_attachment_links` row connects the source file to a `content_attachments` row with the same
  BLAKE3 value.
- `source_file_observations.media_kind = audio`.
- At least one audio probe field is present.

## Insufficient Evidence

The following never create or refresh track identity candidates by themselves:

- BLAKE3 hash evidence without current playable-media promotion.
- Path, title, filename, directory, or metadata similarity.
- CUE file path proximity, CUE parsing, or inferred CUE-to-audio pairing.
- Renderer requests or renderer-side grouping.
- Image, CUE, unsupported, video-without-current-v0-promotion, playlist, prep, waveform, stem, sleeve, crate, or artwork
  state.

## Grouping Rules

One candidate group may contain multiple evidence rows when current playable-media observations share the same exact BLAKE3
content evidence. This is exact content evidence grouping only. It is not a semantic track decision.

The current substrate promotes one `playable_media` row per attachment, so duplicate source files with the same
BLAKE3 attachment are represented as one member with multiple source-file evidence rows. A playable-media observation may
belong to only one active v0 candidate.

Different hashes do not group merely because paths, titles, filenames, or probe observations look similar. Different encodes of
the same musical recording are not solved by v0 unless future evidence explicitly supports that conclusion.

CUE sheets are not members and are not evidence rows in v0. They remain independent `source_files` rows because CUE parse
observations and CUE-to-audio association are future layers.

## Production And Reads

Candidate production is store-owned through a bounded source-scoped maintenance function. The renderer does not own
candidate production and does not supply grouping decisions.

The read model is diagnostic and store/service-level. It returns candidate id, status, members, evidence source-file and
attachment provenance, BLAKE3 evidence, probe artifact reference, why the candidate exists, and what it does not prove.
It is not a UI surface and not canonical identity output.

## Non-Goals

Track identity candidates v0 do not:

- create canonical track rows;
- create user decisions or user-facing identity;
- create track identity decision rows directly;
- create playlists, crates, sleeves, prep facets, waveform or stem authority, artwork intelligence, or UI;
- parse CUE sheets or infer CUE-to-audio pairing;
- reconcile metadata or match different encodes;
- use any deleted or caller-supplied opaque identity key as content identity.

## Future Work

Future layers may add user-authored decisions, canonical track identity, CUE parse observations, CUE-to-audio
association, multi-encode matching, metadata reconciliation, prep facets, playlists, crates, sleeves, waveform authority,
stem authority, and artwork intelligence. Those layers must consume this candidate foundation as evidence, not
reinterpret it as canonical track identity.
