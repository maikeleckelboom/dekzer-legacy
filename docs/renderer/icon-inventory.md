---
status: candidate
ratification-target: renderer-icon-inventory-v1
revision: inventory-pass-3
doctrine-version: 0.2
last-reviewed: 2026-06-11
owner: renderer-icon-surface
canonical-context:
  - product/product-doctrine
  - local-source-icon-doctrine
scope:
  - renderer-icon-registry
  - icon-semantic-roles
  - lucide-to-semantic-registry-migration
  - vendored-svg-substrate
  - tree-icon-resolution
  - contents-table-icon-resolution
---

# Renderer Icon Inventory

## Purpose

Inventory every current renderer icon export and every usage site. Define the semantic role namespace and
implementation constraints for a controlled migration from the Lucide-backed surface to a **Dekzer semantic icon
registry backed by vendored Fluent SVGs**.

This document is a **design inventory and boundary map**. No visual migration is performed in this pass.

---

## 1. Icon Adapter Architecture

### 1.1 Current state

```
src/renderer/icons/
  index.ts          barrel: re-exports icons, types, and the Icon component
  types.ts          IconComponent, IconSize, IconTone, IconFrame, IconProps
  tokens.ts         CSS class maps for size/tone
  icon.vue          Icon wrapper component (renders any IconComponent via <component :is>)
  lucide.ts         adapter: imports @lucide/vue symbols, aliases them to semantic names
  custom.ts         hand-drawn Vue components for icons not in Lucide (SdCardIcon)
```

**Boundary rule:** All product code imports icons from `src/renderer/icons` (the barrel) or
`src/renderer/icons/lucide` (the adapter). Direct `@lucide/vue` imports outside `lucide.ts` and `types.ts` (type-only)
are violations.

**Current state:** `presentation.ts` imports from `../../icons/lucide` directly instead of the barrel. This is a
consistency concern but still within the adapter boundary.

### 1.2 Target architecture

The migration replaces `lucide.ts` with a Dekzer semantic icon registry. This is not a Fluent adapter replacing a
Lucide adapter. It is a product-owned registry that happens to be backed by vendored Fluent SVGs as source material.

Product code does not gain a Fluent dependency. Product code does not gain knowledge of Fluent names, Fluent package
identifiers, or icon-library internals. The registry is the only place that knows which vendored SVG asset backs
which semantic role.

```
src/renderer/icons/
  index.ts          barrel: re-exports roles, types, and the Icon component
  types.ts          IconRole, IconComponent, IconSize, IconTone, IconFrame, IconProps
  tokens.ts         CSS class maps for size/tone (unchanged)
  icon.vue          Icon wrapper component (unchanged interface)
  registry.ts       Dekzer semantic registry: canonical role → vendored SVG asset
  vendor/           vendored Fluent SVG sources (selected subset only)
    *.svg
```

`lucide.ts` is removed. `custom.ts` is absorbed into `registry.ts` or maintained alongside it for product-specific
drawings not covered by vendored Fluent sources.

**Registry responsibilities** (per local-source-icon-doctrine):

- Canonical name to asset mapping
- Size-specific asset selection (16px and 20px as separate targets)
- Badge overlay mapping
- Foreground token behavior
- Selected, muted, disabled, pending, and active state rendering

The registry does not decide source state, readiness, or row kind. Those are substrate and projection
responsibilities.

**Updated boundary rule:** All product code imports semantic role identifiers from `src/renderer/icons` (the
barrel). No product code imports Fluent names, Lucide names, or raw SVG paths. The registry is the only place those
implementation details live.

---

## 2. Icon Export Inventory

### 2.1 Exported Icons (Complete)

