---
status: superseded
superseded-by: docs/decisions/library-preparation-substrate-v1.md
purpose: historical source material only
authority: not active architecture authority
---

# Schema Rewrite: V1 Library + Preparation Substrate

**Decision authority:** `docs/decisions/library-preparation-substrate-v1.md`
**Input:** `20260502000000_substrate_baseline.sql`
**Output:** `20260527000000_substrate_v1.sql`

Read the decision doc before reading this prompt. All architectural questions
are settled there. This prompt is an implementation brief, not a design session.

---

## Mission

Replace the `LibraryAssets`-based schema with the v1 substrate. This is a
baseline replacement on a pre-product system. No live data. No dual-write.
No compatibility aliases. The legacy epoch is gone.

---

## Style Rules

```text
All table names:      snake_case
All column names:     snake_case
All constraints:      STRICT on every table
Timestamps:           INTEGER (Unix ms, named *_at)
Booleans:             INTEGER 0/1 with CHECK (col IN (0, 1))
Epoch:                20260527000000
Schema generation:    20260527000000_substrate_v1
```

Carry over all existing CHECK constraints, partial indexes, and ordering
indexes from the input unless the column they reference is being removed.

---

## Tables to Remove

Remove these tables and all indexes and constraints that reference them.
They must not appear in the output schema.

```text
LibraryAssets
LibraryAssetAttachments
LibraryAssetMetadataCorrections
LibraryAssetCapabilities
LibraryBrowserRows
Playlists
PlaylistEntries
CapabilitySpecs
CapabilityDependencies
PrepPolicies
PrepPolicyTargets
PrepAssignments
ResolvedLibraryAssetPrepTargets
```

---

## Tables to Rename (snake_case only, no structural change)

```text
LibraryMetadata                   → library_metadata
SourceFacts                       → source_facts
SourceSegmentSets                 → source_segment_sets
SourceSegments                    → source_segments
ArtifactClaims                    → artifact_claims
ProjectionChangeLog               → projection_change_log
ProjectionSubscribers             → projection_subscribers
ProjectionCursors                 → projection_cursors
ProjectionRetentionWatermarks     → projection_retention_watermarks
```

Update all FK references and index names to match the new names. No
column changes. No constraint changes.

---

## Tables to Restructure

### `WorkRuns` → `work_runs`

Restructure as the parent of `work_items`. This reverses the old direction:
`work_runs` is now the execution context that owns many `work_items`.

New shape:

```sql
CREATE TABLE work_runs (
    work_run_id     INTEGER PRIMARY KEY,
    run_kind        TEXT    NOT NULL CHECK (length(trim(run_kind)) > 0),
    agent_key       TEXT    NOT NULL CHECK (length(trim(agent_key)) > 0),
    agent_version   TEXT    NOT NULL CHECK (length(trim(agent_version)) > 0),
    started_at      INTEGER NOT NULL,
    completed_at    INTEGER,
    outcome         TEXT    NOT NULL
        CHECK (outcome IN ('running', 'completed', 'failed', 'canceled')),
    failure_kind    TEXT,
    error_detail    TEXT,
    CHECK (completed_at IS NULL OR completed_at >= started_at)
) STRICT;
```

### `WorkItems` → `work_items`

Restructure as a child of `work_runs`. The FK direction flips: a work item
belongs to one work run. Remove `subject_kind = 'library_asset'`,
`subject_kind = 'projection_domain'`, and the `rebuild_projection` work_kind.
Remove `capability_kind`, `target_profile_key`, and `target_quality` columns.

New shape:

