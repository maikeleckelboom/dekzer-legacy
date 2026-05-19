# Dekzer

Dekzer is the product monorepo root.

Current state:

- `apps/desktop` is the booting Electron/Vue desktop app.
- `crates/library-domain` is the first migrated Rust library substrate crate.
- `crates/library-store-sqlite` is the migrated durable SQLite store crate.
- `crates/library-boundary-protocol` is the migrated Rust boundary DTO/protocol crate.
- `crates/library-boundary-service` maps boundary protocol commands to the real SQLite store.
- `crates/xtask` owns generated boundary contract export/check tooling.
- `packages/library-boundary-contract` is the generated-only TypeScript boundary contract package.
- `packages/library-boundary-client` is the hand-authored, transport-agnostic TypeScript client/session package over the generated boundary contract.
- `workspace-host` is not imported.
- Remaining `music-library-core` slices beyond the domain, SQLite store, boundary protocol, boundary service, generated boundary contract, and TypeScript boundary client are not imported.
- Exclave is an external dependency candidate, not vendored into this repo.

Run workspace commands from this directory:

```bash
pnpm install
pnpm run dev:desktop
pnpm run library:contract:export
pnpm run library:contract:check
pnpm run library:contract:build
pnpm run typecheck
pnpm run library:client:typecheck
pnpm run library:client:build
pnpm run library:client:test
pnpm run check
pnpm run build
pnpm run test
pnpm run test:rust
pnpm run fmt:rust
pnpm run lint:rust
pnpm run build:desktop
```

Current root scripts:

- `dev:desktop` starts the desktop app.
- `build:desktop` builds the desktop app.
- `library:contract:export` regenerates `packages/library-boundary-contract` from `crates/library-boundary-protocol`.
- `library:contract:check` verifies the generated boundary contract package is current.
- `library:contract:build` builds the generated boundary contract package.
- `library:client:typecheck` typechecks the hand-authored TypeScript boundary client.
- `library:client:build` builds the generated boundary contract package, then the TypeScript boundary client.
- `library:client:test` runs the boundary client/session validation fixture.
- `typecheck` runs the generated boundary contract package typecheck, boundary client typecheck, and desktop typecheck.
- `build` builds the generated boundary contract package, boundary client, and desktop app.
- `test` runs the Rust workspace test suite.
- `test:rust` runs `cargo test --workspace`.
- `fmt:rust` runs `cargo fmt --all --check`.
- `lint:rust` runs `cargo clippy --workspace --all-targets -- -D warnings`.
- `check` runs the boundary contract stale check, TypeScript typechecks, boundary client validation fixture, Rust fmt, clippy, and test gates.

Workspace ownership:

- `apps/*` is for product applications.
- `packages/*` is for JavaScript and TypeScript packages, starting with the generated-only library boundary contract and the transport-agnostic library boundary client.
- `crates/*` is for Rust workspace ownership, starting with `crates/library-domain`.
- `docs/*` is for product canon, decisions, and architecture documents.

Only `apps/desktop`, the migrated Rust library domain, store, boundary protocol and service crates, the generated TypeScript boundary contract, and the transport-agnostic TypeScript boundary client are present as product code after this slice. See `docs/decisions/repo-consolidation-plan.md` and `docs/decisions/migration-ledger.md` for the migration plan.
