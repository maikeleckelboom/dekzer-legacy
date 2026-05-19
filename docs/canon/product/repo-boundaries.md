# Repo Boundaries

## Status

Active product canon for repository ownership.

## Exclave Boundary

Exclave owns reusable runtime and boundary substrate only. It may provide generic control-plane envelopes, session validity, warm publication framing, resource handles, hot-lane codecs, host contracts, schema tooling, and conformance primitives.

Exclave must remain product-neutral. It must not define Dekzer music semantics, workspace product shell behavior, library root registration, scan policy, playlist meaning, preparation meaning, or desktop app composition.

Dekzer may depend on Exclave as an external dependency. Dekzer must not vendor Exclave.

## Dekzer Boundary

Dekzer owns the product monorepo. It owns the Electron desktop app, Vue renderer shell, workspace product surfaces, local music library substrate, generated boundary contract/client packages for product subsystems, Rust/SQLite durable local state, and active product canon.

The Electron scaffold is only an app host. It does not own product architecture. The renderer is a projection/client layer. Vue stays at the presentation edge.

## Library Substrate Ownership

Dekzer owns library substrate semantics:

- local roots
- source locators
- source state
- source locations
- source directories and files
- literal hierarchy
- library assets
- playlists
- preparation state
- scan state
- library read models
- generated boundary contracts for the library subsystem

`music-library-core` should move into Dekzer through controlled slices because those concepts are Dekzer product authority, not reusable substrate.

## Workspace Shell Ownership

Dekzer owns the workspace shell as product composition. Workspace shell code decides how real product surfaces are assembled in the app.

Reusable workspace topology, layout, interaction-session, or projection-runtime primitives belong in Exclave only when they are proven product-neutral. Dekzer may consume those primitives, but product shell behavior remains Dekzer-owned.

## Renderer And Client Ownership

The renderer owns presentation, viewport realization, local transient interaction state, and command intent from the user.

The renderer does not own durable substrate truth, read-model semantics, scan policy, boundary protocol definitions, SQLite access, or Rust authority behavior. It consumes generated clients and renders projections.

## Rust And SQLite Ownership

Rust owns product authority behavior for local substrate operations. SQLite owns durable local storage, relational integrity, and durable read tables where the product needs them.

Rust and SQLite together own the durable library substrate. The desktop app and renderer never become a second owner for the same facts.

## Forbidden Dependency Directions

The following directions are forbidden:

- Exclave depending on Dekzer product packages or crates
- library domain, store, read model, or authority crates depending on Electron, Vue, renderer packages, or desktop composition
- renderer packages depending on SQLite, store internals, or Rust authority internals
- generated TypeScript contracts depending on handwritten client convenience code
- desktop composition replacing missing substrate contracts with fake product wiring
- boundary protocol packages becoming owners of durable product semantics
- old and new package names coexisting as aliases

## Workspace-Host Demo Status

`workspace-host` demo code is non-authoritative. Its `apps/desktop--draft` app shell, demo hosts, draft library browser behavior, sample layouts, generated output, fixtures, and visual/product assumptions must not move into Dekzer.

`workspace-host` may be inspected only to classify reusable runtime candidates, names to reject, demo code to quarantine, and tests or invariants worth recreating cleanly under the correct owner.

## Product Rule

One concept has one owner and one canonical representation. Historical repo splits do not create product boundaries.
