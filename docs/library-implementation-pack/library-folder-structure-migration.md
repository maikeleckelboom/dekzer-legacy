# Library Folder Structure Migration Plan

## Status

Status: implementation companion
Owner: Electron library boundary and renderer architecture
Companion architecture doc: `docs/architecture/library-folder-structure.md`

This document defines the mechanical migration from the current Electron library folder layout to the ratified
`main/library/` and `shared/library/` structure.

The migration is behavior-preserving. It removes the previous structure rather than supporting it.

## Migration goal

Move the Electron library domain into canonical domain-grouped folders:

- `src/main/library/`
- `src/shared/library/`

Centralize library IPC channel ownership under:

- `src/shared/library/boundary/controlPlane.ts`
- `src/shared/library/boundary/publicationPlane.ts`

Reserve boundary command registration ownership with:

- `src/main/library/boundary/commandRegistry.ts`

Do not reorganize `src/renderer/library/` beyond import updates.

## Non-goals

Do not implement the boundary spine behavior in this migration.

Do not move command registration behavior into `commandRegistry.ts` yet.

Do not change scan, hierarchy, contents, navigation, roots, source lifecycle, source maintenance, hashing, track
identity, attachment identity, view-state, renderer state, or preload behavior.

Do not change IPC channel string values.

Do not introduce compatibility modules.

Do not add aliases for old paths.

Do not leave old folders behind.

Do not perform latency fixes, icon migration, UI cleanup, formatting-only churn, or broad refactors in this pass.

## Allowed changes

This migration may do only the following:

- move files to canonical folders
- rename files according to the move map
- update import paths
- relocate channel constants into `shared/library/boundary/controlPlane.ts`
- relocate publication channel constants into `shared/library/boundary/publicationPlane.ts`
- add `shared/library/boundary/rendererApi.ts`
- update `shared/rendererApi.ts` to compose `LibraryApi`
- add `main/library/boundary/commandRegistry.ts` as an inert ownership stub
- update tests and test support imports
- update current docs that describe active folder structure

## Forbidden changes

This migration must not:

- leave old-path re-export files
- add wrapper modules for previous names
- add compatibility barrels
- preserve previous import paths
- change channel string literal values
- change request/reply/result shape semantics
- change runtime behavior
- change renderer interaction behavior
- change service/protocol behavior
- introduce duplicate channel constants
- move renderer folders into a main/shared mirror
- create broad barrels that hide ownership
- introduce comments that describe previous paths as supported alternatives

## Command registry stub

Create this file:

`apps/desktop/src/main/library/boundary/commandRegistry.ts`

Content:

```ts
// Reserved: library IPC command handler registration lives here.
// Domain handlers register through this spine, not via ipcMain.handle directly.
export {}
```

This stub is intentionally inert. It reserves ownership only.

## Channel ownership

### Control plane

Create:

`apps/desktop/src/shared/library/boundary/controlPlane.ts`

It owns every library command/read IPC channel string.

Use grouped names with `as const`.

The values must be copied exactly from the current codebase.

```ts
export const libraryControlChannels = {
  attachmentIdentity: {
    read: '<existing string value>',
  },
  contents: {
    read: '<existing string value>',
  },
  hierarchy: {
    read: '<existing string value>',
  },
  navigation: {
    read: '<existing string value>',
  },
  roots: {
    cancel: '<existing string value>',
    chooseLocal: '<existing string value>',
    read: '<existing string value>',
    register: '<existing string value>',
    scan: '<existing string value>',
    unregister: '<existing string value>',
  },
  source: {
    fileHashing: '<existing string value>',
    lifecycle: '<existing string value>',
    maintenance: '<existing string value>',
  },
  trackIdentity: {
    candidates: '<existing string value>',
    decisions: '<existing string value>',
  },
  viewState: {
    read: '<existing string value>',
    write: '<existing string value>',
  },
} as const
```

The placeholder strings above are not implementation values. The implementation must use the existing literal channel
strings.

### Publication plane

Create:

`apps/desktop/src/shared/library/boundary/publicationPlane.ts`

It owns every library publication/event IPC channel string.

Use grouped names with `as const`.

Do not mix publication channels into `controlPlane.ts`.

### Domain files after channel relocation

After migration, shared domain files own shapes only.

Examples:

- `shared/library/roots/register.ts` owns renderer-facing register request/result types.
- `shared/library/viewState/persistence.ts` owns persisted view-state payload/result types.
- `shared/library/contents/read.ts` owns renderer-facing contents read types.