```sql
CREATE TABLE work_items (
    work_item_id      INTEGER PRIMARY KEY,
    work_run_id       INTEGER NOT NULL REFERENCES work_runs (work_run_id) ON DELETE CASCADE,
    subject_kind      TEXT    NOT NULL
        CHECK (subject_kind IN ('source_file', 'source_segment', 'track', 'track_attachment')),
    subject_id        INTEGER NOT NULL CHECK (subject_id > 0),
    work_kind         TEXT    NOT NULL
        CHECK (work_kind IN (
            'inspect_source',
            'accept_segmentation',
            'resolve_identity',
            'compute_facet',
            'rebind_location'
        )),
    facet_key         TEXT REFERENCES preparation_facets (facet_key),
    priority_class    TEXT    NOT NULL
        CHECK (priority_class IN ('urgent', 'interactive', 'background')),
    basis_fingerprint TEXT    NOT NULL CHECK (length(trim(basis_fingerprint)) > 0),
    state             TEXT    NOT NULL
        CHECK (state IN ('queued', 'leased', 'completed', 'blocked', 'failed', 'canceled')),
    leased_until      INTEGER,
    attempt_count     INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    blocked_reason    TEXT,
    failure_kind      TEXT,
    error_detail      TEXT,
    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    CHECK ((state = 'leased' AND leased_until IS NOT NULL) OR state <> 'leased'),
    CHECK (
        (work_kind = 'compute_facet' AND facet_key IS NOT NULL)
        OR (work_kind <> 'compute_facet' AND facet_key IS NULL)
    )
) STRICT;
```

Carry over all deduplication indexes translated to the new column set.

### `Artifacts` → `artifacts`

Replace entirely with explicit DDL. Do not derive from the old table
shape — too many columns are removed or renamed.

```sql
CREATE TABLE artifacts (
    artifact_id       INTEGER PRIMARY KEY,
    work_item_id      INTEGER NOT NULL REFERENCES work_items (work_item_id) ON DELETE CASCADE,
    subject_kind      TEXT    NOT NULL
        CHECK (subject_kind IN ('source_file', 'source_segment', 'track', 'track_attachment')),
    subject_id        INTEGER NOT NULL CHECK (subject_id > 0),
    facet_key         TEXT REFERENCES preparation_facets (facet_key),
    adapter_key       TEXT    NOT NULL CHECK (length(trim(adapter_key)) > 0),
    adapter_version   TEXT    NOT NULL CHECK (length(trim(adapter_version)) > 0),
    basis_fingerprint TEXT    NOT NULL CHECK (length(trim(basis_fingerprint)) > 0),
    media_type        TEXT    NOT NULL CHECK (length(trim(media_type)) > 0),
    storage_kind      TEXT    NOT NULL CHECK (storage_kind IN ('inline', 'file')),
    payload_hash      TEXT    NOT NULL CHECK (length(trim(payload_hash)) > 0),
    created_at        INTEGER NOT NULL
) STRICT;

CREATE INDEX artifacts_work_item
    ON artifacts (work_item_id);

CREATE INDEX artifacts_subject_facet
    ON artifacts (subject_kind, subject_id, facet_key, created_at DESC);
```

Validation note: if `facet_key IS NOT NULL`, it must reference a facet whose
`subject_kind` matches the artifact's `subject_kind`. SQLite cannot express
the cross-table subject_kind match in a CHECK; enforce this in a post-load
validation query.

Do not add a `superseded_by` column. Supersession is recorded in
`artifact_supersessions`.

### `projection_change_log`

Update `projection_domain` CHECK if it previously referenced `library_asset`
or capability domains. Keep `library_browser` and `navigation` as the only
valid projection domains.

---

## Tables to Add

### Layer 1 addition: `observed_file_tags`

Raw tag fields observed from audio/video containers during source scan.
One row per (source_file, tag_namespace, field_key, tag_index).
This is a ledger of observations, not curated metadata.

