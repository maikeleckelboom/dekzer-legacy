---
authority: support-map
purpose: Target implementation structure map. Not architecture authority.
canonical-context:
  - product/product-doctrine
  - library-preparation-substrate-v1
  - source-root-scan-admission-contract
---

# Dekzer Code Structure — Future Implementation Spec

_Support map for the library subsystem. All layers: Rust crates, TypeScript packages, Electron main, preload, shared types, renderer domain owners, Vue presentation._

_Naming conventions: Rust files use snake_case. TypeScript and Vue files use camelCase. Doc filenames use kebab-case._

---

## Target Principle

```text
Library is the product namespace.
Domains inside library own specific contracts.
No single "library controller" becomes the mayor of everything.
```

Layer map:

```text
durable vocabulary          crates/library-domain/
durable substrate           crates/library-store-sqlite/
boundary contracts (Rust)   crates/library-boundary-protocol/
boundary service (Rust)     crates/library-boundary-service/
boundary contracts (TS)     packages/library-boundary-contract/
boundary client (TS)        packages/library-boundary-client/
host integration            apps/desktop/src/main/library/
IPC surface                 apps/desktop/src/preload/rendererApi.ts
main↔renderer types         apps/desktop/src/shared/library/
renderer domain owners      apps/desktop/src/renderer/library/
Vue presentation            *.vue files inside domain folders
decision records            docs/decisions/
```

---

## Naming Law

```text
Folder context carries the domain noun.
File names must not repeat the parent folder name unless the repetition
is required for package identity, external API clarity, or disambiguation
across imports.

Prefer role names inside domain folders:
  root.vue, row.vue, read.ts, run.ts, status.rs, create.rs

Public functions and exported types may remain explicit because
call sites need meaning without relying on file paths.

SQL table names use full durable vocabulary:
  preparation_facets
Rust files use full vocabulary when folder context does not already carry it.
Inside a domain folder, prefer role names:
  preparation/apply_decision.rs
Renderer folders and API call sites use short vocabulary:
  prep/
  library.prep
These are not interchangeable — the split is intentional.
```

---

## Monorepo Shape

```text
crates/
  library-domain/
  library-store-sqlite/
  library-boundary-protocol/
  library-boundary-service/

packages/
  library-boundary-contract/
  library-boundary-client/

apps/
  desktop/
    src/
      main/
      preload/
      shared/
      renderer/

docs/
  decisions/
  contracts/
```

---

## `crates/`

### `library-domain/`

Pure vocabulary. No SQLite, no IPC, no transport. The shared type surface
that `library-store-sqlite`, `library-boundary-protocol`, and
`library-boundary-service` all depend on. Decouples the domain model from
any persistence or wire representation.

```text
crates/library-domain/src/
  lib.rs
  ids.rs                 --  SourceId, DirectoryId, FileId, TrackId, SleeveId, …
  sources.rs             --  Source, SourceLocation, AvailabilityState, MountState
  hierarchy.rs           --  Directory, HierarchyRow, HierarchyPage
  contents.rs            --  SourceFile, PrimaryMedia, ContentRow, ContentsPage
  browse.rs              --  BrowseScope, BrowsePolicy, BrowseOrder
  organization.rs        --  Crate, Playlist, SmartList, Folder, Tag
  preparation.rs         --  FacetKind, FacetState, ReadinessProjection, PreparationJob, PreparationArtifact
  sleeves.rs             --  Sleeve, SleeveContents, SleeveIntent
  history.rs             --  HistoryEvent, EventKind
  search.rs              --  SearchQuery, SearchResults, FilterSet
  scan.rs                --  ScanStatus, ScanEvent, ScanPhase
  diagnostics.rs         --  DiagnosticIssue, IssueKind, CheckResult
```

---

### `library-store-sqlite/`

The durable library substrate. Rust + SQLite. Owns the ledger.