| #   | Export Name            | Lucide Source    | Custom  | Used?   | Usage Files                                   |
| --- | ---------------------- | ---------------- | ------- | ------- | --------------------------------------------- |
| 1   | `Icon` (component)     | —                | —       | Yes     | `treeRow.vue`, `table.vue`, `panel.vue`       |
| 2   | `ArchiveIcon`          | `Archive`        | No      | No      | —                                             |
| 3   | `CircleCheckIcon`      | `CircleCheck`    | No      | No      | —                                             |
| 4   | `CircleDotIcon`        | `CircleDot`      | No      | No      | —                                             |
| 5   | `CircleIcon`           | `Circle`         | No      | No      | —                                             |
| 6   | `CircleOffIcon`        | `CircleOff`      | No      | No      | —                                             |
| 7   | `CircleXIcon`          | `CircleX`        | No      | **Yes** | `panel.vue` (remove source button)            |
| 8   | `CloudIcon`            | `Cloud`          | No      | No      | —                                             |
| 9   | `CloudSyncIcon`        | `CloudSync`      | No      | No      | —                                             |
| 10  | `DatabaseIcon`         | `Database`       | No      | No      | —                                             |
| 11  | `DisclosureOpenIcon`   | `ChevronDown`    | No      | **Yes** | `treeRow.vue` (expand affordance)             |
| 12  | `DisclosureClosedIcon` | `ChevronRight`   | No      | **Yes** | `treeRow.vue` (expand affordance)             |
| 13  | `DiscIcon`             | `Disc`           | No      | No      | —                                             |
| 14  | `FileIcon`             | `File`           | No      | No      | —                                             |
| 15  | `FileTextIcon`         | `FileText`       | No      | **Yes** | `table.vue` (cueSheet, metadata icons)        |
| 16  | `FolderIcon`           | `Folder`         | No      | **Yes** | `presentation.ts`, `table.vue`                |
| 17  | `FolderOpenIcon`       | `FolderOpen`     | No      | **Yes** | `presentation.ts`, `table.vue`                |
| 18  | `FolderPlusIcon`       | `FolderPlus`     | No      | No      | —                                             |
| 19  | `HardDriveIcon`        | `HardDrive`      | No      | No      | —                                             |
| 20  | `ImageIcon`            | `Image`          | No      | **Yes** | `table.vue` (image file icon)                 |
| 21  | `ListMusicIcon`        | `ListMusic`      | No      | **Yes** | `presentation.ts`, `table.vue`                |
| 22  | `LoadingIcon`          | `Loader2`        | No      | **Yes** | `presentation.ts`, `table.vue`                |
| 23  | `LockIcon`             | `Lock`           | No      | No      | —                                             |
| 24  | `MoreIcon`             | `MoreHorizontal` | No      | **Yes** | `presentation.ts`, `table.vue`                |
| 25  | `MusicIcon`            | `Music`          | No      | **Yes** | `table.vue` (audio file icon)                 |
| 26  | `NavigationIcon`       | `Compass`        | No      | **Yes** | `presentation.ts` (collectionView, smartView) |
| 27  | `NetworkIcon`          | `Network`        | No      | No      | —                                             |
| 28  | `PinIcon`              | `Pin`            | No      | No      | —                                             |
| 29  | `PinOffIcon`           | `PinOff`         | No      | No      | —                                             |
| 30  | `RefreshCwIcon`        | `RefreshCw`      | No      | No      | —                                             |
| 31  | `ScanIcon`             | `ScanLine`       | No      | **Yes** | `panel.vue` (scan button)                     |
| 32  | `ServerIcon`           | `Server`         | No      | No      | —                                             |
| 33  | `SmartphoneIcon`       | `Smartphone`     | No      | No      | —                                             |
| 34  | `SourceIcon`           | `Library`        | No      | **Yes** | `presentation.ts` (source role)               |
| 35  | `StateIcon`            | `Info`           | No      | **Yes** | `presentation.ts`, `table.vue`                |
| 36  | `TriangleAlertIcon`    | `TriangleAlert`  | No      | No      | —                                             |
| 37  | `UsbIcon`              | `Usb`            | No      | No      | —                                             |
| 38  | `VideoIcon`            | `Video`          | No      | **Yes** | `table.vue` (video file icon)                 |
| 39  | `WarningIcon`          | `AlertCircle`    | No      | **Yes** | `presentation.ts`, `table.vue`                |
| 40  | `SdCardIcon`           | —                | **Yes** | No      | —                                             |

### 2.2 Exported Types

| Export          | Used? | Usage                                         |
| --------------- | ----- | --------------------------------------------- |
| `IconComponent` | Yes   | `presentation.ts`, `treeRow.vue`, `table.vue` |
| `IconSize`      | Yes   | `tokens.ts` (internal), `icon.vue`            |
| `IconTone`      | Yes   | `treeRow.vue`, `table.vue`                    |
| `IconFrame`     | No    | (defined but not consumed outside `icon.vue`) |
| `IconProps`     | Yes   | `icon.vue`                                    |

