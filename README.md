# Dekzer

Dekzer is the product monorepo root.

Current state:

- `apps/desktop` is the booting Electron/Vue desktop app.
- `crates/library-domain` is the first migrated Rust library substrate crate.
- `crates/library-store-sqlite` is the migrated durable SQLite store crate.
- `crates/library-boundary-protocol` is the migrated Rust boundary DTO/protocol crate.
- `crates/library-boundary-service` maps boundary protocol commands to the real SQLite store.
- `workspace-host` is not imported.
- Remaining `music-library-core` slices beyond the domain, SQLite store, boundary protocol, and boundary service are not imported.
- Exclave is an external dependency candidate, not vendored into this repo.

Run workspace commands from this directory:

```bash
pnpm install
pnpm run dev:desktop
pnpm run typecheck
pnpm run check
pnpm run test
pnpm run test:rust
pnpm run fmt:rust
pnpm run lint:rust
pnpm run build:desktop
```

Current root scripts:

- `dev:desktop` starts the desktop app.
- `build:desktop` builds the desktop app.
- `typecheck` runs the desktop typecheck.
- `test` runs the Rust workspace test suite.
- `test:rust` runs `cargo test --workspace`.
- `fmt:rust` runs `cargo fmt --all --check`.
- `lint:rust` runs `cargo clippy --workspace --all-targets -- -D warnings`.
- `check` runs the desktop typecheck plus Rust fmt, clippy, and test gates.

Workspace ownership:

- `apps/*` is for product applications.
- `packages/*` is for future JavaScript and TypeScript packages.
- `crates/*` is for Rust workspace ownership, starting with `crates/library-domain`.
- `docs/*` is for product canon, decisions, and architecture documents.

Only `apps/desktop` and the migrated Rust library domain, store, and boundary protocol crates are present as product code after this slice. See `docs/decisions/repo-consolidation-plan.md` and `docs/decisions/migration-ledger.md` for the migration plan.