Shared domain files do not define channel constants.

## Renderer API split

Create:

`apps/desktop/src/shared/library/boundary/rendererApi.ts`

It owns the library-domain API surface type currently embedded in the app-level renderer API.

Update:

`apps/desktop/src/shared/rendererApi.ts`

so it imports the library-domain API surface and composes the root `RendererApi`.

The runtime API shape exposed to the renderer must not change.

Preload still exposes one `window.dekzer` API. Renderer code still consumes the same app-level API shape.

## Move map

Use this map as the canonical migration source. Do not keep files at old locations.

### Main layer

| Pre-migration path                                           | Canonical path                                 |
|--------------------------------------------------------------|------------------------------------------------|
| `src/main/libraryAttachmentIdentity/read.ts`                 | `src/main/library/attachmentIdentity/read.ts`  |
| `src/main/libraryBoundary/config.ts`                         | `src/main/library/boundary/config.ts`          |
| `src/main/libraryBoundary/errors.ts`                         | `src/main/library/boundary/errors.ts`          |
| `src/main/libraryBoundary/eventPump.ts`                      | `src/main/library/boundary/eventPump.ts`       |
| `src/main/libraryBoundary/host.ts`                           | `src/main/library/boundary/host.ts`            |
| `src/main/libraryBoundary/status.ts`                         | `src/main/library/boundary/status.ts`          |
| `src/main/libraryContents/read.ts`                           | `src/main/library/contents/read.ts`            |
| `src/main/libraryHierarchy/mapping.ts`                       | `src/main/library/hierarchy/mapping.ts`        |
| `src/main/libraryHierarchy/readChildren.ts`                  | `src/main/library/hierarchy/read.ts`           |
| `src/main/libraryHierarchy/request.ts`                       | `src/main/library/hierarchy/request.ts`        |
| `src/main/libraryHierarchy/target.ts`                        | `src/main/library/hierarchy/target.ts`         |
| `src/main/libraryNavigation/readRows.ts`                     | `src/main/library/navigation/read.ts`          |
| `src/main/libraryRoots/cancelScan.ts`                        | `src/main/library/roots/cancel.ts`             |
| `src/main/libraryRoots/chooseAndRegisterLocal.ts`            | `src/main/library/roots/chooseLocal.ts`        |
| `src/main/libraryRoots/readLocalRoots.ts`                    | `src/main/library/roots/read.ts`               |
| `src/main/libraryRoots/registerLocalRoot.ts`                 | `src/main/library/roots/register.ts`           |
| `src/main/libraryRoots/runScan.ts`                           | `src/main/library/roots/scan.ts`               |
| `src/main/libraryRoots/unregisterLocalRoot.ts`               | `src/main/library/roots/unregister.ts`         |
| `src/main/librarySourceFileHashing/hashSourceFilesBlake3.ts` | `src/main/library/source/fileHashing.ts`       |
| `src/main/librarySourceLifecycle/readSourceLifecycle.ts`     | `src/main/library/source/lifecycle.ts`         |
| `src/main/librarySourceMaintenance/sourceMaintenance.ts`     | `src/main/library/source/maintenance.ts`       |
| `src/main/libraryTrackIdentityReview/candidates.ts`          | `src/main/library/trackIdentity/candidates.ts` |
| `src/main/libraryTrackIdentityDecisions/decisionCommands.ts` | `src/main/library/trackIdentity/decisions.ts`  |
| `src/main/libraryViewState/viewState.ts`                     | `src/main/library/viewState/persistence.ts`    |

Add:

| New path                                       | Purpose                                                    |
|------------------------------------------------|------------------------------------------------------------|
| `src/main/library/boundary/commandRegistry.ts` | Inert ownership stub for future command registration spine |

### Shared layer

