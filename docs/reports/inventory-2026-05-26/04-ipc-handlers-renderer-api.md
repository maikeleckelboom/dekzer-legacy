# Sub-Agent 4: IPC Handlers & Renderer API -- `desktop:library-selected-contents:read` Pipeline

**Date**: 2026-05-26
**Scope**: `apps/desktop/src/main/`, `apps/desktop/src/shared/`, `apps/desktop/src/preload/`,
`packages/library-boundary-client/`, `packages/library-boundary-contract/`

---

## 1. Channel Definition (shared layer)

**File:** `apps/desktop/src/shared/librarySelectedContents/read.ts`

This is the single source of truth for the IPC channel name and all shared types:

**Channel:**

```ts
export const selectedContentsReadChannels = {
  read: 'desktop:library-selected-contents:read'
} as const
```

**Request type** (`SelectedContentsRequest`):

```ts
{
  scope?: SelectedContentsScope   // source | sourceLocation | directory
  limit?: number
  cursor?: string
}
```

**Response type** (`SelectedContentsReadResult`): a discriminated union:

- `{ state: 'ready', result: SelectedContentsResult }` -- success
- `{ state: ErrorState, error: { code, message } }` -- error (
  `hostUnavailable | noTarget | notFound | invalidRequest | readFailed`)

The success payload (`SelectedContentsResult`) includes:

- `state`, `scope`, `rows[]`, `coverage`, optional `nextCursor`, optional `detail`

Each `SelectedContentsRow` includes `mediaClass` (`'audio' | 'video'`), `availabilityState`, `prepReadinessSummary`, and
other fields.

---

## 2. Preload / Context Bridge (exposing the API to the renderer)

**File:** `apps/desktop/src/preload/index.ts`

```ts
import { contextBridge as rendererContext, ipcRenderer } from 'electron'
import { exposeRendererApi } from './rendererApi'
exposeRendererApi(rendererContext, ipcRenderer)
```

**File:** `apps/desktop/src/preload/rendererApi.ts`

The `exposeRendererApi` function calls:

```ts
rendererContext.exposeInMainWorld('dekzer', createRendererApi(ipcRenderer))
```

The `selectedContents` mapping in `createRendererApi`:

```ts
selectedContents: {
  async read(request: SelectedContentsRequest): Promise<SelectedContentsReadResult> {
    return (await ipcRenderer.invoke(
      selectedContentsReadChannels.read,   // 'desktop:library-selected-contents:read'
      request
    )) as SelectedContentsReadResult
  }
}
```

Key observation: The **entire** `SelectedContentsRequest` object is passed through `ipcRenderer.invoke` without any
transformation. The typed cast `as SelectedContentsReadResult` is a development-time assertion.

**Renderer type declaration:**
**File:** `apps/desktop/src/renderer/env.d.ts`

```ts
declare global {
  interface Window {
    readonly dekzer: RendererApi
  }
}
```

---

## 3. Shared RendererApi Type

**File:** `apps/desktop/src/shared/rendererApi.ts`

The relevant portion:

```ts
export type LibrarySelectedContentsApi = {
  read(request: SelectedContentsRequest): Promise<SelectedContentsReadResult>
}
```

This is nested under `RendererApi.library.selectedContents`.

---

## 4. Main Process Registration

**File:** `apps/desktop/src/main/index.ts` (lines 8, 66)

```ts
import { registerSelectedContentsReadIpc } from './librarySelectedContents/read'
// ...
app.whenReady().then(() => {
  const host = createLibraryBoundaryHost({ app, isDev: is.dev })
  // ...
  registerSelectedContentsReadIpc(ipcMain, host)  // <-- line 66
})
```

---

## 5. Main Process IPC Handler (the core of your question)

**File:** `apps/desktop/src/main/librarySelectedContents/read.ts`

### 5a. Registration function

```ts
export function registerSelectedContentsReadIpc(
  ipcMain: LibrarySelectedContentsReadIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(selectedContentsReadChannels.read, (_event, request) =>
    readSelectedContentsThroughHost(host, request)
  )
}
```

