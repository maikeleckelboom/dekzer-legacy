# Dekzer Documentation Authority Map

This map names the active authority role of the current documentation tree. One concept has one canonical owner.
Implementation companions may describe narrower implemented shapes but do not override their canonical owner.

## Product And Roadmap

| Doc                                                                                                                           | Role                     | Ownership                                                                                                                                                             |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [docs/product/product-doctrine.md](product/product-doctrine.md)                                                               | canonical owner          | V0 local DJ foundation target, product phasing, trust laws, identity/evidence separation, readiness doctrine, workflow-filter language, future spatial performance memory compatibility, and substrate admission pressure. |
| [docs/library/prepared-room/model.md](library/prepared-room/model.md)                                                         | canonical owner          | Future Prepared Room domain objects, five-layer performance stack, room-specific laws, and substrate constraints required by that future domain.                                  |
| [docs/library/roadmap/product-roadmap-and-substrate-authority.md](library/roadmap/product-roadmap-and-substrate-authority.md) | implementation companion | Current implementation checkpoint, V0 product phasing, dependency order, vetoes, and long-term substrate sequence. Layer contracts remain authoritative for implementation detail. |
| [docs/product/first-slice-substrate-map.md](product/first-slice-substrate-map.md)                                             | implementation companion | First-slice ownership and implemented boundary map.                                                                                                                   |
| [docs/product/source-activation-and-navigation-readiness.md](product/source-activation-and-navigation-readiness.md)           | canonical owner          | Source activation readiness, child readiness, contents readiness labels, no-false-empty states, retained browser state, and activated browser-flow audit criteria.    |
| [docs/decisions/substrate-implementation-discipline.md](decisions/substrate-implementation-discipline.md)                     | implementation companion | Reusable delivery discipline for substrate changes.                                                                                                                   |
| [docs/decisions/implementation-structure.md](decisions/implementation-structure.md)                                           | future architecture      | Target package/domain structure; not current behavior authority.                                                                                                      |

## Source, Scan, And Lifecycle

| Doc                                                                                                                         | Role                     | Ownership                                                                                                                |
| --------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| [docs/library/source/root-scan-admission-contract.md](library/source/root-scan-admission-contract.md)                       | canonical owner          | Root registration, scan preflight, traversal, candidate admission, work budgets, and observation persistence.            |
| [docs/decisions/source-access-and-scan-coverage.md](decisions/source-access-and-scan-coverage.md)                           | canonical owner          | Access outcomes, coverage semantics, and the rule that access failure is not empty contents.                             |
| [docs/decisions/source-locations-lifecycle-contract.md](decisions/source-locations-lifecycle-contract.md)                   | canonical owner          | Source-location identity and lifecycle semantics.                                                                        |
| [docs/library/source/root-admission-policy.md](library/source/root-admission-policy.md)                                     | canonical owner          | Product admission policy, confirmation integrity, overlap behavior, and source-class decisions.                          |
| [docs/library/source/local-browse-entry-points-contract.md](library/source/local-browse-entry-points-contract.md)         | canonical owner          | Local browse entry point kinds, identity, status, available backend actions, admission action, and the entry-point browse-versus-scan boundary. |
| [docs/library/source/local-browse-items-contract.md](library/source/local-browse-items-contract.md)     | canonical owner          | Local browse item kinds, item/window identity, bounded item read windowing, available backend actions, admission action, and renderer projection distinction. |
| [docs/library/source/default-music-source-discovery.md](library/source/default-music-source-discovery.md)                   | implementation companion | Platform Music folder implementation companion for the `music` local browse entry point and default Music admission action. |
| [docs/library/source/lifecycle-visible-state-contract.md](library/source/lifecycle-visible-state-contract.md)               | canonical owner          | User-visible source lifecycle, durable lifecycle read usage, unavailability, relocation, removal, and recovery behavior. |
| [docs/library/source/maintenance-orchestration-contract.md](library/source/maintenance-orchestration-contract.md)           | canonical owner          | Backend-owned bounded maintenance ordering and invalidation behavior.                                                    |
| [docs/library/health/source-integrity-read-model-contract.md](library/health/source-integrity-read-model-contract.md)       | implementation companion | Source-scoped collection health/source integrity read model shape and read-only facet composition.                       |
| [docs/library/source/background-root-scan-lifecycle-diagrams.md](library/source/background-root-scan-lifecycle-diagrams.md) | implementation companion | Scan and source-lifecycle flow diagrams.                                                                                 |
| [docs/library/source/substrate-e2e-flow-diagrams.md](library/source/substrate-e2e-flow-diagrams.md)                         | implementation companion | End-to-end substrate flow diagrams.                                                                                      |

