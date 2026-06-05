---
status: design-proposal
doctrine-version: 0.1
last-reviewed: 2026-06-06
owner: library-substrate-boundary
canonical-context:
  - library-contents-browse-policy
  - library-tree-selection-contents-contract
  - media-relevant-file-inventory-contract
scope:
  - audio-browse-row-v0-design
  - contents-read-model-proposal
---

# Audio Browse Row V0

## Status

Design proposal. Not implemented.

This document defines the first intentional audio-row read-model/product-row contract. It does not introduce a runtime
type, generated contract, store model, or renderer behavior.

## Owner

Library substrate and boundary own the read model. Renderer owns presentation of returned rows only.

## Problem

The current default contents browse is smooth enough for source-file audio browsing, but the row still feels like raw
file inventory. V0 needs a narrow product row that keeps current source-file truth, gives the contents table enough
stable fields for professional browsing, and avoids premature track identity or analysis claims.

## Current behavior this proposal builds on

The tree is navigation-only. Selecting a source, source location, or directory derives a contents scope. The default
contents read requests `rowProfile = sourceFile`, `mediaClasses = audio`, and `recursion = recursive`.

Current contents rows come from `ContentsFileRow` in the boundary contract and `StoreContentsFileRow` in the store read
model. Renderer projection maps those rows into `ContentRow` for table display. Warm contents snapshots, retained rows,
delayed pending state, and coalesced invalidation refresh are runtime/perception behavior, not row authority.

## V0 field classification

| Field                                | Classification | Source of authority                                                                                                                                     |
| ------------------------------------ | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Stable row id                        | current        | `ContentsFileRow.id`; renderer uses it as the content row key.                                                                                          |
| Source id                            | current        | `ContentsFileRow.sourceId` and `StoreContentsFileRow.source_id`.                                                                                        |
| Source file id                       | current        | `ContentsFileRow.sourceFileId` and `StoreContentsFileRow.source_file_id`.                                                                               |
| Parent directory id                  | current        | `ContentsFileRow.parentDirectoryId` and `StoreContentsFileRow.parent_directory_id`.                                                                     |
| Display label                        | current        | `ContentsFileRow.label`; renderer currently displays this as row label.                                                                                 |
| File name                            | current        | `ContentsFileRow.fileName` and `StoreContentsFileRow.file_name`.                                                                                        |
| Source-relative path                 | current        | `ContentsFileRow.relativePath` and `StoreContentsFileRow.relative_path`; used as current location provenance.                                           |
| Media class                          | current        | `ContentsFileRow.mediaClass`; default policy restricts this to audio.                                                                                   |
| File kind                            | current        | `ContentsFileRow.fileKind`; current audio rows use path-derived `audio`.                                                                                |
| Presence                             | current        | `ContentsFileRow.presence`; values are present, missing, or removed.                                                                                    |
| Availability state                   | current        | Optional `ContentsFileRow.availabilityState`; currently available when the backing read model has it.                                                   |
| Updated timestamp                    | current        | Optional `ContentsFileRow.updatedAtMs` mapped from `StoreContentsFileRow.updated_at`.                                                                   |
| Coverage state                       | current        | `ContentsResult.coverage`; row pages must not claim completeness without it.                                                                            |
| Pagination cursor                    | current        | `ContentsResult.nextCursor`; cursor identity is owned by the contents read.                                                                             |
| Normalized extension                 | proposed       | Needed for a stable table column if extension/container should be displayed separately from file name. Not a current boundary field.                    |
| Source display label                 | proposed       | Useful when a recursive source or source-location browse spans multiple visible source roots/locations. Not a current row field.                        |
| Source-location id/label             | proposed       | Useful provenance for multi-location source browsing. Current scope may be a source location, but rows do not carry per-row source-location provenance. |
| Row version                          | proposed       | Needed if future row identity needs versioned stale checks beyond `updatedAtMs` and cursor identity. Not a current source-file row field.               |
| Container/codec format               | proposed       | Only if backed by current or newly contracted file evidence. V0 must not infer codec from unsupported analysis surfaces.                                |
| Title, artist, album                 | rejected       | These appear only in optional primary-media summary today and are not reliable source-file browse-row fields for V0.                                    |
| Duration                             | rejected       | Not part of V0 unless a future reliable audio-row contract makes it current.                                                                            |
| BPM                                  | rejected       | Analysis fact, not V0 browse identity.                                                                                                                  |
| Musical key                          | rejected       | Analysis fact, not V0 browse identity.                                                                                                                  |
| Waveform                             | rejected       | Analysis artifact, not V0 browse identity.                                                                                                              |
| Artwork                              | rejected       | Role decision, not V0 browse identity.                                                                                                                  |
| CUE association                      | rejected       | Association/segmentation decision, not V0 browse identity.                                                                                              |
| Canonical track id                   | rejected       | Track identity decision, not V0 browse identity.                                                                                                        |
| Duplicate or same-song key           | rejected       | Identity resolution, not V0 browse identity.                                                                                                            |
| Analysis readiness                   | rejected       | Preparation has grouped detail surfaces; V0 does not collapse it into one field.                                                                        |
| Stems state                          | rejected       | Preparation/analysis domain, not V0 browse identity.                                                                                                    |
| Tags, notes, crates, sleeves, routes | rejected       | Organization domains, not V0 browse identity.                                                                                                           |
| Cloud or streaming availability      | rejected       | Not in the current row contract for V0.                                                                                                                 |

