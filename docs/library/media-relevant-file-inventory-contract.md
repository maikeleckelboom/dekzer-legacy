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
| ---------------------------- | --------------------------------------------------------------------------------------------------------- |
| `source_file_id`             | Stable durable file row identity inside the store.                                                        |
| `source_id`                  | Owning source root.                                                                                       |
| `parent_source_directory_id` | Immediate directory when known.                                                                           |
| `relative_path`, `name`      | Source-relative location and display filename.                                                            |
| `size_bytes`, `mtime_ns`     | Observed filesystem metadata when available.                                                              |
| `file_kind`                  | Fine path-derived classifier: audio, video, image, cue_sheet, log_doc, text_doc, archive, other, unknown. |
| `file_class`                 | Coarse class: audio, video, image, unsupported, none.                                                     |
| `presence_state`             | present, missing, or removed.                                                                             |
| timestamps                   | First discovery, last observation, presence change, creation, update.                                     |

`file_kind` and `file_class` are provisional path-derived facts. They are not proof of playability, readiness, track
identity, or artwork role.

## Default Contents Policy

The renderer default for selected library contents is audio-first and audio-only in V0:

| Field         | Value         |
| ------------- | ------------- |
| `policy.kind` | `audioBrowse` |
| `scopeDepth`  | recursive     |

The renderer derives the scope from selection and sends this policy to `readContents`. The backend owns the query and
admission. The renderer must not fan out tree children, synthesize directory contents, or answer the selected scope from
the hierarchy cache.

`audioBrowse` implies audio and has no caller-supplied class filter. Explicit inventory surfaces use
`sourceFileInventory.fileClasses`.

## Scope Behavior

| Scope          | Recursive behavior                                                                                              |
| -------------- | --------------------------------------------------------------------------------------------------------------- |
| source         | Reads rows under the whole source, or under accepted source locations when user-visible source locations exist. |
| sourceLocation | Reads rows under that registered location prefix.                                                               |
| directory      | Reads rows under that directory prefix.                                                                         |

Immediate scope depth reads only immediate files for the selected scope. Recursive source and directory reads are backend
queries over durable source-file rows.

Unavailable, blocked, failed, or incomplete sources must return typed state. They must not be collapsed into an
authoritative empty result. Stored rows may still be returned with non-complete coverage when the source is unavailable
or indexing is incomplete.

## Explicit Source-File Inventory Admission

Explicit non-default source-file inventory reads may include:

| Stored facts                                                                    | Explicit inventory admission                                 |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `file_class = audio`                                                            | Include.                                                     |
| `file_class = video`                                                            | Include when policy requests video.                          |
| `file_class = image`                                                            | Include when policy requests image.                          |
| `file_class = unsupported` and `file_kind = cue_sheet`                          | Include when policy requests unsupported companion metadata. |
| `file_class = unsupported` and `file_kind` is log_doc, text_doc, archive, other | Exclude from normal inventory.                               |
| `file_class = none` or `file_kind = unknown`                                    | Exclude from normal inventory.                               |

`none` is a persisted/internal classification for files that received no more specific class during scan. It is not a
requestable `ContentsFileClass` value. A future diagnostic or raw inventory mode may choose to expose it, but that
requires a separate decision.

This contract intentionally does not admit every `unsupported` row. `unsupported` is too broad for product inventory
because it can include notes, PDFs, archives, binary data, and other unrelated files. CUE sheets are admitted because
they are media-adjacent companion metadata. They remain source-file rows and never become primary media rows.

Changing the unsupported admission scope (for example, admitting `log_doc` or `text_doc` unsupported rows, or removing
the cue-sheet filter) requires a separate decision. The current behavior is preserved.

The V0 main contents browse default requests only `audio`. It does not include video, image, unsupported, CUE, text, or
metadata companion files unless a future explicit product surface chooses a broader policy.

## CUE And Images

CUE sheets are represented only as `sourceFileInventory` rows with `fileClass = unsupported` and
`fileKind = cueSheet`.
The inventory does not pair CUE sheets with FLAC files, does not parse track splits, and does not infer a playable
primary-media item from a CUE file.

Image rows are represented only as image files. The inventory does not decide whether an image is cover art, label art,
folder art, or unrelated imagery.

## Presence

`sourceFileInventory` contents rows may include present, missing, and removed media-relevant files. The presence state must be
shown honestly. A missing or removed row is still a durable inventory fact.

`primaryMedia` rows are playable/performance projection rows and remain present-file scoped. They may include audio and
video only. A policy that asks `primaryMedia` for image or unsupported rows must return `policyConflict`.

## Ordering And Cursor

`sourceFileInventory` rows use the persisted source-file browse order. Cursor identity includes
scope, the complete policy discriminant, scopeDepth, the requested `fileClasses`, and the last row ordering position.
The file-class identity is derived from the requested policy, not from the rows returned on the current page. Changing
`fileClasses` across pages returns `cursorInvalid`.

Cursor resumption must follow the same ordering contract and produce no duplicates and no gaps. Cursor mismatches return
`cursorInvalid`.

The cursor must page the same backend query. It must not switch to renderer-local filtering or tree fanout.

## Projection Boundary

Rust and SQLite own durable facts and read-model admission. Boundary protocol exposes `fileClass`, `fileKind`,
`presence`, and stable source-file identifiers.

The renderer may project icons and labels:

| Row facts              | Renderer projection |
| ---------------------- | ------------------- |
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