```sql
CREATE TABLE observed_file_tags (
    source_file_id  INTEGER NOT NULL
        REFERENCES source_files (source_file_id) ON DELETE CASCADE,
    tag_namespace   TEXT    NOT NULL CHECK (length(trim(tag_namespace)) > 0),
    field_key       TEXT    NOT NULL CHECK (length(trim(field_key)) > 0),
    tag_index       INTEGER NOT NULL DEFAULT 0 CHECK (tag_index >= 0),
    value_text      TEXT,
    value_int       INTEGER,
    value_real      REAL,
    observed_at     INTEGER NOT NULL,
    PRIMARY KEY (source_file_id, tag_namespace, field_key, tag_index),
    CHECK (
        (value_text IS NOT NULL AND value_int IS NULL AND value_real IS NULL)
        OR (value_text IS NULL AND value_int IS NOT NULL AND value_real IS NULL)
        OR (value_text IS NULL AND value_int IS NULL AND value_real IS NOT NULL)
        OR (value_text IS NULL AND value_int IS NULL AND value_real IS NULL)
    )
) STRICT;

CREATE INDEX observed_file_tags_source
    ON observed_file_tags (source_file_id);
```

### Layer 2: `tracks`

```sql
CREATE TABLE tracks (
    track_id        INTEGER PRIMARY KEY,
    identity_key    TEXT    NOT NULL UNIQUE CHECK (length(trim(identity_key)) > 0),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;
```

### Layer 2: `track_metadata`

Curated metadata for a track. One row per track. Fields are nullable when
not yet established.

```sql
CREATE TABLE track_metadata (
    track_id     INTEGER PRIMARY KEY REFERENCES tracks (track_id) ON DELETE CASCADE,
    title        TEXT,
    artist       TEXT,
    album        TEXT,
    album_artist TEXT,
    track_number INTEGER CHECK (track_number IS NULL OR track_number >= 0),
    disc_number  INTEGER CHECK (disc_number IS NULL OR disc_number >= 0),
    year         INTEGER CHECK (year IS NULL OR year >= 0),
    genre        TEXT,
    comment      TEXT,
    isrc         TEXT,
    updated_at   INTEGER NOT NULL
) STRICT;
```

### Layer 2: `track_attachments`

A media payload owned by a track. `attachment_kind` distinguishes primary
media from alternates.

```sql
CREATE TABLE track_attachments (
    track_attachment_id  INTEGER PRIMARY KEY,
    track_id             INTEGER NOT NULL REFERENCES tracks (track_id) ON DELETE CASCADE,
    attachment_kind      TEXT    NOT NULL
        CHECK (attachment_kind IN (
            'primary_media',
            'alternate_encoding',
            'stem_source',
            'archive_source'
        )),
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;

CREATE INDEX track_attachments_track
    ON track_attachments (track_id);
```

### Layer 2: `track_attachment_locations`

When `source_file_id` is NULL, `source_path_snapshot` and
`availability_state != 'available'` are required. A segment location is
always anchored through its source file: if `source_segment_id` is present,
`source_file_id` must also be present.

```sql
CREATE TABLE track_attachment_locations (
    location_id           INTEGER PRIMARY KEY,
    track_attachment_id   INTEGER NOT NULL
        REFERENCES track_attachments (track_attachment_id) ON DELETE CASCADE,
    source_file_id        INTEGER
        REFERENCES source_files (source_file_id) ON DELETE SET NULL,
    source_segment_id     INTEGER
        REFERENCES source_segments (source_segment_id) ON DELETE SET NULL,
    source_id_snapshot    INTEGER NOT NULL,
    source_path_snapshot  TEXT    NOT NULL CHECK (length(trim(source_path_snapshot)) > 0),
    availability_state    TEXT    NOT NULL
        CHECK (availability_state IN ('available', 'unavailable', 'unknown')),
    created_at            INTEGER NOT NULL,
    updated_at            INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    CHECK (
        source_file_id IS NOT NULL
        OR availability_state != 'available'
    ),
    CHECK (
        source_segment_id IS NULL
        OR source_file_id IS NOT NULL
    )
) STRICT;

CREATE INDEX track_attachment_locations_attachment
    ON track_attachment_locations (track_attachment_id);

CREATE INDEX track_attachment_locations_source_file
    ON track_attachment_locations (source_file_id)
    WHERE source_file_id IS NOT NULL;

-- One active attachment per whole source file (no segment):
CREATE UNIQUE INDEX track_attachment_locations_source_file_unique
    ON track_attachment_locations (source_file_id)
    WHERE source_file_id IS NOT NULL
      AND source_segment_id IS NULL
      AND availability_state = 'available';

-- One active attachment per source segment:
CREATE UNIQUE INDEX track_attachment_locations_source_segment_unique
    ON track_attachment_locations (source_segment_id)
    WHERE source_segment_id IS NOT NULL
      AND availability_state = 'available';
```

