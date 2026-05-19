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
- `workspace-host` is not imported.
- Remaining `music-library-core` slices beyond the domain, SQLite store, boundary protocol, boundary service, and generated boundary contract are not imported.
- Exclave is an external dependency candidate, not vendored into this repo.

Run workspace commands from this directory:

```bash
pnpm install
pnpm run dev:desktop
pnpm run library:contract:export
pnpm run library:contract:check
pnpm run typecheck
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
- `typecheck` runs the generated boundary contract package typecheck and desktop typecheck.
- `build` builds the generated boundary contract package and desktop app.
- `test` runs the Rust workspace test suite.
- `test:rust` runs `cargo test --workspace`.
- `fmt:rust` runs `cargo fmt --all --check`.
- `lint:rust` runs `cargo clippy --workspace --all-targets -- -D warnings`.
- `check` runs the boundary contract stale check, TypeScript typechecks, Rust fmt, clippy, and test gates.

Workspace ownership:

- `apps/*` is for product applications.
- `packages/*` is for JavaScript and TypeScript packages, starting with the generated-only library boundary contract.
- `crates/*` is for Rust workspace ownership, starting with `crates/library-domain`.
- `docs/*` is for product canon, decisions, and architecture documents.

Only `apps/desktop`, the migrated Rust library domain, store, boundary protocol and service crates, and the generated TypeScript boundary contract are present as product code after this slice. See `docs/decisions/repo-consolidation-plan.md` and `docs/decisions/migration-ledger.md` for the migration plan.
