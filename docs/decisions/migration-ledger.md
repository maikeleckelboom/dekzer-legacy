# Migration Ledger

## Status

Active migration ledger. The first controlled `music-library-core` slice has moved: `crates/library-domain`.

## Workspace-Host Source Surfaces

| Source surface | Target Dekzer path | Decision | Owner after migration | Blockers | Validation required | Exit criterion |
| --- | --- | --- | --- | --- | --- | --- |
| `packages/workspace-language` | None in Dekzer unless product-specific language remains | Defer to Exclave evaluation or rewrite | Exclave if product-neutral; Dekzer only for product shell vocabulary | Must separate product shell meaning from generic topology language | Recreated public-surface and compiler tests | No `workspace-host` package name remains; one owner is chosen |
| `packages/workspace-core` | None in Dekzer for generic IR | Defer to Exclave evaluation | Exclave if generic | Need proof that IR carries no Dekzer product semantics | Rust/TS serialization and canonical model tests recreated | Generic IR is external dependency, not copied into Dekzer |
| `packages/workspace-compiler` | None in Dekzer unless product shell compiler is needed | Defer to Exclave evaluation | Exclave for generic compile/lower path | Must reject authored demo layout assumptions | Compiler boundary tests recreated | Dekzer consumes only a declared package, no dual compiler path |
| `packages/workspace-runtime` | None for generic runtime; `packages/workspace-shell` for product assembly only | Rewrite or defer | Exclave for runtime primitives; Dekzer for product composition | Need split between runtime policy and product shell | Realization baseline tests recreated without demo hosts | Runtime rules are outside desktop app code |
| `packages/layout-core` and `crates/layout-core` | None in Dekzer if generic | Defer to Exclave evaluation | Exclave | Need decide Rust vs TS solver ownership | Resize baseline and static solve tests recreated | Deterministic layout law has one external owner |
| `crates/workspace-core` | None in Dekzer if generic | Defer to Exclave evaluation | Exclave | Must not import product names | Normalized workspace tests recreated | Rust workspace model does not depend on Dekzer app |
| `crates/layout-wasm` | None in Dekzer if generic | Defer | Exclave | WASM boundary must match chosen solver owner | ABI and browser adapter tests | No Dekzer-owned WASM solver unless product-specific |
| `packages/grid-contract`, `packages/grid-model`, `packages/grid-realization`, `packages/grid-vue` | None initially | Defer or delete | Exclave only if still useful and generic | Need prove grid terms still belong in the product direction | Public surface, projection, virtualization tests if retained | Deleted from migration scope unless a real product surface asks for them |
| `docs/canon/frontend` | `docs/canon/product` only for still-current product law | Rewrite selected decisions | Dekzer docs | Must remove `workspace-host` as product boundary | Architecture review against repo-boundaries doc | Only current law remains; historical repo split is gone |
| `docs/archive/frontend` | None | Delete/defer as historical reference | None | Must not import archive as authority | None unless a specific invariant is rewritten | Archive is not moved into active Dekzer canon |
| Workspace runtime tests | New tests beside future owner packages | Rewrite conceptually | Exclave or Dekzer depending on owner | Need final package/crate owner | Recreated tests pass in owning repo | Invariants survive without copying source |

## Workspace-Host Do Not Move

| Source surface | Target Dekzer path | Decision | Owner after migration | Blockers | Validation required | Exit criterion |
| --- | --- | --- | --- | --- | --- | --- |
| `apps/desktop--draft` | None | Do not move | None | Demo desktop composition is non-authoritative | Confirm no files copied | Deleted from migration plan |
| `apps/desktop--draft/src/renderer/components/*Host.vue` | None | Do not move | None | Prototype navigation, library, details, and pane hosts would import demo UI truth | Confirm Dekzer surfaces are rewritten from canon | No draft host component exists in Dekzer |
| `apps/desktop--draft/src/renderer/library/desktopLibrarySurface.ts` | None | Do not move | None | Draft library wiring mixes desktop prototype with external library repo | Real library boundary tests in Dekzer later | No fake library surface import path |
| `apps/desktop--draft/src/renderer/workspace/*` | `packages/workspace-shell` only through rewrite | Do not copy | Dekzer only after rewrite | Product shell must be built against real owners | Future desktop boot and typecheck | No copied authored demo workspace |
| `apps/desktop--draft/tests/*` | Future tests only by rewritten behavior | Rewrite conceptually | Dekzer test suite later | Test names reference draft behavior | New tests prove real imports | No test depends on draft app shell |
| `crates/library-surface-stdio` in `workspace-host` | `crates/library-boundary-stdio` only if rewritten | Do not move | Dekzer later if needed | It belongs to draft integration history | Future boundary service integration tests | No stdio adapter copied from `workspace-host` |
| `dist`, `.tmp-fixtures`, `node_modules`, generated artifacts | None | Delete | None | Generated/build output cannot become source | `git status` and clean build generation later | No generated output committed by migration |
| Demo routes, sample layouts, labs, screenshots, playgrounds, fixtures | None | Delete | None | Non-authoritative product behavior | None | Not present in Dekzer |
| `@dekzer/workspace-host` package identity | None | Delete name | None | Misnames a historical repo as product owner | Search for old name after migration | Name absent from active Dekzer paths |

