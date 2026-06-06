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

The following facts are established by the Phase 0 inventory:

1. **selectedContentsRead has zero non-pane consumers.**

   It is scoped to the library contents pane path and has no deck, waveform, playlist, prep, or readiness consumers.

2. **sourceLocation is supported end to end.**

   Support exists through SQL `source_locations`, Rust protocol, service validation/mapping, store
   `SourceLocationPrefix`
   resolution, main IPC normalization, shared TS, renderer binding mapping, and tests.

3. **Recursive source/directory/sourceLocation contents queries are feasible with the current schema.**

   Current contents reads already use binary-collation descendant predicates. No schema migration is required for
   recursive sourceFile/profile media-class filtering.

4. **Cursor pagination is implemented for contents reads.**

   Cursor identity is encoded (base64url-encoded JSON, version 1) and validated. Cursor binds to scope, policy
   (media classes and row profile), recursion, and ordering; it carries a last-row position tuple specific to each
   row profile (sourceFile: `relative_path_key` + `source_file_id`; primaryMedia: `availability_priority`,
   `title_key`, `artist_key`, `album_key`, `relative_path_key`, `source_file_id`). The store validates cursor
   identity against the current request. Mismatched or un-decodable cursors return `CursorInvalid` with no
   `nextCursor`. Successful reads return `nextCursor` when more rows exist. The renderer sends cursors through the
   `loadContentsPage` action and accumulates pages in the contents boundary. Contents pagination is distinct from
   tree load-more (`loadChildren`).

5. **Hierarchy read is not the final owner for selected scope contents.**

   It provides immediate children admitted by the product/boundary surface. It does not own recursive selected
   contents, aggregated contents coverage, or primary-media summary joins.

   **Historical note:** `sourceFileVisibility` was implementation debt that has been removed from renderer-facing
   contracts. Library tree row admission is now a product/boundary surface concern, not a renderer-facing parameter.

---

## Contract Vocabulary

```ts
type ContentsRowProfile =
  | { readonly kind: 'sourceFile' }
  | { readonly kind: 'primaryMedia' }
  | { readonly kind: 'audioBrowse' }
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

**audioBrowse:**

- audio only
- reuses the current contents file-row payload shape
- returns V0 rows with source-file audio parity for equivalent scope, recursion, limit, and cursor
- rejects non-audio media classes with `policyConflict`
- cursor identity is distinct from `sourceFile` and `primaryMedia`

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

type ContentsMediaClass = 'audio' | 'video' | 'image' | 'unsupported'
type ContentsFileKind =
  | 'audio'
  | 'video'
  | 'image'
  | 'cueSheet'
  | 'logDoc'
  | 'textDoc'
  | 'archive'
  | 'other'
  | 'unknown'

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
- sourceFile profile may include audio, video, image, and admitted unsupported companion rows.
- sourceFile profile never carries primaryMedia summary.
- primaryMedia profile may include audio and video only.
- primaryMedia + image or unsupported returns `policyConflict`.
- audioBrowse profile may include audio only.
- audioBrowse + video, image, or unsupported returns `policyConflict`.
- audioBrowse reuses the contents file-row payload shape and does not introduce a dedicated row type.
- Image rows never carry primaryMedia summary.
- `mediaClasses` are deterministic arrays, not Set.
- `mediaClasses` are canonicalized in deterministic order: audio, video, image, unsupported.
- Normal media-relevant source-file inventory admits unsupported rows only when `fileKind = cueSheet`.
- Unsupported docs, archives, binaries, unknown files, and `none` files are excluded from normal contents policy.
- See `docs/library/media-relevant-file-inventory-contract.md` for the default inventory policy.
- Cursor pagination is implemented; `nextCursor` is produced when more rows exist.
- Provided cursor must not be silently ignored or treated as page one.
- Invalid or mismatched cursor returns `cursorInvalid` with no `nextCursor`.

---

## Migration Decision

One implementation migration was used:

- `contentsRead` was introduced
- the pane was migrated
- `selectedContentsRead` was removed from the implementation

Do not reintroduce `selectedContentsRead` as a public API.
Do not add aliases or compatibility wrappers.
Do not widen `SelectedContentsMediaClass` to image.
Define new `Contents*` contract names instead.

---

## Cursor Pagination

Cursor pagination is implemented for contents reads:

- Cursor is encoded (base64url-encoded JSON, version 1) with scope, policy, recursion, row profile, media classes,
  and last-row ordering position.
- Cursor identity is validated against the current request; mismatches return `cursorInvalid`.
- `nextCursor` is produced when more rows exist beyond the limit.
- The renderer sends cursors through `loadContentsPage` and accumulates pages in the contents boundary.
- Contents pagination (`loadContentsPage`) is distinct from tree load-more (`loadChildren`).

Future cursor work, if any, should extend cursor identity fields rather than replace the encoding or validation
mechanism.

This decision does not defer cursor pagination.
