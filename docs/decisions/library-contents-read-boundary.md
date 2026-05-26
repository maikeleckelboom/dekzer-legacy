# Library Contents Read Boundary

_Canonical authority boundary document. Defines the single parameterized contents read boundary for contents-pane reads.
Governs all future contents query, pagination, and projection implementation._

---

## Decision

Dekzer uses one parameterized library contents read boundary for contents-pane reads.

A contents read is parameterized by:

- scope
- policy
- recursion
- limit
- cursor

Renderer product modes compile into a typed contents policy at the renderer edge. Backend, shared, protocol, and query
code does not branch on UI mode names.

---

## Motivation

The existing selectedContentsRead path is primary-media-only. It cannot represent image rows because its media class is
audio/video only and its rows carry primary-media/readiness/prep-shaped fields.

The temporary renderer visible-files proof demonstrated desired UI behavior, but renderer hierarchy cache is not the
final owner of selected scope contents.

---

## Inventory Findings

The following facts are established by the Phase 0 inventory at
`docs/reports/inventory-2026-05-26/05-main-report.md` and its supporting sub-reports:

1. **selectedContentsRead has zero non-pane consumers.**

   It is scoped to the library contents pane path and has no deck, waveform, playlist, prep, or readiness consumers.
   See `docs/reports/inventory-2026-05-26/01-selectedContents-call-sites.md`.

2. **sourceLocation is supported end to end.**

   Support exists through SQL `source_locations`, Rust protocol, service validation/mapping, store
   `SourceLocationPrefix`
   resolution, main IPC normalization, shared TS, renderer binding mapping, and tests.
   See `docs/reports/inventory-2026-05-26/02-sourceLocation-support.md`.

3. **Recursive source/directory/sourceLocation contents queries are feasible with the current schema.**

   Current selected contents already uses binary-collation descendant predicates. No schema migration is required for
   recursive sourceFile/profile media-class filtering.

4. **Cursor is typed but not implemented.**

   Cursor is optional string only, has no encoding, no scope fingerprint, no query/order binding, no mismatch
   validation, `nextCursor` is never produced, store refuses provided cursors, and renderer does not send cursors.
   See `docs/reports/inventory-2026-05-26/03-cursor-pagination.md`.

5. **Hierarchy read is not the final owner for selected scope contents.**

   It provides immediate tree children and `sourceFileVisibility`-aware rows. It does not own recursive selected
   contents, aggregated contents coverage, or primary-media summary joins.

---

## Contract Vocabulary

```ts
type ContentsRowProfile =
  | { readonly kind: 'sourceFile' }
  | { readonly kind: 'primaryMedia' }
```

**sourceFile:**

- literal source-file facts
- audio, video, image allowed
- no primaryMedia summary
- no prep, readiness, waveform, or stems joins
- used by image-inclusive visible browsing

**primaryMedia:**

- audio and video only
- may carry primary-media, library-asset, readiness, prep, waveform, and stems summary
- rejects image with `policyConflict`
- replaces the old selectedContentsRead behavior

The following terms are not used as shared or backend contract concepts:

- `performanceFile`
- `performanceAndImages`
- `Performance`
- `Performance + Images`
- `Tracks`
- `Tracks + Images`

---

## Target Request Shape

```ts
type ContentsReadRequest = {
  readonly scope: ContentsScope
  readonly policy: ContentsReadPolicy
  readonly recursion: ContentsRecursion
  readonly limit?: number
  readonly cursor?: string
}

type ContentsScope =
  | { readonly kind: 'source'; readonly sourceId: string }
  | { readonly kind: 'sourceLocation'; readonly sourceLocationId: string }
  | { readonly kind: 'directory'; readonly sourceId: string; readonly sourceDirectoryId: string }

type ContentsRecursion = 'immediate' | 'recursive'

type ContentsMediaClass = 'audio' | 'video' | 'image'

type ContentsReadPolicy = {
  readonly mediaClasses: readonly ContentsMediaClass[]
  readonly rowProfile: ContentsRowProfile
}
```

---

## Invariants

- One contents read boundary owns contents-pane reads.
- Renderer mode names do not cross into backend, shared, protocol, or query code.
- Renderer sends typed policy, never raw SQL.
- Backend and query code own media filtering.
- Renderer does not answer authoritative selected scope contents from loaded hierarchy cache.
- sourceFile profile may include audio, video, and image.
- sourceFile profile never carries primaryMedia summary.
- primaryMedia profile may include audio and video only.
- primaryMedia + image returns `policyConflict`.
- Image rows never carry primaryMedia summary.
- `mediaClasses` are deterministic arrays, not Set.
- `mediaClasses` are canonicalized in deterministic order: audio, video, image.
- Unsupported and `none` files are excluded from normal contents policy.
- Cursor remains first-page-only until real cursor identity is implemented.
- Provided cursor must not be silently ignored or treated as page one.

---

## Migration Decision

Use one implementation migration:

- introduce `contentsRead`
- migrate the pane
- remove `selectedContentsRead` public path before final report

Do not leave both `selectedContentsRead` and `contentsRead` as permanent public APIs.
Do not add aliases or compatibility wrappers.
Do not widen `SelectedContentsMediaClass` to image.
Define new `Contents*` contract names instead.

---

## Future Cursor Work

Real cursor support is a later slice:

- encode cursor with version
- bind to scope fingerprint
- bind to policy, media classes, and row profile
- bind to recursion and ordering
- carry last-row key tuple
- reject mismatches with `cursorInvalid`

This decision does not implement real cursor pagination.

---

## Supporting Inventory Reports

- `docs/reports/inventory-2026-05-26/05-main-report.md` — Phase 0 inventory summary
- `docs/reports/inventory-2026-05-26/01-selectedContents-call-sites.md` — call-site inventory
- `docs/reports/inventory-2026-05-26/02-sourceLocation-support.md` — sourceLocation end-to-end analysis
- `docs/reports/inventory-2026-05-26/03-cursor-pagination.md` — cursor/pagination state
- `docs/reports/inventory-2026-05-26/04-ipc-handlers-renderer-api.md` — IPC pipeline detail
