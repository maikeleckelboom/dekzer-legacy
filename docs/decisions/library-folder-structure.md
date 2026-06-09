# Library Folder Structure

## Status

Status: ratified architecture
Owner: Electron library boundary and renderer architecture
Applies to:

- apps/desktop/src/main/library/
- apps/desktop/src/shared/library/
- apps/desktop/src/renderer/library/
- library-facing preload and renderer API type composition

This document defines the canonical folder structure and naming law for the Electron-side library domain. It is not a
migration checklist. The companion migration document owns the mechanical move plan and verification gates.

## Architecture law

Layer roots group by product domain. Inside a domain, folders name owned concepts or renderer concerns. File names name
local operations. Transport ownership lives in the boundary spine. Domain folders own behavior or shapes, never channel
strings. Renderer structure is concern-owned, not a mirror of main/shared.

## Core separation

The Electron library code is split by layer ownership.

### Main layer

`src/main/library/` owns host-side behavior:

- IPC command handling and forwarding
- library boundary host lifecycle
- domain operation behavior
- mapping between generated service/protocol replies and renderer-facing TypeScript shapes
- publication forwarding from the boundary service to the renderer
- persistence behavior that requires Node.js APIs

### Shared layer

`src/shared/library/` owns renderer-facing TypeScript contracts:

- request, reply, and result shapes crossing `contextBridge`
- the per-domain library API type
- control-plane channel constants
- publication-plane channel constants
- event parsing and event types used by preload/renderer

Shared files do not own Node.js behavior, host behavior, renderer state, or service/protocol authority.

### Renderer layer

`src/renderer/library/` owns renderer-side concerns:

- IPC client calls
- retained boundary reads
- runtime state
- projection
- tree interaction
- contents presentation
- panel UI composition

The renderer is not a structural mirror of main/shared. It is organized by renderer concerns.

### Service/protocol authority

Generated service/protocol contracts remain the service boundary authority. Shared domain files define the
renderer-facing Electron surface. Mapping between those surfaces belongs at the main boundary edge.

## Target structure

### `src/main/library/`

```text
main/library/
├── attachmentIdentity/
│   └── read.ts
├── boundary/
│   ├── commandRegistry.ts
│   ├── config.ts
│   ├── errors.ts
│   ├── eventPump.ts
│   ├── host.ts
│   └── status.ts
├── contents/
│   └── read.ts
├── hierarchy/
│   ├── mapping.ts
│   ├── read.ts
│   ├── request.ts
│   └── target.ts
├── navigation/
│   └── read.ts
├── roots/
│   ├── cancel.ts
│   ├── chooseLocal.ts
│   ├── read.ts
│   ├── register.ts
│   ├── scan.ts
│   └── unregister.ts
├── source/
│   ├── fileHashing.ts
│   ├── lifecycle.ts
│   └── maintenance.ts
├── trackIdentity/
│   ├── candidates.ts
│   └── decisions.ts
└── viewState/
    └── persistence.ts
```

### `src/shared/library/`

```text
shared/library/
├── attachmentIdentity/
│   └── read.ts
├── boundary/
│   ├── controlPlane.ts
│   ├── publicationPlane.ts
│   ├── eventParser.ts
│   ├── events.ts
│   ├── rendererApi.ts
│   └── status.ts
├── contents/
│   └── read.ts
├── hierarchy/
│   └── read.ts
├── navigation/
│   └── read.ts
├── roots/
│   ├── cancel.ts
│   ├── chooseLocal.ts
│   ├── read.ts
│   ├── register.ts
│   ├── scan.ts
│   └── unregister.ts
├── source/
│   ├── fileHashing.ts
│   ├── lifecycle.ts
│   └── maintenance.ts
├── trackIdentity/
│   ├── candidates.ts
│   └── decisions.ts
└── viewState/
    └── persistence.ts
```

### `src/renderer/library/`

Renderer structure remains concern-owned.

```text
renderer/library/
├── boundary/
├── contents/
├── runtime/
├── tree/
├── panel.vue
└── state.ts
```

Renderer import paths update when shared/main paths move. Renderer folders are not reorganized to mirror main/shared
subdomains.

## Boundary spine ownership

### Control plane

`shared/library/boundary/controlPlane.ts` is the single owner for library command/read IPC channel constants.

It uses grouped names and literal typing:

```ts
export const libraryControlChannels = {
  attachmentIdentity: {
    read: '<existing channel string>'
  },
  contents: {
    read: '<existing channel string>'
  },
  hierarchy: {
    read: '<existing channel string>'
  },
  navigation: {
    read: '<existing channel string>'
  },
  roots: {
    cancel: '<existing channel string>',
    chooseLocal: '<existing channel string>',
    read: '<existing channel string>',
    register: '<existing channel string>',
    scan: '<existing channel string>',
    unregister: '<existing channel string>'
  },
  source: {
    fileHashing: '<existing channel string>',
    lifecycle: '<existing channel string>',
    maintenance: '<existing channel string>'
  },
  trackIdentity: {
    candidates: '<existing channel string>',
    decisions: '<existing channel string>'
  },
  viewState: {
    read: '<existing channel string>',
    write: '<existing channel string>'
  }
} as const
```

The placeholder strings above describe the shape only. The actual implementation must preserve existing channel string
values exactly.

### Publication plane

`shared/library/boundary/publicationPlane.ts` is the single owner for library event/publication channel constants.

Publication channels are distinct from control-plane channels because they carry one-way publication/event traffic from
main/preload to renderer rather than request/reply commands.

### Command registry

`main/library/boundary/commandRegistry.ts` reserves the final command registration owner.

The initial structural migration may leave it as a compile-safe ownership stub:

```ts
// Reserved: library IPC command handler registration lives here.
// Domain handlers register through this spine, not via ipcMain.handle directly.
export {}
```

The stub is intentionally inert. It reserves ownership. It does not implement the command registry behavior. The
boundary-spine implementation owns that later behavior change.

### Event pump

`main/library/boundary/eventPump.ts` owns boundary event pumping and main-to-renderer publication forwarding. Renderer
code must not schedule boundary-service `ReadAfter` or `WaitForEventsAfter` activity.

## Renderer API ownership

`shared/library/boundary/rendererApi.ts` owns the library-domain API surface type.

`shared/rendererApi.ts` stays at shared root and composes the whole app API surface.

```ts
import type { LibraryApi } from './library/boundary/rendererApi'

export interface RendererApi {
  readonly library: LibraryApi
}
```

Future product domains may follow this pattern only after they earn independent ownership. Do not add a new domain
merely because a feature name exists.

Preload wires the composed surface to `contextBridge`. Shared owns the type contract.

## Conventions

### C1 — Domain grouping is mandatory

Every feature domain gets a named folder at the root of each Electron layer:

- `main/{domain}/`
- `shared/{domain}/`
- `renderer/{domain}/`

The current domain is `library`.

Domain subfolders are not dumped at layer root. A domain folder exists first; owned concepts sit inside it.

### C2 — No domain prefix inside a domain folder

Inside `main/library/`, folders are `roots/`, `hierarchy/`, `boundary/`, and similar concept names.

Do not prefix subfolders with the domain name. The parent folder already supplies the domain context.

### C3 — Files name the operation, not the context

Inside `roots/`, files are named by local operation:

- `read.ts`
- `register.ts`
- `scan.ts`
- `cancel.ts`

The folder carries concept context. The file carries operation context.

Exported identifiers must still carry full semantic context:

```ts
// shared/library/roots/read.ts
export type ReadLocalRootsResult = { ... }
```

Do not export vague names such as `ReadResult` from short operation files when those names cross file boundaries.

### C4 — Layer ownership, not feature ownership

Matching folder names across layers do not mean matching ownership.

`main/library/roots/` owns behavior.

`shared/library/roots/` owns renderer-facing request/result shapes.

They are not twins. Shared files do not contain Node.js APIs. Main files may import channel constants but do not define
them.

### C5 — Renderer organizes by concern, not by domain mirror

Renderer library folders represent renderer concerns:

- `boundary/`
- `runtime/`
- `tree/`
- `contents/`

Do not reorganize renderer into a mirror of main/shared subdomains.

### C6 — `shared/rendererApi.ts` stays at shared root

The root renderer API file composes app-level domains. It imports per-domain API surfaces from their boundary folders.

Preload wires this surface. Shared owns the type contract.

### C7 — Consolidate before proliferating

Before adding a top-level folder inside `library/`, decide whether the concept belongs inside an existing owner.

Subdomain count must be earned. A folder exists because it owns a stable concept boundary, not because a feature needs
somewhere to live.

### C8 — Channel strings have one owner

