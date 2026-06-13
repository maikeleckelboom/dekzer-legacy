# Substrate SQL And Legacy Audit

Date: 2026-06-12

Deletion-history note, 2026-06-13: this document preserves the pre-deletion audit vocabulary and is no longer a live
implementation target list. Prompt 2 deleted the legacy store/schema/projection/domain surfaces from the current
baseline. Active implementation contracts must use the current source-file, `SourceFacts`, attachment, primary-media,
track-identity candidate, and track-identity decision substrate.

Scope: current library substrate in `crates/library-store-sqlite`, its baseline migration, boundary protocol/service exposure, desktop/client references, and current doctrine documents. This is a greenfield audit: compatibility, dormant authority, transitional vocabulary, and dual paths are risks unless the current A-5/A-6 substrate proves they are still needed.

Doctrine inputs treated as current unless contradicted by the tree:

- A-5: `docs/library/evidence/attachment-occurrence-model-readiness.md`
- A-6: `docs/library/user-decision-pattern-contract.md`
- Source integrity: `docs/library/health/source-integrity-read-model-contract.md`
- Search/filter: `docs/library/search-filter-substrate-contract.md`
- Root admission: `docs/library/source/root-admission-policy.md`

## 1. Executive Verdict

Verdict: needs a deletion/quarantine sprint before more product-facing feature work.

The current A-5/A-6 substrate is safe with targeted repairs for V0 substrate use, but the full library baseline is not clean enough to keep building product surfaces on as-is. Current source registration, source hierarchy, source-file inventory, accepted facts, attachment identity, A-5 occurrence reads, source integrity, search/filter, primary-media candidates, and A-6 track identity candidates/decisions have coherent ownership. However, legacy asset/prep/segment/browser/playlist surfaces are still schema-live, code-live, boundary-visible, and partly renderer-visible. That makes the greenfield baseline ambiguous for future agents.

No immediate data-corruption veto was found. The repairs are still gating because hidden legacy can absorb new feature work into the wrong identity model. Product-facing CUE, canonical track identity, duplicate/relocation UI, prep facets, and source-file preference should wait until the P0 legacy quarantine and the P0/P1 bounding repairs below are complete.

Safe to build on now:

- Source registration/admission and source lifecycle reads.
- Source hierarchy and source file inventory.
- `SourceFacts` as accepted source-file fact evidence, with a watch on its mixed hash/probe shape.
- `content_attachments` and `source_file_attachment_links` for exact-byte attachment identity.
- A-5 attachment occurrence read as bounded detail, not as a row-level hot path.
- Search/filter index as a projection/index, not authority.
- A-6 track identity decision tables as a narrow candidate decision implementation.

Not safe as a greenfield product baseline:

- `LibraryAssets`, `LibraryAssetAttachments`, `SourceSegmentSets`, `SourceSegments`, `LibraryAssetMetadataCorrections`, `LibraryAssetCapabilities`, `LibraryBrowserRows`, `LibraryBrowserRows_fts`.
- Playlist writes and `PlaylistEntries.library_asset_id`.
- Prep policy/assignment/resolution surfaces that still route through legacy asset identity.
- Work item kinds and projection rebuild paths that can still produce legacy library-asset state.

## 2. Current Product Substrate Map

