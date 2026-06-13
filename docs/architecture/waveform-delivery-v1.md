# Waveform Delivery V1

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define the proposed V1 delivery boundary for waveform manifests and binary waveform tiles.

The delivery design must support deck-visible waveforms, overview waveforms, inspector waveforms, and future stem lanes without blocking frames or routing binary payloads through JSON IPC.

## Decision

V1 tile delivery uses:

    custom privileged app protocol + renderer worker fetch

Control-plane APIs may return readiness, manifest summaries, and opaque tile fetch descriptors. Binary tile payloads are fetched by a renderer worker through the app protocol.

## Rejected for V1

| Rejected mechanism | Reason |
| --- | --- |
| Normal JSON IPC for tile bytes | Structured clone / serialization overhead and high copy pressure for waveform tiles. |
| Multiple competing delivery modes | Produces divergent cache and parser contracts. V1 needs one path. |
| SharedArrayBuffer as a requirement | Requires cross-origin isolation and a torn-read protocol. Useful later for hot lanes, not required for tile bytes. |
| Electron mmap_file_range abstraction | Electron renderers do not receive a first-class safe mmap file-range primitive for app-owned files. |
| Raw filesystem paths to renderer | Breaks containment, permission, and future cache ownership. |

## Boundary split

### Control plane

Small structured messages:

- readiness
- manifest summary
- tile descriptors
- deck visual bundle summaries
- missing ranges
- stale/failure states

### Data plane

Binary tile bytes:

- fetched by worker,
- bounded by descriptor,
- validated by artifact id and basis hash,
- served by app protocol from artifact store,
- parsed off main thread.

## App protocol requirements

The app protocol must:

- be privileged for fetch use,
- reject arbitrary paths,
- require opaque tile descriptor or signed/validated token,
- validate artifact id and basis hash,
- enforce byte and tile limits,
- emit cache headers only if safe for app-local artifacts,
- never expose raw local filesystem paths,
- never block the renderer main thread.

Example conceptual URL shape:

    app-waveform://tile/{descriptor-id}

The descriptor id resolves inside the main/backend process to a specific artifact/lod/tile range. The URL is not a path and not stable public identity.

Descriptor URLs are delivery handles, not stable artifact identity. Stable identity remains the artifact id plus basis
hash and target identity.

## Electron Security Checklist

The implementation must satisfy these checks before any renderer can fetch tile bytes:

- privileged scheme registration for the app waveform protocol
- no arbitrary path resolution from renderer input
- descriptor expiry
- origin restrictions
- artifact id and basis hash validation
- descriptor and requested tile range validation
- byte and tile count limits
- no raw filesystem path exposure

## Manifest summary

`WaveformManifestSummary`:

| Field | Meaning |
| --- | --- |
| `artifactId` | Stable artifact id. |
| `basisHash` | Canonical basis hash for validation. |
| `targetKind` | Target kind, such as `playable_media_full_mix` or future `audio_component`. |
| `targetId` | Target id. |
| `schemaKey` | Column schema key. |
| `trackLengthSamples` | Track-local/component-local sample length. |
| `analysisSampleRate` | Analysis sample rate. |
| `lods` | List of LOD summaries. |
| `normalizationHints` | Rendering hints, not basis inputs. |
| `readiness` | Artifact readiness. |

`WaveformLodSummary`:

| Field | Meaning |
| --- | --- |
| `lod` | LOD number 0..4. |
| `bucketSamples` | Samples per column. |
| `columnCount` | Number of columns. |
| `columnBytes` | Column byte width. |
| `tileColumnCount` | Columns per tile. |
| `tileCount` | Tile count. |
| `persistence` | persistent, partial, on_demand, evicted. |

## Tile descriptors

`TileFetchDescriptor`:

| Field | Meaning |
| --- | --- |
| `descriptorId` | Opaque descriptor id used by app protocol. |
| `artifactId` | Artifact id. |
| `basisHash` | Basis hash expected by renderer. |
| `lod` | LOD number. |
| `tileIndex` | Tile index. |
| `columnStart` | First column. |
| `columnCount` | Number of columns. |
| `columnBytes` | Column byte width. |
| `payloadBytes` | Derived payload byte length. |
| `fetchUrl` | App-protocol URL. |
| `expiresAt` | Descriptor expiration, if enforced. |

`TileRangeDescriptor`:

| Field | Meaning |
| --- | --- |
| `sampleStart` | Track-local sample start. |
| `sampleEnd` | Track-local sample end. |
| `lod` | Requested LOD. |
| `reason` | deck_visible, deck_prefetch, stem_lane_visible, overview, inspector, edit_zoom. |
| `maxBytes` | Hard limit for this range. |

`TileMissingReason`:

- `not_generated`
- `evicted`
- `corrupt`
- `over_budget`
- `basis_mismatch`
- `artifact_not_ready`

## Batch request

`ReadWaveformBatchRequest` is control-plane only. It asks the backend for descriptors and readiness, not for raw tile bytes.

Fields:

- batch reason
- windows, each with surface id, artifact id, target id, lod, sample window
- max total bytes
- max total tiles

`ReadWaveformBatchResponse` returns:

- tile fetch descriptors
- missing ranges
- readiness changes
- descriptor expiration

It does not return binary bytes in JSON.

## Deck visual bundle

Cold deck visual bootstrap should avoid three serialized round trips.

`PrepareDeckVisualBundleRequest` includes:

- deck id
- playable media id or future audio component target
- initial sample window
- include full mix
- include stems if ready/required/none
- include overview
- include visible window
- max total bytes and tiles

`PrepareDeckVisualBundleResponse` includes:

- full-mix readiness
- optional stem-set readiness
- manifest summaries
- initial tile descriptors
- missing/pending work summary

Binary tiles still use worker fetch through descriptor URLs.

## Waveform readiness

`WaveformReadiness` separates artifact readiness from playback readiness:

- missing
- queued
- running
- partial
- ready
- stale
- failed
- evicted

Stem set readiness must separately track:

- source availability
- alignment readiness
- playback bundle readiness
- waveform readiness
- real-time playable boolean

A stem set can be playback-ready while waveform is missing, or waveform-ready while playback bundle is missing. These states must not be conflated.

## Normalization hints

`NormalizationHints` are renderer hints, not artifact basis inputs:

- component peak absolute
- parent mix peak absolute, if known
- stem set peak absolute, if known
- suggested default amplitude policy

Updating parent/stem-set peak hints must not invalidate waveform artifacts.

## Surface context

`WaveformSurfaceContext` is renderer-owned:

- surface id
- target kind/id
- component kind
- stem role, if any
- color token, if any
- amplitude policy
- orientation
- render quality policy

Artifacts carry signal facts. Renderer context carries color and presentation policy.

## Contract location decision

Future implementation should place contracts in layers by ownership:

| Contract | Proposed owner |
| --- | --- |
| Artifact/readiness/request semantics | Rust boundary protocol and generated contract package. |
| App protocol URL construction | Desktop main/preload boundary, not general library protocol. |
| Renderer surface context | Desktop renderer shared types. |
| Binary column/tile schemas | Shared architecture docs first; later Rust + TS constants generated or mirrored from a single source. |
| Worker parser types | Renderer implementation detail derived from generated/shared schema constants. |

Do not create a new package until the boundary generation pattern clearly requires it.

## Frame path law

At frame time, renderer behavior is:

- tile present: render it,
- lower LOD present: render fallback,
- tile missing: render placeholder/gap/previous image and request asynchronously,
- upload budget exceeded: defer upload and keep previous/lower-detail visual,
- descriptor expired: request a new descriptor asynchronously.

Never wait on fetch, IPC, SQLite, file IO, tile parse, GPU upload, or analysis completion.

Acceptance condition: the renderer main thread must never fetch tile bytes. Tile byte fetch and parse belong to a
renderer worker or later equivalent off-main-thread data plane.
