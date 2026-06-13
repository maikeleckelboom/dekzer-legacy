# Realtime Audio Page Handoff

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define the proposed real-time-safe page handoff between media prefetch/decode services and the future audio callback.

This document is separate from waveform delivery. Waveform tiles are visual analysis artifacts. Audio pages are playback material. They must not share a hot path.

## Audio callback laws

The audio callback must not perform:

- SQLite reads or writes,
- filesystem access,
- allocation,
- locks,
- waiting,
- logging,
- IPC,
- JavaScript calls,
- waveform generation,
- neural stem separation,
- codec probing,
- dynamic graph construction,
- designed-in page faults.

The callback may only:

- read preallocated parameter snapshots,
- read resident PCM pages or ring buffers,
- advance deterministic transport,
- run preallocated DSP nodes,
- write output buffers,
- publish lightweight telemetry counters/atomics.

## Playback material

Future stem playback uses a real-time-safe playback bundle, not arbitrary compressed files decoded on the callback.

V1 preferred runtime format:

- planar PCM pages,
- f32 in memory,
- one page stream per active component/stem member,
- deck-local parent timebase coordinates,
- immutable page memory after publication.

Disk storage format remains a later implementation decision. Runtime pages must be resident before callback consumption.

## Page stream

A page stream belongs to one playable component:

- full mix stream, or
- stem member stream.

A deck with four active stems may have four stem page streams plus optional mix stream during transitions.

Each page has:

| Field | Meaning |
| --- | --- |
| stream id | Deck/component stream identity. |
| generation | Changes when playback material or alignment changes. |
| page index | Zero-based page number in track-local time. |
| frame start | Track-local start frame. |
| frame count | Number of valid frames. |
| channel layout | Mono/stereo/planar layout. |
| sample format | Runtime f32. |
| pointer/handle | Runtime-resident page memory. |
| state | resident, warming, missing, stale. |

Default V1 page size proposal:

    32768 frames per stream page

At 48 kHz this is about 682 ms per page. This is a proposal, not law. Human review may change it after performance
measurement.

## Memory Budget

The runtime budget must be explicit before implementation. The stress scenario is 8 decks x 4 stems x lookahead.

With stereo f32 pages, one 32768-frame page is about 256 KiB per stream. Four stem streams per deck are about 1 MiB per
page horizon per deck before allocator, alignment, metadata, or optional mix stream overhead. Eight decks are about
8 MiB per page horizon for stems only.

Lookahead multiplies that cost. A 4-page lookahead is about 32 MiB for 8 decks x 4 stems before overhead; adding full
mix transition streams, loop-end warm pages, decode staging, and retained old generations must fit inside a measured
resident working-set budget. The implementation plan must set hard per-deck and global ceilings before real-time use.

## Publication model

The media prefetch service owns page warming and publication.

The audio callback reads a published immutable snapshot:

    PublishedDeckPages
      deck generation
      transport epoch
      stream slots
      page table pointer per stream
      valid page generations

Publication rules:

- producer builds/warms pages off callback,
- producer publishes a new snapshot with release ordering,
- callback reads the current snapshot with acquire ordering,
- callback never mutates page tables,
- callback never increments reference counts,
- reclamation uses epoch/deferred reclamation outside the callback.

The exact primitive may later use an existing hot-plane/Seqlok/Exclave mechanism if present in the repo. If that substrate is not present in Dekzer, do not pretend it exists and do not implement a parallel primitive before architecture review.

## Prefetch policy

On deck load, prefetch at least:

- current playhead minus 2 seconds,
- current playhead plus 20–30 seconds,
- configured hot cue windows,
- active loop region if known.

For stems, prefetch all active stem streams for the same parent timebase range. A deck is not real-time stem-playback-ready until the needed current range is resident for every required stem stream.

## Seek policy

On seek:

1. transport change is staged,
2. prefetch target window,
3. publish page snapshot for target region,
4. arm playback once required pages are resident.

If user forces an immediate seek into a cold region, the callback still must not block. It renders controlled silence/ramp for missing streams while prefetch catches up, and the system records a missing-page event.

A missing page is a controlled product state. An audio callback underrun is a bug.

## Loop policy

Loops are first-class prefetch inputs.

When a loop is armed:

- warm loop start window,
- warm loop end window,
- warm full short loop if small enough,
- preserve parent timebase mapping across wrap.

If loop start is not warm, the system must either delay arming the loop or mark the loop as not real-time-ready. The callback must not fault or synchronously decode at the wrap point.

## Stem switching policy

Switching between mix-only playback and stem playback must be phase-aligned and loop-aware.

Before switching to stem mode:

1. verify stem playback bundle readiness,
2. verify current position pages are resident for all required stems,
3. if looping, verify pages around both loop start and loop end,
4. choose a switch window that does not cross a loop boundary if possible,
5. crossfade using parent timebase transport mapping,
6. release old source only after the fade completes.

If the loop boundary falls inside the switch fade and the system cannot prove both sides are warm, V1 should defer the switch until a safe window or require an explicit user action. It must not perform a blind crossfade across inconsistent loop positions.

## Missing page behavior

If a page is missing on callback:

- emit a short ramp to controlled silence or use previous-safe output according to deck policy,
- increment an atomic missing-page counter,
- request urgent prefetch outside the callback,
- do not allocate,
- do not log synchronously,
- do not block.

## Memory locking

`mlock`/`VirtualLock` is optional/deferred for V1 architecture.

The design must not depend on successful OS page locking because permissions, platform behavior, and memory pressure vary. It may use page locking opportunistically for loaded deck ranges after measurement.

Required regardless of OS locking:

- bounded resident working set,
- prefetch lookahead,
- no designed-in page faults on callback,
- missing-page fallback.

## Validation targets

Future implementation must test:

- 8 decks loaded,
- 4 stems active per deck,
- loop start/end switching,
- rapid seek under load,
- background waveform analysis active,
- background stem separation paused/throttled,
- zero callback allocations,
- zero callback locks,
- zero callback filesystem/SQLite/IPC calls.
