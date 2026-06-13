# Contents Policy Shape

## Status

Implemented.

This document is the canonical support and decision record for the profile-specific contents policy and the
source-file classification vocabulary used by that policy.

## Decision Summary

The contents read protocol uses a discriminated policy union:

```ts
type ContentsReadPolicy =
  | { readonly kind: 'playableMediaBrowse' }
  | { readonly kind: 'audioBrowse' }
  | {
      readonly kind: 'sourceFileInventory'
      readonly fileClasses: readonly ContentsFileClass[]
    }
  | {
      readonly kind: 'playableMedia'
      readonly mediaKinds: readonly PlayableMediaKind[]
    }
```

Each policy variant owns its admissible filter space:

- `playableMediaBrowse` implies audio and video and accepts no caller-supplied class filter.
- `audioBrowse` implies audio and accepts no caller-supplied class filter.
- `sourceFileInventory` owns explicit source-file `fileClasses` filtering.
- `playableMedia` owns `mediaKinds`, which is playable-media vocabulary rather than source-file inventory vocabulary.
- Cursor identity binds the complete policy discriminant and canonicalized variant state.
- Renderer code does not filter, sort, or derive authoritative browse policy.

There is no global contents class filter and no row-profile object in the active protocol.

## Vocabulary Layers

### Detailed Source-File Taxonomy

`source_files.file_kind` is the detailed detected source-file taxonomy:

- `audio`
- `video`
- `image`
- `cue_sheet`
- `log_doc`
- `text_doc`
- `archive`
- `other`
- `unknown`

`file_kind` remains unchanged by the coarse-class rename.

### Coarse Source-File Inventory Classification

`source_files.file_class` is the persisted coarse source-file inventory classification used by browsing, SQL
predicates, directory rollups, and fast contents queries:

- `audio`
- `video`
- `image`
- `unsupported`
- `none`

The values and classification semantics are unchanged. `unsupported` and `none` are valid persisted inventory
classifications, which is why the durable concept is named `file_class`.

Boundary row observations use `fileClass`. Rust store and protocol types use `FileClass` vocabulary. The active implementation
does not expose a source-file row observation named with media-class vocabulary.

### Source-File Inventory Policy

`sourceFileInventory.fileClasses` filters coarse source-file inventory observations. The active contents policy domain exposes
`audio`, `video`, `image`, and `unsupported` as requestable browse classes.

`none` is a persisted/internal classification for files that could not be assigned a more specific class. It is not a
requestable `ContentsFileClass` value and is excluded from normal contents admission. A future diagnostic or raw inventory
mode may choose to expose it, but that requires a separate decision.

`unsupported` in the current `sourceFileInventory` browse path is not "all unsupported files." The store predicate
narrows unsupported rows to those admitted as media-relevant: currently `cue_sheet` file-kind rows only. This
cue-sheet-admitted behavior is the current product-admitted subset, not a statement that all unsupported files are
product-visible. Changing this admission scope requires a separate decision.

### Audio Browse Policy

`audioBrowse` is audio by definition:

- no caller-supplied class field;
- source-file audio parity for equivalent scope, scopeDepth, limit, and cursor behavior;
- the existing contents file-row payload with `fileClass`;
- no renderer-side filtering or sorting;
- no new audio row type or endpoint.

Extension-only classification treats `.m4a` as audio and `.mp4` as video. `audioBrowse` therefore excludes MP4 unless
future media-probe authority establishes a different product policy; it never widens based on extension ambiguity.

### Playable-Media Browse Policy

`playableMediaBrowse` is the canonical protocol policy for the product's **Media** filter:

- no caller-supplied class field;
- included durable classes are `audio` and `video`;
- images are browse-relevant but omitted by this policy;
- unsupported raw files, diagnostics-only files, generic metadata companions, and CUE sheets are ignored by this
  policy;
- rows reuse the existing contents file-row payload and source-file ordering;
- the renderer requests the policy kind and does not duplicate the audio/video class list.

### Policy Omission Metadata

`ContentsResult.hasPolicyOmittedRows` is required and service-owned. It is a scope-level boolean, not a page-local
count. The store applies the same scope, scopeDepth, presence, and policy universe used by the read.