## Non-goals

V0 does not implement the read model, add generated contracts, add renderer filtering/sorting, or change contents
policy. It does not define canonical tracks, deck loading, CUE parsing, artwork inference, duplicate resolution,
analysis readiness, waveform display, stems, tags, notes, crates, sleeves, routes, or cloud/streaming availability.

V0 does not describe preparation as one flat object or one status. If preparation appears later, it must preserve the
separate fact, structure, artifact, work, outcome, and satisfaction concepts from the preparation detail surfaces.

## Authority boundaries

| Owner                         | Owns                                                                                                     |
| ----------------------------- | -------------------------------------------------------------------------------------------------------- |
| Store/read model              | Source-file identity, current row facts, ordering, cursor position, coverage.                            |
| Boundary protocol             | Typed app-safe fields and absence/presence of optional fields.                                           |
| Renderer                      | Table layout, icons, labels, contained scrolling, delayed pending presentation, retained-row perception. |
| Invalidation refresh planning | Coalescing current visible refreshes and clearing warm runtime snapshots.                                |

Renderer must not infer authoritative browse fields from tree rows, filesystem paths outside the boundary, local sort
state, or warm snapshots.

## Migration path from current contents source-file rows

1. Keep the current default contents policy as `sourceFile` + audio + recursive.
2. Define the future audio-row read model as a boundary/store projection over current source-file rows.
3. Start V0 with current fields that already cross the boundary.
4. Add proposed provenance or extension fields only when their store authority is explicit.
5. Preserve cursor identity and coverage semantics so old and new pages cannot be mixed silently.
6. Switch renderer projection after the new read model exists and tests prove parity for current source-file audio rows.

## Rejection cases

A future implementation fails this proposal if it:

- adds audio rows back into tree navigation;
- introduces renderer-side filtering or sorting as browse authority;
- treats retained rows or warm snapshots as durable truth;
- exposes optional primary-media summary fields as reliable V0 browse-row fields;
- adds BPM, musical key, duration, artwork, waveform, CUE association, canonical track, analysis readiness, stems, tags,
  notes, crates, sleeves, routes, or cloud/streaming availability to V0;
- collapses preparation into one status;
- changes contents policy or media classes to make the proposal pass.

## Test and acceptance plan for future implementation

Future implementation should prove:

- audio rows match current source-file audio results for the same source, source location, directory, recursion, limit,
  and cursor;
- row ids remain stable across refreshes when source-file identity is unchanged;
- cursor paging produces no duplicates or gaps;
- source removal clears accepted selection and contents after removal is accepted;
- incomplete, blocked, unavailable, and missing-location states do not render as authoritative empty results;
- warm prefetch never widens policy or authorizes stale data;
- contents table rows remain browsable inside the library panel without app-level overflow.

## Suggested implementation order, no code

1. Ratify this proposal against current `ContentsFileRow` and store fields.
2. Decide whether normalized extension, source display label, source-location provenance, row version, or container
   format are required for V0 acceptance.
3. Add store/read-model tests for any proposed fields before exposing them.
4. Add boundary contract fields only after store authority exists.
5. Add renderer projection for the new audio row surface.
6. Keep current source-file audio browse as the fallback until parity tests pass.

## Open gaps, if current fields are insufficient

Current rows do not expose a normalized extension field, source display label, source-location provenance per row, a
source-file row version, or a reliable audio container/codec field. If the contents table needs those as product
columns, the future implementation must add explicit read-model authority instead of deriving them as hidden renderer
truth.