---

## 3. Icon Usage Site Map

### 3.1 `library/tree/treeRow.vue`

**Imports:** `DisclosureClosedIcon`, `DisclosureOpenIcon`, `Icon`

| Site                        | Icon                                          | Semantic Meaning                             |
| --------------------------- | --------------------------------------------- | -------------------------------------------- |
| Disclosure button (line 98) | `DisclosureOpenIcon` / `DisclosureClosedIcon` | Expand/collapse state of tree node           |
| Row icon slot (line 103)    | resolved via `resolveBrowserTreeRowIcon()`    | Node-type icon (source, folder, state, etc.) |

`resolveBrowserTreeRowIcon` delegates to `presentation.ts`.

### 3.2 `library/tree/presentation.ts`

**Imports from `../../icons/lucide`:** `FolderIcon`, `FolderOpenIcon`, `ListMusicIcon`, `LoadingIcon`, `MoreIcon`,
`NavigationIcon`, `SourceIcon`, `StateIcon`, `WarningIcon`

| Tree Role            | Icon                                        | Condition                    |
| -------------------- | ------------------------------------------- | ---------------------------- |
| `collectionView`     | `NavigationIcon`                            | always                       |
| `source`             | `SourceIcon`                                | always                       |
| `sourceLocation`     | `FolderOpenIcon` / `FolderIcon`             | `isExpanded` toggle          |
| `literalDirectory`   | `FolderOpenIcon` / `FolderIcon`             | `isExpanded` toggle          |
| `preparationSurface` | `StateIcon`                                 | always                       |
| `playlistSurface`    | `ListMusicIcon`                             | always                       |
| `smartView`          | `NavigationIcon`                            | always                       |
| `state`              | `LoadingIcon` / `WarningIcon` / `StateIcon` | depends on `node.icon` value |
| `action`             | `LoadingIcon` / `WarningIcon` / `MoreIcon`  | depends on `node.icon` value |

**Critical callout: Folder expansion mapping.** `sourceLocation` and `literalDirectory` currently swap between
`FolderIcon` (collapsed) and `FolderOpenIcon` (expanded). This couples expansion disclosure to the folder icon
shape. **The long-term contract is that the chevron (`disclosure.open`/`disclosure.closed`) owns expansion state
and the folder icon is always `folder.plain` regardless of expansion.** This split is already partially realized:
the chevron does render independently in the disclosure button slot. The folder shape swap is a visual duplicate of
information already conveyed by the chevron.

### 3.3 `library/contents/table.vue`

**Imports:** `FileTextIcon`, `FolderIcon`, `FolderOpenIcon`, `Icon`, `ImageIcon`, `ListMusicIcon`, `LoadingIcon`,
`MoreIcon`, `MusicIcon`, `StateIcon`, `VideoIcon`, `WarningIcon`

| Content Row Icon Key | Resolved Icon   | Semantic Meaning           |
| -------------------- | --------------- | -------------------------- |
| `folder`             | `FolderIcon`    | Directory in contents list |
| `music`              | `MusicIcon`     | Audio file                 |
| `video`              | `VideoIcon`     | Video file                 |
| `image`              | `ImageIcon`     | Image file                 |
| `cueSheet`           | `FileTextIcon`  | Cue sheet file             |
| `playlist`           | `ListMusicIcon` | Playlist                   |
| `metadata`           | `FileTextIcon`  | Metadata/unsupported file  |
| `more`               | `MoreIcon`      | Load-more row              |
| `loading`            | `LoadingIcon`   | Loading state              |
| `warning`            | `WarningIcon`   | Warning/error state        |
| `state`              | `StateIcon`     | Generic state              |

**Action icon:** `FolderOpenIcon` for `loadChildren` action, otherwise `MoreIcon`.

### 3.4 `library/panel.vue`

**Imports:** `CircleXIcon`, `Icon`, `ScanIcon`

| Site                            | Icon          | Semantic Meaning            |
| ------------------------------- | ------------- | --------------------------- |
| Scan button (line 615)          | `ScanIcon`    | Initiate media scan on root |
| Remove source button (line 627) | `CircleXIcon` | Remove a registered source  |

