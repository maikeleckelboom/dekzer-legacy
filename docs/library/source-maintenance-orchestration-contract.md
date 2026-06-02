---
status: accepted
last-reviewed: 2026-06-02
owner: library-boundary-service
canonical-context:
  - observed-file-facts-contract
  - attachment-identity-foundation-contract
  - media-probe-observations-contract
  - source-lifecycle-backend-contract-gap
scope:
  - source-maintenance-orchestration
  - bounded-maintenance-unit
  - runtime-maintenance-state
---

# Source Maintenance Orchestration Contract

## Purpose

Source maintenance is the backend-owned orchestration path for one bounded substrate maintenance unit for a source.
It coordinates existing evidence and attachment authority paths; it does not create a new evidence type, durable
identity layer, readiness taxonomy, scheduler loop, or renderer-owned maintenance state.

The current unit exists so a scan completion or explicit command can make bounded progress on:

1. BLAKE3 observed-file hash evidence for pending media-relevant source files.
2. Attachment materialization from current BLAKE3 facts.
3. Audio-only media probe observations for pending audio source files.
4. Maintained read-model invalidation through the existing honest scopes.

This is substrate maintenance only. It is not preparation readiness, track identity, `primaryMedia` promotion, CUE
association, waveform generation, stems, artwork intelligence, playlist UI, or renderer presentation state.

## Maintenance Unit Order

The deterministic order is:

1. Run one bounded source-scoped BLAKE3 hash batch.
2. Run one bounded source-scoped attachment materialization batch.
3. Run one bounded source-scoped media probe batch.
4. Publish maintained snapshot invalidations after each phase using current maintained revision scopes.

Attachment materialization follows hashing because `content_attachments` and `source_file_attachment_links` consume
current BLAKE3 `SourceFacts`. Media probing follows materialization because probe commits merge current compatible
BLAKE3 evidence into `SourceFacts`; existing attachment links remain current when the hash evidence is preserved.

The order must not be interpreted as product preparation. Remaining candidates are expected after a bounded unit.

## Bounded Behavior

The command is source-scoped and accepts:

- `sourceId`
- optional `hashLimit`
- optional `attachmentLimit`
- optional `probeLimit`

The renderer never supplies filesystem paths and never supplies a source-file-id list for v0. Limits are validated as
positive integers and are capped by backend policy. No phase loops until the source is drained.

Per-file failures do not spin. A failed hash or probe outcome is reflected in the phase summary; unrelated successful
candidates in the bounded batch can still commit through their existing authority paths.

## Command Boundary

The boundary command is:

| Layer | Command |
| --- | --- |
| Rust protocol | `SourceMaintenance.RunSourceMaintenance` |
| Generated TS contract | `runSourceMaintenance` |
| Desktop IPC/preload API | `library.sourceMaintenance.runSourceMaintenance({ sourceId, hashLimit?, attachmentLimit?, probeLimit? })` |

The reply includes:

- `sourceId`
- effective limits for hash, attachment, and probe phases
- hash summary counts and remaining hash candidates
- attachment materialization summary counts
- probe summary counts and remaining probe candidates
- current/stale/missing attachment-link summary when the source attachment read model can answer
- typed `sourceFailure` for unavailable, missing, blocked, or not-found source state
- coarse run status: `completed`, `partial`, `skipped`, or `failed`

The command does not expose local filesystem paths.

## Read Snapshot

The snapshot read is:

| Layer | Read |
| --- | --- |
| Rust protocol | `SnapshotRead.ReadSourceMaintenance` |
| Generated TS contract | `readSourceMaintenance` |
| Desktop IPC/preload API | `library.sourceMaintenance.readSourceMaintenance({ sourceId })` |

The snapshot computes:

- source lifecycle failure state from `readSourceLifecycle`
- remaining BLAKE3 hash candidates
- remaining media probe candidates
- attachment current/stale/missing counts from the attachment identity read model
- in-memory service runtime status: `idle` or `running`
- in-memory last bounded run summary, when this service instance has run one

Runtime maintenance state is service-owned memory only. It is not durable identity and is not stored in SQLite. Durable
truth remains in `SourceFacts`, `content_attachments`, and `source_file_attachment_links`.

If source lifecycle prevents maintenance, the snapshot returns a typed source failure and does not report an empty
success. Snapshot status maps source-not-found to `failed`, missing/unavailable source roots to `unavailable`, blocked
roots to `blocked`, active in-memory maintenance to `running`, and otherwise `idle`.

## Scan-Triggered Behavior

Successful root scan completion requests one bounded source maintenance unit for the completed durable source id after
the terminal scan event and scan-maintained invalidations are published.

Blocked, failed, and cancelled scans do not request source maintenance. Scan-triggered source maintenance remains
service-owned and bounded. There is no hidden endless background drain loop and no renderer scheduler.

The scan command still uses `rootId` in the public roots API. Registered local root ids are durable source ids for this
maintenance unit.

## Invalidation And Events

Each phase commits through the existing authority path:

- BLAKE3 hashing commits accepted `SourceFacts` through inspect-source work/artifact authority.
- Attachment materialization updates `content_attachments` and `source_file_attachment_links`.
- Media probing commits accepted `SourceFacts` through inspect-source work/artifact authority.

After each phase, the service publishes maintained snapshot invalidations from current maintained revisions. The narrow
honest maintained scope today is `LibraryBrowser`; attachment identity reads remain explicit until a precise maintained
attachment scope exists. No new source-maintenance event family is introduced in v0.

## Non-Goals

Source maintenance v0 does not:

- implement track identity;
- promote attachments into `primaryMedia`;
- parse CUE sheets or pair CUE with audio;
- probe video files;
- generate waveform data, stems, prep rows, artwork intelligence, playlist UI, crates, sleeves, badges, chips, or
  visible renderer status;
- create durable scheduler state or duplicate scheduler state in the renderer;
- resolve filesystem paths outside the backend.

## Future Work

Future work remains separate:

- scheduler policy beyond scan completion and explicit command invocation;
- source-location-scoped maintenance;
- video probe adapter selection;
- CUE parse observations owned by CUE source-file rows;
- attachment-to-`primaryMedia` promotion;
- track identity;
- preparation, waveform, stems, and artwork work.
