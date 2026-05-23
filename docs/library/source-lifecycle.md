# Source Removal and Root Lifecycle Semantics

## Current Implemented State

### Protocol surface (boundary-protocol)

| Command             | Direction          | Notes                                                            |
|---------------------|--------------------|------------------------------------------------------------------|
| `RegisterLocalRoot` | renderer → service | Registers absolute-path root. Returns `rootId`, `canonicalPath`. |
| `RunRootScan`       | renderer → service | Triggers scan by `rootId`. Returns scan summary.                 |
| `ReadLocalRoots`    | renderer → service | Returns all absolute-path roots with `available \| unavailable`. |

No remove/unregister/deactivate command exists.

### Domain types (library-domain)

| Type                      | Variants                                         | Purpose                                                                                                                                     |
|---------------------------|--------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------|
| `SourceAvailabilityState` | `Available`, `Unavailable`, `Degraded`           | Not persisted; derived from mount/resolution                                                                                                |
| `SourceResolutionStatus`  | `Resolved`, `Missing`, `Inaccessible`, `Unknown` | Persisted in `source_state.resolution_status`                                                                                               |
| `SourceScanPhase`         | `Idle`, `Scanning`, `Blocked`, `Failed`          | Persisted in `source_scan_state.scan_phase`                                                                                                 |
| `SourcePresenceState`     | `Present`, `Missing`, `Removed`                  | Persisted in `source_files.presence_state` and `source_directories.presence_state` — per-file observation state, not source-level lifecycle |

`SourcePresenceState::Removed` is for file-level observations, not source registration removal.

### Schema (SQLite baseline)

`sources` table columns:

- `source_id`, `source_class` (CHECK: `internal`, `external_mounted`, `removable_mounted`), `authority` (CHECK:
  `system`, `device`), `identity_key` (UNIQUE), `display_name`, `medium_label`, `is_user_visible` (0/1), `created_at`,
  `updated_at`

`source_locators` table:

- One row per source. `locator_kind` is `absolute_path` or `removable_volume`. FK to `sources ON DELETE CASCADE`.

`source_state` table:

- One row per source. `mount_status`, `resolution_status`, `mount_epoch`, `effective_path`, `last_seen_at`. FK to
  `sources ON DELETE CASCADE`.

`source_scan_state` table:

- One row per source. `scan_phase`, scan timestamps. FK to `sources ON DELETE CASCADE`.

`is_user_visible` is used by navigation projection queries (`WHERE is_user_visible = 1`) to filter sources from the
navigation tree. Currently always set to 1 on creation. This is the closest existing hook for "hide from UI" but it has
no "removed" or "inactive" semantic.

### Store (library-store-sqlite)

- `register_local_root`: canonicalizes path → `bootstrap_root` → upserts source + locator + state. Always sets
  `is_user_visible = 1`.
- `read_local_roots`: queries `source_locators` WHERE `locator_kind = 'absolute_path'`. Joins `source_state` for
  availability. Returns all registered local roots regardless of `is_user_visible`.
- Root lifecycle module (`authority/roots/lifecycle.rs`): full mount/unmount/eject state machine for removable volumes.
  All methods are `#[allow(dead_code)]`. Not exposed via protocol.
- Navigation projection (`publication/projections.rs`): filters `WHERE s.is_user_visible = 1`.

### Renderer (desktop)

- `readLocalRoots.ts`: `LocalRootAvailability` is `'available' | 'unavailable'`.
- `localRootActions.ts`: hydrates roots on startup; no remove action.
- `library-boundary-exposure.md`: `registerLocalRoot` is host-internal; `chooseAndRegisterLocal` is renderer-callable.

## Existing Dormant Substrate

The root lifecycle module in `authority/roots/lifecycle.rs` already implements:

| Component                   | Rust type                                                                  | Purpose                                                                                     |
|-----------------------------|----------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| Removable root registration | `RegisterRemovableRootInput`                                               | Enroll removable volume by device identity                                                  |
| Mount events                | `ApplyRootMountedInput`                                                    | Volume appeared at mount path                                                               |
| Unmount request             | `ApplyRootUnmountRequestedInput`                                           | OS/user requests eject                                                                      |
| Eject cancel                | `ApplyRootEjectCancelledInput`                                             | Eject cancelled                                                                             |
| Unmount pending             | `ApplyRootUnmountPendingInput`                                             | Unmount is in progress                                                                      |
| Unmounted event             | `ApplyRootUnmountedInput`                                                  | Volume gone                                                                                 |
| Change event                | `ApplyRootChangedInput`                                                    | Volume label/fs type changed                                                                |
| Scan lifecycle              | `mark_root_scan_started/completed/failed/blocked`                          | Scan state transitions                                                                      |
| Scan interruption           | `interrupt_root_bound_work` / `resume_root_bound_work`                     | Unmount-driven scan interruption                                                            |
| Availability refresh        | `refresh_absolute_path_root` / `refresh_root_availability_from_filesystem` | Re-evaluate local path resolution                                                           |
| Mount status                | `RootMountStatus`                                                          | `Unknown`, `Mounted`, `Unmounted`, `EjectRequested`, `EjectPending`                         |
| Resolution status           | `RootResolutionStatus`                                                     | `Unknown`, `Resolved`, `Missing`, `Inaccessible`                                            |
| Scan phase                  | `RootScanPhase`                                                            | `Idle`, `Scanning`, `BlockedUnavailable`, `InterruptedUnavailable`, `Failed`                |
| Availability mapping        | `availability_state_from_mount_and_resolution`                             | Derived: mounted+resolved → `available`; unmounted/unknown → `unavailable`; else `degraded` |

All lifecycle methods on `SqliteDurableStore` are `#[allow(dead_code)]`. The module header states: "Root lifecycle
remains schema-backed substrate for future device/root work, but it is not part of the maintained boundary protocol
center."

## Product Semantics

### Source states

| State                  | Meaning                                                                                                                          | Source still registered? | Files on disk?    | Navigation shows it?      |
|------------------------|----------------------------------------------------------------------------------------------------------------------------------|--------------------------|-------------------|---------------------------|
| **Active Available**   | Registered and path resolves                                                                                                     | Yes                      | Yes               | Yes, normal row           |
| **Active Unavailable** | Registered but path currently cannot be resolved (disk offline, drive unplugged for local paths, volume unmounted for removable) | Yes                      | Unknown           | Yes, with issue indicator |
| **Removed**            | Registration is inactive; user explicitly removed from Dekzer                                                                    | No (or inactive)         | Yes (not deleted) | No                        |

### Key distinctions

1. **Unavailable** — Dekzer still owns the registration. The source locator and state are intact. The path cannot
   currently be resolved. This happens when: a local path stops resolving (drive letter changed, folder renamed), or a
   removable volume is unmounted. Navigation should show the source with an issue indicator. Resolution can become
   `Available` again later.

2. **Unmounted/Ejected** — A *removable volume* lifecycle state, not a removal. The volume is physically disconnected.
   The source registration persists. `RootMountStatus::Unmounted` means "volume is not present right now." This is a
   subset of Unavailable.

3. **Removed from Dekzer** — The user explicitly chose to unregister this source. The source row transitions to an
   inactive/removed registration state. It must NOT appear as an active library location. Files on disk are untouched.
   Historical metadata (file observations, scan history) is preserved by default for possible relink.

4. **Scan interrupted by unmount** — The scan phase transitions to `InterruptedUnavailable` or `BlockedUnavailable`.
   This is a transient operational state within an active registration. Not removal.

5. **User canceled source registration** — If the user cancels during the add folder dialog, no registration is ever
   created. If cancellation happens after registration but before/during scan, this is handled by the scan phase state
   machine. Not a distinct removal state.

### Remove from Dekzer semantics

- Does **not** delete files from disk.
- Makes the source registration **inactive**: it must not appear in `readLocalRoots` results or navigation.
- Preserves `source_files`, `source_directories`, `source_locations` metadata unless there is an explicit product law
  for cleanup.
- Preserves the `sources` row so that re-adding the same path (or re-mounting the same device) can relink to existing
  metadata via `identity_key` uniqueness.
- Sets `is_user_visible = 0` on the source row, which already controls navigation visibility.
- The source can be **re-activated** by re-registering the same path; `upsert_source` will match on `identity_key` and
  restore the existing `source_id`.

### Re-add semantics

When a user re-adds a previously removed source:

- `register_local_root` canonicalizes the path → same `identity_key` → `upsert_source` finds existing row → sets
  `is_user_visible = 1`, updates `display_name` and timestamps.
- Existing metadata (file observations, scan history) is still linked to the same `source_id` and will be re-evaluated
  on next scan.

## Proposed Durable States

### `sources.is_user_visible` semantics (existing column, repurposed)

| Value | Meaning                                                                             |
|-------|-------------------------------------------------------------------------------------|
| `1`   | Active registration. Source appears in `readLocalRoots` and navigation.             |
| `0`   | Removed/inactive registration. Source is hidden from active queries and navigation. |

This column already exists and is already used by navigation projections. For local-path sources, a "remove" operation
sets `is_user_visible = 0`. A re-add sets it back to `1`. No schema change needed for the basic removal case.

