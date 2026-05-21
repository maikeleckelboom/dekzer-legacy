# Library Boundary Naming Inventory

## Verdict

Deeper names are generally correct rather than wrong. The contract types (`LiteralHierarchyNode`,
`LibraryAssetBrowserRow`, `ReadNavigationNodeLibraryBrowserWindowRequest`) carry explicit prefixes that serve as
wire-contract anchors across the Rust/TS boundary. The desktop main mapping layer (mapping.ts) correctly translates
contract names into shorter app-facing names (e.g., `sourceDirectoryId` → `directoryId`, `presenceState` → `presence`).
The shared DTO layer is already clean after recent commits. The remaining naming debt is concentrated in two areas: (a)
`sourceId`/`sourceLocationId` leaking through to the renderer DTO without translation, and (b) the
`LibraryNavigationRow` shared DTO being a 1:1 pass-through of the contract `NavigationRow` fields with a redundant
`Library` prefix added. No protocol regeneration is needed yet.

## Naming Layers

| Layer                             | Naming Rule                                                                                                                                                                                                                                                                                                |
|-----------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Rust/SQLite/store                 | `Store` prefix for store-layer row types (`StoreLiteralHierarchyNode`, `ScopedLibraryAssetBrowserRow`). Uses database-oriented field names (`source_directory_id`, `parent_source_directory_id`).                                                                                                          |
| Rust boundary protocol            | Derived with `ts_rs::TS`, serde `rename_all = camelCase`. Uses explicit type prefixes (`LiteralHierarchy`, `LibraryAsset`, `NavigationRow`). Variant names mirror store but with distinct type wrappers.                                                                                                   |
| Generated TS contract             | Exact structural mirror of the Rust protocol. All type names and field names match 1:1 (camelCase). Must not be edited by hand.                                                                                                                                                                            |
| Desktop main mapping              | The bridge. Translates contract names → shared DTO names. `sourceDirectoryId` → `directoryId`, `parentSourceDirectoryId` → `parentDirectoryId`, `sourceFileId` → `fileId`, `presenceState` → `presence`. Drops unused fields (`sourceId` on `LiteralHierarchyNode` is consumed internally, not forwarded). |
| Desktop shared DTO                | App-facing types. Uses shortened names: `EntryPoint` (not `LiteralHierarchyEntryPoint`), `ChildRow` (not `LiteralHierarchyNode`), `directoryId`/`fileId`/`parentDirectoryId`/`presence`. Navigation types add `Library` prefix (`LibraryNavigationRow`).                                                   |
| Renderer browser state/projection | Consumes shared DTO types exclusively. Uses `bindingsById` (not parallel projection maps). Owns types like `BrowserTreeNode`, `SourceState`, `DirectoryState`, `RowBinding`.                                                                                                                               |
| Generic tree primitive            | `BrowserTreeNode`, `BrowserTreeVisibleItem`, `BrowserTreeChildrenState`. Purely UI-model names with no library baggage.                                                                                                                                                                                    |

## Keep

Names that are long but acceptable, with reasons:

| Name                                                                                         | Layer      | Reason                                                                                                                                                                        |
|----------------------------------------------------------------------------------------------|------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `LiteralHierarchyEntryPoint`                                                                 | Contract   | Distinguishes it from other entry point concepts (navigation entry point, browser entry point). The "Literal" prefix is a product term meaning "actual filesystem hierarchy." |
| `LiteralHierarchyNode`                                                                       | Contract   | Same reason; "Literal" is the product distinction from navigation tree nodes and browser rows.                                                                                |
| `LiteralHierarchyWindow`                                                                     | Contract   | Windowed read pattern; mirrors `LibraryBrowserWindow` naming convention.                                                                                                      |
| `LiteralHierarchyNodeKind`                                                                   | Contract   | Enum values (`Directory`, `File`) are short; the type name needs the qualifier to avoid collision.                                                                            |
| `LiteralHierarchyPresenceState`                                                              | Contract   | Values are short (`Present`, `Missing`, `Removed`); type name is explicit about what kind of presence this is.                                                                |
| `LibraryAssetBrowserRow`                                                                     | Contract   | Distinct from `NavigationRow` and `LiteralHierarchyNode`. The `LibraryAsset` prefix is the product domain concept.                                                            |
| `LibraryBrowserWindow`                                                                       | Contract   | Consistent with `LiteralHierarchyWindow`; windowed read on the library browser pane.                                                                                          |
| `NavigationRowFamily` / `NavigationRowKind` / `NavigationRowSelectorKind`                    | Contract   | Short enum variants; type names distinguish these three related-but-different enums.                                                                                          |
| All `LibraryAssetPreparation*` types                                                         | Contract   | The full compound prefix is necessary domain vocabulary (preparation is a distinct library subsystem).                                                                        |
| `LibraryAssetWaveformOverview*` types                                                        | Contract   | Explicit about what kind of overview (waveform, not stems or beatgrid).                                                                                                       |
| `ReadLibraryAssetWaveformOverviewRequest/Reply`                                              | Contract   | Follows the `Read` + `Entity` + `Request/Reply` pattern consistently.                                                                                                         |
| `MaintainedSnapshotScopeRevision` / `MaintainedSnapshotScope` / `MaintainedSnapshotRevision` | Contract   | Infrastructure types; "Maintained" distinguishes from ephemeral snapshots.                                                                                                    |
| `BrowserTreeNode` / `BrowserTreeVisibleItem`                                                 | Renderer   | Generic tree view model; clean names with no library baggage.                                                                                                                 |
| `ChildRow`                                                                                   | Shared DTO | Good: short, app-facing, distinguishes from `NavigationRow`.                                                                                                                  |
| `EntryPoint`                                                                                 | Shared DTO | Good: shortened from `LiteralHierarchyEntryPoint`.                                                                                                                            |
| `Presence`                                                                                   | Shared DTO | Good: shortened from `LiteralHierarchyPresenceState`.                                                                                                                         |