| Area | Durable owner | Read model owner | Write owner | Boundary exposure | Renderer exposure | Classification |
| --- | --- | --- | --- | --- | --- | --- |
| Source registration and admission | `sources`, `source_locators`, `source_registration_proposals` | Local roots/source lifecycle reads | `store/sources.rs`, `authority/ingest/discovery.rs`, root admission service | Register/unregister/read local roots, source lifecycle | Source management flows | Current durable authority |
| Source locator and lifecycle state | `sources`, `source_locators`, `source_state`, `source_scan_state`, `source_root_navigation_state`, `source_locations` | `read_models/source_lifecycle.rs`, `source_integrity.rs`, navigation projections | Source/root lifecycle authorities and scan maintenance | `readSourceLifecycle`, `readSourceIntegrity`, navigation reads | Tree/source UI | Current, with `source_locations` semantic guard |
| Source hierarchy | `source_directories`, `source_files` | `read_models/literal_hierarchy.rs`, `contents.rs`, `search_filter` rebuild | scan observation/source authorities | Tree children, contents, search/filter | Tree and contents UI | Current durable authority |
| Source file inventory | `source_files` | contents, observed facts, source integrity, search/filter, attachment reads | scan observation writes | Contents, observed facts, integrity, attachment, search/filter | Contents/search | Current durable authority |
| Source facts/probe evidence | `SourceFacts`, plus accepted `WorkItems`/`WorkRuns`/`Artifacts` trail | `read_models/observed_file_facts.rs`, `source_integrity.rs`, contents/search/filter joins | `store/source_file_hash.rs`, `store/source_file_media_probe.rs`, `authority/sources/source_facts.rs` | Observed facts, source integrity, search/filter/contents derived state | Source details/search/contents | Current durable fact authority, shape watch |
| Attachment identity | `content_attachments`, `source_file_attachment_links` | `read_models/attachment_identity.rs` | `store/attachment_identity.rs` maintenance | `readSourceFileAttachment`, `readAttachmentSourceFiles`, summaries | Indirect/limited | Current durable attachment authority |
| Attachment occurrence read | Derived over `source_file_attachment_links`, `source_files`, `sources`, `SourceFacts` | `read_models/attachment_identity.rs` limited occurrence read | No durable occurrence write | `readAttachmentSourceFiles` | Indirect/limited | Current bounded-detail read, not hot path |
| Source integrity/read health | No standalone table; read model over source state, hierarchy, facts, attachments, search coverage | `read_models/source_integrity.rs`, `source_integrity_reads.rs` | Maintenance updates source/fact/index state | `readSourceIntegrity` | Source health UI | Current integrity/readiness read |
| Search/filter index | `search_filter_index_metadata`, `search_filter_index_source_coverage`, `search_filter_index_rows`, `search_filter_index_fts` | `read_models/search_filter/*` | `rebuild_search_filter_index_for_source` | `readSearchFilter`, rebuild command | `apps/desktop/src/renderer/library/runtime/searchFilterState.ts` | Current projection/index |
| Primary media candidates | `primary_media_candidates` | contents primary media policy, candidate production summaries | `store/primary_media_promotion.rs` | Contents policy, maintenance | Contents if policy enabled | Current candidate/projection state |
| Track identity candidates | `track_identity_candidates`, members, evidence | candidate/review/decision reads | `store/track_identity_candidates.rs` | Candidate/review reads, accept/reject/defer commands | Future/review-adjacent | Current decision candidate state |
| Track identity decisions | `track_identity_decisions`, source scope, evidence snapshots | `read_models/track_identity_decisions.rs`, review read | `store/track_identity_decisions.rs` and decision commands | Track identity decision reads and writes | Future/review-adjacent | Current A-6 durable decision authority |
| Playlists | `Playlists`, `PlaylistEntries` | navigation rows and legacy browser rows | `store/playlists.rs`, boundary service playlist writes | Playlist write command family | Tree/contents types and icons remain | Legacy live surface |
| Legacy asset/prep/segment surfaces | `LibraryAssets`, segment, prep, capability, browser tables | legacy library browser, waveform, prep detail, navigation projections | library asset/promotion/work authorities | Library asset browser/detail reads, playlist/prep navigation | Some tree/contents roles and support code | Legacy/dormant and must be quarantined |

## 3. Table Inventory

