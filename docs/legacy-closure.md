# Dekzer legacy: closure and handoff

**Status:** development concluded on 9 October 2026. GitHub archival is left to the repository owner.

**Code baseline before closure:** `928abf99952dba12650dff553fba17339b182da7` on both
`main` and `dev`. This close-out changes documentation only.

## Why this generation ended

This repository explored a local-first DJ workstation by building trustworthy music-library
infrastructure before introducing decks and live performance. The next Dekzer direction takes
a different product starting point: **a DJ performance is a durable, shared musical object**.
Preparation, execution, historical evidence, music discovery, and audience participation
can be connected through that performance identity, independently of any single mixer,
recording, livestream, or DJ application.

This is a change of product focus, not a declaration that the previous engineering was
worthless. The new direction does not automatically inherit this repository's implementation,
architecture, database schema, or roadmap.

## Implemented in legacy V0

- Windows desktop prototype using Electron, Vue, Rust, and SQLite, running from source.
- Registration and indexing of local music sources, scoped browsing and search, and source
  availability and scan-coverage states that distinguish unknown from verified empty.
- Byte observations, BLAKE3 content attachments, playable-media evidence, exact-content
  track candidates, and explicit candidate decisions. Canonical musical track identity
  remains unimplemented.
- A Rust-owned generated TypeScript boundary contract, stdio transport, and tests across
  library, protocol, desktop, and selected Electron end-to-end workflows.
- Limited WAV-based technical analysis and an advisory musical-analysis experiment.

See [product](product.md), [architecture](architecture.md), and [domain model](domain-model.md)
for the exact historical boundaries and their rationale.

## Not implemented

This repository does **not** contain a live-safe DJ engine, working playback decks, controller
integration, livestreaming, a durable preparation workflow, Prepared Rooms, performance
instances, immutable Performed Rooms, audience participation, or a public performance
platform. The [spatial performance memory vision](vision/spatial-performance-memory.md) is
preserved as unimplemented research rather than a promise of delivered functionality.

## Useful ideas to carry forward

- A path or file occurrence is not musical identity. A performance must not depend on a
  mutable filesystem location for its meaning.
- Observed evidence, inferred identity, and an accepted human decision require distinct
  provenance. They must not overwrite each other silently.
- Unavailable material should not be silently treated as absent, and an incomplete read
  should not be reported as a verified empty result.
- Background work must not hijack selection or other user intent.
- A prepared musical environment, an actual performance instance, and historical evidence
  answer different questions. Their relationships deserve a product-level model.

These are **research inputs**, not mandated packages or schema contracts for a successor.
Any reused implementation must earn its place against a concrete new user workflow.

## Known limitations and review notes at closure

- The application is pre-alpha, Windows-only for V0, and not ready for live performance.
  Packaged builds do not bundle the Rust backend executable.
- `pnpm verify` and the separate interactive Electron acceptance gate exist, but the
  inspected baseline has no GitHub Actions workflow enforcing them on every commit.
  No fresh runtime test execution was performed for this documentation-only close-out.
- The renderer-facing preload API exposes `registerLocalPath`, while
  [architecture.md](architecture.md) describes arbitrary absolute-path registration as
  host-internal. This is a documented boundary mismatch that requires review before
  carrying the code forward. It is not a demonstrated security exploit.
- The real scan-cancellation integration test can skip when its Rust binary is missing.
  A green test gate does not, by itself, prove that this test ran.
- Large modules and multiple protocol adapters should be reassessed for maintainability
  instead of copied without review.
- Rust workspace manifests declare MIT, but there is no repository-level `LICENSE` file.
  Reuse rights and third-party dependencies should be reviewed before migration.

## Repository handoff

Source code, tests, surviving documentation, and Git history are retained for inspection.
No source changes, migrations, code deletion, or history rewrites are part of this close-out.
Historical plans in this repository should not be treated as active commitments for a
successor Dekzer project. The owner may archive this repository after reviewing the final
documentation commit. Archival itself is not performed here.
