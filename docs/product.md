# Dekzer product

Dekzer is a local-first DJ workstation. V0 is a reliable local music library foundation, not a performance system.

This document describes what Dekzer is building now. The long-term product thesis lives in
[the spatial performance memory vision](vision/spatial-performance-memory.md).

## The problem

A DJ library is not a directory tree with playlists on top, and most DJ software treats it as one.

That produces failures that only become visible under pressure. A folder reads as empty while discovery is still
running. A drive is unplugged and the tracks silently vanish from a prepared set. Two copies of the same file become
two unrelated entries, or get merged into one without asking. An imported BPM value from another application becomes
indistinguishable from a measured one. A background scan finishes and moves the user's selection.

Each of these is the same mistake in a different place: the system claims to know something it has not established.
The cost is not paid during preparation. It is paid at the moment the library is trusted, which is usually the moment
it matters most.

Dekzer starts below the deck. Before there is a waveform or a crossfader, the library has to be able to say what it
actually knows, what it does not, and where the difference lies.

## What V0 is

V0 is a working Windows desktop application that registers local music sources, indexes them into SQLite, and lets a
user browse, search, and filter them without the library lying about its own state.

Working today:

- **Local source intake.** Windows known folders and fixed or removable volumes resolve into browsable entry points.
  A source is previewed before admission, and browsing a folder is separate from registering and scanning it.
- **Activation before full scan.** A registered source becomes navigable once a bounded first window is readable.
  Recursive discovery continues behind that.
- **Durable indexing.** Rust owns registration, recursive discovery, scan coverage, and file observations. SQLite holds
  the hierarchy and evidence across restarts.
- **Browsing.** Source and folder scopes, recursive selected-scope contents, paginated tree and contents reads, list
  and column views.
- **Scoped search and workflow filters.** Search runs against a SQLite FTS5 projection and stays inside the selected
  scope. Audio, Audio + Video, and All Files change the contents policy without changing the selected scope.
- **Honest availability.** Missing, unavailable, blocked, failed, probing, incomplete, retained, and verified-empty
  remain distinct states. A missing source stays registered and recovers when its path returns.
- **Evidence and identity substrate.** BLAKE3 observations, exact-byte attachments, occurrence links, media probe
  observations, playable-media promotion, exact-content candidates, and explicit candidate decisions, all without
  claiming canonical track identity.

The application runs from source and is not ready for live performance.

## Product principles

These are the rules that decide current implementation questions. They are stated once here and applied by the
documents that own each area.

**A path is not an identity.** A filesystem location records where one occurrence was observed. It does not establish a
lasting musical identity. Byte identity, acoustic identity, and track identity are separate layers, and Dekzer keeps
them separate. See [the domain model](domain-model.md).

**Evidence is not a decision.** Hashes, probes, imported values, and analyzer output are observations. Acceptance,
rejection, deferral, override, preference, merge, and removal are decisions with provenance. Recomputation never
silently overwrites an accepted choice, and nothing is cleaned up, merged, or forgotten without an explicit decision
record.

**Empty is a proven state.** Zero rows is not enough. Scope, filter, depth, cursor, access state, and coverage all have
to agree before the interface may claim a scope is empty. See [the library browser](library-browser.md).

**Background work does not own user intent.** Scanning, maintenance, invalidation replay, and lifecycle refresh update
projections. They do not change selection, expansion, focus, scroll position, or navigation.

**The renderer projects truth.** Vue owns presentation and ephemeral interaction state. It does not invent filesystem
completeness, source availability, product identity, or durable history. See [the architecture](architecture.md).

**Uncertainty surfaces during preparation.** Source loss, changed analysis basis, unsupported formats, and unresolved
identity have to be visible while there is still time to act. A visual defect is a bug. A silent readiness claim that
turns out to be false is a trust failure, and it is the more expensive of the two.

**The library stays local.** Library contents, paths, and preparation state are not sent to external services without
explicit consent. Cloud sync, catalog lookup, and external analysis would be additive. Removing them must not
invalidate local authority.

## Explicit V0 non-goals

These are not partially built or nearly ready. They are absent by choice, because the substrate they would need does
not exist yet.

- Hardware and controller integration
- Streaming service providers
- A playback deck, transport, or live performance mode
- Production waveform generation, storage, or rendering
- Canonical track identity, metadata reconciliation, and track merging
- Preparation facets such as beatgrids, cues, loops, and stems
- Playlists and crates as a central workflow model
- Prepared Room, Performed Room, and any runtime evidence surface
- Audience, social, AR, and VR surfaces
- Import from rekordbox, Serato, or Traktor
- macOS and Linux as supported development or usage paths

Two bounded exceptions exist and are deliberately narrow. A headless Rust core computes deterministic technical facts
for supported integer PCM WAV files. The desktop inspector exposes a one-shot musical-analysis experiment for BPM, key,
and beatgrid over 16-bit PCM WAV. Its results are advisory, carry warnings and basis information, and are never
persisted.

## Where this is going

The sequencing that connects V0 to later work is in [the roadmap](roadmap.md). The long-term differentiator, spatial
performance memory, is described in [the vision document](vision/spatial-performance-memory.md) and is explicitly not
current functionality. Current substrate work supports it only where preserving identity, availability, provenance,
and the evidence-versus-decision boundary is worth doing on its own merits.
