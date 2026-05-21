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
- `apps/desktop/src/main` owns the lazy desktop library boundary host policy for stdio binary selection, Electron `userData`, environment selection, diagnostics routing, readiness, and shutdown lifecycle.
- `workspace-host` is not imported.
- Remaining `music-library-core` slices beyond the domain, SQLite store, boundary protocol, boundary service, stdio transport, generated boundary contract, TypeScript boundary client, and desktop host owner are not imported.
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
pnpm --filter @dekzer/desktop run validate:library-boundary-host
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
- `pnpm --filter @dekzer/desktop run validate:library-boundary-host` validates the desktop main-process library boundary host config/error policy without launching Electron or requiring the Rust stdio binary.
- `typecheck` runs the generated boundary contract package typecheck, boundary client typecheck, stdio transport typecheck, and desktop typecheck.
- `build` builds the generated boundary contract package, boundary client, stdio transport package, and desktop app.
- `test` runs the boundary client/session validation fixture, stdio transport validation and cross-process smoke fixtures, and the Rust workspace test suite.
- `test:rust` runs `cargo test --workspace`.
- `fmt:rust` runs `cargo fmt --all --check`.
- `lint:rust` runs `cargo clippy --workspace --all-targets -- -D warnings`.
- `check` runs the boundary contract stale check, stdio transport contract stale check, TypeScript typechecks, desktop host validation, boundary client and stdio validation fixtures, Rust fmt, clippy, and test gates.

Stdio readiness:

- `crates/library-boundary-stdio` emits `{"type":"ready","server":"libraryBoundaryStdio"}` on stdout only after CLI parsing and `LibraryBoundaryService::open(...)` succeed.
- `packages/library-boundary-stdio-transport` consumes that Rust-owned ready envelope contract and exposes `LibraryBoundaryStdioTransport.ready`.
- The desktop main-process host awaits transport readiness before exposing a `LibraryBoundaryClient`; preload, renderer, IPC, file picker, root registration, scans, and event pumping remain unwired.

Desktop library storage:

- Production resolves the library user data root from Electron main's `app.getPath("userData")`; the Rust store derives `library.sqlite3` under that root.
- Development resolves the same host-owned user data root, marks the store environment as `development`, and the Rust store derives `development/library.sqlite3` under that root so renderer HMR does not relocate storage.
- `DEKZER_LIBRARY_USER_DATA_PATH` may override the user data root for local diagnostics. The path must be absolute, and the override is resolved only in `apps/desktop/src/main/libraryBoundary/config.ts`.
- `library-boundary-stdio storage status --user-data <path>` prints the derived development and production database, sidecar, WAL, and SHM paths without opening or resetting SQLite.
- `library-boundary-stdio storage reset --user-data <path> --confirm-delete` deletes only the derived development storage directory. Reset is not automatic and is not exposed through the renderer.

Workspace ownership:

- `apps/*` is for product applications.
- `packages/*` is for JavaScript and TypeScript packages, starting with the generated-only library boundary contract, transport-agnostic library boundary client, and stdio transport.
- `crates/*` is for Rust workspace ownership, starting with `crates/library-domain`.
- `docs/*` is for product canon, decisions, and architecture documents.

Only `apps/desktop`, its main-process library boundary host owner, the migrated Rust library domain, store, boundary protocol, boundary service, stdio server crate, generated TypeScript boundary contract, transport-agnostic TypeScript boundary client, and stdio transport package are present as product code after this slice. See `docs/decisions/repo-consolidation-plan.md` and `docs/decisions/migration-ledger.md` for the migration plan.
