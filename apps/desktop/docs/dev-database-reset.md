# Dev Database Reset

After a greenfield substrate baseline schema change, an existing development SQLite database may fail validation at startup:

```
database schema state is malformed:
database schema does not match the canonical substrate baseline:
compared index set mismatch:
```

This happens because the existing `library.sqlite3` was created against a previous baseline. The
canonical baseline has changed and the engine refuses to open a database that does not match.

The fix is to point the desktop host at a fresh user data directory so a new database is created from
the current baseline on first launch.

## Using `DESKTOP_LIBRARY_USER_DATA_PATH`

The supported override is the `DESKTOP_LIBRARY_USER_DATA_PATH` environment variable. It redirects
the Rust store root without touching Electron's own user data.

Set it to an absolute path that does not contain an existing `library.sqlite3`:

```powershell
$env:DESKTOP_LIBRARY_USER_DATA_PATH="C:\dev\dekzer\.dev-user-data\fresh"
pnpm --filter @dekzer/desktop run dev
```

The path must be absolute. The desktop host passes it through to the Rust stdio server as
`--user-data-path`, and the store derives `development/library.sqlite3` under it.

Once set, the variable applies for the rest of the shell session. To clear it:

```powershell
Remove-Item Env:DESKTOP_LIBRARY_USER_DATA_PATH
```

## Alternative: delete Electron userData

You can also delete the previous development database directly from Electron's user data directory.
The exact path depends on the Electron `app.getPath('userData')` resolution.

```powershell
# Typical development path
Remove-Item "$env:LOCALAPPDATA\dekzer-dev\library\development" -Recurse -Force
pnpm --filter @dekzer/desktop run dev
```

The env override is preferred because it is explicit and reversible — you keep the old database around
in case you need to switch back to the previous baseline branch.

## What the engine does

- If the user data path is empty (no `library.sqlite3`), the Rust store runs the canonical baseline
  SQL and creates all tables, indexes, and seed rows.
- If the user data path contains a database, the store compares its schema against the canonical
  baseline. A mismatch is a hard error; the engine does not auto-delete or migrate data.
- No desktop code in the TypeScript layer performs data deletion on your behalf.
