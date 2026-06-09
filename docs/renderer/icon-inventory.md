---
status: candidate
ratification-target: renderer-icon-inventory-v1
revision: inventory-pass-1
doctrine-version: 0.1
last-reviewed: 2026-06-06
owner: renderer-icon-surface
canonical-context:
  - product/product-doctrine
  - local-source-icon-doctrine
scope:
  - renderer-icon-adapter
  - icon-semantic-roles
  - lucide-to-fluent-migration
  - tree-icon-resolution
  - contents-table-icon-resolution
---

# Renderer Icon Inventory

## Purpose

Inventory every current renderer icon export and every usage site. Define the semantic role namespace for a controlled
migration from the Lucide-backed surface to a Fluent-backed semantic surface.

This document is a **design inventory and boundary map**. No visual migration is performed in this pass.

---

## 1. Icon Adapter Architecture

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
`FolderIcon` (collapsed) and `FolderOpenIcon` (expanded). This couples expansion disclosure to the folder icon shape. \*
*The long-term contract is that the chevron (`DisclosureOpenIcon`/`DisclosureClosedIcon`) owns expansion state and the
folder icon is always `folder.plain` regardless of expansion.\*\* This split is already partially realized: the chevron
*does\* render independently in the disclosure button slot. The folder shape swap is a visual duplicate of information
already conveyed by the chevron.

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

**Near-issue:** `library/tree/presentation.ts` imports from `../../icons/lucide` instead of `../../icons` (the barrel).
Both go through the adapter, but the barrel is the preferred import surface for product code.

---

## 5. Semantic Icon Role Namespace (Proposed)

Define a flat namespace of semantic roles. These replace the current Lucide-aliased export names. Each role encodes
_what the icon communicates_, not _which icon pack shape it uses_.

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

## 6. Migration Map: Current Icon → Proposed Fluent Status

| Current Export         | Proposed Role                       | Fluent Status                | Notes                                                             |
| ---------------------- | ----------------------------------- | ---------------------------- | ----------------------------------------------------------------- |
| `DisclosureOpenIcon`   | `disclosure.open`                   | **Replace now**              | Direct Fluent chevron equivalent                                  |
| `DisclosureClosedIcon` | `disclosure.closed`                 | **Replace now**              | Direct Fluent chevron equivalent                                  |
| `FolderIcon`           | `folder.plain`                      | **Replace now**              | Stop toggling shape on expansion                                  |
| `FolderOpenIcon`       | —                                   | **Remove**                   | Superseded by `disclosure.open`; remove expansion-coupled variant |
| `ListMusicIcon`        | `media.playlist`                    | **Replace now**              |                                                                   |
| `LoadingIcon`          | `state.loading`                     | **Replace now**              |                                                                   |
| `MoreIcon`             | `action.more`                       | **Replace now**              |                                                                   |
| `NavigationIcon`       | `navigation.collection`             | **Replace now**              |                                                                   |
| `SourceIcon`           | `source.local`                      | **Replace now**              |                                                                   |
| `StateIcon`            | `state.unknown`                     | **Replace now**              |                                                                   |
| `WarningIcon`          | `state.warning`                     | **Replace now**              |                                                                   |
| `FileTextIcon`         | `media.cueSheet` / `media.metadata` | **Replace now**              | May bifurcate into two roles later                                |
| `ImageIcon`            | `media.image`                       | **Replace now**              |                                                                   |
| `MusicIcon`            | `media.audio`                       | **Replace now**              |                                                                   |
| `VideoIcon`            | `media.video`                       | **Replace now**              |                                                                   |
| `CircleXIcon`          | `action.remove`                     | **Replace now**              |                                                                   |
| `ScanIcon`             | `action.scan`                       | **Replace now**              |                                                                   |
| `ArchiveIcon`          | (future)                            | **Keep temporarily**         | Needed for future source kinds                                    |
| `CircleCheckIcon`      | (future)                            | **Keep temporarily**         |                                                                   |
| `CircleDotIcon`        | (future)                            | **Keep temporarily**         |                                                                   |
| `CircleIcon`           | (future)                            | **Keep temporarily**         |                                                                   |
| `CircleOffIcon`        | (future)                            | **Keep temporarily**         |                                                                   |
| `CloudIcon`            | (future)                            | **Keep temporarily**         | Needed for cloud sources                                          |
| `CloudSyncIcon`        | (future)                            | **Keep temporarily**         |                                                                   |
| `DatabaseIcon`         | (future)                            | **Keep temporarily**         |                                                                   |
| `DiscIcon`             | (future)                            | **Keep temporarily**         | Disc/cd icon                                                      |
| `FileIcon`             | (future)                            | **Keep temporarily**         |                                                                   |
| `FolderPlusIcon`       | (future)                            | **Keep temporarily**         |                                                                   |
| `HardDriveIcon`        | `source.drive`                      | **Keep temporarily**         | Will become `source.drive` when used                              |
| `LockIcon`             | (future)                            | **Keep temporarily**         | Permission/locked badge                                           |
| `NetworkIcon`          | `source.service`                    | **Keep temporarily**         | Will become `source.service` when used                            |
| `PinIcon`              | (future)                            | **Keep temporarily**         |                                                                   |
| `PinOffIcon`           | (future)                            | **Keep temporarily**         |                                                                   |
| `RefreshCwIcon`        | `action.refresh`                    | **Keep temporarily**         |                                                                   |
| `ServerIcon`           | (future)                            | **Keep temporarily**         |                                                                   |
| `SmartphoneIcon`       | (future)                            | **Keep temporarily**         |                                                                   |
| `TriangleAlertIcon`    | `state.warning`                     | **Remove**                   | Superseded by `WarningIcon` → `state.warning`                     |
| `UsbIcon`              | (future)                            | **Keep temporarily**         | USB drive badge/icon                                              |
| `SdCardIcon`           | (future)                            | **Custom derivative needed** | Custom SVG; Fluent equivalent audit needed                        |

