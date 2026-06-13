---
status: proposal
owner: product-architecture
review: required-before-implementation
---

# Analysis Work Artifact Substrate Decision

Status: proposal — requires human review before implementation.

## Decision question

Future waveform, analysis, and stems need durable work scheduling and artifact provenance. The current repo already has
`work_items`, `work_runs`, and `work_artifacts`, but the schema is narrow. This decision compares extending that
substrate with introducing new analysis-specific tables.

## Current facts

- `work_items` currently supports only `source_file` and `projection_domain` subjects.
- `work_items` currently supports only `inspect_source_file` and `rebuild_projection` work kinds.
- `work_artifacts` currently supports only `inspection_result` and `projection_snapshot` artifact kinds.
- `source_file_observations.accepted_artifact_id` already points at `work_artifacts`.
- `work_artifacts` already supports inline payloads and file-store entries.
- The current scheduler behavior is narrow queue/lease behavior, not the future scheduler in
  `docs/decisions/work-scheduling.md`.
- There are no `analysis_work_items`, `analysis_work_runs`, or `analysis_artifacts` tables.

## Option 1: extend current `work_items` / `work_runs` / `work_artifacts`

| Dimension                               | Analysis                                                                                                                                                                                                              |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fit with current schema                 | Best fit. The existing tables already model work items, runs, artifacts, storage kinds, adapter provenance, basis fingerprints, and artifact claims. They need expansion, not replacement.                            |
| Migration cost                          | Moderate. Existing enum checks, indexes, and authority code must expand. Existing rows are small and current names can remain.                                                                                        |
| Naming consistency                      | Strong. Current repo vocabulary already uses `work_*` generically. Extending it avoids a second work vocabulary.                                                                                                      |
| Scheduler needs                         | Needs explicit additions for ordering, lanes/backends, lease identity, resource governors, and chunking. Current `priority_class` and `leased_until` are insufficient.                                                |
| Artifact DAG needs                      | Needs a new dependency table and producer linkage. Existing `work_artifacts` has no dependency graph.                                                                                                                 |
| Work item coalescing                    | Existing unique index on active work by subject/work/basis is a useful precedent. It must expand to target/facet/policy identity.                                                                                     |
| Output artifact linkage                 | Needs an output/provenance relation: either work item to produced artifact, run to artifact, and/or artifact producer fields. Current run-to-artifact exists but work item completion does not name a primary output. |
| Resource governor needs                 | Needs new scheduler/resource tables or fields. Current schema has no resource budgets, compute backend, chunk progress, or resource claims.                                                                           |
| Source maintenance interaction          | Good. Source maintenance already uses current work/artifact authority for accepted hash/probe evidence. Analysis can join the same artifact authority without duplicating source maintenance outputs.                 |
| Renderer boundary needs                 | Good. The generated boundary can add analysis reads and resource handles later without inventing a separate table family.                                                                                             |
| Waveform compatibility                  | Good. Waveform artifacts are work artifacts with analysis-specific artifact kinds, basis, storage manifest, dependency rows, and target keys.                                                                         |
| Future stem/stem-playback compatibility | Good if target vocabulary supports component roles and artifact dependencies. Stem bundles can depend on full-mix/audio component analysis without new work authority.                                                |
| Risk of duplicate authority             | Low if schema expansion is explicit and reviewed.                                                                                                                                                                     |
| Risk of compatibility-alias thinking    | Low. This is current vocabulary, not a compatibility alias. The risk is only widening too much in one migration.                                                                                                      |

## Option 2: introduce `analysis_work_items` / `analysis_work_runs` / `analysis_artifacts`

