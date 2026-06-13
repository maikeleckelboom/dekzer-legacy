---
status: accepted
last-reviewed: 2026-06-11
owner: library-boundary-service
canonical-context:
  - source-access-and-scan-coverage
  - lifecycle-visible-state-contract
  - maintenance-orchestration-contract
  - media-relevant-file-inventory-contract
  - observed-file-observations-contract
  - attachment-identity-contract
  - playable-media-promotion-contract
  - track-identity-candidate-contract
  - track-identity-decision-contract
scope:
  - source-integrity-read-model
  - collection-health-v0-source-row
  - read-only-health-facets
---

# Source Integrity Read Model Contract

## Purpose

The source integrity read model is the V0 backend-owned collection health row. It is source-scoped first and composes
existing source lifecycle, scan coverage, source-file inventory, attachment identity, and source maintenance snapshot
observations. It does not create a new authority model.

The read answers whether a source and its substrate evidence can be trusted without collapsing separate concerns into a
single healthy/unhealthy boolean.

## Boundary

The public read is:

| Layer                   | Read                                                        |
| ----------------------- | ----------------------------------------------------------- |
| Rust protocol           | `SnapshotRead.ReadSourceIntegrity`                          |
| Generated TS contract   | `readSourceIntegrity`                                       |
| Boundary client         | `client.readSourceIntegrity({ sourceId })`                  |

The read is read-only. It must not hash files, probe files, materialize attachments, promote playable media, produce
track identity candidates, create decisions, mutate maintenance runtime state, publish invalidations, or request a
scheduler run.

## Facets

The reply is source-scoped and facet-separated:

- `sourceAvailability`: source-not-found vs known lifecycle state, including typed maintenance-compatible source
  failure when lifecycle prevents trustworthy evidence reads.
- `coverageIntegrity`: whole-source coverage state using the same coverage vocabulary as contents reads. Complete empty
  is authoritative only when coverage proves the source was accessible and fully scanned.
- `inventory`: source-file counts by presence state, file class, file kind, plus media-relevant inventory counts. These
  counts do not claim playability, artwork role, preparation readiness, or track identity.
- `evidenceAndMaintenance`: remaining BLAKE3, media probe, playable-media promotion, track identity candidate production,
  and track identity decision production candidates from source maintenance snapshot logic.
- `attachmentIntegrity`: current, stale, and missing attachment-link counts from exact-byte/source-occurrence evidence.
- `runtimeMaintenance`: in-memory idle/running state and last bounded run summary from the current service instance.

## Authority Rules

Source lifecycle remains owned by `readSourceLifecycle` and source lifecycle storage. Coverage remains scan/source
directory state. Inventory remains `source_files`. Attachment integrity remains the attachment identity read model.
Evidence backlog and runtime maintenance state remain source maintenance snapshot behavior.

This read may aggregate these observations, but must not duplicate their semantics or make a second durable state model.

## Coverage Count Semantics

`coverageIntegrity.totalDirectoriesCount` counts source directory rows that participate in source integrity coverage:
present rows plus known missing rows. `coverageIntegrity.missingDirectoriesCount` counts known directory rows where
`source_directories.presence_state = 'missing'`.

Known missing descendant directories make whole-source coverage `incomplete` and make
`emptyResultAuthoritative = false`, unless source/root-level lifecycle evidence already produces
`sourceUnavailable` or `locationMissing`. Blocked, failed, scanning, and pending directory coverage keep their existing
precedence over descendant missing-directory coverage.

Removed directory rows are not counted in Source Integrity V0 coverage. Current substrate contracts establish missing
directories as successful absence observations for coverage, but do not yet prove that removed directory rows participate
in health coverage.

## Collection Aggregate

No collection-level aggregate is introduced in V0. A correct aggregate would be only a pure rollup over source-scoped
rows. The current slice lands the source-scoped row first so aggregate semantics cannot drift into a second health
authority.

## Non-Goals

This read model does not expose UI, cleanup actions, duplicate or relocation product surfaces, canonical track identity,
CUE parsing or pairing, search/filter behavior, renderer-owned health state, durable scheduler state, local filesystem
paths, or preparation readiness.
