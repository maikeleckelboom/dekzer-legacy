# Library Contents Browse Policy

## Decision

Default main library contents browse is audio-first and audio-only for V0.

The default renderer contents request uses the backend-owned audio browse row profile with:

| Field          | V0 default    |
| -------------- | ------------- |
| `rowProfile`   | `audioBrowse` |
| `mediaClasses` | audio         |
| `recursion`    | recursive     |

The renderer may choose this policy, but the boundary and backend own what rows are returned. The projection and table
render returned rows and must not hide disallowed rows with renderer-side filtering or sorting.

## Boundaries

Raw source-file inventory remains a backend capability for future diagnostic or details modes. Explicit non-default
source-file policies may still request image, video, and admitted unsupported source-file rows where the backend supports
them.

Video is not part of the V0 default browse policy. Artwork, CUE sheets, and metadata companion files are not default
playable rows. They may exist in durable source inventory and future explicit surfaces, but they are not returned by the
default main contents browse.

This decision does not parse CUE sheets, infer artwork or CUE associations, create canonical track identity, or change
durable source inventory semantics.

## Current Row Contract

Default contents rows use the audio browse profile and reuse the existing contents file-row payload shape. They are
suitable for an audio-first file browse table, but they are not canonical tracks and do not decide same-song identity,
analysis readiness, deck load readiness, CUE association, or artwork role.

The contents read and store own ordering, media-class admission, row profile admission, recursion, cursor identity, and
coverage. Renderer projection may choose labels, icons, state rows, and table layout for returned rows only.

## Perception Contract

Retained contents rows are perception continuity, not data authority. Pending state may be delayed to avoid spinner
flash on fast reads. Warm contents prefetch is a short-lived runtime optimization for matching requests; it does not
authorize rows and must be cleared by relevant invalidation or generation changes.

The contents table must remain browsable inside the library panel. This is an acceptance rule for the product surface,
not a requirement for a particular CSS implementation.
