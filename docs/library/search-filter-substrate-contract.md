---
status: doctrine-gate
doctrine-version: 0.1
last-reviewed: 2026-06-11
owner: library-substrate-boundary
canonical-context:
  - product-doctrine
  - product-roadmap-and-substrate-authority
  - source-hierarchy-contract
  - contents-read-boundary-contract
  - browse-policy-and-classification
  - representation-contract
scope:
  - search-filter-substrate
  - backend-owned-search-index
  - query-identity-and-cursor-law
  - staleness-and-rebuild
  - result-authority
---

# Search/Filter Substrate Contract

## 1. Status and Scope

This is the doctrine gate for **A-8 Search/filter index v0**. It defines the substrate contract that must exist before
any search/filter browser surface is implemented.

This pass is doctrine only:

- no code implementation;
- no schema migration;
- no renderer UI;
- no search box implementation;
- no source browser feature change.

The purpose is to block renderer-owned search truth and define the backend-owned search/filter projection over current
substrate authority.

This contract explicitly does not introduce:

- track identity;
- canonical tracks;
- playlist, crate, smart-list, or Prepared Room search;
- product duplicate or relocation surfaces;
- automatic cleanup, removal, or merge behavior;
- collection health reads;
- CUE parsing, CUE-to-audio pairing, or CUE-derived split rows;
- tag ledger behavior;
- media candidate rows;
- preparation facets;
- deck runtime behavior.

Search/filter v0 is a local-library substrate over sources, source locations, directories, source files, and available
evidence summaries. It is not a product collection layer.

## 2. Authority Model

Search/filter is a backend-owned projection/index over existing substrate authority.

The search index does not become a new source of truth. It is a derived read model whose membership and fields are
rebuilt from authoritative tables and read models:

| Result concern                                                    | Authority layer                                                                              |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Source identity, display, lifecycle/access state                  | `sources`, `source_locators`, `source_state`, source lifecycle reads                         |
| Source locations                                                  | `source_locations`                                                                           |
| Directory identity, path, presence, scan coverage                 | `source_directories`                                                                         |
| Source-file identity, path, name, file class, file kind, presence | `source_files`                                                                               |
| Current BLAKE3 evidence availability                              | current `SourceFacts` basis and `content_hash_algorithm = 'blake3'`                          |
| Attachment identity summary                                       | `content_attachments` plus `source_file_attachment_links`, with link status computed by join |
| Media probe summary                                               | current-basis `SourceFacts` probe fields                                                     |
| CUE/companion classification                                      | current source-file `file_kind`/`file_class` only                                            |

The renderer may:

- submit typed search/filter requests;
- render accepted backend results;
- keep stale-safe prior results visible while a new request is pending;
- request the next page using the backend-provided cursor;
- discard pending or returned results whose echoed query identity does not match the active request.

The renderer must not:

- scan the library itself;
- build or maintain its own source truth;
- own search result membership;
- filter an accepted result set into a different authoritative result set;
- merge pages across incompatible query identities;
- infer file class, evidence state, attachment status, media probe state, or source state from display strings;
- infer track identity from source-file, BLAKE3, attachment, or probe evidence.

Search/filter reads are sibling boundaries to hierarchy and contents reads. The tree remains navigation-only. Contents
reads remain selected-scope row reads. Search/filter may return typed results from multiple target kinds, but only as a
backend-owned generated query representation.

## 3. Indexed Material for v0

A-8 v0 indexes the fields needed for local source discovery, source-file discovery, and evidence-aware filtering.

Indexed material:

- source identity and display fields;
- source locator/effective root display fields where safe to expose through existing source rules;
- source-location names and relative prefixes;
- directory names and source-relative path segments;
- source-file names and source-relative path segments;
- file class and file kind;
- media relevance according to current source-file inventory policy;
- current source-file presence state;
- source and directory scan/access state needed to avoid false-empty results;
- whether current BLAKE3 evidence exists for the source file;
- attachment identity summary where available:
  - attachment id;
  - hash algorithm and a display-safe hash summary;
  - computed link status: current, stale, or missing;
- media probe summary where available:
  - probe summary present or missing;
  - probe-supported evidence when current fields exist;
  - unsupported or failed status only when current implementation has an explicit basis for that result;
  - container/audio summary fields already stored in `SourceFacts`;
- CUE/source companion classification that is already available from `file_kind = 'cue_sheet'` or existing companion
  file classification.

Explicitly deferred:

- canonical tracks;
- canonical track metadata;
- media candidates;
- primary-media activation as the default browser/search population;
- preparation/readiness facets;
- tags as canonical metadata;
- crates, playlists, smart lists, sleeves, Prepared Room objects, and authored membership;
- acoustic fingerprints;
- CUE parse observations;
- CUE-to-audio associations;
- CUE-derived split tracks;
- occurrence, product duplicate, relocation, offline, or backup/copy views;
- recommendations, ranking intelligence, and musical similarity.

Existing exact-content track identity candidate and decision tables do not authorize v0 search results to become
canonical track results. A-8 may not surface those tables as the default search population unless a later contract names
the result kind and authority.

## 4. Search Target Model

Search results are typed. They must not be flattened into tracks.

V0 target kinds:

| Result kind                 | Stable identity                                                          | Required authority                       |
| --------------------------- | ------------------------------------------------------------------------ | ---------------------------------------- |
| `source`                    | `source_id`                                                              | source lifecycle/source record authority |
| `sourceLocation`            | `source_location_id`                                                     | source-location authority                |
| `directory`                 | `source_directory_id` plus `source_id`                                   | source hierarchy authority               |
| `sourceFile`                | `source_file_id` plus `source_id`                                        | source-file inventory authority          |
| `attachmentEvidenceSummary` | attachment id plus source-file link id when link-scoped                  | attachment identity read authority       |
| `mediaProbeEvidenceSummary` | `source_file_id` plus accepted probe artifact/fact basis where available | observed-facts/media-probe authority     |

The backend may choose to return evidence summaries as separate results or as attached summaries on `sourceFile`
results. Either shape is valid only when the response states the result kind and authority layer explicitly.

Every result must carry:

- `resultKind`;
- stable result identity for that kind;
- authority layer;
- source id when applicable;
- source-location id, directory id, or source-file id when applicable;
- display label;
- source-relative path or display path when applicable;
- matched fields or match reason where possible;
- evidence summary fields that are already authorized by the indexed material;
- current/stale/rebuild state where applicable;
- result capability profile sufficient to prevent track, playlist, crate, delete, merge, or source-move assumptions.

Search result identity is result-kind scoped. A source file and an attachment evidence summary may both point at the
same bytes evidence, but they are different result identities with different authorities and actions.

## 5. Filter Model

Filters are backend-owned request inputs. They are not renderer-side list filtering.

The request composes these dimensions:

| Dimension             | v0 values                                                                                              |
| --------------------- | ------------------------------------------------------------------------------------------------------ |
| Scope                 | library, source, source location, directory                                                            |
| Recursion             | immediate, recursive                                                                                   |
| Target kind           | source, source location, directory, source file, evidence summaries, or backend-supported combinations |
| File class            | audio, video, image, unsupported, none when explicitly admitted                                        |
| File kind             | audio, video, image, cue sheet, log doc, text doc, archive, other, unknown                             |
| Media relevance       | audio workflow, playable media, explicit inventory, companion files when supported                     |
| Presence/access state | present, missing, removed, unavailable source, blocked source, failed source, incomplete coverage      |
| BLAKE3 evidence       | has current BLAKE3, missing BLAKE3, stale BLAKE3 basis                                                 |
| Probe evidence        | has current probe summary, missing probe, probe failed, unsupported, not admitted                      |
| Attachment state      | has current attachment link, stale link, missing link                                                  |
| Text query            | empty or normalized search text                                                                        |

Composition law:

1. Scope limits the result universe first.
2. Recursion chooses immediate or descendant coverage inside that scope.
3. Target kind determines which indexed rows may be returned.
4. File class, file kind, media relevance, presence/access, evidence, probe, and attachment filters are conjunctive
   unless the request explicitly uses a backend-defined union group.
5. Text query intersects the filtered universe. An empty text query is allowed only for filter/browse use cases that the
   backend explicitly admits.
6. Sorting and pagination are applied after the backend has resolved the full query semantics.

Renderer-visible controls may compile into this request, but the renderer does not own the predicate. If the backend
does not support a filter combination, it must return a typed unsupported/policy-conflict state rather than silently
approximating the result in renderer code.

## 6. Query Identity and Cursor Law

Search/filter request identity is the complete tuple that decides whether a request, response, cursor, retained result,
or accumulated page belongs to the same logical query stream.

The identity tuple includes:

- scope kind and stable scope identifiers;
- recursion mode;
- target-kind set;
- normalized text query;
- complete canonicalized filter set;
- sort;
- page size;
- result field/profile version, if the boundary supports multiple profiles;
- index generation or snapshot token, if the implementation exposes one;
- cursor version and cursor identity fields for paged reads.

Cursor law:

- Cursor is backend-issued and opaque to the renderer.
- Cursor is bound to the complete query identity.
- Cursor reuse across a changed query identity is invalid.
- Changing text query, filter set, scope, recursion, target kind, sort, page size, or index generation resets
  pagination.
- A malformed, expired, generation-mismatched, or identity-mismatched cursor returns `cursorInvalid` with no next cursor
  and no authoritative empty state.
- The backend must never silently ignore a supplied cursor and return page one.
- Pages are appended only when the response identity exactly matches the accepted active identity and the cursor
  continues the same stream.
- The renderer must not merge pages from different accepted identities, even when display labels appear similar.

Renderer retention law:

- The renderer may retain the last accepted results for the same accepted query identity.
- During a new pending request with a changed identity, the renderer may show prior results only as explicit
  stale-safe/retained-pending presentation.
- Retained-pending presentation must not prove empty, current, complete, or final.
- A response whose echoed identity does not match the active request must be ignored or rejected.

Cursor ordering must be deterministic. A-8 must use keyset pagination, not offset pagination, for search/filter result
pages unless it proves offset windows cannot duplicate or skip rows across index generations. The cursor position must
include stable tie-breakers, ending with result kind and stable result id.

## 7. Sorting

V0 sort options are conservative:

- `relevance` when a non-empty text query exists;
- `pathName` stable sort for path/name browsing and filter-only queries;
- `evidenceStatus` only if A-8 can implement it cleanly as a deterministic backend order with stable tie-breakers.

`relevance` is a text-search ranking over indexed fields only. It is not musical similarity, readiness, popularity, user
preference, collection identity, or track identity.

When implemented in v0, relevance order is deterministic and limited to indexed text evidence:

1. exact label match;
2. label prefix match;
3. label contains match;
4. path contains match;
5. FTS/text-only match;
6. stable path/name sort key;
7. result kind order;
8. stable result id.

All sorts must end in a deterministic tie-breaker:

1. source-relative normalized path or display sort key where applicable;
2. result kind order;
3. stable result id.

V0 must not rank by canonical track fields, musical metadata, acoustic fingerprints, imported tags, preparation state,
playlist membership, crate membership, or Prepared Room context.

## 8. SQLite FTS Decision

A-8 should implement a **hybrid normalized search table plus SQLite FTS5 text index**.

The normalized table is the source of search/filter result membership, authority fields, filter columns, generation
metadata, and deterministic keyset sort fields. The FTS5 table is a text acceleration structure keyed to the normalized
row identity.

Rationale:

- Dekzer is local-first desktop software; SQLite is already the local store and keeps search data local.
- Large libraries need indexed text lookup over path segments and names; `LIKE` scans over `source_files` will not age
  well.
- Filters need structured columns and deterministic pagination; FTS alone is not an authority model.
- Path/name search benefits from tokenized text plus exact/prefix path matching. The normalized table can carry stable
  path sort keys while FTS supplies candidate matching.
- Incremental rebuild can update rows by substrate identity instead of rebuilding every query from raw tables.
- Future tags, media candidates, tracks, and prep facets can add new target rows or text fields without giving the
  renderer an index.
- Query identity can include an index generation/snapshot token, and cursors can include deterministic sort positions.

The FTS table should be contentless or externally-contented only if A-8 can keep rebuild semantics simple and testable.
If that introduces unnecessary complexity, A-8 may use an ordinary FTS table whose row payload is rebuilt from the
normalized search table. The key law is unchanged: normalized backend rows decide membership and filters; FTS only helps
text matching.

Exact prefix/path matching should be preserved alongside FTS. A query that looks like a path fragment may match path
segments deterministically even when FTS tokenization would split punctuation differently. This is an additive backend
matching rule, not renderer filtering.

## 9. Staleness and Rebuild

An index row is stale when it no longer reflects the substrate authority row(s) from which it was derived.

Staleness triggers:

- source display/lifecycle/access state changes that affect source result fields or scoped availability;
- source locator/effective path changes that affect display-safe source path fields;
- source-location insert/update/removal;
- source-directory insert/update/removal, path/name changes, presence changes, scan state changes, or parent changes;
- source-file insert/update/removal, path/name changes, file class/kind changes, presence changes, parent changes, size
  changes, or mtime changes;
