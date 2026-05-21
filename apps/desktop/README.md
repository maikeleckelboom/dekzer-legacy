# Dekzer Desktop

Dekzer Desktop is the Electron shell for the Dekzer product workspace.

Run it from the workspace root at `C:\dev\dekzer`.

## Root Commands

```bash
pnpm install
pnpm run dev:desktop
pnpm run typecheck
pnpm run desktop:validate
pnpm run build:desktop
```

This package intentionally contains the bootable desktop shell plus the lazy main-process library boundary host owner. The renderer API exposes narrow host status, hierarchy read, and local-root commands; storage path selection remains in main.
