# Dekzer Desktop

Dekzer Desktop is the Electron shell for the Dekzer product workspace.

Run it from the workspace root at `C:\dev\dekzer`.

## Root Commands

```bash
pnpm install
pnpm run dev:desktop
pnpm run typecheck
pnpm --filter @dekzer/desktop run validate:host
pnpm run build:desktop
```

This package intentionally contains the bootable desktop shell plus the lazy main-process library boundary host owner. The host does not expose renderer IPC, start the Rust stdio process automatically, or register library roots.