> **Store-logic note:** `source_file_id` uses `ON DELETE SET NULL`, but the
> uniqueness indexes and the `available` availability check mean that deleting
> a `source_file` row while a location referencing it is still `available`
> will leave a stale `available` row after the NULL is set — or may conflict
> if another location for the same file exists. Physical deletion of
> `source_file` rows that are referenced by `available` attachment locations
> must be performed through store logic that first sets the location's
> `availability_state` to `'unavailable'`. Do not rely on `ON DELETE SET NULL`
> to handle lifecycle state transitions.

### Layer 2: `track_identity_candidates`

Candidate matches produced by identity resolution heuristics. A candidate
is a proposal with evidence — not accepted identity authority. Candidates
exist before identity is accepted, so `track_id` must be optional. The
primary key is the observed source subject; `candidate_track_id` is the
proposed track to link or create.

```sql
CREATE TABLE track_identity_candidates (
    candidate_id        INTEGER PRIMARY KEY,
    subject_kind        TEXT    NOT NULL
        CHECK (subject_kind IN ('source_file', 'source_segment')),
    subject_id          INTEGER NOT NULL CHECK (subject_id > 0),
    candidate_track_id  INTEGER REFERENCES tracks (track_id) ON DELETE SET NULL,
    candidate_kind      TEXT    NOT NULL CHECK (length(trim(candidate_kind)) > 0),
    evidence_kind       TEXT    NOT NULL CHECK (length(trim(evidence_kind)) > 0),
    confidence          REAL    NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    payload_json        TEXT    NOT NULL,
    created_at          INTEGER NOT NULL
) STRICT;

CREATE INDEX track_identity_candidates_subject
    ON track_identity_candidates (subject_kind, subject_id, candidate_kind);

CREATE INDEX track_identity_candidates_track
    ON track_identity_candidates (candidate_track_id)
    WHERE candidate_track_id IS NOT NULL;
```

### Layer 2: `track_identity_resolution_results`

Output of a resolver run for a given work item. `track_id` is nullable —
resolution may conclude that no existing track applies (e.g. create-new
outcome). `work_item_id` is the authoritative link back to the job that
produced this result.

```sql
CREATE TABLE track_identity_resolution_results (
    resolution_result_id  INTEGER PRIMARY KEY,
    work_item_id          INTEGER NOT NULL REFERENCES work_items (work_item_id) ON DELETE CASCADE,
    track_id              INTEGER REFERENCES tracks (track_id) ON DELETE SET NULL,
    resolver_key          TEXT    NOT NULL CHECK (length(trim(resolver_key)) > 0),
    resolver_version      TEXT    NOT NULL CHECK (length(trim(resolver_version)) > 0),
    outcome               TEXT    NOT NULL
        CHECK (outcome IN ('resolved', 'ambiguous', 'no_match', 'suppressed', 'failed')),
    created_at            INTEGER NOT NULL
) STRICT;

CREATE INDEX track_identity_resolution_results_work_item
    ON track_identity_resolution_results (work_item_id);

CREATE INDEX track_identity_resolution_results_track
    ON track_identity_resolution_results (track_id, created_at DESC)
    WHERE track_id IS NOT NULL;
```

### Layer 2: `track_identity_result_options`

Individual options surfaced when resolution is ambiguous. Ambiguity is
structured data, not a summary field.

```sql
CREATE TABLE track_identity_result_options (
    option_id             INTEGER PRIMARY KEY,
    resolution_result_id  INTEGER NOT NULL
        REFERENCES track_identity_resolution_results (resolution_result_id) ON DELETE CASCADE,
    ordinal               INTEGER NOT NULL CHECK (ordinal >= 0),
    candidate_id          INTEGER REFERENCES track_identity_candidates (candidate_id),
    confidence            REAL    NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    payload_json          TEXT    NOT NULL,
    UNIQUE (resolution_result_id, ordinal)
) STRICT;
```

