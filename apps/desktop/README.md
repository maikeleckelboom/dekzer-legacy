# Dekzer Desktop

Dekzer Desktop is the Electron shell for the Dekzer product workspace.

Run it from the workspace root at `C:\dev\dekzer`.

## Root Commands

```bash
pnpm install
pnpm run desktop:dev
pnpm run desktop:dev:fresh
pnpm run typecheck
pnpm run desktop:test
pnpm run desktop:verify
pnpm run build:desktop
```

This package intentionally contains the bootable desktop shell plus the lazy main-process library boundary host owner. The renderer API exposes narrow host status, hierarchy read, and local-root commands; storage path selection remains in main.

## Developer docs

- [Development](../../docs/development.md) covers requirements, verification, storage status, doctor, and reset.
- [TypeScript absence semantics](./docs/typescript-absence-semantics.md) covers `undefined` versus `null` at the
  boundary.