| Pre-migration path                                             | Canonical path                                   |
|----------------------------------------------------------------|--------------------------------------------------|
| `src/shared/libraryAttachmentIdentity/read.ts`                 | `src/shared/library/attachmentIdentity/read.ts`  |
| `src/shared/libraryBoundary/eventParser.ts`                    | `src/shared/library/boundary/eventParser.ts`     |
| `src/shared/libraryBoundary/events.ts`                         | `src/shared/library/boundary/events.ts`          |
| `src/shared/libraryBoundary/status.ts`                         | `src/shared/library/boundary/status.ts`          |
| `src/shared/libraryContents/read.ts`                           | `src/shared/library/contents/read.ts`            |
| `src/shared/libraryHierarchy/readChildren.ts`                  | `src/shared/library/hierarchy/read.ts`           |
| `src/shared/libraryNavigation/readRows.ts`                     | `src/shared/library/navigation/read.ts`          |
| `src/shared/libraryRoots/cancelScan.ts`                        | `src/shared/library/roots/cancel.ts`             |
| `src/shared/libraryRoots/chooseAndRegisterLocal.ts`            | `src/shared/library/roots/chooseLocal.ts`        |
| `src/shared/libraryRoots/readLocalRoots.ts`                    | `src/shared/library/roots/read.ts`               |
| `src/shared/libraryRoots/registerLocalRoot.ts`                 | `src/shared/library/roots/register.ts`           |
| `src/shared/libraryRoots/runScan.ts`                           | `src/shared/library/roots/scan.ts`               |
| `src/shared/libraryRoots/unregisterLocalRoot.ts`               | `src/shared/library/roots/unregister.ts`         |
| `src/shared/librarySourceFileHashing/hashSourceFilesBlake3.ts` | `src/shared/library/source/fileHashing.ts`       |
| `src/shared/librarySourceLifecycle/readSourceLifecycle.ts`     | `src/shared/library/source/lifecycle.ts`         |
| `src/shared/librarySourceMaintenance/sourceMaintenance.ts`     | `src/shared/library/source/maintenance.ts`       |
| `src/shared/libraryTrackIdentityReview/candidates.ts`          | `src/shared/library/trackIdentity/candidates.ts` |
| `src/shared/libraryTrackIdentityDecisions/decisionCommands.ts` | `src/shared/library/trackIdentity/decisions.ts`  |
| `src/shared/libraryViewState/viewState.ts`                     | `src/shared/library/viewState/persistence.ts`    |

Add:

| New path                                          | Purpose                                                  |
|---------------------------------------------------|----------------------------------------------------------|
| `src/shared/library/boundary/controlPlane.ts`     | Single owner for command/read IPC channel constants      |
| `src/shared/library/boundary/publicationPlane.ts` | Single owner for publication/event IPC channel constants |
| `src/shared/library/boundary/rendererApi.ts`      | Library-domain renderer API surface type                 |

### Renderer layer

Do not move renderer folders.

Update imports only.

Renderer paths that may need import updates include:

- `src/renderer/library/boundary/`
- `src/renderer/library/runtime/`
- `src/renderer/library/tree/`
- `src/renderer/library/contents/`
- `src/renderer/library/panel.vue`
- tests under `tests/unit/renderer/library/`

## Tests and support move rules

Integration tests may be grouped under `tests/integration/main/library/` if the current tree is being cleaned at the
same time, but this is optional for the first migration pass.

If test files move, keep test file names descriptive. Do not rename tests to generic `read.test.ts` unless the folder
context fully restores clarity.

Test support files should follow the same domain grouping where useful:

```text
tests/support/library/
├── boundary.ts
└── hierarchy.ts
```

Do not let test support preserve old import paths.

## Source guards

After the migration, search for previous folder names.

Required checks:

```text
libraryAttachmentIdentity
libraryBoundary
libraryContents
libraryHierarchy
libraryNavigation
libraryRoots
librarySourceFileHashing
librarySourceLifecycle
librarySourceMaintenance
libraryTrackIdentityDecisions
libraryTrackIdentityReview
libraryViewState
```

Expected result:

- zero hits in active source code
- zero hits in active import paths
- zero hits in active architecture docs, except this migration document's move map
- no old-path compatibility files

Search for domain-owned channel constants.

Expected result:

- channel string constants live only in `shared/library/boundary/controlPlane.ts` and
  `shared/library/boundary/publicationPlane.ts`
- shared domain folders do not define channel objects

Search for old channel object names.

Expected result:

- old names are removed or replaced by the new boundary-owned channel object
- no alias exports preserve old names

## Deletion acceptance criteria

The migration is not complete until all of these are true:

- no old `src/main/libraryX/` folders remain
- no old `src/shared/libraryX/` folders remain
- no old-path re-export files remain
- no alias modules exist to preserve old imports
- no compatibility comments point readers to old paths
- no domain-owned channel constants remain outside `shared/library/boundary/controlPlane.ts` and
  `shared/library/boundary/publicationPlane.ts`
