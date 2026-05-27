---
status: candidate
doctrine-version: 0.1
last-reviewed: 2026-05-28
owner: renderer-substrate-boundary
canonical-context:
  - product-doctrine-shortened
  - library-tree-frame-stability-contract
  - source-root-scan-admission-contract
  - source-hierarchy-contract (TODO: not yet written)
  - first-slice-substrate-map (TODO: not yet written)
scope:
  - source-visible-state
  - source-unavailability-ux
  - source-relocation-ux
  - source-removal-ux
  - tree-source-row-states
---

# Source Lifecycle Visible State Contract

## Core law

**A known source does not disappear from the user's view merely because its current filesystem resolution is
unavailable.**

The library is the DJ's collection. It is not the set of things currently mounted. The substrate knows about sources
whether or not they are accessible today. The product must reflect that knowledge honestly, not silently erase it
when the OS reports a drive as gone.

The only action that removes a source from the visible tree is an explicit user-initiated removal.

## Source lifecycle states

| State                  | Meaning                                                                             |
|------------------------|-------------------------------------------------------------------------------------|
| `mounted`              | Source is mounted and accessible. Reads and scans proceed normally.                 |
| `unavailable`          | Source is known but currently not reachable (drive ejected, network offline, etc.). |
| `relocating`           | Source path has changed; substrate is resolving the new location.                   |
| `blocked`              | Source is present but access is prevented (permissions, privacy gate, etc.).        |
| `partially_accessible` | Some subtrees are accessible; others are inaccessible or blocked.                   |
| `cloud_placeholder`    | Source file is a cloud sync placeholder; content is not locally available.          |
| `removed_by_user`      | User has explicitly removed this source from the library.                           |
| `forgotten`            | Source has been explicitly purged from the substrate by user request.               |

`removed_by_user` and `forgotten` are the only states that remove a source from
the visible tree. They are distinct: `removed_by_user` may retain metadata;
`forgotten` purges durable substrate records.

## Tree behavior per state

| State                  | Source root row behavior                                              | Child branch behavior                                                           |
|------------------------|-----------------------------------------------------------------------|---------------------------------------------------------------------------------|
| `mounted`              | Render normally.                                                      | Render normally.                                                                |
| `unavailable`          | Keep root row. Show `unavailable` presentation with last-known label. | Keep last-known child structure in degraded presentation.                       |
| `relocating`           | Keep root row. Show resolving indicator. Preserve child structure.    | Keep last-known child structure until new resolution confirmed.                 |
| `blocked`              | Keep root row. Show `blocked` with reason code.                       | Keep last-known child structure in blocked presentation.                        |
| `partially_accessible` | Render normally. Mark accessible state.                               | Accessible branches render normally; inaccessible branches show `inaccessible`. |
| `cloud_placeholder`    | Keep root row. Show placeholder/offline indicator.                    | Show placeholder state for content-unavailable children.                        |
| `removed_by_user`      | Remove root row from tree.                                            | Cache cleared for this source.                                                  |
| `forgotten`            | Remove root row from tree. Substrate records purged.                  | Cache cleared for this source.                                                  |

## Required: source row must survive unavailability

When a source transitions to `unavailable`:

1. The source root row remains in the tree.
2. The last-known child structure remains visible in `inaccessible` presentation.
3. The source root row updates its presentation to reflect `unavailable`.
4. No scan or projection reads are issued against the unavailable source.
5. No cache clear is triggered solely by the unavailability transition.

The DJ's mental map of their library is preserved. The product communicates the
source's state, not its absence.

## Required: recovery is seamless

When a source transitions from `unavailable` back to `mounted`:

1. The source root row updates to `mounted` presentation.
2. Existing cached child structure is checked for staleness.
3. If the substrate node identities are preserved and the scan epoch is unchanged,
   cached rows may remain valid without a full re-read.
4. If a scan has run while the source was unavailable, affected branches are
   refreshed following the targeted invalidation rules from the frame-stability
   contract.
5. Recovery must not flash the tree to empty.

## Relocation behavior

Source relocation means the filesystem path to a source has changed, but the
substrate source identity is preserved.

Common cases:

- Drive remounts under a different letter (Windows)
- Cloud provider changes a local sync path
- User renames a root folder
- Junction or symlink resolves differently

Required behavior during relocation:

| Step                 | Behavior                                                       |
|----------------------|----------------------------------------------------------------|
| Relocation detected  | Keep source root row. Begin resolving new path.                |
| Resolution confirmed | Update `pathDisplay` on source row and affected child rows.    |
| Node IDs preserved   | Branch cache remains valid; no re-read required.               |
| Node IDs invalidated | Full cache clear for this source; re-read under new structure. |
| Resolution fails     | Transition to `unavailable`.                                   |

Path display updates must not cause frame flicker. A display string change is a
patch to an existing row, not a row replacement.

## Blocked and permission states

A source may be present but blocked by OS permissions, privacy gates, or security
policy. This is not the same as unavailable.

| Condition                      | Presentation                                         |
|--------------------------------|------------------------------------------------------|
| Full access denied             | Source root row shows `blocked` with reason.         |
| Privacy permission not granted | Source root row shows `blocked: privacy_permission`. |
| Partial subdirectory denial    | Affected subdirectory nodes show `inaccessible`.     |
| File-level permission error    | Affected file rows show `inaccessible`.              |

Blocked sources are still known sources. The root row is kept. The DJ can take
action (grant permission, contact IT, check OS settings) without the library
forgetting the source.

## Cloud placeholder behavior

A cloud sync source (iCloud, Dropbox, OneDrive) may show placeholder files that
are not locally available. These are not missing files. They are known content
that has not been downloaded.

Required behavior:

| Condition                              | Presentation                                                               |
|----------------------------------------|----------------------------------------------------------------------------|
| File locally available                 | Render normally.                                                           |
| File is a placeholder (not downloaded) | Show placeholder indicator. Mark `availability_state = cloud_placeholder`. |
| File is downloading                    | Show downloading indicator.                                                |
| Download failed                        | Show download failed indicator.                                            |

A cloud placeholder must not be presented as a missing file. The distinction
matters: missing means unknown filesystem absence; placeholder means known
deferred availability.

## Source removal vs forgetting

| Action        | User intent                         | Substrate behavior                                  | Tree behavior     |
|---------------|-------------------------------------|-----------------------------------------------------|-------------------|
| Remove source | Stop monitoring this source.        | Source marked inactive; metadata retained.          | Root row removed. |
| Forget source | Purge all knowledge of this source. | Source and all child records purged from substrate. | Root row removed. |
| Eject/unmount | OS-level action; no library intent. | Source transitions to `unavailable`.                | Root row kept.    |

Eject is never treated as removal. Only explicit user actions in the library UI
trigger removal or forgetting.

## Visible state and the contents panel

When a selected scope's source becomes unavailable, the contents panel shows
`unavailable` state for that scope. It does not clear its rows. It does not
reset selection.

When the source recovers, the contents panel re-validates its scope and refreshes
if needed, following the same stale-response rules as normal reads.

## Source state and preparation work

Active preparation work against an unavailable source is paused, not cancelled.
When the source recovers, work resumes if the relevant source files are still
present and accessible.

Preparation facet states for tracks attached to an unavailable source remain
in their last-known state. They are not reset to `unknown`.

## Non-goals

This contract does not define source admission, how new sources are registered,
scan scheduling, or the full source state machine at the substrate level. Those
are governed by the source-root-scan-admission-contract. This contract governs
only what the user sees in the tree and contents panel during source lifecycle
transitions.
