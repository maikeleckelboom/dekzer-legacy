# Dev Database Reset

After a greenfield substrate baseline schema change, an existing development SQLite database may fail
validation at startup:

```
database schema state is malformed:
database schema does not match the canonical substrate baseline:
compared index set mismatch:
```

This happens because the existing `library.sqlite3` was created against a previous baseline. The
canonical baseline has changed and the engine refuses to open a database that does not match.

The fix is to point the desktop host at a fresh user data directory so a new database is created from
the current baseline on first launch.

In development mode, the desktop host now defaults to `<repo>/.dev-user-data/default` for its library
storage root. This path is deterministic and repo-local, unaffected by Electron's own user data
directory. To reset it, use the dedicated storage commands.

## Using the dedicated storage commands

The desktop package provides commands to inspect and reset the development storage:

```powershell
# From workspace root (recommended)
pnpm run desktop:storage:status
pnpm run desktop:storage:reset -- --confirm-delete

# From desktop package
pnpm --filter @dekzer/desktop run storage:status
pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete
```

These commands use the existing Rust `library-boundary-stdio storage` command under the hood. They
target `<repo>/.dev-user-data/default` by default.

Reset is development-only and requires explicit confirmation via `--confirm-delete`. The command
will not run without it.

The resolved user data root is printed before the Rust command runs.

## Targeting a custom path with the storage commands

You can override the target path for a single invocation with `--user-data`:

```powershell
pnpm --filter @dekzer/desktop run storage:reset -- --user-data "D:\scratch\dev-user-data" --confirm-delete
```

The path must be absolute.

## Override via `DESKTOP_LIBRARY_USER_DATA_PATH`

The environment variable `DESKTOP_LIBRARY_USER_DATA_PATH` redirects the Rust store root without
touching Electron's own user data. It applies for the entire shell session and takes priority over
the default `<repo>/.dev-user-data/default` path.

```powershell
$env:DESKTOP_LIBRARY_USER_DATA_PATH = "C:\dev\dekzer\.dev-user-data\fresh"
pnpm --filter @dekzer/desktop run dev
```

The path must be absolute. The desktop host passes it through to the Rust stdio server as
`--user-data-path`, and the store derives `development/library.sqlite3` under it.

Once set, the variable applies for the rest of the shell session. To clear it:

```powershell
Remove-Item Env:DESKTOP_LIBRARY_USER_DATA_PATH
```

This override is intended for advanced or manual cases. For routine development, the default path
`<repo>/.dev-user-data/default` is the recommended target.

## Alternative: delete the dev-user-data directory directly

You can delete the default development storage directory directly:

```powershell
Remove-Item ".dev-user-data\default" -Recurse -Force
```

This targets the `<repo>/.dev-user-data/default` path from the repo root. The env override or
storage command `--user-data` flag are preferred because they are explicit and reversible -- you
keep the old database around in case you need to switch back to the previous baseline branch.

## What the engine does

- If the user data path is empty (no `library.sqlite3`), the Rust store runs the canonical baseline
  SQL and creates all tables, indexes, and seed rows.
- If the user data path contains a database, the store compares its schema against the canonical
  baseline. A mismatch is a hard error; the engine does not auto-delete or migrate data.
- No desktop code in the TypeScript layer performs data deletion on your behalf.
- The `storage:reset` command delegates to the Rust `library-boundary-stdio storage reset` command,
  which requires `--confirm-delete`. It is a development-only safety mechanism.