## Music-Library-Core Source Surfaces

| Source surface | Target Dekzer path | Decision | Owner after migration | Blockers | Validation required | Exit criterion |
| --- | --- | --- | --- | --- | --- | --- |
| `crates/library-domain` | `crates/library-domain` | Moved in first substrate slice | Dekzer library substrate | None; leaf crate with no local or external dependencies | `cargo metadata --format-version 1 --no-deps`; `cargo fmt --all --check`; `cargo test -p library-domain`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `pnpm run check` | Crate builds in Dekzer with no old repo identity and root Rust gates are active |
| `crates/library-sqlite` | `crates/library-store-sqlite` | Moved in second substrate slice | Dekzer Rust/SQLite substrate | None; depends locally only on `library-domain`; contains store-level read DTO/query mechanics and narrow root-scan materialization already owned by the old store crate | Store tests, schema bootstrap, root scan, reopen tests | Durable store passes without renderer access |
| `migrations/20260502000000_substrate_baseline.sql` | `crates/library-store-sqlite/migrations/20260502000000_substrate_baseline.sql` | Moved in second substrate slice | Dekzer Rust/SQLite substrate | None | Bootstrap and schema comparison tests | Baseline schema has one owner under the store crate |
| `crates/library-read-kernel` | `crates/library-read-model` | Move and rename | Dekzer read model owner | Depends on store and domain move | Literal hierarchy, navigation, browser, waveform, prep detail read tests | Read models query durable state only |
| `crates/library-surface-protocol` | `crates/library-boundary-protocol` | Move and rename | Dekzer boundary protocol | Need replace `surface` vocabulary | Contract generation and protocol tests | Rust protocol is source of generated TS contract |
| `crates/library-surface-service` | `crates/library-boundary-service` | Move and rename | Dekzer boundary service | Needs domain, store, read model, protocol in place | Register root, run scan, literal hierarchy, service reopen tests | Service maps requests to substrate owners |
| Missing explicit authority layer | `crates/library-authority` | Add only when behavior demands it | Dekzer library authority | Must avoid broad abstraction | Operation-level tests for product commands | Authority removes hidden ownership, not just indirection |
| `packages/library-surface-contract` | `packages/library-boundary-contract` | Regenerate, do not copy generated output | Dekzer generated TS boundary | Need protocol crate moved first | Export/check contract, TS build/typecheck | Generated-only package uses `@dekzer` scope |
| `packages/library-surface-client` | `packages/library-boundary-client` | Move and rename | Dekzer boundary client | Depends on generated contract rename | Client/session tests, runtime smoke | No DTO forks, no old package scope |
| `packages/library-surface-client/fixtures/session-validation.ts` | Boundary client tests | Rewrite or move with rename | Dekzer boundary client tests | Fixture names and package scope must change | Session lifecycle, pump, listener, monotonic revision tests | Tests prove real client behavior without fake product rows |
| `xtask` contract tooling | `crates/library-tooling` or root `xtask` after decision | Defer | Dekzer tooling | Need repo-wide tooling convention | Export/check generated contract tests | One generation command, no stale artifacts |
| Active docs under `docs/` | `docs/canon/product` and `docs/decisions` | Rewrite selected docs | Dekzer docs | Need remove old repo names and archive clutter | Doc review against repo-boundaries | Current docs use Dekzer target names |
| `docs/archive/**`, repo bundles, generated source archives | None | Delete or leave behind | None | Too much historical noise for active canon | None | Not imported into Dekzer |
| `package-lock.json`, npm workspace root | None | Delete instead of moving | None | Dekzer uses pnpm | `pnpm install` in Dekzer when package deps change | No npm lock in Dekzer |