| Table | Classification | Live reads | Live writes | Docs describe as current | Recommendation and blockers |
| --- | --- | --- | --- | --- | --- |
| `LibraryMetadata` | Current maintenance/work state | bootstrap/schema | bootstrap/schema | Yes | Keep. DB metadata, not product authority. |
| `sources` | Current durable authority | source lifecycle, integrity, contents, search/filter, hierarchy | source registration/lifecycle | Yes | Keep. |
| `source_locators` | Current durable authority | local roots, lifecycle | source registration | Yes | Keep. |
| `source_state` | Current durable authority | lifecycle, integrity, search visibility | source lifecycle | Yes | Keep. |
| `source_scan_state` | Current maintenance/work state | integrity, lifecycle | scan maintenance | Yes | Keep. |
| `source_root_navigation_state` | Current maintenance/work state | root navigation/lifecycle | root lifecycle | Yes | Keep. |
| `source_registration_proposals` | Current maintenance/work state | root admission/local roots | `record_or_read_source_registration_proposal` | Yes | Keep, but treat as admission work state, not durable user decision authority. |
| `source_locations` | Current durable authority | lifecycle, navigation, search/filter scope | source/root lifecycle | Yes | Keep with guard. Blocker to cleanup: current navigation and source-location scope reads use it. |
| `browser_user_order` | Current durable ordering preference | navigation/projection reads | source/location lifecycle writes | Yes, as source/navigation order | Keep with explicit guard. It is not A-6 `prefer`/`pin`. |
| `browser_user_prefs` | Future-only placeholder/unclear | No production reads found in targeted pass | No production writes found in targeted pass | Weak/unclear | Quarantine later if unused after a focused prefs audit. |
| `source_directories` | Current durable authority | hierarchy, contents, search/filter, integrity | scan observation | Yes | Keep. |
| `source_files` | Current durable authority | hierarchy, contents, search/filter, facts, attachments, candidates | scan observation | Yes | Keep. |
| `LibraryAssets` | Legacy/dormant | legacy browser, waveform/prep, playlist/prep/projection paths | `authority/library_asset`, promotion paths | Mixed: some docs still call live transitional | Quarantine now, delete later. Blockers: playlist entries, library browser rows, waveform/prep detail reads, projection rebuild, tests. |
| `Playlists` | Legacy live surface | navigation/projection and service tests | playlist boundary writes | Still product-visible in code | Quarantine now. Blockers: boundary command family and renderer tree/contents playlist roles. |
| `PlaylistEntries` | Legacy live surface | playlist browser/navigation | playlist boundary writes | Not aligned with A-5/A-6 current substrate | Quarantine now. Blocker: FK to `LibraryAssets.library_asset_id` and playlist API shape. |
| `CapabilitySpecs` | Legacy/prep seed plus work support | prep detail, waveform overview, capability invalidation, projection | schema seeds | Some docs call live prep substrate | Keep temporarily with explicit guard until prep substrate is redecided. Blocker: seeded constraints, read models, work/capability code. |
| `CapabilityDependencies` | Legacy/prep support | capability invalidation, schema checks | seeds/tests | Some docs call live prep substrate | Keep temporarily with explicit guard. Blocker: capability invalidation code and schema validation. |
| `PrepPolicies` | Legacy/prep live surface | navigation/library browser prep windows | `store/work_items.rs` | Some docs call live prep substrate | Quarantine now. Blocker: navigation rows and prep read models. |
| `PrepPolicyTargets` | Legacy/prep live surface | prep policy resolution/navigation | prep writes | Some docs call live prep substrate | Quarantine now with `PrepPolicies`. |
| `PrepAssignments` | Legacy/prep live surface | prep detail/library browser | `replace_prep_assignments` | Some docs call live prep substrate | Quarantine now. |
| `ResolvedLibraryAssetPrepTargets` | Legacy/prep live surface | prep detail/library browser | `replace_resolved_library_asset_prep_targets` | Some docs call live prep substrate | Quarantine now. Blocker: FK to `LibraryAssets`. |
| `WorkItems` | Current maintenance/work state with legacy work kinds | source inspection, maintenance, legacy promotion/capability | work authorities and stores | Yes for work; legacy kinds not greenfield | Keep table, quarantine legacy subject/work kinds. Blocker: `SourceFacts` accepted artifacts depend on current inspect-source work trail. |
| `WorkRuns` | Current maintenance/work state | source facts, artifact trail, work reads | work run authority | Yes | Keep. |
| `Artifacts` | Current evidence artifact store with legacy users | `SourceFacts`, waveform/prep, file store | source hash/probe and legacy capability writes | Yes for facts; mixed legacy use | Keep table, guard accepted artifact kinds. Blocker: accepted source facts require artifacts. |
| `ArtifactInlinePayloads` | Current artifact payload store with legacy users | source facts, waveform/prep, file store | artifact authority | Yes for facts | Keep with guard. |
| `ArtifactFileStoreEntries` | Current artifact file store with legacy users | waveform/prep/file store | artifact authority | Mixed | Keep with guard. |
| `ArtifactClaims` | Current maintenance/reclamation state | file store reclamation | artifact authority | Mixed | Keep. |
| `SourceFacts` | Current durable authority | observed facts, integrity, contents, search/filter, candidates | source hash/probe acceptance | Yes | Keep, but add invariant tests around basis currentness and mixed hash/probe accepted artifact semantics. |
| `content_attachments` | Current durable authority | attachment reads, contents/search/filter, primary media, track identity | attachment materialization | Yes | Keep. |
| `source_file_attachment_links` | Current durable link authority | attachment occurrence/read, source summary, contents/search/filter, candidates | attachment materialization | Yes | Keep with invariant test for denormalized `source_id`. |
| `search_filter_index_metadata` | Current projection/index state | search/filter read | rebuild | Yes | Keep. |
| `search_filter_index_source_coverage` | Current projection/index state | search/filter read and source integrity | rebuild | Yes | Keep. |
| `search_filter_index_rows` | Current projection/index | search/filter read | rebuild | Yes | Keep. Must not become authority. |
| `search_filter_index_fts` | Current projection/index | search/filter read | rebuild | Yes | Keep. |
| `primary_media_candidates` | Current decision/candidate state | contents primary media, track identity production | primary-media promotion | Yes | Keep as candidate/projection, not canonical track authority. |
| `track_identity_candidates` | Current decision/candidate state | candidate/review/decision reads | candidate production | Yes | Keep. |
| `track_identity_candidate_members` | Current decision/candidate state | candidate/review/decision reads | candidate production | Yes | Keep. |
| `track_identity_candidate_evidence` | Current decision/candidate state | review/decision reads | candidate production | Yes | Keep. |
| `track_identity_decisions` | Current durable decision authority | decision/effective/review reads | decision commands and system production | Yes | Keep. A-6-conforming narrow implementation. |
| `track_identity_decision_source_scope` | Current durable decision authority | effective decision/review reads | decision writes | Yes | Keep. |
| `track_identity_decision_evidence` | Current durable decision snapshot | decision detail/review reads | decision writes | Yes | Keep. |
| `SourceSegmentSets` | Legacy/dormant but code-live | legacy promotion, invalidation, file store, docs/tests | accept segmentation authority | Docs mark dormant/legacy | Quarantine now. Blockers: promotion/invalidation/file-store code and tests. |
| `SourceSegments` | Legacy/dormant but code-live | same as `SourceSegmentSets` | segment authority | Docs mark dormant/legacy | Quarantine now with segment sets. |
| `LibraryAssetAttachments` | Legacy/dormant but code-live | legacy browser/projection/invalidation/promotion | resolve/rebind library asset | Docs mark dormant/transitional | Quarantine now, delete after legacy browser/asset removal. |
| `LibraryAssetMetadataCorrections` | Legacy user correction authority | projection/promotion/library asset correction reads | `store/promotion.rs` | A-6 says needs future migration | Quarantine now. Blocker: projection and correction APIs/tests. |
| `LibraryAssetCapabilities` | Legacy/prep/capability state | waveform/prep/projection/invalidation | capability computation/projection | Some docs call live prep substrate | Quarantine now. Blocker: waveform/prep read models and capability invalidation. |
| `LibraryBrowserRows` | Legacy projection | legacy library browser, navigation, waveform invalidation | projection rebuild, waveform writes | Some docs call live maintained projection | Quarantine now. Blocker: boundary snapshot reads, navigation rows, tests. |
| `LibraryBrowserRows_fts` | Legacy projection/index | legacy browser search | projection rebuild | Some docs call live maintained projection | Quarantine now with `LibraryBrowserRows`. |
| `navigation_rows` | Current projection contaminated by legacy rows | navigation reads, desktop tree | projection rebuild | Yes | Keep table but remove legacy playlist/prep/library-asset row families in quarantine. |
| `ProjectionChangeLog` | Current projection/event maintenance | revisions/events | projection publication | Yes | Keep. |
| `ProjectionSubscribers` | Current projection/event maintenance | event stream | projection publication | Yes | Keep. |
| `ProjectionCursors` | Current projection/event maintenance | projection retention | projection publication | Yes | Keep. |
| `ProjectionRetentionWatermarks` | Current projection/event maintenance | projection retention | projection publication | Yes | Keep. |