### Layer 2: `track_identity_decisions`

Accepted resolution: which option was chosen, by whom, and when.

```sql
CREATE TABLE track_identity_decisions (
    decision_id           INTEGER PRIMARY KEY,
    track_id              INTEGER REFERENCES tracks (track_id) ON DELETE SET NULL,
    resolution_result_id  INTEGER
        REFERENCES track_identity_resolution_results (resolution_result_id),
    decided_option_id     INTEGER
        REFERENCES track_identity_result_options (option_id),
    decision_kind         TEXT    NOT NULL
        CHECK (decision_kind IN ('accepted', 'rejected', 'deferred')),
    decided_by            TEXT    NOT NULL
        CHECK (decided_by IN ('system', 'user')),
    decided_at            INTEGER NOT NULL,
    CHECK (
        decision_kind <> 'accepted'
        OR track_id IS NOT NULL
    )
) STRICT;

CREATE INDEX track_identity_decisions_track
    ON track_identity_decisions (track_id, decided_at DESC)
    WHERE track_id IS NOT NULL;
```

### Layer 2: `track_identity_suppressions`

Pairs that must never be proposed as candidates again. Symmetric by law.

```sql
CREATE TABLE track_identity_suppressions (
    suppression_id  INTEGER PRIMARY KEY,
    track_id_a      INTEGER NOT NULL REFERENCES tracks (track_id) ON DELETE CASCADE,
    track_id_b      INTEGER NOT NULL REFERENCES tracks (track_id) ON DELETE CASCADE,
    suppressed_by   TEXT    NOT NULL CHECK (suppressed_by IN ('system', 'user')),
    suppressed_at   INTEGER NOT NULL,
    CHECK (track_id_a < track_id_b)
) STRICT;

CREATE UNIQUE INDEX track_identity_suppressions_pair
    ON track_identity_suppressions (track_id_a, track_id_b);
```

### Layer 3: `preparation_facets`

Governed registry. Facet kinds are rows, not SQL enums or CHECK lists.
All tables that reference a facet use:
`facet_key TEXT NOT NULL REFERENCES preparation_facets (facet_key)`

```sql
CREATE TABLE preparation_facets (
    facet_key               TEXT PRIMARY KEY CHECK (length(trim(facet_key)) > 0),
    subject_kind            TEXT NOT NULL
        CHECK (subject_kind IN ('source_file', 'source_segment', 'track', 'track_attachment')),
    display_name            TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    evidence_schema_version TEXT NOT NULL CHECK (length(trim(evidence_schema_version)) > 0),
    definition_version      TEXT NOT NULL CHECK (length(trim(definition_version)) > 0),
    created_at              INTEGER NOT NULL,
    updated_at              INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;
```

### Layer 3: `preparation_facet_dependencies`

Replaces `CapabilityDependencies`. Must not form a cycle (INV-06).

```sql
CREATE TABLE preparation_facet_dependencies (
    upstream_facet_key    TEXT NOT NULL REFERENCES preparation_facets (facet_key) ON DELETE CASCADE,
    downstream_facet_key  TEXT NOT NULL REFERENCES preparation_facets (facet_key) ON DELETE CASCADE,
    invalidation_mode     TEXT NOT NULL CHECK (length(trim(invalidation_mode)) > 0),
    created_at            INTEGER NOT NULL,
    PRIMARY KEY (upstream_facet_key, downstream_facet_key),
    CHECK (upstream_facet_key <> downstream_facet_key)
) STRICT;
```

### Layer 3: `preparation_facet_states`

Generic subject model. One row per (subject_kind, subject_id, facet_key).
Unique index enforces INV-03. `current_artifact_id` points to the active
artifact for this subject/facet pair.