---

## 4. Direct `@lucide/vue` Import Audit

**Findings: No violations.** All `@lucide/vue` imports are confined to:

- `icons/lucide.ts` — the designated adapter file (31 import lines)
- `icons/types.ts` — type-only import of `LucideIcon`

No product component imports from `@lucide/vue` directly.

**Near-issue:** `library/tree/presentation.ts` imports from `../../icons/lucide` instead of `../../icons` (the
barrel). Both go through the adapter, but the barrel is the preferred import surface for product code.

---

## 5. Semantic Icon Role Namespace

A flat namespace of stable product-facing role identifiers. Product code requests roles; the registry resolves
roles to vendored SVG assets. Each role encodes _what the icon communicates_, not which library drew the shape.
Role names do not change when the underlying asset changes.

### 5.1 Disclosure

| Role                | Context                     |
| ------------------- | --------------------------- |
| `disclosure.closed` | Tree/branch can be expanded |
| `disclosure.open`   | Tree/branch is expanded     |

### 5.2 Folder

| Role                  | Context                                  |
| --------------------- | ---------------------------------------- |
| `folder.plain`        | Generic directory (no known media facet) |
| `folder.audioFacet`   | Directory known to contain audio         |
| `folder.videoFacet`   | Directory known to contain video         |
| `folder.mixedFacet`   | Directory known to contain audio + video |
| `folder.systemMusic`  | OS-recognized Music folder               |
| `folder.systemVideos` | OS-recognized Videos folder              |

**Constraint:** Folder roles MUST NOT encode expansion state. Expansion is owned by `disclosure.*`.

### 5.3 Source

| Role             | Context                             |
| ---------------- | ----------------------------------- |
| `source.local`   | Local library/root source           |
| `source.drive`   | Drive/volume (internal or external) |
| `source.service` | Network/cloud service source        |

### 5.4 Media

| Role             | Context                   |
| ---------------- | ------------------------- |
| `media.audio`    | Audio file                |
| `media.video`    | Video file                |
| `media.image`    | Image file                |
| `media.cueSheet` | Cue sheet file            |
| `media.playlist` | Playlist                  |
| `media.metadata` | Metadata/unsupported file |
| `media.document` | Generic file/document     |

### 5.5 State

| Role            | Context                     |
| --------------- | --------------------------- |
| `state.loading` | Content or node is loading  |
| `state.warning` | Warning or error state      |
| `state.unknown` | Generic/informational state |
| `state.empty`   | Empty result or no content  |

### 5.6 Action

| Role              | Context                  |
| ----------------- | ------------------------ |
| `action.more`     | More items/load more     |
| `action.scan`     | Initiate scan            |
| `action.remove`   | Remove/delete            |
| `action.refresh`  | Refresh/reload           |
| `action.navigate` | Navigate to view/section |

### 5.7 Navigation

| Role                    | Context                 |
| ----------------------- | ----------------------- |
| `navigation.collection` | Collection view         |
| `navigation.view`       | Smart view / saved view |

---

## 6. Migration Map: Current Export → Registry Role

