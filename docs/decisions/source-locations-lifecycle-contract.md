# Source Locations and Lifecycle Contract

## Decision

A source location is an intentional, selectable sub-root inside a source. It is an internal architecture concept. The
default user-facing concept is a named library folder under a source, such as `Music` or `DJ Pool`.

A source location is not a separate source, not a fake renderer folder, and not every discovered directory.

For the canonical rules regarding contents queries, tree selection, renderer boundaries, result shapes, and index
requirements, see `selected-contents-scope-depth-rule.md`.

## Source-location lifecycle contract

### 1. Effective root anchoring

`source_locations.relative_path` is relative to the effective source root, not necessarily the raw mount root.

```text
effective_source_root = resolved_mount_root + source_locators.relative_suffix
```

Without this rule, removable volume paths can be evaluated incorrectly when `source_locators.relative_suffix` is
non-empty.

### 2. Observed vs registered

`source_locations` manages two authority models:

- `authority = device`, `location_kind = observed_path`: scanner or importer-discovered evidence. It may be used to
  propose folders. It does not affect source-level aggregate scope.
- `authority = user`, `location_kind = registered_subpath`: an accepted library folder. It contributes to aggregate
  scope when `is_user_visible = 1`.

Do not treat observed paths and registered subpaths as the same product state.

### 3. Accepting a proposal

When the user accepts a proposed source location, the durable result must be a user-registered source location.

- If a matching `device/observed_path` row exists for the same `source_id + relative_path`, acceptance updates or
  upserts that row into `user/registered_subpath` and sets `is_user_visible = 1`.
- If no matching row exists, acceptance inserts a new `user/registered_subpath` row.

The accepted state is durable configuration. It must not be inferred from whether the location is currently reachable.

### 4. Nested location rejection

Nested source locations under the same source are strictly rejected at registration in MVP.

Before inserting or updating a `user/registered_subpath`, the Rust authority/service layer must query existing accepted
source locations for the same source.

If either path is a proper prefix of the other, such as `Music` and `Music/DJ Pool`, the write transaction is rejected
with `SourceLocationPathOverlap`.

Do not rely on SQLite constraints, renderer validation, or aggregate-query de-duplication to enforce prefix overlap.

### 5. Root unavailable fallback

A newly attached source with zero configured source locations degrades to whole-source browsing only when the effective
source root is readable.

- Zero configured locations plus readable source root: whole-source fallback.
- Zero configured locations plus blocked or unreadable source root: blocked or unavailable state.
- Permission-denied, blocked, unresolved, or unavailable roots must never be shown as empty.

### 6. Root source-location impossibility

The source root is represented by source selection. Do not create a `source_location` row for the source root in MVP.

`source_locations.relative_path` cannot be empty and cannot represent `.`. That is intentional, not accidental.

### 7. Proposal suppression

Declining a source-location proposal requires durable suppression so the user is not nagged on every scan.

Proposal suppression is not represented by `source_locations`.

Until a dedicated proposal/suppression table exists, such as `source_location_proposal_suppression`, decline and ignore
flows are incomplete product behavior. Do not overload hidden source locations to mean declined proposals.

Minimum greenfield table shape:

```sql
CREATE TABLE source_location_proposal_suppression (
  source_location_proposal_suppression_id INTEGER PRIMARY KEY,
  source_id INTEGER NOT NULL REFERENCES sources(source_id) ON DELETE CASCADE,
  relative_path TEXT NOT NULL,
  heuristic_key TEXT NOT NULL,
  heuristic_version INTEGER NOT NULL,
  suppressed_at INTEGER NOT NULL,
  UNIQUE (source_id, relative_path, heuristic_key, heuristic_version)
);
```

This table is a bounded future shape, not an instruction to add a patch migration. In the greenfield baseline, implement
it directly in the baseline SQL when decline/ignore UI becomes product-visible.

### 8. Reconnect and verification

When a known source reconnects, Dekzer verifies configured source locations against the effective source root.

If a previously accepted source location is missing, renamed, permission-denied, or otherwise unreachable:

- keep the source-location configuration intact in the database
- surface `location_missing` or `blocked` state in the renderer result
- offer repair or relink workflows
- do not silently remove the source location
- do not treat the source as having zero configured locations
- do not fall back to whole-source browsing

### 9. Deletion and cascade laws

Deleting or hiding a source location never deletes source files, directories, assets, attachments, or browser rows. It
only changes source-location configuration and source-level aggregate scope.

Source deletion cascades substrate rows by schema. Literal directory parent relationships must also cascade in the
greenfield baseline. `ON DELETE SET NULL` is rejected for ordinary hierarchy parent relationships because it can create
orphaned directory or file rows.

Deleting a parent directory remains a scan-finalization or repair operation, not ordinary UI maintenance. The source
authority must finalize the affected subtree in one transaction.

Finalizing an affected subtree means the authority performs all of the following as one atomic write:

- identifies the affected subtree by `source_id` and canonical directory identity or relative-path prefix
- marks missing directories and files as `presence_state = 'removed'` when the physical entries disappeared but the
  source remains known
- cascade-deletes directory/file rows only when intentionally destroying an authority-owned subtree, such as source
  deletion or a repair rebuild that recreates canonical rows
