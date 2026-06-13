# Local Browse Items Contract

## Purpose And Non-Goals

This contract defines `LocalBrowseItem`: an immediate path item returned by a bounded local browse item read.

Core law:

> A `LocalBrowseItem` is not an admitted source row.

Local browse items are ephemeral read results. They may represent a directory, media-relevant file, unsupported file,
inaccessible path, rejected system path, or unknown filesystem item. They are not source directories, source files,
contents rows, search rows, durable inventory rows, or track identity rows.

V0 does not implement visible renderer UI, workspace layout, automatic source registration, scan start, waveform,
analysis, playlists, CUE interpretation, canonical track identity, source inventory mutation, or renderer filesystem
crawling.

Local browse entry points and local browse items are separate boundary concepts. Entry points are platform/default browse
roots. Items are bounded path entries under one validated entry point root and parent path.

## Item Kinds

The finite V0 item kinds are:

| Item kind         | Meaning                                                                  |
| ----------------- | ------------------------------------------------------------------------ |
| `directory`       | Immediate directory item that may be browsed and may request admission.  |
| `mediaFile`       | Immediate media-relevant file shown before admission.                    |
| `unsupportedFile` | Immediate file that is accessible but not media-relevant for V0 action.  |
| `rejectedRoot`    | Directory path that is displayable but rejected by local browse policy.  |
| `inaccessible`    | Item path whose metadata or access state cannot be resolved safely.      |
| `unknown`         | Item path with an unknown or unsupported filesystem kind.                |

V0 may include files only as local browse items. Files discovered by a local browse item read must not appear in
admitted-source tree rows. Admitted-source trees remain source, source-location, and source-directory scoped.

## Identity

`LocalBrowseItemIdentity` is:

```text
(entryPointKind, rootCanonicalPath, itemCanonicalPath)
```

`entryPointKind` identifies the entry point kind. `rootCanonicalPath` identifies the validated local browse root that
bounds the read. `itemCanonicalPath` identifies the immediate item path resolved for this read.

Labels are not identity. Relative display paths are not identity. Item identity is resolved at read time and is not
persisted as source identity before admission.

## Window Identity

`LocalBrowseWindowIdentity` is:

```text
(entryPointKind, rootCanonicalPath, parentCanonicalPath)
```

The item read validates `entryPointKind` and `rootCanonicalPath` against the current resolved entry points using the same
canonical normalization used for entry point deduplication. A mismatched pair fails the read with
`rootIdentityMismatch` and must not browse the requested parent.

After the root is validated, `parentCanonicalPath` may target the root or a descendant directory. The parent path must
remain inside the validated root boundary. Local browse root validation does not register a source, does not create
source lifecycle state, and does not start scan work.

## Selector Model

Future renderer selection must distinguish local browse selectors from admitted-source selectors:

- A local browse selector targets local entry points and local browse items.
- An admitted-source selector targets source, source-location, and source-directory rows.

The current admitted-source tree and contents selectors must not be reused for local browse items. Local browse items do
not satisfy `LibraryTreeEntryPoint`, source-directory selection, or `ContentsScope`.

## Browse Versus Scan

Local browse item reads are bounded pre-admission reads. They may enumerate immediate items under the selected local
parent path.

They must not:

- recurse by default;
- hash files;
- run media probes;
- create `source_file_facts`;
- create `source_files`;
- create `source_directories`;
- create `source_locations`;
- create `navigation_rows`;
- populate search/filter rows;
- publish source lifecycle events;
- start scan work;
- materialize attachments;
- promote primary media;
- produce track identity work.

Registration and scan remain explicit later actions after source-root admission.

## Windowing

The V0 item read reply includes:

- `windowIdentity`: `entryPointKind`, `rootCanonicalPath`, and `parentCanonicalPath`;
- `offset`;
- `limit`;
- `totalItems`;
- `items`;
- read `status`;
- optional read `failure`;
- optional item `failure`.

V0 keeps offset/limit and exact `totalItems`. The implementation may enumerate immediate directory entries into
lightweight ordering keys, sort those keys, and materialize only the requested window of protocol items. It must not
construct full protocol item rows for every directory entry before applying `offset` and `limit`.

Cursor pagination can replace offset/limit later if filesystem ordering, resume, or mutation behavior requires stronger
cursor identity.

## Ordering

Local browse items are ordered by item group:

1. directories, including rejected roots;
2. media-relevant files;
3. unsupported files;
4. inaccessible items;
5. unknown items.