## Library Navigation And Contents

| Doc                                                                                                                           | Role                     | Ownership                                                                                                                                                                                      |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [docs/library/tree/source-hierarchy-contract.md](library/tree/source-hierarchy-contract.md)                                   | canonical owner          | Navigation-only tree containment, selectable folders, child-browse-scope disclosure law, false-disclosure rejection, and exclusion of file/track rows from the tree.                           |
| [docs/library/tree/tree-contract.md](library/tree/tree-contract.md)                                                           | canonical owner          | Canonical tree structure and ownership decisions.                                                                                                                                              |
| [docs/library/tree/frame-stability-contract.md](library/tree/frame-stability-contract.md)                                     | canonical owner          | Branch continuity, cache behavior, stable node identity, guards, prefetch bounds, and drag stability.                                                                                          |
| [docs/library/tree/selection-contents-contract.md](library/tree/selection-contents-contract.md)                               | canonical owner          | Scope-only tree selection, separate reveal intent, contents coupling, retained rows, and refresh behavior.                                                                                     |
| [docs/library/tree/row-action-and-dnd-scope-contract.md](library/tree/row-action-and-dnd-scope-contract.md)                   | canonical owner          | Tree row action surface, full-width selection, forgiving reveal lane, double-click prohibition, and D&D scope separation.                                                                      |
| [docs/library/boundary/browser-workspace-surface-contract.md](library/boundary/browser-workspace-surface-contract.md)         | canonical owner          | Library Workspace surface identity, internal panel containment, geometry hints, and workspace handoff.                                                                                          |
| [docs/library/contents/read-boundary-contract.md](library/contents/read-boundary-contract.md)                                 | canonical owner          | Parameterized contents read contract, request identity, selected scope, policy/filter identity, depth, pagination, coverage, and omission metadata.                                            |
| [docs/library/contents/selected-scope-depth-rule.md](library/contents/selected-scope-depth-rule.md)                           | canonical owner          | Selected-scope depth semantics and the identity split between immediate navigation and recursive selected contents.                                                                            |
| [docs/library/contents/browse-policy.md](library/contents/browse-policy.md)                                                   | implementation companion | Current product-filter to implemented backend contents policy mapping, current renderer/backend fallback state, omission metadata mapping, and empty-state semantics for implemented policies. |
| [docs/library/contents/policy-implementation.md](library/contents/policy-implementation.md)                                   | implementation companion | Implemented policy union, file-class vocabulary, cursor identity, and policy-specific omission behavior.                                                                                       |
| [docs/library/contents/audio-browse-row-implementation.md](library/contents/audio-browse-row-implementation.md)               | implementation companion | Implemented `audioBrowse` row shape and boundary behavior.                                                                                                                                     |
| [docs/decisions/browse-policy-integrity-and-omission-metadata.md](decisions/browse-policy-integrity-and-omission-metadata.md) | implementation companion | Integrity rules for implemented policy variants and omission metadata.                                                                                                                         |
| [docs/library/source/browse-order-contract.md](library/source/browse-order-contract.md)                                       | canonical owner          | Backend-owned natural ordering for source, directory, and file browsing.                                                                                                                       |

