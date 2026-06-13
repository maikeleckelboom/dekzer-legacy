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

# Attachment Identity Contract

## Status

This is the first accepted attachment identity layer. Durable attachment identity remains Rust/store-owned, bounded
materialization is service-owned maintenance, and attachment identity now has an explicit read-only boundary. There is
still no public materialization command, scheduler, or UI exposure.

## Purpose

Attachment identity answers a narrow bytes-evidence question:

- which current source-file observations have the same BLAKE3 content evidence;
- which durable attachment/content row represents that evidence;
- which source-file rows currently link to that attachment as observed source-file occurrences.

It does not create tracks, playlist items, playable primary media, preparation targets, CUE associations, product
contents projections, or user-facing readiness.

## Relation To Source Files

`source_files` remains the durable source-relative inventory authority. A source-file attachment link records one
observed occurrence of attachment bytes for a `source_file_id` and copies the source inventory context needed by this
layer:

- `source_file_id`
- `source_id`
- `file_kind`
- `created_at`
- `updated_at`

The link does not store a hash copy. Hash authority stays on the referenced `content_attachments` row. `file_kind`
belongs on the link, not on the attachment/content row, because it is source-file interpretation context.

## Relation To Observed File Facts / SourceFacts

`SourceFacts` remains evidence. Attachment materialization consumes only current observed facts. A source-file link is
current only when the current observed-facts read for that `source_file_id` has:

- `content_hash_algorithm = 'blake3'`;
- `content_hash_value` equal to the linked `content_attachments.content_hash_value`;
- current observed-fact status according to the observed-file-facts basis comparison.

Facts with stale basis, missing facts, or non-BLAKE3 facts do not materialize current links.

## Relation To BLAKE3 Evidence

BLAKE3 evidence is bytes evidence produced before this layer. Attachment materialization does not hash files, resolve
filesystem paths, scan roots, or repair missing facts. It reads accepted `SourceFacts` rows and source-file basis only.

## Service-Owned Maintenance

The boundary service owns the current attachment materialization maintenance hook. The hook is intentionally narrow:

- successful scan completion requests one source-scoped maintenance cycle for the completed source;
- that cycle runs at most one bounded BLAKE3 hash maintenance pass and then at most one bounded attachment
  materialization pass for the same `source_id`;
- if BLAKE3 evidence is already current but attachment links are missing, scan-triggered maintenance may still run the
  one bounded materialization pass;
- the pending source request is cleared after that bounded cycle;
- remaining hash candidates and remaining attachment materialization candidates are left to explicit commands or future
  scheduler work.

Manual `hashSourceFilesBlake3` is source-scoped. After a successful non-source-failure manual hash batch, the service
runs one bounded internal `materialize_attachments_for_source(source_id, limit)` unit for the same source. The public
hash command reply is unchanged and does not report attachment work; attachment identity read state is exposed only
through the explicit read boundary below.

The current service limit constants are intentionally small:

- `SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT`
- `SOURCE_ATTACHMENT_MATERIALIZATION_MAX_PASSES_PER_RUN = 1`

The service checks stop/shutdown between the hash unit and the attachment materialization unit. If shutdown arrives
while an individual file hash is in progress, the existing hash job may finish that file before the next stop check.

Attachment materialization mutates `content_attachments` and `source_file_attachment_links`. Attachment identity reads
are explicit snapshot reads. They are not currently attached to a maintained snapshot invalidation scope, because
navigation rows are not an honest precise signal for attachment-only link changes.

## Read Boundary

Attachment identity read exposure is narrow and read-only.

Service/client commands:

- `readSourceFileAttachment`
- `readAttachmentSourceFiles`
- `readSourceAttachmentSummary`

Desktop, preload, and renderer exposure is grouped under `library.attachmentIdentity.*` and forwards only those read
commands. Reads do not hash source files, materialize attachments, resolve filesystem paths, publish invalidation
events, or populate product views.

Read statuses are:

- `ok`
- `notFound`
- `invalidRequest`
- `readFailed`

`readAttachmentSourceFiles` is bounded by an optional `limit`. The reply reports `effectiveLimit` and
`remainingSourceFileLinks`; pagination remains future work.

`readSourceAttachmentSummary` reports current links, stale links, source files with current BLAKE3 facts, source files
with attachment links, source files missing attachment links, and `unmaterializedBlake3FactsCount`. In this v0,
`unmaterializedBlake3FactsCount` is equal to `sourceFilesMissingAttachmentLinksCount`; broader backlog estimation
belongs to collection health / source integrity work.

The boundary deliberately does not expose duplicate, relocation, product UI, track identity, CUE association,
`primaryMedia`, preparation, playlist, waveform, or product-contents projection behavior.

## Attachment/Content Record Authority

`content_attachments` is the canonical durable bytes identity table for this pass.

Columns:

- `attachment_id INTEGER PRIMARY KEY`
- `content_hash_algorithm TEXT NOT NULL CHECK content_hash_algorithm = 'blake3'`
- `content_hash_value TEXT NOT NULL`
- `first_observed_at INTEGER NOT NULL`
- `updated_at INTEGER NOT NULL`
- `UNIQUE (content_hash_algorithm, content_hash_value)`

It deliberately has no `file_kind`, title, artist, `primaryMedia`, track, playlist, preparation, or capability columns.

## Source-File Attachment Link Authority

`source_file_attachment_links` is the canonical source-file occurrence table for this pass.

Columns:

- `source_file_attachment_link_id INTEGER PRIMARY KEY`
- `attachment_id`
- `source_file_id`
- `source_id`
- `file_kind`
- `created_at`
- `updated_at`
- `UNIQUE (source_file_id)`

Indexes exist for `source_file_id` and `attachment_id`. The table represents the one current materialized attachment
occurrence for a `source_file_id` in this v0. It deliberately has no link history, stored `is_current`, stored
`is_stale`, stored hash copy, CUE/audio association, track FK, playlist FK, or preparation FK.

## Staleness Model

Staleness is computed by read-model join against current `SourceFacts`, `source_files` basis, and the linked
`content_attachments` hash. It is not stored as a boolean or cached status column. The read model exposes
`link_status = Current | Stale`, and any protocol-level link `content_hash_value` is derived from
`content_attachments`.

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

`MaterializeAttachmentsForSourceResult` is Rust/store/service-internal and reports:

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
files to audio files, or create split/association rows.

## Explicitly Deferred

- cascade behavior when a `source_file` is removed from inventory beyond the current FK behavior;
- orphaned `content_attachments` cleanup / garbage collection after link replacement;
- durable scheduler/drain behavior beyond one bounded scan/manual maintenance unit;
- precise attachment invalidation scope;
- product duplicate / relocation view;
- attachment-detail UI;
- track identity;
- CUE-to-audio association;
- `primaryMedia` activation;
- media probing and format metadata;
- artwork intelligence;
- waveform and preparation;
- product contents projection from attachment records;
- playlist or crate membership.
