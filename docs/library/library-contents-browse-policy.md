# Library Contents Browse Policy

## Decision

Default main library contents browse is audio-first and audio-only for V0.

The default renderer contents request uses the existing literal source-file row profile with:

| Field          | V0 default   |
| -------------- | ------------ |
| `rowProfile`   | `sourceFile` |
| `mediaClasses` | audio        |
| `recursion`    | recursive    |

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
