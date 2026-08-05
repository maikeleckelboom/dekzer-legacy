# Evidence and user decisions are separate records

Status: Accepted
Date: 2026-08-05

## Context

Dekzer observes things about files: a BLAKE3 hash, a container probe, a grouping of items that share exact content. It
also needs to record what a person concluded: that two occurrences are the same item, that a suggested grouping is
wrong, that a judgement should wait.

The obvious implementation puts both on the same row. A candidate grouping gets a `status` column, maintenance writes
`accepted`, and a user action overwrites it. It is compact, it reads naturally, and it is wrong in a way that only
shows up after a user has done real work.

The failure is that recomputation and judgement become the same write. A rescan produces new evidence, maintenance
runs, and a column that held a person's decision now holds a machine's. Nothing errors. The user finds out when
something they rejected months ago reappears, or when a correction they made silently reverts after they plug in a
drive.

The same collapse also loses the basis. If the row holds only a current status, there is no record of what the
evidence looked like when the decision was made, so there is no way to tell a decision that is still valid from one
whose ground has shifted.

## Decision

Evidence and decisions never share a row.

**Evidence** records what the system observed or inferred. Source file observations hold basis-bound facts: the hash,
the probe results, and the file state they were taken against. Track identity candidates hold groupings derived from
that evidence. Each accepted observation cites the work artifact that produced it, so evidence traces back to the run
and adapter version that generated it.

**Decisions** are separate records over candidates, holding accept, reject, defer, or superseded, with a decision
source. Maintenance writes system decisions. Explicit commands write user decisions. Precedence is backend-owned: a
current user decision beats a system decision, and a current user reject or defer blocks a system accept from becoming
effective.

**Decision evidence is a snapshot.** A decision copies the provenance identifiers it acted on rather than joining live
to them. It does not cascade from current candidate, source, or attachment rows. The decision stays readable as the
judgement it was at the time it was made.

**Staleness is computed, not stored.** No `is_current` or `is_stale` column on observations or attachment links.
Validity is derived by comparing against current observation state. A stored flag is a cached answer to a question
whose inputs change without notifying the cache.

**Nothing is cleaned up, merged, removed, or forgotten without an explicit decision record.** Dekzer may surface
evidence that the same bytes exist in several places, or that content appears to have moved. It never acts on it.
Occurrence views are diagnostic, not action recommendations.

## Consequences

Reads get more work. Determining whether a link is current means joining to current observations and comparing hashes
rather than reading a boolean. This is a real cost, paid on every read that cares.

A decision can outlive the evidence that justified it, and be visibly stale rather than silently rewritten. Conflict
between a decision and changed evidence remains inspectable until someone resolves it explicitly. That means the
system can hold a contradiction, which is a state the interface has to be able to show.

Maintenance can run repeatedly without destroying user work, which is what makes bounded background maintenance safe
to run at all. See [bounded work items](0003-bounded-work-items.md).

There is no automatic cleanup path, so orphaned attachment rows after link replacement accumulate. That is a known
deferral and preferable to an automatic deletion path in a system that holds the only record of a user's library
decisions.

The pattern has to be reimplemented per layer rather than inherited from one shared table, because each layer's
evidence and decision shapes differ. That is duplication of structure, accepted so that each layer can define its own
conflict behaviour.

## Rejected alternatives

**A status column on the candidate.** The compact version described above. Rejected because it makes recomputation and
judgement the same write, which is precisely the failure.

**A single shared decisions table across all layers.** Attractive for consistency. Rejected because it forces one
target identity shape and one conflict policy on layers whose evidence differs, and a generic decision row ends up
either too loose to constrain anything or too rigid for the next layer.

**Stored staleness flags with invalidation on write.** Faster reads. Rejected because every code path that changes an
observation becomes responsible for updating flags elsewhere, and the first path that forgets produces silently wrong
answers with no failing test.

**Live joins for decision evidence instead of snapshots.** Keeps decisions consistent with current data. Rejected
because that is the wrong kind of consistency: a decision is a historical fact about a judgement, and rewriting its
basis when evidence changes destroys the record of what was actually decided.

**Automatic merge of exact-content duplicates.** The strongest case for automation, since identical bytes really are
identical. Rejected because identical bytes are not proof of the same musical item, and because a wrong automatic merge
is not recoverable from the user's perspective.

## Implementation evidence

- Observations with basis and artifact citation: `source_file_observations` in
  `crates/library-store-sqlite/migrations/20260502000000_substrate_baseline.sql`
- Candidates and decisions: `track_identity_candidates`, `track_identity_decisions`,
  `track_identity_decision_evidence`, `track_identity_decision_source_scope` in the same migration
- Decision commands: `crates/library-boundary-protocol/src/commands/track_identity_decisions.rs`
- Computed link staleness: no `is_stale` column on `source_file_attachment_links`, which derives status by joining
  current observations
- Conceptual model: [the domain model](../domain-model.md)
