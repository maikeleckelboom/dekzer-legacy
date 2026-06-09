# Electron Boundary Spine

**Status:** accepted
**Doctrine-version:** 0.1
**Last-reviewed:** 2026-06-09
**Owner:** desktop-boundary-substrate
**Scope:** renderer ↔ main ↔ Rust library service IPC discipline

---

## Architectural Law

**Rust publishes. Main pumps. Renderer subscribes.**

The Rust library service is the sole author of boundary events. The main process owns
boundary event reception: it reads events from the Rust boundary stream and forwards
publication events to the renderer. The renderer subscribes to main-forwarded publication
only.

The renderer does not invoke `ReadAfter` or `WaitForEventsAfter` directly. Event
reception is subscription, not renderer-initiated polling. This law governs all event
delivery design in this document.

---

## Implementation Sequence

Do not mix this work with browse-policy omission metadata.
Do not mix this work with source-root admission or default discovery.

Recommended sequence:

1. Finish or merge source-add activation if already green.
2. Finish browse-policy omission metadata inventory and implement that slice only if the
   inventory proves the boolean can be computed honestly.
3. Verify this accepted decision remains the active boundary owner before any spine implementation begins.
4. Consolidate the Electron library command/event spine.
5. Verify that renderer-side event polling is structurally removed or guarded.
6. Implement source-root admission and default music source discovery on top of the spine.
7. Introduce ResourcePlane helpers only when a real resource payload requires them.

The spine must happen before source-root admission and default discovery because source
admission adds new command and event surface. Adding that surface before the spine
increases IPC entropy.

The spine must not block small already-green product repairs, but it must block larger new
boundary surfaces.

---

## First Spine Implementation Slice

The first implementation slice is structural, not semantic.

**In scope:**

- central channel owner for library IPC
- plane-separated channel constants
- one explicit library command registry with service readiness gate
- typed command handler wrapper
- Boundary Event Pump as a named main-process lifecycle owner
- product result versus host failure separation
- service-not-ready as host failure
- renderer boundary client with subscription and cleanup lifecycle
- preload listener cleanup API
- event emit helpers for main-to-renderer forwarding
- generated contract type imports
- validation hooks
- tests and source guards

**Out of scope:**

- source-root admission
- default music source discovery
- browse-policy omission metadata
- playable-media policy changes
- source scan behavior changes
- tree behavior changes
- contents row fields
- waveform or resource payload implementation
- ResourcePlane implementation
- Exclave integration
- new schema generation system
- broad preload rewrite
- app shell redesign

The first slice may move existing handlers into the spine, but it must not change product
behavior unless a behavior is already a boundary bug.

---

## File Ownership Target

Use existing files if they already own these roles. Do not create parallel paths.

The spine has six owners.

### 1. Channel Owner

**File:** `apps/desktop/src/main/library/channels.ts`

**Owns:**

- `ControlPlane` constants
- `PublicationPlane` constants
- future `ResourcePlane` constants
- channel naming convention
- no duplicate channel strings

**Does not own:** handler logic, emit logic, pump lifecycle, product behavior.

```typescript
export const ControlPlane = {
  ContentsRead: 'library.contents.read',
  HierarchyRead: 'library.hierarchy.read',
  SourceAdd: 'library.source.add',
  SourceRemove: 'library.source.remove',
  ScanStart: 'library.scan.start',
  ScanCancel: 'library.scan.cancel',
  SourceLifecycleRead: 'library.source.lifecycle.read'
} as const

export const PublicationPlane = {
  BrowserInvalidated: 'library.invalidation.browser',
  SourceLifecycleChanged: 'library.source.lifecycle.changed',
  ScanEvent: 'library.scan.event',
  BoundaryGapDetected: 'library.boundary.gap.detected'
} as const

export const ResourcePlane = {
  // Reserved. Future: WaveformReadRange, ArtworkOpen, AnalysisBlobReadRange.
} as const

// Type-level plane membership — used to enforce primitive selection in
// the registry and emit helpers.
export type ControlChannel = (typeof ControlPlane)[keyof typeof ControlPlane]
export type PublicationChannel = (typeof PublicationPlane)[keyof typeof PublicationPlane]
export type ResourceChannel = (typeof ResourcePlane)[keyof typeof ResourcePlane]
```

