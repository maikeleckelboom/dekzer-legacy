# Background work runs as bounded maintenance units

Status: Accepted
Date: 2026-08-05

## Context

Scanning a source produces follow-on work: hashing files, materializing attachments, probing containers, promoting
playable media, producing candidates, producing decisions. On a real DJ library this is tens of thousands of items,
some of them very large.

The instinctive design is a scheduler. Priority lanes, resource budgets per physical device, checkpointing for large
files, lease recovery, starvation prevention. That design exists, was written out in detail, and is the right answer
for a system under real workload pressure.

It is also a large amount of machinery whose behaviour cannot be validated without the workload it exists to manage.
Written first, its priority weights, lane assignments, and budget boundaries would be guesses, and wrong guesses in a
scheduler are hard to detect because the system keeps working, just badly.

The immediate requirement was narrower: scan completion must not block, a large file must not stall the application,
and repeated maintenance runs must not corrupt anything.

## Decision

Work runs as bounded maintenance units rather than through a scheduler.

**Scan-triggered maintenance performs at most one pass per stage**, then stops and clears the pending request.
Remaining candidates are explicit-command or future work. It never synchronously drains a large source.

**Work items are idempotent by construction.** A partial unique index over subject, work kind, and basis fingerprint,
restricted to active states, makes duplicate live work for the same subject and basis impossible at the database level
rather than by convention in the code that enqueues it.

**Work items carry a basis fingerprint.** The fingerprint identifies the input state the work was created against. Work
whose basis no longer matches current observations is stale and is not executed against outdated inputs.

**Work items are leased.** A leased item carries an expiry, enforced by a schema check, so an abandoned run is
detectable rather than permanently occupying its subject.

**Runs and artifacts are recorded separately from the item.** A work item may have many runs. A run produces artifacts
carrying the adapter key, adapter version, basis fingerprint, and payload hash that produced them. Artifacts are stored
inline or in a file store depending on size.

**Accepted observations cite the artifact that produced them.** `source_file_observations.accepted_artifact_id` is a
non-null foreign key to `work_artifacts`. An observation cannot exist without the run that produced it, which is what
makes evidence provenance a schema property rather than a convention.

**The scope is deliberately small.** Two work kinds exist: inspecting a source file and rebuilding a projection. Three
priority classes exist and are not used for lane routing.

## Consequences

Large sources are not fully processed by one scan. Remaining hash, attachment, and promotion candidates need an
explicit command or a later maintenance trigger. Backlog is visible through remaining counts rather than hidden inside
a queue, which is honest but means a freshly scanned large source is not immediately fully analyzed.

A single large file cannot stall the pipeline, because a pass is bounded by count rather than running to exhaustion.
That was the actual requirement, and it is met without lanes or budgets.

Maintenance can be re-run safely. Combined with the idempotence index and with evidence and decisions being separate
records, repeated runs converge rather than duplicating or overwriting. See
[evidence separated from decisions](0002-evidence-separated-from-decisions.md).

Every accepted observation is traceable to an adapter version. When a probe adapter produces bad results, the affected
observations are identifiable rather than being indistinguishable from good ones.

There is no fairness guarantee. Nothing prevents one source's work from being processed ahead of another's beyond the
order maintenance happens to run in. This is acceptable at current scale and is the first thing that will break under
a heavier workload.

There is no checkpointing. A large hash restarts rather than resuming. This is a real cost on very large files and is
the second thing that will need to change.

## Rejected alternatives

**A full scheduler with lanes, resource budget groups, checkpointing, and starvation prevention.** The design is
sound and this decision is not an argument against it. Rejected for now because its parameters cannot be chosen
honestly without a workload to tune against, and because it is a large surface to maintain in support of a requirement
that bounded passes already satisfy. It is recorded in Git history and should be reconsidered when a measured workload
shows bounded units are insufficient.

**Draining all pending work on scan completion.** Simple and complete. Rejected because it makes scan completion
unbounded on large sources, which is the failure the bounded pass exists to prevent.

**A plain FIFO queue.** Rejected for the same reason a scheduler was deferred, from the other direction: it adds queue
machinery without adding the prioritization that would justify it.

**Storing staleness on the work item instead of comparing basis fingerprints.** Rejected for the same reason stored
staleness was rejected for observations. A cached flag goes wrong quietly.

**Letting observations exist without citing an artifact.** Simpler schema, one fewer required foreign key. Rejected
because provenance enforced by convention is provenance that eventually is not there.

## Implementation evidence

- Schema: `work_items`, `work_runs`, `work_artifacts`, `work_artifact_inline_payloads`,
  `work_artifact_file_store_entries`, `work_artifact_claims` in
  `crates/library-store-sqlite/migrations/20260502000000_substrate_baseline.sql`
- Idempotence: the `work_items_active_work` partial unique index over subject, work kind, and basis fingerprint
- Lease enforcement: the `state = 'leased'` implies non-null `leased_until` check constraint
- Provenance: the non-null `accepted_artifact_id` foreign key on `source_file_observations`
- Authority code: `crates/library-store-sqlite/src/authority/work/`
- Bounded maintenance: `crates/library-boundary-service/src/service.rs`