---

## 7. Tree Folder Behavior — Explicit Callout

### Current behavior

```
sourceLocation  → isExpanded ? FolderOpenIcon : FolderIcon
literalDirectory → isExpanded ? FolderOpenIcon : FolderIcon
```

The folder shape changes between closed and open based on the node expansion state. The disclosure chevron (
`DisclosureOpenIcon`/`DisclosureClosedIcon`) also renders independently in the button slot.

### Target behavior

The chevron (`disclosure.open` / `disclosure.closed`) is the sole visual indicator of expansion. The folder icon is
always `folder.plain` (or a faceted variant like `folder.audioFacet` when the directory's media composition is known)
and does not change shape on expansion.

**Migration step:** In `presentation.ts`, change the `sourceLocation` and `literalDirectory` cases to always return
`FolderIcon` (mapped to `folder.plain`). The `FolderOpenIcon` export can then be removed from the adapter.

### Facet differentiation

The `ChildRow` type (from `shared/libraryHierarchy/readChildren.ts`) already carries `directoryPrimaryMediaState`,
`directoryImageMediaState`, and `directoryScanState`. These fields can drive facet selection on directory nodes in a
future pass:

- `directoryPrimaryMediaState` → `folder.audioFacet` / `folder.videoFacet` / `folder.mixedFacet`

This is **not implemented** in this pass — it is called out here as the designated data binding for folder facet icons.

---

## 8. `LocationSourceIcon` Type Alignment

`sourcePresentation.ts` defines a `LocationSourceIcon` union type with 16 values that describe source-kinds. These
currently have **no visual icon mapping** — they are pure semantic labels. The proposed `source.*` roles above map to a
subset:

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
- [x] Migration status assigned for every current icon
- [x] No Fluent dependency added; no SVGs copied
- [x] No visual migration performed

---

## 11. Unused Exports — Summary

The following 22 exports have no consumer in the renderer product code. They are retained in the adapter for future
features or may be removed in the Fluence migration pass:

`ArchiveIcon`, `CircleCheckIcon`, `CircleDotIcon`, `CircleIcon`, `CircleOffIcon`, `CloudIcon`, `CloudSyncIcon`,
`DatabaseIcon`, `DiscIcon`, `FileIcon`, `FolderPlusIcon`, `HardDriveIcon`, `LockIcon`, `NetworkIcon`, `PinIcon`,
`PinOffIcon`, `RefreshCwIcon`, `ServerIcon`, `SmartphoneIcon`, `TriangleAlertIcon`, `UsbIcon`, `SdCardIcon`

`TriangleAlertIcon` duplicates `WarningIcon` (both are alert/warning shapes) — it should be removed.

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
| `apps/desktop/src/shared/libraryHierarchy/readChildren.ts`     | `ChildRow` type with media state fields |
