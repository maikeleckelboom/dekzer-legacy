# Phase 0 Inventory Report: Selected Contents Read Boundary Before Parameterization

**Date**: 2026-05-26  
**HEAD**: `c7f505c docs(library): align library decision docs`  
**Working tree**: Clean

---

## 1. Current HEAD and git status

- **HEAD**: `c7f505c docs(library): align library decision docs`
- **Working tree**: Clean (no uncommitted changes)
- **Branch**: Not explicitly checked, but no local changes present

---

## 2. selectedContentsRead Call-Site Inventory

**Verdict: SAFE for single-pass migration. Zero non-pane usages.**

Every single usage of `selectedContents` across the entire codebase is strictly scoped to the **library contents pane** feature. The chain is:

```
Vue panel → boundary composable → preload bridge → IPC channel → main handler → boundary client → Rust service → SQLite store
```

**Zero references** found in:
- `apps/desktop/src/renderer/library/deck/`
- `apps/desktop/src/renderer/library/waveform/`
- `apps/desktop/src/renderer/library/playlist/`
- `apps/desktop/src/renderer/library/prep/`

No deck loading, waveform prefetch, playlist behavior, or prep/readiness usage depends on `selectedContentsRead`.

The only consumer is `panel.vue:54` (via `useSelectedContentsRead`) which feeds the contents pane projection at `contents/projection.ts:124`.

**Complete file listing**: See `docs/reports/inventory-2026-05-26/01-selectedContents-call-sites.md`.

---

## 3. SourceLocation Support Finding

**Verdict: SUPPORTED END TO END.**

Every layer covers `sourceLocation` scope completely:

| Layer | Coverage |
|-------|----------|
| SQL schema | `source_locations` table + indexes since baseline migration |
| Rust protocol | `SelectedContentsScope::SourceLocation{source_location_id: i64}` enum variant |
| Rust service | `validate_selected_contents_scope` validates positive i64; `store_selected_contents_scope` maps it |
| Rust store | `resolve_scope` calls `load_accepted_source_location` (resolves ID to `(source_id, relative_path)`), produces `SourceLocationPrefix`, queries with descendant predicate |
| Main IPC | `normalizeScope` at `apps/desktop/src/main/librarySelectedContents/read.ts:151-154` validates `kind === 'sourceLocation'` with opaque ID regex |
| Shared TS | `SelectedContentsScope` union includes `{ kind: 'sourceLocation', sourceLocationId: string }` |
| Renderer boundary | `selectedContentsScopeForBinding` at `apps/desktop/src/renderer/library/boundary/selectedContentsRead.ts:189-191` maps source target to sourceLocation scope |
| Tests | 15+ Rust test functions in `selected_contents.rs:1745+` for source location scopes |

No partial gaps. No typed-but-unimplemented variants.

**Complete file listing**: See `docs/reports/inventory-2026-05-26/02-sourceLocation-support.md`.

---

## 4. Current Selected Contents Contract Summary

### Request (`SelectedContentsRequest`)
```
{ scope?: SelectedContentsScope, limit?: number (default 100, max 200), cursor?: string }
```

### Scope variants
- `source` (sourceId)
- `sourceLocation` (sourceLocationId) -- fully supported
- `directory` (sourceId, sourceDirectoryId)

### Result (`SelectedContentsReadResult`)
Discriminated union: `ready | hostUnavailable | noTarget | notFound | invalidRequest | readFailed`

### Row (`SelectedContentsRow`)
- **Core identity**: `stableId`, `label`, `origin` (`libraryAsset | sourceFile`), `libraryAssetId?`, `rowVersion?`, `primarySourceFileId?`, `scopedSourceFileId`, `sourceId`, `relativePath`, `fileName`
- **Media class**: `'audio' | 'video'` -- **NO image** (limitation)
- **Availability**: `available | unavailable | degraded`
- **Primary media data** (always carried): `title?`, `artist?`, `album?`, `durationMs?`, `musicalKey?`, `tempoBpm?`
- **Prep/readiness** (always carried): `waveformQualityCurrent?`, `waveformQualityTarget?`, `stemsStateSummary?`, `prepReadinessSummary`
- **Timestamp**: `updatedAtMs`

### Coverage
States: `complete | pending | scanning | blocked | failed | sourceUnavailable | locationMissing | incomplete`

### Key limitations for parameterization
1. Images NOT representable (enum has no `'image'`)
2. Always carries primaryMedia fields (even for sourceFile rows -- filled with `'underprepared'` defaults)
3. Has `cursor`/`limit` typed but cursor is stubbed (never works)
4. No `mediaClass` filter on request -- backend hardcodes audio/video only
5. No row-profile control -- always returns full primaryMedia data

---

## 5. Current Backend Query Shape

