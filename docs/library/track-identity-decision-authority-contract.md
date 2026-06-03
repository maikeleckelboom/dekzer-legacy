---
status: provisional
last-reviewed: 2026-06-03
owner: library-boundary-service
canonical-context:
  - track-identity-candidate-contract
  - track-identity-decision-contract
  - source-maintenance-orchestration-contract
scope:
  - track-identity-decision-authority
  - explicit-user-decision-authority
  - effective-decision-semantics
---

# Track Identity Decision Authority Contract

## Purpose

Track identity decision commands are the explicit backend-owned command surface for accepting, rejecting, or deferring a
track identity candidate before canonical track identity exists.

This contract adds decision authority, not track authority. It records user intent over an existing
`track_identity_candidate_id`, preserves evidence snapshots when evidence is current, and lets backend read models decide
which current decision is effective now.

## Command Family

The public command family is `TrackIdentityDecisionCommand`, carried by the outer `trackIdentityDecisions` command
family. The family is decision authority, not a transport write surface.

Commands:

- `AcceptTrackIdentityCandidate`
- `RejectTrackIdentityCandidate`
- `DeferTrackIdentityCandidate`

Requests contain:

- `candidateId`
- optional `reason`

Requests must not contain filesystem paths, source paths, title, artist, album, metadata, canonical track ids, or track
ids. Renderer and preload code may forward decision intent, but the service/store own validation, command behavior, and
effective-decision semantics.

## Decision Source Model

Protocol callers do not provide `decision_source`.

Backend-owned sources are:

- `system_exact_content_v0`: automatic source-maintenance acceptance for active exact-content candidates.
- `user_local_v0`: explicit local user accept/reject/defer commands.

The existing `track_identity_decisions.decision_source` column remains the authority. No source-kind/source-key split is
introduced in this slice.

## Current And Effective Decisions

A current decision is a decision row whose `superseded_by_decision_id` is null and whose `decision_state` is not
`superseded`.

An effective decision is the current decision that governs a candidate now.

Precedence:

- a current user decision wins over a current system decision;
- if no current user decision exists, a current system decision may be effective;
- current user `rejected` and `deferred` decisions block current system `accepted` decisions from being effective;
- superseded decisions are never effective;
- stale candidates or stale evidence produce explicit stale current status in the backend read model.

Effective-decision summaries are candidate-level resolution. Historical decision rows may expose that candidate summary,
but the summary is not owned by the historical row itself. The renderer must not compute precedence.

## Supersession Behavior

Accept/reject/defer commands supersede prior current `user_local_v0` decisions for the same candidate. The replacement
decision becomes current. Prior user decision rows remain readable as history and retain their original evidence
snapshots.

System maintenance does not supersede user decisions. Public supersession remains future; v0 correction is expressed by
running a new accept/reject/defer user command.

Repeated same-source same-state user decisions are idempotent and return the existing current decision instead of
creating duplicate current user rows.

## Evidence Snapshot Behavior

Accept requires:

- candidate exists;
- candidate is active;
- at least one current supporting candidate evidence row exists.

Accept snapshots the current supporting evidence rows into `track_identity_decision_evidence`.

Reject/defer require the candidate to exist, but may target a stale candidate. If current evidence exists, the decision
snapshots it. If no current evidence exists, the decision is still recorded with the candidate status at decision time
and zero evidence snapshot rows. Reject/defer must not fabricate stale evidence as supporting evidence.

Old decision evidence snapshots are immutable copied provenance. Later source facts, attachment links, candidate,
candidate-member, candidate-evidence, primary-media, attachment, source-file, source, probe, or replacement changes must
not rewrite or cascade-delete historical snapshot rows. Decision deletion is the snapshot retention boundary.

## Source Maintenance Interaction

Source maintenance may still create `system_exact_content_v0` accepted decisions for active exact-content candidates.

Maintenance must not write user decisions and must not silently override a current user decision. A current user
`rejected` or `deferred` decision is candidate-scoped and durable; it prevents a system accepted decision from being
effective and is skipped by the system exact-content decision-production phase.

Existing system decisions remain historical unless superseded by a later system maintenance rule in a future contract.

## What This Proves

An explicit user decision proves only:

- a backend command recorded local user intent for one candidate;
- the decision source was controlled by the backend;
- the decision state, reason, candidate status, and evidence snapshots were preserved at decision time;
- backend effective-decision precedence can respect user decisions before canonical track identity exists.

## What This Does Not Prove

This contract does not prove or create:

- canonical track identity;
- a track shell or renderer-facing Track object;
- semantic same-song or same-recording identity across encodes;
- metadata reconciliation from title, artist, album, filename, path, directory, or tags;
- CUE-to-audio pairing or CUE parsing;
- prep, playlists, crates, sleeves, waveform authority, stem authority, artwork intelligence, or UI readiness.

Canonical track shells remain future work because a track must represent a stronger product identity than an accepted or
rejected exact-content candidate decision.
