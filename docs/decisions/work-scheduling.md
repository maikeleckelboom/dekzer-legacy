# Work Scheduling and Large-File Fairness

_Companion to `media-role-classification.md` and `source-access-and-scan-coverage.md`. Governs how the library pipeline
schedules, prioritizes, budgets, checkpoints, cancels, and recovers work across scanner, sniffer, probe, identity,
classifier, readiness, and analysis stages._

---

Dekzer needs a work scheduler, not a work queue.

A queue runs jobs in order. A scheduler decides which unit of work creates the
most product value next, under bounded resources.

```
A 4 GB audio file is roughly equivalent to 70–95 typical CD-quality FLAC
tracks averaging 40–55 MB each. It must not stall those tracks from becoming
browse-ready while it processes in a lower-priority lane.

A giant waveform analysis must not block a cheap sniff on a different file.

A removable USB drive must not be hammered by concurrent decoders while the
UI is browsing it.
```

These are not edge cases. They are the expected operating conditions of a DJ
library on real hardware.

---

## Corrected Canon

These rules resolve contradictions in earlier drafts and govern all downstream
design decisions.

```
A work item is a scheduler-owned request to advance one substrate output for
one subject, under one policy version and one work basis.

A user action does not directly create work items. It creates foreground
interest or an explicit request. The control plane and the owning substrate
authority convert those signals into enqueue, promote, cancel, or no-op
decisions.

Deck load does not perform expensive work inline. It queries current readiness
synchronously. If readiness is unknown or pending, it requests interactive
readiness work through the control plane and receives a bounded response:
ready | pending | blocked | unavailable. It does not wait for work to finish.

Effective priority is computed from urgency, value, cost, and age at scheduling
time. It is not stored as a column.

Work identity is idempotent. The same subject, work kind, target, policy
version, and basis fingerprint must not have duplicate live work items
simultaneously.

Large work is sliced when the handler supports checkpointing. Checkpoints bind
to the work basis, not necessarily to file_identity_id. Hash work checkpoints
against a source observation basis before file identity exists.

Resource budgets are bound to resource budget groups, not merely logical
sources. A resource budget group may represent storage IO, decode capacity,
CPU, GPU, or network IO. A work item may require multiple resource budget
groups simultaneously. It may run only when all required groups have capacity.

Readiness work produces bounded answers for foreground requests. Analysis work
must never be required for basic deck-load readiness unless the target
explicitly requires a derived artifact.

Discovery is a scan session, not a per-file work item. Scan sessions produce
source file observations; downstream work items follow from those observations.

The scheduler owns selection, leasing, priority computation, budget enforcement,
and cancellation flow. It does not own media facts, role assignments, readiness
rows, analysis artifacts, or source observations. Workers write only through
the authority that owns each output table.

Foreground interest has a TTL. Scheduler urgency from foreground interest
expires unless refreshed by current UI or session context. Foreground interest
is session-scoped, not durable product state.
```

---

## Definitions

**Browse-ready**

A library item is browse-ready when Dekzer has enough substrate facts to project
an honest row in the relevant browser context. Browse-ready does not mean
deck-ready. It does not mean fully analyzed. It does not mean identity-resolved.
An item may be browse-ready with a visible `pending` state marker if the row is
provisional.

**Readiness**

Readiness answers: can this item be used for target X right now?

Readiness work must be bounded or scheduled in slices small enough to produce
a response on a foreground timeline. Readiness must not require waveform,
BPM, beatgrid, or stem analysis unless the target explicitly depends on that
artifact.

**Analysis**

Analysis answers: what derived artifact or fact improves preparation or
playback?

Analysis work (waveform, BPM, key, beatgrid, loudness, stems) is expensive,
may be very large, must be chunkable for large files, and is never on the
critical path for deck-load readiness unless the deck target explicitly requires
the artifact.

Readiness and analysis are independent work classes with independent scheduling
lanes. A deck load request must not block on analysis completion.

---

## Scheduling Doctrine

### 1. Work items are not a FIFO queue

Work items carry priority inputs. The scheduler computes effective priority
at scheduling time from urgency, value, cost, and age. Items are not ordered
by arrival time.

No implementation may process work items strictly in insertion order without
scheduler-computed priority. That is a queue, not a scheduler.

### 2. Priority is four-dimensional

```
Urgency   How time-sensitive is this to the user right now.
          A deck load response is urgent. Background hashing is not.

Value     What product state does completion unlock.
          A probe that unlocks browse visibility for many items is high value.
          A waveform job for a file the user has never touched is low value.

Cost      Inverse of resource consumption. Cheaper work is preferred at equal
          urgency and value. This bias keeps the system responsive.

Age       How long this item has waited. Prevents permanent starvation.
          Age boost is monotonic and policy-defined. The first implementation
          may use linear aging. The function is not substrate law.
          Age boost is clamped: it cannot promote a low-urgency item above
          an interactive-lane item.
```