All library command/read IPC channel constants live in `shared/library/boundary/controlPlane.ts`.

All library publication/event IPC channel constants live in `shared/library/boundary/publicationPlane.ts`.

Domain folders define request/reply/result shapes only. Domain folders do not define, export, or re-export channel
strings.

### C9 — Folder migration must not change runtime behavior

Structural migrations may move files, update import paths, relocate unchanged channel constants, create boundary
ownership files, and add inert ownership stubs.

Structural migrations must not change runtime behavior, request/result shapes, channel string values, scan behavior,
hierarchy behavior, contents behavior, source lifecycle behavior, renderer interaction behavior, or IPC semantics.

### C10 — Barrels may compose, not hide ownership

Index files are allowed only at defined composition points.

Valid composition examples:

- `shared/rendererApi.ts`
- a carefully scoped per-domain API composition file

Invalid composition examples:

- a broad barrel that re-exports every shape from every library subfolder
- a barrel that hides where channel strings or behavior are owned
- a barrel that preserves an old import path after a move

### C11 — `source/` is post-admission only

`source/` owns post-admission operations on an admitted source:

- source lifecycle reads
- source maintenance commands
- source-file hashing

`source/` does not own root admission, navigation, hierarchy reads, contents reads, track identity, or attachment
identity.

Rejected examples:

- `source/read.ts` is too vague.
- `source/files.ts` is too broad.
- `source/scan.ts` is wrong when the operation is root scan admission/control owned by `roots/`.

### C12 — Shared domain shapes are renderer-facing contracts, not protocol authority

Shared domain files define what the renderer sends and receives over `contextBridge`.

They do not define what the Rust service produces. Generated service/protocol contracts remain the service authority.

Mapping between generated protocol output and renderer-facing TypeScript shapes belongs at the main boundary edge.

If the renderer-facing shape and generated protocol shape drift, the repair belongs in boundary mapping and/or contract
generation, not in duplicate schema authority.

### C13 — Persisted read/write storage behavior is named `persistence.ts`

Files that own persisted read/write storage behavior are named `persistence.ts`.

For library view state:

- `viewState/` names the product concept.
- `persistence.ts` names the behavior: persisted read/write storage.

Do not use generic names such as `state.ts` or repeated names such as `viewState.ts` for persisted storage behavior.

### C14 — No compatibility paths, aliases, or old-structure hints

This architecture is replacement, not compatibility.

After migration, the new structure is the only structure. Do not leave old-path files that re-export from new paths. Do
not add compatibility aliases. Do not keep wrapper modules for previous folder names. Do not leave comments that teach
future readers the previous structure as an available option.

Pre-migration names may appear only in the migration document while the migration is being planned or reviewed. They
must not appear in active source code, active import paths, public barrels, or current architecture comments after
migration lands.

There is one way to import each concept. There is one owner for each channel string. There is one canonical folder for
each owned concept.

## Key decisions

### Domain folders remove repeated prefixes

A layer-level domain folder provides context. Subfolders do not repeat that context.

This makes paths shorter, clearer, and scalable across future domains that earn independent ownership.

### Channel strings live in the boundary spine

Channel strings are transport contract. They are not domain behavior and not renderer state.

Central ownership keeps the IPC surface auditable and prevents every domain from growing its own mini-boundary.

### Control plane and publication plane are separate

Control-plane channels carry request/reply command traffic.

Publication-plane channels carry event/publication traffic.

Separate files keep the two IPC patterns visible and auditable.

### `source/` consolidates post-admission operations

Source lifecycle, source maintenance, and source-file hashing all operate on admitted sources. They share a concept
boundary and should not remain separate top-level domains.

### `trackIdentity/` consolidates one workflow

Candidate review and decisions are two sides of the same workflow. They belong under one concept folder.

### `viewState/persistence.ts` names behavior honestly

The file owns durable view-state read/write behavior. The name must say that. Generic `state.ts` is not acceptable for
persisted storage behavior.

### `commandRegistry.ts` is reserved immediately

The stub reserves the future IPC command registration owner before the boundary-spine implementation lands.

It prevents future readers and agents from treating scattered `ipcMain.handle` registration as the desired architecture.

### Renderer structure remains concern-owned

Renderer code has different ownership pressures than main/shared. It organizes by boundary client, runtime state,
projection, and UI concerns.

A symmetric folder tree would make the renderer worse.
