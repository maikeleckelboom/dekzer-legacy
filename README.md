# Dekzer

Dekzer is the product monorepo root.

Current state:

- `apps/desktop` is the booting Electron/Vue desktop app.
- `crates/library-domain` is the first migrated Rust library substrate crate.
- `crates/library-store-sqlite` is the migrated durable SQLite store crate.
- `crates/library-boundary-protocol` is the migrated Rust boundary DTO/protocol crate.
- `crates/library-boundary-service` maps boundary protocol commands to the real SQLite store.
- `crates/library-boundary-stdio` is the boundary stdio server binary for JSON-lines command transport.
- `crates/xtask` owns generated boundary contract export/check tooling.
- `packages/library-boundary-contract` is the generated-only TypeScript boundary contract package.
- `packages/library-boundary-client` is the hand-authored, transport-agnostic TypeScript client/session package over the generated boundary contract.
- `packages/library-boundary-stdio-transport` is the Node stdio transport package that implements the boundary client transport seam by spawning the Rust stdio server.
- `workspace-host` is not imported.
- Remaining `music-library-core` slices beyond the domain, SQLite store, boundary protocol, boundary service, stdio transport, generated boundary contract, and TypeScript boundary client are not imported.
- Exclave is an external dependency candidate, not vendored into this repo.

Run workspace commands from this directory:

```bash
pnpm install
pnpm run dev:desktop
pnpm run library:contract:export
pnpm run library:contract:check
pnpm run library:contract:build
pnpm run library:stdio:contract:export
pnpm run library:stdio:contract:check
pnpm run typecheck
pnpm run library:client:typecheck
pnpm run library:client:build
pnpm run library:client:test
pnpm run library:stdio:typecheck
pnpm run library:stdio:build
pnpm run library:stdio:test
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
- `library:stdio:contract:export` regenerates the Rust-owned stdio transport envelope artifact consumed by the TypeScript stdio transport package.
- `library:stdio:contract:check` verifies the generated stdio transport envelope artifact is current.
- `library:client:typecheck` typechecks the hand-authored TypeScript boundary client.
- `library:client:build` builds the generated boundary contract package, then the TypeScript boundary client.
- `library:client:test` runs the boundary client/session validation fixture.
- `library:stdio:typecheck` typechecks the TypeScript stdio transport package.
- `library:stdio:build` builds the generated boundary contract package, boundary client, and stdio transport package.
- `library:stdio:test` builds the Rust stdio server and runs the stdio transport validation and cross-process smoke fixtures.
- `typecheck` runs the generated boundary contract package typecheck, boundary client typecheck, stdio transport typecheck, and desktop typecheck.
- `build` builds the generated boundary contract package, boundary client, stdio transport package, and desktop app.
- `test` runs the boundary client/session validation fixture, stdio transport validation and cross-process smoke fixtures, and the Rust workspace test suite.
- `test:rust` runs `cargo test --workspace`.
- `fmt:rust` runs `cargo fmt --all --check`.
- `lint:rust` runs `cargo clippy --workspace --all-targets -- -D warnings`.
- `check` runs the boundary contract stale check, stdio transport contract stale check, TypeScript typechecks, boundary client and stdio validation fixtures, Rust fmt, clippy, and test gates.

Workspace ownership:

- `apps/*` is for product applications.
- `packages/*` is for JavaScript and TypeScript packages, starting with the generated-only library boundary contract, transport-agnostic library boundary client, and stdio transport.
- `crates/*` is for Rust workspace ownership, starting with `crates/library-domain`.
- `docs/*` is for product canon, decisions, and architecture documents.

Only `apps/desktop`, the migrated Rust library domain, store, boundary protocol, boundary service, stdio server crate, generated TypeScript boundary contract, transport-agnostic TypeScript boundary client, and stdio transport package are present as product code after this slice. See `docs/decisions/repo-consolidation-plan.md` and `docs/decisions/migration-ledger.md` for the migration plan.