Effective priority is a weighted combination of these four dimensions. Weights
and aging rates are scheduler policy, configurable per lane. They must not be
hardcoded in schema or in work item rows.

The schema stores the inputs. The scheduler owns the function.

### 3. Priority is context- and cost-sensitive, not type-sensitive

`audio > video > image` is too crude. It produces the wrong behavior on real
hardware.

In a normal DJ preparation workflow, audio performance readiness receives a
higher base value than video and image work. That base value is one input.
Urgency, foreground context, work cost, source budget, and age can all
override it.

Illustrative examples (not fixed law):

```
small foreground audio readiness    >  large background audio analysis
video candidate in selected folder  >  background audio analysis for untouched file
image needed for visible artwork    >  background audio waveform
```

Do not encode a fixed priority chain across media types. Encode a priority
function that receives context as input.

### 4. Work kinds declare their lane; compatible kinds may share a lane

Work kinds must declare their lane, resource requirements, cost model, and
checkpoint behavior. A lane may contain multiple compatible work kinds.

Incompatible work kinds — those where one would systematically block
higher-value progress of the other — must not share a lane.

Example: `sniff` and `probe` may share the `recognition` lane. They are both
recognition-class work. The scheduler can still prefer cheap sniff over
expensive probe within that lane using the priority function.

Example: `waveform` must not share the `recognition` lane. A giant waveform
job would block cheap sniff and probe work that unlocks browse visibility.

### 5. Expensive work must not block discovery or projection

The scanner must continue discovering the library even when probe, hash, or
analysis work is in-flight on large files.

The projection layer must continue serving browse rows from already-classified
items even when expensive work is pending on other items.

Browse rendering must not trigger work item creation as a side effect.
Deck load must not perform expensive probe, analysis, or readiness evaluation
inline. See the Deck Load Semantics section below.

### 6. Large work must be resumable where the operation allows it

```
Bounded single-attempt work (runs to completion or fails; not checkpointed):
  sniff
  embedded tag read
  container metadata probe — only when the probe policy defines max bytes,
    max wall time, and cancellation behavior

  A probe is bounded only when its policy explicitly constrains it.
  An unbounded probe must run in a constrained lane with conservative resource
  budgets and a strict lease timeout.

Chunkable (progress can be saved and resumed):
  content hash
  waveform overview
  loudness scan
  waveform detailed pass
  BPM/key analysis on long files
```

For chunkable operations, the substrate stores a checkpoint. The scheduler can
run a slice, yield to higher-priority work, and resume in a subsequent slice.

Checkpoints bind to the work basis, not necessarily to `file_identity_id`.
For hash work (which produces the identity), the checkpoint binds to the source
observation basis. See Work Basis and Checkpoint Staleness below.

### 7. Resource budgets are bound to resource budget groups

A budget governs:

```
max concurrent units of this resource kind
max bytes processed per scheduling slice (IO-bound work)
max milliseconds per scheduling slice
cooldown between slices
```

Budgets are per resource budget group, not per logical source. Multiple
registered sources may share a physical storage device, CPU pool, or decoder
pool. The group budget is the binding constraint.

A work item may require multiple resource budget groups simultaneously.
It may run only when all required groups have capacity. See Resource
Requirement Resolution below.

### 8. Discovery is a scan session, not a per-file work item

Source-wide directory traversal is fundamentally different from a per-file
probe or analysis job. It is a long-running session that continuously produces
source file observations.

Discovery is tracked as scan runs. The scanner budgets its IO through the
appropriate resource budget group. The per-file work items (sniff, probe, hash,
classify, readiness, analysis) are produced as downstream consequences of
discovery, not as part of the scan session itself.

Do not model source-wide directory traversal as a `work_items` row.

Scan runs are also long-running, cancelable, budgeted, and recoverable. A scan
can be abandoned just as work items can be abandoned. A removable source can
vanish mid-scan. A policy change can supersede a scan. The `scan_runs` table
carries enough lifecycle structure to handle these cases:

```
last_heartbeat_at: the scanner refreshes this periodically; a stale heartbeat
  signals an abandoned scan, just as stale heartbeat_at signals an abandoned
  work item.
cancelled_at / cancellation_reason: a scan cancelled because the source was
  removed, superseded by a newer scan, or stopped by user action records the
  reason.
scan_policy_version: allows detection of scans made under a superseded policy.
  A new scan under a changed policy supersedes any in-flight scan under the previous
  version.
basis_scope: narrows what the scan covers; a targeted scan does not imply full
  source coverage.
resource_budget_group_id: the budget group governing this scan's IO. Stored at
  scan-start so diagnostics remain accurate even after source assignment changes.
```

### 9. The scheduler does not own output tables

The scheduler owns selection, leasing, priority computation, budget
enforcement, and cancellation flow.

The scheduler must not write to:

```
source_files
media_probe_results
media_streams
signature_sniff_results
library_item_roles
library_item_role_events
item_readiness
file_identities
scan_runs
```

Workers write outputs only through the authority that owns each table.
If the scheduler appears to need direct write access to an output table,
that is a sign of ownership boundary violation, not a scheduler requirement.

### 10. Foreground interest expires

Foreground interest is session-scoped. It is not durable product state.

Scheduler urgency from foreground interest expires unless refreshed by
current UI or session context. An item the user selected five minutes ago
in a panel they have since navigated away from does not continue receiving
interactive priority.

Foreground interest must be explicitly refreshed to remain active.
The implementation may use a session heartbeat, a TTL field, or an
event-driven model. The doctrine requirement is that stale foreground interest
does not permanently occupy scheduler capacity.

---

## Work Lanes

Lanes carry their own budget and aging configuration. A work item is assigned
exactly one lane.

```
interactive
  User-driven foreground work. Highest urgency class.
  An item belongs in this lane when it was produced by a direct user action
  or is required to unblock a UI operation the user is actively attempting.

  Valid triggers for interactive-lane enqueue or promotion:
    user explicitly requests probe or analysis on a file
    user selects a file and the detail panel requires readiness facts not
      yet available
    deck load path requests readiness work via the control plane

  Invalid triggers (must not promote to interactive):
    passive rendering of a disabled control
    any background scan event
    browse rendering of any kind
    policy change events
    file selection in a non-focused or background panel

  Interactive-lane abuse directly harms UI responsiveness.

browse-readiness
  Minimum facts to make the library tree useful during active scan.
  Work in this lane may produce:
    directory revealability facts (folder chevron state)
    provisional source file recognition facts (extension + sniff)
    readiness summary placeholders (pending | unknown | blocked)
  Work in this lane may cause the classifier authority to produce provisional
    role claims needed for honest browser rows. The lane itself does not own
    or write role claims; outputs are written only by their owning authorities.
  Work in this lane must not produce:
    full waveform
    BPM, key, or beatgrid analysis
    stems
    durable content identity unless already available cheaply
    visual thumbnails
  Items here target audio and video performance candidates in sources the
  user is currently or recently browsing.

recognition
  Sniff and probe work on candidates with media-relevance evidence.
  Not user-triggered. Needed to confirm, upgrade, or retract provisional
  classifications. Sniff and probe may share this lane. They are compatible:
  the scheduler prefers cheap sniff over expensive probe via the priority
  function.

identity
  Content hash computation and file_identity_id binding.
  Deferred. Must not block discovery, sniff, probe, or browse-readiness lanes.
  Prioritizes items with pending user assignments that require identity binding.

analysis
  Waveform, BPM, key, beatgrid, loudness, stem separation.
  Expensive. Must be chunkable for large files.
  Not required for basic deck-load readiness.
  Large files here must checkpoint and yield to higher-priority lanes.

visual
  Image and video visual asset processing.
  Lower default base value than audio analysis in DJ workflows.
  May be promoted by user action or active visual workflow context.
  The visual lane may exist before a top-level Visual Assets UI surface exists.
  Work in this lane is only scheduled when substrate evidence, user action, or
  an explicit visual workflow requires it. Do not schedule broad visual
  processing merely because image files exist.

diagnostic
  Explanatory background work: re-sniffing rejected items, checking probe
  failures, verifying unavailable files, repairing known inconsistencies.
  Lowest default priority. Must not starve; must not preempt useful work.
```

There is no dedicated `readiness` lane. `readiness` is a work kind, not a lane.
Readiness work is assigned to `interactive`, `browse_readiness`, `diagnostic`, or another defined lane depending on
trigger and urgency. The work kind
records what the work produces; the lane expresses the resource and urgency
policy under which it runs.

---

## Deck Load Semantics

Deck load has two separate paths. They must not be conflated.

**Query path (synchronous, no work created)**

```
1. Deck load request arrives.
2. Query item_readiness for (library_item_id, audio_deck) or (video_deck).
3. If status = ready    → proceed with load.
   If status = blocked  → return blocked + reason to UI. No work created.
   If status = degraded → proceed with warning.
   If status = pending  → proceed to request path.
   If no row exists     → proceed to request path.
   If file unavailable  → return unavailable to UI.
```

**Request path (via control plane, not inline)**

```
4. Deck load path sends a readiness request to the control plane.
   It does not create a work_items row directly.
5. Control plane checks for an existing live readiness work item for this
   subject and target.
   If one exists: promote it to interactive lane if not already there.
   If none exists: ask the readiness evaluator to enqueue one in the
     interactive lane.
6. Control plane enforces idempotence before any insert.
7. Deck load path receives a bounded synchronous response: pending.
8. UI presents a pending/loading state. It does not block.
9. When readiness evaluation completes, the deck receives an async
   notification and the query path re-runs.
```

