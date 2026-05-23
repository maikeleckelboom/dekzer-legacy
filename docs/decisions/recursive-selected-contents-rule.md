# Recursive Selected Contents Rule

## Decision

When a source-rooted library tree row is selected, the contents table represents the media-relevant contents under that selected row recursively.

Tree expansion and contents selection are separate product surfaces:

- Tree expansion shows immediate hierarchy children for navigation.
- Tree selection shows recursive media-relevant descendant contents for work.

The contents table must not show only the immediate loaded tree children unless the selected row is known to have no relevant descendants beyond them.

This document is the canonical source for recursive selection, query execution, renderer contracts, result shapes, scan coverage, and index requirements.

For product semantics regarding source locations, including creation, lifecycle, display, proposal handling, and reconnect behavior, see `source-locations-lifecycle-contract.md`.

## Scope

This rule applies to source-rooted hierarchy selections:

- source
- source location
- directory

It does not define playlist, crate, smart-view, search-result, all-tracks, recently-added, or other collection-backed selection semantics. Those nodes use separate query contracts.

## Short canon

```text
Tree expansion navigates immediate hierarchy.
Tree selection projects recursive media contents.
Source, source_location, and directory selections all resolve to substrate targets.
The substrate owns descendant enumeration.
The renderer never crawls the tree to answer recursive contents.
Partial scans must be visible as partial.
Exceptional availability beats ordinary media identity.
Virtualization is supported by this rule, not made harder by it.
Collection-backed nodes use separate query contracts.
```

## Selector payload contract

Selectable navigation rows must carry a durable selector payload. The selector payload must be canonical JSON with no extra fields.

Canonical `selector_kind` values:

- `source`
- `source_location`
- `directory`

Canonical payloads:

```json
{ "kind": "source", "source_id": 3 }
```

```json
{ "kind": "source_location", "source_id": 3, "source_location_id": 7 }
```

```json
{ "kind": "directory", "source_id": 3, "source_directory_id": 42 }
```

If `selector_kind` and `payload.kind` disagree, the renderer must reject the row as invalid projection data.

The renderer may pass selector payloads through. It must not parse product identity from labels, visible tree shape, DOM structure, expanded state, or path text.

The substrate or boundary resolver owns payload validation and conversion into typed targets.

## Schema pre-requisites and enum contracts

To support virtual trees and recursive completeness guarantees without eager reads, the schema must include structural and coverage facts.

### Required columns

The baseline schema must include these facts directly in the baseline SQL. This project is greenfield for the current substrate baseline; do not add a patch migration for this pass unless explicitly instructed.

```sql
source_directories.has_child_directories INTEGER NOT NULL DEFAULT 0
CHECK (has_child_directories IN (0, 1));

source_directories.has_media_descendant INTEGER NOT NULL DEFAULT 0
CHECK (has_media_descendant IN (0, 1));

source_directories.mtime_ns INTEGER;
source_directories.scanned_at INTEGER;

source_directories.dir_scan_state TEXT NOT NULL DEFAULT 'pending'
CHECK (dir_scan_state IN ('pending', 'scanning', 'complete', 'failed', 'blocked'));

source_directories.dir_scan_error_kind TEXT;
source_directories.dir_scan_error_detail TEXT;

source_directories.dir_scan_updated_at INTEGER NOT NULL;

source_scan_state.scan_phase TEXT NOT NULL DEFAULT 'idle'
CHECK (scan_phase IN ('idle', 'scanning', 'blocked', 'failed', 'complete'));
```

`dir_scan_updated_at` is `NOT NULL` and required on insert in the greenfield baseline. `mtime_ns` and `scanned_at` are nullable because directories can be pending, blocked, missing, or unavailable before the authority has trustworthy filesystem metadata or a completed scan attempt.

The baseline schema must also make literal hierarchy ownership structurally safe:

```sql
source_directories.parent_source_directory_id
  REFERENCES source_directories(source_directory_id)
  ON DELETE CASCADE;

source_files.parent_source_directory_id
  REFERENCES source_directories(source_directory_id)
  ON DELETE CASCADE;
```

Do not keep `ON DELETE SET NULL` for ordinary parent-directory relationships in the greenfield baseline. Deleting a directory subtree must not produce orphaned hierarchy rows.

`has_child_directories` and `has_media_descendant` are distinct facts:

- `has_child_directories` answers whether immediate child directories exist, regardless of media relevance.
- `has_media_descendant` answers whether the substrate has positive evidence that this directory has media-relevant descendants.

`has_media_descendant = 0` does not mean the directory is proven empty unless the relevant directory coverage is complete. Use `dir_scan_state` to distinguish unknown, pending, scanning, blocked, failed, and complete coverage.

`has_media_descendant` replaces `media_browseability` in the greenfield baseline. Do not keep both facts.

### Enum behavior matrix

Every enum value must map to a documented query behavior and visible state.

#### `sources.source_class`

- `internal`: not mount-state gated. Still subject to storage, permission, database, and scan-state failures. It must not be treated as unreachable due to removable-device mount state.
- `external_mounted`: subject to mount state.
- `removable_mounted`: subject to mount state.

#### `source_state.mount_status`

- `mounted`: media may be queryable.
- `unknown`: source availability is unresolved. The renderer must show resolving/unknown state or stale known rows as unavailable/uncertain. It must not assume availability.
- `unmounted`: media is globally unavailable. The contents projection returns `source_unavailable`, not an empty table.
- `eject_requested`: media is globally unavailable or becoming unavailable. The contents projection returns `source_unavailable` or `stale_cursor`, not an empty table.
- `eject_pending`: media is globally unavailable or becoming unavailable. The contents projection returns `source_unavailable` or `stale_cursor`, not an empty table.

#### `source_scan_state.scan_phase` and `source_directories.dir_scan_state`

`source_scan_state.scan_phase` describes source-level scan lifecycle. `idle` means no active worker; it does not mean fully scanned.

Source-level `scan_phase` values:

- `idle`: no active source scan worker. The source may still have incomplete directory coverage.
- `scanning`: active source scan worker.
- `blocked`: source scan cannot proceed due to permission, policy, or source availability.
- `failed`: source scan aborted with an error.
- `complete`: all `source_directories` rows for the source have `dir_scan_state = 'complete'`, and the source scan is not blocked or failed.

The transition to source-level `complete` happens only during scan finalization, in the same write transaction that proves there are no remaining `pending`, `scanning`, `blocked`, or `failed` directory rows for the source. If a rescan, reconnect verification, directory insertion, or source-location repair reopens coverage for any directory, source-level `scan_phase` must leave `complete`.

`source_directories.dir_scan_state` describes subtree coverage. Recursive selected contents completeness is derived from directory scan states for the selected subtree, not from source-level `complete` alone.

Directory-level `dir_scan_state` values:

- `pending`: directory is known but not scanned. Recursive contents are partial.
- `scanning`: active enumeration. Recursive contents are partial.
- `blocked`: cannot scan due to permission or policy. Recursive contents are blocked.
- `failed`: scan aborted. Recursive contents are failed.
- `complete`: directory fully scanned. Recursive contents are complete.

#### `LibraryBrowserRows.availability_state`

- `available`: rendered normally and playable.
- `degraded`: included in the default view, rendered with warning, and playable.
- `unavailable`: excluded from the ordinary playable-media result when the selected scope is available; shown as unavailable when the selected source or scope itself is unavailable and the projection has known prior rows; shown in explicit issue/readiness views; never treated as empty.

#### `source_files.media_class`

- `audio`: included in primary recursive contents.
- `video`: included in primary recursive contents.
- `unsupported`: strictly excluded from default recursive contents.
- `none`: strictly excluded from default recursive contents.

## Query execution contract

The renderer passes a selected target to the substrate-owned read path. The substrate resolves the target, queries media, collapses duplicates, and returns browse projection rows.

Conceptually:

```text
selected tree row
  -> resolve selector/binding target
  -> derive source_id and optional relative path prefix/scope
  -> query recursive descendant media rows from substrate/index
  -> join through asset identity and browse projection rows
  -> return rows plus scan coverage metadata
  -> render contents table
```

### 1. Hot path: path-prefix range

The hot recursive contents path must use `source_id + normalized relative_path` prefix ranges.

Recursive CTEs are reserved for operations that need directory identity mapping, repair, validation, or migration.

### 2. Binary collation required

The `char(48)` range trick requires strict bytewise ordering. This must be baked into schema and query definitions.

Required indexes:

```sql
CREATE INDEX source_files_source_relative_path_binary
ON source_files (source_id, relative_path COLLATE BINARY);

CREATE INDEX source_directories_source_relative_path_binary
ON source_directories (source_id, relative_path COLLATE BINARY);
```