```text
crates/library-store-sqlite/src/
  lib.rs

  schema/
    migrations/
    bootstrap.rs
    ids.rs

  substrate/
    sources.rs
    locations.rs
    directories.rs
    source_files.rs
    media_roles.rs
    primary_media.rs
    presence.rs
    scan_coverage.rs

  store/
    sources.rs
    scan.rs
    hierarchy.rs
    contents.rs
    organization.rs
    preparation.rs
    sleeves.rs
    history.rs
    requests.rs
    imports.rs
    exports.rs
    devices.rs
    search.rs

  read_models/
    sources/
      summary.rs
      availability.rs
    scan/
      status.rs
      coverage.rs
    hierarchy/
      page.rs
      row.rs
    contents/
      page.rs
      row.rs
    organization/
      browser.rs
      crates.rs
      playlists.rs
      smart_lists.rs
      folders.rs
      tags.rs
    preparation/
      status.rs
      facet.rs
      readiness.rs
    browse/
      order.rs
    sleeves/
      detail.rs
      contents.rs
    search/
      results.rs
    history/
      event.rs
    exports/
      manifest.rs
    devices/
      capabilities.rs
    diagnostics/
      issue.rs

  write_models/
    sources/
      register.rs
      remove.rs
      scan.rs
    metadata/
      update.rs
    preparation/
      apply_decision.rs
    organization/
      crates/
        create.rs
        rename.rs
        delete.rs
      playlists/
        create.rs
        reorder.rs
        delete.rs
      tags/
        create.rs
        assign.rs
        remove.rs
    sleeves/
      create.rs
      update.rs
      delete.rs
    history/
      record.rs
    imports/
      claim.rs
      resolve.rs
    exports/
      snapshot.rs
      rollback.rs
    requests/
      enqueue.rs
      resolve.rs

  diagnostics/
    integrity_checks.rs
    query_plans.rs
    boundary_errors.rs
```

**Owns:** source registration and lifecycle, literal source hierarchy, source
file inventory, media classification, primary media projection, contents reads,
browse order, scan coverage, preparation artifact state, organization objects,
integrity diagnostics.

**Does not own:** presentation, interaction, UI state, or renderer decisions.

---

### `library-boundary-protocol/`

The Rust-side contract. Commands are requests from the host into the service.
Events are facts emitted by the service to any subscriber.

```text
crates/library-boundary-protocol/src/
  lib.rs
  contract.rs

  commands/
    mod.rs
    sources.rs
    hierarchy.rs
    contents.rs
    search.rs
    organization.rs
    prep.rs
    sleeves.rs
    history.rs
    requests.rs
    imports.rs
    exports.rs
    devices.rs
    diagnostics.rs

  events/
    mod.rs
    sources.rs
    scan.rs
    hierarchy.rs
    organization.rs
    prep.rs
    sleeves.rs
    history.rs
    requests.rs
    imports.rs
    exports.rs
    devices.rs
    diagnostics.rs
```

---

### `library-boundary-service/`

The Rust-side service. Handles incoming commands and publishes events.
Each handler module owns one domain's command set.

`service.rs` is orchestration only: lifecycle, handler registry, dispatch
wiring, event publisher wiring. No business logic creeps into `service.rs`.
Command semantics, store calls, and domain-specific validation belong in
the relevant `handlers/` module.

```text
crates/library-boundary-service/src/
  lib.rs
  service.rs
  snapshot_read_protocol.rs
  command_protocol.rs
  event_protocol.rs

  handlers/
    mod.rs
    sources.rs
    hierarchy.rs
    contents.rs
    search.rs
    organization.rs
    prep.rs
    sleeves.rs
    history.rs
    requests.rs
    imports.rs
    exports.rs
    devices.rs
    diagnostics.rs
```

---

## `packages/`

### `library-boundary-contract/`

Generated from the Rust boundary manifest. Desktop main uses it directly.
Renderer receives compatible typed shapes through `preload/rendererApi.ts` —
not by depending on this package directly.

```text
packages/library-boundary-contract/
  index.ts
  manifest.json
  boundary-contract.schema.json
```

---

### `library-boundary-client/`

TypeScript client for desktop main. Renderer code never imports this.
That is an Electron trust boundary, not a convenience preference.

```text
packages/library-boundary-client/src/
  client.ts
  errors.ts
  transport.ts
```

---

## `apps/desktop/src/`

### `main/library/`

Host-side integration. Wires the Rust service to IPC. Each sub-module
handles one domain's command dispatch. All domains live under
`main/library/` so the namespace mirrors `shared/library/` and
`renderer/library/` exactly.

