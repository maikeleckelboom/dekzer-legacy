# Library Preparation Substrate v1

**Status:** ACCEPTED FOR V1 SCHEMA IMPLEMENTATION
**Location:** `docs/decisions/library-preparation-substrate-v1.md`

---

## Context

The library system needs a durable schema that can carry both the scanning
phase (source inventory) and the preparation phase (what we know about each
track and whether it is ready). These two phases are distinct in time and
in ownership, but they share a substrate: the track identity layer sits
between them.

The previous schema epoch used a `LibraryAssets` table as the primary object
identity. That model conflates file-level observation with collection-level
identity. A file is not a track. A track is a collection identity decision,
which may be backed by one or many files across one or many sources.

This decision defines the v1 substrate that replaces the legacy epoch. It is
not the full product schema. It is the first real slice that carries source
inventory, track identity, attachments, preparation work, and the readiness
projection. Everything after this builds on it.

---

## Vocabulary Settlement

These terms are the canonical names for their concepts throughout the codebase,
documentation, and schema. They are not interchangeable.

```text
source             — an observed root (folder, drive, disc image, network share)
source file        — a file found on disk within a source
observed file tags — file-level metadata extracted during scan (ID3, Vorbis, etc.)
track              — durable collection identity; may survive across source changes
attachment         — a media payload owned by a track (primary file or alternate)
location           — the connection from an attachment back to observed source material, with source/path snapshots when the live source row is absent
preparation facet  — an independently evaluated dimension of readiness
artifact           — evidence produced by a work run (analysis output, grid, etc.)
work run           — an execution context that produced artifacts
work item          — a single unit of processing within a work run
readiness          — a projected verdict computed from facet evidence; not a stored field
```

Vocabulary scope rules:

```text
preparation  — durable substrate concept; used in SQL table names and Rust types
               that live outside a domain folder context
prep         — ergonomic shorthand; used in renderer folders, API namespaces,
               and Rust types that already carry folder context
readiness    — always distinct from preparation; readiness is a projection outcome,
               not a process state
facet        — always scoped; a facet is meaningless without its kind
artifact     — always owned by a work item; artifacts are not freestanding objects
```

---

## Substrate Layers

The v1 substrate has four layers. Higher layers depend on lower layers. Lower layers must not depend on higher layers.

```text
Layer 1: Source inventory
  sources, source_locations, source_directories,
  source_files, source_facts, observed_file_tags

Layer 2: Track identity
  tracks, track_metadata, track_attachments,
  track_attachment_locations, track_identity_candidates,
  track_identity_resolution_results, track_identity_result_options,
  track_identity_decisions, track_identity_suppressions

Layer 3: Preparation work
  work_runs, work_items, artifacts, artifact_supersessions,
  artifact_inline_payloads, artifact_file_store_entries,
  preparation_facets, preparation_facet_dependencies,
  preparation_facet_states, resolved_preparation_targets

Layer 4: Browser projections
  navigation_rows, track_browser_rows
```

Layer 4 is materialized from layers 1–3. It is not the source of truth.
Its entries are rebuilt or invalidated when their upstream inputs change.
Layer 1 must know nothing about preparation, identity decisions, or projections.

---

## In Scope for v1

```text
Source inventory
  sources
  source locations
  source directories
  source files
  source facts
  observed file tags

Track identity
  tracks
  track metadata
  track attachments
  track attachment locations
  track identity candidates
  track identity resolution results
  track identity result options
  track identity decisions
  track identity suppressions

Preparation work
  work runs
  work items
  artifacts
  artifact supersessions
  artifact inline payloads
  artifact file store entries
  preparation facets (governed registry)
  preparation facet dependencies
  preparation facet states
  resolved preparation targets

Browser projections
  navigation rows
  track browser rows
```

---

## Out of Scope for v1

These will be modeled when their domains are active. Including them now would
require vocabulary that is not yet settled.

```text
Organization objects (crates, playlists, smart lists, folders, tags)
Sleeves
Compatibility catalogs and export snapshots
Requests and request queue
Devices and preflight records
History and RT Flight Deck runtime evidence
Advanced preparation dashboards
Full Prepared Room UI
```

The preparation facet set is defined in v1, but the facet editors (beatGrid,
cuePoints, phraseMarkers, notes) are out of scope. The facets exist as states
in the substrate. The editor surfaces come later.

---

## Schema Key Decisions

### KD-1: Files are not tracks

A `source_file` is an observed fact. A `track` is a collection identity
decision. They are connected through `track_attachments` and
`track_attachment_locations`, not through a foreign key from source_files to
tracks.

Consequence: a source file may be unattached (discovered but not yet claimed
by any track). A track may have multiple attachments. An attachment may have
multiple locations if the same media appears in multiple sources.