## Library Browse Representation

| Doc                                                                                               | Role            | Ownership                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [docs/library/browse/policy-and-classification.md](library/browse/policy-and-classification.md)   | canonical owner | Product/domain browse policy, built-in filter registry and activation contract, classification vocabulary, interpretation links, row universes, facets, row identity, filter-aware content/facet projection behavior, filter-agnostic tree containment constraints, CUE/companion-file rules, and navigation readiness probe independence. |
| [docs/library/browse/representation-contract.md](library/browse/representation-contract.md)       | canonical owner | Library browse representation model, representation classes, ownership and provenance, capability model, repeated-material semantics, and ordering ownership matrix.                                                                                                                                                                       |
| [docs/library/browse/representation-composition.md](library/browse/representation-composition.md) | canonical owner | Library representation composition, realization forms, panel instances, composition shells, topology placement, and the rule that implementations must not hard-code sidebar-only composition.                                                                                                                                             |

## File Facts, Identity, And Classification

| Doc                                                                                                                           | Role                     | Ownership                                                                                                              |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| [docs/library/evidence/media-relevant-file-inventory-contract.md](library/evidence/media-relevant-file-inventory-contract.md) | canonical owner          | Raw media-relevant source inventory, admission, presence, ordering, and projection limits.                             |
| [docs/library/evidence/observed-file-facts-contract.md](library/evidence/observed-file-facts-contract.md)                     | canonical owner          | Basis-bound source-file evidence and validity.                                                                         |
| [docs/library/evidence/media-probe-observations-contract.md](library/evidence/media-probe-observations-contract.md)           | canonical owner          | Accepted media probe observations and their evidence basis.                                                            |
| [docs/library/evidence/attachment-identity-contract.md](library/evidence/attachment-identity-contract.md)                     | canonical owner          | Exact-byte attachment identity and source-file attachment links.                                                       |
| [docs/library/evidence/attachment-occurrence-model-readiness.md](library/evidence/attachment-occurrence-model-readiness.md)   | implementation companion | A-5 attachment occurrence evidence readiness, ownership map, read boundary, schema stance, and non-goals.              |
| [docs/library/evidence/primary-media-promotion-contract.md](library/evidence/primary-media-promotion-contract.md)             | canonical owner          | Evidence-backed primary-media v0 promotion.                                                                            |
| [docs/library/evidence/media-identity-schema-authority.md](library/evidence/media-identity-schema-authority.md)               | implementation companion | Current schema/read-path ownership across inventory, attachment, primary-media, exact-content identity candidates, and candidate decisions. |
| [docs/decisions/library-preparation-substrate.md](decisions/library-preparation-substrate.md)                                 | future architecture      | Historical preparation/work/readiness architecture notes only. Not current schema authority and not an implementation target. Future preparation, analysis, and waveform work must be re-contracted from the post-deletion substrate. |
| [docs/decisions/media-role-classification.md](decisions/media-role-classification.md)                                         | future architecture      | Long-term classification, role, readiness, and projection lessons. Historical schema examples are not current targets. |
| [docs/decisions/work-scheduling.md](decisions/work-scheduling.md)                                                             | future architecture      | Long-term scheduler, priority, checkpoint, lease, and cancellation model.                                              |

## User Decision Pattern

| Doc                                                                                       | Role            | Ownership                                                                                                                                 |
| ----------------------------------------------------------------------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| [docs/library/user-decision-pattern-contract.md](library/user-decision-pattern-contract.md) | canonical owner | A-6 evidence/candidate/decision/projection separation, reusable decision verbs, target identity, provenance, recompute, and conflict law. |

## Track Identity Decisions