```text
apps/desktop/src/main/library/
  host.ts

  sources/
    register.ts
    remove.ts
    rescan.ts

  scan/
    run.ts
    status.ts

  hierarchy/
    read.ts

  contents/
    read.ts

  search/
    query.ts

  organization/
    crates.ts
    playlists.ts
    smartLists.ts
    folders.ts
    tags.ts

  prep/
    jobs.ts
    artifacts.ts
    readiness.ts

  sleeves/
    read.ts
    write.ts

  history/
    record.ts
    read.ts

  requests/
    enqueue.ts
    resolve.ts

  imports/
    claim.ts
    validate.ts

  exports/
    snapshot.ts
    rollback.ts

  devices/
    capabilities.ts
    targets.ts

  diagnostics/
    runCheck.ts
    listIssues.ts
    boundaryErrors.ts
```

---

### `preload/`

The only channel renderer code uses to reach the host.
All domain namespaces are composed into the single renderer API here.

```text
apps/desktop/src/preload/
  rendererApi.ts
```

---

### `shared/library/`

Types and contracts shared between main and renderer. No logic.
Mirrors the domain structure of `main/library/` and `renderer/library/`.

```text
apps/desktop/src/shared/library/
  sources/
    register.ts
    remove.ts
    scan.ts

  scan/
    status.ts

  hierarchy/
    read.ts

  contents/
    read.ts

  search/
    query.ts

  organization/
    crates.ts
    playlists.ts
    smartLists.ts
    folders.ts
    tags.ts

  prep/
    jobs.ts
    readiness.ts
    facets.ts

  sleeves/
    types.ts

  history/
    types.ts

  requests/
    types.ts

  imports/
    types.ts

  exports/
    types.ts

  devices/
    types.ts

  diagnostics/
    errors.ts
    issues.ts
```

---

### `renderer/library/`

The renderer-side library system. Each subfolder is a domain with a
defined owner and a defined boundary. Controllers talk to the host via
`rendererApi.ts` — never via `library-boundary-client` directly.

