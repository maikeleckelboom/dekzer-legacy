# Dekzer Documentation Authority Map

_This is the canonical index of Dekzer documentation authority. Every doc in this repo must have one clear role.
No two active docs may define the same authority._

## Product doctrine

| Doc                                  | Role                           | Notes                                                                                                                                  |
| ------------------------------------ | ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/product-doctrine-shortened.md` | **Canonical product doctrine** | Active doctrine. Owns product position, identity stack, claims, readiness, conflict resolution, modeling laws, first slice boundaries. |
| `docs/archive/product-doctrine.md`   | Superseded archive             | Long-form v2 draft. Superseded by `product-doctrine-shortened.md`. Historical source material only.                                    |

## Source and scan contracts

| Doc                                                     | Role                                                                      | Notes                                                                                                                                                                                                                                                   |
| ------------------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/source-root-scan-admission-contract.md`           | **Canonical scan admission contract** (with marked implementation status) | Owns root registration, root identity, scan preflight, traversal policy, candidate admission, magic reads, work budgets, observation persistence. Implementation status note distinguishes desired architecture from current incomplete implementation. |
| `docs/source-root-scan-reconnaissance-2026-05-27.md`    | Archived evidence report                                                  | Evidence only. Do not use as architecture authority. Canonical contract: `source-root-scan-admission-contract.md`.                                                                                                                                      |
| `docs/decisions/source-access-and-scan-coverage.md`     | Valid architectural decision                                              | Source access and scan coverage rules. Complements scan admission contract at lower detail level. Does not supersede the admission contract.                                                                                                            |
| `docs/decisions/source-locations-lifecycle-contract.md` | Valid architectural decision                                              | Source location lifecycle and aggregate scope. Complements scan admission contract for source-location specifics.                                                                                                                                       |
| `docs/library/source-lifecycle-backend-contract-gap.md` | Inventory / contract gap                                                  | Backend inventory of durable lifecycle facts, runtime events, and the explicit contract gap for a backend-owned source lifecycle read surface.                                                                                                          |

## Library browser contracts

| Doc                                                          | Role                                                         | Owns                                                                                                                                                                                                             |
| ------------------------------------------------------------ | ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/library/library-tree-frame-stability-contract.md`      | **Canonical library contract** (with marked future sections) | Branch rendering continuity, cache behavior, node identity, response guards, prefetch bounds, drag stability. Future sections marked for row profiles, batch reads, child summaries, targeted scan invalidation. |
| `docs/library/library-row-profile-contract.md`               | **Future architecture**                                      | Row-kind projection policy for future product-owned tree profiles. Current `readLibraryTreeChildren` does not accept `rowProfile`.                                                                               |
| `docs/library/library-tree-selection-contents-contract.md`   | **Canonical library contract**                               | Selected-node scope derivation, contents panel read-model, selection survival under source state changes, first-page law, scan update invalidation for selected scope.                                           |
| `docs/library/library-tree-track-segment-rows-contract.md`   | **Future architecture**                                      | Node-kind taxonomy for future tree-projection row kinds (`primary_media`, `segment`, `companion`, etc.). Current first-slice tree row kinds are `file` and `directory`.                                          |
| `docs/library/source-lifecycle-visible-state-contract.md`    | **Canonical library contract**                               | Source visible availability states, unavailability UX, relocation UX, cloud placeholder behavior, source removal vs forgetting.                                                                                  |
| `docs/library/library-browser-workspace-surface-contract.md` | **Canonical library contract**                               | Library Browser workspace surface identity, topology handoff, geometry/viewport hints, internal layout presets, authority partition.                                                                             |
| `docs/library/library-boundary-event-stream-contract.md`     | **Canonical library contract**                               | Cursor-only boundary event model, `ReadAfter` semantics, event ring, gap recovery, scan event family, maintained snapshot invalidation, event parser contract.                                                   |
| `docs/library/media-relevant-file-inventory-contract.md`     | **Canonical library contract**                               | Media-relevant source-file inventory policy, store/read-model ownership, default contents admission, CUE/image boundaries, presence, ordering, and renderer projection limits.                                   |
| `docs/library/media-identity-schema-authority.md`            | **Canonical library contract**                               | Asset identity, `primaryMedia` authority, CUE association, content hashing placement, and intentionally dormant identity/prep surfaces. Depends on source lifecycle and media-relevant file inventory contracts. |
| `docs/library/attachment-identity-foundation-contract.md`    | **Canonical library contract**                               | Rust/store-only attachment identity foundation from current BLAKE3 observed-file facts, source-file attachment links, computed staleness, and deferred boundary/track/prep work.                                 |
| `docs/library/observed-file-facts-contract.md`               | **Canonical library contract**                               | Observed source-file evidence, file-basis validity, algorithm-tagged content hash placement, CUE observation ownership, and current/future boundaries for probing and identity consumption.                      |
| `docs/library/source-maintenance-orchestration-contract.md`  | **Canonical library contract**                               | Backend-owned bounded source maintenance unit order, command/read boundary, runtime-vs-durable ownership, scan-triggered behavior, and invalidation behavior.                                                    |
| `docs/library/primary-media-promotion-contract.md`           | **Canonical library contract**                               | Evidence-backed primary-media v0 promotion target, eligibility, `readContents` behavior, source-maintenance integration, and non-goals.                                                                          |
| `docs/library/track-identity-candidate-contract.md`          | **Canonical library contract**                               | Backend/store-owned exact evidence track identity candidate foundation, grouping provenance, staleness, read model, and explicit non-canonical boundaries.                                                       |
| `docs/library/track-identity-decision-contract.md`           | Provisional implementation contract                          | Backend/store-owned track identity decision foundation over candidates, exact-content v0 decision production, candidate-level effective decision semantics, copied-provenance snapshots, supersession, and non-canonical boundaries. |
| `docs/library/track-identity-decision-authority-contract.md` | Provisional implementation contract                          | Backend-owned accept/reject/defer command family for candidate-scoped user decisions, controlled decision sources, user/system precedence, source-maintenance interaction, and canonical-track non-goals.        |

## Contents read boundary docs

| Doc                                                  | Role                             | Notes                                                                                                                                                                                                                               |
| ---------------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/decisions/library-contents-read-boundary.md`   | Canonical contents read boundary | Defines the single parameterized contents read boundary. Stale `sourceFileVisibility`-aware claim removed (historical note preserved). Complements `library-tree-selection-contents-contract.md` (which governs renderer coupling). |
| `docs/decisions/recursive-selected-contents-rule.md` | Valid architectural decision     | CURRENT WITH LEGACY VOCABULARY. Schema-specific references predate v1 substrate. Architectural rules remain valid. For current v1 vocabulary see `docs/decisions/library-preparation-substrate-v1.md`.                              |

