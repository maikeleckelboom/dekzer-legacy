# Dekzer

Dekzer is the product workspace root.

Run workspace commands from this directory:

```bash
pnpm install
pnpm run dev:desktop
pnpm run typecheck
pnpm run build:desktop
```

Current workspace ownership:

- `apps/*` is for product applications.
- `packages/*` is for future JavaScript and TypeScript packages.
- `crates/*` is reserved for future Rust workspace ownership.
- `docs/*` is reserved for future product and architecture documents.

Only `apps/desktop` is present in this foundation pass.