## 4. SQL Inventory By Layer

| Layer/file/function | Tables touched | Role | Cardinality and bounds | Sort/order and indexes | V0 safety | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `store/sources.rs` registration/local roots/proposals | `sources`, `source_locators`, `source_registration_proposals`, `source_state`, `source_scan_state`, `browser_user_order` | command write/read | root-scoped, proposal lookup by canonical path | proposal active canonical-path unique index; locator path indexes | Safe | none |
| `authority/ingest/discovery.rs` scan observation | `source_directories`, `source_files`, `source_scan_state`, `WorkItems` | command write/maintenance batch | source-scoped scan traversal | source/path unique indexes | Safe | watch |
| `read_models/source_lifecycle.rs` | source state/locator/location tables | bounded-detail read | one source | PK/source indexes | Safe | none |
| `read_models/literal_hierarchy.rs::read_children` | `source_directories`, `source_files` | hot-path read | parent-scoped `LIMIT/OFFSET` | parent browse indexes support per table; union still sorts | Safe V0 | watch |
| `read_models/contents.rs::read_contents` source-file/audio policies | `source_files`, `SourceFacts`, attachment tables, source/location tables | hot-path read | `LIMIT + 1` with cursor identity | browse-order indexes and keyset cursor | Safe | none |
| `read_models/contents.rs::read_contents` primary-media policy | `source_files`, `source_file_attachment_links`, `content_attachments`, `primary_media_candidates`, `SourceFacts` | hot-path if enabled, otherwise bounded product read | `LIMIT + 1`, CTE windowing | source browse and attachment indexes; window sort may be heavy | Safe for V0 if not default hot path | watch |
| `read_models/contents.rs::read_has_policy_omitted_rows` | `source_files`, candidates | integrity/omission read | small `EXISTS` checks per request | source/file indexes | Safe | none |
| `read_models/observed_file_facts.rs` | `source_files`, `SourceFacts` | bounded-detail read | one source file | PK/source-file facts indexes | Safe | none |
| `store/source_file_hash.rs` candidate/read/commit | `source_files`, `SourceFacts`, `WorkItems`, `WorkRuns`, `Artifacts`, payload tables | maintenance batch and command write | source/file scoped, explicit limit; separate count | source presence index; `lower(relative_path)` order not indexed | Safe V0 | watch |
| `store/source_file_media_probe.rs` candidate/read/commit | same as hash plus probe columns | maintenance batch and command write | source/file scoped, explicit limit; separate count | source presence/class indexes; `lower(relative_path)` order not indexed | Safe V0 | watch |
| `authority/sources/source_facts.rs` promotion | `SourceFacts`, `WorkItems`, `WorkRuns`, `Artifacts` | command write | one source file fact at a time | PK/unique source-file fact | Safe | watch |
| `store/attachment_identity.rs::materialize_attachments_for_source` | `source_files`, `SourceFacts`, `content_attachments`, `source_file_attachment_links` | maintenance batch/projection write | reads all source rows before applying Rust-side `limit` | source presence index; `lower(relative_path)` order not indexed | Safe small V0 only | repair |
| `read_models/attachment_identity.rs::get_attachment_for_source_file` | attachment/link/source/facts tables | bounded-detail read | one source file, `LIMIT 1` | `UNIQUE(source_file_id)` and link index | Safe | none |
| `read_models/attachment_identity.rs::get_source_files_for_attachment_limited` | attachment/link/source/facts tables | bounded-detail read | summary scans all occurrences, page `LIMIT` | attachment index; order by source/source_file lacks composite cursor index | Safe as A-5 detail read | watch |
| `read_models/attachment_identity.rs::get_source_files_for_attachment` | same | dormant/legacy path | unbounded all occurrences | attachment index only | Not for product hot path | repair |
| `read_models/source_integrity.rs` | source state, directories, files, facts, attachments, search coverage | integrity/readiness read | source-scoped aggregates | source indexes; group scans by source | Safe | watch |
| `read_models/search_filter/read.rs` | search filter rows/FTS/coverage plus source visibility | hot-path read | `LIMIT + 1`, cursor identity, scope predicates | projection indexes and FTS; LIKE fallback can scan | Safe V0 | watch |
| `read_models/search_filter/rebuild.rs` | search filter rows/FTS/coverage/source tables | projection rebuild | source-scoped purge/rebuild | deletes FTS rows one by one, then inserts | Safe maintenance | watch |
| `store/primary_media_promotion.rs` | attachment/source facts, `primary_media_candidates` | maintenance batch/projection write | source-scoped, reads full candidate set then Rust-side limit | attachment/fact indexes; window/order may sort | Safe small V0 only | repair |
| `store/track_identity_candidates.rs` production | primary media, attachment links, candidate/member/evidence tables | maintenance batch/candidate write | source-scoped but reads candidate set and counts by full vector | candidate status/member/evidence indexes | Safe internal only | repair |
| `read_models/track_identity_candidates.rs` | candidate/member/evidence tables | bounded-detail read | source-scoped limit, then per-candidate hydration | candidate status index; N+1 hydration | Safe internal | watch |
| `store/track_identity_decisions.rs` production/commands | candidate/member/evidence/decision tables | command write and maintenance batch | decision commands bounded; production reads full candidate set before Rust-side limit | decision current-source/candidate/state indexes | Decision commands safe; production needs repair | repair |
| `read_models/track_identity_decisions.rs` | decision/source/evidence/candidate tables | bounded-detail read | bounded by source/candidate/limit; hydrates evidence/effective per decision | decision indexes; N+1 detail hydration | Safe V0 detail | watch |
| `read_models/track_identity_review.rs` | candidate/member/evidence/decision/source tables | review read | unbounded factual candidates, N+1 hydration, filters after hydration; V1 cursor TODO | candidate indexes partly help; correlated subqueries can grow badly | Not product hot-path safe | repair |
| `store/playlists.rs` | `Playlists`, `PlaylistEntries`, `navigation_rows` | legacy command write | playlist-scoped | playlist/library-asset indexes | Do not build on | veto for new product |
| `read_models/library_browser.rs` | `LibraryBrowserRows`, legacy attachments/segments/prep/playlist tables | dormant/legacy path | windowed but over legacy projection | FTS and projection indexes | Do not build on | veto for new product |
| `read_models/library_asset_waveform_overview.rs` | `LibraryAssets`, capabilities, artifacts, browser rows | dormant/legacy bounded-detail read | one library asset | capability/artifact indexes | Do not build on | veto for new product |
| `read_models/library_asset_preparation_detail.rs` | prep/capability/work/artifact/library asset tables | dormant/legacy bounded-detail read | one library asset, multiple hydrated sections | mixed indexes | Do not build on | veto for new product |
| `publication/projections.rs` | `navigation_rows`, `LibraryBrowserRows`, FTS, legacy and source tables | projection rebuild | broad rebuild/load-current/load-next | multiple projection indexes; broad table reads | Keep for current nav, quarantine legacy rows | repair |
| `publication/projections.rs` change log/subscribers | projection metadata tables | projection/event maintenance | bounded by cursor/subscriber | sequence/domain indexes | Safe | none |

