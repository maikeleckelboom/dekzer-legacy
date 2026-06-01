---
status: accepted
last-reviewed: 2026-06-01
owner: library-substrate-boundary
canonical-context:
  - observed-file-facts-contract
  - media-identity-schema-authority
scope:
  - attachment-identity
  - blake3-content-evidence
  - source-file-attachment-links
---

# Attachment Identity Foundation Contract

## Status

This is the first accepted attachment identity layer. It is Rust/store-only and has no boundary, desktop, preload,
renderer, command, event, scheduler, or UI exposure.

## Purpose

Attachment identity answers a narrow bytes-evidence question:

- which current source-file observations have the same BLAKE3 content evidence;
- which durable attachment/content row represents that evidence;
- which source-file rows currently link to that attachment as observed source-file occurrences.

It does not create tracks, playlist items, playable primary media, preparation targets, browser rows, CUE associations,
or user-facing readiness.

## Relation To Source Files

`source_files` remains the durable source-relative inventory authority. A source-file attachment link records one
observed occurrence of attachment bytes for a `source_file_id` and copies the source inventory context needed by this
layer:

- `source_file_id`
- `source_id`
- `content_hash_value`
- `file_kind`
- `created_at`
- `updated_at`

`file_kind` belongs on the link, not on the attachment/content row, because it is source-file interpretation context.

## Relation To Observed File Facts / SourceFacts

`SourceFacts` remains evidence. Attachment materialization consumes only current observed facts. A source-file link is
current only when the current observed-facts read for that `source_file_id` has:

- `content_hash_algorithm = 'blake3'`;
- `content_hash_value` equal to the link's copied hash value;
- current observed-fact status according to the observed-file-facts basis comparison.

Facts with stale basis, missing facts, or non-BLAKE3 facts do not materialize current links.

## Relation To BLAKE3 Evidence

BLAKE3 evidence is bytes evidence produced before this layer. Attachment materialization does not hash files, resolve
filesystem paths, scan roots, or repair missing facts. It reads accepted `SourceFacts` rows and source-file basis only.

## Attachment/Content Record Authority

`content_attachments` is the canonical durable bytes identity table for this pass.

Columns:

- `attachment_id INTEGER PRIMARY KEY`
- `content_hash_algorithm TEXT NOT NULL CHECK content_hash_algorithm = 'blake3'`
- `content_hash_value TEXT NOT NULL`
- `first_observed_at INTEGER NOT NULL`
- `updated_at INTEGER NOT NULL`
- `UNIQUE (content_hash_algorithm, content_hash_value)`

It deliberately has no `file_kind`, `equivalence_fingerprint`, title, artist, `primaryMedia`, track, playlist, prep, or
capability columns.

## Source-File Attachment Link Authority

`source_file_attachment_links` is the canonical source-file occurrence table for this pass.

Columns:

- `source_file_attachment_link_id INTEGER PRIMARY KEY`
- `attachment_id`
- `source_file_id`
- `source_id`
- `content_hash_value`
- `file_kind`
- `created_at`
- `updated_at`
- `UNIQUE (source_file_id)`

Indexes exist for `source_file_id` and `attachment_id`. The table represents the one current materialized attachment
occurrence for a `source_file_id` in this v0. It deliberately has no link history, stored `is_current`, stored
`is_stale`, CUE/audio association, track FK, playlist FK, or prep FK.

## Staleness Model

Staleness is computed by read-model join against current `SourceFacts` and `source_files` basis. It is not stored as a
boolean or cached status column. The read model exposes `link_status = Current | Stale`.

## Merge Policy

Materialization is source-scoped through:

`SqliteDurableStore::materialize_attachments_for_source(source_id, limit)`

Policy:

- new BLAKE3 hash evidence inserts one `content_attachments` row;
- already-known BLAKE3 hash evidence refreshes that `content_attachments.updated_at`;
- same hash across source files reuses the existing attachment and inserts more source-file links;
- same source file and same hash refreshes the link `updated_at` and attachment `updated_at`;
- same source file with a different current BLAKE3 hash deletes the old source-file link and inserts a new one;
- old attachment rows remain durable, and may become orphaned when no source-file links still point at them;
- `first_observed_at` is preserved on attachment refresh.

No link history exists in this v0. Hash-change history is represented only by preserved `content_attachments` rows and
the current source-file link.

## Materialization Outcome Fields

`MaterializeAttachmentsForSourceResult` is Rust/store-only and reports:

- `attachments_created` increments once per newly inserted `content_attachments` row;
- `attachments_refreshed` increments once per already-known BLAKE3 hash touched by the run;
- `links_created` increments when a source file receives its first current attachment link;
- `links_replaced` increments when a source file's old link is deleted and a new hash link is inserted;
- `links_refreshed` increments when an existing same-hash source-file link is touched;
- skipped counters report stale facts, non-BLAKE3 facts, and missing facts.

Two source files with the same new BLAKE3 value in one run create one attachment and two links. A later run against the
same BLAKE3 value refreshes the existing attachment instead of creating another one.

## Duplicate File Behavior

Two or more source files with the same current BLAKE3 value map to one attachment row and separate source-file links.
This is duplicate content evidence only. It is not track identity.

## CUE File Behavior

CUE source files may materialize as their own attachments from their own current BLAKE3 facts. Adjacent audio source
files may materialize as their own attachments. Materialization does not parse CUE sheets, infer adjacency, pair CUE
files to audio files, or create source segments.

## LibraryAssets And LibraryAssetAttachments

Task 1 decision block:

- Final table names: `content_attachments` and `source_file_attachment_links`.
- `LibraryAssets` and `LibraryAssetAttachments` are left in place as dormant/transitional library-asset and segment
  promotion substrate.
- They are not adapted or renamed because `LibraryAssets` is referenced by playlists, prep targets, capabilities,
  browser rows, waveform/prep read models, and projection rebuild code; `LibraryAssetAttachments` is segment-based and
  depends on `SourceSegments` / `SourceSegmentSets`.
- Reshaping them would drag playlist, prep, capability, browser projection, and segment promotion concerns into this
  slice.
- Surviving old columns remain as-is for old substrate behavior.
- No old columns are dropped in this pass.
- The smallest honest shape is a greenfield canonical bytes-identity table plus a source-file occurrence link table.

## equivalence_fingerprint After This Pass

`LibraryAssets.equivalence_fingerprint` remains an opaque transitional key for old `LibraryAssets` flows. It is not
content identity, is not read by attachment materialization, is not copied into `content_attachments`, and does not
satisfy BLAKE3 evidence.

## Explicitly Deferred

- cascade behavior when a `source_file` is removed from inventory beyond the current FK behavior;
- orphaned `content_attachments` cleanup / garbage collection after link replacement;
- auto-triggering materialization from scan completion or hash maintenance hooks;
- boundary, service, desktop, preload, renderer, command, event, or TypeScript exposure;
- track identity;
- CUE-to-audio association;
- `primaryMedia` activation;
- media probing and format metadata;
- artwork intelligence;
- waveform and prep;
- browser row population from attachment records;
- playlist or crate membership.
