# Local Browser Entry Points Contract

## Purpose

This contract defines local browser entry points: displayable local filesystem locations that let the user begin
browsing immediately and then choose a path for source root admission.

Core law:

> A displayable local browser entry point is not an admitted source.

Local browser entry points are browse candidates and admission candidates. They are not source rows, not source
locations, not source scan roots, not scan plans, and not durable library inventory.

This contract owns:

- local filesystem entry point classes;
- browse candidate identity;
- browse candidate status;
- the browse-versus-scan boundary before admission;
- the handoff from a selected path to source root admission.

## Non-Goals

Local browser entry points must not create, own, or imply any of the following before source admission succeeds:

- silent import;
- silent indexing;
- hidden scan;
- source lifecycle rows or half-states;
- `source_locations`;
- `source_files`;
- `source_directories`;
- `SourceFacts`;
- `navigation_rows`;
- source scan roots;
- source facts;
- renderer-owned filesystem crawling.

A rejected or confirmation-required path must not become a partial source object while waiting for user action.

## Product Model

Dekzer should feel local-first immediately: obvious local places are visible without setup. Users can see the OS drive,
other local drives, removable drives, home, Desktop, Downloads, and Music without first understanding the source model.

Broad paths remain safe because browsing is not scanning. A bounded local browse read may show a user what is nearby,
but source registration remains explicit and scanning remains explicit.

Admission remains service-owned. The renderer may display entry points, local candidate rows, admission results, warning
copy, and action states, but it does not classify source roots or decide whether a path may persist.

## Entry Point Classes

V0 is Windows-first. These entry point classes are the canonical V0 classes.

| Entry point class       | Meaning                                                                  | Source admission stance                                      |
| ----------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------ |
| `systemDriveRoot`       | Root of the OS/system volume, such as the resolved Windows system drive. | Displayable browse entry point; rejected as a source root.   |
| `localDataVolumeRoot`   | Local non-system volume root.                                            | Requires admission confirmation.                             |
| `removableVolumeRoot`   | Removable drive root.                                                    | Requires admission confirmation and removable policy.        |
| `userHome`              | Current user's home directory.                                           | Requires admission confirmation.                             |
| `desktop`               | Current user's Desktop directory.                                        | Requires admission confirmation.                             |
| `downloads`             | Current user's Downloads directory.                                      | Requires admission confirmation.                             |
| `music`                 | Platform-resolved Music directory.                                       | Admission hint only; may be accepted directly only if source root admission returns `defaultMusicFolder`. |

The `music` class is the only class whose platform default path may be admitted directly without confirmation, and only
when the path is resolved by platform APIs and source root admission classifies it as `defaultMusicFolder`. The
`defaultMusicFolder` value on a local browser entry point is display/action guidance only. It is not an admission result
and does not bypass root admission.

### Cross-Platform Navigation Aid

This table is non-normative. It is a future navigation aid, not an implementation specification.

| Concept        | Windows                                             | macOS                                | Linux                                      |
| -------------- | --------------------------------------------------- | ------------------------------------ | ------------------------------------------ |
| Music folder   | Known Folder API `FOLDERID_Music`                   | `FileManager` music directory        | XDG `MUSIC`, with `~/Music` fallback       |
| Home directory | User profile directory                              | User home directory                  | User home directory                        |
| Volumes        | Drive roots and volume GUID paths                   | `/Volumes/<name>`                    | Mount points such as `/media` or `/mnt`    |
| Desktop        | Known Folder API `FOLDERID_Desktop`                 | Desktop user directory               | XDG `DESKTOP`, with `~/Desktop` fallback   |
| Downloads      | Known Folder API `FOLDERID_Downloads` when present  | Downloads user directory             | XDG `DOWNLOAD`, with `~/Downloads` fallback |

## Identity and Stable Keys

A local browser entry point is identified by the two-field tuple:

```
(EntryPointKind, canonicalPath)
```

`EntryPointKind` is the entry point class. `canonicalPath` is a platform-resolved absolute path resolved at read time.
It is suitable for candidate deduplication and action targeting, but it is never persisted as source identity before
admission.

Labels are not identity. "Music", "Downloads", drive labels, localized display names, and user-visible aliases may
change without changing the candidate identity.

Entry point rows are not source rows. They must not reuse source ids, source-location ids, directory ids, or source
lifecycle ids.

## Entry Point Status

Entry point status describes local browser candidate availability. It is not a source lifecycle state.

| Status                      | Meaning                                                                       |
| --------------------------- | ----------------------------------------------------------------------------- |
| `resolving`                 | Platform path or volume information is still being resolved.                  |
| `available`                 | The path exists and is cheaply reachable for bounded browse reads.            |
| `unavailable`               | The path cannot currently be reached, but absence is not proven permanent.    |
| `permissionBlocked`         | The path exists or is known, but access is blocked by permissions or policy.  |
| `missing`                   | The resolved path does not currently exist.                                   |
| `unsupportedPlatform`       | The entry point class has no supported resolver on the current platform.      |
| `duplicateOfAdmittedSource` | The candidate path exactly matches an already admitted source path.           |

These statuses must not be projected as `mounted`, `blocked`, `partial`, `scanning`, `completed`, or any other admitted
source lifecycle state.

