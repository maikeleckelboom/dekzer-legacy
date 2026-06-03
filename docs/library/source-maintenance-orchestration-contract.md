---
status: accepted
last-reviewed: 2026-06-03
owner: library-boundary-service
canonical-context:
  - observed-file-facts-contract
  - attachment-identity-foundation-contract
  - media-probe-observations-contract
  - primary-media-promotion-contract
  - track-identity-candidate-contract
  - track-identity-decision-contract
  - track-identity-decision-write-contract
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
canonical identity layer, readiness taxonomy, scheduler loop, or renderer-owned maintenance state.
The Rust service owner is the `source_maintenance` module in `library-boundary-service`.

The current unit exists so a scan completion or explicit command can make bounded progress on:

1. BLAKE3 observed-file hash evidence for pending media-relevant source files.
2. Attachment materialization from current BLAKE3 facts.
3. Audio-only media probe observations for pending audio source files.
4. Evidence-backed primary-media promotion from current attachments and audio probe facts.
5. Track identity candidate production from current evidence-backed primary-media candidates.
6. Track identity decision production from active exact-content candidates.
7. Maintained read-model invalidation through the existing honest scopes.

This is substrate maintenance only. Primary-media promotion v0 is attachment/probe evidence promotion, not preparation
readiness, canonical track identity, CUE association, waveform generation, stems, artwork intelligence, playlist UI, or
renderer presentation state. Track identity candidate production is exact current evidence grouping only; it is not a
canonical track decision. Track identity decision production creates reversible/supersedable decision records only; it
does not create canonical tracks.

## Maintenance Unit Order

The deterministic order is:

1. Run one bounded source-scoped BLAKE3 hash batch.
2. Run one bounded source-scoped attachment materialization batch.
3. Run one bounded source-scoped media probe batch.
4. Run one bounded source-scoped primary-media promotion batch.
5. Run one bounded source-scoped track identity candidate production batch.
6. Run one bounded source-scoped track identity decision production batch.
7. Publish maintained snapshot invalidations after each phase using current maintained revision scopes.

Attachment materialization follows hashing because `content_attachments` and `source_file_attachment_links` consume
current BLAKE3 `SourceFacts`. Media probing follows materialization because probe commits merge current compatible
BLAKE3 evidence into `SourceFacts`; existing attachment links remain current when the hash evidence is preserved.
Primary-media promotion follows probing because it requires current attachment links and at least one current audio
probe fact. Track identity candidate production follows primary-media promotion because it consumes only current
evidence-backed `primary_media_candidates` rows and revalidates the source-file, attachment, BLAKE3, and probe evidence
before producing or refreshing candidate rows. Track identity decision production follows candidate production because it
consumes only active exact-content candidates with current candidate evidence and produces current
`system_exact_content_v0` accepted decision records only when a current system decision does not already exist and no
current user `rejected` or `deferred` decision blocks the candidate.

The order must not be interpreted as product preparation. Remaining candidates are expected after a bounded unit.

## Bounded Behavior

The command is source-scoped and accepts:

- `sourceId`
- optional `hashLimit`
- optional `attachmentLimit`
- optional `probeLimit`
- optional `promotionLimit`
- optional `identityCandidateLimit`
- optional `identityDecisionLimit`

The renderer never supplies filesystem paths and never supplies a source-file-id list for v0. Limits are validated as
positive integers and are capped by backend policy. No phase loops until the source is drained.

Per-file failures do not spin. A failed hash or probe outcome is reflected in the phase summary; unrelated successful
candidates in the bounded batch can still commit through their existing authority paths.

## Runtime Scheduling

Runtime scheduling is owned by the backend service instance. Pending source ids are held in memory, deduped by source
id, and drained deterministically in ascending source-id order. Active source ids are also held in memory and prevent a
second same-source unit from starting while one is already running.

A manual `runSourceMaintenance` command for a source that is already active returns a bounded `skipped` run reply with
the requested effective limits and does not run hashing, attachment materialization, probing, primary-media promotion,
track identity candidate production, or track identity decision production. That scheduler skip is not recorded as
`lastRun`.

