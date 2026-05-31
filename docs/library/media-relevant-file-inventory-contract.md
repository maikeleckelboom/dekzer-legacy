---
status: accepted
last-reviewed: 2026-05-31
owner: library-substrate-boundary
canonical-context:
  - source-root-scan-admission-contract
  - library-contents-read-boundary
  - library-tree-selection-contents-contract
scope:
  - source-file-inventory
  - contents-read-policy
  - media-admission
---

# Media-Relevant File Inventory Contract

## Decision

The scanner and durable store may retain raw source-file observations for every regular file they observe. The product
contents inventory must be hardened at the read-model policy boundary, not by deleting or refusing raw substrate facts.

This preserves diagnostic and future migration room while making the contents pane answer the product question:

> Which media-relevant files are known under this selected source or directory?

## Durable Facts

`source_files` owns durable file facts. The required inventory facts are:

| Fact                         | Meaning                                                                                                   |
|------------------------------|-----------------------------------------------------------------------------------------------------------|
| `source_file_id`             | Stable durable file row identity inside the store.                                                        |
| `source_id`                  | Owning source root.                                                                                       |
| `parent_source_directory_id` | Immediate directory when known.                                                                           |
| `relative_path`, `name`      | Source-relative location and display filename.                                                            |
| `size_bytes`, `mtime_ns`     | Observed filesystem metadata when available.                                                              |
| `file_kind`                  | Fine path-derived classifier: audio, video, image, cue_sheet, log_doc, text_doc, archive, other, unknown. |
| `media_class`                | Coarse class: audio, video, image, unsupported, none.                                                     |
| `presence_state`             | present, missing, or removed.                                                                             |
| timestamps                   | First discovery, last observation, presence change, creation, update.                                     |

`file_kind` and `media_class` are provisional path-derived facts. They are not proof of playability, readiness, track
identity, or artwork role.

## Default Contents Policy

The renderer default for selected library contents is:

| Field          | Value                            |
|----------------|----------------------------------|
| `rowProfile`   | `sourceFile`                     |
| `mediaClasses` | audio, video, image, unsupported |
| `recursion`    | recursive                        |

The renderer derives the scope from selection and sends this policy to `readContents`. The backend owns the query and
admission. The renderer must not fan out tree children, synthesize directory contents, or answer the selected scope from
the hierarchy cache.

## Scope Behavior

| Scope          | Recursive behavior                                                                                              |
|----------------|-----------------------------------------------------------------------------------------------------------------|
| source         | Reads rows under the whole source, or under accepted source locations when user-visible source locations exist. |
| sourceLocation | Reads rows under that registered location prefix.                                                               |
| directory      | Reads rows under that directory prefix.                                                                         |

Immediate recursion reads only immediate files for the selected scope. Recursive source and directory reads are backend
queries over durable source-file rows.

Unavailable, blocked, failed, or incomplete sources must return typed state. They must not be collapsed into an
authoritative empty result. Stored rows may still be returned with non-complete coverage when the source is unavailable
or indexing is incomplete.

## Media Admission

Default media-relevant source-file inventory includes:

| Stored facts                                                                     | Default inventory admission           |
|----------------------------------------------------------------------------------|---------------------------------------|
| `media_class = audio`                                                            | Include.                              |
| `media_class = video`                                                            | Include.                              |
| `media_class = image`                                                            | Include as an image file.             |
| `media_class = unsupported` and `file_kind = cue_sheet`                          | Include as a companion metadata file. |
| `media_class = unsupported` and `file_kind` is log_doc, text_doc, archive, other | Exclude from default inventory.       |
| `media_class = none` or `file_kind = unknown`                                    | Exclude from default inventory.       |

This contract intentionally does not admit every `unsupported` row. `unsupported` is too broad for product inventory
because it can include notes, PDFs, archives, binary data, and other unrelated files. CUE sheets are admitted because
they are media-adjacent companion metadata. They remain source-file rows and never become primary media rows.

## CUE And Images

CUE sheets are represented only as `sourceFile` rows with `mediaClass = unsupported` and `fileKind = cueSheet`.
The inventory does not pair CUE sheets with FLAC files, does not parse track splits, and does not infer a playable
primary-media item from a CUE file.

Image rows are represented only as image files. The inventory does not decide whether an image is cover art, label art,
folder art, or unrelated imagery.

## Presence

`sourceFile` contents rows may include present, missing, and removed media-relevant files. The presence state must be
shown honestly. A missing or removed row is still a durable inventory fact.

`primaryMedia` rows are playable/performance projection rows and remain present-file scoped. They may include audio and
video only. A policy that asks `primaryMedia` for image or unsupported rows must return `policyConflict`.

## Ordering And Cursor

`sourceFile` inventory rows are ordered by lowercased relative path, then `source_file_id`. Cursor identity includes
scope, row profile, recursion, the requested media classes including `unsupported`, and the last row ordering position.
The media-class identity is derived from the requested policy, not from the rows returned on the current page. Changing
media classes across pages returns `cursorInvalid`.

Cursor resumption must follow the same ordering contract and produce no duplicates and no gaps. Cursor mismatches return
`cursorInvalid`.

The cursor must page the same backend query. It must not switch to renderer-local filtering or tree fanout.

## Projection Boundary

Rust and SQLite own durable facts and read-model admission. Boundary protocol exposes `mediaClass`, `fileKind`,
`presence`, and stable source-file identifiers.

The renderer may project icons and labels:

| Row facts              | Renderer projection |
|------------------------|---------------------|
| audio                  | music/file row      |
| video                  | video/file row      |
| image                  | image/file row      |
| unsupported + cueSheet | cue sheet/file row  |

The renderer must not add scanning intelligence, CUE pairing, artwork role detection, file probing, polling, or sleep
based refresh logic.

## Future Extensions

A future diagnostic or developer view may expose all raw source-file observations. That must be a distinct policy or
surface. It must not weaken the default media-relevant inventory admission defined here.

A future media probing layer may promote additional unsupported files into richer media facts. That must be based on
durable evidence and must not overload the current path-derived `unsupported` class.