Deck load never performs probe, analysis, or readiness evaluation inline.
Deck load never creates `work_items` rows directly.
Deck load never waits unboundedly for work to complete.

---

## Work Item Creation Ownership

Only substrate authorities create or mutate work items. User actions produce
foreground interest signals or explicit requests. The control plane and the
owning pipeline authority convert those signals into enqueue, promote, cancel,
or no-op decisions.

```
Scanner (via scan session output):
  Queues sniff work for files with unknown or absent extensions.
  Queues identity work for files with pending user assignments needing hash.

Sniffer:
  Queues probe work when signature_family indicates media.

Probe Worker:
  Queues classify work when probe produces new stream facts.
  Queues readiness evaluation work for affected (item, target) pairs.

Classifier:
  Queues analysis work for newly accepted performance items.
  Queues readiness evaluation work when role assignments change.

Readiness Evaluator:
  May create and own readiness work items when requested by the control
  plane, probe worker, classifier, or policy engine.
  Does not create downstream probe, classify, or analysis work as a side
  effect of evaluating readiness. Its only write outputs are item_readiness
  rows and the readiness work items that produce them.

Control Plane:
  Enqueues or promotes work items in response to foreground interest signals,
  deck load requests, and explicit user requests.
  Enforces idempotence before inserting.

Policy Engine:
  Queues re-probe or re-analysis work when probe policy or analysis policy
  version changes.

Forbidden creators:
  Renderer
  Projection / Query Layer
  Browse rendering path
  Deck load path (directly)
```

---

## Classify Work Item Shape

Classify work items have a specific canonical shape. Do not leave it open to
interpretation.

```
subject_kind    = source_file
subject_id      = source_files.source_file_id
basis_kind      = probe_result
basis_fingerprint = probe_result_id + classifier_policy_version (namespaced)
target_kind     = classification
target_key      = media_roles
policy_version  = classifier_policy:v{N}
```

Library item creation and role assignment are the **outputs** of classify work,
not the subjects. The subject is the source file whose probe facts triggered
reclassification.

---

## Work Basis and Checkpoint Staleness

Every work item records the input facts it depends on. This is the work basis.
A checkpoint or live work item is stale when current substrate facts no longer
match the recorded basis.

**Work basis kinds:**

```
source_observation
  The work depends on path observation facts, not content identity.
  Used for hash work, which runs before file_identity_id exists.

  basis_fingerprint components:
    source_id
    source_file_id
    relative_path
    size_bytes
    mtime_ns
    platform_file_id (when available)
    observation_policy_version (tracks changes to how observations are taken)

file_identity
  The work depends on confirmed file content identity.
  Used for analysis, waveform, readiness (post-identity).

  basis_fingerprint components:
    file_identity_id
    policy_version (namespaced, e.g. waveform_policy:v2)

probe_result
  The work depends on a specific probe result.
  Used for classifier work triggered by a specific probe.

  basis_fingerprint components:
    probe_result_id
    classifier_policy_version (namespaced)

policy_epoch
  The work is a policy-driven re-evaluation across items.

  basis_fingerprint components:
    affected_scope_id
    policy_version (namespaced)
```

**Staleness rule:**

```
A work item is stale when current substrate facts for its basis_kind no
longer match its recorded basis_fingerprint.

A checkpoint is valid only when its recorded basis_fingerprint still matches
the current work item's basis_fingerprint.

If a checkpoint is stale, it is discarded. The work item restarts from the
beginning, not from the stale checkpoint.
```

`observation_policy_version` is included in the source observation fingerprint
because what constitutes a stable observation evolves. When inode, birthtime,
case normalization, or new identity signals are introduced, prior fingerprints must
not silently represent the new semantics.

---

## Work Item Idempotence

The same work must not have duplicate live entries simultaneously.

A **live** work item has `status IN (pending, scheduled, running)`.

Uniqueness rule:

```sql
UNIQUE (work_kind, subject_kind, subject_id, target_kind, target_key,
        policy_version, basis_fingerprint)
WHERE status IN ('pending', 'scheduled', 'running')
```

`target_kind` and `target_key` are `NOT NULL DEFAULT 'none'`. SQL uniqueness
with nullable columns permits duplicates in most databases (including SQLite,
where NULLs are considered distinct in unique indexes). Using sentinel `'none'`
values instead of NULL makes the dedupe key work correctly.

Completed, cancelled, stale, and failed rows may coexist with new live rows
for the same work identity, allowing re-work after failure or policy change
without purging history.

Before creating a work item, the creating authority must check for an existing
live row. If one exists, it promotes or updates it rather than inserting a
duplicate.

---

## Resource Requirement Resolution

A work item may require multiple resource budget groups simultaneously. It may
run only when all required groups have capacity.

Resource requirements are not stored per work item. They are resolved at
scheduling time from `work_kind`, subject source context, `source_resource_assignments`,
and policy. This avoids stale requirement rows when source assignments change.