## 5. Query Performance Risks

- Attachment materialization reads all source-file rows for a source before applying the requested limit in Rust. This is a maintenance path, but it becomes expensive on large sources and should be SQL-limited before duplicate/relocation or source-file preference work.
- A-5 occurrence read is acceptable as bounded detail, but the summary scans every occurrence for an attachment and the unbounded `get_source_files_for_attachment` path exists. Do not use it as a hot row-level read.
- Track identity review reads all factual candidates, hydrates each candidate one by one, filters after hydration, and has a V1 cursor TODO. This is not safe for product-facing review UI until bounded/cursored.
- Decision reads are bounded, but they hydrate evidence/effective state per decision. This is acceptable for detail/review windows, not broad lists.
- Track identity candidate and decision production paths read full candidate sets and recount by materializing vectors even when a limit exists.
- Primary media promotion reads full production candidate lists and skip summaries before applying a limit.
- Search/filter is properly cursor-bounded, but text fallback combines FTS with `lower(display_label) LIKE` and `lower(display_path) LIKE`; broad text queries can still scan projection rows.
- Search/filter rebuild deletes FTS rows one by one before deleting normalized rows. This is acceptable for source-scoped rebuild but should be watched for large libraries.
- Literal hierarchy uses offset pagination over a union of directories/files. It is acceptable for V0 folder views, but large single folders need keyset pagination later.
- Several source integrity summaries use source-scoped `GROUP BY`/`COUNT` over `source_files`. Acceptable for readiness reads, not hot loops.
- `source_file_attachment_links.source_id` duplicates `source_files.source_id`. It supports source-scoped counts but lacks a schema-level invariant tying the denormalized value to the linked file.
- Legacy projection rebuild is broad and still builds `LibraryBrowserRows`, playlist rows, prep rows, and source-location rows together. The current navigation table is useful, but legacy rows poison rebuild ownership.

