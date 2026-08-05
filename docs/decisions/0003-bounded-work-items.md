# Background work is bounded-count maintenance with durable provenance

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

The immediate requirements were narrower. Scan completion must not enqueue an unbounded amount of synchronous work.
Repeated maintenance runs must not corrupt anything or duplicate work. Every accepted observation must be traceable to
the run that produced it. Evidence taken against a file that changed underneath must not be committed.

## Decision

Work runs as bounded-count maintenance units, and the work tables exist to provide provenance and uniqueness rather
than execution control.

**A maintenance pass attempts a bounded number of candidates.** Scan-triggered maintenance performs at most one pass
per stage, then stops and clears the pending request. Remaining candidates are explicit-command or future work. It
never enumerates and drains an entire source.

**Expensive reading happens before the write path opens.** The implemented hash and probe paths follow the same shape:
load the initial basis, perform the expensive operation against the file, then open a write transaction. Inside that
transaction the basis is reloaded and compared, and a mismatch rejects the commit.

**Work items are created at commit time, not before the work.** Within that write transaction, the pass queues the
work item, claims it with a lease, starts a run, records the artifact, and accepts the observation. The work item, run,
and artifact therefore describe an accepted unit of work rather than scheduling one.

**Work identity is unique while active.** A partial unique index over subject, work kind, and basis fingerprint,
restricted to active states, makes duplicate live work for the same subject and basis impossible at the database level
rather than by convention in the code that enqueues it.

**Accepted observations cite the artifact that produced them.** `source_file_observations.accepted_artifact_id` is a
non-null foreign key to `work_artifacts`, and artifacts carry the adapter key, adapter version, basis fingerprint, and
payload hash. An observation cannot exist without the run that produced it, which makes evidence provenance a schema
property rather than a convention.

**The scope is deliberately small.** Two work kinds exist: inspecting a source file and rebuilding a projection. Three
priority classes exist and are not used for lane routing.

## Consequences

**Batch limits bound how many candidates a pass attempts. They do not bound the time spent on one candidate.** Hashing
reads a file to completion in a 64 KiB loop with no byte limit, time limit, checkpoint, or cancellation check. A large
or slow file may still occupy a maintenance pass. This is a real limitation and the first thing that will need to
change under a heavier workload.

**Basis validation prevents an outdated result from being committed. It does not prevent the underlying work from
being performed.** A file that changes while it is being hashed is hashed to the end, and the commit is then rejected.
The wasted read is accepted as the cost of not holding a write transaction open across an unbounded file read.

**Leases cover the accepted commit lifecycle, not the expensive operation.** Because the work item is created after the
read completes, a lease does not protect a long-running hash from a second attempt, and lease expiry is not currently a
recovery path for abandoned reading. It bounds the committed record, not the worker.

**There is no scheduler and no fairness guarantee.** Nothing prevents one source's work being processed ahead of
another's beyond the order maintenance happens to run in. Nothing prioritizes a cheap operation over an expensive one.

Large sources are therefore not fully processed by one scan. Remaining candidates need an explicit command or a later
maintenance trigger, and backlog is visible through remaining counts rather than hidden inside a queue.

Maintenance can be re-run safely. Combined with the active-row uniqueness index and with evidence and decisions being
separate records, repeated runs converge rather than duplicating or overwriting. See
[evidence separated from decisions](0002-evidence-separated-from-decisions.md).

Every accepted observation is traceable to an adapter version. When a probe adapter produces bad results, the affected
observations are identifiable rather than being indistinguishable from good ones.

## Rejected alternatives

**A full scheduler with lanes, resource budget groups, checkpointing, and starvation prevention.** The design is
sound and this decision is not an argument against it. Rejected for now because its parameters cannot be chosen
honestly without a workload to tune against, and because it is a large surface to maintain. It is recorded in Git
history and should be reconsidered when a measured workload shows bounded-count passes are insufficient. The
limitations listed above are the evidence that would justify it.

**Draining all pending work on scan completion.** Simple and complete. Rejected because it makes scan completion
unbounded on large sources.

**Holding the write transaction open across the file read.** Would let the work item and lease genuinely govern
execution. Rejected because it holds a SQLite write lock for the duration of an arbitrarily long read, blocking every
other writer. Deferring the transaction until after the read is why basis revalidation exists at all.

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
- Active-row uniqueness: the `work_items_active_work` partial unique index over subject, work kind, and basis
  fingerprint
- Provenance: the non-null `accepted_artifact_id` foreign key on `source_file_observations`
- Hash path ordering and basis revalidation: `hash_source_file_blake3_with_after_hash` and
  `commit_blake3_hash_evidence` in `crates/library-store-sqlite/src/store/source_file_hash.rs`
- Unbounded read loop: `hash_file_blake3` in the same file
- Probe path with the same ordering: `probe_source_file_media_with_after_probe` in
  `crates/library-store-sqlite/src/store/source_file_media_probe.rs`
- Batch bounding: `effective_hash_batch_limit` and `hash_source_file_blake3_batch`
- Authority code: `crates/library-store-sqlite/src/authority/work/`
- Bounded maintenance trigger: `crates/library-boundary-service/src/service.rs`
