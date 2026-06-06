# Contents Policy Shape

## Status

Implemented.

This document is the canonical support and decision record for the implemented profile-specific contents policy.

The schema column `source_files.media_class` remains valid persisted inventory vocabulary and must not be renamed.

The contents read protocol uses profile-specific policy. The schema column `source_files.media_class` remains persisted
source-file inventory vocabulary. The active protocol no longer exposes a global `mediaClasses` filter.

---

## Decision summary

The contents read protocol must not expose a global `mediaClasses` filter.

The active shape is profile-specific policy:

- `audioBrowse` implies audio and accepts no caller-supplied class filter.
- `sourceFileInventory` owns explicit source-file class filtering.
- `primaryMedia` owns its profile-specific `mediaKinds` filter over the existing projection.
- `source_files.media_class` remains a persisted source-file inventory fact.
- Renderer code does not filter, sort, or derive authoritative browse policy.
- Cursor identity binds the full policy discriminant.

The implemented migration removed the protocol shape:

```
ContentsReadPolicy {
  rowProfile:   RowProfile
  mediaClasses: MediaClass[]
}
```

and replaced it with a discriminated policy union.

---

## Current problem

The contents read protocol currently has:

```
ContentsReadPolicy {
  row_profile:   RowProfile
  media_classes: [MediaClass]
}
```

The `row_profile` selects what kind of browse rows are returned. The `media_classes` list filters which source-file
media classes are included.

Before `audioBrowse` existed, this was tolerable. The main active profile was `sourceFile`, which returns source-file
inventory rows, so `media_classes` let the caller request audio, video, images, unsupported companion files, or some
combination.

After `audioBrowse`, the contract became redundant. A caller requests `audioBrowse` and must also pass
`media_classes: [audio]`. The store validates this: `audioBrowse` with any other `media_classes` value is a policy
conflict.

The enforcement is correct. The contract is not.

The contract should not expose an impossible choice. `audioBrowse` already implies audio.

---

## The smell

`audioBrowse` already declares the browse intent. Requiring the caller to also provide `media_classes: [audio]` is wrong
because it is:

- redundant: the profile already encodes the intent;
- misleading: it implies the caller could request `audioBrowse` with video, image, or unsupported files;
- leaky: it exposes source-file inventory vocabulary at the product browse level;
- defensive by construction: the store must reject combinations the protocol should not represent.

The contract makes invalid combinations unrepresentable.

---

## Three vocabulary layers that must stay separate

### Layer 1: Source-file inventory classification

This is a persisted store fact. It lives in `source_files.file_kind` and `source_files.media_class`.

`file_kind` is the detailed file taxonomy:

- `audio`
- `video`
- `image`
- `cue_sheet`
- `log_doc`
- `text_doc`
- `archive`
- `other`
- `unknown`

`media_class` is the coarser browse classification derived from `file_kind`:

- `audio`
- `video`
- `image`
- `unsupported`
- `none`

The column name `media_class` is correct. It is a persisted inventory fact about a source file.

`source_directories.has_primary_media_descendant` and `source_directories.has_image_media_descendant` are
directory-level rollups of this inventory classification. Their vocabulary is correct.

### Layer 2: Browse row profile

This is a product/read-model choice that selects the shape and authority of returned rows.

Active and target profile concepts:

- `audioBrowse`
- `sourceFileInventory`
- `primaryMedia`

The profile answers:

What kind of contents surface is the caller requesting?

The profile owns its admissible policy space.

`audioBrowse` is audio by definition. `sourceFileInventory` is raw source-file inventory and can accept explicit
file-class filters. `primaryMedia` is a separate profile that must be backed by a real media/identity projection before
product activation.

### Layer 3: Profile-specific filter space

This is the caller-selectable filter within a profile's admissible scope.

For `audioBrowse`:

- no caller filter;
- audio is implied by the profile.

For `sourceFileInventory`:

- caller-selectable source-file class filters;
- suggested field: `fileClasses`;
- preserves current source-file inventory behavior.