| Current Export         | Proposed Role                       | Action      | Notes                                                                      |
| ---------------------- | ----------------------------------- | ----------- | -------------------------------------------------------------------------- |
| `DisclosureOpenIcon`   | `disclosure.open`                   | **Replace** | Vendor Fluent chevron SVG; map to role in registry                         |
| `DisclosureClosedIcon` | `disclosure.closed`                 | **Replace** | Vendor Fluent chevron SVG; map to role in registry                         |
| `FolderIcon`           | `folder.plain`                      | **Replace** | Stop toggling shape on expansion; see Section 7                            |
| `ListMusicIcon`        | `media.playlist`                    | **Replace** |                                                                            |
| `LoadingIcon`          | `state.loading`                     | **Replace** |                                                                            |
| `MoreIcon`             | `action.more`                       | **Replace** |                                                                            |
| `NavigationIcon`       | `navigation.collection`             | **Replace** |                                                                            |
| `SourceIcon`           | `source.local`                      | **Replace** |                                                                            |
| `StateIcon`            | `state.unknown`                     | **Replace** |                                                                            |
| `WarningIcon`          | `state.warning`                     | **Replace** |                                                                            |
| `FileTextIcon`         | `media.cueSheet` / `media.metadata` | **Replace** | May bifurcate into two roles later; one vendored SVG can back both         |
| `ImageIcon`            | `media.image`                       | **Replace** |                                                                            |
| `MusicIcon`            | `media.audio`                       | **Replace** |                                                                            |
| `VideoIcon`            | `media.video`                       | **Replace** |                                                                            |
| `CircleXIcon`          | `action.remove`                     | **Replace** |                                                                            |
| `ScanIcon`             | `action.scan`                       | **Replace** |                                                                            |
| `FolderOpenIcon`       | —                                   | **Delete**  | Expansion-coupled; superseded by `disclosure.open`; no registry equivalent |
| `TriangleAlertIcon`    | —                                   | **Delete**  | Duplicate of `WarningIcon`; no consumer                                    |
| `ArchiveIcon`          | —                                   | **Delete**  | No consumer; vendor fresh SVG when archive role is defined                 |
| `CircleCheckIcon`      | —                                   | **Delete**  | No consumer                                                                |
| `CircleDotIcon`        | —                                   | **Delete**  | No consumer                                                                |
| `CircleIcon`           | —                                   | **Delete**  | No consumer                                                                |
| `CircleOffIcon`        | —                                   | **Delete**  | No consumer                                                                |
| `CloudIcon`            | —                                   | **Delete**  | No consumer; `volume.cloud` / `source.service` cover when needed           |
| `CloudSyncIcon`        | —                                   | **Delete**  | No consumer                                                                |
| `DatabaseIcon`         | —                                   | **Delete**  | No consumer                                                                |
| `DiscIcon`             | —                                   | **Delete**  | No consumer; disc role not yet in namespace                                |
| `FileIcon`             | —                                   | **Delete**  | No consumer                                                                |
| `FolderPlusIcon`       | —                                   | **Delete**  | No consumer                                                                |
| `HardDriveIcon`        | `source.drive`                      | **Delete**  | No consumer; role defined; vendor fresh SVG when feature lands             |
| `LockIcon`             | `badge.lock`                        | **Delete**  | No consumer; role defined in doctrine; vendor when permission badges land  |
| `NetworkIcon`          | `source.service`                    | **Delete**  | No consumer; role defined; vendor fresh SVG when feature lands             |
| `PinIcon`              | —                                   | **Delete**  | No consumer                                                                |
| `PinOffIcon`           | —                                   | **Delete**  | No consumer                                                                |
| `RefreshCwIcon`        | `action.refresh`                    | **Delete**  | No consumer; role defined; vendor fresh SVG when feature lands             |
| `ServerIcon`           | —                                   | **Delete**  | No consumer                                                                |
| `SmartphoneIcon`       | —                                   | **Delete**  | No consumer                                                                |
| `UsbIcon`              | —                                   | **Delete**  | No consumer; drive/badge roles cover when needed                           |
| `SdCardIcon`           | —                                   | **Delete**  | No consumer; not a Lucide import (`custom.ts`); deleted in cleanup         |

---

## 7. Tree Folder Behavior — Explicit Callout

### Current behavior

```
sourceLocation   → isExpanded ? FolderOpenIcon : FolderIcon
literalDirectory → isExpanded ? FolderOpenIcon : FolderIcon
```

The folder shape changes between closed and open based on the node expansion state. The disclosure chevron
(`DisclosureOpenIcon`/`DisclosureClosedIcon`) also renders independently in the button slot.

### Target behavior

The chevron (`disclosure.open` / `disclosure.closed`) is the sole visual indicator of expansion. The folder icon
is always `folder.plain` (or a faceted variant like `folder.audioFacet` when the directory's media composition is
known) and does not change shape on expansion.

**Migration step:** In `presentation.ts`, change the `sourceLocation` and `literalDirectory` cases to always return
`folder.plain`. In `table.vue`, change the `loadChildren` action icon from `FolderOpenIcon` to an appropriate
non-folder-open role. `FolderOpenIcon` is removed from the registry with no equivalent.

### Facet differentiation

