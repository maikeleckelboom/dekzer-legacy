# Repo Consolidation Plan

## Status

Accepted migration-control-plane direction.

## Purpose

This document prepares `C:\dev\dekzer` as the long-term Dekzer product monorepo root without migrating product code yet.

The current visible desktop app must remain honest:

- Dekzer boots from `apps/desktop`.
- `workspace-host` is not imported.
- `music-library-core` is not imported.
- Exclave remains an external dependency, not vendored code.

## Final Repo Boundary

### Exclave

Exclave owns reusable boundary and runtime primitives with no Dekzer product semantics. It may own generic control, warm, resource, session, hot, publication, schema, conformance, host-contract, ABI, and codec primitives.

Exclave must not own Dekzer library concepts, workspace product shell composition, music semantics, local source registration, scan policy, playlist meaning, preparation meaning, or desktop product assembly.

### Dekzer

Dekzer owns the product monorepo:

- Electron desktop app
- Vue renderer shell and product surfaces
- workspace shell and product composition
- local music library substrate
- Rust and SQLite durable local state
- generated TypeScript boundary contract and client packages for Dekzer product subsystems
- product docs, canon, decisions, and migration ledgers

## Why `music-library-core` Moves Into Dekzer

`music-library-core` contains Dekzer product semantics: local roots, source locations, literal hierarchy, library assets, preparation, playlists, scan state, maintained read models, and library boundary protocol. Those are not reusable runtime substrate concepts. They are the product's durable music authority and must live in Dekzer.

The move must happen through controlled slices. The current code is valuable, but the repo identity, package scope, and `surface` vocabulary are not the long-term product shape. The target is a Dekzer library substrate with explicit boundary layers, not a renamed external product.

## Why `workspace-host` Does Not Remain The Product Root

`workspace-host` was useful exploration history for workspace topology and runtime behavior. It is not the Dekzer product root because:

- the desktop app in Dekzer already owns the active product scaffold
- `workspace-host` mixes reusable runtime work with draft desktop composition
- its `apps/desktop--draft` surface is prototype behavior, not product authority
- keeping it as the product root would preserve the old three-repo split instead of reducing it
- product shell and library browser behavior must be rewritten inside Dekzer against real product owners

## Why The `workspace-host` Demo Code Is Rejected

The `workspace-host` demo proves interaction ideas, but it is not product design. It must not be copied because it would import fake ownership, draft shell assumptions, demo library composition, old package links to `music-library-core`, and visual/product decisions that are no longer authoritative.

The only legal use of `workspace-host` in this migration is inspection for:

- reusable workspace topology/runtime candidates
- names to reject
- prototype surfaces to quarantine
- tests or invariants worth recreating cleanly later

## What Stays In Exclave

The following classes stay in Exclave, or move to Exclave only if extracted without Dekzer semantics:

- generic control-plane envelopes and failure handling
- session validity and binding lifetime primitives
- warm payload framing and publication mechanics
- resource handle and resource range protocols
- hot lane codecs, bindings, readers, writers, and ABI conformance
- host boundary contracts that do not name Dekzer product concepts
- schema, compiler, and conformance tooling for generic projection runtime contracts
- reusable runtime invariants that can be proven product-neutral

## What May Later Move From `workspace-host`

Move means "port or rewrite under the correct owner," not copy. Candidates:

- workspace topology language ideas, if product-neutral, to Exclave
- compiler/runtime realization invariants, if product-neutral, to Exclave
- layout solver laws and resize-session invariants, if product-neutral, to Exclave
- Vue adapter patterns only after product shell ownership is rewritten in Dekzer
- tests around deterministic layout, resize sessions, projection legality, and public package surfaces, recreated cleanly

## What Must Not Move From `workspace-host`

These are rejected:

- `apps/desktop--draft`
- demo app shell
- demo routes, sample layouts, playgrounds, labs, experiments, screenshots, fixtures, generated artifacts, `dist`, and `node_modules`
- prototype `NavigationTreeHost`, `LibraryBrowserHost`, `DetailsInspectorHost`, and related demo hosts
- draft desktop library surface wiring
- fake or sample content rows
- package names that make `workspace-host` sound like a product owner
- any code path that treats the renderer or desktop draft as substrate authority

## What May Later Move From `music-library-core`

Controlled migration candidates:

- Rust library domain types
- SQLite durable store and schema migrations
- local root registration and scan authority
- source, source-location, source-directory, and source-file substrate
- literal hierarchy reads
- maintained read models and invalidation concepts
- library browser and navigation read models
- library boundary protocol and service
- generated TypeScript boundary contract
- handwritten TypeScript boundary client and session tests
- tests for register root, scan, literal hierarchy, service reopen integrity, event pump behavior, and generated contract freshness