- `SourceFacts` basis change, insertion, removal, hash change, probe-field change, or current/stale basis transition;
- `content_attachments` insertion or hash summary change;
- `source_file_attachment_links` insertion, replacement, deletion, or computed link-status transition;
- classifier or indexer version changes that alter tokenization, indexed field selection, normalized sort fields, or
  filter columns.

Index rows may have their own rebuild state because the search index owns that derived state. That does not authorize
stored stale booleans on observed-fact or attachment-link authorities. Evidence/link validity remains computed by the
authority read model; the search index may cache a derived status for its own row generation and must repair it from
authority on rebuild.

Rebuild model:

- Rebuild is backend-owned.
- Rebuild may be eager for small direct writes and bounded/scheduled for scan or maintenance batches.
- Rebuild work must be source/scoped where possible and bounded by implementation limits.
- A-8 must expose whether the index is current, rebuilding, partially rebuilt, unavailable, or behind substrate truth for
  the requested scope.
- If the index is behind substrate truth, reads may return retained indexed results only with explicit
  `indexState = rebuilding` or equivalent state. They must not claim complete/current coverage.
- If the backend cannot prove a query's index coverage, the result is incomplete/pending, not authoritative empty.

Repair model:

- Source scan changes enqueue or run rebuild for affected source, source locations, directories, and source files.
- Observed fact changes rebuild affected source-file and evidence-summary rows.
- Probe changes rebuild affected source-file/probe-summary rows.
- Attachment changes rebuild affected source-file and attachment-summary rows.
- Gap recovery or unknown invalidation may request a conservative visible-scope or library-wide rebuild check.
- Full rebuild is allowed as an explicit repair path when generation/version checks detect drift.

Index generation:

- The backend should maintain an index generation or snapshot token per whole index, per source, or per maintained
  partition.
- Source, source-location, and directory scopes derive coverage from the owning source's search index coverage row.
- Library scope is complete only when every visible source relevant to the query has ready source coverage for the
  current indexer version.
- Missing, rebuilding, partial, or failed source coverage may return retained rows, but it must not return an
  authoritative empty result.
- Search responses echo the generation/snapshot token they read.
- Cursors include that token if exposed.
- When generation changes, old cursors are invalid unless the backend can prove the cursor's snapshot remains readable.

## 10. Invalidation and Read-After

Search/filter readers refresh through existing boundary event principles plus an A-8-owned index generation signal.

Inputs that may require refresh:

- `SourceScanEvent` terminal events for visible or queried sources;
- `MaintainedSnapshotInvalidated` events for source/browser scopes that affect underlying rows;
- future maintained snapshot invalidation scope for search index, if A-8 adds one;
- index generation changes observed in search/filter responses;
- event-ring gap recovery.

UI contract:

- The renderer does not replay scan details to patch search membership.
- The renderer requests an authoritative reread for the active query identity when an invalidation/generation change
  applies.
- The renderer clears incompatible cursors and page accumulators.
- The renderer may retain accepted results as pending/stale-safe presentation until the authoritative reread returns.
- Gap recovery schedules a conservative authoritative reread and may clear warm search snapshots.

Publication/replay details are boundary-service implementation concerns. The UI contract only requires that readers can
detect "my accepted result may be stale" and issue a new backend read. The event stream remains cursor-only and
session-bounded; it is not a durable search replay log.

## 11. Language Law

Search/filter language must preserve identity-layer boundaries.

Allowed terms for attachment/source evidence:

- "same content";
- "same bytes";
- "same attachment";
- "content attachment";
- "source-file occurrence";
- "current attachment link";
- "stale attachment link".

Forbidden terms and claims:

| Never say                                    | Required replacement or rule                                                                          |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| "same track"                                 | Use "same content", "same bytes", or "same attachment" when evidence is attachment/source based.      |
| "duplicate song"                             | Use "same content evidence in multiple locations" only in a later authorized occurrence surface.      |
| "duplicate track"                            | Never for source-file, hash, attachment, or probe evidence.                                           |
| "track moved"                                | Use "content attachment found at new source-file path" only in a later authorized occurrence surface. |
| "safe to delete"                             | Never recommend deletion.                                                                             |
| "cleanup", "remove duplicates", "auto-merge" | Never as system actions. Surface evidence only and require an explicit user decision record.          |