## Rename Soon

| Name                                                                        | Layer      | Proposed                                                   | Risk | Reason                                                                                                                                                                                                                                                                     |
|-----------------------------------------------------------------------------|------------|------------------------------------------------------------|------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `LibraryNavigationRow`                                                      | Shared DTO | `NavigationRow`                                            | Low  | The shared DTO type re-adds a `Library` prefix that the contract already strips (contract uses `NavigationRow` without prefix). The `Library` prefix is redundant because the DTO already lives in a `libraryNavigation` module. The contract has the more canonical name. |
| `LibraryNavigationReadRowsResult` / `LibraryNavigationReadRowsState` / etc. | Shared DTO | Drop `Library` prefix throughout the libraryNavigation DTO | Low  | These types live in `shared/libraryNavigation/readRows.ts`; the module path already scopes them. `NavigationReadRowsResult` would match the contract's pattern better.                                                                                                     |

## Rename Later

Names that are probably wrong but require a deliberate protocol/regeneration sprint:

| Name                                                                                                | Layer                            | Proposed                                                                                                                    | Risk   | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
|-----------------------------------------------------------------------------------------------------|----------------------------------|-----------------------------------------------------------------------------------------------------------------------------|--------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `sourceId` on `EntryPoint` / `LiteralHierarchyEntryPoint`                                           | Contract → Shared DTO → Renderer | `sourceId` stays at contract. In shared DTO: consider whether it should be translated (e.g. `sourceId` already reads well). | Medium | This is the only database-noun field that leaks from contract all the way to renderer without translation. However, `sourceId` is a genuine product concept (the ID of a registered source root), not a database artifact. The `sourceLocationId` companion is similar. These pass through because the renderer needs them to construct request keys. The full translation chain is: store `source_id` → contract `sourceId` → shared DTO `sourceId` → renderer. The question is whether this is a leak or a legitimate domain identifier. |
| `sourceLocationId` on `EntryPoint`                                                                  | Contract → Shared DTO → Renderer | Same analysis as `sourceId`                                                                                                 | Medium | Same treatment. Both are product IDs, not DB table names.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `ReadNavigationNodeLibraryBrowserWindowRequest` / `SearchNavigationNodeLibraryBrowserWindowRequest` | Contract                         | Potentially `ReadBrowserWindowRequest` / `SearchBrowserWindowRequest`                                                       | High   | These are the longest names in the protocol. However, shortened names would collide with retired tags (see test `retired_direct_browser_transport_tags_are_not_accepted`). Renaming requires regenerating the contract and updating the desktop bridge, IPC channels, and all tests.                                                                                                                                                                                                                                                       |
| Contract reply variant `NavigationNodeLibraryBrowserWindow` / `NavigationNodeLibraryBrowserSearch`  | Contract                         | If request types are renamed, these must follow                                                                             | High   | Tag strings in `SnapshotReadReply` must match request rename decisions.                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |

## Do Not Rename

Names that are intentionally explicit and should not be shortened:

| Name                                                                      | Reason                                                                                                                                    |
|---------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| `parentNavigationRowId`                                                   | The "parent" qualifier is essential for tree semantics. Shortening to `parentRowId` would lose the entity it belongs to.                  |
| `navigationRowId`                                                         | Matches the entity name `NavigationRow`. Consistent with `libraryAssetId`, `sourceId`, etc.                                               |
| `parentSourceDirectoryId` (contract side only)                            | Must stay explicit at the contract boundary. The mapping layer correctly shortens it downstream.                                          |
| `primarySourceFileId` / `scopedSourceFileId` (contract)                   | These are distinct concepts (the canonical file vs. the file within the current scope). Must stay explicit.                               |
| `aggregateReadinessSummary`                                               | Domain term; "aggregate" distinguishes from per-capability readiness.                                                                     |
| `artifactCoverageState` / `capabilityUpdatedAtMs` / `artifactCreatedAtMs` | These are specific waveform-adapter fields on `LibraryAssetWaveformOverview`; renaming would break adapter integration.                   |
| `sourceProfileKey` / `sourceQualityCurrent` / `sourceSampleCount`         | Waveform overview fields; "source" here refers to the source audio profile, not the source root. The naming is correct in domain context. |
| `basisFingerprint` / `acceptedArtifactId`                                 | Artifact identification fields; domain vocabulary.                                                                                        |

## Boundary Leaks

Any backend/store/protocol names that still leak into renderer/shared app-facing code:

| Leak                                                                  | Location                                                                                                                       | Severity                       | Detail                                                                                                                                                                                                                                                                                                                                                                     |
|-----------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------|--------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `sourceId` / `sourceLocationId` on `EntryPoint`                       | `shared/libraryHierarchy/readChildren.ts:33,37` → renderer `hierarchyRead.ts` → `hierarchyProjection.ts` → `hierarchyState.ts` | Low                            | These are product identifiers (source root IDs and source location IDs), not database column names. They pass through the mapping without translation because the renderer needs them for request key construction (`createEntryPointRequestKey`). The shared DTO type `EntryPoint` mirrors the contract variant names exactly, but these happen to be good names already. |
| `sourceId` on `SourceTarget` in `hierarchyState.ts:15`                | `renderer/libraryBrowser/hierarchyState.ts`                                                                                    | Low                            | This is a renderer-convenience type that wraps the shared DTO's `EntryPoint`. It re-exposes `sourceId` / `sourceLocationId` indirectly. Not a leak since `EntryPoint` already exposes them.                                                                                                                                                                                |
| `navigationRowId` / `parentNavigationRowId` on `LibraryNavigationRow` | `shared/libraryNavigation/readRows.ts:51-53` → renderer `hierarchyProjection.ts`                                               | None (acceptable pass-through) | These are exact mirrors of the contract. The shared DTO chose to forward all fields 1:1. This is a deliberate design choice—navigation rows are already user-facing concepts.                                                                                                                                                                                              |

**Assessment**: The main mapping layer is effective. No database table names (`source_directory_id`,
`parent_source_directory_id`, `presence_state`) appear in the renderer or shared DTO. The only contract fields that
transit unchanged are `sourceId`/`sourceLocationId` (on `EntryPoint`) and all `NavigationRow` fields (on
`LibraryNavigationRow`)—both of which are product concepts, not database artifacts.

## Suggested Rename Map

| Current                               | Proposed                       | Layer      | Risk | Reason                                                                                                     |
|---------------------------------------|--------------------------------|------------|------|------------------------------------------------------------------------------------------------------------|
| `LibraryNavigationRow`                | `NavigationRow`                | Shared DTO | Low  | Contract uses `NavigationRow` without prefix; shared DTO re-adds `Library`. Module path already scopes it. |
| `LibraryNavigationReadRowsResult`     | `NavigationReadRowsResult`     | Shared DTO | Low  | Consistent prefix removal in `shared/libraryNavigation`.                                                   |
| `LibraryNavigationReadRowsState`      | `NavigationReadRowsState`      | Shared DTO | Low  | Same.                                                                                                      |
| `LibraryNavigationReadRowsErrorCode`  | `NavigationReadRowsErrorCode`  | Shared DTO | Low  | Same.                                                                                                      |
| `LibraryNavigationReadRowsErrorState` | `NavigationReadRowsErrorState` | Shared DTO | Low  | Same.                                                                                                      |
| `LibraryNavigationReadRowsError`      | `NavigationReadRowsError`      | Shared DTO | Low  | Same.                                                                                                      |
| `LibraryNavigationRowFamily`          | `NavigationRowFamily`          | Shared DTO | Low  | Contract uses `NavigationRowFamily` directly.                                                              |
| `LibraryNavigationRowKind`            | `NavigationRowKind`            | Shared DTO | Low  | Contract uses `NavigationRowKind` directly.                                                                |
| `LibraryNavigationRowSelectorKind`    | `NavigationRowSelectorKind`    | Shared DTO | Low  | Contract uses `NavigationRowSelectorKind` directly.                                                        |
| `LibraryNavigationReadRowsRequest`    | `NavigationReadRowsRequest`    | Shared DTO | Low  | Same prefix removal.                                                                                       |

