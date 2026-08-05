# Development

Everything needed to run, verify, and reset the repository. Commands here are checked against `package.json` and
`apps/desktop/package.json`.

## Requirements

- **Windows.** The only supported V0 path. Source intake resolves Windows known folders and volumes.
- **Node.js 25 or newer.** Enforced by the root `engines` field.
- **pnpm 10.33.2.** Pinned by `packageManager`. Run `corepack enable` if pnpm is not already available.
- **A stable Rust toolchain with Rust 2024 edition support.** Cargo is invoked by development preflight, contract
  export, and several test scripts.
- **An interactive desktop session** for the Electron end-to-end tests. They launch a real window and will not run
  headless.

## Install and run

```bash
pnpm install
```

```bash
pnpm desktop:dev
```

`desktop:dev` is non-destructive and keeps the current development database. Preflight invokes Cargo to check storage
compatibility and builds the Rust stdio server when needed.

```bash
pnpm desktop:dev:fresh
```

`desktop:dev:fresh` resets development storage first, then starts normally. Use it after a schema baseline change or
when a clean store is wanted.

## Verify

```bash
pnpm verify
```

This is the pre-merge gate. In order it runs `git diff --check`, both generated-contract staleness checks, TypeScript
and Vue type checking across every package, the desktop Vitest suite, the boundary client tests, the stdio transport
tests against a compiled Rust server, the full Cargo test suite, ESLint, Prettier, `cargo fmt --check`, and Clippy with
warnings denied.

```bash
pnpm --filter @dekzer/desktop verify:e2e:library-v0
```

The focused Electron acceptance gate. It is separate from `pnpm verify` because it builds the Rust server, builds the
desktop app, and launches Electron. It covers launch, source admission and scan, scoped search, contents pagination,
restart persistence, missing-source recovery, and remove-and-re-add freshness, using isolated storage and generated WAV
fixtures. It proves the covered workflow, not broad real-media compatibility.

Narrower entry points exist when the full gate is more than you need:

```bash
pnpm typecheck
```

```bash
pnpm test
```

```bash
pnpm lint
```

## Fix versus verify

Verification commands report. Fix commands write.

```bash
pnpm fix
```

`fix` runs Prettier and ESLint with `--fix` across the desktop app, then `cargo fmt --all`. `pnpm fix:ts` and
`pnpm fix:rust` run each half. `pnpm format:check` and `pnpm lint` only report, and are what `pnpm verify` uses.

Run `pnpm fix` before `pnpm verify` when a run fails on formatting. It will not fix type errors, failing tests, or
Clippy findings that need a real change.

## Generated contracts

The Rust boundary protocol is the source of truth for cross-language types. The TypeScript contract package and JSON
Schema are generated from it and committed.

```bash
pnpm library:contract:export
```

```bash
pnpm library:stdio:contract:export
```

Export regenerates the checked-in output. The matching `:check` variants compare the current Rust source against what
is committed and fail when they differ. Both checks run inside `pnpm verify`, so a protocol change that skips export
fails the build rather than reaching the renderer as a silent mismatch.

After changing anything in `crates/library-boundary-protocol`, run both export commands and commit the result. Do not
hand-edit files under `packages/library-boundary-contract`.

## Development storage

Development storage is repo-local:

```text
<repo>/.dev-user-data/default
```

The Rust store derives the development database at `development/library.sqlite3` under that root. Production storage
comes from Electron's `userData` path and is never touched by development commands.

```bash
pnpm desktop:storage:status
```

Prints the resolved user data path, where that path came from, and the Rust-owned status JSON.

```bash
pnpm desktop:storage:doctor
```

Non-destructive. Reports whether the development database exists and its schema compatibility: `missing`,
`compatible`, `incompatible`, or `unreadable`. A missing database is created by normal dev startup. An incompatible or
unreadable one needs an explicit reset.

```bash
pnpm desktop:storage:reset -- --confirm-delete
```

Deletes only the derived development storage directory: the development SQLite database, its sidecars, and the
app-owned artifact file store. It does not touch production storage and does not touch music files. The
`--confirm-delete` flag is required, and `desktop:dev:fresh` passes it internally.

Storage commands resolve their target in this order:

1. `--user-data <absolutePath>` for a single invocation
2. `DESKTOP_LIBRARY_USER_DATA_PATH` for the shell session
3. `<repo>/.dev-user-data/default`

The path must be absolute.

## Environment variables

| Variable                               | Effect                                                                                                          |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `DESKTOP_LIBRARY_USER_DATA_PATH`       | Overrides the development user data root for storage commands and desktop dev. Must be absolute.                |
| `DEKZER_LIBRARY_BOUNDARY_STDIO_BINARY` | Overrides the path to the Rust boundary stdio executable. Useful when running against a binary built elsewhere. |

## Limitations

**Packaged builds do not include the Rust backend executable.** `pnpm build` and the `electron-builder` targets
produce a desktop bundle, but the application currently runs from source for real use. Treat packaging output as
incomplete.

**Windows only.** Non-Windows platforms return typed unsupported outcomes from local browse entry point resolution.
The rest of the stack is not exercised elsewhere.

**End-to-end tests need a real desktop session.** They will fail in a headless CI container without a display.

**`pnpm verify` does not run the Electron acceptance gate.** Run
`pnpm --filter @dekzer/desktop verify:e2e:library-v0` separately when changes touch source admission, scanning,
search, pagination, or persistence.

**Cargo is required even for TypeScript-only work,** because contract checks and the stdio transport tests build Rust.