- updates `source_directories.dir_scan_state`, `dir_scan_updated_at`, `scanned_at`, and scan error fields for every
  affected directory
- updates `source_directories.has_child_directories`, `source_directories.has_primary_media_descendant`, and
  `source_directories.has_image_media_descendant` for the affected directory and all impacted ancestors
- updates source-level `source_scan_state.scan_phase` after directory-level coverage has been reconciled
- commits browser/content projection invalidation in the same transaction, so subscribers cannot observe half-finalized
  hierarchy state

If these operations cannot be completed together, the operation must fail or retry. It must not leave orphaned hierarchy
rows, stale affordance flags, or source-level scan state that disagrees with directory coverage.

### 10. Creation paths

Source locations may be created only through intentional product flows:

1. The user chooses a folder during source setup.
2. The user promotes a scanned directory from the tree.
3. The user drags a folder into the library/source area.
4. Dekzer proposes a likely folder after scan, and the user accepts it.
5. An explicit importer flow creates a source location for a known structure, such as a Rekordbox export.

Do not silently create user-visible source locations from weak heuristics.

## Sidebar display and grouping policy

Default sidebar rules for source-rooted hierarchy:

1. Default source expansion shows accepted user-visible library folders first, directly under the source.
2. No `Locations` grouping node is shown by default.
3. Unregistered physical hierarchy remains accessible through an explicit affordance, such as `Browse whole drive` or
   `Browse whole source`, below the library folders.
4. If no accepted user-visible source locations exist, source expansion shows raw immediate hierarchy.
5. `browser_user_order` owns source-location sidebar order. The renderer must not invent a second ordering mechanism.

Source locations must have a subtle visual distinction from ordinary directories, such as pinned-folder or
library-folder treatment. Do not use a text badge such as `Location`.

Badges remain reserved for exceptional or actionable state.

## Source-level aggregate scope

A source can conceptually behave as a library-root source or a container source. MVP derives this from accepted source
locations rather than storing a separate source-scope enum.

- Zero accepted user-visible source locations: source selection means whole-source recursive primary media, provided the
  source root is readable.
- One or more accepted user-visible source locations: source selection means the aggregate of those accepted source
  locations.

Device-observed, suggested, hidden, rejected, dismissed, inferred, or not-user-visible locations are excluded from
aggregate scope.

Configured but unavailable source locations still count as configured source locations. They must surface unavailable,
blocked, degraded, missing, or partial state. They must not trigger whole-source fallback.

### Visible scope indicator

When source selection uses accepted source locations instead of the whole physical source, the contents table must make
that scope visible immediately in the header.

Example copy:

```text
Showing 2 pinned folders · Browse whole drive
```

The escape affordance opens or switches to the raw whole-source scope without changing the accepted source-location
configuration.

The user must never see fewer source-level results after accepting a location without a visible, persistent explanation
of the active scope.

## Display name policy

Source locations have a durable relative path and an optional display name. Name source locations from, in priority
order:

1. user-provided `display_name`
2. importer-provided semantic `display_name`
3. last segment of `relative_path` when it is meaningful
4. disambiguated relative path when the last segment is ambiguous or collides under the same source, such as
   `Contents on Rekordbox Export`

Two sources may each have a `Music` location. That is valid.

Two source locations under the same source must not have indistinguishable display names. Disambiguate with path context
when needed.

## Implementation readiness

Implementation is allowed only after:

- the greenfield baseline SQL is edited directly; do not create a patch migration for this pass
- `source_directories.has_child_directories` exists in the baseline schema
- `source_directories.has_primary_media_descendant` exists in the baseline schema
- `source_directories.has_image_media_descendant` exists in the baseline schema
- `source_directories.dir_scan_state` exists in the baseline schema
- `source_directories.mtime_ns` exists in the baseline schema
- `source_directories.scanned_at` exists in the baseline schema
- `source_scan_state.scan_phase` includes `complete` and follows the transition rule in
  `selected-contents-scope-depth-rule.md`
- parent-directory foreign keys for `source_directories` and `source_files` cascade instead of setting children to
  `NULL`
- hot-path prefix indexes exist with `BINARY` collation
- `SelectedContentsResult` is represented as a real typed boundary contract
- fixtures cover aggregate scope, missing locations, duplicate collapse, scan coverage, and prefix-boundary exclusion
- proposal suppression is either implemented through the dedicated table shape above or explicitly deferred from
  product-visible decline/ignore flows

## Code-review laws

- Do not model source locations as separate sources.
- Do not model source locations as fake renderer folders.
- Do not expose `source_location` as default user-facing copy.
- Do not derive source-location identity from display labels.
- Do not make recursive contents depend on expanded tree state.
- Do not crawl renderer tree nodes to build contents rows.
- Do not make every folder a source location.
- Do not silently create user-visible source locations from weak heuristics.
- Do not silently hide missing source locations.
- Do not show a `Locations` grouping node by default.
- Do not let device-observed source locations affect aggregate scope before acceptance.
- Do not include `is_user_visible = 0` source locations in source-level aggregate scope.
- Do not treat zero reachable source locations the same as zero configured source locations.
- Do not allow nested source locations under the same source in MVP.