Queries must compare using the same collation:

```sql
sf.source_id = :source_id
AND sf.relative_path COLLATE BINARY >= :prefix || '/'
AND sf.relative_path COLLATE BINARY <  :prefix || char(48)
```

`char(48)` is `'0'`. This relies on the ASCII/UTF-8 ordering invariant that `/` is codepoint 47 and `0` is codepoint 48. They are adjacent, so no valid descendant path beginning with `prefix || '/'` can sort outside the range, and no sibling path can sort between `prefix || '/'` and `prefix || '0'`.

The upper-bound expression must be produced by a named helper or named query builder, not copied inline as an unexplained magic literal.

### 3. Source-root prefix handling

A source root uses the source-level scope policy.

For whole-source scope, omit the prefix predicate or use a deliberate root-prefix helper that includes all source files for that source.

Do not accidentally require `relative_path >= '/'` for root selection, because stored relative paths are not absolute paths.

### 4. Source-location and directory prefix handling

A source location or directory uses its canonical relative path as `:prefix`.

The recursive query covers descendants whose paths begin with:

```text
:prefix || '/'
```

The selected directory's own files are included when their stored relative paths use `prefix/file.ext`.

### 5. Aggregate source-level scope

For source selection with accepted locations, recursive contents cover the union of accepted visible source-location subtrees.

Aggregate scope includes only source locations with:

```text
authority = user
location_kind = registered_subpath
is_user_visible = 1
```

Device-observed, suggested, hidden, rejected, dismissed, inferred, or unregistered locations are excluded.

Configured but unavailable source locations do not trigger whole-source fallback.

### 6. Aggregate scan completeness

For source selection with accepted locations, scan coverage must be computed over the union of accepted visible source-location subtrees.

If any included location subtree is `pending`, `scanning`, `failed`, or `blocked`, the source aggregate coverage is not complete.

Required conceptual query shape for aggregate source selection:

```sql
WITH accepted_locations(relative_path) AS (
  SELECT relative_path
  FROM source_locations
  WHERE source_id = :source_id
    AND authority = 'user'
    AND location_kind = 'registered_subpath'
    AND is_user_visible = 1
), covered_directories AS (
  SELECT sd.dir_scan_state
  FROM source_directories sd
  WHERE sd.source_id = :source_id
    AND EXISTS (
      SELECT 1
      FROM accepted_locations al
      WHERE sd.relative_path COLLATE BINARY = al.relative_path COLLATE BINARY
         OR (
           sd.relative_path COLLATE BINARY >= al.relative_path || '/'
           AND sd.relative_path COLLATE BINARY <  al.relative_path || char(48)
         )
    )
)
SELECT
  CASE
    WHEN SUM(CASE WHEN dir_scan_state = 'blocked' THEN 1 ELSE 0 END) > 0 THEN 'blocked'
    WHEN SUM(CASE WHEN dir_scan_state = 'failed' THEN 1 ELSE 0 END) > 0 THEN 'failed'
    WHEN SUM(CASE WHEN dir_scan_state IN ('pending', 'scanning') THEN 1 ELSE 0 END) > 0 THEN 'partial'
    ELSE 'complete'
  END AS aggregate_coverage_state
FROM covered_directories;
```

The production query may use a different equivalent shape, but it must cover every accepted visible source-location prefix in the same read transaction as the contents rows. It must not check only the first accepted location or only the source root.

### 7. Read transaction rule

Contents rows and scan coverage must be read from one consistent snapshot.

Do not run the rows query and coverage query across different implicit read states where scan progress can advance between them and produce inconsistent UI.

### 8. Strict ordering rule

Ordering is deterministic and projection-owned. Do not rely on internal SQLite rowid ordering, current renderer order, expanded tree order, or incidental query result order.

Default first-load ordering is `title ASC` after availability priority, with artist, album, path, and durable row identity as deterministic tie-breakers. This is the default order for the contents table unless the user explicitly chooses another sort.

```sql
ORDER BY
  CASE availability_state
    WHEN 'available' THEN 0
    WHEN 'degraded' THEN 1
    WHEN 'unavailable' THEN 2
    ELSE 3
  END ASC,
  lower(COALESCE(title, relative_path, '')) ASC,
  lower(COALESCE(artist, '')) ASC,
  lower(COALESCE(album, '')) ASC,
  lower(COALESCE(relative_path, '')) ASC,
  library_asset_id ASC
```

