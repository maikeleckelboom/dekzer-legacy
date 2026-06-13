# Local Browser Candidate Rows Contract

## Purpose And Non-Goals

Local browser candidate rows represent bounded, pre-admission filesystem browse results. They let the user inspect an
entry point or descendant folder before choosing whether to request source root admission.

Core law:

> A local browser child row is not an admitted source row.

Candidate rows are not source hierarchy rows, not contents rows, not source inventory rows, and not durable library
state. They must not allocate source ids, source-location ids, source-directory ids, source-file ids, content attachment
ids, navigation-row ids, or track/candidate ids.

V0 does not implement visible renderer UI, workspace layout, automatic source registration, scan start, waveform,
analysis, playlists, CUE interpretation, canonical track identity, or source inventory mutation.

## Row Classes

The finite V0 local child row classes are:

| Row class                  | Meaning                                                                 |
| -------------------------- | ----------------------------------------------------------------------- |
| `directoryCandidate`       | Immediate child directory that may be browsed and may request admission. |
| `mediaFileCandidate`       | Immediate media-relevant file candidate shown before admission.          |
| `unsupportedFileCandidate` | Immediate file that is accessible but not media-relevant for V0 action.  |
| `rejectedRootCandidate`    | Directory candidate whose path is displayable but not directly admissible. |
| `inaccessibleCandidate`    | Child path whose metadata or access state cannot be resolved safely.     |
| `unknownCandidate`         | Child path with an unknown or unsupported filesystem kind.               |

V0 may include files only as local candidate rows. Files discovered by a local browser child read must not appear in
admitted-source tree rows. Admitted-source trees remain source, source-location, and source-directory scoped.

## Identity

Candidate identity is the tuple:

```
(entryPointKind, rootCanonicalPath, candidateCanonicalPath)
```

`entryPointKind` identifies the entry point class. `rootCanonicalPath` identifies the local browser root that bounds the
read. `candidateCanonicalPath` identifies the child candidate path resolved for this read.

Labels are not identity. Relative display paths are not identity. Candidate identity is resolved at read time and is not
persisted as source identity before admission.

## Selector Model

Future renderer selection must distinguish local browser selectors from admitted-source selectors:

- A local browser selector targets local entry points and local candidate rows.
- An admitted-source selector targets source, source-location, and source-directory rows.

The current admitted-source tree and contents selectors must not be reused for local candidate rows. Local browser child
rows do not satisfy `LibraryTreeEntryPoint`, source-directory selection, or `ContentsScope`.

## Browse Versus Scan

Local child reads are bounded candidate reads. They may enumerate bounded immediate children of the selected local
candidate path.

They must not:

- recurse by default;
- hash files;
- run media probes;
- create `SourceFacts`;
- create `source_files`;
- create `source_directories`;
- create `source_locations`;
- create `navigation_rows`;
- populate search/filter rows;
- publish source lifecycle events;
- start scan work;
- materialize attachments;
- promote primary media;
- produce track identity candidates.

Registration and scan remain explicit later actions after source root admission.

## Windowing

The V0 local child read window includes:

- `windowIdentity`: `entryPointKind`, `rootCanonicalPath`, and `parentCanonicalPath`;
- `offset`;
- `limit`;
- `totalRows`;
- `rows`;
- read `status`;
- optional read `failure`;
- optional row `failure`.

Offset/limit pagination is acceptable for V0. Cursor-based pagination can replace it later if filesystem ordering,
resume, or mutation behavior requires stronger cursor identity.

## Ordering

Local child rows are ordered by row class group:

1. directories, including rejected root candidates;
2. media-relevant files;
3. unsupported files;
4. inaccessible candidates;
5. unknown candidates.

Within each group, use stable platform-normalized display/path ordering.

## Media Relevance

Media relevance follows the current policy and classification vocabulary in
[`policy-and-classification.md`](../browse/policy-and-classification.md) and the media-relevant inventory contract.

Audio, video, image, and companion metadata extensions are candidate display hints before admission. Classification
before admission is provisional and must not create durable file classification state.

## Admission Handoff

A directory candidate can request source root admission for its `candidateCanonicalPath`.

A media file candidate cannot be registered as a source root directly. It may offer a later action to choose the parent
directory for admission. Unsupported, inaccessible, and unknown file candidates are not directly admissible.

Rejected root candidates must surface a reason and must not silently no-op. Admission hints are not admission results.
Only source root admission can accept, require confirmation, or reject a path for persistence.

## Renderer Projection Contract

Future renderer projection must keep these row kinds distinct:

- local entry point rows;
- local child candidate rows;
- admitted source rows;
- source location rows;
- source directory rows;
- contents/media rows.

Do not collapse them into one generic library row type. The same canonical path can appear as a candidate and as an
admitted source representation, but those are different objects with different authority and actions.

## Replacement After Admission

After a candidate path is admitted and registered, the UI should replace or suppress the candidate in favor of the
admitted source representation. It must not show duplicate rows as equivalent objects. Exact admitted-source path matches
may be marked as `duplicateOfAdmittedSource` during local candidate reads without mutating source state.

## Acceptance Bars

Backend read acceptance:

- `readLocalBrowserChildren` validates root identity, parent path, offset, and limit.
- Reads enumerate immediate children only and apply a bounded limit.
- Windows V0 avoids following symlink or junction escapes and maps missing, permission, unavailable, and unsupported
  states to local child read or row failures.
- Non-Windows V0 returns `unsupportedPlatform` consistently.
- Exact admitted source path matches are marked with duplicate status/hint without mutating sources.
- Local child reads do not create source lifecycle rows, source locations, source directories, source files, SourceFacts,
  navigation rows, search/filter rows, attachment rows, scan jobs, or source events.

Protocol acceptance:

- The snapshot command is `readLocalBrowserChildren`.
- Request identity includes `entryPointKind`, `rootCanonicalPath`, `parentCanonicalPath`, `offset`, and `limit`.
- Reply identity includes read status, window identity, offset, limit, totalRows, rows, and failure.
- Rows include candidate identity, finite row kind, display name, candidate status, platform, optional file kind, optional
  media relevance hint, admission hint, affordances, and optional failure.
- The row type is separate from `NavigationRow`, `LibraryTreeNode`, `ContentsRow`, and search/filter results.

Test acceptance:

- Tests cover bounded immediate children, offset/limit, ordering, media and unsupported file candidates, non-recursion,
  symlink/junction refusal, missing/access failure mapping, duplicate marking, media-file parent admission handoff,
  system-drive read-only behavior, non-Windows unsupported behavior, and unchanged application table row counts.

Desktop forwarding acceptance:

- Main, shared, and preload expose the read as a local browser child read.
- No renderer filesystem crawling is introduced.
- No visible renderer UI or state is implemented in this slice.

Future renderer UI acceptance:

- Local entry point rows, local child candidate rows, admitted sources, source locations, source directories, and
  contents/media rows remain visually and semantically distinct.
- Candidate replacement after admission prevents duplicate equivalent objects.
- Renderer actions request admission or browse through backend boundaries only.