- channel string literal values are unchanged
- renderer continues to consume the same app-level API shape
- preload continues to expose the same app-level API shape
- `commandRegistry.ts` exists as an inert ownership stub
- `shared/library/boundary/rendererApi.ts` owns the library-domain API type
- `shared/rendererApi.ts` composes the library-domain API type
- no runtime behavior changes are introduced

## Migration sequence

1. Create `main/library/` and `shared/library/` target folder groups.
2. Move shared domain shape files into `shared/library/`.
3. Move shared boundary files into `shared/library/boundary/`.
4. Create `shared/library/boundary/controlPlane.ts`.
5. Create `shared/library/boundary/publicationPlane.ts`.
6. Move channel constants into the appropriate boundary plane files with unchanged string values.
7. Create `shared/library/boundary/rendererApi.ts`.
8. Update `shared/rendererApi.ts` to compose the library-domain API surface.
9. Move main domain behavior files into `main/library/`.
10. Move main boundary files into `main/library/boundary/`.
11. Add `main/library/boundary/commandRegistry.ts` stub.
12. Update main/preload/renderer imports.
13. Update tests and support imports.
14. Delete empty previous folders.
15. Run source guards.
16. Run validation.

## Import update rules

Prefer direct imports from owned files over broad barrels.

Valid:

```ts
import type { ReadLocalRootsResult } from '../../shared/library/roots/read'
import { libraryControlChannels } from '../../shared/library/boundary/controlPlane'
```

Invalid:

```ts
import { ReadLocalRootsResult } from '../../shared/library'
import { oldRootChannels } from '../../shared/libraryRoots/channels'
```

When a file name is short, exported names must remain semantically qualified.

Valid:

```ts
export type ReadHierarchyChildrenRequest = { ... }
```

Invalid:

```ts
export type ReadRequest = { ... }
```

## Channel value preservation

Before moving channel constants, record existing literal values.

After moving channel constants, compare values manually or with a small script.

Changing the TypeScript object path is allowed.

Changing the runtime string value is not allowed.

Example allowed change:

```ts
// Before
rootChannels.registerLocalRoot === 'desktop:library:roots:register-local-root'

// After
libraryControlChannels.roots.register === 'desktop:library:roots:register-local-root'
```

Example forbidden change:

```ts
libraryControlChannels.roots.register === 'desktop:library:roots:register'
```

## Boundary-spine transition rule

This migration reserves boundary-spine ownership but does not implement the full spine.

Allowed:

- add `commandRegistry.ts` stub
- centralize channel constants
- update call sites to import channel constants from boundary plane files

Forbidden:

- move all `ipcMain.handle` calls into `commandRegistry.ts`
- add command wrapper behavior
- add validation wrapper behavior
- change product result versus host failure behavior
- change event pump scheduling behavior
- change preload cleanup behavior

The later boundary-spine implementation owns those behavior changes.

## Validation

Run the lightest validation that proves the structural migration did not break TypeScript or tests.

Minimum:

```text
pnpm --filter @dekzer/desktop run typecheck
pnpm --filter @dekzer/desktop run test:unit
pnpm --filter @dekzer/desktop run format:check
```

If integration test imports changed, also run the relevant integration tests:

```text
pnpm --filter @dekzer/desktop run test:integration
```

If repository-level validation is affordable, run:

```text
pnpm verify
```

Do not skip source guards even if tests pass.

## Review checklist

A reviewer should be able to verify this migration mechanically.

Check:

- File tree matches the architecture document.
- Old folder groups are gone.
- Imports point to canonical paths.
- No compatibility aliases exist.
- No wrapper modules preserve old paths.
- Channel constants are centralized by plane.
- Channel literal values are unchanged.
- Domain shared files own shapes only.
- Main files own behavior only.
- Renderer structure is not mirrored to main/shared.
- `viewState/persistence.ts` names persisted read/write behavior.
- `source/fileHashing.ts` is used instead of vague `source/hashing.ts`.
- `commandRegistry.ts` is present but inert.
- Validation passes.

## Suggested commit message

```text
refactor(desktop): group library boundary folders by ownership
```

Alternative:

```text
refactor(desktop): consolidate library folder ownership
```

## Final report contract for the agent

Report only:

- files moved
- files added
- files deleted
- import update summary
- channel constants moved and confirmation that string values are unchanged
- source guard results
- validation run
- current git status
- risks
- suggested commit message
