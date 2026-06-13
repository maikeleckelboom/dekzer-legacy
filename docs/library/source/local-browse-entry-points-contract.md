# Local Browse Entry Points Contract

## Purpose

This contract defines `LocalBrowseEntryPoint`: a platform/default browse root such as Music, Desktop, Downloads, the
system drive, a local data volume, a removable volume, or the user home.

Core law:

> A `LocalBrowseEntryPoint` is not an admitted source.

Entry points let the user begin a pre-admission local browse flow. They are not source rows, source locations, source
scan roots, scan plans, durable inventory, navigation rows, or renderer-owned filesystem state.

This contract owns:

- entry point kinds and identity;
- entry point status;
- entry point available backend actions;
- entry point admission action guidance for the later source-root admission flow;
- the entry-point side of the browse-versus-scan boundary.

[`local-browse-items-contract.md`](local-browse-items-contract.md) owns `LocalBrowseItem`, immediate child item
classification, local browse item window identity, item read windowing, and item action policy.

## Non-Goals

Local browse entry point reads must not create, own, or imply any of the following before source admission succeeds:

- silent import;
- silent indexing;
- hidden scan;
- source lifecycle rows or half-states;
- `source_locations`;
- `source_files`;
- `source_directories`;
- `SourceFacts`;
- `navigation_rows`;
- search/filter rows;
- hash, probe, attachment, primary-media, or track-identity work;
- renderer-owned filesystem crawling.

A rejected or confirmation-required path must not become a partial source object while waiting for user action.

## Product Model

Dekzer should feel local-first immediately: obvious local places are visible without setup. Users can see the OS drive,
other local drives, removable drives, home, Desktop, Downloads, and Music without first understanding the source model.

Broad paths remain safe because browsing is not scanning. A local browse read may show immediate nearby paths, but source
admission, source registration, and scan start remain separate explicit actions.

Entry points and items are different concepts:

- `LocalBrowseEntryPoint` is a platform/default browse root.
- `LocalBrowseItem` is an ephemeral immediate child path returned by a bounded local browse item read.

Neither is an admitted source.

## Entry Point Kinds

V0 is Windows-first. These entry point kinds are the canonical V0 kinds.

| Entry point kind       | Meaning                                                                  | Source admission stance                                             |
| ---------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------- |
| `systemDriveRoot`      | Root of the OS/system volume, such as the resolved Windows system drive. | Browseable entry point; not directly admissible as a source root.    |
| `localDataVolumeRoot`  | Local non-system volume root.                                            | May request admission; admission policy decides confirmation.        |
| `removableVolumeRoot`  | Removable drive root.                                                    | May request admission; removable policy decides warning/confirmation. |
| `userHome`             | Current user's home directory.                                           | May request admission; admission policy decides confirmation.        |
| `desktop`              | Current user's Desktop directory.                                        | May request admission.                                              |
| `downloads`            | Current user's Downloads directory.                                      | May request admission.                                              |
| `music`                | Platform-resolved Music directory.                                       | May request default Music admission action.                         |

`music` is the only entry point whose local browse action can be `requestDefaultMusicFolderAdmission`. That action is a
display/action recommendation only. It is not an admission result and cannot bypass root admission.

## Identity

A `LocalBrowseEntryPoint` is identified by:

```text
(entryPointKind, canonicalPath)
```

`entryPointKind` is the platform/default entry point class. `canonicalPath` is the platform-resolved absolute path at
read time. The tuple is suitable for local browse deduplication and action targeting, but it is never persisted as source
identity before admission.

Labels are not identity. Display names, drive labels, localized names, and user-visible aliases may change without
changing the entry point identity.

## Status

Entry point status describes local browse availability. It is not a source lifecycle state.

| Status                      | Meaning                                                                      |
| --------------------------- | ---------------------------------------------------------------------------- |
| `resolving`                 | Platform path or volume information is still being resolved.                 |
| `available`                 | The path exists and is cheaply reachable for bounded browse reads.           |
| `unavailable`               | The path cannot currently be reached, but absence is not proven permanent.   |
| `permissionBlocked`         | The path exists or is known, but access is blocked by permissions or policy. |
| `missing`                   | The resolved path does not currently exist.                                  |
| `unsupportedPlatform`       | The entry point kind has no supported resolver on the current platform.      |
| `duplicateOfAdmittedSource` | The entry point path exactly matches an already admitted source path.         |