Default resolution by work kind:

```
sniff:            storage_device
probe:            storage_device, decoder_pool
hash:             storage_device, cpu_pool
classify:         cpu_pool, database_write
readiness:        cpu_pool, database_write
waveform:         storage_device, decoder_pool, cpu_pool, database_write
bpm_key:          storage_device, decoder_pool, cpu_pool
beatgrid:         cpu_pool
loudness:         storage_device, decoder_pool, cpu_pool
visual_process:   storage_device, cpu_pool (+ gpu_pool when available)
```

If resolution cannot determine required groups (e.g. source has no assignment),
the work item uses a default fallback group for each resource kind.

The scheduler must not start a work item when any required resource budget
group has no remaining capacity. It must not partially acquire resources and
wait for the rest — it must either acquire all or defer.

When a work item requires multiple resource budget groups, the scheduler must
evaluate and reserve all required groups in deterministic `resource_budget_group_id`
ascending order inside one scheduling transaction. It must not acquire group A
in one transaction and group B in a subsequent transaction. Deterministic ordering prevents
scheduler-level deadlocks when two workers compete for overlapping group sets.

---

## Substrate Schema

These tables must be present before any pipeline implementation that expects
correct scheduling behavior. They are not the full scheduler runtime.

```sql
-- Scan sessions. Directory traversal is tracked here, not in work_items.
scan_runs (
  scan_run_id             INTEGER PRIMARY KEY,
  source_id               INTEGER NOT NULL,
  run_kind                TEXT NOT NULL,
    -- CHECK (run_kind IN ('full', 'incremental', 'targeted'))
  status                  TEXT NOT NULL,
    -- CHECK (status IN ('running', 'complete', 'failed', 'cancelled'))
  scan_policy_version     TEXT NOT NULL,       -- namespaced; e.g. scan_policy:v1
  started_by              TEXT NOT NULL,       -- scanner | user_request | policy_engine
  basis_scope             TEXT,                -- path prefix or NULL for full-source scan
  started_at              INTEGER NOT NULL,
  last_heartbeat_at       INTEGER,             -- scanner refreshes; detect stalled scans
  completed_at            INTEGER,
  cancelled_at            INTEGER,
  cancellation_reason     TEXT,
    -- CHECK (cancellation_reason IN (
    --   'source_removed', 'policy_change', 'user_cancelled', 'superseded', NULL))
  files_discovered        INTEGER,
  files_updated           INTEGER,
  files_removed           INTEGER,
  error_code              TEXT,

  -- Resource budget group used for scan IO. Resolved at scan-start from
  -- source_resource_assignments. Stored here so diagnostics can answer
  -- which budget group governed a scan even after assignment changes.
  resource_budget_group_id INTEGER REFERENCES resource_budget_groups(resource_budget_group_id)
)

-- Resource budget groups. One row per constraining resource.
-- Bound to resource kind, not to logical source.
resource_budget_groups (
  resource_budget_group_id    INTEGER PRIMARY KEY,
  resource_kind               TEXT NOT NULL,
    -- CHECK (resource_kind IN (
    --   'storage_device', 'cpu_pool', 'decoder_pool', 'gpu_pool',
    --   'network_mount', 'database_write', 'unknown'))
  profile_kind                TEXT NOT NULL,
    -- CHECK (profile_kind IN (
    --   'nvme', 'sata_hdd', 'sata_ssd', 'usb2', 'usb3', 'usb3_ssd',
    --   'network', 'cpu_default', 'gpu_default', 'removable_unknown', 'unknown'))
  label                       TEXT,
  max_concurrent_units        INTEGER NOT NULL,
  budget_bytes_per_slice      INTEGER,
    -- null for non-IO resource kinds (cpu_pool, database_write, etc.)
    -- database_write does not use budget_bytes_per_slice; see note below.
  max_ms_per_slice            INTEGER NOT NULL,
  cooldown_ms                 INTEGER NOT NULL

  -- Note: database_write budget group.
  -- database_write gates write transactions and projection rebuild pressure.
  -- It does not express IO volume in bytes. It uses max_concurrent_units
  -- (max concurrent write transactions) and max_ms_per_slice (max transaction
  -- duration policy). budget_bytes_per_slice must be NULL for this kind.
  -- Do not assign byte budgets to database_write rows.
)

-- Links a registered source to its resource budget groups.
-- History preserved. Only current assignments (superseded_at IS NULL) are active.
source_resource_assignments (
  source_resource_assignment_id INTEGER PRIMARY KEY,
  source_id                   INTEGER NOT NULL,
  resource_budget_group_id    INTEGER NOT NULL
    REFERENCES resource_budget_groups(resource_budget_group_id),
  assignment_kind             TEXT NOT NULL,
    -- CHECK (assignment_kind IN ('storage', 'decode', 'network'))
  assignment_confidence       TEXT NOT NULL,
    -- CHECK (assignment_confidence IN ('detected', 'configured', 'inferred', 'default'))
  assigned_at                 INTEGER NOT NULL,
  superseded_at               INTEGER,
  UNIQUE (source_id, assignment_kind) WHERE superseded_at IS NULL
)

-- Work items. Scheduler-owned priority at scheduling time.
work_items (
  work_item_id                INTEGER PRIMARY KEY,

  -- Work kind. scan_directory is not valid here — use scan_runs.
  work_kind                   TEXT NOT NULL,
    -- CHECK (work_kind IN (
    --   'sniff', 'probe', 'hash', 'classify', 'readiness',
    --   'waveform', 'bpm_key', 'beatgrid', 'loudness', 'visual_process'))

  -- Subject: canonical identity of what this work acts upon.
  -- subject_kind + subject_id is the authoritative reference.
  -- The creating authority must validate subject_kind + subject_id on insert.
  -- Tests must cover invalid subject references for each work_kind.
  --
  -- Relational convenience ids below are derivable aliases of subject_id.
  -- Invariant: for subject_kind = 'source_file', source_file_id = subject_id.
  --            for subject_kind = 'library_item', library_item_id = subject_id.
  --            for subject_kind = 'file_identity', file_identity_id = subject_id.
  -- Rows where convenience ids contradict subject_id are invalid and must not
  -- be inserted. Only one convenience id should be non-null; it must equal
  -- subject_id when populated.
  --
  -- Because subject_id is polymorphic, database foreign keys cannot fully
  -- enforce subject validity. The creating authority owns this invariant.
  subject_kind                TEXT NOT NULL,
    -- CHECK (subject_kind IN (
    --   'source', 'source_directory', 'source_file',
    --   'file_identity', 'library_item'))
  subject_id                  INTEGER NOT NULL,

  -- Relational FK conveniences. At most one should be non-null.
  -- When non-null, must equal subject_id. Null when not applicable.
  source_file_id              INTEGER REFERENCES source_files(source_file_id),
  library_item_id             INTEGER REFERENCES library_items(library_item_id),
  file_identity_id            INTEGER REFERENCES file_identities(file_identity_id),

  -- Target: what this work is producing or checking.
  -- 'none' / 'none' for work kinds without a specific target (e.g. hash, sniff).
  target_kind                 TEXT NOT NULL DEFAULT 'none',
    -- CHECK (target_kind IN (
    --   'classification', 'readiness', 'analysis_profile', 'identity', 'none'))
  target_key                  TEXT NOT NULL DEFAULT 'none',
    -- 'audio_deck' for readiness, 'waveform_overview:v1' for analysis, 'none' otherwise

  -- Basis: input facts this work depends on. Used for staleness and deduplication.
  basis_kind                  TEXT NOT NULL,
    -- CHECK (basis_kind IN (
    --   'source_observation', 'file_identity', 'probe_result',
    --   'library_item', 'policy_epoch'))
  basis_fingerprint           TEXT NOT NULL,

  -- Policy version for this work item. Must be namespaced by policy family.
  -- Valid examples: sniff_policy:v1, probe_policy:v1, hash_policy:v1,
  --                 waveform_policy:v2, classifier_policy:v1
  -- Invalid: v1, 1, latest
  -- Validation is performed by the work authority on insert, not by SQLite CHECK,
  -- because policy families are open-ended. The authority must reject any
  -- policy_version string that does not contain a namespace separator (':').
  policy_version              TEXT NOT NULL,

  -- Provenance: which scan run caused this work item to be created, if any.
  -- Nullable. Set when downstream work follows directly from a scan observation.
  -- Allows diagnostics to answer: which scan run caused this sniff/hash/probe?
  scan_run_id                 INTEGER REFERENCES scan_runs(scan_run_id),

  -- Scheduling
  lane                        TEXT NOT NULL,
    -- CHECK (lane IN (
    --   'interactive', 'browse_readiness', 'recognition',
    --   'identity', 'analysis', 'visual', 'diagnostic'))
  cost_class                  TEXT NOT NULL,
    -- CHECK (cost_class IN ('tiny', 'small', 'medium', 'large', 'huge', 'unknown'))
  estimated_bytes             INTEGER,         -- null for non-IO work

  -- Creation and mutation timestamps.
  created_at                  INTEGER NOT NULL,
  updated_at                  INTEGER NOT NULL,
    -- CHECK (updated_at >= created_at)
    -- Must be updated on every status change, lease change, retry, or cancellation.
  created_by                  TEXT NOT NULL,
    -- CHECK (created_by IN (
    --   'scanner', 'sniffer', 'probe_worker', 'classifier',
    --   'readiness_evaluator', 'control_plane', 'policy_engine'))

  -- Scheduling control
  not_before                  INTEGER,         -- null = schedulable immediately

  -- Lease. Null when not leased.
  lease_owner                 TEXT,            -- worker identity string
  leased_at                   INTEGER,
  lease_expires_at            INTEGER,
  heartbeat_at                INTEGER,         -- worker refreshes; scheduler detects abandonment

  -- Cooperative cancellation.
  -- pending/scheduled items: both timestamps written together; status → cancelled
  -- running items: cancellation_requested_at written first; worker confirms at yield
  cancellation_requested_at   INTEGER,
  cancelled_at                INTEGER,
  cancellation_reason         TEXT,
    -- CHECK (cancellation_reason IN (
    --   'source_removed', 'file_changed', 'superseded', 'policy_change',
    --   'user_cancelled', 'duplicate', NULL))

  -- Status
  status                      TEXT NOT NULL,
    -- CHECK (status IN (
    --   'pending', 'scheduled', 'running', 'complete',
    --   'failed', 'cancelled', 'stale'))
  started_at                  INTEGER,
  completed_at                INTEGER,

  -- Retry
  last_error_code             TEXT,
  last_failed_at              INTEGER,
  retry_count                 INTEGER NOT NULL DEFAULT 0,
  max_retries                 INTEGER NOT NULL DEFAULT 3

  -- Idempotence partial unique index (declared in migration):
  -- UNIQUE (work_kind, subject_kind, subject_id, target_kind, target_key,
  --         policy_version, basis_fingerprint)
  -- WHERE status IN ('pending', 'scheduled', 'running')
)

-- Progress checkpoints for chunkable work.
-- One row per work item (PRIMARY KEY enforces this).
-- Valid only when basis_fingerprint matches work_item.basis_fingerprint at resume.
work_item_progress (
  work_item_id                INTEGER PRIMARY KEY REFERENCES work_items(work_item_id),
  basis_fingerprint           TEXT NOT NULL,   -- must match work_items.basis_fingerprint
  file_identity_id            INTEGER REFERENCES file_identities(file_identity_id),
    -- null for hash work (identity does not yet exist)
  completed_bytes             INTEGER,
  completed_frames            INTEGER,
  completed_duration_ms       INTEGER,
  checkpoint_token            BLOB,            -- opaque; interpreted by work_kind handler
  last_checkpoint_at          INTEGER NOT NULL
)
```