For `primaryMedia`:

- profile-specific media-kind policy only if required by the backed projection;
- suggested field: `mediaKinds`;
- not a canonical-track surface.

The old global `mediaClasses` field conflates all three layers into one protocol field. The new policy shape separates
them.

---

## Protocol shape

Conceptually:

```
{ kind: "audioBrowse" }
```

No filter field. The profile implies audio.

```
{
  kind: "sourceFileInventory",
  fileClasses: ["audio", "video", "image", "cueSheet", "unsupported"]
}
```

Caller-selectable source-file inventory filter.

```
{
  kind: "primaryMedia",
  mediaKinds: ["audio", "video"]
}
```

Profile-specific media-kind policy over the existing projection authority.

The exact Rust and generated TypeScript names should follow current project style, but the ownership rule is fixed:

Each policy variant owns its own admissible filter space.

There is no global `mediaClasses` field.

---

## Source-file inventory vocabulary

`source_files.media_class` is correct and must remain.

It answers:

What coarse browse classification does this persisted source file have?

It is not a product browse policy. It is not a contents profile. It is not a caller-level policy field.

The protocol may expose source-file inventory filtering through `sourceFileInventory.fileClasses`, but that is a
profile-specific filter over inventory facts, not a global media-class filter for every contents profile.

---

## Audio browse policy

`audioBrowse` implies audio.

The target request shape contains no class/filter field. A caller cannot ask for `audioBrowse` plus video, images,
unsupported files, or any other class.

The store/read model remains the authority:

- it returns source-file-backed audio rows;
- it preserves the current audio browse V0 file-row payload;
- it preserves parity with the current `sourceFile + audio` behavior;
- it preserves coverage, cursor, scope, recursion, and ordering behavior;
- it does not add extension, source label, source-location provenance, row version, container, codec, analysis,
  identity, or preparation fields.

---

## Source-file inventory policy

`sourceFileInventory` is the raw source-file inventory profile.

It owns explicit file-class filtering.

The target policy shape uses a profile-specific field such as `fileClasses`.

This profile preserves current source-file inventory behavior, including the ability to browse classes outside audio
when explicitly requested.

The exact enum name should be chosen during implementation. Acceptable candidates:

- `SourceFileClass`
- `ContentsFileClass`

Avoid names that imply container or analysis authority:

- not `format`
- not `container`
- not `codec`

---

## Primary-media policy

`primaryMedia` remains a separate profile-specific policy.

It must not become a canonical-track surface by accident.

If the current protocol behavior requires filter preservation, use a profile-specific field such as `mediaKinds`. That
field belongs to the `primaryMedia` policy variant only.

Do not activate new product UI for `primaryMedia` as part of the contents policy migration.

Do not add canonical track identity, track decisions, preparation readiness, waveform, artwork, CUE, stems, tags, notes,
crates, sleeves, routes, or cloud availability.

---

## Cursor identity

Contents cursor identity must bind the full policy discriminant.

Required invariants:

- `audioBrowse` cursors cannot be reused by `sourceFileInventory`;
- `audioBrowse` cursors cannot be reused by `primaryMedia`;
- `sourceFileInventory` cursors bind canonicalized `fileClasses`;
- `primaryMedia` cursors bind the profile-specific policy state;
- scope, recursion, and ordering position remain part of cursor identity;
- old cursors may become invalid in active development.

The cursor identity should reject mismatched policy rather than silently accepting the wrong page continuation.

---

## Naming cleanup in store code

The store-only naming lie was removed in this slice.

Its known variants are:

| Removed name              | Implemented name        | Meaning                                   |
| ------------------------- | ----------------------- | ----------------------------------------- |
| `LibraryTreeRowAdmission` | `SourceFileClassFilter` | Source-file class predicate helper        |
| `NavigationOnly`          | `NavigationOnly`        | Exclude all files from tree navigation    |
| `Performance`             | `PrimaryMedia`          | Audio + video source-file classes         |
| `PerformanceAndImages`    | `PrimaryMediaAndImages` | Audio + video + image source-file classes |