These statuses must not be projected as `mounted`, `blocked`, `partial`, `scanning`, `completed`, or any other admitted
source lifecycle state.

Entry point status resolution is a local-browse read concern. It must use shallow path status checks only: no recursive
traversal, no directory enumeration, no source inventory materialization, and no item production.

## Available Actions

`availableActions` is backend-declared action availability. It is not renderer visual state.

Entry point actions are:

- `canBrowse`;
- `canRequestAdmission`;
- `canChooseDescendant`;
- `canRequestParentAdmission`.

Rules:

- An available entry point can be browsed.
- `systemDriveRoot` can be browseable while not directly admissible.
- An available non-system entry point may request admission for its path.
- `duplicateOfAdmittedSource` remains a status, not an action.
- Entry points never request parent admission.

## Admission Action

`admissionAction` is a display/action recommendation for the later source-root admission flow. It is not an admission
result, does not persist anything, does not start scan, and cannot bypass root admission.

Entry point actions:

- `requestAdmission`: the UI may ask source-root admission to evaluate this entry point path.
- `requestDefaultMusicFolderAdmission`: the UI may route the platform Music path into the default Music admission flow.
- `null`: no direct entry point admission action is available.

Confirmation remains owned by source-root admission results, not by local browse.

## Browse Versus Scan

Local browse reads are pre-admission snapshot reads. They may expose entry points and bounded immediate item windows.

They must not create or own `source_files`, `source_directories`, `SourceFacts`, `source_locations`, or
`navigation_rows` before admission. They must not populate search/filter rows, publish source scan lifecycle events,
start hashing, run media probes, materialize attachments, promote primary media, produce track identity, trigger cloud
downloads, or follow symlink/junction escapes.

## Admission Handoff

The handoff is explicit:

1. The user selects an entry point or descendant path.
2. The service runs source-root admission.
3. Admission returns `accepted`, `acceptedWithWarning`, or `rejected`.
4. Registration happens only after accepted admission or confirmed `acceptedWithWarning`.
5. Scan starts only after source registration and explicit user action.

Admission may reject a path that was displayable as a local browse entry point. Displayability is navigation state, not
persistence authority.

## Default Music Relationship

[`default-music-source-discovery.md`](default-music-source-discovery.md) is the implementation companion for the
`music` entry point kind. This contract owns the general `LocalBrowseEntryPoint` model.

The Music companion owns only platform Music path resolution, Music-specific production, Music-specific deduplication,
and admission handoff for default Music. Local browse items under Music remain owned by
[`local-browse-items-contract.md`](local-browse-items-contract.md).

## Navigation and Tree Relationship

Current source/source-location navigation remains admitted-source navigation. A local browse entry point is not a source
navigation row until admission and registration have succeeded.

A future renderer projection may show local browse entry points near admitted sources. Even then, their substrate owner
and row type remain distinct: local browse entry points are pre-admission browse roots, while admitted sources, source
locations, and directories remain source hierarchy rows.

Current tree selection and contents reads operate on admitted source selectors: source, source location, or source
directory. They do not operate on local browse entry point identity.

## Implemented V0 Read

Implemented V0 backend read:

- `readLocalBrowseEntryPoints` returns the current platform's displayable local browse entry points.
- The read returns identity, display name, status, platform, `admissionAction`, `availableActions`, and platform failure
  detail when resolution partially fails.
- The read is a snapshot read boundary only.
- Exact admitted source canonical path matches are marked as `duplicateOfAdmittedSource` while preserving the entry point
  versus source distinction.
- The next implementation step is renderer projection/UI. The substrate vocabulary is not expected to churn again before
  that projection work.

## Summary Term

`summary` is reserved for aggregate, count, or digest objects. Local browse entry point rows and item rows must not use
`Summary` for ordinary replies, status, action state, or row/item values.