## 6. Data Shape Review

`source_file_attachment_links.source_id`

- Would design today: only if source-scoped attachment queries need it enough to justify denormalization.
- Role: durable link authority with denormalized helper.
- Risk: `source_file_id` already resolves to `source_files.source_id`; without a trigger/check/test, `source_id` can drift.
- Repair: add invariant tests before features depend on source-scoped attachment counts. Consider migrating to a generated/joined shape later, but do not change schema in this audit.

`LibraryAssets` and `PlaylistEntries.library_asset_id`

- Would design today: no. A-5/A-6 current identity is source-file evidence, exact-byte attachment identity, primary-media candidates, and candidate decisions.
- Role: historical library-asset authority and playlist target.
- Risk: product agents can attach new behavior to asset identity instead of current attachment/candidate/decision identity.
- Repair: quarantine playlist and library-asset boundary/read surfaces before product features.

Legacy prep tables

- Would design today: no, not in current V0 substrate. Prep policy, assignment, and resolved target shapes depend on library assets and capability state that predates current evidence/decision contracts.
- Role: historical preparation substrate.
- Risk: prep facets can revive library-asset identity and bypass A-6 decision patterns.
- Repair: quarantine now; redesign prep facets only after canonical track/item identity exists.

`source_locations` observed versus registered semantics

- Would design today: mostly yes, but with sharper naming. Current rows combine observed path/source location state with registered subpath semantics.
- Role: source locator/lifecycle durable state.
- Risk: can be mistaken for a general relocation/duplicate decision authority. A-6 says relocation/preference decisions need explicit decision tables.
- Repair: keep for source lifecycle and navigation, but guard docs and tests against using it as suppression/preference authority.

`SourceFacts` accepted artifact meaning

- Would design today: yes for basis-bound accepted source-file facts, but maybe split hash and probe accepted artifact trails more explicitly.
- Role: durable fact authority, not projection.
- Risk: one row merges hash and media-probe facts and carries accepted artifact identity; future agents can overread this as a generic media evidence object.
- Repair: keep; add tests around basis currentness and accepted artifact semantics before expanding fact families.

`search_filter_index_rows`

- Would design today: yes as projection/index, not authority.
- Role: source/search projection row with filter columns and FTS mirror.
- Risk: denormalized columns can look authoritative.
- Repair: retain explicit docs/tests that rebuild source tables are authority and index rows are disposable.

`track_identity_decisions`

- Would design today: yes for the narrow A-6 implementation.
- Role: durable candidate decision authority with evidence snapshot and scoped precedence.
- Risk: production/review reads need bounding; decisions intentionally avoid FK dependence on live candidates, which is correct but needs continued documentation.
- Repair: keep shape; bound review reads before product UI.

## 7. Legacy And Hidden-Debt Verdict

Hidden legacy exists. The risk is not just dormant tables in the migration. Several legacy surfaces are still reachable through store APIs, projection rebuilds, boundary commands, generated client types, and desktop tree/contents vocabulary.

