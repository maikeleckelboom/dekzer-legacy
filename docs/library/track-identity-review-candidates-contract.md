---
status: accepted
last-reviewed: 2026-06-09
owner: library-store-sqlite
canonical-context:
  - track-identity-candidate-contract
  - track-identity-decision-contract
  - track-identity-decision-authority-contract
scope:
  - track-identity-review-candidates
  - backend-owned-review-state
  - candidate-decision-read-model
---

# Track Identity Review Candidates Contract

## Purpose

The track identity review-candidates read model answers which exact-content identity candidates exist, what evidence
summary and source participation they have, which effective decision currently governs them, and whether the backend
classifies them as needing review.

This is a read model. It is not decision authority, not candidate production authority, and not a canonical identity
surface.

## Request Shape

The public read command is `readTrackIdentityReviewCandidates`.

V0 request fields:

- `sourceId` optional source filter.
- `reviewState` optional filter. Omitted means all states.
- `limit` required positive limit, capped at 200 in V0.

Valid `reviewState` filter values are:

- `needsUserDecision`
- `systemAccepted`
- `userAccepted`
- `userRejected`
- `userDeferred`
- `staleDecision`

Requests must not carry filesystem paths, source paths, file paths, metadata, title, artist, album, canonical ids, or
track ids.

Omit `reviewState` to read all review states. V0 does not include an `all` review-state value.

## Response Shape

The response contains `candidates`.

Each candidate includes:

- `candidateId`
- `candidateKind`
- `candidateEvidenceBasis`
- `candidateStatus`
- `evidenceKeyAlgorithm`
- `evidenceKeyValue`
- `evidenceSummary`
- `sourceSummary`
- `reviewState`
- `effectiveDecision`
- `createdAtMs`
- `updatedAtMs`

`evidenceSummary` contains counts only:

- `memberCount`
- `evidenceCount`
- `currentEvidenceCount`

`sourceSummary` contains:

- `sourceCount`
- an optional small source sample when the store can fetch source ids and display names cheaply.

The response must not expose raw filesystem paths, source paths, file paths, filenames, title, artist, album, or
metadata in V0.

## Review States

The backend owns review-state derivation. Protocol, client, preload, shared TypeScript, renderer, and desktop code must
only forward the typed read result.

V0 review states are exactly:

- `needsUserDecision`: the candidate has no effective decision.
- `systemAccepted`: the effective decision is system accepted and current.
- `userAccepted`: the effective decision is user accepted and current.
- `userRejected`: the effective decision is user rejected and current.
- `userDeferred`: the effective decision is user deferred and current.
- `staleDecision`: an effective decision exists but its current status is stale.

The store fetches factual candidate, evidence, source, and effective-decision summary columns, then maps those facts to
the `reviewState` enum in Rust. SQL must not compute `reviewState`.

`blockedByUserDecision` is not a V0 state. It remains a future product gap if the review loop needs to distinguish user
reject/defer blocking semantics from the effective user decision itself.

## Effective Decision Relationship

The read model reuses backend-owned effective-decision semantics from the track identity decision read model:

- current user decisions win over current system decisions;
- current system decisions govern only when no current user decision governs;
- user reject and defer remain explicit user decisions and prevent silent system acceptance from becoming effective;
- stale candidate or evidence status is surfaced as `staleDecision` instead of being hidden.

The review read may expose `userBlockingDecisionState` and `maskedSystemDecisionId` when the existing effective-decision
summary exposes them. It must not mutate decision evidence or decision source-scope rows.

## Source Filtering

For live candidate review, source membership comes from candidate evidence source provenance. Decision source scope is
copied decision provenance and is not the primary membership authority for this read.

When `sourceId` is provided, candidates appear only if candidate evidence contains that source id. Candidates with no
candidate evidence source provenance remain readable in all-sources scope but do not appear in source-filtered reads.
The read model must not fabricate source scope.

## Evidence Summary Behavior

Evidence summaries are candidate evidence summaries. `currentEvidenceCount` uses the existing current candidate evidence
predicate. `evidenceKeyValue` is the candidate evidence digest and is acceptable in V0.

The read does not infer semantic identity from digest equality, paths, filenames, directories, title, artist, album,
metadata, or source participation.

## V0 Limit-Only Behavior

V0 ordering is stable backend ordering by `candidateId` ascending. `limit` must be between 1 and 200. V0 does not
implement cursoring.

TODO for future cursoring:

- cursor version;
- scope and filter identity;
- last `candidate_id` position.

Do not implement cursor serialization, deserialization, or validation until the cursor contract is promoted.

## What This Proves

This read model proves only:

- the backend can list candidate-centered review rows;
- each row is backed by stored candidate evidence summary and source participation facts;
- each row reports the backend-owned effective decision summary;
- each row has a backend-derived review state.

## What This Does Not Prove

This read model does not prove or create:

- canonical track identity;
- user-facing track objects;
- same-song or same-recording semantics across encodes;
- metadata correctness or metadata reconciliation;
- CUE-to-audio association;
- playlist, preparation, waveform, stem, sleeve, crate, artwork, or UI readiness.

Canonical track shells remain future because a product track identity must represent a stronger authority than an
exact-content candidate plus an effective decision.
