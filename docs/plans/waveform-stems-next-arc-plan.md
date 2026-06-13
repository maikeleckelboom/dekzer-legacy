# Waveform and Stems Next Arc Plan

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define the current-branch sequence for turning waveform/stem architecture into implementation-safe work.

Baseline: dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`.

## Rule

Do not skip gates. Each prompt requires human review before the next prompt.

## Gate 0 — Docs intake and library/workspace contract alignment

Prerequisites:

- proposal source docs available locally,
- current library/workspace authority docs reviewed.

Touched layers:

- docs only.

Non-goals:

- no code,
- no migrations,
- no generated contracts,
- no renderer, IPC, analyzer, audio, or waveform implementation.

Validation:

- proposal docs carry visible status,
- library/workspace authority ambiguity patched,
- stale vocabulary and proposal-status searches reviewed.

Acceptance:

- applied library surface contract exists,
- waveform/runtime docs are intake proposals only,
- later gates remain later work.

## Gate 1 — Human review of proposal docs

Prerequisites:

- proposed docs added or staged locally,
- no implementation yet.

Touched layers:

- docs only.

Non-goals:

- no migrations,
- no generated contracts,
- no renderer code,
- no analyzer code,
- no audio engine code.

Validation:

- markdown formatting if repo has a check,
- stale vocabulary search.

Acceptance:

- current vocabulary accepted,
- target model accepted,
- work/artifact substrate direction accepted,
- binary format proposal accepted for isolated spike.

## Gate 2 — Doc cleanup plan review

Prerequisites:

- docs inventory complete,
- target structure agreed.

Touched layers:

- docs only.

Non-goals:

- no moving files yet.

Validation:

- inventory reviewed manually.

Acceptance:

- folder count and path convention accepted,
- stale docs classification accepted.

## Gate 3 — Isolated waveform-column-v1 spike

Prerequisites:

- `waveform-column-v1.md` accepted as proposal.

Touched layers:

- isolated spike/test harness, outside app integration.

Non-goals:

- no store schema,
- no renderer integration,
- no IPC/delivery,
- no app UI.

Validation:

- deterministic binary hashes,
- fixture manifest,
- artifact size report,
- render validation report.

Acceptance:

- 52/40 byte schemas proven,
- LOD4 cache/on-demand position confirmed,
- Canvas2D/WebGPU recommendation sharpened.

## Gate 4 — Work/artifact substrate implementation plan

Prerequisites:

- analysis artifact substrate proposal accepted,
- spike does not change artifact assumptions.

Touched layers:

- store schema plan,
- migration plan,
- boundary plan.

Non-goals:

- no large implementation in planning prompt.

Validation:

- schema diff reviewed,
- no duplicate authority.

Acceptance:

- current `work_*` extension or alternative accepted,
- migration smoke test plan accepted.

## Gate 5 — Store schema and migration smoke

Prerequisites:

- Gate 4 accepted.

Touched layers:

- SQLite migrations,
- schema module,
- focused tests.

Non-goals:

- no waveform analyzer,
- no renderer.

Validation:

- migration tests,
- targeted store tests,
- relevant verify slice.

Acceptance:

- work priority/leases/dependencies/artifact manifests represented without duplicate authority.

## Gate 6 — Scheduler and resource governor

Prerequisites:

- substrate fields exist.

Touched layers:

- worker scheduling,
- resource governor scaffolding,
- focused tests.

Non-goals:

- no neural stem separation implementation.

Validation:

- queue ordering tests,
- lease recovery tests,
- progress write throttle tests.

Acceptance:

- numeric priority stable,
- enqueue order stable,
- stale leases recover,
- compute backend represented.

## Gate 7 — Artifact DAG and basis hashing

Prerequisites:

- artifact substrate exists.

Touched layers:

- basis hashing,
- dependency DAG,
- invalidation cascade tests.

Non-goals:

- no waveform payload generation yet.

Validation:

- canonical hash tests,
- dependency reverse lookup tests,
- stale/supersession tests.

Acceptance:

- parent peak hints do not invalidate stem waveforms,
- upstream artifact changes cascade efficiently.

## Gate 8 — Tile store

Prerequisites:

- column/tile specs accepted,
- artifact store exists.

Touched layers:

- artifact file store,
- manifest sidecar,
- tile validation.

Non-goals:

- no renderer delivery yet.

Validation:

- tile header byte tests,
- payload hash tests,
- manifest projection tests.

Acceptance:

- no waveform payload in SQLite,
- fixed 160-byte header,
- bounded reads possible.

## Gate 9 — Delivery protocol

Prerequisites:

- tile store exists,
- delivery doc accepted.

Touched layers:

- desktop main protocol handler,
- preload/control-plane descriptors,
- renderer worker fetch scaffold.

Non-goals:

- no full renderer implementation.

Validation:

- descriptor validation tests,
- no JSON IPC binary payload tests,
- bounded fetch tests.

Acceptance:

- custom privileged app protocol returns tile bytes safely,
- control plane returns descriptors/readiness only.

## Gate 10 — Waveform analyzer

Prerequisites:

- artifact/tile store exists,
- basis hashing exists.

Touched layers:

- analysis worker,
- DSP pipeline,
- waveform artifact writer.

Non-goals:

- no real-time audio,
- no stems unless full-mix path is stable.

Validation:

- deterministic fixture tests,
- long-track memory ceiling test,
- focused integration tests.

Acceptance:

- background generation only,
- LOD policy respected,
- no full-track hot path payloads.

## Gate 11 — Renderer scaffold

Prerequisites:

- delivery protocol and analyzer output exist.

Touched layers:

- renderer worker parser,
- tile cache,
- placeholder/fallback rendering.

Non-goals:

- no final visual polish.

Validation:

- no main-thread parse tests where practical,
- missing tile fallback test,
- 32-surface stress harness if possible.

Acceptance:

- frame path never waits for tile data,
- horizontal/vertical projections use same artifact data.

## Gate 12 — Audio page handoff prototype

Prerequisites:

- audio page handoff proposal accepted.

Touched layers:

- isolated audio runtime prototype or future engine scaffold.

Non-goals:

- no production stem playback yet.

Validation:

- zero callback locks/allocation/file/SQLite/IPC calls,
- missing page fallback,
- loop-aware switch test.

Acceptance:

- real-time handoff model proven before stem playback schema hardens.

## Gate 13 — Stem playback bundle

Prerequisites:

- audio page handoff prototype accepted,
- artifact substrate supports dependency DAG,
- stem target model accepted.

Touched layers:

- stem set/member planning,
- playback bundle artifact planning,
- alignment profile planning.

Non-goals:

- no neural separation until resource governor is real.

Validation:

- dedicated stem readiness model tests,
- alignment profile tests if implemented.

Acceptance:

- stem playback readiness and waveform readiness stay separate.