Facet states describe readiness/evidence validity — not job execution state.
Job execution state (queued/leased) belongs to `work_items`.

```sql
CREATE TABLE preparation_facet_states (
    subject_kind         TEXT    NOT NULL
        CHECK (subject_kind IN ('source_file', 'source_segment', 'track', 'track_attachment')),
    subject_id           INTEGER NOT NULL CHECK (subject_id > 0),
    facet_key            TEXT    NOT NULL REFERENCES preparation_facets (facet_key),
    state                TEXT    NOT NULL
        CHECK (state IN (
            'unknown',
            'pending',
            'ready',
            'degraded',
            'blocked',
            'failed',
            'stale',
            'suppressed',
            'not_applicable'
        )),
    current_artifact_id  INTEGER REFERENCES artifacts (artifact_id),
    updated_at           INTEGER NOT NULL,
    PRIMARY KEY (subject_kind, subject_id, facet_key)
) STRICT;

CREATE INDEX preparation_facet_states_subject
    ON preparation_facet_states (subject_kind, subject_id);

CREATE INDEX preparation_facet_states_facet_state
    ON preparation_facet_states (facet_key, state);
```

### Layer 3: `artifact_inline_payloads`

Explicit DDL (not rename-only — old column names differ). `payload` carries
the raw bytes for inline-stored artifacts.

```sql
CREATE TABLE artifact_inline_payloads (
    artifact_id  INTEGER PRIMARY KEY REFERENCES artifacts (artifact_id) ON DELETE CASCADE,
    payload      BLOB    NOT NULL,
    created_at   INTEGER NOT NULL
) STRICT;
```

### Layer 3: `artifact_file_store_entries`

Explicit DDL (not rename-only — old schema used `payload_bytes`; this uses
`byte_size` to match the decision doc). `media_type` is nullable for entries
where content type is inherited from the parent artifact row.

```sql
CREATE TABLE artifact_file_store_entries (
    artifact_id    INTEGER PRIMARY KEY REFERENCES artifacts (artifact_id) ON DELETE CASCADE,
    root_kind      TEXT    NOT NULL CHECK (length(trim(root_kind)) > 0),
    relative_path  TEXT    NOT NULL CHECK (length(trim(relative_path)) > 0),
    byte_size      INTEGER NOT NULL CHECK (byte_size >= 0),
    media_type     TEXT,
    created_at     INTEGER NOT NULL
) STRICT;
```

### Layer 3: `artifact_supersessions`

Records when one artifact supersedes another. Artifacts are immutable after
creation; this edge table is the only supersession record (INV-04).

```sql
CREATE TABLE artifact_supersessions (
    superseded_artifact_id   INTEGER NOT NULL
        REFERENCES artifacts (artifact_id) ON DELETE CASCADE,
    superseding_artifact_id  INTEGER NOT NULL
        REFERENCES artifacts (artifact_id) ON DELETE CASCADE,
    reason                   TEXT NOT NULL CHECK (length(trim(reason)) > 0),
    created_at               INTEGER NOT NULL,
    PRIMARY KEY (superseded_artifact_id, superseding_artifact_id),
    CHECK (superseded_artifact_id <> superseding_artifact_id)
) STRICT;

CREATE INDEX artifact_supersessions_superseding
    ON artifact_supersessions (superseding_artifact_id);
```

### Layer 3: `resolved_preparation_targets`

Replaces `ResolvedLibraryAssetPrepTargets`. Uses the generic subject model.
No `library_asset_id`. No policy FK (prep policy machinery is out of v1
scope; this table holds the resolved result, not the policy chain).

```sql
CREATE TABLE resolved_preparation_targets (
    subject_kind   TEXT    NOT NULL
        CHECK (subject_kind IN ('source_file', 'source_segment', 'track', 'track_attachment')),
    subject_id     INTEGER NOT NULL CHECK (subject_id > 0),
    facet_key      TEXT    NOT NULL REFERENCES preparation_facets (facet_key),
    priority_class TEXT    NOT NULL
        CHECK (priority_class IN ('urgent', 'interactive', 'background')),
    updated_at     INTEGER NOT NULL,
    PRIMARY KEY (subject_kind, subject_id, facet_key)
) STRICT;
```