---

## Work Item Lifecycle

```
Pending
  work_items row inserted with status = pending.
  created_by identifies the creating authority.
  basis_fingerprint computed from current input facts.

Scheduled
  Scheduler selects item based on priority computation.
  status → scheduled.
  lease_owner, leased_at, lease_expires_at written atomically.
  No two workers may hold the lease simultaneously.

Running
  Worker acquires item. status → running. updated_at written.
  Worker writes heartbeat_at periodically.
  Chunkable work: checkpoints written to work_item_progress during execution.

Completed
  Worker validates lease ownership and current basis before committing outputs.
  A worker whose lease has expired must not write outputs; it must abort and
  allow lease recovery to re-queue the item if retries remain.
  A worker whose basis is stale (current substrate facts no longer match
  basis_fingerprint) must not write outputs; it must mark the item stale
  and stop.
  status → complete, completed_at written. Lease fields cleared.
  updated_at written.
  Progress checkpoint: may be deleted or retained for diagnostics according to
  policy, but must not be used for future work unless the work_item_id matches
  a live row with the same basis_fingerprint. The default is to delete the
  progress row on completion to keep the table clean; diagnostic retention
  is an explicit policy choice.

Failed (retryable)
  Worker encountered an error.
  last_error_code and last_failed_at written.
  retry_count < max_retries: status → pending, retry_count incremented.
  Lease fields cleared. updated_at written.

Failed (terminal)
  retry_count >= max_retries.
  status remains failed. updated_at written.
  Visible in diagnostics. Requires user action or policy change to re-queue.

Cancelled — pending or scheduled item
  External authority triggers cancellation.
  cancellation_requested_at and cancelled_at written together in one operation.
  status → cancelled immediately. updated_at written.
  No cooperative step needed; item has not started.
  Lease fields cleared.

Cancelled — running item (cooperative)
  External authority writes cancellation_requested_at. updated_at written.
  status remains running.
  Worker observes cancellation_requested_at at next checkpoint or yield boundary.
  Worker stops safely. cancelled_at written. status → cancelled. updated_at written.
  Lease fields cleared. Progress checkpoint discarded.

Stale
  At execution time, current substrate facts for basis_kind no longer match
  basis_fingerprint.
  status → stale. updated_at written.
  A new work item with the updated basis may be enqueued by the owning authority.

Lease Recovery
  Scheduler detects: status = running AND lease_expires_at < now AND
  heartbeat_at is stale.
  Scheduler treats item as abandoned.
  retry_count < max_retries: status → pending, lease cleared, retry_count++,
    updated_at written.
  Terminal: status → failed.
  A late worker whose lease has expired and whose item has been recovered
  must not commit outputs if it completes. Workers must validate lease
  ownership before any output write. A write from a lease-expired worker
  is silently dropped or causes an integrity error; it must not produce
  duplicate or stale substrate facts.
```

