---
status: accepted
last-reviewed: 2026-06-02
owner: library-store-sqlite
canonical-context:
  - observed-file-facts-contract
  - attachment-identity-foundation-contract
  - media-probe-observations-contract
  - primary-media-promotion-contract
  - track-identity-candidate-contract
  - source-maintenance-orchestration-contract
scope:
  - track-identity-decisions
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

V0 automatic production creates only `accepted` decisions with `decision_source = system_exact_content_v0`. That source
means the backend accepted an active exact-primary-media-content candidate under the v0 exact-content decision basis. It
does not mean a user accepted the candidate and does not claim semantic identity beyond exact current content evidence.

## Decision States

`track_identity_decisions.decision_state` is:

- `accepted`: the decision source accepted the candidate under the recorded decision basis.
- `rejected`: reserved for a future explicit decision path.
- `deferred`: reserved for a future explicit decision path.
- `superseded`: reserved for decisions replaced by a later decision.

Current decisions are rows with `superseded_by_decision_id IS NULL`. A future correction path may create a replacement
decision and set `superseded_by_decision_id` on the older decision. Superseded rows remain historical provenance.

## V0 Production

Source maintenance may run one bounded source-scoped decision production batch after track identity candidate production.

V0 production may create an automatic accepted decision only when all of these are true:

- the candidate is `active`;
- the candidate kind is `exact_primary_media_content`;
- the candidate evidence basis is `current_primary_media_exact_blake3`;
- at least one source-scoped candidate evidence row validates as current against source-file, attachment, BLAKE3 facts,
  audio probe fields, and probe artifact evidence;
- the candidate does not already have a current `system_exact_content_v0` decision.

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

Only candidate evidence rows that validate as current under the decision's evidence predicate are snapshotted.
Stale evidence rows belonging to the same candidate are excluded from the snapshot even if they remain durable
in `track_identity_candidate_evidence`. This ensures provenance reflects only the evidence that actually
supported the decision at the time it was made.

The snapshot is provenance, not a new content identity authority. BLAKE3 evidence remains exact bytes evidence and
`LibraryAssets.equivalence_fingerprint` must not be used as content identity.

## Read Model

The store read model is diagnostic. It exposes decision id, candidate id, decision state, decision source, decision
basis, decision reason, evidence key, supersession status, candidate-derived current/stale status, provenance evidence,
what the decision proves, and what it does not prove.

There is no renderer-owned identity decision surface in v0.

## Future Work

Future work may add:

- user-authored accept/reject/defer/supersede commands;
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