Every ordered query needs a stable tie-breaker. `library_asset_id` is the default tie-breaker for recursive contents rows.

### 9. Deduplication and asset collapse

If multiple scoped `source_files` map to the same `library_asset_id`, the result returns one row.

Deduplication rule:

1. Group by `library_asset_id`.
2. Prefer an attachment whose source file is present.
3. If `LibraryBrowserRows.primary_source_file_id` is within the selected scope and present, use it as the scoped attachment anchor.
4. Otherwise, use the present scoped source file with the lowest `source_file_id`.
5. If only degraded or unavailable attachments exist, choose the stable attachment that best represents the availability state and surface that state.

Do not leave duplicate collapse to the renderer.

### 10. Pagination cursor contract

Recursive contents pagination uses keyset cursors, not offset cursors. Offset pagination is not stable enough for a live-updating scan result set because inserts, removals, and duplicate-collapse changes can shift row positions between page reads.

The cursor stores the last emitted row's complete ordered key tuple for the active sort:

```ts
type SelectedContentsCursor = {
  version: 1
  scopeFingerprint: string
  order: 'availability_title_artist_album_path_asset_id'
  last: {
    availabilityRank: number
    titleKey: string
    artistKey: string
    albumKey: string
    relativePathKey: string
    libraryAssetId: string
  }
}
```

The serialized cursor may be an opaque string at the renderer boundary, but the substrate owns its decoded structure and validation. A cursor is valid only for the selected scope fingerprint and ordering that produced it.

If the selected source or scope becomes unavailable, unmounted, permission-denied, changes aggregate scope, changes ordering, or otherwise invalidates the cursor during pagination, the next page request must return `source_unavailable`, `location_missing`, `blocked`, or `stale_cursor`.

It must never return an empty page as if the scope simply ran out of media.

## Renderer boundary and result shape

The substrate returns a strictly typed result shape. The renderer consumes this projection and must never rebuild product display state from raw filesystem rows.

All durable SQLite row identifiers crossing the renderer boundary are encoded as strings unless the boundary contract explicitly proves they are safe JavaScript integers.

```ts
type SelectedContentsCursor = string

type SelectedContentsPage = {
  cursor?: SelectedContentsCursor
  limit: number
  totalKnownRows?: number
}

type SelectedContentsResult =
  | {
      state: 'ready'
      scope: SelectedContentsScope
      rows: readonly SelectedContentsRow[]
      page: SelectedContentsPage
      coverage: {
        state: 'complete'
      }
    }
  | {
      state: 'partial'
      scope: SelectedContentsScope
      rows: readonly SelectedContentsRow[]
      page: SelectedContentsPage
      coverage: {
        state: 'scanning' | 'pending'
        scannedRowCount?: number
        detail?: string
      }
    }
  | {
      state: 'empty'
      scope: SelectedContentsScope
      rows: []
      coverage: {
        state: 'complete'
      }
    }
  | {
      state: 'source_unavailable'
      scope: SelectedContentsScope
      reason: string
      knownRows?: readonly SelectedContentsRow[]
    }
  | {
      state: 'location_missing'
      scope: SelectedContentsScope
      reason: string
      knownRows?: readonly SelectedContentsRow[]
    }
  | {
      state: 'blocked'
      scope: SelectedContentsScope
      reason: string
      rows?: readonly SelectedContentsRow[]
    }
  | {
      state: 'failed'
      scope: SelectedContentsScope
      error: string
      rows?: readonly SelectedContentsRow[]
    }
  | {
      state: 'stale_cursor'
      scope: SelectedContentsScope
      reason: string
    }

type SelectedContentsRow = {
  libraryAssetId: string
  rowVersion: number
  title?: string
  artist?: string
  album?: string
  durationMs?: number
  availabilityState: 'available' | 'unavailable' | 'degraded'
  primarySourceFileId?: string
  scopedSourceFileId?: string
  relativePath?: string
  readiness?: SelectedContentsReadinessSummary
}

type SelectedContentsReadinessSummary = {
  summaryLabel?: string
  facets?: readonly SelectedContentsReadinessFacet[]
}

type SelectedContentsReadinessFacet = {
  kind: string
  state: 'ready' | 'needs_attention' | 'pending' | 'unavailable' | 'unknown'
  label?: string
}
```