## What Gets Deleted Instead Of Moved

- `workspace-host` demo desktop code
- generated `dist` outputs, `.tmp-fixtures`, `node_modules`, and build caches
- old `@music-library-core` package scope
- `music-library-core` repo identity
- `surface` naming where it means process or authority boundary
- compatibility aliases, wrapper packages, and dual old/new paths
- stale docs that preserve historical repo splits as product law

## Target Package And Crate Paths

| Owner | Target path | Source direction | Notes |
| --- | --- | --- | --- |
| Dekzer | `apps/desktop` | Already present | Electron app and Vue renderer edge. |
| Dekzer | `packages/workspace-shell` | Future rewrite | Product shell composition only, no reusable runtime authority. |
| Dekzer | `packages/library-boundary-contract` | Regenerate from Rust protocol | Replaces `@music-library-core/library-surface-contract`; generated-only. |
| Dekzer | `packages/library-boundary-client` | Controlled port and rename | Replaces `@music-library-core/library-surface-client`; no DTO copies. |
| Dekzer | `crates/library-domain` | Controlled move | Product vocabulary and typed ids. |
| Dekzer | `crates/library-store-sqlite` | Controlled move and rename | Durable SQLite substrate. |
| Dekzer | None for `library-read-kernel` | Rejected after audit | Store-owned rows/windows stay in `crates/library-store-sqlite`; future boundary service maps them directly to protocol DTOs. |
| Dekzer | `crates/library-authority` | Future explicit layer | Product operations such as register root and scan. |
| Dekzer | `crates/library-boundary-protocol` | Controlled move and rename | Rust source for generated boundary contract. |
| Dekzer | `crates/library-boundary-service` | Controlled move and rename | Maps boundary requests to product owners. |
| Dekzer | `crates/library-boundary-stdio` | Future rewrite | Host adapter if still needed; do not copy the `workspace-host` draft. |
| Exclave | external packages and crates | External dependency | Boundary/runtime primitives only, not vendored into Dekzer. |

## Dependency Direction Rules

- Dekzer may depend on Exclave boundary/runtime primitives.
- Exclave must not depend on Dekzer product packages or crates.
- Renderer and desktop composition may depend on generated clients and presentation adapters.
- Renderer must not depend on SQLite, Rust store internals, or durable product authority crates.
- Library store, read model, domain, and authority crates must not depend on Electron, Vue, renderer packages, or desktop composition.
- Boundary service may depend on product authorities and boundary protocol.
- Generated TypeScript contract is downstream of Rust protocol.
- No aliases, compatibility wrappers, dual paths, or legacy package scopes.

## Validation Strategy

Every migration slice must prove the owner it touches:

- root workspace: `git diff --check`, `pnpm -w list`, `pnpm run typecheck`, `cargo metadata`
- generated boundary contract: export, check freshness, build, typecheck, client/session tests
- library substrate: `cargo test -p library-store-sqlite`, read-model tests, reopen tests
- desktop composition: typecheck, build, and boot the Electron app against real imports only
- Exclave integration: conformance tests in Exclave plus Dekzer boundary integration tests

No slice is accepted if validation passes only through fake data, mock product wiring, or desktop-side bypasses.

## Migration Order

1. Establish Dekzer root docs, root workspace metadata, and honest boot status.
2. Decide target names and delete old naming from future docs.
3. Move library substrate slices from `music-library-core` into Dekzer in dependency order: domain, store, protocol, service, generated contract, client.
4. Introduce explicit `library-authority` only where it removes hidden ownership.
5. Recreate required tests in Dekzer and keep behavior unchanged before broad renames.
6. Connect the desktop app to real library boundary imports.
7. Port or rewrite product-owned workspace shell surfaces in Dekzer.
8. Extract or align product-neutral workspace runtime primitives with Exclave.
9. Delete historical repo paths after the Dekzer path is canonical and validated.

## Stop Rules

Stop a slice immediately if any of these happen:

- a `workspace-host` demo source file is being copied
- a `music-library-core` source file is moved without a named target owner and validation plan
- a compatibility wrapper or alias is proposed
- renderer code starts owning SQLite access or product substrate semantics
- desktop composition starts replacing a missing lower-layer contract
- Exclave receives Dekzer product semantics
- fake rows, fake subsystem imports, or mock production wiring appear
- old and new names are both active for the same concept

## First Real Product Slice After Migration

The first real product slice is:

`register local music root -> scan literal hierarchy -> persist substrate state -> restart app -> browse persisted hierarchy immediately from local authority`

This slice proves the important things: durable local ownership, literal hierarchy as first-class substrate, renderer as projection, desktop composition over real contracts, and restart integrity.