- `playableMediaBrowse` omission candidates are browse-relevant images.
- `audioBrowse` omission candidates are browse-relevant video and image rows.
- `sourceFileInventory` omission candidates are requestable browse classes not selected by `fileClasses`; unsupported
  candidates remain limited to admitted CUE rows.
- `playableMedia` returns `false` in this slice and does not consult raw `source_files` as a proxy.

Incomplete coverage with zero rows is not authoritative empty. Complete zero-row results with omissions are empty only
for the active policy.

### Primary-Media Policy

`playableMedia` remains separate:

- its filter is `mediaKinds`;
- playable-media summaries and evidence fields keep `mediaKind` vocabulary;
- it is not a source-file inventory class rename target;
- it does not become a canonical-track surface.

The names `playableMedia`, `PlayableMediaKind`, `mediaKind`, and `mediaKinds` remain valid where they describe
playable-media policy, assets, analysis, or evidence.

## Directory Rollups

The directory observations remain:

- `source_directories.has_playable_media_descendant`
- `source_directories.has_image_media_descendant`

These are directory-level media-descendant rollups. Their names and semantics are unchanged.

Tree navigation remains navigation-only. Renaming source-file classification observations does not add source-file or audio
rows to tree navigation.

## Cursor Identity

Contents cursor identity binds:

- scope;
- the complete policy discriminant;
- canonicalized `sourceFileInventory.fileClasses` or `playableMedia.mediaKinds`;
- scopeDepth;
- request generation/key at the renderer contents boundary;
- the policy-specific ordering position.

Required invariants:

- `playableMediaBrowse` cursors cannot be reused by any other policy;
- `audioBrowse` cursors cannot be reused by `sourceFileInventory`;
- `audioBrowse` cursors cannot be reused by `playableMedia`;
- `sourceFileInventory` cursors reject changed `fileClasses`;
- `playableMedia` cursors reject changed `mediaKinds`;
- `immediate` and `recursive` scopeDepth values reject each other's cursors;
- source, source-location, and directory scopes reject each other's cursors even when their current filesystem ranges
  overlap;
- filter switches re-key contents reads and do not reuse retained rows as accepted rows for the new policy identity;
- ordering and pagination behavior remain unchanged.

The schema and row-field rename does not alter cursor policy identity.

## Implementation Guardrails

- Do not add compatibility aliases.
- Do not keep dual `file_class` and legacy source-file class columns.
- Do not change persisted classification values or semantics.
- Do not rename `file_kind`.
- Do not rename playable-media policy or `mediaKind` fields.
- Do not rename directory media-descendant rollups.
- Do not add a new endpoint, row payload type, or row union.
- Do not change source hierarchy or tree behavior.
- Do not add renderer filtering, sorting, or authoritative field derivation.
- Do not broaden the contents row or table design.

## Contract Invariants

The current implementation proves:

- schema, SQL, store rows, boundary rows, generated contracts, desktop adapters, and renderer projection use
  `file_class`/`fileClass`/`FileClass` for the coarse source-file inventory concept;
- classification values remain `audio`, `video`, `image`, `unsupported`, and `none`;
- `playableMediaBrowse` includes audio and video and backs the **Media** workflow filter;
- `audioBrowse` backs the initial **Audio** workflow filter;
- `audioBrowse` behavior remains equivalent to source-file audio reads;
- `sourceFileInventory.fileClasses` filtering remains intact;
- `playableMedia.mediaKinds` behavior remains intact;
- cursor identity remains policy-specific;
- renderer and tree authority boundaries remain unchanged.

## Decision Ratified

1. `file_kind` is the detailed source-file taxonomy.
2. `file_class` is the coarse persisted source-file inventory classification.
3. `sourceFileInventory.fileClasses` is the caller-facing inventory filter.
4. `playableMediaBrowse` implies audio and video and has no caller-supplied class filter.
5. `audioBrowse` implies audio and has no caller-supplied class filter.
6. Policy omission metadata is required and service-owned.
7. Playable-media vocabulary remains separate and valid.
8. The active shape uses no compatibility aliases.

## Non-Goals

- Audio browse field expansion.
- Canonical track identity.
- Playable-media product activation.
- Preparation-facet redesign.
- Source-file classification semantic changes.
- Renderer table redesign.
- Exclave changes.