The `_event` is completely ignored. The `request` is `unknown`-typed.

### 5b. Main handler flow (`readSelectedContentsThroughHost`)

1. **Normalize the request** via `normalizeRequest(request)` (returns either a `NormalizedRequest` or an error
   `SelectedContentsReadResult`)
2. **Check for early errors** -- if normalization produced an error result, return it immediately
3. **Get the host client** via `getStartedClient(host)` -- throws if host is `notStarted`/`stopping`/`stopped`/`failed`;
   those are caught and mapped to error results
4. **Call the backend** via `client.readSelectedContents(...)`
5. **Map the contract response** back via `mapSelectedContentsResult(contractResult)`
6. **Error catch** -- any unexpected exception becomes
   `{ state: 'readFailed', error: { code: 'readFailed', message: '...' } }`

### 5c. Request validation / normalization (detailed)

**`normalizeRequest(request)`** validates each field in sequence:

| Field         | Validation                                                                                                            | Default                                                         |
|---------------|-----------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------|
| **top-level** | Must be a record (`typeof === 'object' && !== null && !Array.isArray`)                                                | Error: `invalidRequest`                                         |
| **scope**     | Checked by `normalizeScope()` -- must be a record with `kind` string, and one of three patterns (see below)           | Error: `noTarget` if null/undefined, `invalidRequest` otherwise |
| **limit**     | Optional. If absent, defaults to `100`. If present, must be a positive integer `<= 200`.                              | `100`                                                           |
| **cursor**    | Optional. If `null`/`undefined`, omitted from the normalized request. If present, must be a non-empty trimmed string. | Omitted if absent                                               |

**`normalizeScope(value)`** -- **This is the answer to "whether `sourceLocation` is handled"**:

It recognizes exactly three scope kinds, each validated with the `positiveOpaqueIdPattern` regex (`/^[1-9]\d*$/`):

| Scope kind         | Required fields                 | Validation                                                         |
|--------------------|---------------------------------|--------------------------------------------------------------------|
| `'source'`         | `sourceId`                      | Must be a positive opaque ID string (starts with 1-9, then digits) |
| `'sourceLocation'` | `sourceLocationId`              | Must be a positive opaque ID string                                |
| `'directory'`      | `sourceId`, `sourceDirectoryId` | Both must be positive opaque ID strings                            |

**YES, `sourceLocation` IS handled.** The validation explicitly checks `value.kind === 'sourceLocation'` and
`isPositiveOpaqueId(value.sourceLocationId)` (line 151).

### 5d. Mapping TS request to backend contract

The `mapScopeToContract()` function translates from the desktop's shared type to the contract types from
`@dekzer/library-boundary-contract`:

| Desktop shape                                        | Contract shape                                                    |
|------------------------------------------------------|-------------------------------------------------------------------|
| `{ kind: 'source', sourceId }`                       | `{ type: 'source', payload: { sourceId } }`                       |
| `{ kind: 'sourceLocation', sourceLocationId }`       | `{ type: 'sourceLocation', payload: { sourceLocationId } }`       |
| `{ kind: 'directory', sourceId, sourceDirectoryId }` | `{ type: 'directory', payload: { sourceId, sourceDirectoryId } }` |

The full request sent to the host client:

```ts
const reply = await client.readSelectedContents({
  scope: mapScopeToContract(normalizedRequest.scope),
  limit: normalizedRequest.limit,
  ...(normalizedRequest.cursor === undefined ? {} : { cursor: normalizedRequest.cursor })
} satisfies ReadSelectedContentsRequest)
```

**YES, `cursor` IS passed through** -- it is conditionally included in the contract request if the normalized request
has one.

### 5e. Response shaping

The contract reply (`ReadSelectedContentsReply`) contains a `result: SelectedContentsResult`. This is mapped by
`mapSelectedContentsResult()` and `mapSelectedContentsRow()`:

Each contract row field is mapped:

- Non-null fields on the contract like `row.libraryAssetId`, `row.rowVersion`, etc. are converted from `string | null`
  to `string | undefined` (the `null` values are omitted via spread conditionals)
- Required fields are always included: `stableId`, `label`, `origin`, `scopedSourceFileId`, `sourceId`, `relativePath`,
  `fileName`, `mediaClass`, `availabilityState`, `prepReadinessSummary`, `updatedAtMs`
- `coverage` is mapped by `mapCoverage()`: `state`, `recursiveScopeComplete`, `emptyResultAuthoritative`, optional
  `detail`
- `nextCursor` and `detail` from result are conditionally included

The final success response shape:

```ts
{
  state: 'ready',
  result: {
    state: SelectedContentsState,
    scope: SelectedContentsScope,       // mapped back from contract
    rows: readonly SelectedContentsRow[],
    coverage: ContentsCoverage,
    nextCursor?: string,                // passed through if present
    detail?: string                     // passed through if present
  }
}
```

### 5f. MediaClass filtering

**NO, there is no `mediaClass` filtering at this layer.** The `mediaClass` field from each contract row is passed
through as-is in `mapSelectedContentsRow()`. Additionally, the `SelectedContentsRequest` type does not include a
`mediaClass` filter field -- neither the desktop shared type nor the contract type has one. Filtering by media class
would need to happen either in the renderer boundary or in the backend service.

---

## 6. Renderer Boundary (caller side)

**File:** `apps/desktop/src/renderer/library/boundary/selectedContentsRead.ts`

The Vue composable `useSelectedContentsRead`:

- Builds the `scope` from a `RowBinding` (derived from the selected hierarchy node)
- Calls `selectedContentsApi.read({ scope, limit: 100 })` -- notably, it does NOT send `cursor` or any `mediaClass`
  filter
- Manages loading/ready/idle/failed state transitions

---

## 7. Backend Client

**File:** `packages/library-boundary-client/src/client.ts` (lines 192-206)

```ts
readSelectedContents(
  request: ReadSelectedContentsRequest
): Promise<ReadSelectedContentsReply> {
  return this.sendAndExpect(
    { type: "snapshotRead", payload: { type: "readSelectedContents", payload: request } },
    "snapshotRead",
    "selectedContents"
  );
}
```

Sends the request over the stdio transport to the Rust boundary service.

---

## 8. LibraryBoundaryHost

**File:** `apps/desktop/src/main/libraryBoundary/host.ts`

The `host.client` getter (line 97-103) throws if the host is not `'started'` or if `#client` is undefined. The IPC
handler catches this via `getStartedClient()` and maps it to appropriate error results based on the host's `state` and
the specific error `code`.

---

## Summary Table

| Question                                            | Answer                                                                                                                                                                                                              |
|-----------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| How does the IPC handler map TS request to backend? | Normalizes request, translates `{kind, ...}` to `{type, payload: {...}}` for the `scope`, passes `limit` and `cursor` to `host.client.readSelectedContents()`                                                       |
| How is the request validated?                       | Three-stage validation: (1) request must be a record, (2) scope must have valid kind with positive opaque IDs, (3) limit must be 1-200 integer, (4) cursor must be non-empty string if present                      |
| How is the response shaped?                         | Contract `ReadSelectedContentsReply.result` is mapped row-by-row, converting `null` to `undefined` for optional fields, wrapped in `{state: 'ready', result}`                                                       |
| Is `sourceLocation` handled in the main IPC layer?  | **YES** -- `normalizeScope()` explicitly checks `kind === 'sourceLocation'` and validates `sourceLocationId` (line 151), and `mapScopeToContract()`/`mapScopeFromContract()` both include a `sourceLocation` branch |
| Is `cursor` passed through?                         | **YES** -- normalized and forwarded to the backend contract request, and the backend's `nextCursor` is passed back in the response                                                                                  |
| Does `mediaClass` filtering happen at this layer?   | **NO** -- `mediaClass` is only a field on the response `SelectedContentsRow`, passed through as-is. No request-side filter for media class exists in either the desktop shared types or the contract types.         |