The `ChildRow` type (from `shared/library/hierarchy/read.ts`) already carries `directoryPrimaryMediaState`,
`directoryImageMediaState`, and `directoryScanState`. These fields can drive facet selection on directory nodes in
a future pass:

- `directoryPrimaryMediaState` → `folder.audioFacet` / `folder.videoFacet` / `folder.mixedFacet`

This is **not implemented** in this pass — it is called out here as the designated data binding for folder facet
icons.

---

## 8. `LocationSourceIcon` Type Alignment

`sourcePresentation.ts` defines a `LocationSourceIcon` union type with 16 values that describe source-kinds. These
currently have **no visual icon mapping** — they are pure semantic labels. The proposed `source.*` roles above map
to a subset:

| `LocationSourceIcon` | Proposed Role                                |
| -------------------- | -------------------------------------------- |
| `library`            | `source.local`                               |
| `folder`             | `folder.plain`                               |
| `drive`              | `source.drive`                               |
| `driveInternal`      | `source.drive` (internal variant)            |
| `driveExternal`      | `source.drive` (external variant)            |
| `usbDrive`           | `source.drive` (USB variant)                 |
| `sdCard`             | `source.drive` (SD variant)                  |
| `networkShare`       | `source.service`                             |
| `nas`                | `source.service`                             |
| `cloud`              | `source.service`                             |
| `cloudMirror`        | `source.service`                             |
| `mobileDevice`       | `source.service`                             |
| `djDevice`           | `source.service`                             |
| `disc`               | (media.disc — not yet in semantic namespace) |
| `archive`            | (future)                                     |
| `missingSource`      | `state.warning`                              |

This mapping is for future use. No source-kind icons are currently rendered.

---

## 9. Violations and Near-Issues

| Severity       | File                              | Issue                                                               |
| -------------- | --------------------------------- | ------------------------------------------------------------------- |
| **Near-issue** | `library/tree/presentation.ts:12` | Imports from `../../icons/lucide` instead of `../../icons` (barrel) |
| **None**       | —                                 | No direct `@lucide/vue` import in product components                |

---

## 10. Acceptance Checklist

- [x] Every exported icon from `src/renderer/icons` is accounted for (40 icons + 1 component + 5 types)
- [x] Every usage site is documented (4 files: `treeRow.vue`, `presentation.ts`, `table.vue`, `panel.vue`)
- [x] Tree folder expansion behavior explicitly called out
- [x] Semantic role namespace defined (disclosure, folder, source, media, state, action, navigation)
- [x] Migration action assigned for every current icon
- [x] No Fluent dependency added; no SVGs copied
- [x] No visual migration performed
- [x] Target registry architecture defined
- [x] Implementation constraints documented
- [x] Sprint scheduling positioned

---

## 11. Unused Exports — Deletion List

The following 23 exports have no consumer in the renderer product code. All are deleted in the icon registry
sprint. The Lucide stubs are not carried forward as placeholders.

`ArchiveIcon`, `CircleCheckIcon`, `CircleDotIcon`, `CircleIcon`, `CircleOffIcon`, `CloudIcon`, `CloudSyncIcon`,
`DatabaseIcon`, `DiscIcon`, `FileIcon`, `FolderOpenIcon`, `FolderPlusIcon`, `HardDriveIcon`, `LockIcon`,
`NetworkIcon`, `PinIcon`, `PinOffIcon`, `RefreshCwIcon`, `ServerIcon`, `SmartphoneIcon`, `TriangleAlertIcon`,
`UsbIcon`, `SdCardIcon`

Where a semantic role is already defined for a deleted export (`source.drive`, `source.service`, `action.refresh`,
`badge.lock`), the role remains in the namespace. A fresh SVG will be vendored when the feature that needs it
lands. The Lucide shape is not the reference point for that future drawing.

`SdCardIcon` is in `custom.ts`, not `lucide.ts`. It has no consumer and is deleted as part of the same cleanup.
`FolderOpenIcon` has consumers but is deleted because the expansion-coupling it encodes is prohibited.

---

## 12. Implementation Constraints

These constraints apply to the icon registry sprint. They are not implementation notes — they are binding
decisions.

### 1. Vendor SVGs into the repo

Selected Fluent SVG sources are committed to `src/renderer/icons/vendor/`. There is no runtime dependency on
`@fluentui/svg-icons` or any other icon package. There is no live Icones lookup. There is no CDN reference. Fluent
names do not appear in product code.