## Implementation discipline

| Doc                                           | Role                               | Notes                                                                                                                                                                           |
| --------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/substrate-implementation-discipline.md` | Reusable implementation discipline | Mined from superseded V1 Substrate Port brief. Preserves durable process laws only. Not architecture authority. Supersedes `code_change_brief_v_1_substrate_port.md` (deleted). |

## Evidence reports

| Doc                                                  | Role                     | Notes                                                                        |
| ---------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------- |
| `docs/source-root-scan-reconnaissance-2026-05-27.md` | Archived evidence report | Evidence only. Canonical contract: `source-root-scan-admission-contract.md`. |

## Superseded / archive material

| Doc                                     | Role               | Notes                                                                                                                                     |
| --------------------------------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/archive/product-doctrine.md`      | Superseded archive | Superseded by `product-doctrine-shortened.md`.                                                                                            |
| `docs/archive/schema-rewrite-prompt.md` | Superseded archive | Implementation prompt, not canonical doc. Superseded by current schema baseline and `docs/decisions/library-preparation-substrate-v1.md`. |

## Support maps

| Doc                                  | Role                      | Notes                                                                                                   |
| ------------------------------------ | ------------------------- | ------------------------------------------------------------------------------------------------------- |
| `docs/implementation-structure.md`   | Support map               | Future implementation target shape. Not architecture authority. Guides crate/package/domain structure.  |
| `docs/local-source-icon-doctrine.md` | Icon doctrine             | Canonical for iconography rules. Depends on scan admission contract for root classification vocabulary. |
| `docs/first-slice-substrate-map.md`  | First slice authority map | Defines the first product slice. Not architecture authority.                                            |
| `docs/source-hierarchy-contract.md`  | Source hierarchy contract | Defines hierarchy read boundary, pagination, coverage, and renderer cache permissions.                  |

## Schema and substrate decisions

| Doc                                                      | Role                                 | Notes                                                                                 |
| -------------------------------------------------------- | ------------------------------------ | ------------------------------------------------------------------------------------- |
| `docs/decisions/library-preparation-substrate-v1.md`     | Canonical schema decision (ACCEPTED) | v1 substrate schema authority. Replaces `LibraryAssets`-based schema.                 |
| `docs/decisions/library-boundary-exposure.md`            | Valid architectural decision         | IPC boundary exposure rules for library operations.                                   |
| `docs/decisions/library-to-deck-performance-boundary.md` | Future architecture boundary         | FUTURE ARCHITECTURE. Authority boundaries valid; table names predate v1 substrate.    |
| `docs/decisions/media-role-classification.md`            | Future architecture boundary         | FUTURE ARCHITECTURE. Classification pipeline valid; table names predate v1 substrate. |
| `docs/decisions/work-scheduling.md`                      | Future architecture boundary         | FUTURE ARCHITECTURE. Scheduler design valid; table names predate v1 substrate.        |

## Workspace docs

Self-contained under `docs/workspace/`. See individual files. The primary entry is
`docs/workspace/visual-workspace-doctrine.md`.

## Known forward references (intentionally do not exist yet)

| Referenced name             | Used by                                    | Reason                                                                                                   |
| --------------------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| `workspace-layout-contract` | Library browser workspace surface contract | TODO: Not yet written. Workspace topology contracts exist but specific layout contract not yet separate. |