No constant appears in more than one plane. If a channel moves between planes, the old
constant is removed, not aliased.

### 2. Command Registry

**File:** `apps/desktop/src/main/library/commands.ts`

**Owns:**

- registering all library control-plane commands
- explicit command allowlist
- dependency injection through function signature — not ambient globals
- `decodeCommandRequest` hook at main ingress
- service readiness gate
- service call handoff
- product result normalisation
- unexpected exception normalisation to `HostCommandFailure`
- host failure logging
- `validateCommandResponse` hook at main egress

**Does not own:** source admission policy, scan lifecycle, boundary event polling, cursor
tracking for publication events, selection behavior, contents filtering, renderer
projection.

**Operational law:** command handlers translate IPC requests to service method calls and
normalise results. They must not contain conditional branching on product domain values.
If product branching is needed, it lives in the service method, not the handler.

Dependency injection shape:

```typescript
export function registerLibraryCommands(deps: {
  ipcMain: Electron.IpcMain
  libraryService: LibraryService // typed service, not ambient global or singleton
  getWindows: () => BrowserWindow[]
}): void {
  /* ... */
}
```

#### Exposure Classes and Local-Root Trust

Library operations have two exposure classes:

- **renderer-callable** operations are available through preload and IPC. Their inputs must be renderer-safe.
- **host-internal** operations are available only inside Desktop Main or the library service path. They may consume
  host-owned values after the host has selected or validated them.

The renderer may request `chooseAndRegisterLocal`, but Desktop Main owns the native folder picker and the resulting
absolute path. `registerLocalRoot` is host-internal and must not be exposed through renderer IPC, preload, or renderer
feature APIs.

| Operation                | Exposure          | Boundary rule                                                               |
| ------------------------ | ----------------- | --------------------------------------------------------------------------- |
| `chooseAndRegisterLocal` | renderer-callable | Takes no absolute-path input; Desktop Main owns native selection.           |
| `registerLocalRoot`      | host-internal     | May accept the host-selected absolute path inside the trusted host path.    |
| `runScan`                | renderer-callable | Accepts source/root identity, not an absolute path.                         |
| hierarchy/contents reads | renderer-callable | Read-only typed requests through the command registry.                      |
| host status read/listen  | renderer-callable | Reports host availability without exposing host-internal registration APIs. |

No alternate renderer path may expose a host-internal operation. If exposure metadata becomes generated, extend the
existing boundary protocol and export pipeline rather than creating a second operation registry.

### 3. Boundary Event Pump

**File:** `apps/desktop/src/main/library/pump.ts`
**Concrete implementation name:** `MainLibraryEventPump`, unless the existing codebase
already has a clearer equivalent owner.

The Boundary Event Pump is a main-process lifecycle owner. It is not a command handler,
not a renderer feature client, and not a generic event helper. It owns the transition from
the Rust boundary event stream to renderer subscription delivery.

**Owns:**

- main-process bootstrap of boundary event reception
- the `ReadAfter` or `WaitForEventsAfter` call loop against the Rust boundary
- event cursor tracking
- gap detection and gap signal forwarding to the renderer
- classification of boundary event read outcomes: ready, gap, failed
- backoff after read failure
- forwarding parsed publication events to main-to-renderer emit helpers
- teardown when the owning window or app lifecycle ends

**Does not own:** event authorship, scan lifecycle policy, renderer refresh policy,
renderer selection behavior, product state authority, command handler registration.

The pump may call control-plane commands against the Rust boundary because event reads are
command-shaped at the transport layer. That does not make the renderer a command consumer
for events.

The pump is the only owner allowed to schedule continued `ReadAfter` or
`WaitForEventsAfter` calls. Renderer code must never schedule that loop.

Interface shape:

```typescript
interface MainLibraryEventPump {
  start(deps: PumpDeps): void
  stop(): void
}

interface PumpDeps {
  boundaryClient: RustBoundaryClient
  emitHelpers: LibraryEmitHelpers
  onGap: (gap: BoundaryGapEvent) => void
}
// Must not: author events, own scan policy, touch renderer state,
// register command handlers, or make product decisions.
```