---

## Starvation Prevention

Large items in the analysis lane must not wait indefinitely.

```
Every scheduling cycle, items that have not been scheduled receive an age boost.
Age boost rate is configured per lane.
Age boost is monotonic. The first implementation may use linear aging.
  The function is scheduler policy, not substrate law.
Age boost is clamped: it cannot promote a low-urgency item above an
  interactive-lane item at the same urgency class.
Age boost is computed at scheduling time from created_at and lane_aging_rate.
  It is never stored as a column in work_items.
```

---

## Forbidden Patterns

```
Processing work items in strict insertion order without scheduler priority
Deck load path creating work_items rows directly
Deck load path performing probe, analysis, or readiness evaluation inline
Deck load path waiting unboundedly for work completion
Browse rendering creating work_items rows as a side effect
Projection / query layer creating work_items rows as a side effect
Renderer accessing substrate tables directly
Scheduler writing to any output table (probe results, roles, readiness, identities)
Storing effective_priority as a column in work_items
Storing age_boost as a column in work_items
Running hash work synchronously during discovery
Running analysis work synchronously during probe
Running analysis work as a prerequisite for basic deck-load readiness
Running chunkable work without a checkpoint schema
Resuming from a checkpoint whose basis_fingerprint differs from the work item
Applying resource budgets per logical source when multiple sources share a group
Promoting passive disabled-control rendering to interactive-lane work
Inserting a duplicate live work item instead of promoting the existing one
Modeling source-wide directory traversal as a work_items row (use scan_runs)
Using target_kind = NULL or target_key = NULL in the dedupe key (use 'none')
Using an un-namespaced policy_version such as 'v1' or '1' or 'latest'
Acquiring some required resource budget groups and blocking for the rest
Inserting a work_items row where a non-null convenience id differs from subject_id
Populating more than one convenience id (source_file_id / library_item_id /
  file_identity_id) on a single work_items row
```