```text
apps/desktop/src/renderer/library/
  panel.vue
  context.ts

  shell/
    surface.vue
    sourceToolbar.vue
    scanBanner.vue
    browser.vue
    splitPane.vue

  sources/
    controller.ts
    projection.ts
    actions.ts
    types.ts

  scan/
    controller.ts
    projection.ts
    copy.ts
    types.ts

  hierarchy/
    controller.ts
    projection.ts
    replay.ts
    bindings.ts
    actions.ts
    types.ts

    tree/
      controller.ts
      projection.ts
      root.vue
      group.vue
      row.vue
      item.vue

    columns/
      controller.ts
      projection.ts
      path.ts
      types.ts
      root.vue
      pane.vue
      row.vue

  contents/
    controller.ts
    projection.ts
    pagination.ts
    actions.ts
    types.ts
    table.vue

  selection/
    controller.ts
    binding.ts
    scope.ts
    types.ts

  browsePolicy/
    visibility.ts
    contents.ts
    hierarchy.ts
    types.ts

  browseOrder/
    profile.ts
    types.ts

  viewState/
    store.ts
    restore.ts
    persistence.ts
    types.ts

  search/
    controller.ts
    projection.ts
    actions.ts
    types.ts
    results/
      controller.ts
      projection.ts
      types.ts
      table.vue
    filters/
      bpm.ts
      key.ts
      energy.ts
      readiness.ts
      duplicates.ts
      types.ts

  organization/
    controller.ts
    types.ts
    playlists/
      controller.ts
      projection.ts
      actions.ts
      types.ts
      list.vue
      item.vue
    crates/
      controller.ts
      projection.ts
      actions.ts
      types.ts
      list.vue
      item.vue
    smartLists/
      controller.ts
      projection.ts
      actions.ts
      types.ts
      list.vue
    folders/
      controller.ts
      projection.ts
      actions.ts
      types.ts
    tags/
      controller.ts
      projection.ts
      actions.ts
      types.ts
      chip.vue

  prep/
    controller.ts
    types.ts

    jobs/
      controller.ts
      projection.ts
      actions.ts
      types.ts
      list.vue

    readiness/
      controller.ts
      projection.ts
      types.ts
      badge.vue
      panel.vue

    artifacts/
      controller.ts
      projection.ts
      types.ts

    beatGrid/
      types.ts
      editor.vue

    cuePoints/
      types.ts
      editor.vue

    phraseMarkers/
      types.ts
      editor.vue

    waveform/
      types.ts

    analysis/
      types.ts

    loudness/
      types.ts

    key/
      types.ts

    energy/
      types.ts

    stems/
      types.ts

    notes/
      types.ts
      editor.vue

    transitionIdeas/
      types.ts

  sleeves/
    controller.ts
    projection.ts
    actions.ts
    types.ts
    list.vue
    detail.vue
    candidates.vue

  history/
    controller.ts
    projection.ts
    types.ts
    playHistory/
      controller.ts
      projection.ts
      types.ts
      list.vue
    importHistory/
      controller.ts
      types.ts
      list.vue
    scanHistory/
      types.ts

  requests/
    controller.ts
    projection.ts
    types.ts
    queue/
      controller.ts
      types.ts
      list.vue
    eligibility/
      controller.ts
      types.ts

  imports/
    controller.ts
    types.ts
    rekordbox/
      controller.ts
      projection.ts
      types.ts
      wizard.vue
    serato/
      controller.ts
      projection.ts
      types.ts
      wizard.vue
    traktor/
      controller.ts
      projection.ts
      types.ts
      wizard.vue
    engine/
      controller.ts
      projection.ts
      types.ts
      wizard.vue
    filesystem/
      controller.ts
      projection.ts
      types.ts
      wizard.vue

  exports/
    controller.ts
    types.ts
    usbExport/
      controller.ts
      projection.ts
      actions.ts
      types.ts
      wizard.vue
    validation/
      controller.ts
      types.ts
    rollback/
      controller.ts
      types.ts

  devices/
    controller.ts
    projection.ts
    types.ts
    exportTargets/
      controller.ts
      projection.ts
      types.ts
      list.vue
    preflight/
      controller.ts
      projection.ts
      types.ts
      panel.vue

  diagnostics/
    controller.ts
    projection.ts
    actions.ts
    types.ts
    boundaryErrors.ts
    debugEvents.ts
    checks/
      missingFiles.ts
      staleMetadata.ts
      duplicateTracks.ts
      duplicateAttachments.ts
      brokenCues.ts
      unsupportedFiles.ts
      scanFailures.ts
      exportRisks.ts
    panel.vue
```

---

## Renderer Domain Status

| Domain         | Product question                                   | Surface                                                               |
| -------------- | -------------------------------------------------- | --------------------------------------------------------------------- |
| `sources`      | Where does music come from?                        | active                                                                |
| `scan`         | What is the scanner doing right now?               | active                                                                |
| `hierarchy`    | Where is it in the source structure?               | active                                                                |
| `contents`     | What files are in this selected scope?             | active                                                                |
| `selection`    | What is the selected backend scope?                | active                                                                |
| `browsePolicy` | What policy does this visibility mode map to?      | active                                                                |
| `browseOrder`  | What ordering identity does the backend own?       | active stub                                                           |
| `viewState`    | What is the persisted renderer view intent?        | active                                                                |
| `diagnostics`  | Can this library be trusted tonight?               | active partial — boundary errors + debug events; check cockpit future |
| `search`       | What matches this intent across the library?       | future surface                                                        |
| `organization` | How does the DJ arrange intent?                    | future surface                                                        |
| `prep`         | What do we know and what is ready?                 | jobs/readiness surface future; facet editors beyond that              |
| `sleeves`      | What working stacks exist?                         | future surface                                                        |
| `history`      | What happened, and when?                           | future surface                                                        |
| `requests`     | What is incoming from outside?                     | future surface                                                        |
| `imports`      | What is being repatriated?                         | future surface                                                        |
| `exports`      | What is being committed to a target?               | future surface                                                        |
| `devices`      | What targets exist and what are their constraints? | future surface                                                        |

The tree above is target shape — not a demand to create every file now.
All north-star product domains are represented so the product map is explicit.
The renderer also carries active infrastructure domains that are not product domains.

```text
Future domains may exist as boundary stubs.
Future surfaces may not exist as fake UI.
```

