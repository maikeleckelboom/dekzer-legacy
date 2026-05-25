# Dev Database Lifecycle

Desktop development storage is repo-local by default:

```text
<repo>/.dev-user-data/default
```

The Rust store derives the development database at `development/library.sqlite3` under that user
data root. Production storage comes from Electron's `app.getPath("userData")` and is not reset by
development commands.

## Normal Dev

Use normal dev startup when you want to keep the current development database:

```powershell
pnpm run desktop:dev
pnpm --filter @dekzer/desktop run dev
```

`dev` is non-destructive.

## Fresh Dev

Use fresh dev startup when a schema baseline changed or you explicitly want a clean development
store:

```powershell
pnpm run desktop:dev:fresh
pnpm --filter @dekzer/desktop run dev:fresh
```

`dev:fresh` runs the development storage reset command with internal `--confirm-delete`, then starts
normal dev. It targets only development storage.

## Inspect Storage

```powershell
pnpm run desktop:storage:status
pnpm run desktop:storage:doctor

pnpm --filter @dekzer/desktop run storage:status
pnpm --filter @dekzer/desktop run storage:doctor
```

`storage:status` prints the resolved user data path, its source, and the Rust-owned status JSON.
`storage:doctor` is non-destructive. It prints the resolved target, checks whether the development
database exists, and reports the Rust-owned schema compatibility status: `missing`, `compatible`,
`incompatible`, or `unreadable`. Missing databases are created by normal dev startup; incompatible
or unreadable development databases can be reset explicitly.

## Reset Storage

```powershell
pnpm run desktop:storage:reset -- --confirm-delete
pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete
```

Reset delegates to `library-boundary-stdio storage reset` and deletes only the derived development
storage directory, including the development SQLite database, sidecars, and app-owned artifact file
store. It does not delete production storage and does not delete user music files.

## Target Resolution

Storage commands resolve the user data path in this order:

1. `--user-data <absolutePath>` for one storage command invocation.
2. `DESKTOP_LIBRARY_USER_DATA_PATH` for the current shell session.
3. `<repo>/.dev-user-data/default`.

Example single-command override:

```powershell
pnpm --filter @dekzer/desktop run storage:doctor -- --user-data "D:\scratch\dev-user-data"
pnpm --filter @dekzer/desktop run storage:reset -- --user-data "D:\scratch\dev-user-data" --confirm-delete
```

Example session override used by both storage commands and desktop dev:

```powershell
$env:DESKTOP_LIBRARY_USER_DATA_PATH = "C:\dev\dekzer\.dev-user-data\fresh"
pnpm run desktop:storage:doctor
pnpm run desktop:dev
Remove-Item Env:DESKTOP_LIBRARY_USER_DATA_PATH
```

The path must be absolute.
