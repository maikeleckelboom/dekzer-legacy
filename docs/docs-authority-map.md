# Dekzer Documentation Authority Map

This map names the active authority role of the current documentation tree. One concept has one canonical owner.
Implementation companions may describe narrower implemented shapes but do not override their canonical owner.

## Product And Roadmap

| Doc                                                       | Role                     | Ownership                                                                                                                                                             |
| --------------------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/product/product-doctrine.md`                        | canonical owner          | Product thesis, trust laws, identity/evidence separation, readiness doctrine, workflow-filter language, spatial performance memory, and substrate admission pressure. |
| `docs/library/prepared-room-canonical-foundations.md`     | canonical owner          | Prepared Room domain objects, five-layer performance stack, room-specific laws, and substrate constraints required by that future domain.                             |
| `docs/library/product-roadmap-and-substrate-authority.md` | implementation companion | Current implementation checkpoint, dependency order, vetoes, and long-term substrate sequence. Layer contracts remain authoritative for implementation detail.        |
| `docs/first-slice-substrate-map.md`                       | implementation companion | First-slice ownership and implemented boundary map.                                                                                                                   |
| `docs/substrate-implementation-discipline.md`             | implementation companion | Reusable delivery discipline for substrate changes.                                                                                                                   |
| `docs/implementation-structure.md`                        | future architecture      | Target package/domain structure; not current behavior authority.                                                                                                      |

## Source, Scan, And Lifecycle

| Doc                                                         | Role                     | Ownership                                                                                                                |
| ----------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `docs/source-root-scan-admission-contract.md`               | canonical owner          | Root registration, scan preflight, traversal, candidate admission, work budgets, and observation persistence.            |
| `docs/decisions/source-access-and-scan-coverage.md`         | canonical owner          | Access outcomes, coverage semantics, and the rule that access failure is not empty contents.                             |
| `docs/decisions/source-locations-lifecycle-contract.md`     | canonical owner          | Source-location identity and lifecycle semantics.                                                                        |
| `docs/library/source-root-admission-policy.md`              | canonical owner          | Product admission policy, confirmation integrity, overlap behavior, and source-class decisions.                          |
| `docs/library/default-music-source-discovery.md`            | canonical owner          | First-run platform Music-folder discovery and admission handoff.                                                         |
| `docs/library/source-lifecycle-visible-state-contract.md`   | canonical owner          | User-visible source lifecycle, durable lifecycle read usage, unavailability, relocation, removal, and recovery behavior. |
| `docs/library/source-maintenance-orchestration-contract.md` | canonical owner          | Backend-owned bounded maintenance ordering and invalidation behavior.                                                    |
| `docs/library/background-root-scan-lifecycle-diagrams.md`   | implementation companion | Scan and source-lifecycle flow diagrams.                                                                                 |
| `docs/library/library-substrate-e2e-flow-diagrams.md`       | implementation companion | End-to-end substrate flow diagrams.                                                                                      |

## Library Navigation And Contents

| Doc                                                               | Role                     | Ownership                                                                                                               |
| ----------------------------------------------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `docs/library/source-hierarchy-contract.md`                       | canonical owner          | Navigation-only tree containment, selectable folders, disclosure rules, and exclusion of file/track rows from the tree. |
| `docs/decisions/library-tree-canonical-contract.md`               | canonical owner          | Canonical tree structure and ownership decisions.                                                                       |
| `docs/library/library-tree-frame-stability-contract.md`           | canonical owner          | Branch continuity, cache behavior, stable node identity, guards, prefetch bounds, and drag stability.                   |
| `docs/library/library-tree-selection-contents-contract.md`        | canonical owner          | Scope-only tree selection, separate reveal intent, contents coupling, retained rows, and refresh behavior.              |
| `docs/library/library-tree-row-action-and-dnd-scope-contract.md`  | canonical owner          | Tree row action surface, full-width selection, forgiving reveal lane, double-click prohibition, and D&D scope separation. |
| `docs/library/library-browser-workspace-surface-contract.md`      | canonical owner          | Library Browser surface identity, internal panel containment, geometry hints, and workspace handoff.                    |
| `docs/decisions/library-contents-read-boundary.md`                | canonical owner          | Parameterized contents read contract, scope, policy, depth, pagination, coverage, and omission metadata.                |
| `docs/decisions/selected-contents-scope-depth-rule.md`            | canonical owner          | Selected-scope depth semantics.                                                                                         |
| `docs/library/library-contents-browse-policy.md`                  | canonical owner          | Product workflow-filter to contents-policy mapping and empty-state semantics.                                           |
| `docs/library/contents-policy-shape.md`                           | implementation companion | Implemented policy union, file-class vocabulary, cursor identity, and policy-specific omission behavior.                |
| `docs/library/audio-browse-row-v0.md`                             | implementation companion | Implemented `audioBrowse` row shape and boundary behavior.                                                              |
| `docs/decisions/browse-policy-integrity-and-omission-metadata.md` | implementation companion | Integrity rules for implemented policy variants and omission metadata.                                                  |
| `docs/library/source-browse-order-contract.md`                    | canonical owner          | Backend-owned natural ordering for source, directory, and file browsing.                                                |

## Browse Policy, Classification, and Filter Registry

| Doc                                                       | Role            | Ownership                                                                                                                                                                      |
| --------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `docs/library/library-browse-policy-and-classification.md` | canonical owner | Eight-owner separation model (source structure, source-file facts, classification, identity/evidence, interpretation, content projection, browse policy, facet projection), built-in filter registry, complete vocabulary, CUE/sidecar rules, tree/facet behavior, row identity, and navigation readiness probe relationship. |

## File Facts, Identity, And Classification

| Doc                                                       | Role                     | Ownership                                                                                                              |
| --------------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| `docs/library/media-relevant-file-inventory-contract.md`  | canonical owner          | Raw media-relevant source inventory, admission, presence, ordering, and projection limits.                             |
| `docs/library/observed-file-facts-contract.md`            | canonical owner          | Basis-bound source-file evidence and validity.                                                                         |
| `docs/library/media-probe-observations-contract.md`       | canonical owner          | Accepted media probe observations and their evidence basis.                                                            |
| `docs/library/attachment-identity-foundation-contract.md` | canonical owner          | Exact-byte attachment identity and source-file attachment links.                                                       |
| `docs/library/primary-media-promotion-contract.md`        | canonical owner          | Evidence-backed primary-media v0 promotion.                                                                            |
| `docs/library/media-identity-schema-authority.md`         | implementation companion | Current schema/read-path ownership across inventory, attachment, primary-media, and exact-content identity layers.     |
| `docs/decisions/library-preparation-substrate-v1.md`      | canonical owner          | Accepted v1 preparation substrate schema.                                                                              |
| `docs/decisions/media-role-classification.md`             | future architecture      | Long-term classification, role, readiness, and projection lessons. Historical schema examples are not current targets. |
| `docs/decisions/work-scheduling.md`                       | future architecture      | Long-term scheduler, priority, checkpoint, lease, and cancellation model.                                              |

## Track Identity Decisions

| Doc                                                          | Role            | Ownership                                                                                |
| ------------------------------------------------------------ | --------------- | ---------------------------------------------------------------------------------------- |
| `docs/library/track-identity-candidate-contract.md`          | canonical owner | Exact-current-evidence track identity candidate production and provenance.               |
| `docs/library/track-identity-decision-contract.md`           | canonical owner | Durable candidate decisions, evidence snapshots, source scope, and effective precedence. |
| `docs/library/track-identity-decision-authority-contract.md` | canonical owner | Explicit backend-owned accept/reject/defer commands and user/system authority.           |
| `docs/library/track-identity-review-candidates-contract.md`  | canonical owner | Candidate-centered review read model and backend-derived review state.                   |

## Desktop Boundary

| Doc                                                      | Role                | Ownership                                                                                                                           |
| -------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `docs/decisions/electron-boundary-spine.md`              | canonical owner     | Electron command/publication/resource planes, exposure classes, host failures, validation hooks, and Boundary Event Pump ownership. |
| `docs/library/library-boundary-event-stream-contract.md` | canonical owner     | Rust event-ring payloads, ordering, cursor semantics, event families, gaps, and scan-event limitations.                             |
| `docs/decisions/library-folder-structure.md`             | canonical owner     | Canonical Electron library folder structure, boundary spine ownership, layer ownership laws, naming conventions, and channel ownership. |
| `docs/decisions/library-to-deck-performance-boundary.md` | future architecture | Library-to-deck authority boundary; historical table vocabulary is not current schema authority.                                    |

## Migrations

Migration documents are temporary companions. They describe one-time mechanical moves and will be removed after migration lands.

| Doc                                                | Role                | Ownership                                                                                             |
| -------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------- |
| `docs/migrations/library-folder-structure-migration.md` | migration companion | Temporary mechanical migration plan from legacy Electron library folder layout to canonical structure. This document will be removed after the migration lands and is not ongoing product authority. |

## Supporting Product Surfaces

| Doc                                  | Role              | Ownership                              |
| ------------------------------------ | ----------------- | -------------------------------------- |
| `docs/local-source-icon-doctrine.md` | canonical owner   | Local-source and media icon semantics. |
| `docs/renderer/icon-inventory.md`    | research/evidence | Current renderer icon usage inventory. |

## Workspace

Workspace documentation is self-contained and unchanged by library authority cleanup.

| Doc                                                              | Role                     |
| ---------------------------------------------------------------- | ------------------------ |
| `docs/workspace/visual-workspace-doctrine.md`                    | canonical owner          |
| `docs/workspace/adr-workspace-floating-docking-law.md`           | canonical owner          |
| `docs/workspace/canonical-topology-families.md`                  | canonical owner          |
| `docs/workspace/responsive-topology-matrix.md`                   | implementation companion |
| `docs/workspace/scale-and-settings-architecture.md`              | canonical owner          |
| `docs/workspace/topology-modeling-law.md`                        | canonical owner          |
| `docs/workspace/workspace-engine-foundation-spec.md`             | canonical owner          |
| `docs/workspace/workspace-topology-negotiation-law.md`           | canonical owner          |
| `docs/workspace/workspace-topology-negotiation-solver-sketch.md` | implementation companion |