```sql
-- A file is observed independently of whether it becomes an attachment
source_files(id, source_id, path, size, mtime, content_hash, …)

-- An attachment is a track-owned media payload
track_attachments(id, track_id, kind, …)

-- A location connects an attachment to a specific source file
track_attachment_locations(id, attachment_id, source_file_id, …)
```

### KD-2: Readiness is not stored as durable truth

Readiness is not stored as a canonical record. It is projected from
`preparation_facet_states` and the evidence those states reference.

The `tracks` table has no `readiness` column.

`track_browser_rows` may store a derived `preparation_readiness_summary` for
renderer performance. That value is disposable projection state — not durable
truth. It must be invalidated when any contributing facet state, artifact
state, attachment state, or source availability changes.

### KD-3: Preparation facets are a governed registry

`preparation_facets` is a governed registry table, not a tag system and not a
SQL enum. Adding a new facet kind requires a registry migration and a versioned
facet definition. It does not require modifying every table that references
`facet_key`. All dependent tables use:

```sql
facet_key TEXT NOT NULL REFERENCES preparation_facets(facet_key)
```

Not repeated `CHECK` lists.

Each facet definition carries:

```text
facet_key              — unique stable identifier
                         (e.g. beat_grid, cue_points, waveform_rms,
                          loudness_lufs, musical_key, track_energy,
                          phrase_markers, stem_separation, notes, transition_ideas)
subject_kind           — which subject kind this facet applies to
                         (track, track_attachment, source_file, source_segment)
display_name           — human-readable label
evidence_schema_version — version of the artifact schema this facet produces
definition_version     — version of the facet definition itself
```

Algorithm and producer versions belong on `work_runs`, `work_items`, and
`artifacts` — not on the facet definition. Different artifacts for the same
facet can be produced by different algorithm versions.

Dependency declarations live in `preparation_facet_dependencies`, not as
inline foreign keys on the facet row.

A subject's facet state (in `preparation_facet_states`) is one of:

```text
unknown         — no evidence yet; no work scheduled
pending         — work is needed or queued; not authoritative job state
ready           — evidence satisfies the facet
degraded        — evidence is usable but confidence is reduced
blocked         — a hard upstream dependency is unavailable
failed          — work was attempted and failed
stale           — evidence exists but no longer matches the current
                  dependency state or algorithm version
suppressed      — intentionally excluded from preparation
not_applicable  — this facet does not apply to this subject
```

`scheduled` and `running` are job states. They belong to `work_items`, not
to facet state. A facet in `pending` state means work is indicated; the
work item itself records whether that work is queued or running.

### KD-4: Work runs own artifacts; preparation has a subject model

Work items are owned by work runs. Artifacts are owned by work items.
Artifacts are immutable after creation — payload and producer identity do not
change. Supersession is recorded in a separate edge table, not as a mutable
column on the artifact row.

Preparation work has a subject model because not all preparation is
track-scoped. Allowed v1 subject kinds:

```text
source_file        — container readability, tag parse, format detection
source_segment     — (reserved; disc image chapter-level analysis)
track              — cue points, notes, transition ideas
track_attachment   — waveform, duration, loudness, audio fingerprint, beat grid
```

```text
work_runs(
  id, kind, started_at, completed_at, agent_version, …
)

work_items(
  id, run_id, subject_kind, subject_id, facet_key,
  started_at, completed_at, outcome, …
)

artifacts(
  id, work_item_id, subject_kind, subject_id, facet_key,
  content_type, created_at, …
)

artifact_supersessions(
  superseded_artifact_id,
  superseding_artifact_id,
  reason,
  created_at
)

artifact_inline_payloads(id, artifact_id, payload_bytes)

artifact_file_store_entries(id, artifact_id, store_path, content_type, byte_size)
```

`preparation_facet_states` carries a `current_artifact_id` pointer to the
active artifact for that subject/facet pair:

```text
preparation_facet_states(
  subject_kind, subject_id, facet_key,
  state, current_artifact_id, updated_at
)
```

This makes the uniqueness invariant enforceable: at most one
`preparation_facet_states` row per `(subject_kind, subject_id, facet_key)`,
enforced by a unique index. If `current_artifact_id` is present, it must
reference a valid artifact whose subject and facet match the state row.

### KD-5: Identity resolution is a decision, not a merge

`track_identity_candidates` records candidate matches from identity resolution
heuristics (acoustic fingerprint, tag similarity, path pattern). A candidate
is not accepted identity authority — it is a proposal with evidence.

`track_identity_resolution_results` records the resolver's output for a given
run. `track_identity_result_options` records the individual options surfaced
when resolution is ambiguous, so ambiguity is data the system can inspect —
not a summary field.

