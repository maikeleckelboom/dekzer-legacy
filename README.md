# Dekzer

> Dekzer is a local-first DJ workstation built to make music libraries trustworthy before performance workflows become ambitious.

DJ software cannot become trustworthy at performance time if its library model is already lying about identity,
availability, completeness, or provenance. Dekzer builds that foundation first.

The current V0 is a working Windows desktop application for registering local music sources, indexing them into SQLite,
browsing source and folder scopes, filtering and searching indexed files, and inspecting readiness without confusing
incomplete work with an empty library.

**Status:** pre-alpha and under active development. Windows is the only supported V0 development path today. The app
runs from source, and packaged builds do not yet include the Rust backend executable. Dekzer is not ready for live DJ
performance.

[What works today](#what-works-today) · [Architecture](#architecture) · [Current status](#current-status) ·
[Run locally](#run-locally) · [Start reading](#start-reading)

## Why Dekzer exists

A DJ library is not just a directory tree with playlists on top.

A path says where one occurrence was observed. It does not establish a lasting musical identity. Drives disconnect,
folders move, and the same bytes can appear in several locations. Even identical bytes do not automatically prove that
two occurrences should become one product track.

The same distinction applies to analysis and metadata. A hash is evidence. A media probe is evidence. A BPM or key
estimate is evidence. None of those observations silently becomes a user decision, a trusted preparation artifact, or a
canonical track.

Completeness also has to be earned. A read that returns no rows may describe an empty folder, a source that is still
indexing, a blocked path, an unavailable drive, a stale cursor, or a failed operation. Collapsing those states into
"nothing here" makes the library fast only by making it unreliable.

Dekzer therefore starts below the deck. It establishes stable references, explicit source health, bounded background
work, deterministic reads, and provenance-aware identity layers first. Future preparation and performance history can
then refer to what the system actually knows.

## What works today

- **Local source intake.** The Add Source flow resolves Windows known folders and fixed or removable volumes, supports a
  native folder picker, previews local contents before admission, and keeps browsing separate from registration and
  indexing.
- **Activation before full scan.** Registering a source persists it and establishes a bounded first navigation window.
  The source and its immediate folders can become browsable before recursive discovery completes.
- **Durable indexing.** Rust owns source registration, recursive discovery, scan coverage, file observations, and bounded
  maintenance. SQLite stores the source hierarchy and evidence across desktop restarts.
- **Library browsing.** The Vue workstation exposes source and folder navigation, recursive selected-scope contents,
  offset-paginated tree reads, cursor-paginated contents, list and column views, and explicit Load More behavior.
- **Scoped search and workflow filters.** Search is backed by a SQLite FTS5 projection and stays within the selected
  library, source, or folder scope. Audio, Audio + Video, and All Files profiles change the contents policy without
  changing the selected scope. Search uses backend relevance ordering. User-selectable sort controls are not implemented.
- **Honest availability and coverage.** Missing, unavailable, permission-blocked, failed, probing, incomplete, retained,
  and verified-empty states remain distinct. A missing source stays registered and can recover when its path returns.
- **Stable renderer projections.** Pending replacement reads retain accepted tree or contents state for the same identity.
  Scan and invalidation events refresh projections without taking selection, expansion, focus, or navigation from the
  user.
- **Evidence and identity substrate.** BLAKE3 observations, exact-byte attachments, source-file occurrence links, media
  probe observations, playable-media promotion, exact-content track candidates, and explicit candidate decisions are
  implemented without claiming canonical track identity.
- **Rust-owned desktop contract.** Rust protocol definitions generate the TypeScript contract and JSON Schema. Electron
  communicates with the Rust service through a tested JSON-lines stdio transport and exposes a narrow preload API to
  the renderer.
- **Bounded analysis work.** A headless Rust core computes deterministic technical facts for supported integer PCM WAV
  files. The desktop inspector also exposes a one-shot 16-bit PCM WAV musical-analysis experiment for BPM, key, and
  beatgrid evidence. Its results are advisory or inconclusive, carry warnings and basis information, and are not saved.

## Design laws

### A path is not an identity

Filesystem location identifies an occurrence. Dekzer keeps source paths, observed file evidence, exact-byte attachment
identity, playable media, and musical identity candidates in separate layers. Some source identity is still path-based
in V0, and the model does not pretend that this closes the identity problem.

### Evidence is not a decision

Hashes, probes, imported values, analyzer output, and identity candidates record observation or inference. Acceptance,
rejection, deferral, override, preference, cleanup, merge, and removal require separate decisions with provenance.
Recomputation must not silently overwrite an accepted user choice.

### Empty is a proven state

Zero rows are not enough. The requested scope, filter, depth, cursor, access state, and coverage must agree before the UI
may claim verified empty. Unknown, probing, incomplete, blocked, failed, unavailable, missing, and stale remain visible
states.

### Background work does not own user intent

Scanning, maintenance, invalidation replay, and lifecycle refresh may update accepted projections. They do not select a
different source, expand or collapse a branch, move focus, change scroll position, or redirect navigation.

### The renderer projects truth

Vue owns presentation and ephemeral interaction state. It does not invent filesystem completeness, source availability,
product identity, durable history, or analysis authority. Those observations arrive through typed backend reads.

### Every layer has one owner

Rust domain code owns durable vocabulary. SQLite owns persisted state. The boundary service owns orchestration. Generated
contracts own cross-language protocol shapes. Electron main owns host lifecycle and IPC. Preload owns the safe renderer
surface. Vue owns projection and interaction.

## Architecture

```text
Vue renderer
    │ narrow typed renderer API
Electron preload
    │ grouped command and publication IPC
Electron main boundary spine and host
    │ transport-agnostic TypeScript client
Node stdio transport
    │ Rust-owned JSON-lines request, reply, and readiness envelopes
Rust boundary protocol and service
    │
Rust domain and SQLite store
```

The boundaries exist because the desktop has several different kinds of responsibility:

| Layer                                 | Concrete responsibility                                                                                               |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Rust domain and SQLite                | Own source lifecycle, observations, coverage, identity evidence, decisions, revisions, and deterministic reads.       |
| Rust protocol and service             | Define commands, replies, events, validation vocabulary, and store orchestration. They do not own presentation.       |
| Generated TypeScript contract         | Reproduce Rust-owned protocol types and schema for TypeScript consumers. Stale checks prevent hand-maintained drift.  |
| TypeScript client and stdio transport | Keep command semantics independent of process transport, then implement the current cross-process JSON-lines route.   |
| Electron main                         | Own backend process startup, storage location, readiness, diagnostics, event pumping, IPC registration, and shutdown. |
| Preload                               | Expose a narrow `contextBridge` API. Raw Electron primitives do not enter feature code.                               |
| Vue renderer                          | Turn accepted reads into stable tree, contents, status, inspector, and interaction projections.                       |

This split is not a generic layering exercise. Local filesystem work can block, drives can disappear, the database must
outlive a renderer reload, and the Rust and TypeScript sides must agree on every state that can reach the user. Keeping
those concerns with explicit owners makes failures testable instead of incidental.

## Domain model

The model progresses from physical evidence toward product meaning:

```text
Source and source locations                         implemented
  → observed file evidence                         implemented
    → exact-byte attachment identity               implemented
      → playable media                             implemented
        → exact-content identity candidates
          and explicit decisions                   implemented substrate
          → canonical musical identity             planned
            → preparation facets and artifacts     planned
              → workflow and performance history  future
```

Each layer answers a different question:

- Where may music live, and can the source currently be reached?
- What did the system observe about this file occurrence, and is that evidence still current?
- Which occurrences contain identical bytes?
- What is currently supported as playable media?
- Which musical item might the evidence represent?
- What did the system or user accept, reject, defer, or override?
- What preparation is current and trusted for use?
- How was the material eventually staged and used in a performance?

The current repository implements the foundation through exact-content candidates and decisions. Those records are not
canonical tracks. Preparation, Prepared Room, Performed Room, and runtime evidence remain downstream work.

## Current status

| Area                                              | Status                   | Current boundary                                                                                                                           |
| ------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Local source registration and activation          | Working V0 slice         | Windows known folders, volumes, folder selection, admission, and pre-scan navigation readiness.                                            |
| Scanning and durable library substrate            | Working V0 slice         | SQLite-backed hierarchy, inventory, coverage, observations, maintenance, and restart persistence.                                          |
| Hierarchy and contents browsing                   | Working V0 slice         | Source and folder scopes, recursive contents, retained reads, list and column projections, and pagination.                                 |
| Scoped search and filters                         | Working V0 slice         | FTS5-backed text search plus Audio, Audio + Video, and All Files profiles. Search order is backend-owned. User sort controls are deferred. |
| Missing and offline handling                      | Working V0 slice         | Known sources remain visible as unavailable or missing and recover after restore. Blocked and incomplete do not become empty.              |
| Attachment, playable-media, and identity evidence | Implemented substrate    | Exact bytes, occurrence links, playable media, candidates, decisions, and review reads. No canonical track authority yet.                  |
| Technical audio analysis                          | Implemented bounded core | Deterministic technical facts for supported integer PCM WAV. Headless and test-oriented, with no persistence or product authority.         |
| Musical analysis                                  | Bounded experiment       | Inspector-triggered BPM, key, and beatgrid attempt for 16-bit PCM WAV. Advisory, warning-bearing, and never persisted.                     |
| Preparation workflow                              | Planned                  | Durable preparation facets, artifact contracts, and readiness decisions are not implemented.                                               |
| Prepared Room and Performed Room                  | Future vision only       | The domain model constrains identity and provenance work but has no product-backed UI or persistence.                                      |
| Waveforms and deck runtime                        | Proposal and future work | Architecture documents exist. No production waveform artifact pipeline, playback deck, or performance session exists in this repository.   |
| Hardware and streaming services                   | Outside V0               | No controller integration, hardware support, or streaming provider support.                                                                |

## Verification

The repository checks behavior at several boundaries:

- Rust-to-TypeScript contract export and stale detection for both the protocol package and stdio readiness envelope
- TypeScript and Vue type checking across the generated contract, client, transport, main process, preload, and renderer
- Rust workspace tests for domain, SQLite, service, protocol, transport, scanning, identity, and analyzer behavior
- Vitest unit and integration coverage for renderer projections, source lifecycle, IPC mappings, host behavior, and real
  Rust-backed desktop operations
- TypeScript client validation and cross-process stdio smoke tests against the compiled Rust server
- Playwright Electron acceptance for launch, source admission and scan, scoped search, contents pagination, restart
  persistence, missing-source recovery, and remove/re-add freshness
- Formatting, lint, `git diff --check`, Rust `fmt`, and Clippy gates

`pnpm verify` runs the normal pre-merge gate. The Electron V0 acceptance gate is separate because it builds and launches
the desktop app. It uses isolated storage and generated WAV fixtures, so it proves the covered workflow but does not
claim broad real-media compatibility.

## Roadmap

The roadmap follows dependency order rather than feature volume:

1. Harden the current local library workflow and its availability, coverage, search, and projection contracts.
2. Extend exact-byte and playable-media evidence into durable canonical musical identity without silent merging.
3. Define and implement provenance-aware preparation, analysis, and waveform artifacts.
4. Build a basic local preparation and performance workflow on those trusted references.
5. Introduce Prepared Room, performance instances, and immutable Performed Room history.
6. Add runtime evidence and further projection surfaces only after their upstream owners exist.

The full sequence, its blockers, and what is deliberately deferred live in the [roadmap](docs/roadmap.md).

## Run locally

Requirements:

- Windows for the current local-source V0 path
- Node.js 25 or newer
- pnpm 10.33.2, pinned in `package.json`
- a stable Rust toolchain with Rust 2024 edition support
- an interactive desktop session for Electron end-to-end tests

From the repository root:

```bash
pnpm install
pnpm desktop:dev:fresh
pnpm verify
pnpm --filter @dekzer/desktop verify:e2e:library-v0
```

`desktop:dev:fresh` resets only Dekzer's development storage before launch. Use `pnpm desktop:dev` when that state should
be retained. The development preflight invokes Cargo for storage compatibility and builds the Rust stdio server as
needed. The focused Electron command builds both the server and desktop app before running the seven V0 acceptance
scenarios.

## Start reading

- [Product](docs/product.md) explains the V0 product target, the principles that decide current work, and the explicit
  non-goals.
- [Architecture](docs/architecture.md) explains command, publication, host failure, readiness, and renderer ownership
  across the desktop process boundary.
- [Domain model](docs/domain-model.md) explains the identity, evidence, and decision layers and what each may claim.
- [Library browser](docs/library-browser.md) defines pre-scan browsing, coverage, verified-empty behavior, state
  retention, and user-intent safety.
- [Roadmap](docs/roadmap.md) records what works now, what comes next, and what is deferred.
- [Development](docs/development.md) covers requirements, commands, verification, and storage reset.

The [documentation index](docs/README.md) lists the full set, including the future vision document and the surviving
decision records.

## Project status and license

Dekzer is pre-alpha. It is suitable for architecture review, implementation study, and controlled development testing.
It is not suitable for real DJ performance, library migration, or irreplaceable preparation work.

This is an independent product and systems-engineering project. No contribution process or support commitment is
published yet.

The Rust workspace manifests declare the project crates as MIT licensed. A repository-level `LICENSE` file is not
currently present, so repository-wide reuse terms are not fully documented yet.