**Stub law:** A future-domain folder may exist to reserve an ownership boundary,
but it may contain only `controller.ts`, `types.ts`, or pure facet types until
real behavior exists. No `projection.ts`, `actions.ts`, or Vue component may
appear without an implemented data path and an owner. A stub must be tied to
an implementation phase in the sequence below.

---

## Renderer Domain Key Laws

### `panel.vue`

Composition surface only. Creates controllers, connects projections, renders
shell, routes emitted actions. Nothing else.

### `sources/`

Source lifecycle: add, remove, rescan, list, availability.
Does not own tree rows, contents rows, selected scope, or scan internals.

### `scan/`

Scan status and summary copy. Wording for scan state lives here — not in
contents, not in panel. Owns the distinction between files discovered,
primary media discovered, image discoveries, and jobs scheduled.

### `hierarchy/`

`tree/` and `columns/` are presentation variants of the same data authority.
They share the hierarchy read boundary, selection binding, browse policy,
browse order, and view-state primitives. They differ in presentation model —
not in data authority.

**Key law:** expanded state and selected node are view intent. Loaded child
windows are visibility-specific cache. These must stay separate.

### `contents/`

**Key law:** contents rows must be backend/query-owned, not
renderer-derived from loaded tree state. The `sourceFile` row profile and
the `primaryMedia` row profile are distinct. `browsePolicy/` governs which
profile is active for a given visibility mode.

### `selection/`

Owns the mapping from selected node to selected backend scope. Prevents every
controller from running its own binding lookup ritual. Owns: selectedNodeId,
selected hierarchy binding, contents scope, source vs sourceLocation
vs directory scope resolution, selection unavailable state.

### `browsePolicy/`

Owns the mapping from renderer product mode to boundary policy values. Mode
names live here. The backend receives policy values, not UI mode names.

### `browseOrder/`

**Key law:** the renderer does not reorder paginated backend results. The backend
owns browse order. Cursor payloads encode the ordering identity. Eventually
owns: sourceBrowseOrder, primaryMediaOrder, crateOrder, playlistOrder,
requestQueueOrder, historyOrder.

### `viewState/`

Renderer view intent only — not backend state. Owns: expandedNodeIds,
selectedNodeId (if persisted), last source visibility mode, panel split
position, restoration rules.

### `search/`

Not only text search. Filters span BPM, key, energy, readiness, missing-file
state, duplicate track / attachment detection, and path. Filter modules are pure functions —
no state of their own.

### `organization/`

Semantic law: a manual crate is not a smart query. A smart list is not a
crate. A frozen smart result may become a materialized snapshot, but then it
is a new object with its own identity. See doctrine §9.3.

`sleeves` is a top-level domain alongside `organization`, not nested inside it.

### `prep/`

Preparation is a plane, not one status. Each facet is independently evaluated.
Readiness is a projection from evidence across facets — not a field on the
track.

```text
Preparation is the work plane.
Readiness is the projected answer.
Facets are the independently evaluated dimensions.
Artifacts are the evidence.
Jobs produce the evidence.
```

The first active prep surface is jobs/readiness/artifacts. Facet editors
(beatGrid, cuePoints, phraseMarkers, notes) appear when their editor surfaces
are implemented. Facet type stubs are present now so the domain shape is
not invented at implementation time. See doctrine §7.

### `sleeves/`

Sleeves are workflow objects. Not playlists. Not organization. A sleeve
gathers tracks, notes, ordering intent, readiness facts, alternates, and
purpose. Sleeves may reference organization objects, but they are not part
of `organization/`. First-class domain, own service.

### `history/`

Not only "played tracks." Feeds provenance, trust, undo, and RT Flight Deck.
See doctrine §16.

### `requests/`

Event and mobile workflows: queue, requester, dedupe, eligibility, notes,
status. Stub here makes mic workflows, fallback automix, second-screen UX,
and post-event reporting possible without a future architecture rupture.

### `imports/`

Repatriation, not middleware. An imported fact is a claim in state `imported`
— not native authority. See doctrine §6 (Claim Lifecycle) and §V
(Repatriation). Each system (rekordbox, serato, traktor, engine, filesystem)
is a governed importer with provenance and validation.

### `exports/`