A-8 search/filter results must not use attachment evidence, BLAKE3 evidence, probe evidence, path similarity, tag-like
strings, or CUE companion classification to imply track identity.

Search/filter may help find evidence. It must not resolve identity, recommend removal, or collapse rows.

## 12. A-8 Implementation Gates

A-8 is acceptable only when it satisfies these gates.

### Schema/index tables

A-8 should add:

- a normalized search-index row table owned by the search/filter index;
- a SQLite FTS5 table keyed to normalized search-index rows;
- generation/version metadata for the index or index partitions;
- source-owned coverage rows keyed by source id, indexer version, generation, and ready/rebuilding/partial/failed state;
- optional backend-owned rebuild job/state rows if needed for bounded rebuild.

A-8 must not add:

- canonical track tables;
- prep facet tables;
- tag ledger tables;
- occurrence/product duplicate/relocation tables;
- renderer cache tables as search authority;
- stored stale booleans on `SourceFacts` or `source_file_attachment_links`.

### Backend read boundary shape

The boundary must expose one typed read command, shaped around:

- `scope`;
- `recursion`;
- `targetKinds`;
- `textQuery`;
- canonicalized `filters`;
- `sort`;
- `limit`;
- optional `cursor`;
- request identity echo fields.

The response must include:

- `state`: ready, rebuilding/partial, cursorInvalid, unsupported/policyConflict, or readFailed;
- echoed query identity;
- result rows with result kind, stable identity, typed authority layer, display fields, typed match reason where
  possible, and typed index/evidence/link state;
- scope/index coverage state;
- `nextCursor` only when more rows exist for the same identity;
- generation/snapshot token if implemented.

### Cursor behavior

Tests must prove:

- malformed cursor returns cursor-invalid state with no rows accepted as authoritative page continuation;
- changed scope rejects old cursor;
- changed recursion rejects old cursor;
- changed text query rejects old cursor;
- changed filters reject old cursor;
- changed sort rejects old cursor;
- changed page size rejects old cursor unless A-8 explicitly excludes page size from identity with a written rationale;
- changed index generation rejects old cursor unless stable snapshot reads are implemented;
- page two has no duplicates and no gaps under deterministic sort;
- renderer-facing shared code cannot append rows across incompatible identities.

### Rebuild behavior

A-8 must prove:

- source scan changes enqueue or run bounded rebuild for affected rows;
- source-file path/name/presence/file-class changes update or remove affected index rows;
- current BLAKE3 evidence changes update evidence filters and attachment summaries;
- media probe changes update probe filters and summaries;
- attachment link changes update attachment filters and summaries;
- index-version/tokenizer changes can trigger full or partition rebuild;
- incomplete index coverage is exposed as partial/rebuilding, not empty.

### Minimal test matrix

Minimum backend tests:

- source, source-location, directory, and source-file result kinds;
- library, source, source-location, and directory scopes;
- immediate and recursive scope behavior;
- text query over names and path segments;
- empty text query with backend-admitted filters;
- file class/media relevance filters;
- present/missing/removed filtering;
- current/missing/stale BLAKE3 evidence filters;
- current/stale/missing attachment link filters;
- current/missing/failed/unsupported probe filters where data exists;
- relevance sort with deterministic tie-breakers;
- path/name sort with deterministic tie-breakers;
- cursor identity rejection cases listed above;
- index-behind-substrate state.

Minimum boundary/renderer tests:

- renderer submits request and renders accepted results without local search membership logic;
- retained-pending results do not prove empty/current/complete;
- invalidation/gap recovery clears incompatible cursors and schedules reread;
- page accumulation is keyed by echoed query identity.

### Explicit non-goals

A-8 must not implement:

- renderer-owned search index or renderer-side authoritative filtering;
- search UI/search box behavior;
- canonical track search;
- playlist, crate, smart-list, sleeve, Prepared Room, or deck search;
- CUE parsing or CUE-derived split rows;
- tag ledger or canonical metadata;
- media candidate layer;
- collection health read model;
- occurrence/product duplicate/relocation model;
- preparation facets;
- automatic cleanup, removal, or merge actions.

## Final Law

Search/filter v0 is a backend-owned generated query representation over the local library substrate. It may search
sources, locations, directories, source files, and authorized evidence summaries. It must preserve authority labels,
cursor identity, staleness state, and identity-layer language. It must not create tracks, imply collection identity, or
allow the renderer to decide what the library search results are.