## Recommended Next Prompt

> fix(desktop): drop redundant Library prefix from shared libraryNavigation DTO names

Scope: `apps/desktop/src/shared/libraryNavigation/readRows.ts` and all importers (main, renderer, tests).
Drop the `Library` prefix from all exported types in that file, matching the contract naming convention.
Update importers in `apps/desktop/src/main/libraryNavigation/readRows.ts`, renderer files, and test files.
Do not touch contract, protocol, or generate code.

## Files Inspected

**Rust protocol:**

- `crates/library-boundary-protocol/src/commands/snapshot_reads.rs` (1654 lines)
- `crates/library-boundary-service/src/snapshot_read_protocol.rs` (659 lines)
- `crates/library-boundary-service/src/service.rs` (971 lines)

**Generated contract:**

- `packages/library-boundary-contract/index.ts` (177 lines)

**Desktop main:**

- `apps/desktop/src/main/libraryHierarchy/mapping.ts` (82 lines)
- `apps/desktop/src/main/libraryHierarchy/request.ts` (222 lines)
- `apps/desktop/src/main/libraryHierarchy/readChildren.ts` (176 lines)
- `apps/desktop/src/main/libraryHierarchy/target.ts` (83 lines)
- `apps/desktop/src/main/libraryNavigation/readRows.ts` (204 lines)

**Desktop shared DTO:**

- `apps/desktop/src/shared/libraryHierarchy/readChildren.ts` (104 lines)
- `apps/desktop/src/shared/libraryNavigation/readRows.ts` (77 lines)

**Renderer:**

- `apps/desktop/src/renderer/libraryBrowser/hierarchyRead.ts` (978 lines)
- `apps/desktop/src/renderer/libraryBrowser/hierarchyProjection.ts` (606 lines)
- `apps/desktop/src/renderer/libraryBrowser/hierarchyState.ts` (132 lines)
- `apps/desktop/src/renderer/libraryBrowser/rootLifecycle.ts` (135 lines)
- `apps/desktop/src/renderer/libraryBrowser/localRootActions.ts` (286 lines)
- `apps/desktop/src/renderer/libraryBrowser/tree/types.ts` (47 lines)
- `apps/desktop/src/renderer/libraryBrowser/tree/projection.ts` (168 lines)
- `apps/desktop/src/renderer/libraryBrowser/tree/controller.ts` (123 lines)
- `apps/desktop/src/renderer/libraryBrowser/tree/keys.ts` (183 lines)
- `apps/desktop/src/renderer/libraryBrowser/tree/context.ts` (26 lines)

**Tests:**

- `packages/library-boundary-stdio-transport/tests/stdioTransportSmoke.ts` (146 lines)

## Top 5 Naming Concerns

1. **`LibraryNavigationRow` shared DTO prefix is redundant.** The contract exports `NavigationRow`. The shared DTO
   re-adds `Library` to every exported type in `shared/libraryNavigation/readRows.ts`, creating a divergence without
   adding information (the module path already scopes these types). This is the highest-priority cleanup because it's
   low-risk and touches only the desktop app, not the protocol.

2. **`sourceId` / `sourceLocationId` transit unchanged from contract to renderer DTO.** These are product IDs, not DB
   leaks, but the fact that they mirror the contract field names exactly (while other fields like `sourceDirectoryId`
   get translated to `directoryId`) creates an inconsistency in the mapping layer's thoroughness. The shared DTO's
   `EntryPoint` type is structurally identical to the contract's `LiteralHierarchyEntryPoint` variants.

3. **`ReadNavigationNodeLibraryBrowserWindowRequest` is the longest name in the protocol.** At 43 characters, it's
   verbose. However, renaming it would break the contract tags (`readNavigationNodeLibraryBrowserWindow`), require
   Rust-side rename + regeneration, and risk collision with retired tags. This is a rename-later item.

4. **The `LibraryNavigation*` shared DTO types mirror contract field names 1:1 without any field-level translation.**
   The `mapNavigationRow` function copies every field verbatim. This is acceptable for navigation rows (they're
   display-oriented), but contrasts with the hierarchy mapping which translates `sourceDirectoryId` → `directoryId`,
   etc.

5. **The navigation row mapping has no validation/enrichment layer.** Unlike the hierarchy mapping (which drops null
   rows and validates invariants), the navigation mapping is a pure shape-copy. If the contract navigation row structure
   ever changes, the shared DTO will drift silently. Consider whether a validation pass is warranted.
