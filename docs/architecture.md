# Architecture

This describes the system as it exists in this repository. It does not describe planned packages or a target directory
tree. The repository itself is the accurate source for current structure.

## Shape

```text
Vue renderer
    |  typed renderer API, no Electron primitives in feature code
Electron preload (contextBridge)
    |  command invoke and publication subscribe channels
Electron main: boundary spine and host lifecycle
    |  transport-agnostic TypeScript boundary client
Node stdio transport
    |  Rust-owned JSON-lines request, reply, and readiness envelopes
Rust boundary protocol and service
    |
Rust domain and SQLite store
```

Six crates and three packages implement this. `library-domain` holds durable vocabulary. `library-store-sqlite` owns
persistence. `library-boundary-protocol` defines the wire contract. `library-boundary-service` orchestrates.
`library-boundary-stdio` is the process entry point. `xtask` runs contract export and staleness checks. On the
TypeScript side, `library-boundary-contract` is generated, `library-boundary-client` is transport-agnostic, and
`library-boundary-stdio-transport` implements the current route.

The split is not a layering exercise. It exists because filesystem work blocks, drives disappear, the database must
outlive a renderer reload, and two languages have to agree on every state that can reach the user. Giving each of those
an explicit owner makes failures testable rather than incidental.

## Ownership

**Rust domain and SQLite** own durable state. One baseline migration defines the schema: sources, locators, state, scan
state, locations, directories, files, work items and artifacts, file observations, content attachments and links, the
search-filter index, playable media, track identity candidates and decisions, navigation rows, and the projection
change log. Everything durable that the product knows lives here. Nothing above this layer holds authority over it.

**The Rust boundary protocol** defines commands, replies, events, and validation vocabulary. It is the single source of
truth for cross-language shapes. It does not own presentation.

**The Rust boundary service** handles commands, publishes events, and runs bounded maintenance. Maintenance after a
scan performs one pass each of hashing, attachment materialization, probing, playable-media promotion, candidate
production, and decision production, then stops. Each pass attempts a bounded number of candidates, so it does not
enumerate and drain a whole source. It does not bound the time spent on any one candidate. Remaining work is
explicit-command or future-scheduler work. See
[the bounded work items decision](decisions/0003-bounded-work-items.md).

**Generated contracts** reproduce the Rust protocol as TypeScript types and JSON Schema. `pnpm library:contract:check`
and `pnpm library:stdio:contract:check` fail the build when the checked-in output no longer matches the Rust source, so
the TypeScript side cannot drift by hand-editing. See
[the boundary contract decision](decisions/0001-rust-owned-boundary-contract.md).

**The TypeScript client** keeps command semantics independent of transport. **The stdio transport** implements the
current cross-process route as JSON lines over stdin and stdout, with its own readiness envelope contract.

**Electron main** owns host lifecycle: backend process startup, binary resolution, storage location, readiness,
diagnostics, event pumping, IPC registration, and shutdown. Library integration lives under `src/main/library/`, split
by domain rather than by technical layer.

**Preload** exposes one narrow `contextBridge` API. Raw Electron primitives do not reach feature code, and every
listener registration returns a cleanup function.

**The Vue renderer** turns accepted reads into tree, contents, status, inspector, and interaction projections. It holds
interaction state: selection, expansion, focus, scroll. It holds no filesystem authority.

## Events

**Rust publishes. Main pumps. Renderer subscribes.**

The Rust service is the sole author of boundary events. Main reads them from the boundary stream, tracks the event
cursor, detects gaps, and forwards publication events to the renderer. The renderer subscribes to main-forwarded
publications and never schedules the read loop itself.

This matters because the alternative, renderer-initiated polling, makes the renderer a participant in event ordering
and gap recovery. Cursor tracking then has as many owners as there are windows. Keeping the loop in one main-process
owner leaves the renderer with subscription and cleanup only.

## Two plane types, two IPC primitives

Channel plane membership decides the Electron mechanism rather than only the naming.

**Control plane** channels use `ipcMain.handle` and `ipcRenderer.invoke`. One request, one typed result, promise-based.

**Publication plane** channels use `webContents.send` and `ipcRenderer.on`, always through a cleanup-returning wrapper.
They carry invalidations, lifecycle notifications, and summaries.

Neither carries large binary payloads. A channel constant belongs to exactly one plane. Waveform data, artwork, and
analysis blobs are not routed through either, which is a live constraint rather than a hypothetical one, because
waveform work is a future layer and this is the boundary it will have to respect.

## Three kinds of failure

Feature code sees one result type and does not write `try`/`catch` for normal library failures. Underneath, three
outcomes stay distinct because they need different responses.

A **product result** means the service ran and returned a domain outcome, success or failure: source already
registered, cursor invalid, scan already running.

A **host command failure** means the handler ran but host infrastructure failed: an unexpected exception,
serialization failure, validation failure, or the service not yet being ready. Service-not-ready is deliberately a host
failure and not a domain error, because it says nothing about the request and everything about timing. Encoding it as a
domain error teaches every caller to treat a startup race as a product condition.

A **transport failure** means the invoke never reached a handler: channel missing, preload absent, renderer destroyed.

## Exposure classes

Some operations are safe to call from the renderer and some are not, and the difference is about who supplies the
input.

**Renderer-callable** operations take renderer-safe inputs. `chooseAndRegisterLocal` qualifies because it takes no path
at all: main owns the native folder picker and the absolute path that comes back from it. Hierarchy and contents reads
qualify because they take scope identity rather than paths.

**Host-internal** operations may consume host-selected absolute paths. `registerLocalRoot` is one, and it is not
exposed through IPC, preload, or renderer feature code. A renderer that can name an arbitrary absolute path for
registration is a renderer that can register anything on the machine.

## Analysis

Two Rust crates sit outside the library spine. `analyzer-core` computes deterministic technical facts for supported
integer PCM WAV files. `analyzer-musical-stratum` backs the inspector's one-shot BPM, key, and beatgrid experiment for
16-bit PCM WAV. Both are headless and bounded. Neither persists results, and neither carries product authority. The
musical analysis path is reachable from the inspector and returns advisory or inconclusive outcomes with warnings and
basis information attached.

## What the architecture does not do

There is no scheduler. The work authority supports priority-ordered batch claiming and lease expiry reclaim, but
nothing drives it as one, and maintenance runs as bounded-count passes rather than a prioritized multi-lane queue.

There is no resource plane. It is reserved in the channel vocabulary and unimplemented, because no payload requires it
yet.

There is no renderer-side authority over filesystem state, and source guards in the test suite enforce that: no raw
library IPC outside the channel owner, no publication channel used with `invoke`, no renderer-scheduled event reads, no
locally redeclared contract types.

Packaged builds do not yet bundle the Rust backend executable, so the application currently runs from source. See
[development](development.md).