### `source_state` availability mapping (existing)

The existing `availability_state_from_mount_and_resolution` function correctly maps:

- `Mounted + Resolved` → `available`
- `Mounted + !Resolved` → `degraded`
- `Unmounted/Unknown + *` → `unavailable`
- `EjectRequested/EjectPending + *` → `degraded`

No change needed to this mapping. The `is_user_visible` column controls whether a source is *in* the active set at all;
availability governs the display condition for active sources.

### Source lifecycle compound state

For any source, the effective lifecycle is the combination of:

```
is_user_visible  ×  mount_status  ×  resolution_status  →  behavior
     1                Mounted          Resolved           →  Active Available
     1                Mounted          Missing            →  Active Unavailable (degraded)
     1                Mounted          Inaccessible       →  Active Unavailable (degraded)
     1                Mounted          Unknown            →  Active Available (for removable)
     1                Unmounted        *                  →  Active Unavailable (volume gone)
     1                EjectReq/Pending *                  →  Active Degraded
     0                *                *                  →  Removed (hidden everywhere)
```

## Operations and Ownership

| Operation                      | Owner                               | Target                        | Effect                                                                                                 |
|--------------------------------|-------------------------------------|-------------------------------|--------------------------------------------------------------------------------------------------------|
| `RegisterLocalRoot`            | Protocol (existing)                 | Local path source             | Upsert source + locator + state; `is_user_visible = 1`; canonicalize path                              |
| `UnregisterLocalRoot`          | Protocol (new)                      | Active local root by `rootId` | Set `is_user_visible = 0` on source row; do not delete source, locators, state, or observations        |
| `ReadLocalRoots`               | Protocol (existing)                 | All active local roots        | Return absolute-path locators WHERE `is_user_visible = 1`; include unavailable ones                    |
| `RefreshLocalRootAvailability` | Host-internal (new or lifecycle)    | All active local roots        | Re-evaluate `resolution_status` for absolute-path roots on `refresh_root_availability_from_filesystem` |
| Removable mount/unmount events | Host-internal (lifecycle substrate) | Removable sources             | Transitions `mount_status`, `resolution_status`, `effective_path`; all dormant                         |

### UnregisterLocalRoot semantics

- Input: `rootId: i64`
- Validates: root exists, locator_kind = `absolute_path`
- Effect: `UPDATE sources SET is_user_visible = 0, updated_at = <now> WHERE source_id = <rootId>`
- Does not modify `source_state`, `source_locators`, `source_files`, `source_directories`, `source_locations`
- Triggers navigation projection reseed (source row changes visibility)
- Returns: `UnregisterLocalRootReply { unregistered: bool }`

### ReadLocalRoots semantics change

Currently queries all `source_locators WHERE locator_kind = 'absolute_path'` regardless of `is_user_visible`. Should
filter:

```sql
WHERE sl.locator_kind = 'absolute_path' AND s.is_user_visible = 1
```

This requires a JOIN to `sources` which is not present in the current query. The join already exists in navigation
projections.

## What `ReadLocalRoots` Should Mean

| Scenario                         | Current behavior                   | Proposed behavior                                             |
|----------------------------------|------------------------------------|---------------------------------------------------------------|
| 1 active local root, path exists | Returns root, `available`          | Same                                                          |
| 1 active local root, path gone   | Returns root, `unavailable`        | Same                                                          |
| 1 removed (unregistered) root    | Returns root, `unavailable`        | **Not returned** — excluded by `is_user_visible = 0`          |
| 1 active removable root, mounted | Not returned (locator_kind filter) | Same — `ReadLocalRoots` is for local absolute-path roots only |
| Mixed: 2 active, 1 removed       | Returns all 3                      | Returns 2 active only                                         |

For removable volumes, a separate `ReadRemovableRoots` or lifecycle event channel will be needed later. Not in scope
here.

### Protocol wire addition

`LocalRootAvailability` currently has `Available | Unavailable`. This remains correct for `ReadLocalRoots` — it
describes the runtime resolvability of an active registration, not its lifecycle status. Removed roots are simply absent
from the reply.

A future `ReadAllSources` admin query may include removed/inactive roots with a `registrationState` field. Not in scope.

## What Remove/Unregister Should Mean

1. User initiates "Remove source from library" in the UI.
2. Renderer sends `UnregisterLocalRoot { rootId }` via protocol.
3. Service sets `sources.is_user_visible = 0` for that `source_id`.
4. Navigation projection reseeded → source row disappears from tree.
5. `ReadLocalRoots` no longer returns this root.
6. All metadata (`source_files`, `source_directories`, `source_locations`, `source_state`, `source_scan_state`) remains
   in the database, linked to the same `source_id`.