### Scope resolution (`resolve_scope` at `selected_contents.rs:220`)
- `Source{source_id}` → auto-detects: `WholeSource` (if no accepted locations) or `AcceptedSourceLocations`
- `SourceLocation{source_location_id}` → `SourceLocationPrefix{source_id, relative_path}` (prefix = location's relative_path from `source_locations` table)
- `Directory{source_id, source_directory_id}` → `Prefix{source_id, relative_path}` (prefix = directory tree path) or `MissingLocation`

### SQL query structure (`selected_rows_sql` at `:1030`)
1. **CTE `scope_files`**: `source_files` WHERE `presence_state = 'present'` AND `media_class IN ('audio','video')` (hardcoded `SourceFileVisibility::Performance`) AND scope predicate
2. **CTE `promoted_scope`**: JOIN chain `scope_files → SourceSegmentSets → SourceSegments → LibraryAssetAttachments → LibraryBrowserRows` with `ROW_NUMBER()` deduplication by `library_asset_id`
3. **CTE `promoted`**: filter to `attachment_rank = 1`
4. **CTE `source_file_rows`**: scope_files NOT in promoted_scope. Filled with `availability_state = 'available'`, `prep_readiness_summary = 'underprepared'`, all other primaryMedia fields NULL
5. **Final SELECT**: `promoted UNION ALL source_file_rows` ORDER BY availability priority, then lower(title/artist/album/relative_path), then `scoped_source_file_id`. `LIMIT ?`.

### Recursive behavior
Uses `source_file_descendant_predicate`:
```sql
(relative_path COLLATE BINARY >= prefix || '/'
 AND relative_path COLLATE BINARY < prefix || char(48))
```
This prefix-range scan uses the binary collation index on `source_files`. **No schema migration needed** for recursive queries with any media class filter.

### Media filtering
Hardcoded to `Performance` (audio/video only) via `source_file_visibility_predicate_sql_for_column(SourceFileVisibility::Performance, "sf.media_class")`.

### Coverage computation
Computed from source scan phase, directory enrollment (pending/scanning/blocked/failed counts), and per-location coverage classification.

### Pagination
`LIMIT` only (no cursor). If cursor is `Some`, immediately returns `Failed` with stub message.

---

## 6. Cursor/Pagination Finding

**Verdict: Cursor is a stub. Adding `cursorInvalid` state is small. Implementing full cursor is medium effort.**

| Aspect | Status |
|--------|--------|
| Cursor encoding | **None** -- `Option<String>` everywhere, no structure |
| Scope binding | **None** |
| Order binding | **None** |
| Mismatch detection | **None** -- `cursorInvalid`/`stale_cursor` not in any code enum |
| Store behavior | Stub-refuses: `if cursor.is_some() { return Failed }` |
| `nextCursor` | Always `None` |
| Renderer usage | Never sends cursor |
| Tests | Zero cursor tests |

**Smallest safe cursor plan**:
1. Add `cursorInvalid` as a typed error state (TS + Rust protocol + service) -- placeholder only
2. Keep store stub-refusing (safe default)
3. When ready: encode as `{version: 1, scopeFingerprint, order, last}`, validate fingerprint on every call
4. Existing tests pass unchanged

**Complete file listing**: See `docs/reports/inventory-2026-05-26/03-cursor-pagination.md`.

---

## 7. Schema/Index Feasibility for Recursive Visible-File Query

**Verdict: Feasible without migration.**

The `source_file_descendant_predicate` (COLLATE BINARY prefix range) already exists and is used for both `Prefix` and `SourceLocationPrefix` resolved scopes. Test names confirm proper index usage:
- `selected_contents_directory_prefix_uses_binary_collation_index`
- `selected_contents_whole_source_uses_index_not_table_scan`
- `selected_contents_accepted_locations_avoids_source_files_table_scan`

Adding `'image'` to the predicate just changes the `IN` list from `('audio','video')` to `('audio','video','image')` -- no schema change, same index usage.

---

## 8. Hierarchy-Read Relationship and Why Renderer-Cache Derivation Is Not Final Owner

**Hierarchy read provides**:
- `FileMediaClass = 'audio' | 'video' | 'image' | 'unsupported' | 'none'` (supports images)
- `SourceFileVisibility = 'performance' | 'performanceAndImages'` (UI toggle names)
- Offset/limit pagination on literal tree nodes
- `has_primary_media_descendant` / `has_image_media_descendant` directory hints

**What hierarchy read does NOT provide**:
- Recursive scope queries (only immediate children)
- Backend coverage computation (scan state aggregation)
- `LibraryBrowserRows` joins (no title, artist, album, durationMs, waveform, stems, prepReadinessSummary)
- Authoritative selected-scope contents

**Current renderer fallback** (`projectSourceVisibleFiles`/`projectDirectoryVisibleFiles` at `contents/projection.ts:640-742`):
- Only used when `sourceFileVisibility === 'performanceAndImages'`
- Walks loaded hierarchy cache to find visible file rows
- Is explicitly temporary -- the design target is a single backend contents read parameterized by policy

**Why renderer should not permanently derive contents from hierarchy cache**:
1. Hierarchy is literal tree only (immediate children) -- cannot answer recursive scope queries
2. Hierarchy rows don't carry primaryMedia data (LibraryBrowserRows fields)
3. Hierarchy coverage is per-directory, not aggregated scope coverage
4. Two code paths (hierarchy walk vs selectedContents read) create divergence risk

---

## 9. Recommended Migration Option

**Option B: Introduce `contentsRead`, migrate pane, then remove `selectedContentsRead`.**

### Benefits
- Incremental: new boundary lives alongside old until renderer switches over
- Backwards-compatible during transition (no other consumers)
- Can test both paths during migration
- Explicit removal step prevents two permanent paths

### Risks
- If old path is not removed after migration, two permanent paths exist
- Mitigated by: explicit deprecation comment + removal in same or very next PR

### Files likely touched (all layers)
- `apps/desktop/src/shared/libraryContents/read.ts` -- new shared types
- `apps/desktop/src/shared/rendererApi.ts` -- new API method
- `apps/desktop/src/preload/rendererApi.ts` -- new bridge method
- `apps/desktop/src/main/libraryContents/read.ts` -- new IPC handler
- `apps/desktop/src/main/index.ts` -- register new handler
- `apps/desktop/src/renderer/library/boundary/contentsRead.ts` -- new composable
- `apps/desktop/src/renderer/library/contents/projection.ts` -- switch to new source
- `apps/desktop/src/renderer/library/panel.vue` -- use new composable
- `packages/library-boundary-contract/index.ts` -- ts_rs generated types
- `crates/library-boundary-protocol/src/commands/snapshot_reads.rs` -- new request/reply
- `crates/library-boundary-service/src/service.rs` -- new service method
- `crates/library-boundary-service/src/snapshot_read_protocol.rs` -- new mappings
- `crates/library-store-sqlite/src/read_models/contents.rs` -- new read model
- `crates/library-store-sqlite/src/store/contents_reads.rs` -- new store facade
- `crates/library-store-sqlite/src/read_models/mod.rs` -- new module
- `crates/library-store-sqlite/src/store/mod.rs` -- new module
- `crates/library-store-sqlite/src/lib.rs` -- new re-exports

### Option C rejected
Creates two permanent contents read boundaries (`selectedContentsRead` for primaryMedia + `contentsRead` for sourceFile), violating the "one contents read boundary" target.

### Option A rejected (for initial pass)
Coordinated rename across all layers (TS + Rust) in one pass is riskier than Option B's add-then-remove approach.

---

## 10. Recommended Contract Shape

Using the sourceFile/primaryMedia vocabulary as specified:

```ts
// Channel
export const contentsReadChannels = {
  read: 'desktop:library-contents:read'
} as const

// Error codes
type ContentsReadErrorState = 'hostUnavailable' | 'noTarget' | 'notFound' | 'invalidRequest' | 'readFailed' | 'policyConflict' | 'cursorInvalid'

// Request
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

type ContentsRowProfile =
  | { readonly kind: 'sourceFile' }
  | { readonly kind: 'primaryMedia' }

// Result row
type ContentsFileRow = {
  readonly id: string
  readonly sourceId: string
  readonly sourceFileId: string
  readonly parentDirectoryId?: string
  readonly label: string
  readonly relativePath?: string
  readonly mediaClass: ContentsMediaClass
  readonly presence: 'present' | 'missing' | 'removed'
  readonly rowVersion: string
  readonly primaryMedia?: PrimaryMediaSummary  // Only for primaryMedia profile
}

// Result
type ContentsReadResult = {
  readonly state: 'ready'
  readonly result: ContentsResult
} | {
  readonly state: ContentsReadErrorState
  readonly error: { readonly code: string; readonly message: string }
}
```

### Required invariants
1. `sourceFile` profile: rows may be audio/video/image; `primaryMedia` field is **never** present
2. `primaryMedia` profile: rows may be audio/video only; `primaryMedia` field is always present for library-asset rows
3. `primaryMedia` + `image` in `mediaClasses`: returns `policyConflict` error (before query)
4. Image rows never carry `primaryMedia` data
5. Renderer mode names (`'performance'`, `'performanceAndImages'`) never cross the shared boundary
6. `mediaClasses` canonicalized in deterministic order: `['audio', 'video', 'image']`
7. Invalid policy combinations rejected with explicit error, not silently coerced
8. Cursor mismatch returns `cursorInvalid` (placeholder state)
9. SQL filtering belongs in backend; renderer never sends raw SQL
10. Renderer does not answer authoritative selected scope contents from tree cache

---

## 11. Risks/Blockers

| Risk | Severity | Status |
|------|----------|--------|
| selectedContentsRead has non-pane call sites | **HIGH** | **None found** -- safe |
| sourceLocation partial support | **HIGH** | **Fully supported** -- safe |
| Cursor requires larger subsystem | **MEDIUM** | **Stub acceptable** -- deferred |
| Recursive query needs schema migration | **MEDIUM** | **No migration needed** -- descendant predicate exists |
| Two permanent read paths after migration | **MEDIUM** | **Avoided** -- Option B + explicit removal |
| Replacing in one pass risky | **MEDIUM** | **Mitigated** -- Option B |
| Images not in current Rust protocol enum | **MEDIUM** | **Requires protocol change** -- extend `SelectedContentsMediaClass` |
| Backend ownership ambiguity | **LOW** | **Clear** -- single chain store→service→protocol→IPC |

**No stop-rule blockers found.** All stop conditions (from the task spec) are resolved.

---

## 12. Exact Next Implementation Prompt Outline

```
Phase 1 -- Protocol + Contract (Rust first, then generated TS)
- Add ContentsMediaClass with 'audio'|'video'|'image' to library-boundary-protocol
- Add ContentsReadRequest / ContentsReadPolicy / ContentsRowProfile to protocol
- Add ContentsReadReply / ContentsFileRow / PrimaryMediaSummary to protocol
- Add policyConflict + cursorInvalid error variants
- Add cursor: Option<String> + nextCursor fields (stub as today)
- Regenerate TS contract via ts_rs

Phase 2 -- Rust store
- New read_models/contents.rs (copied from selected_contents.rs as base)
- Parameterize source_file_visibility_predicate_sql_for_column with policy.mediaClasses
- Add rowProfile filtering: omit primaryMedia fields for sourceFile profile
- Add policyConflict validation: primaryMedia + image rejects before query
- Add cursor stub (returns Failed as today)
- Add canonical mediaClasses ordering
- Copy relevant tests from selected_contents tests; extend with policy tests

Phase 3 -- Rust service
- Add read_contents method to BoundaryService
- Add validate_contents_policy with profile/mediaClass conflict check
- Add store_contents_policy mapping
- Add map_read_contents_reply
- Register in dispatch

Phase 4 -- TS shared + main IPC
- New shared/libraryContents/read.ts: contentsReadChannels + full type set
- New main/libraryContents/read.ts: IPC handler (modeled on selectedContents read.ts)
- Add contents to RendererApi type
- Add contents to preload bridge
- Register new IPC channel in main/index.ts

Phase 5 -- Renderer
- New boundary/contentsRead.ts: composable (modeled on selectedContentsRead.ts)
- Update contents/projection.ts: use new contentsRead for ALL modes
- Update panel.vue: use new composable, map UI visibility to policy
- Remove hierarchy-cache walk fallback (projectSourceVisibleFiles/projectDirectoryVisibleFiles)

Phase 6 -- Cleanup (same or next PR)
- Remove selectedContentsRead composable, IPC handler, shared types
- Remove selected_contents.rs read model
- Remove selectedContents from RendererApi and preload
- Update all test imports

Phase 7 -- Cursor (later, separate feature)
- Implement cursor encoding, scope fingerprint, order binding
- Implement keyset pagination in store
- Implement cursorInvalid detection
- Add cursor tests
```

---

## 13. Suggested Commit Message

```
feat(library): introduce parameterized contents read boundary

Replace the fixed primary-media-only selectedContentsRead with a
parameterized contentsRead that supports:

- scope (source, sourceLocation, directory)
- policy (mediaClasses + rowProfile: sourceFile | primaryMedia)
- recursion (immediate, recursive)
- limit and cursor pagination (cursor stubbed, deferred)

The renderer "Performance" and "Performance + Images" modes now both
use the single backend contents read, with policy controlling whether
audio/video/image are returned and whether primaryMedia summary fields
are included. Source-file profile rows never carry primaryMedia data.
primaryMedia + image policy combinations return policyConflict.

Remove the renderer hierarchy-cache walk fallback as the sole source of
truth for visible files. Remove selectedContentsRead in the same pass
since it has zero non-pane consumers and is fully replaced.
```

---

## Appendix: Supporting Sub-Agent Reports

1. [01-selectedContents-call-sites.md](./01-selectedContents-call-sites.md) -- Complete call-site inventory
2. [02-sourceLocation-support.md](./02-sourceLocation-support.md) -- Full sourceLocation end-to-end analysis
3. [03-cursor-pagination.md](./03-cursor-pagination.md) -- Cursor/pagination state and plan
4. [04-ipc-handlers-renderer-api.md](./04-ipc-handlers-renderer-api.md) -- IPC pipeline detail