Entry point status resolution is a local-browser read concern. It must use shallow path status checks only: no recursive
traversal, no directory enumeration, no source inventory materialization, and no child-row production. On Windows V0,
Known Folder resolution, system-drive root detection, fixed/removable volume discovery, and path status checks use
platform APIs. The path status check prefers attribute/status APIs that do not open files for content and maps missing,
permission-blocked, unavailable, and non-directory paths into local browser candidate statuses deterministically.

## Allowed and Disabled Actions

Allowed actions on a local browser entry point or local candidate row:

- browse bounded shallow children;
- choose a child folder;
- request admission for this path;
- request admission for a selected descendant path;
- start scan after admission succeeds, source registration succeeds, and the user explicitly starts the scan;
- open a folder picker.

Disabled or warning actions:

- `systemDriveRoot` can be shown as a browse entry point, but it cannot be registered as a source root.
- `userHome`, `desktop`, and `downloads` require confirmation through admission.
- `music` can be admitted directly only through platform resolution plus admission classification as `defaultMusicFolder`.
- `removableVolumeRoot` and `localDataVolumeRoot` require admission confirmation.
- Cloud and network roots require warning policy when detected by admission.

Admission hints on entry points are not admission results. `requiresConfirmation` does not persist anything by itself,
`defaultMusicFolder` does not skip the admission command, and `notDirectlyAdmissible` still allows browsing where this
contract permits browsing. The only path to persistence remains source root admission followed by source registration.

A rejected admission result from a non-admissible entry point must surface a clear rejection reason. It must not produce
an empty state, a silent no-op, or a partial lifecycle row.

## Browse Versus Scan

Local browse reads are bounded candidate reads. They are not source scans.

Local browse reads may:

- enumerate bounded child windows;
- expose child folders as local candidate rows;
- expose media-relevant files as local candidate rows;
- surface permission, missing, unavailable, and unsupported candidate states.

Local browse reads must not create or own `source_files`, `source_directories`, `SourceFacts`, `source_locations`, or
`navigation_rows` before admission. They must not populate search/filter rows, publish source scan lifecycle events,
start hashing, run media probes, materialize attachments, promote primary media, produce track identity, trigger cloud
downloads, or follow symlink or junction escapes.

## Admission Handoff

The handoff is explicit:

1. The user selects an entry point or descendant path.
2. The service runs source root admission.
3. Admission returns `accepted`, `acceptedWithWarning`, or `rejected`.
4. Registration happens only after accepted admission or confirmed `acceptedWithWarning`.
5. Scan starts only after source registration and explicit user action.

Admission may reject a path that was displayable as a local browser entry point. Displayability is a navigation affordance,
not persistence authority.

## Default Music Relationship

[`default-music-source-discovery.md`](default-music-source-discovery.md) is the implementation companion for the `music`
entry point class. This contract owns the general local browser entry point model.

The Music companion owns only platform Music path resolution, Music-specific candidate production, Music-specific
deduplication application, and admission handoff for `defaultMusicFolder`.

## Navigation and Tree Relationship

Current source/source-location navigation remains admitted-source navigation. A local browser entry point is not a
source navigation row until admission and registration have succeeded.

A future UI may show local browser entry points in the same left-side region as admitted sources. Even then, their
substrate owner and row type remain distinct: local browser entry points are local candidate rows, while admitted
sources, source locations, and directories remain source hierarchy rows.

Current tree selection and contents reads operate on admitted source selectors: source, source location, or source
directory. They do not operate on local browser candidate selectors.

## Local Browser Read Model Expectations

Implemented V0 backend read:

- `readLocalBrowserEntryPoints` returns the current platform's displayable local entry point candidates.
- The read returns candidate status, identity, display name, platform, admission hint, affordance flags, and platform
  failure detail when resolution partially fails.
- The read is a snapshot read boundary only. It does not register sources, create source lifecycle rows, populate
  navigation rows, start scans, or materialize inventory/facts/work.
- Exact admitted source canonical path matches are marked as `duplicateOfAdmittedSource` while preserving the candidate
  versus source distinction.
- The implemented read resolves only entry point candidates. It does not implement local child browsing, local browser
  UI, source admission, source registration, or scan start.

Future backend reads should satisfy these constraints:

- Read the available local browser entry points for the current platform.
- Read local entry point children with bounded windows.
- Classify child rows as directory, media-relevant file, admissible candidate, warning candidate, rejected root, or unknown.
- Perform no recursive traversal by default.
- Apply strict timeout budgets.
- Avoid cloud download triggers.
- Refuse symlink or junction escape traversal.
- Represent permission and access failures as row states.
- Return enough admission target information for the service to run source root admission on user action.

This section is a placeholder for future read boundaries, not an implementation specification.

## Deduplication and Overlap

An exact admitted source match marks the entry point or selected path as `duplicateOfAdmittedSource`.

Parent/child overlap is handled by source root admission when the user acts. Local browser entry point reads may still
show and browse visible candidates whose paths are related to an existing source.

The UI must not present duplicate source rows as if candidates and admitted sources are the same object. A candidate row
may point at the same canonical path as a source, but it must remain visually and semantically distinct or be replaced
by the admitted source row according to the active surface's projection rules.

## Open Implementation Questions

`DiscoveredMusicSource` must either become a specialization of the general `LocalBrowserEntryPoint` model or be replaced
by it during implementation.
