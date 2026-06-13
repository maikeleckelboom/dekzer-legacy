---
status: proposal-ledger
owner: product-architecture
review: required-before-implementation
---

# Waveform Stems Deferred Proposal Ledger

This ledger records proposal documents that still need to be written after human review. It is not the full waveform,
delivery, analyzer, renderer, basis, CUE, or stem design.

## 1. waveform-column-v1

Why deferred: Column byte layout should not freeze before the current target vocabulary and work/artifact substrate
direction are reviewed.

Prerequisite decision or repo fact: `playable_media_full_mix_waveform_target_v0` vocabulary and the work/artifact
extension decision.

Expected future doc path: `docs/architecture/proposals/waveform-column-v1.md`

Implementation stop condition: Stop if the prompt would add migrations, generated contracts, analyzer code, renderer
code, or binary delivery before the column contract is reviewed.

Currently intended proposal facts:

- `mix_spectral_microtrace_v1`
- LOD0/1/2 = 32 bytes
- LOD3/4 = 52 bytes
- `stem_amp_microtrace_v1`
- LOD0/1/2 = 20 bytes
- LOD3/4 = 40 bytes
- high-detail schemas use `u16` position fields to avoid odd-offset `i16` reads
- LOD4 is on-demand/cache by default, especially for stems

## 2. waveform-tile-v1

Why deferred: Tile identity, storage, state, supersession, and dependency rules depend on the approved work/artifact
extension.

Prerequisite decision or repo fact: Artifact format/storage manifest fields and artifact dependency table decision.

Expected future doc path: `docs/architecture/proposals/waveform-tile-v1.md`

Implementation stop condition: Stop if tile rows are attached to browser rows, prep rows, source-file occurrences, or
canonical track identity.

## 3. waveform-delivery-v1

Why deferred: The repo has IPC handlers but no current custom protocol, ResourcePlane implementation, or binary resource
delivery substrate.

Prerequisite decision or repo fact: Electron boundary ResourcePlane or app protocol decision after tile storage shape is
reviewed.

Expected future doc path: `docs/architecture/proposals/waveform-delivery-v1.md`

Implementation stop condition: Stop if delivery is implemented through normal JSON IPC, if it requires SharedArrayBuffer
for tile bytes, or if it invents a non-existent Electron mmap API.

Currently intended decision:

- V1 should evaluate custom privileged app protocol + renderer worker fetch as the preferred path.
- Reject normal JSON IPC for binary tile payloads.
- Do not require SharedArrayBuffer for waveform tile bytes.
- Do not invent Electron `mmap_file_range`.

## 4. realtime-audio-page-handoff

Why deferred: No Seqlok/seqlock, hot-plane, or deck audio clock snapshot substrate exists in the current repo.

Prerequisite decision or repo fact: A reviewed runtime/audio handoff substrate and explicit deck/audio timing authority.

Expected future doc path: `docs/architecture/proposals/realtime-audio-page-handoff.md`

Implementation stop condition: Stop if the prompt claims Seqlok, hot-plane, or deck-clock snapshot infrastructure exists
without current repo evidence.

## 5. analysis-basis-hashing-v1

Why deferred: Analysis basis must be defined after target vocabulary, artifact dependencies, and currentness rules are
reviewed.

Prerequisite decision or repo fact: Work/artifact extension approval and waveform target vocabulary approval.

Expected future doc path: `docs/architecture/proposals/analysis-basis-hashing-v1.md`

Implementation stop condition: Stop if basis is just a source path, just an attachment hash, or omits decoder/policy
version and source-file observation basis.

## 6. cue-sheet-subrange-analysis-model-v1

Why deferred: Current CUE files are source-file inventory rows only. CUE parse observations and CUE-to-audio association
are not implemented.

Prerequisite decision or repo fact: Reviewed CUE parse/association contract and explicit subrange target vocabulary.

Expected future doc path: `docs/architecture/proposals/cue-sheet-subrange-analysis-model-v1.md`

Implementation stop condition: Stop if CUE proximity creates tracks, canonical identity, or waveform subranges without
an association contract.

## 7. isolated waveform-column spike

Why deferred: A spike can validate byte layouts without forcing schema, protocol, analyzer, or renderer commitments.

Prerequisite decision or repo fact: Human review agrees the spike is isolated and does not create durable product
authority.

Expected future doc path: `docs/architecture/spikes/isolated-waveform-column-spike.md`

Implementation stop condition: Stop if the spike edits migrations, generated contracts, runtime boundary commands, or
renderer UI.

## 8. WebGPU multi-surface renderer spike

Why deferred: Current repo has no WebGPU or OffscreenCanvas implementation. Renderer delivery should follow the tile and
delivery contracts.

Prerequisite decision or repo fact: Delivery contract and tile contract reviewed; current Electron/browser WebGPU
support verified in the implementation environment.

Expected future doc path: `docs/architecture/spikes/webgpu-multi-surface-renderer-spike.md`

Implementation stop condition: Stop if the spike becomes the production renderer or assumes WebGPU is present without
runtime detection and fallback strategy.

## 9. dedicated stem alignment spike

Why deferred: Stems need alignment, component roles, bundle provenance, and playback compatibility decisions that do not
exist today.

Prerequisite decision or repo fact: Future stem playback bundle contract and component-role vocabulary review.

Expected future doc path: `docs/architecture/spikes/dedicated-stem-alignment-spike.md`

Implementation stop condition: Stop if the spike creates durable component rows, stem playback code, or semantic track
identity.

## 10. streaming analysis memory ceiling spike

Why deferred: Large-file waveform/stem analysis needs empirical memory ceilings before analyzer implementation.

Prerequisite decision or repo fact: Work chunking/resource governor direction and artifact storage manifest direction.

Expected future doc path: `docs/architecture/spikes/streaming-analysis-memory-ceiling-spike.md`

Implementation stop condition: Stop if the spike writes production analyzer code, production migrations, or unbounded
decode buffers.