### 4. Event Emit Helpers

**File:** `apps/desktop/src/main/library/events.ts`

**Owns:**

- main-to-renderer publication forwarding only
- `webContents.send` calls for library publication channels
- `PublicationPlane` channel constants
- event payload typing
- optional main-to-renderer payload validation
- delivery target policy

**Does not own:** Rust-side event authorship, boundary event stream publication, event
meaning, event polling, cursor tracking, refresh scheduling, renderer invalidation policy,
scan state authority.

Rust remains the source of boundary events. Emit helpers forward already-parsed events
from main to renderer subscribers. They do not decide which events exist.

No raw `webContents.send` call for library events appears anywhere outside this file.

### 5. Preload Boundary

**File:** `apps/desktop/src/preload/libraryApi.ts`

**Owns:**

- safe command invocation wrappers
- safe listener registration with cleanup-returning functions
- no raw `ipcRenderer` exposure to renderer feature code

**Does not own:** event polling, renderer feature policy, library state, selection,
contents projection.

Every exposed listener must return a cleanup callback. Callers must invoke it on unmount
or when the subscription is no longer needed.

### 6. Renderer Boundary Client

**File:** `apps/desktop/src/renderer/library/boundary/client.ts`

**Owns:**

- feature-facing typed command API
- transport failure normalisation
- product result forwarding
- subscription registration for pump-forwarded publication events
- cleanup lifecycle for renderer subscriptions
- `decodeCommandResponse` and `decodePublicationEvent` hooks at renderer ingress

**Does not own:** `ReadAfter` or `WaitForEventsAfter` scheduling, source facts, contents
filtering, scan meaning, authority decisions.

The renderer boundary client actively subscribes through preload APIs and owns calling
the returned cleanup function when the consuming surface unmounts.

---

## Result Model

The spine must distinguish three operational outcomes. They must never be conflated.
Feature code sees only `BoundaryResult<T>` and must not write `try`/`catch` for normal
library failures.

```typescript
// ── Layer 1: Product result ──────────────────────────────────────────────────
// The service ran and returned a domain outcome.
// Examples: contentsReadOk, sourceAlreadyRegistered, invalidCursor,
//           scanAlreadyRunning, policyConflict.
type LibraryResult<T> = { kind: 'ok'; value: T } | { kind: 'err'; error: LibraryError }

// ── Layer 2: Host command failure ────────────────────────────────────────────
// The handler ran, but the host infrastructure failed during handling.
// Examples: handler threw unexpectedly, serialisation failed inside main,
//           response failed validation, service not yet ready.
type HostCommandFailure = {
  kind: 'hostFailure'
  reason: 'handlerCrashed' | 'serializationFailed' | 'validationFailed' | 'serviceNotReady'
  detail?: string
}

// ── Layer 3: Transport / preload failure ─────────────────────────────────────
// The invoke could not reach the handler at all.
// Examples: channel missing, preload API absent, renderer destroyed,
//           Electron invoke rejected before any command outcome was returned.
// The command wrapper may never run in this case. The renderer boundary
// client is responsible for catching invoke rejection and producing this shape.
type TransportFailure = {
  kind: 'transportFailure'
  reason: 'channelMissing' | 'preloadMissing' | 'rendererDestroyed' | 'invokeRejected'
}

// ── Feature-facing outer type ─────────────────────────────────────────────────
// This is what renderer feature code receives. Nothing rawer crosses this line.
type BoundaryResult<T> = LibraryResult<T> | HostCommandFailure | TransportFailure
```

---

## Service-Not-Ready Law

If a command arrives before the Rust service or boundary dependency is initialised, the
command registry returns a host failure.

Service-not-ready is not a product failure. It must not be encoded as `LibraryError`. It
must not be silently queued inside an individual command handler.

The registry uses a shared service readiness gate. A missing or uninitialised service
produces `{ kind: 'hostFailure', reason: 'serviceNotReady' }`.

Feature code may render loading, retry, or unavailable state based on host failure
signals, but product policy does not live in the IPC handler.

Startup bootstrap code may wait for readiness before exposing a feature surface. Once a
command reaches the registry without a ready service, the outcome is host failure.

