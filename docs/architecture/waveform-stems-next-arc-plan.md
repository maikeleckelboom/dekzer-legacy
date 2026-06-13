---
status: proposal
owner: product-architecture
review: required-before-implementation
---

# Waveform Stems Next Arc Plan

This is the current-branch prompt order after the baseline discovery pass. It assumes no implementation work happens
until human review approves the relevant proposal docs.

## 1. Human review of this discovery/decision output

Prerequisites: This docs-only package is complete.

Touched layers: Documentation only.

Explicit non-goals: No migrations, contracts, runtime code, renderer code, analyzer code, or delivery protocol.

Validation command: `pnpm run format:check`

## 2. Contract freeze docs if review approves

Prerequisites: Human review resolves target vocabulary and work/artifact substrate direction.

Touched layers: Documentation only.

Explicit non-goals: No generated output, no Rust protocol, no SQLite migration.

Validation command: `pnpm run format:check`

## 3. Isolated waveform-column-v1 spike using 52/40 high-detail schemas

Prerequisites: Review-approved column-spike scope and placeholder target vocabulary.

Touched layers: Isolated spike docs or scratch-only spike files if explicitly approved.

Explicit non-goals: No migrations, no generated contracts, no analyzer integration, no renderer integration, no delivery
protocol.

Validation command: `pnpm run format:check`

## 4. Human review of spike

Prerequisites: Spike output and measurements are available.

Touched layers: Documentation/review only.

Explicit non-goals: No productionization and no schema changes.

Validation command: `pnpm run format:check`

## 5. Store schema prompt only after work/artifact substrate decision is accepted by human review

Prerequisites: Human review accepts extending current `work_*` or explicitly selects another option.

Touched layers: SQLite migration, Rust schema validation, Rust domain enums, store authority tests.

Explicit non-goals: No generated boundary commands, no scheduler runtime, no analyzer, no renderer UI, no stem playback.

Validation command: `cargo test -p library-store-sqlite`

## 6. Migration smoke test prompt

Prerequisites: Store schema prompt lands and schema validation compiles.

Touched layers: SQLite baseline install/status tests and migration smoke coverage.

Explicit non-goals: No new product behavior and no renderer changes.

Validation command: `cargo test -p library-store-sqlite`

## 7. Scheduler/resource-governor prompt

Prerequisites: Reviewed work schema additions for scheduler fields, leases, and resource groups.

Touched layers: Rust store authority, scheduler/service internals, focused tests.

Explicit non-goals: No analyzer algorithm, no waveform tile format, no renderer delivery.

Validation command: `cargo test -p library-store-sqlite -p library-boundary-service`

## 8. Artifact DAG/basis prompt

Prerequisites: Artifact dependency/state/supersession schema exists and analysis basis contract is reviewed.

Touched layers: Rust store authority for artifact dependencies, basis validation, supersession/current queries.

Explicit non-goals: No waveform analyzer, no stem playback, no custom protocol.

Validation command: `cargo test -p library-store-sqlite`

## 9. Tile store prompt

Prerequisites: `waveform-column-v1`, `waveform-tile-v1`, and artifact manifest decisions are reviewed.

Touched layers: Rust store artifact/file-store authority and tests.

Explicit non-goals: No renderer drawing, no analyzer algorithm, no delivery protocol.

Validation command: `cargo test -p library-store-sqlite`

## 10. Delivery protocol prompt

Prerequisites: Tile storage shape exists and `waveform-delivery-v1` is reviewed.

Touched layers: Electron main/preload/shared boundary, resource delivery plumbing, focused tests.

Explicit non-goals: No analyzer, no renderer waveform UI, no SharedArrayBuffer requirement, no JSON IPC tile payloads.

Validation command: `pnpm run desktop:test`

## 11. Analyzer prompt

Prerequisites: Scheduler/resource governor, artifact DAG/basis, tile store, and target vocabulary are implemented and
validated.

Touched layers: Rust analyzer adapter, work runner integration, artifact writer tests.

Explicit non-goals: No renderer UI, no stem playback, no CUE subrange analysis unless separately contracted.

Validation command: `cargo test --workspace`

## 12. Renderer scaffold prompt

Prerequisites: Delivery protocol can serve real or fixture waveform tiles.

Touched layers: Renderer state/projection, worker fetch path, canvas/WebGPU fallback decision if reviewed.

Explicit non-goals: No analyzer changes, no schema changes, no binary payloads over normal JSON IPC.

Validation command: `pnpm run desktop:test`

## 13. Audio page handoff prototype prompt

Prerequisites: Realtime handoff proposal reviewed and current runtime substrate exists.

Touched layers: Runtime/audio prototype surfaces only as explicitly scoped by review.

Explicit non-goals: No claims that Seqlok/hot-plane infrastructure exists unless implemented first, no production deck
runtime.

Validation command: Prompt-specific prototype validation plus `pnpm run desktop:test`

## 14. Stem playback bundle prompt

Prerequisites: Stem alignment spike reviewed, component-role vocabulary reviewed, waveform/stem artifact dependency
rules exist, and playback bundle contract is written.

Touched layers: Store artifact dependencies, playback bundle boundary, focused runtime tests.

Explicit non-goals: No durable component table without a reviewed contract, no canonical track identity, no renderer-only
stem authority.

Validation command: `cargo test --workspace && pnpm run desktop:test`