`readiness` is optional because preparation is not one flat status. Readiness facets may later cover independent preparation domains such as analysis, cue readiness, beatgrid, stems, loudness, source/file readiness, notes, and transition planning.

When a selected target is permanently deleted, active subscribers receive a tombstone/invalidated-target result, such as `location_missing` or `source_unavailable`, not an empty result.

## Visible contents states matrix

The UI must reflect the exact `SelectedContentsResult` state truthfully.

- `loading`: show progress skeleton or loading row. Do not show stale empty copy.
- `partial`: show rows found so far plus `Still indexing. Results may be incomplete.`
- `empty`: show `No playable media found under this folder.` Only valid with complete coverage.
- `source_unavailable`: show known rows as unavailable if supplied, or show a global unavailable state. Never show empty.
- `location_missing`: show the configured library folder as missing and offer repair or relink actions.
- `blocked`: show explicit permission-needed or policy-blocked state.
- `failed`: show failed state with retry or rescan action.
- degraded rows: include by default, mark visually degraded, keep playable.
- unsupported or `none` media rows: strictly exclude from default recursive contents.

## Virtualization and tree affordance rule

The tree and contents table are independently virtualizable because they answer different questions:

```text
tree expansion -> immediate child rows for navigation
contents table -> recursive media rows for selected target
```

The renderer does not need to hold recursive descendant contents inside the tree. It holds only:

- expanded node set
- flat visible tree rows
- scroll position
- selected target

The contents table uses its own cursor, page size, order key, and virtual scroll state. Its pagination must not depend on tree scroll position or tree expansion state.

Virtual tree chevrons are driven by substrate affordance signals:

- `has_child_directories`
- `has_media_descendant`

The renderer must not fetch depth+1 children for every visible row just to decide whether to show a chevron.

## Forbidden anti-patterns

These are explicit code-review vetoes:

1. The renderer must never walk its own tree nodes to build the contents table.
2. The renderer must never treat expansion state as a filter on the contents result set.
3. The renderer must never issue raw `source_files` queries directly.
4. The renderer must never parse source, source location, directory, or file identity out of display labels or path text.
5. The renderer must never silently present partial recursive results as complete.
6. The renderer must never reconstruct product display fields from raw filesystem rows.
7. The renderer must never eagerly fetch depth+1 children for every visible tree row only to decide whether to show a chevron.
8. The renderer must never apply source-rooted path-prefix logic to playlist, crate, smart-view, or collection-backed nodes.

## Implementation readiness

Implementation is allowed only after:

- the greenfield baseline SQL is edited directly; do not create a patch migration for this pass
- `source_directories.has_child_directories` exists in the baseline schema
- `source_directories.has_media_descendant` exists in the baseline schema and replaces `media_browseability`
- `source_directories.dir_scan_state` exists in the baseline schema
- `source_directories.mtime_ns` exists in the baseline schema
- `source_directories.scanned_at` exists in the baseline schema
- `source_scan_state.scan_phase` includes `complete` and has the transition rule defined above
- parent-directory foreign keys for `source_directories` and `source_files` cascade instead of setting children to `NULL`
- hot-path prefix indexes exist with `BINARY` collation
- `SelectedContentsResult` is represented as a real typed boundary contract
- recursive contents pagination uses the keyset cursor contract defined above
- fixtures cover aggregate scope, missing locations, duplicate collapse, scan coverage, and prefix-boundary exclusion
- the hot-path contents query has an `EXPLAIN QUERY PLAN` test proving the planner uses the `source_id + relative_path` binary index for prefix scans

## Test and fixture contract

Implementation must include fixture-based query tests validating identical outcomes to SQLite execution.

At minimum, fixtures must test:

1. exact row identity and duplicate asset collapse
2. exact deterministic sort order
3. total row count and cursor pagination limits
4. scan completeness boundaries, including partial vs complete
5. prefix-boundary exclusions, such as `Music2` being excluded when querying `Music`
6. source-level aggregate includes accepted visible locations only
7. source-level aggregate excludes unaccepted, hidden, and unregistered locations
8. configured but missing locations do not trigger whole-source fallback
9. degraded rows are included and surfaced as degraded/playable
10. unsupported and `none` media files are excluded from default recursive contents
11. selector payloads resolve to typed targets

The test suite must run `EXPLAIN QUERY PLAN` against the hot-path contents query and assert the planner uses the `source_id + relative_path` binary index for prefix scans. Do not lock the entire SQLite plan string unless unavoidable.