Why the type name is wrong:

Tree navigation is now navigation-only. The only tree-relevant value is the no-file predicate. The other variants are
contents/source-file class predicates, not tree admission decisions.

Why `Performance` is wrong:

The schema uses `has_primary_media_descendant`. The correct concept is primary media: audio and video. “Performance” is
ambiguous and does not match schema vocabulary.

This rename is local store vocabulary cleanup. It must not rename the schema column `source_files.media_class`. It must
not change protocol behavior. It must not change renderer behavior. It must not change tree behavior.

---

## Implementation order

1. Confirm compiler baseline.
2. Replace `ContentsReadPolicy { rowProfile, mediaClasses }` with a profile-specific policy union.
3. Bind contents cursor identity to the full policy discriminant.
4. Preserve `audioBrowse` parity with current `sourceFile + audio` behavior.
5. Preserve source-file inventory behavior under the new `sourceFileInventory` policy.
6. Preserve `primaryMedia` behavior without activating new product UI.
7. Rename local store vocabulary to `SourceFileClassFilter`, `PrimaryMedia`, and `PrimaryMediaAndImages`.
8. Regenerate contracts.
9. Update client, desktop adapter, renderer request construction, request keys, tests, and docs.
10. Run targeted validation before one final full verification pass.

---

## Implementation guardrails

Do not rename `source_files.media_class` in the schema. It is correct.

Do not preserve the active protocol-level `mediaClasses` field as a compatibility alias.

Do not keep a dual policy path.

Do not keep active `{ rowProfile, mediaClasses }` request support.

Do not add new profiles on top of the old global `mediaClasses` shape.

Do not add a new endpoint.

Do not add a new row payload type.

Do not add a new row union.

Do not change tree semantics.

Do not reintroduce file/audio rows into tree navigation.

Do not add renderer filtering or sorting.

Do not derive authoritative browse fields in the renderer.

Do not add extension, source label, source-location provenance, row version, container, codec, BPM, key, duration,
waveform, artwork, CUE, canonical track, duplicate resolution, analysis readiness, stems, tags, notes, crates, sleeves,
routes, or cloud/streaming availability.

Do not conflate the store rename with the schema vocabulary. The store rename is local concept cleanup. The schema
column remains correct.

---

## Validation requirements

The implementation slice must prove:

- `audioBrowse` request has no caller-supplied class/filter field;
- `sourceFileInventory` supports explicit file-class filtering;
- `primaryMedia` preserves current behavior through profile-specific policy;
- cursor identity rejects policy mismatch;
- audio browse parity with current source-file audio behavior is preserved;
- source-file inventory behavior is preserved;
- renderer request keys include the policy discriminant;
- warm snapshots do not cross policy boundaries;
- load-more preserves cursor and policy identity;
- generated contracts expose no active `mediaClasses` field;
- active docs no longer describe global `mediaClasses` as current contract.

---

## Decision ratified

The following are settled:

1. The protocol-level global `mediaClasses` field is removed by the active implementation.

2. Profile-specific contents policy is the active target contract.

3. `audioBrowse` implies audio and has no caller-supplied filter.

4. `sourceFileInventory` owns source-file class filtering.

5. `primaryMedia` owns any future primary-media-specific filter space and is not a canonical-track surface.

6. `source_files.media_class` is correct persisted inventory vocabulary and stays.

7. `LibraryTreeRowAdmission` is a misnamed store concept if it still exists. The correct concept is a source-file class
   filter used by contents/source-file query predicates.

8. `Performance` is a wrong variant name for audio + video. The schema's primary-media vocabulary is correct.

---

## What is not covered here

- Audio browse row V0 field expansion.
- Canonical track identity.
- Track identity projection design.
- Primary-media product activation.
- Preparation facets.
- Source-file inventory taxonomy changes.
- Renderer table redesign.
- Exclave boundary migration.