### Layer 4: `track_browser_rows`

Replaces `LibraryBrowserRows`. Anchored on `track_id`. `preparation_readiness_summary`
is disposable projection cache — not durable truth.

```sql
CREATE TABLE track_browser_rows (
    track_id                      INTEGER PRIMARY KEY
        REFERENCES tracks (track_id) ON DELETE CASCADE,
    row_version                   INTEGER NOT NULL CHECK (row_version >= 0),
    primary_source_file_id        INTEGER REFERENCES source_files (source_file_id),
    availability_state            TEXT    NOT NULL
        CHECK (availability_state IN ('available', 'unavailable', 'degraded')),
    title                         TEXT,
    artist                        TEXT,
    album                         TEXT,
    duration_ms                   INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    musical_key                   TEXT,
    tempo_bpm                     REAL    CHECK (tempo_bpm IS NULL OR tempo_bpm >= 0),
    waveform_quality_current      INTEGER CHECK (waveform_quality_current IS NULL OR waveform_quality_current >= 0),
    waveform_quality_target       INTEGER CHECK (waveform_quality_target IS NULL OR waveform_quality_target >= 0),
    stems_state_summary           TEXT,
    preparation_readiness_summary TEXT    NOT NULL
        CHECK (preparation_readiness_summary IN (
            'not_required', 'ready', 'preparing', 'underprepared', 'blocked', 'failed'
        )),
    updated_at                    INTEGER NOT NULL
) STRICT;

CREATE INDEX track_browser_rows_availability
    ON track_browser_rows (availability_state, updated_at DESC);

CREATE INDEX track_browser_rows_readiness
    ON track_browser_rows (preparation_readiness_summary, updated_at DESC);

CREATE VIRTUAL TABLE track_browser_rows_fts USING fts5(
    title,
    artist,
    album,
    tokenize = 'unicode61 remove_diacritics 1'
);
```

---

## Invariants to Validate

Write a validation query or comment block for each invariant after the
schema DDL. These are the acceptance bar.

```text
INV-01  Every track_attachment.track_id references a valid track.
        (Enforced by FK — verify no orphans after any fixture load.)

INV-02  Every track_attachment_location references a valid attachment.
        source_file_id is nullable (ON DELETE SET NULL).
        If source_file_id IS NULL, availability_state != 'available'.

INV-03  At most one preparation_facet_states row per
        (subject_kind, subject_id, facet_key).
        If current_artifact_id IS NOT NULL, the referenced artifact's
        subject_kind, subject_id, and facet_key must match the state row.

INV-04  No artifact row has a superseded_by column.
        Supersession is recorded only in artifact_supersessions.

INV-05  A work_item with state = 'completed' has at least one artifact
        with work_item_id referencing that work_item directly.

INV-06  preparation_facet_dependencies contains no cycles.
        (Assert with a recursive CTE depth-limit check.)

INV-07  track_identity_suppressions: CHECK (track_id_a < track_id_b)
        enforces canonical ordering. Both directions are implied.

INV-08  A missing navigation_row is not an error.
        navigation_rows are stale-safe by design.
```

---

## Do Not Add

```text
Organization tables (crates, playlists, smart_lists, folders, tags)
Sleeves
History tables
Requests tables
Devices tables
Imports/exports tables
Compatibility aliases for removed tables
Dual-write triggers
Any reference to library_asset_id
```

---

## Output

A single `.sql` file:

```text
20260527000000_substrate_v1.sql
```

Header comment block:

```sql
-- Canonical baseline schema.
-- Epoch: 20260527000000
-- Generation: 20260527000000_substrate_v1
-- Authority: docs/decisions/library-preparation-substrate-v1.md
```

The file must parse cleanly under SQLite 3.45+ with `PRAGMA strict = ON`.
All FK references must resolve within the file. No dangling references.