Vendored SVGs are treated as source material for Dekzer-owned drawings. They may be edited for optical weight,
badge anchor points, or size-specific variants.

### 2. Semantic roles before assets

The registry must define the role namespace before SVG assets are bound to it. Product code references roles like
`media.audio`, `action.scan`, `disclosure.closed`. Product code never references `FluentMusicNote24Regular`,
`LucideMusic`, or any icon-library identifier.

### 3. Total Lucide prune — no holdovers

The `lucide.ts` adapter is removed in full. Every export is either replaced by a registry role lookup or deleted.
Unused exports are not carried forward as stubs or placeholders — they are deleted now. When a new role is needed
in the future, vendor a fresh SVG and define the role at that time. The Lucide shape is not the baseline for
that drawing.

`custom.ts` exports with no consumer are deleted in the same pass.

### 4. Stop folder-open shape toggling

The folder glyph does not change on node expansion. `folder.plain` is the stable identity of a generic directory.
Expansion state is owned entirely by `disclosure.open` and `disclosure.closed`. The only permissible folder shape
changes are facet-based (e.g., `folder.audioFacet` when media composition is known), and those must not be driven
by expansion state.

### 5. Preserve customization path

Vendored SVGs are source material, not locked assets. The registry implementation must allow:

- Editing optical weight and stroke for Dekzer's visual density
- Size-specific source files at 16px and 20px (separate design targets per the local-source-icon-doctrine)
- Custom badge anchor points
- Dark-mode behavioral overrides
- Entirely product-owned drawings for roles where no Fluent SVG is suitable

---

## 13. Sprint Scheduling

The icon registry sprint is renderer-edge work. It must not cut ahead of the source → tree → contents → scan →
readiness loop. Premature execution risks re-touching the same files during browser-flow stabilization.

| Step  | Work item                                | Dependency                                     |
| ----- | ---------------------------------------- | ---------------------------------------------- |
| 1     | Scope identity / readiness gate          | —                                              |
| 2     | Activated browser-flow audit             | Step 1                                         |
| 3     | Browser-flow stabilization               | Step 2                                         |
| 4     | Scan invalidation tightening             | Step 3                                         |
| **5** | **Fluent-backed semantic icon registry** | **Step 4**                                     |
| 6     | Explicit source refresh                  | Step 5 (unless refresh becomes urgent earlier) |

The icon sprint is scheduled before larger product surfaces — crates, playlists, CUE, Prepared Rooms. The visual
language layer is foundational to every subsequent renderer surface. Resolving it early prevents drift in later
work.

---

## Files Referenced

| File                                                           | Role                                    |
| -------------------------------------------------------------- | --------------------------------------- |
| `apps/desktop/src/renderer/icons/index.ts`                     | Icon barrel                             |
| `apps/desktop/src/renderer/icons/lucide.ts`                    | Lucide adapter                          |
| `apps/desktop/src/renderer/icons/types.ts`                     | Icon types                              |
| `apps/desktop/src/renderer/icons/icon.vue`                     | Icon wrapper component                  |
| `apps/desktop/src/renderer/icons/custom.ts`                    | Custom SVG icons                        |
| `apps/desktop/src/renderer/icons/tokens.ts`                    | CSS class maps                          |
| `apps/desktop/src/renderer/library/tree/presentation.ts`       | Tree icon resolution                    |
| `apps/desktop/src/renderer/library/tree/treeRow.vue`           | Tree row component                      |
| `apps/desktop/src/renderer/library/tree/types.ts`              | Tree types (`BrowserTreeIcon`)          |
| `apps/desktop/src/renderer/library/tree/projection.ts`         | Tree state-to-node projection           |
| `apps/desktop/src/renderer/library/tree/sourcePresentation.ts` | Source icon types                       |
| `apps/desktop/src/renderer/library/contents/table.vue`         | Contents table component                |
| `apps/desktop/src/renderer/library/contents/projection.ts`     | Contents state-to-row projection        |
| `apps/desktop/src/renderer/library/panel.vue`                  | Library panel                           |
| `apps/desktop/src/shared/library/hierarchy/read.ts`            | `ChildRow` type with media state fields |