A manual `runSourceMaintenance` command for a source that is pending but not active takes immediate ownership of that
source and removes it from pending state before running one bounded unit. The same source must not then run again from
the pending queue immediately after the manual command.

Calling stop clears pending state. Active state is cleared when the in-flight unit exits through its normal or error
cleanup path.

## Command Boundary

The boundary command is:

| Layer                   | Command                                                                                                                                                                     |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust protocol           | `SourceMaintenance.RunSourceMaintenance`                                                                                                                                    |
| Generated TS contract   | `runSourceMaintenance`                                                                                                                                                      |
| Desktop IPC/preload API | `library.sourceMaintenance.runSourceMaintenance({ sourceId, hashLimit?, attachmentLimit?, probeLimit?, promotionLimit?, identityCandidateLimit?, identityDecisionLimit? })` |

The reply includes:

- `sourceId`
- effective limits for hash, attachment, probe, promotion, track identity candidate, and track identity decision phases
- hash summary counts and remaining hash candidates
- attachment materialization summary counts
- probe summary counts and remaining probe candidates
- primary-media promotion summary counts and remaining promotion candidates
- track identity candidate production summary counts and remaining candidate production candidates
- track identity decision production summary counts, user-blocked skip counts, and remaining decision production
  candidates
- current/stale/missing attachment-link summary when the source attachment read model can answer
- typed `sourceFailure` for unavailable, missing, blocked, or not-found source state
- coarse run status: `completed`, `partial`, `skipped`, or `failed`

The command does not expose local filesystem paths.

## Read Snapshot

The snapshot read is:

| Layer                   | Read                                                            |
| ----------------------- | --------------------------------------------------------------- |
| Rust protocol           | `SnapshotRead.ReadSourceMaintenance`                            |
| Generated TS contract   | `readSourceMaintenance`                                         |
| Desktop IPC/preload API | `library.sourceMaintenance.readSourceMaintenance({ sourceId })` |

The snapshot computes:

- source lifecycle failure state from `readSourceLifecycle`
- remaining BLAKE3 hash candidates
- remaining media probe candidates
- remaining primary-media promotion candidates
- remaining track identity candidate production candidates
- remaining track identity decision production candidates
- attachment current/stale/missing counts from the attachment identity read model
- in-memory service runtime status: `idle` or `running`
- in-memory last bounded run summary, when this service instance has run one

Runtime maintenance state is service-owned memory only. It is not durable identity and is not stored in SQLite. Durable
truth remains in `SourceFacts`, `content_attachments`, `source_file_attachment_links`, and
`primary_media_candidates`. Durable track identity candidate evidence remains in `track_identity_candidates`,
`track_identity_candidate_members`, and `track_identity_candidate_evidence`. Durable track identity decisions remain in
`track_identity_decisions` and `track_identity_decision_evidence`.

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
- Primary-media promotion updates `primary_media_candidates` from current attachments and current audio probe facts.
- Track identity candidate production updates `track_identity_candidates`,
  `track_identity_candidate_members`, and `track_identity_candidate_evidence` from current evidence-backed
  primary-media candidates.
- Track identity decision production updates `track_identity_decisions` and `track_identity_decision_evidence` from
  active exact-content candidates, preserves candidate/member/evidence provenance, and skips candidates with current
  user reject/defer decisions.

After each phase, the service publishes maintained snapshot invalidations from current maintained revisions. The narrow
honest maintained scope today is `LibraryBrowser`; attachment identity reads remain explicit until a precise maintained
attachment scope exists. No new source-maintenance event family is introduced in v0.

## Non-Goals

Source maintenance v0 does not:

- implement canonical track identity or user-facing track identity;
- make exact track identity candidates canonical;
- make exact track identity decisions canonical tracks;
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
- video-capable or richer `primaryMedia` promotion beyond audio v0;
- canonical track identity beyond explicit candidate decisions;
- preparation, waveform, stems, and artwork work.