Exports produce named, durable snapshots. Rollback is part of the recovery
doctrine (§13).

### `devices/`

Capability-based hardware abstraction, not brand-first compatibility lists.
Compatibility verdicts record target profile, catalog version, evaluation
timestamp, and evidence grade. See doctrine §7.4 and §11.

### `diagnostics/`

The library trust cockpit. Not optional polish. Part of the trust contract.
Check modules are pure functions registered with the diagnostics controller.
The panel surfaces all issue kinds. Boundary errors and debug events are
active now; check cockpit is future.

---

## Shared Boundary API

The public app API. Each subdomain has a service or controller with one job.
No single `libraryController`.

```ts
library: {
  sources: {
    list(): Promise<Source[]>
    register(path: string): Promise<Source>
    remove(id: SourceId): Promise<void>
    rescan(id: SourceId): Promise<void>
  }

  scan: {
    status(id: SourceId): Promise<ScanStatus>
    subscribe(id: SourceId, handler: ScanEventHandler): Unsubscribe
  }

  hierarchy: {
    read(request: HierarchyReadRequest): Promise<HierarchyPage>
    // HierarchyReadRequest: { scope, policy, cursor, limit }
  }

  contents: {
    read(request: ContentsReadRequest): Promise<ContentsReadResult>
    // ContentsReadRequest: { scope, policy, scopeDepth, limit, cursor }
  }

  search: {
    query(q: SearchQuery): Promise<SearchResults>
  }

  organization: {
    crates: CrateService
    playlists: PlaylistService
    smartLists: SmartListService
    folders: FolderService
    tags: TagService
  }

  sleeves: SleeveService

  prep: {
    jobs: PrepJobService
    artifacts: PrepArtifactService
    readiness: ReadinessService
  }

  history: {
    play: PlayHistoryService
    imports: ImportHistoryService
    scans: ScanHistoryService
  }

  requests: {
    queue: RequestQueueService
    eligibility: EligibilityService
  }

  imports: {
    rekordbox: ImporterService
    serato: ImporterService
    traktor: ImporterService
    engine: ImporterService
    filesystem: ImporterService
  }

  exports: {
    usb: ExportService
    manifest: ManifestService
    rollback: RollbackService
  }

  devices: {
    capabilities: CapabilityService
    targets: ExportTargetService
    preflight: PreflightService
  }

  diagnostics: {
    runCheck(check: DiagnosticCheck): Promise<DiagnosticResult>
    listIssues(scope: DiagnosticScope): Promise<Issue[]>
  }
}
```

---

## Implementation Sequence

```text
Phase 1 (current):
  Fix contents-read failure.
  Finish pagination and action correctness.
  Commit the library contents boundary as stable.

Phase 2 (cleanup):
  Apply naming law to renderer tree (mechanical — no behavior changes).
  Move shell files: librarySurface.vue → surface.vue, browseSurface.vue → browser.vue.
  Move hierarchy Vue files: treeRoot.vue → tree/root.vue, etc.
  Move diagnostics: boundaryErrorProjection.ts → boundaryErrors.ts.
  Move main/hierarchy/readChildren.ts → read.ts; scan/runScan.ts → run.ts.
  Add tests around domain boundaries.
  Do not create future-domain files during this pass unless they already have real behavior.

Phase 3 (column browser):
  Build column browser on the clean hierarchy/columns/ shape.
  hierarchy/columns/ shares data authority with hierarchy/tree/.

Phase 4 (search):
  Query surface. Filter sub-modules as pure functions.

Phase 5 (organization):
  Playlists, crates, smartLists, folders, tags.
  Each sub-domain gets its own controller and projection.

Phase 6 (prep):
  Jobs and readiness surface first.
  Artifacts projection.
  Facet editors (beatGrid, cuePoints, phraseMarkers) when their DSP pipeline
  is stable enough to surface.

Phase 7 (sleeves):
  Working stack domain. Own service.

Phase 8 (imports):
  Governed repatriation per system.

Phase 9 (exports, devices):
  USB export, capability-based device abstraction, preflight.

Phase 10 (diagnostics cockpit, history, requests):
  Full check cockpit. Play history. Request queue.
```

---

_Flag: Will it work tonight? The folder shape must make the answer inspectable._