---

## Runtime Validation Policy

The generated boundary contract package is the source of compile-time types. The spine
imports from `packages/library-boundary-contract`. It does not locally redeclare types.

If generated runtime validators already exist in the contract package, the spine uses
them. If they do not, the first slice still introduces named validation hooks and
documents the gap explicitly.

First-slice validation is structurally present but behaviourally inert when validators
are not yet generated. The acceptance bar requires that hooks are wired and called.
Validation behaviour tests are explicitly gated on contract package runtime-validator
generation and are out of scope for this slice.

Do not invent a parallel local schema system in the first spine slice.

**The four named validation hooks:**

| Hook                      | Direction       | Location                                   | Role                                               |
| ------------------------- | --------------- | ------------------------------------------ | -------------------------------------------------- |
| `decodeCommandRequest`    | renderer → main | command registry, main ingress             | `unknown → Req` before service call                |
| `validateCommandResponse` | main → renderer | command registry, main egress              | `LibraryResult<T>` before IPC send; often identity |
| `decodeCommandResponse`   | main → renderer | renderer boundary client, renderer ingress | `unknown → BoundaryResult<T>`                      |
| `decodePublicationEvent`  | main → renderer | renderer boundary client, event ingress    | `unknown → EventPayload`                           |

`satisfies` at emit/send call sites provides a static shape check at the emission point.
It does not validate runtime JSON arriving from Rust or stdio. Both are needed. Neither
replaces the other.

Validation failure at any hook becomes host failure, not product failure.

---

## Plane Enforcement

Plane membership decides the Electron primitive. This is not a naming convention — it is a
different wire mechanism.

**ControlPlane:**

- uses `ipcMain.handle` on main side
- uses `ipcRenderer.invoke` on renderer side
- one request, one typed result, promise-based
- must not use `send`/`on` as the primary command path
- must not carry large binary payloads

**PublicationPlane:**

- uses `webContents.send` on main side
- uses `ipcRenderer.on` on renderer side, always with a cleanup-returning wrapper
- carries summaries, invalidations, lifecycle notifications, and handles only
- must not use `invoke`/`handle` as the primary event path
- must not carry large binary payloads

**ResourcePlane:**

- reserved for handle-oriented payload access
- future owner for waveform data, artwork, and analysis blob access
- transport chosen per payload class: MessagePort, transferable ArrayBuffer,
  SharedArrayBuffer for hot surfaces, native handle
- must not be prematurely modelled as a generic event stream
- waveform data, artwork payloads, and analysis blobs must not be routed through ControlPlane or PublicationPlane
- large binary payloads remain prohibited there pending ResourcePlane definition

A channel constant must not belong to more than one plane.

---

## Source Guards

The implementation must include source guards or equivalent tests for all of these:

- no raw library `ipcMain.handle` outside the command registry
- no raw library `webContents.send` outside publication emit helpers
- no raw `ipcRenderer.on` exposed to renderer feature code
- no renderer-side `ReadAfter` or `WaitForEventsAfter` scheduling
- no renderer feature code directly invoking `ReadAfter` or `WaitForEventsAfter`
- no library channel string literals outside the channel owner and tests
- no `PublicationPlane` channel used with `invoke` or `handle`
- no `ControlPlane` channel used with `send` or `on`
- generated contract types are imported, not locally redeclared
- command registry contains no product policy
- event emit helpers contain no renderer refresh policy
- event emit helpers do not author Rust-side event meaning
- `MainLibraryEventPump` is the only owner of publication cursor tracking

Source guards may be pragmatic search-based tests where TypeScript cannot enforce the
rule cleanly.

---

## Acceptance Bar

The first spine slice is accepted only if:

- all existing library commands remain callable
- all existing library publication events still reach the renderer
- product errors are returned as `LibraryResult` failures, never thrown
- unexpected handler exceptions become `HostCommandFailure`
- service-not-ready becomes `HostCommandFailure` with reason `serviceNotReady`
- renderer invoke rejection becomes `TransportFailure`
- listener cleanup removes exactly the registered listener
- raw library IPC strings are centralised in `channels.ts`
- command and publication channels are separated by plane
- `MainLibraryEventPump` owns event cursor tracking and read scheduling
- renderer code does not schedule `ReadAfter` or `WaitForEventsAfter` calls directly
- no product behavior moved into the command registry
- no generated boundary types are locally reduplicated
- all four named validation hooks exist and are called, even if validators are inert stubs
- validation behaviour tests are explicitly deferred until generated validators exist
- `ResourcePlane` is reserved and large payload routing through ControlPlane or
  PublicationPlane is rejected
- targeted tests pass
- full verification passes

**Manual smoke must confirm:**

- add source still works
- scan start still works
- scan events still update visible state through main-forwarded publication
- tree expansion still works
- contents reads still work
- remove source still works
- no renderer listener leak visible through repeated mount/unmount or app navigation
- no product error appears as an unhandled exception in renderer logs

---

## Rejection Cases

Reject the implementation if any of these are true:

- command registry becomes a product service
- source admission is implemented inside the spine slice
- scan lifecycle policy moves into command registration
- renderer selection behavior changes accidentally
- renderer schedules boundary event polling
- renderer directly invokes `ReadAfter` or `WaitForEventsAfter` for event reception
- publication events are routed through `invoke`
- commands are routed through event listeners
- raw library IPC strings remain scattered
- feature code catches raw Electron invoke errors directly
- product errors are thrown instead of returned
- service-not-ready is encoded as `LibraryError`
- host failures are encoded as `LibraryError`
- listener APIs do not return cleanup
- runtime validation is spread into feature components
- local request/response types duplicate generated contract types
- `ResourcePlane` is implemented prematurely without a real resource payload
- waveform, artwork, or analysis blobs are routed through ControlPlane or PublicationPlane

---

## Naming Guidance

Use product and plane names that describe ownership.

**Good:**

```
ControlPlane
PublicationPlane
ResourcePlane
MainLibraryEventPump
registerLibraryCommands
createLibraryCommandHandler
emitLibraryBrowserInvalidation
emitLibraryScanEvent
normaliseHostCommandFailure
normaliseTransportFailure
decodeCommandRequest
decodeCommandResponse
decodePublicationEvent
```

**Avoid — these imply too much or too little:**

```
ipcRouter
messageBus
eventHub
commandFramework
libraryCore
boundaryMagic
universalHandler
libraryEvents        (too vague — publication or control?)
handleLibraryEvent   (what kind of event? what layer?)
```

The spine is explicit infrastructure, not a framework. Names should communicate
ownership, not cleverness.

---

## Long-Term Direction

The Electron boundary spine is the bridge between today's Dekzer desktop app and a future
Exclave-compatible architecture. It should make future work easier without forcing
premature integration.

The plane vocabulary is designed to be Exclave-compatible now. When Exclave's typed
projection and resource surfaces are integrated, `ControlPlane`, `PublicationPlane`, and
`ResourcePlane` map directly to Exclave's control, publication, and resource planes
without renaming.

Future work should be able to add:

- source-root admission commands
- default music source discovery commands
- waveform resource handles
- artwork resource handles
- analysis blob resource handles
- warm runtime publication events
- hot realtime bindings over SharedArrayBuffer
- stricter generated runtime validators

without inventing new IPC patterns.

The pump model — Rust publishes, Main pumps, Renderer subscribes — must remain the
boundary shape as transport primitives evolve. A future `WaitForEventsAfter` long-poll or
shared-memory event ring replaces the `ReadAfter` loop inside the pump. It does not
change renderer authority.

The long-term target is not fewer boundaries. The target is boundaries that are explicit,
typed, testable, and owned.

---

## Summary

Dekzer's Electron boundary spine exists to prevent local-first desktop complexity from
becoming hidden renderer authority.

The spine must make these facts structurally true:

- commands are commands
- publications are publications
- resources are resources
- Rust publishes; Main pumps; Renderer subscribes
- product errors are not host failures
- host failures are not product errors
- service-not-ready is a host failure, not a product failure
- transport failures are not host failures
- generated contracts are the type source
- listener cleanup is mandatory
- channel ownership is centralised
- plane membership determines transport primitive
- product authority stays outside the IPC spine

This is core architecture.