## Current Migrated Slice

### `library-domain`

| Field | Value |
| --- | --- |
| Source path | `C:\dev\music-stack\music-library-core\crates\library-domain` |
| Target path | `C:\dev\dekzer\crates\library-domain` |
| Owner after migration | Dekzer library substrate |
| Classification | Leaf crate; pure product vocabulary and invariants; no local crate dependencies; no SQLite; no filesystem scanning; no async runtime; no boundary protocol; no generated TypeScript; no app wiring |
| Validation run | Passed: `cargo metadata --format-version 1 --no-deps`; `cargo fmt --all --check`; `cargo test -p library-domain`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `pnpm run typecheck`; `pnpm run check`; `pnpm run build:desktop`; `git diff --check` |
| Blockers discovered | None |
| Next migration slice | Completed by the second substrate slice: `crates/library-sqlite` moved to `crates/library-store-sqlite` with its owned SQLite substrate and required migration |
| Exit criterion satisfied | Yes: crate builds in Dekzer with no old repo identity and active root Rust quality gates |

### `library-store-sqlite`

| Field | Value |
| --- | --- |
| Source path | `C:\dev\music-stack\music-library-core\crates\library-sqlite` |
| Target path | `C:\dev\dekzer\crates\library-store-sqlite` |
| Owner after migration | Dekzer Rust/SQLite durable library substrate |
| Classification | Safe as the second migration slice: one local dependency (`library-domain`), no dependency on read-kernel/read-model crate, boundary protocol, boundary service, generated TypeScript, desktop, renderer, or `workspace-host`; owns schema/bootstrap, canonical schema validation, SQLite connection policy, write admission, durable source/source-location/source-directory/source-file persistence, work/artifact/playlist persistence, store-level projection tables, and a root-scan materialization operation already present in the old store crate |
| Local dependencies | `library-domain = { path = "../library-domain" }` |
| External dependencies | `rusqlite` with `bundled`, `hooks`, and `trace`; `serde_json`; `sha2`; `thiserror`; `walkdir`; dev dependency `tempfile` |
| Migrations copied | `migrations/20260502000000_substrate_baseline.sql` copied to `crates/library-store-sqlite/migrations/20260502000000_substrate_baseline.sql`; schema include path now resolves inside the store crate |
| Tests copied | Store-owned in-crate tests under `crates/library-store-sqlite/src/**`; source crate had no separate `crates/library-sqlite/tests/**` directory |
| Crate rename | Cargo package renamed from `library-sqlite` to `library-store-sqlite`; crate target is `library_store_sqlite`; no alias crate, wrapper, or dual path was created |
| Old identity removal | The active Dekzer crate no longer uses the `library-sqlite` package identity. Old `surface` wording copied in store comments/tests was replaced with boundary/API wording where it meant the historical boundary naming, and app-owned file-store root kind was renamed from `library_sqlite_state` to `library_store_sqlite_state` for the new store owner |
| Validation run | Passed: `cargo metadata --format-version 1 --no-deps`; `cargo fmt --all --check`; `cargo test -p library-store-sqlite`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `pnpm run typecheck`; `pnpm run check`; `pnpm run build:desktop`; `pnpm run test`; `git diff --check` |
| Blockers discovered | Cargo dependency resolution on Windows required `CARGO_HTTP_CHECK_REVOKE=false` once to fetch the crates.io index because Schannel could not check certificate revocation; after dependencies were fetched, the required Cargo commands ran normally |
| Next migration slice | `crates/library-read-kernel` to `crates/library-read-model`, updating it to depend on `library-store-sqlite` and preserving only read-model/query composition ownership |
| Exit criterion satisfied | Yes: durable SQLite substrate, schema bootstrap, migrations, store tests, root scan materialization, literal hierarchy persistence, and reopen behavior are owned by the Dekzer store crate and validated without renderer or desktop SQLite access |

## Exit Gate For This Ledger

The control-plane portion of this ledger is complete. Ongoing migration slices remain accepted only when:

- `workspace-host` demo code is explicitly classified as non-authoritative
- `music-library-core` surfaces have target owners and validation gates
- copied source code belongs to the named target owner and passes its validation gate
- Dekzer root docs define the final two-repo direction
- Dekzer boot status remains honest about unimported subsystems