| Surface | Recommendation | Exact files/tables likely affected | Product risk if left | Product risk if removed too early |
| --- | --- | --- | --- | --- |
| Library asset authority | Quarantine now, delete later | `LibraryAssets`; `crates/library-store-sqlite/src/authority/library_asset/*`; promotion modules; schema/tests | New features bind to stale asset identity | Tests/projections/waveform/prep paths break until surfaces are retired |
| Segment authority | Quarantine now | `SourceSegmentSets`, `SourceSegments`; `authority/promotion/accept_segmentation.rs`; file-store/invalidation references | CUE/subtrack work may reuse stale segmentation model | Legacy promotion tests and artifact-claim paths break |
| Library asset attachments | Quarantine now | `LibraryAssetAttachments`; `authority/library_asset/attachments.rs`; projection/browser reads | Confuses exact attachment occurrence with segment attachment | Legacy browser projection and capability invalidation break |
| Metadata corrections | Quarantine now, migrate later | `LibraryAssetMetadataCorrections`; `store/promotion.rs`; projection code | Future canonical track corrections may reuse old field override model | Existing correction tests/API paths fail |
| Library asset capabilities/prep | Quarantine now | `LibraryAssetCapabilities`, `CapabilitySpecs`, `CapabilityDependencies`, prep tables; waveform/prep read models | Prep facets revive legacy asset/capability authority | Waveform/prep detail and seeded capability checks fail |
| Library browser projection | Quarantine now | `LibraryBrowserRows`, `LibraryBrowserRows_fts`; `read_models/library_browser.rs`; `publication/projections.rs`; boundary snapshot reads | Product browser may fork between legacy asset rows and current contents/search rows | Navigation/browser tests and snapshot compatibility break |
| Playlists | Quarantine now | `Playlists`, `PlaylistEntries`; `store/playlists.rs`; `commands/playlist_writes.rs`; service handlers; client methods; desktop playlist roles | Playlist product work depends on `library_asset_id` | Any existing playlist tests/UI shell break |
| Legacy work kinds | Keep table, quarantine kinds | `WorkItems`, `WorkRuns`; `authority/work/*`; promotion modules | Work queue can still mint legacy artifacts/state | Source inspection work depends on these tables |
| Navigation legacy row families | Keep table, remove legacy families later | `navigation_rows`; `publication/projections.rs`; desktop tree roles | Tree keeps advertising playlist/prep/library-asset lanes | Source-location tree tests need replacement expectations |
| Docs that normalize transition as current | Quarantine/correct docs | `docs/library/evidence/media-identity-schema-authority.md`, `docs/library/contents/selected-scope-depth-rule.md`, roadmap dormant-table list | Future agents treat old tables as current authority | Some historical context lost unless moved to legacy notes |

## 8. Immediate Next Repair Candidates

1. Priority: P0
   Type: quarantine
   Finding: Legacy asset/prep/browser/playlist surfaces are boundary-visible and schema-live.
   Why it matters: New product features can still choose `library_asset_id` instead of A-5/A-6 identities.
   Suggested next prompt title: "Quarantine legacy library asset, prep, browser, and playlist boundary surfaces"
   Expected touched areas: boundary protocol/service/client, `store/playlists.rs`, legacy read models, projection row families, docs.
   Stop condition: No product-facing command/read can create, read, or advertise legacy asset/prep/playlist identity except behind explicit legacy test-only guards.

2. Priority: P0
   Type: performance
   Finding: Track identity review is unbounded, filters after hydration, and lacks cursor identity.
   Why it matters: A review UI would scan and hydrate the candidate universe.
   Suggested next prompt title: "Bound and cursor track identity review candidates"
   Expected touched areas: `read_models/track_identity_review.rs`, store wrappers, boundary request/reply if cursor is exposed, tests.
   Stop condition: Review reads use SQL-level limit/cursor and do not hydrate candidates that cannot appear in the page.

3. Priority: P1
   Type: performance
   Finding: Attachment materialization applies limit after reading all source candidates.
   Why it matters: Source-wide maintenance cost grows before duplicate/relocation/preference features arrive.
   Suggested next prompt title: "SQL-limit attachment materialization candidates"
   Expected touched areas: `store/attachment_identity.rs`, maintenance tests.
   Stop condition: Candidate SQL applies limit/order, and remaining counts do not require materializing all candidates.

4. Priority: P1
   Type: performance
   Finding: Primary media and track identity producers read full candidate sets and recount with vectors.
   Why it matters: Candidate production will be expensive on large libraries.
   Suggested next prompt title: "Bound primary media and track identity production counts"
   Expected touched areas: `store/primary_media_promotion.rs`, `store/track_identity_candidates.rs`, `store/track_identity_decisions.rs`.
   Stop condition: Production limits/counts are SQL-level and source-scoped.

5. Priority: P1
   Type: schema invariant
   Finding: `source_file_attachment_links.source_id` duplicates `source_files.source_id`.
   Why it matters: A drifted link makes source-scoped attachment summaries wrong.
   Suggested next prompt title: "Add attachment link source invariant tests"
   Expected touched areas: schema/store tests around attachment materialization and source-file updates.
   Stop condition: Tests prove inserted/updated links always match the linked source file and stale mismatches are rejected or impossible.

