---
status: provisional
last-reviewed: 2026-06-03
owner: library-store-sqlite
canonical-context:
  - observed-file-facts-contract
  - attachment-identity-foundation-contract
  - media-probe-observations-contract
  - primary-media-promotion-contract
  - track-identity-candidate-contract
  - track-identity-decision-authority-contract
  - source-maintenance-orchestration-contract
scope:
  - track-identity-decisions
  - effective-track-identity-decisions
  - exact-content-candidate-decisions
  - decision-provenance
---

# Track Identity Decision Contract

## Purpose

Track identity decisions are the first durable backend-owned decision layer above track identity candidates.

They record that a candidate has been classified by a decision source while preserving provenance back to the candidate,
candidate member, and candidate evidence rows that supported the decision. They do not create canonical tracks and do not
turn `track_identity_candidates` into tracks.

## Durable Target

The durable tables are:

- `track_identity_decisions`
- `track_identity_decision_evidence`

No `track_identities` table exists in v0. Adding canonical track shells would be a separate authority step because a
canonical track must mean more than an accepted exact-content candidate.

## Relation To Track Identity Candidates

A decision is over one `track_identity_candidate_id`.

The candidate remains the evidence group. The decision records how that candidate was classified by a decision source.
The decision does not own source-file inventory, attachment identity, primary-media promotion, CUE association, metadata
reconciliation, preparation state, playlist membership, browser rows, waveform state, stem state, or artwork state.

V0 automatic production creates `accepted` decisions with `decision_source = system_exact_content_v0`. Explicit local
user commands create `accepted`, `rejected`, or `deferred` decisions with `decision_source = user_local_v0`.

`system_exact_content_v0` means the backend accepted an active exact-primary-media-content candidate under the v0
exact-content decision basis. It does not mean a user accepted the candidate and does not claim semantic identity beyond
exact current content evidence.

`user_local_v0` means a backend command recorded explicit local user intent for a candidate. Protocol callers cannot
provide arbitrary decision-source strings.

## Decision States

`track_identity_decisions.decision_state` is:

- `accepted`: the decision source accepted the candidate under the recorded decision basis.
- `rejected`: the decision source rejected the candidate under the recorded decision basis.
- `deferred`: the decision source deferred the candidate under the recorded decision basis.
- `superseded`: reserved for historical rows replaced by a later decision.

Current decisions are rows with `superseded_by_decision_id IS NULL` and `decision_state != 'superseded'`.
Accept/reject/defer user commands supersede prior current `user_local_v0` rows for the same candidate. Superseded rows
remain historical provenance and retain their evidence snapshots.

## Effective Decision Semantics

The backend read model owns effective-decision resolution.

Effective decision precedence:

- current user decision wins over current system decision;
- if no current user decision exists, a current system decision may be effective;
- current user `rejected` or `deferred` blocks a current system `accepted` decision from being effective;
- superseded decisions are never effective;
- stale candidates or stale evidence surface as explicit stale current status instead of disappearing silently.

Read-model rows carry a nested `candidate_effective_decision` summary. The summary exposes effective decision id, state,
source, current status, precedence, `user_blocking_decision_state`, and optional `masked_system_decision_id`. User
reject/defer blocks system production/effectiveness even when no system decision exists; `masked_system_decision_id`
only appears when an existing current system decision is masked.

## V0 Production

Source maintenance may run one bounded source-scoped decision production batch after track identity candidate production.

V0 production may create an automatic accepted decision only when all of these are true:

- the candidate is `active`;
- the candidate kind is `exact_primary_media_content`;
- the candidate evidence basis is `current_primary_media_exact_blake3`;
- at least one source-scoped candidate evidence row validates as current against source-file, attachment, BLAKE3 facts,
  audio probe fields, and probe artifact evidence;
- the candidate does not already have a current `system_exact_content_v0` decision.
- the candidate does not have a current blocking `user_local_v0` rejected/deferred decision.

Stale candidates cannot receive new automatic accepted decisions. Existing decisions over later-stale candidates remain
historical records and can be superseded later.

## What A V0 Decision Proves

An automatic v0 accepted decision proves only this:

- a backend-owned source accepted an active exact-content track identity candidate;
- the decision was made under the recorded exact-content decision basis;
- the decision preserved candidate/member/evidence provenance at decision time.

## What A V0 Decision Does Not Prove

A v0 decision does not prove:

- canonical track identity;
- same song forever;
- same recording across different encodes;
- metadata correctness or metadata reconciliation;
- title, artist, album, key, BPM, artwork, cue point, or preparation state;
- CUE-to-audio association;
- playlist, crate, sleeve, waveform, stem, or renderer UI readiness.

## Provenance Requirements

Decision evidence snapshots must preserve:

- decision id;
- candidate id;
- candidate member id;
- candidate evidence id;
- primary-media candidate id;
- attachment id;
- source-file attachment link id;
- source file id and source id;
- BLAKE3 content evidence;
- evidence basis fingerprint;
- probe artifact id.

Only candidate evidence rows that validate as current under the decision's evidence predicate are snapshotted. Stale
evidence rows belonging to the same candidate are excluded from the snapshot even if they remain durable in
`track_identity_candidate_evidence`. This ensures provenance reflects only the evidence that actually supported the
decision at the time it was made.

User accept requires current supporting evidence and snapshots it. User reject/defer may target a stale candidate; they
snapshot current evidence if any exists and otherwise record zero decision evidence rows. Reject/defer must not fabricate
stale evidence as supporting evidence.

The snapshot is provenance, not a new content identity authority. BLAKE3 evidence remains exact bytes evidence and
`LibraryAssets.equivalence_fingerprint` must not be used as content identity.

`track_identity_decision_evidence` is a copied-provenance snapshot. Its candidate, member, candidate-evidence,
primary-media, attachment, source-file attachment link, source-file, source, and probe artifact ids are retained as
historical provenance ids, not live cascade authority. The snapshot may cascade only when the owning
`track_identity_decisions` row is deleted as the explicit retention boundary.

## Read Model

The store read model is diagnostic. It exposes decision id, candidate id, decision state, decision source, decision
basis, decision reason, evidence key, supersession status, candidate-derived current/stale status, the backend-owned
candidate effective-decision summary, provenance evidence, what the decision proves, and what it does not prove.

There is no renderer-owned identity decision precedence in v0. Renderer/preload/desktop code may forward explicit
candidate-scoped decision commands, but the backend owns writes and effective semantics.

## Future Work

Future work may add:

- public supersede-specific commands after a broader correction model exists;
- canonical track identity after additional authority is defined;
- multiple candidates mapping to one canonical identity;
- splitting one candidate into multiple identities;
- CUE parse observations and CUE-to-audio association;
- metadata reconciliation;
- different-encode matching.

Those future layers must consume decision and candidate provenance without treating exact hash evidence as semantic track
identity by itself.

## Non-Goals

Track identity decisions v0 do not:

- create canonical track rows or shells;
- infer semantic identity from path, filename, title, artist, album, directory, or metadata similarity;
- infer CUE-to-audio pairing or parse CUE sheets;
- implement prep, playlists, crates, sleeves, waveform authority, stem authority, artwork intelligence, or UI;
- let the renderer own identity decisions.