7. User re-adds the same path → `RegisterLocalRoot` canonicalizes → same `identity_key` → `upsert_source` sets
   `is_user_visible = 1` → root reappears with existing metadata.

## What Navigation Should Show

| Source condition                       | Navigation behavior                                                            |
|----------------------------------------|--------------------------------------------------------------------------------|
| Active, available                      | Normal source row; full browsing                                               |
| Active, unavailable                    | Source row with issue indicator (e.g., dimmed, warning icon); limited browsing |
| Removed (`is_user_visible = 0`)        | Not shown as an active navigation row                                          |
| Historical removed (future admin view) | Not in current scope                                                           |

## What Protocol Commands Are Recommended

### Add now

| Command               | Request           | Reply                    | Notes                                                       |
|-----------------------|-------------------|--------------------------|-------------------------------------------------------------|
| `UnregisterLocalRoot` | `{ rootId: i64 }` | `{ unregistered: bool }` | Sets `is_user_visible = 0`. Requires absolute-path locator. |

### Future (not now)

| Command                                                               | Notes                                           |
|-----------------------------------------------------------------------|-------------------------------------------------|
| `RegisterRemovableRoot`                                               | Activates dormant removable-root substrate      |
| `ReadRemovableRoots`                                                  | Lists removable-volume sources                  |
| `ApplyRootMounted` / `ApplyRootUnmounted` / `ApplyRootEjectCancelled` | Device event feed from host                     |
| `ReadRootLifecycle`                                                   | Admin/debug: full lifecycle state for a source  |
| `RefreshLocalRootAvailability`                                        | Trigger filesystem re-check for all local roots |

## What Not to Implement Yet

- Do not add `RegisterRemovableRoot` or any removable-root protocol commands.
- Do not add `ReadRootLifecycle` or admin/history queries.
- Do not expose mount/unmount/eject event commands on the boundary protocol.
- Do not add a `removed` variant to `LocalRootAvailability` — removed roots are simply absent from `ReadLocalRoots`.
- Do not physically delete source rows, locators, state, file observations, or scan state.
- Do not add a soft-delete column or `removed_at` timestamp on `sources` for this slice. `is_user_visible = 0` is
  sufficient. A future migration may add `removed_at` for audit purposes.
- Do not touch `source_files.presence_state` — that is a per-file observation lifecycle, not the source registration
  lifecycle.
- Do not change navigation row removal — the existing `is_user_visible = 0` filter in projection queries already handles
  hiding.
- Do not implement UI for source removal.

## Schema Change Assessment

No schema migration is required for the basic `UnregisterLocalRoot` slice:

- `sources.is_user_visible` already exists as `INTEGER NOT NULL DEFAULT 1 CHECK (is_user_visible IN (0, 1))`.
- Navigation projections already filter on `WHERE s.is_user_visible = 1`.
- `source_locators`, `source_state`, `source_scan_state` FK to `sources ON DELETE CASCADE` — but we are not deleting,
  only updating `is_user_visible`.

The `source_class` CHECK constraint (`IN ('internal', 'external_mounted', 'removable_mounted')`) currently does not have
an `inactive` or `removed` variant. This is correct — `is_user_visible` is the registration active flag, not
`source_class`. No change needed.

A future migration could add a `registration_state` column (`'active'`, `'inactive'`) to `sources` replacing the
overloaded `is_user_visible` boolean. Not justified for this slice.

## Acceptance Criteria for Implementation Slice

1. `UnregisterLocalRoot` command added to `LibraryRootCommand` protocol enum with `rootId` request and
   `unregistered: bool` reply.
2. Boundary service validates `rootId > 0`, locates source, confirms `locator_kind = 'absolute_path'`, sets
   `is_user_visible = 0`.
3. `ReadLocalRoots` query filters to `is_user_visible = 1` by joining `sources`.
4. Navigation projection already hides `is_user_visible = 0` sources — verify this works end-to-end.
5. Re-registering the same local root path sets `is_user_visible = 1` and returns the existing `source_id` — existing
   metadata is intact.
6. Removing a root that is already removed is idempotent: returns `{ unregistered: true }`.
7. Removing a root that does not exist returns `{ unregistered: false }` or an error — decide and document.
8. Source files, directories, locations, observations are not touched by remove.
9. No destructive deletes on the `sources` row or any related data.
10. Protocol serialization tests for command/reply round-trip.
11. Integration test: register → read → unregister → read (absent) → re-register → read (present, same rootId).
12. The dormant lifecycle substrate remains behind `#[allow(dead_code)]` — no new protocol exposure.