Within each group, use stable platform-normalized display/path ordering.

## Media Relevance

Media relevance follows the current policy and classification vocabulary in
[`policy-and-classification.md`](../browse/policy-and-classification.md) and the media-relevant inventory contract.

Audio, video, image, and companion metadata extensions are local browse item display signals before admission.
Classification before admission is provisional and must not create durable file classification state.

## Available Actions

`availableActions` is backend-declared action availability. It is not renderer visual state.

Item actions are:

- `canBrowse`;
- `canRequestAdmission`;
- `canChooseDescendant`;
- `canRequestParentAdmission`.

Rules:

- A directory item may be browseable and may request admission for its path.
- A media file item cannot request source admission directly.
- A media file item may request parent admission.
- Unsupported, inaccessible, and unknown file items are not directly admissible.
- `systemDriveRoot` may be browseable as an entry point while rejected system-owned descendants are rejected items.
- `duplicateOfAdmittedSource` remains a status, not an action.

## Admission Action

`admissionAction` is a display/action recommendation for the later source-root admission flow. It is not an admission
result, does not persist anything, does not start scan, and cannot bypass root admission.

Item actions:

- `requestAdmission`: the UI may ask source-root admission to evaluate this directory item path.
- `requestParentAdmission`: the UI may ask source-root admission to evaluate the parent directory of this media file.
- `null`: no item admission action is available.

`requiresConfirmation` is not a local browse action. Confirmation belongs to source-root admission results.

## Renderer Projection Contract

Future renderer projection must keep these concepts distinct:

- local browse entry point rows;
- local browse item rows;
- admitted source rows;
- source location rows;
- source directory rows;
- contents/media rows.

Do not collapse them into one generic library row type. The same canonical path can appear as a local browse item and as
an admitted source representation, but those are different objects with different authority and actions.

The next implementation step after this cleanup is renderer projection/UI, not more substrate naming churn.

## Replacement After Admission

After an item path is admitted and registered, the UI should replace or suppress the local browse item in favor of the
admitted source representation. It must not show duplicate rows as equivalent objects. Exact admitted-source path matches
may be marked as `duplicateOfAdmittedSource` during local browse reads without mutating source state.

## Acceptance Bars

Backend read acceptance:

- `readLocalBrowseItems` validates root identity, parent path, offset, and limit.
- A mismatched `entryPointKind`/`rootCanonicalPath` pair fails the read before parent browsing.
- Reads enumerate immediate items only.
- The implementation bounds protocol item materialization to the requested window.
- Windows V0 avoids following symlink or junction escapes and maps missing, permission, unavailable, and unsupported
  states to local browse read or item failures.
- Non-Windows V0 returns `unsupportedPlatform` consistently.
- Exact admitted source path matches are marked with duplicate status without mutating sources.
- Local browse item reads do not create source lifecycle rows, source locations, source directories, source files,
  `source_file_facts`, navigation rows, search/filter rows, attachment rows, scan jobs, or source events.

Protocol acceptance:

- The snapshot command is `readLocalBrowseItems`.
- Request identity includes `entryPointKind`, `rootCanonicalPath`, `parentCanonicalPath`, `offset`, and `limit`.
- Reply identity includes read status, window identity, offset, limit, `totalItems`, `items`, and failure.
- Items include item identity, finite item kind, display name, item status, platform, optional file kind, optional media
  relevance, optional admission action, available actions, and optional failure.
- The item type is separate from `NavigationRow`, `LibraryTreeNode`, `ContentsRow`, and search/filter results.

Test acceptance:

- Tests cover root identity validation, mismatched root rejection, bounded immediate items, offset/limit, ordering, media
  and unsupported files, non-recursion, missing/access failure mapping, duplicate marking, media-file parent admission
  action, system-drive read-only behavior, non-Windows unsupported behavior, unchanged application table row counts, and
  bounded materialization behavior.

Desktop forwarding acceptance:

- Main, shared, and preload expose the read as a local browse item read.
- No renderer filesystem crawling is introduced.
- No visible renderer UI or state is implemented in this slice.
- Handwritten adapters use folder context. In `localBrowse/items`, helpers may be named `readItems`, `mapItem`,
  `errorResult`, and `isOutcome`; they must not repeat the full local-browse boundary name or carry a transport suffix
  unless one module truly implements multiple transports.

## Summary Term

`summary` is reserved for aggregate, count, or digest objects. Local browse item rows, replies, statuses, and action
policy fields must not use `Summary`.
