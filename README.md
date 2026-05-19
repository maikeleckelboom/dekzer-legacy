# Dekzer

Dekzer is the product monorepo root.

Current state:

- `apps/desktop` is the booting Electron/Vue desktop app.
- `workspace-host` is not imported.
- `music-library-core` is not imported.
- Exclave is an external dependency candidate, not vendored into this repo.

Run workspace commands from this directory:

```bash
pnpm install
pnpm run dev:desktop
pnpm run typecheck
pnpm run check
pnpm run build:desktop
```

Current root scripts:

- `dev:desktop` starts the desktop app.
- `build:desktop` builds the desktop app.
- `typecheck` runs the desktop typecheck.
- `check` currently runs `typecheck`.

There is intentionally no root `test` script yet. It will be added when real package or crate tests exist in this repo.

Workspace ownership:

- `apps/*` is for product applications.
- `packages/*` is for future JavaScript and TypeScript packages.
- `crates/*` is reserved for future Rust workspace ownership.
- `docs/*` is for product canon, decisions, and architecture documents.

Only `apps/desktop` is present as product code in this control-plane pass. See `docs/decisions/repo-consolidation-plan.md` and `docs/decisions/migration-ledger.md` for the migration plan.