---

## Acceptance Bar

The implementation is correct only when all of the following hold:

- For a 4 GB audio file (roughly equivalent to 70–95 CD-quality FLAC tracks
  averaging 40–55 MB each), that batch of tracks reaches browse-ready status
  while the large file has long-running identity and/or analysis work active
  in a lower-priority lane
- A large analysis job does not block a cheap sniff in the recognition lane
- Removing a source cancels pending and scheduled items immediately; running
  items receive cancellation_requested_at and stop cooperatively at the next
  yield boundary
- A work item whose basis_fingerprint does not match current substrate facts
  is treated as stale and not executed
- Chunkable work can be resumed from a checkpoint without re-processing
  completed bytes or frames
- A checkpoint whose basis_fingerprint differs from the current work item is
  discarded; the work item restarts
- Hash work checkpoints against source_observation basis before
  file_identity_id exists; this is not a schema error
- source_observation basis_fingerprint includes relative_path and
  observation_policy_version in addition to size and mtime
- Resource budgets apply per resource_budget_group, not per registered source;
  sources sharing a physical device share one storage budget group
- A work item requiring multiple resource budget groups acquires all or none;
  it does not partially acquire and block
- Deck load queries readiness synchronously and returns ready/pending/blocked/
  unavailable without creating work or blocking on work completion
- Deck load may request interactive readiness work via the control plane; the
  control plane enforces idempotence before any insert
- Browse rendering produces no work items as a side effect
- Interactive-lane items are not created for background scan events or passive
  control rendering
- Two live work items with the same dedupe key cannot coexist; the creating
  authority promotes the existing one instead
- target_kind and target_key are 'none' (not NULL) for work kinds without a
  specific target; dedupe constraints hold correctly
- policy_version is namespaced (e.g. sniff_policy:v1); bare version strings
  are rejected
- Age boost prevents permanent starvation without allowing low-urgency items
  to preempt interactive items
- The scheduler priority function (weights, aging rates) is configurable
  policy; no weights are hardcoded in schema or application constants
- An abandoned running item (expired lease, stale heartbeat) is recovered to
  pending without external intervention
- A retryable failure increments retry_count and returns to pending; terminal
  failures remain failed and appear in diagnostics
- Diagnostics show cancelled, stale, permanently-failed, and lease-abandoned
  items with reasons
- Basic deck-load readiness does not require waveform, BPM, beatgrid, or stems
  completion unless the target explicitly depends on those artifacts
- The scheduler writes to no output table; it only reads for scheduling
  decisions and writes lease/status/cancellation fields on work_items
- A worker whose lease has expired or whose basis is stale cannot commit
  outputs; it must abort and allow lease recovery or stale handling to proceed
- On completion, the progress checkpoint row is deleted by default; retention
  for diagnostics is an explicit policy choice and must not allow a stale
  checkpoint to be resumed as if it were current
