# Sub-Agent 3: Cursor / Pagination Inventory

**Date**: 2026-05-26
**Scope**: Entire codebase at `C:\dev\dekzer`
**Search terms**: `cursor`, `Cursor`, `nextCursor`, `next_cursor`, `pagination`, offset/limit patterns

---

## Verdict: Cursor encoding is NOT implemented. Cursor is a stub.

The codebase has a **keyset-cursor pagination design** for the `ReadSelectedContents` API, but **the cursor-pagination
feature is NOT yet implemented in the store** -- it is a **stub**. When a cursor is passed, the store immediately
returns a `Failed` state with the message `"Selected contents cursor paging is not available in this first slice."`

---

## A. PRIMARY CURSOR/PAGINATION SYSTEM: Selected Contents Keyset Cursor

### A1. Design Document (what cursor pagination *should* do)

| File                                                 | Lines   | Context                                                                                                                                                                                                                                                                                                                                                                     |
|------------------------------------------------------|---------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `docs/decisions/recursive-selected-contents-rule.md` | 441-470 | **Pagination cursor contract** -- defines keyset cursors, the `SelectedContentsCursor` type (v1, scopeFingerprint, order, last row's key tuple), states that cursor is valid only for the scope fingerprint and ordering that produced it. Defines that stale/mismatched cursors must return `source_unavailable`, `location_missing`, `blocked`, or `stale_cursor` errors. |
| `docs/decisions/recursive-selected-contents-rule.md` | 489     | Defines `stale_cursor` state: "pagination cursor is no longer valid."                                                                                                                                                                                                                                                                                                       |
| `docs/decisions/recursive-selected-contents-rule.md` | 540-541 | "The contents table uses its own cursor, page size, order key, and virtual scroll state. Its pagination must not depend on tree scroll position or tree expansion state."                                                                                                                                                                                                   |
| `docs/decisions/recursive-selected-contents-rule.md` | 583     | "recursive contents pagination uses the keyset cursor contract defined above"                                                                                                                                                                                                                                                                                               |
| `docs/decisions/recursive-selected-contents-rule.md` | 596     | Test requirement: "total row count and cursor pagination limits"                                                                                                                                                                                                                                                                                                            |
| `docs/decisions/recursive-selected-contents-rule.md` | 166-170 | `stale_cursor` returned for `eject_requested` / `eject_pending` mount states                                                                                                                                                                                                                                                                                                |

### A2. Protocol / Contract Layer (type definitions)

| File                                                              | Lines   | Context                                                                                                                                                                  |
|-------------------------------------------------------------------|---------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `crates/library-boundary-protocol/src/commands/snapshot_reads.rs` | 285-291 | `ReadSelectedContentsRequest { scope, limit, cursor: Option<String> }` -- **cursor is just typed as `Option<String>`**, no encoding/decoding, no validation of structure |
| `crates/library-boundary-protocol/src/commands/snapshot_reads.rs` | 352-363 | `SelectedContentsResult { state, scope, rows, coverage, next_cursor: Option<String>, detail }` -- **next_cursor is just typed as `Option<String>`**, no encoding         |
| `crates/library-boundary-protocol/src/commands/snapshot_reads.rs` | 379+    | `SelectedContentsState` enum -- does **NOT** include `stale_cursor` or `cursorInvalid`                                                                                   |

**Verdict: Cursor encoding is NOT implemented -- cursor is just a raw `Option<String>` at the protocol level. No
structure, no version, no scope fingerprint.**

### A3. Contract / TypeScript Boundary

| File                                                               | Lines     | Context                                                                         |
|--------------------------------------------------------------------|-----------|---------------------------------------------------------------------------------|
| `packages/library-boundary-contract/index.ts`                      | 55        | `ReadSelectedContentsRequest = { scope, limit, cursor?: string }`               |
| `packages/library-boundary-contract/index.ts`                      | 143       | `SelectedContentsResult = { ..., nextCursor?: string, ... }`                    |
| `packages/library-boundary-contract/boundary-contract.schema.json` | 2172-2176 | `cursor` field in `ReadSelectedContentsRequest`: `{ type: ["string", "null"] }` |
| `packages/library-boundary-contract/boundary-contract.schema.json` | 2370-2374 | `nextCursor` field in `SelectedContentsResult`: `{ type: ["string", "null"] }`  |

### A4. Main Process (Electron main) -- Validation & Passthrough

| File                                                      | Lines   | Context                                                                                                                                                     |
|-----------------------------------------------------------|---------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `apps/desktop/src/main/librarySelectedContents/read.ts`   | 90      | `NormalizedRequest { scope, limit, cursor?: string }`                                                                                                       |
| `apps/desktop/src/main/librarySelectedContents/read.ts`   | 61      | Passes cursor through to host: `...(cursor === undefined ? {} : { cursor })`                                                                                |
| `apps/desktop/src/main/librarySelectedContents/read.ts`   | 114     | `const cursor = normalizeCursor(request.cursor)`                                                                                                            |
| `apps/desktop/src/main/librarySelectedContents/read.ts`   | 198-211 | `normalizeCursor()` function -- validates cursor is a non-empty string. Returns `invalidRequest` error if invalid: `'Selected contents cursor is invalid.'` |
| `apps/desktop/src/main/librarySelectedContents/read.ts`   | 282     | Maps `result.nextCursor` through to the output                                                                                                              |
| `apps/desktop/src/shared/librarySelectedContents/read.ts` | 48      | `SelectedContentsRequest { cursor?: string }`                                                                                                               |
| `apps/desktop/src/shared/librarySelectedContents/read.ts` | 128     | `SelectedContentsResult { nextCursor?: string }`                                                                                                            |

**Verdict: Cursor validation is minimal -- just checks it's a non-empty string. No structure/encoding validation. No
scope binding. No order binding. No cursor mismatch detection.**

### A5. Service Layer (Rust boundary service)

| File                                                            | Lines   | Context                                                                                                                                                                        |
|-----------------------------------------------------------------|---------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `crates/library-boundary-service/src/service.rs`                | 348-363 | `read_selected_contents()` -- validates scope and limit, passes `request.cursor.as_deref()` straight to the store. **No cursor validation.**                                   |
| `crates/library-boundary-service/src/snapshot_read_protocol.rs` | 583-603 | `map_selected_contents_result()` -- passes `result.next_cursor` through unchanged                                                                                              |
| `crates/library-boundary-service/src/service.rs`                | 555-585 | `validate_selected_contents_scope()` -- validates IDs are positive i64. `validate_selected_contents_limit()` -- validates `1..=200`. **No cursor validation function exists.** |

### A6. Store Layer (SQLite) -- **CURSOR IS A STUB**

| File                                                               | Lines          | Context                                                                                                                                                                              |
|--------------------------------------------------------------------|----------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `crates/library-store-sqlite/src/store/selected_contents_reads.rs` | 9-20           | `SqliteDurableStore::read_selected_contents()` -- passes `cursor: Option<&str>` through to `read_selected_contents()`                                                                |
| `crates/library-store-sqlite/src/read_models/selected_contents.rs` | 158-171        | **`read_selected_contents()`** -- **If cursor is `Some`, immediately returns `Failed` state with message `"Selected contents cursor paging is not available in this first slice."`** |
| `crates/library-store-sqlite/src/read_models/selected_contents.rs` | 61             | `StoreSelectedContentsResult { next_cursor: Option<String>, ... }`                                                                                                                   |
| `crates/library-store-sqlite/src/read_models/selected_contents.rs` | 194, 215, 1316 | All paths return `next_cursor: None` -- **cursor pagination never actually produces a nextCursor**                                                                                   |

**Verdict: Cursor encoding is NOT implemented. There is NO cursor scope binding. NO cursor order binding. NO cursor
mismatch validation. The store stub-refuses any cursor. `next_cursor` is always `None`. `stale_cursor` does not exist in
any code enum -- it only appears in the design doc.**

### A7. Renderer (Vue/TypeScript) -- Does NOT use cursors

| File                                                                 | Lines   | Context                                                                                                              |
|----------------------------------------------------------------------|---------|----------------------------------------------------------------------------------------------------------------------|
| `apps/desktop/src/renderer/library/boundary/selectedContentsRead.ts` | 126-129 | **Renderer never passes a cursor** when calling `read({ scope, limit: readLimit })` -- only scope and limit are sent |
| `apps/desktop/src/renderer/library/contents/projection.ts`           | 397-409 | `selectedContentsProjectionKind()` -- does NOT handle `stale_cursor` (because it's not in the type)                  |
| `apps/desktop/src/renderer/library/panel.vue`                        | 32      | Contains `disabled:cursor-not-allowed` -- this is a **CSS class**, not data cursor                                   |

### A8. Rust Tests

| File                                                               | Lines | Context                                                                                                                                                                       |
|--------------------------------------------------------------------|-------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `crates/library-store-sqlite/src/read_models/selected_contents.rs` | 1321+ | Large test module (`mod tests`). **No cursor-related tests found** (0 matches of `cursor`/`Cursor`/`next_cursor`). The only cursor mentions are in the production code stubs. |
| `crates/library-boundary-protocol/src/events/session.rs`           | 223   | Test fixture with `cursor: None`                                                                                                                                              |

---

## B. SEPARATE CONCEPT: `ProjectionCursors` (projection subscriber cursors -- unrelated)

This is a **completely different** concept from selected-contents pagination cursors. It tracks subscriber positions for
the projection pub/sub system.

| File                                                                           | Lines   | Context                                                                                                                         |
|--------------------------------------------------------------------------------|---------|---------------------------------------------------------------------------------------------------------------------------------|
| `crates/library-store-sqlite/migrations/20260502000000_substrate_baseline.sql` | 882-890 | `CREATE TABLE ProjectionCursors` -- `subscriber_id`, `projection_domain` (library_browser/navigation), `position`, `updated_at` |
| `crates/library-store-sqlite/src/publication/projections.rs`                   | 162     | `SELECT COUNT(*), MIN(position) FROM ProjectionCursors WHERE projection_domain = ?1`                                            |
| `crates/library-store-sqlite/src/schema/mod.rs`                                | 120     | Registered as table name `"ProjectionCursors"`                                                                                  |

---

## C. SEPARATE CONCEPT: `std::io::Cursor` (Rust standard library I/O)

Not a data cursor at all -- Rust's in-memory I/O `Cursor` for testing stdin/stdout.

| File                                          | Lines                   | Context                                                                   |
|-----------------------------------------------|-------------------------|---------------------------------------------------------------------------|
| `crates/library-boundary-stdio/src/server.rs` | 118, 162, 213, 244, 276 | `use std::io::{BufReader, Cursor};` and `Cursor::new(input)` in test code |

---

## D. OFFSET/LIMIT PATTERNS (Non-cursor pagination)

These are the **offset-based** pagination systems used elsewhere in the codebase.

### D1. Library Browser Window (offset/limit)

| File                                                              | Lines                       | Context                                                                                |
|-------------------------------------------------------------------|-----------------------------|----------------------------------------------------------------------------------------|
| `crates/library-boundary-protocol/src/commands/snapshot_reads.rs` | 229-230                     | `ReadNavigationNodeLibraryBrowserWindowRequest { offset: usize, limit: usize }`        |
| `crates/library-boundary-protocol/src/commands/snapshot_reads.rs` | 260-261                     | `SearchNavigationNodeLibraryBrowserWindowRequest { ..., offset: usize, limit: usize }` |
| `packages/library-boundary-contract/index.ts`                     | 51, 53                      | TypeScript equivalents                                                                 |
| `packages/library-boundary-contract/index.ts`                     | 157                         | `LibraryBrowserWindow { offset, limit, totalRows, rows }`                              |
| `crates/library-boundary-service/src/snapshot_read_protocol.rs`   | 280-281, 519-520, 1094-1095 | Service-layer offset/limit passthrough                                                 |

### D2. Literal Hierarchy (offset/limit)

| File                                          | Lines | Context                                                                      |
|-----------------------------------------------|-------|------------------------------------------------------------------------------|
| `packages/library-boundary-contract/index.ts` | 47    | `ReadLiteralHierarchyChildrenRequest { ..., offset: number, limit: number }` |
| `packages/library-boundary-contract/index.ts` | 123   | `LiteralHierarchyWindow { offset, limit, totalRows, rows }`                  |

### D3. Renderer Hierarchy Read (offset/limit with validation)

| File                                                          | Lines                                                                                | Context                                                                                                                                                    |
|---------------------------------------------------------------|--------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `apps/desktop/src/renderer/library/boundary/hierarchyRead.ts` | 930-931, 947-948, 963-964, 1021, 1039, 1054, 1067, 1083, 1095, 1111-1119, 1128, 1211 | Extensive offset/limit usage with window validation: checks `offset + nodes.length > totalRows`, `offset >= totalRows`, `window.offset !== expectedOffset` |

### D4. Selected Contents Limit Validation (no offset)

| File                                                              | Lines   | Context                                                                |
|-------------------------------------------------------------------|---------|------------------------------------------------------------------------|
| `apps/desktop/src/main/librarySelectedContents/read.ts`           | 28-29   | `defaultSelectedContentsLimit = 100`, `maxSelectedContentsLimit = 200` |
| `apps/desktop/src/main/librarySelectedContents/read.ts`           | 177-196 | `normalizeLimit()` -- validates 1-200 range                            |
| `crates/library-boundary-service/src/service.rs`                  | 577-585 | `validate_selected_contents_limit()` -- validates 1..=200              |
| `crates/library-boundary-protocol/src/commands/snapshot_reads.rs` | 287     | `limit: usize` in `ReadSelectedContentsRequest`                        |

---

## E. SUMMARY TABLE

| Question                                                     | Answer                                                                                                                                                                                                                                                                                           |
|--------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Is cursor encoding implemented?**                          | **NO.** Cursor is `Option<String>` everywhere with no structure, no version, no scope fingerprint, no order key, no serialized last-row tuple. The design doc defines a `SelectedContentsCursor` type with version/scopeFingerprint/order/last -- but this is **never implemented in any code**. |
| **Is cursor bound to scope?**                                | **NO.** No code binds cursor to scope. The design doc says it should be, but it's not implemented.                                                                                                                                                                                               |
| **Is cursor bound to query/order?**                          | **NO.** No code binds cursor to sort order. The design doc says it should be, but it's not implemented.                                                                                                                                                                                          |
| **Does cursor mismatch validation exist?**                   | **NO.** No code validates cursor against current scope state. No fingerprint comparison.                                                                                                                                                                                                         |
| **Is there a `cursorInvalid` / `stale_cursor` error state?** | **NO.** `stale_cursor` exists only in the design document (`docs/decisions/recursive-selected-contents-rule.md` lines 168, 170, 470, 489). It is NOT present in any `SelectedContentsState` enum (neither Rust nor TypeScript). No `cursorInvalid` error code exists anywhere.                   |
| **Does cursor pagination work?**                             | **NO.** The Rust store `read_selected_contents()` at `crates/library-store-sqlite/src/read_models/selected_contents.rs:164-171` **refuses any cursor** with `Failed` state. `next_cursor` is always `None`.                                                                                      |
| **Does the renderer use cursor?**                            | **NO.** The renderer at `apps/desktop/src/renderer/library/boundary/selectedContentsRead.ts:126` only sends `{ scope, limit }` -- never a cursor.                                                                                                                                                |
| **Are there tests for cursor?**                              | **NO.** No test files reference `cursor`/`nextCursor`/`next_cursor` in the desktop tests. The Rust test module in `selected_contents.rs` has no cursor tests. Only a test fixture at `session.rs:223` sets `cursor: None`.                                                                       |
| **What's the current pagination state?**                     | First-page-only. Always returns all rows up to `limit` (max 200). No continuation possible.                                                                                                                                                                                                      |

---

## F. Smallest Safe Cursor Plan for Parameterized Contents Reads

1. Add `cursorInvalid` to result states (TS shared + Rust protocol + service mapping) as a design placeholder
2. Keep the store stub-refusing any cursor for now (same as current behavior)
3. When implementing: encode cursor as `{version: 1, scopeFingerprint, order, last}`, validate fingerprint on return,
   return `cursorInvalid` on mismatch
4. Existing tests pass unchanged (no cursor to break)