| Dimension                               | Analysis                                                                                                                             |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Fit with current schema                 | Weak. It duplicates an already-present generic work/artifact substrate.                                                              |
| Migration cost                          | High. New table family, new authority code, new generated boundary, and duplicate lifecycle rules would be required.                 |
| Naming consistency                      | Weak. It implies analysis work is separate from work, while source-file observations already use `work_artifacts`.                   |
| Scheduler needs                         | Still needs scheduler/resource/governor design. New tables do not remove that complexity.                                            |
| Artifact DAG needs                      | Still needs DAG and supersession tables. New names do not solve dependency modeling.                                                 |
| Work item coalescing                    | Would need a second coalescing model and duplicate idempotence rules.                                                                |
| Output artifact linkage                 | Would need separate linkage and then bridges back to existing artifacts or duplicate storage semantics.                              |
| Resource governor needs                 | Would either duplicate resource scheduling or require cross-table scheduling joins.                                                  |
| Source maintenance interaction          | Poor. Hash/probe evidence already commits through current artifacts; analysis-specific tables would create a split authority.        |
| Renderer boundary needs                 | More complex. Renderer/backend would need to understand why some artifacts are `work_artifacts` and others are `analysis_artifacts`. |
| Waveform compatibility                  | Possible but unnecessarily separate.                                                                                                 |
| Future stem/stem-playback compatibility | Possible but risks making stems a third work/artifact authority later.                                                               |
| Risk of duplicate authority             | High. The split would create two ways to represent work runs and artifact outputs.                                                   |
| Risk of compatibility-alias thinking    | High. The new names would likely become aliases around current `work_*` or force bridges from current source observations.           |

## Recommended option

Recommend Option 1: extend current `work_items` / `work_runs` / `work_artifacts` for the next implementation arc.

Rationale:

- Current artifact provenance already exists and is referenced by `source_file_observations`.
- A second analysis-specific table family would duplicate work/run/artifact authority.
- The product doctrine and implementation discipline reject compatibility aliases and dual models.
- Waveform and stems need artifact basis, storage, and dependency improvements more than they need a new table prefix.

This recommendation is proposal-level only. It does not implement the schema.

## Required future schema changes at decision level

If human review approves extending current `work_*`, the next store schema prompt should define these changes without
adding runtime analyzer or renderer behavior in the same slice.

| Area                                    | Required future decision                                                                                                                                                                                                        |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Subject/target vocabulary expansion     | Add subject/target vocabulary for `playable_media` analysis subjects, component roles, target keys, policy versions, and basis kinds. Do not use candidates or decisions as canonical artifact subjects.                        |
| `priority_order`                        | Add a deterministic scheduler ordering input or derived persisted tie-breaker. Do not store final effective priority as doctrine.                                                                                               |
| `enqueue_seq`                           | Add monotonic enqueue sequence for fair stable ordering independent of wall-clock collisions.                                                                                                                                   |
| `compute_backend`                       | Record selected backend class such as CPU, GPU, decoder, or external adapter where relevant. Keep names policy-owned and versioned.                                                                                             |
| Lease improvements                      | Add `lease_id`, `lease_owner`, and clearer `lease_until` semantics. Current `leased_until` alone cannot prevent late worker commits.                                                                                            |
| Output artifact / producer linkage      | Keep run-to-artifact linkage and add a work-item output pointer or producer relation when one output is the accepted/current artifact for a target.                                                                             |
| Artifact dependency table               | Add a dependency table such as `work_artifact_dependencies` to model tile sets, basis inputs, derived artifacts, and stem bundle dependencies.                                                                                  |
| Artifact state/supersession model       | Add artifact state and supersession edges. Do not delete history to mark currentness.                                                                                                                                           |
| Artifact format/storage manifest fields | Add format family/version, byte layout version, logical payload role, compression, byte length, tile/LOD metadata, and storage manifest fields.                                                                                 |
| Indexes                                 | Add indexes for active schedulable work by target, active work coalescing by subject/target/policy/basis, artifact lookup by subject/target/kind/state, dependency traversal, supersession/current lookup, and storage cleanup. |

## Implementation stop conditions

Stop before implementation if:

- the schema prompt tries to introduce analysis-specific table names as a duplicate authority;
- the schema prompt expands work/artifact vocabulary without a reviewed target vocabulary;
- artifact state, dependency, and basis modeling are skipped while analyzer code is added;
- the renderer boundary is asked to carry binary tile payloads over normal JSON IPC;
- a migration creates canonical track identity, prep rows, browser-row waveform fields, or source-occurrence-owned waveform fields;
- a prompt combines store schema, generated contracts, scheduler runtime, analyzer, renderer, and delivery protocol in one arc.
