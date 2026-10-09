# Roadmap

> **Superseded roadmap.** This describes the former V0 development sequence, not active
> work or a commitment for the next Dekzer generation. See
> [legacy closure and handoff](legacy-closure.md).

Sequencing follows substrate dependency rather than feature visibility. A layer is built when the thing underneath it
can answer the questions it will ask.

Development history lives in Git. This document describes the current position and what comes next.

## Now

Working and covered by tests:

- Local source intake through Windows known folders, volumes, and a native folder picker, with preview before admission
- Source activation ahead of recursive scan, so a source is browsable while indexing continues
- Rust and SQLite ownership of registration, discovery, scan coverage, file observations, and bounded maintenance
- Tree and contents browsing across source, source-location, and directory scopes, with paginated reads and list and
  column views
- Scoped search over an FTS5 projection, plus Audio, Audio + Video, and All Files workflow filters
- Distinct availability and coverage states, including missing-source recovery after the path returns
- Retained projections across refresh, with scan and lifecycle events unable to take selection, expansion, or focus
- Evidence and identity substrate through exact-content candidates and explicit decisions
- A Rust-owned generated boundary contract with staleness checks, over a tested JSON-lines stdio transport

The current focus is hardening this. Availability, coverage, search, and projection behaviour are the surfaces most
likely to be wrong in ways a user would not notice until it mattered, so they get attention before anything new is
layered on.

## Next

**Canonical musical identity.** The substrate reaches exact-content candidates and reversible decisions. It does not
have a canonical track. Building one means extending byte and playable-media evidence into a durable product identity
that survives path changes, without silent merging and without treating a hash as proof of sameness.

This is the next meaningful product milestone because almost everything after it needs a stable thing to refer to. A
preparation artifact, a crate, and a performance reference all need to name a track that outlives its current path.

Two things block hardening it.

The first is the [media candidate layer](domain-model.md#media-candidate-not-implemented). Current track candidates
group whole files by exact content, which holds only while one file is one musical unit. CUE-backed albums, multi-disc
rips, and long indexed mixes break that in both directions, and they cannot be represented as ordinary playable-file
rows. Acoustic fingerprinting depends on the same layer, because it needs decoded audio and a unit to fingerprint.

The second is an import interoperability contract, because cue, loop, beatgrid, and playlist data from rekordbox,
Serato, and Traktor must be receivable as provenance-bound evidence by the candidate schema from the start. Designing
that schema first and adding foreign data later closes off the ingestion path.

**Supporting work along the way.** Tag observations as raw evidence rather than canonical metadata. A CUE corpus audit
against real collection material before any CUE parse schema is designed, because the edge cases in real archives are
not the ones the specification suggests. Occurrence preference, deciding which copy of the same bytes is preferred,
which needs source health context to resolve coherently.

## Later

Each of these is blocked on the milestone above it, and none should start before it.

**Preparation, analysis, and waveform artifacts.** Independent facets rather than one status: beatgrid, BPM, key,
waveform, cues, loops, phrases, loudness, energy, stems, notes. Each needs an owner, a basis, evidence, current or
stale status, provenance, and a distinction between machine-generated and user-approved. Waveforms are backend-owned
artifacts bound to a basis, not renderer-generated throwaways. Design documents for waveform storage, delivery, and
rendering exist in Git history and will need rewriting against whatever substrate is actually current when the work
starts.

**A basic local preparation and performance workflow** over those trusted references.

**Prepared Room, performance instances, and immutable performed history.** See
[the vision](vision/spatial-performance-memory.md). A room surface built before canonical identity and preparation
exist is a mockup, because it would hold references to things the system cannot yet name durably.

**Runtime evidence and further projection surfaces**, only after the domains that own their content exist.

## Explicitly not now

Deferred on purpose. These are not backlog items waiting for capacity.

- **Hardware and controller integration, and streaming providers.** No substrate obligation and no product foundation.
- **A playback deck and live performance mode.** The library is not yet trustworthy enough to be the thing a deck
  loads from. Building the deck first means discovering library problems during a set.
- **Production waveform generation and UI.** Blocked on a backend-owned artifact substrate.
- **Playlists and crates as the central workflow model.** The organizational model matters and should not default to
  the flat structure that is easiest to build first.
- **AR, VR, audience, and social surfaces.** These project a model that does not exist yet.
- **A work scheduler with lanes, resource budgets, and checkpointing.** Current work runs as bounded-count maintenance
  passes with no per-file time limit, so a large file can still occupy a pass. A scheduler is justified when a measured
  workload shows that is actually hurting, not before. See
  [the bounded work items decision](decisions/0003-bounded-work-items.md).
- **Attachment garbage collection.** Orphaned attachment rows after link replacement are known and left alone. Removing
  data automatically is exactly the behaviour the domain model refuses.
- **User-selectable sort controls.** Ordering is backend-owned. Exposing sort means owning cursor identity across
  orderings.
- **macOS and Linux.** Windows is the only supported development path for V0.

## How this sequence is decided

Two rules, and they explain most of the ordering above.

Evidence comes before interpretation. A product view that claims duplicates, relocations, or identity needs the
evidence layer beneath it to already exist, or the view is guessing.

Contracts come before implementation for anything that crosses a language boundary or hardens a schema. Not as
ceremony, but because a wire format or a schema that receives foreign data is expensive to change once written to a
user's disk.