`track_identity_decisions` records the accepted resolution: which candidates
were accepted, which were rejected, and by whom (system or user).

`track_identity_suppressions` records pairs that should never be proposed as
candidates again, regardless of similarity score.

This separation means identity resolution is auditable and reversible.

### KD-6: Browser rows are projections, not records

`navigation_rows` and `track_browser_rows` are derived tables. They exist
for query performance, not as a source of truth. Their content is rebuilt
from layers 1–3 on demand. They must be invalidatable by source ID, track ID,
facet kind, or work run ID.

The exact invalidation strategy (row-level, table-level, or a materialized
view approach) is implementation-level, not a schema invariant.

---

## Invariants

These are correctness constraints that must hold at all times.

```text
INV-01  Every track_attachment has exactly one track_id that refers to a valid track.

INV-02  Every track_attachment_location refers to a valid attachment.
        source_file_id is nullable (ON DELETE SET NULL).
        If source_file_id is present, it must refer to a valid source_file.
        If source_file_id is absent, the row must retain source identity and
        path snapshots, and availability_state must not be 'available'.
        A location row must survive source row removal — it does not cascade delete.

INV-03  Every preparation_facet_states row refers to a valid (subject_kind, subject_id)
        pair and a valid facet_key in preparation_facets.
        At most one row exists per (subject_kind, subject_id, facet_key).
        If current_artifact_id is present, it must reference an artifact whose
        subject_kind, subject_id, and facet_key match the state row.

INV-04  Artifact payload and producer identity are immutable after creation.
        Supersession is recorded only in artifact_supersessions.
        An artifact row has no superseded_by column.

INV-05  A work_item may not have outcome = 'complete' unless at least one artifact
        exists with work_item_id referencing it.

INV-06  preparation_facet_dependencies must not form a cycle.

INV-07  track_identity_suppressions are symmetric: if (a, b) is suppressed,
        (b, a) is also suppressed or the inverse suppression is implied by the
        direction field.

INV-08  navigation_rows are stale-safe: a missing navigation_row is not an error.
        A stale navigation_row is not exposed to the renderer — it triggers a
        rebuild before the next read.
```

---

## Baseline Replacement Strategy

Dekzer is pre-product. There is no live data to preserve. This is a baseline
replacement, not a production migration.

### Phase A: Replace baseline schema

Create the v1 tables as the new baseline. Remove `LibraryAssets` and all
`library_asset_id` references. Do not add compatibility views or aliases.
Do not add dual-write paths. The legacy model is gone.

### Phase B: Port code references

Update all Rust and TypeScript read/write paths to the v1 vocabulary.
`source_file`, `track`, `track_attachment`, `preparation_facet_states`,
`artifact`, `artifact_supersessions`. No legacy shims.

### Phase C: Seed and fixture conversion

Convert test fixtures and any dev-only seed data to the v1 schema.
Any legacy conversion helper is temporary tooling only — not substrate.
Delete it after use.

### Phase D: Validation

Run schema parse, FK integrity checks, invariant tests (INV-01 through
INV-08), and read-path tests against realistic fixture data. Do not proceed
to Phase E until all checks pass.

### Phase E: Delete legacy vocabulary

Remove all `LibraryAssets` naming from code, docs, tests, and fixtures.
Remove any temporary conversion helpers written in Phase C. The baseline
is clean when no file in the repository references the legacy epoch.

---

## Consequences

**Positive:**

- Source observation and track identity are cleanly separated. Scanning does
  not need to understand collection semantics.
- Preparation is a plane, not a single status flag. Each dimension is
  independently tracked and auditable.
- Readiness has a defined authority path because it is projected from facet state and evidence, not stored as durable record truth.
- Identity resolution is auditable, reversible, and suppressible.
- The browser row projection can be rebuilt from first principles at any time.

**Accepted costs:**

- Track browser rows require a materialization step. Cold starts or large
  invalidations need a rebuild pass before the renderer can show results.
- Identity resolution requires a separate decision record even for obvious
  single-match cases. This is intentional: every claim is auditable.
- Preparation facets are a governed registry, not a free-form tag system.
  Adding a new facet kind requires a registry migration and a definition version
  bump. Facet keys are rows in a reference table, not hardcoded SQL enum values.

**Excluded by design:**

- No `readiness` column on tracks. If a caller needs readiness, they query the
  preparation substrate and project it. Shortcutting this produces stale state.
- No generic `properties` blob on tracks or attachments. Every durable field
  is a named column with a defined type. If a field is worth storing, it is
  worth naming.
- No generic soft-delete flag pattern. Use domain-specific lifecycle,
  availability, supersession, or stale-state records where history matters.
  Physical deletion means the substrate intentionally forgets the record —
  not that the observed thing is currently absent.
