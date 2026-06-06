# Library Contents Browse Policy

## Decision

Default main library contents browse is audio-first and audio-only for V0.

The default renderer contents request uses the backend-owned policy:

```ts
{
  policy: { kind: 'audioBrowse' },
  recursion: 'recursive'
}
```

`audioBrowse` implies audio and accepts no caller-supplied class filter.

The renderer may choose this policy, but the boundary and backend own what rows are returned. The projection and table
render returned rows and must not hide disallowed rows with renderer-side filtering or sorting.

The persisted source-file vocabulary is separate from the policy discriminant:

- `file_kind` is the detailed detected taxonomy.
- `file_class` is the coarse inventory classification with values `audio`, `video`, `image`, `unsupported`, and `none`.
- Contents source-file rows expose the coarse fact as `fileClass`.
- `none` is a persisted/internal classification and is not requestable through `sourceFileInventory.fileClasses`.
- `unsupported` in `sourceFileInventory` admits only cue-sheet files, not all unsupported files.

## Boundaries

Raw source-file inventory remains available through `sourceFileInventory.fileClasses`. Explicit non-default inventory
reads may request `audio`, `video`, `image`, and `unsupported` as requestable browse classes. Inserting `none` as a
`ContentsFileClass` requires a separate decision.

`primaryMedia.mediaKinds` is separate primary-media vocabulary and is not an alias for source-file `fileClasses`.

Video is not part of the V0 default browse policy. Artwork, CUE sheets, and metadata companion files are not default
playable rows. They may exist in durable source inventory and future explicit surfaces, but they are not returned by the
default main contents browse.

This decision does not parse CUE sheets, infer artwork or CUE associations, create canonical track identity, or change
durable source inventory semantics.

## Current Row Contract

Default contents rows use the audio browse profile and reuse the existing contents file-row payload shape. They are
suitable for an audio-first file browse table, but they are not canonical tracks and do not decide same-song identity,
analysis readiness, deck load readiness, CUE association, or artwork role.

The contents read and store own ordering, profile-specific admission, recursion, cursor identity, and coverage.
Renderer projection may choose labels, icons, state rows, and table layout for returned rows only.

## Perception Contract

Retained contents rows are perception continuity, not data authority. Pending state may be delayed to avoid spinner
flash on fast reads. Warm contents prefetch is a short-lived runtime optimization for matching requests; it does not
authorize rows and must be cleared by relevant invalidation or generation changes.

The contents table must remain browsable inside the library panel. This is an acceptance rule for the product surface,
not a requirement for a particular CSS implementation.