6. Priority: P1
   Type: docs
   Finding: Current docs still call legacy tables live transitional/current in places.
   Why it matters: Agents can treat compatibility wrappers as greenfield authority.
   Suggested next prompt title: "Correct legacy authority wording in media identity and contents docs"
   Expected touched areas: authority map, media identity schema authority, selected-scope depth rule, roadmap dormant inventory.
   Stop condition: Docs point current work to source/file/attachment/candidate/decision substrate and label legacy surfaces as quarantined.

7. Priority: P1
   Type: boundary cleanup
   Finding: Playlist writes are a first-class command family but target `library_asset_id`.
   Why it matters: Playlist product work would start from the wrong target identity.
   Suggested next prompt title: "Retire or guard playlist write boundary"
   Expected touched areas: `commands/playlist_writes.rs`, boundary service, generated client package, desktop mocks/tests.
   Stop condition: No production boundary exposes `appendLibraryAssetToPlaylist`.

8. Priority: P2
   Type: performance
   Finding: Search/filter text fallback can scan projection rows.
   Why it matters: Broad library text search can become heavy as rows grow.
   Suggested next prompt title: "Audit search/filter text fallback query plan"
   Expected touched areas: `read_models/search_filter/read.rs`, query-plan tests, FTS behavior tests.
   Stop condition: Broad text queries prefer FTS and any LIKE fallback is bounded or justified.

9. Priority: P2
   Type: performance
   Finding: Literal hierarchy uses offset pagination over unioned children.
   Why it matters: Huge folders can degrade navigation.
   Suggested next prompt title: "Keyset cursor for literal hierarchy children"
   Expected touched areas: `read_models/literal_hierarchy.rs`, tree boundary if cursor shape changes.
   Stop condition: Folder children page by stable keyset without offset scan.

10. Priority: P2
    Type: test
    Finding: `SourceFacts` merges hash/probe accepted fact state in one row.
    Why it matters: Future fact families can accidentally overwrite basis or accepted artifact semantics.
    Suggested next prompt title: "Lock SourceFacts basis and accepted artifact invariants"
    Expected touched areas: `authority/sources/source_facts.rs`, hash/probe store tests.
    Stop condition: Tests cover currentness, hash/probe merge behavior, accepted artifact references, and stale basis handling.

## 9. Non-Goals And Accepted Debt

- Bounded detail reads are acceptable when they are not hot-path product lists. This includes A-5 occurrence reads with an explicit limit and decision detail reads.
- Source-scoped integrity summaries may scan/group source files for V0. They are readiness reads, not row-level browse loops.
- Small duplicate summary plus page-query patterns are acceptable for bounded detail reads, as long as they are not used as list hot paths.
- Internal maintenance reads may remain source-scoped and batch-oriented for V0, provided they do not become user-interactive product reads.
- Search/filter rows may denormalize source/file/fact/attachment state because they are explicitly rebuildable projection/index rows.
- `WorkItems`, `WorkRuns`, and artifact tables remain acceptable because current `SourceFacts` needs accepted artifact provenance. The debt is legacy work kinds, not the existence of the work/artifact tables.
- Docs-only future doctrine is acceptable if it does not point implementation agents at legacy tables as current authority.

## 10. Acceptance Bar Before Next Feature Work

Before CUE, canonical track identity, duplicate/relocation UI, prep facets, or source-file preference:

- Legacy tables/surfaces that must be deleted or quarantined first: `LibraryAssets`, `PlaylistEntries`, `LibraryAssetAttachments`, `SourceSegmentSets`, `SourceSegments`, `LibraryAssetMetadataCorrections`, `LibraryAssetCapabilities`, `LibraryBrowserRows`, `LibraryBrowserRows_fts`, prep policy/assignment/resolution tables, and legacy playlist/prep/library-asset navigation row families.
- SQL reads that need cursor/bounding first: `read_models/track_identity_review.rs`; attachment materialization candidate reads; primary-media and track-identity production candidate/count reads. The unbounded attachment occurrence helper must not be product-exposed.
- Data-shape invariants that need tests: `source_file_attachment_links.source_id` matches `source_files.source_id`; `SourceFacts` basis currentness and accepted artifact merge behavior; search/filter rows remain disposable projection; `track_identity_decisions` snapshot evidence stays independent of live candidate mutation.
- Docs that must stop pointing agents at legacy authority: roadmap dormant inventory, `media-identity-schema-authority.md`, and legacy passages in `selected-scope-depth-rule.md` that still describe `LibraryBrowserRows`/`library_asset_id` behavior as implementation shape.
- Boundaries safe to build on now: source lifecycle/integrity, contents source-file/audio reads, search/filter reads, observed facts, source-file attachment reads, limited A-5 attachment occurrence read, and A-6 track identity decision commands. The track identity review read is safe only after the P0 bounding/cursor repair.