| Doc                                                                                                                                   | Role            | Ownership                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------- |
| [docs/library/identity/track-identity-candidate-contract.md](library/identity/track-identity-candidate-contract.md)                   | canonical owner | Exact-current-evidence track identity candidate production and provenance.               |
| [docs/library/identity/track-identity-decision-contract.md](library/identity/track-identity-decision-contract.md)                     | canonical owner | Durable candidate decisions, evidence snapshots, source scope, and effective precedence. |
| [docs/library/identity/track-identity-decision-authority-contract.md](library/identity/track-identity-decision-authority-contract.md) | canonical owner | Explicit backend-owned accept/reject/defer commands and user/system authority.           |
| [docs/library/identity/track-identity-review-candidates-contract.md](library/identity/track-identity-review-candidates-contract.md)   | canonical owner | Candidate-centered review read model and backend-derived review state.                   |

## Desktop Boundary

| Doc                                                                                                         | Role                | Ownership                                                                                                                               |
| ----------------------------------------------------------------------------------------------------------- | ------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| [docs/decisions/electron-boundary-spine.md](decisions/electron-boundary-spine.md)                           | canonical owner     | Electron command/publication/resource planes, exposure classes, host failures, validation hooks, and Boundary Event Pump ownership.     |
| [docs/library/boundary/event-stream-contract.md](library/boundary/event-stream-contract.md)                 | canonical owner     | Rust event-ring payloads, ordering, cursor semantics, event families, gaps, and scan-event limitations.                                 |
| [docs/decisions/library-folder-structure.md](decisions/library-folder-structure.md)                         | canonical owner     | Canonical Electron library folder structure, boundary spine ownership, layer ownership laws, naming conventions, and channel ownership. |
| [docs/decisions/library-to-deck-performance-boundary.md](decisions/library-to-deck-performance-boundary.md) | future architecture | Library-to-deck authority boundary; historical table vocabulary is not current schema authority.                                        |

## Migrations

Migration documents are temporary companions. They describe one-time mechanical moves and will be removed after migration lands.

| Doc                                                                                                       | Role                | Ownership                                                                                                                                                                                            |
| --------------------------------------------------------------------------------------------------------- | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [docs/migrations/library-folder-structure-migration.md](migrations/library-folder-structure-migration.md) | migration companion | Temporary mechanical migration plan from legacy Electron library folder layout to canonical structure. This document will be removed after the migration lands and is not ongoing product authority. |

## Supporting Product Surfaces

| Doc                                                                                 | Role              | Ownership                              |
| ----------------------------------------------------------------------------------- | ----------------- | -------------------------------------- |
| [docs/product/local-source-icon-doctrine.md](product/local-source-icon-doctrine.md) | canonical owner   | Local-source and media icon semantics. |
| [docs/renderer/icon-inventory.md](renderer/icon-inventory.md)                       | research/evidence | Current renderer icon usage inventory. |

## Workspace

Workspace documentation is self-contained. Filename normalization does not change workspace authority.

| Doc                                                                                                     | Role                     |
| ------------------------------------------------------------------------------------------------------- | ------------------------ |
| [docs/workspace/visual-workspace-doctrine.md](workspace/visual-workspace-doctrine.md)                   | canonical owner          |
| [docs/workspace/floating-docking-law.md](workspace/floating-docking-law.md)                             | canonical owner          |
| [docs/workspace/topology-families.md](workspace/topology-families.md)                                   | canonical owner          |
| [docs/workspace/responsive-topology-matrix.md](workspace/responsive-topology-matrix.md)                 | implementation companion |
| [docs/workspace/scale-and-settings-architecture.md](workspace/scale-and-settings-architecture.md)       | canonical owner          |
| [docs/workspace/topology-modeling-law.md](workspace/topology-modeling-law.md)                           | canonical owner          |
| [docs/workspace/engine-spec.md](workspace/engine-spec.md)                                               | canonical owner          |
| [docs/workspace/topology-negotiation-law.md](workspace/topology-negotiation-law.md)                     | canonical owner          |
| [docs/workspace/topology-negotiation-solver-sketch.md](workspace/topology-negotiation-solver-sketch.md) | implementation companion |
