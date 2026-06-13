# Waveform Rendering Runtime V1

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define the renderer-side runtime contract for displaying waveform artifacts without skipping frames, even under future 8-deck, 4-stem-per-deck workloads.

## Stress Target

Worst-case visual workload to keep as a stress target, not the first acceptance bar:

- 8 active decks,
- 4 stem lanes per deck,
- 32 concurrent stem waveform surfaces,
- plus optional full-mix/overview/inspector surfaces,
- 60 Hz minimum, 120 Hz desirable on capable hardware.

The renderer must degrade detail before it stalls frames.

## Frame path laws

During a frame, the renderer must not wait for:

- backend IPC,
- app protocol fetch,
- SQLite,
- file IO,
- tile parsing,
- GPU upload completion,
- analysis completion.

If a tile is missing:

1. render cached tile, lower LOD, previous image, gap, or placeholder,
2. enqueue asynchronous descriptor/fetch request,
3. continue the frame.

## Main thread role

The main renderer thread owns:

- layout,
- input,
- surface registration,
- cheap viewport state,
- presentation composition decisions.

It must not:

- parse binary waveform columns,
- draw dense waveform columns directly,
- upload GPU buffers,
- read waveform tile bytes,
- block on tile fetches.

## Worker topology

### Production target

The production target is one waveform GPU worker or a small bounded GPU worker topology that owns:

- GPU device/context where supported,
- tile decode/parse into GPU-friendly buffers,
- upload budget,
- draw budget,
- surface state table,
- LOD transition state,
- cache miss reporting.

This should render all waveform surfaces through one coordinated frame scheduler, not independent worker storms per surface.

### Canvas2D V0 Scaffold

Canvas2D retained dirty-strip rendering is the V0 scaffold path before the GPU worker target is accepted. It must use
retained dirty-strip rendering for steady scrolling:

- scroll existing raster with drawImage or equivalent,
- draw only newly exposed strip,
- full redraw only on resize, zoom change, theme/color change, hard seek outside buffer, or LOD transition.

Do not create one worker per waveform surface. Use a small worker pool or atlas/layer strategy.

## LOD selection

LOD is chosen from viewport scale:

- LOD0: tiny previews and very long overviews,
- LOD1: full-track overview strips,
- LOD2: standard deck waveform,
- LOD3: performance zoom/high-detail deck view,
- LOD4: sample-close/cue edit, on-demand/cache.

High-detail microtrace rendering requires enough pixel width per column. If columns are subpixel, use rect/envelope fallback. Do not force splines into subpixel channels.

## LOD transition pins

LOD transitions may temporarily need old and new tiles. Tile cache must distinguish:

- visible pins,
- transition pins,
- edit pins,
- prefetch pins.

Under memory pressure:

1. cancel or shorten crossfade,
2. drop microtrace on background/far surfaces,
3. fall back to rect mode,
4. drop non-visible prefetch,
5. never block the frame.

## Tile cache hierarchy

Renderer cache hierarchy:

- raw fetched tile bytes,
- decoded structure-of-arrays columns,
- GPU buffers or retained raster strips.

Cache key:

- artifact id,
- basis hash,
- schema key,
- LOD,
- tile index,
- renderer decode version.

Do not write SQLite `last_accessed_at` per frame. Runtime recency is in memory. Persistence is coarse and only for eviction/diagnostics.

## Surface context

Renderer surface context provides:

- surface id,
- target kind/id,
- component kind,
- stem role if any,
- orientation,
- color token,
- amplitude policy,
- quality policy.

Artifacts do not contain stem role colors. Stems use renderer/theme color plus artifact geometry/transient facts.

## Audio clock

Waveform rendering follows audio transport snapshots. It must not query playback through IPC every frame.

Future implementation should use a shared atomic/hot-plane snapshot if available. If no such substrate exists in Dekzer, do not claim one. The interface should still model:

- deck id,
- transport epoch,
- rate-change epoch,
- parent sample position,
- host time at snapshot,
- playback rate,
- loop state version,
- playing flag.

Renderer extrapolates from the latest snapshot. If stale, visuals may lag; audio never waits.

## Validation requirements

Future stress tests should include the stress target when the production runtime is ready. First acceptance can use a
smaller visible-surface count if it still proves off-main-thread fetch/parse and retained rendering behavior.

Stress target:

- 32 visible waveform surfaces,
- cold tile cache,
- rapid seek every 250 ms for 10 seconds,
- zoom oscillation across LOD2/LOD3 boundary,
- GPU upload budget constrained,
- background waveform analysis active,
- background stem separation active/throttled,
- no main-thread binary parse,
- no known waveform-induced frame waits.
