---
status: candidate
ratification-target: local-source-icon-doctrine-v1
revision: ratification-candidate-pass-1
doctrine-version: 0.3
last-reviewed: 2026-05-27
owner: library-browser-architecture
canonical-context:
  - product/product-doctrine
  - source-root-scan-admission-contract
  - first-slice-substrate-map
scope:
  - local-source-iconography
  - source-browser-presentation
  - source-root-state-projection
  - icon-registry-contract
  - renderer-library-browser
---

# Local Source Icon Doctrine

## Governing rule

**Native meaning, product-owned drawing.**

The operating system lends semantic recognition: Desktop, Downloads, Music, folders, drives, removable volumes,
permissions, and paths.

Dekzer owns the glyph language: shape, stroke, optical density, badge system, selected state, muted state, disabled
state, dark-mode behavior, and icon role mapping.

This is not a compromise. It is the product position.

## Purpose

Dekzer must let users recognize local filesystem places instantly without allowing operating system iconography to
define the product’s music system.

The browser should feel familiar enough to navigate immediately, but authored enough to belong to Dekzer.

This document defines the icon taxonomy, badge composition law, source-state projection contract, substrate boundary,
and first implementation constraints for local-source iconography.

## Canonical context note

This doctrine depends on the source root scan admission contract for root classification, registration proposal state,
source identity, relocation observations, cloud-backed classification, and scan-state vocabulary.

The icon doctrine does not redefine those substrate observations. It defines how renderer rows project those observations into
Dekzer-owned icon roles, badges, and row treatments.

## Core principle

When Dekzer is showing the user’s computer, the user should immediately understand the place being shown.

When Dekzer is showing the user’s music system, the visual language must be Dekzer-owned.

The browser must not feel like Windows Explorer, Finder, or a generic file picker embedded inside a DJ app.

## Recognition boundary

| Context                                                            | Icon authority                                                 |
| ------------------------------------------------------------------ | -------------------------------------------------------------- |
| Native folder picker                                               | Operating system                                               |
| File open/save dialogs                                             | Operating system                                               |
| Source registration and relocation workflows                       | Dekzer-authored icons, with native meaning                     |
| Main source tree                                                   | Dekzer-authored icons                                          |
| Main library browse surface                                        | Dekzer-authored product icons                                  |
| Raw diagnostic file mode                                           | Native icons may be allowed behind an explicit diagnostic mode |
| Prepared room, crates, sleeves, routes, history, analysis surfaces | Dekzer-authored product icons only                             |

Native icons are allowed at operating-system interaction boundaries. They are not allowed to define the everyday Dekzer
browser language.

## Vetoes

### No OS file-association icons in the normal library browse surface

A Windows `.mp3` icon, Finder file icon, or user-default app icon is not a Dekzer track identity.

A row representing a canonical track uses a Dekzer track glyph. Format belongs in metadata, not in the primary object
glyph.

### No persisted icon blobs in SQLite

The durable substrate stores role, kind, state, identity, classification, readiness, and source relationships.

It does not store rendered SVGs, PNGs, native icon blobs, CSS classes, or platform-specific icon assets.

### No native icon provider as product authority

Native icon APIs may support diagnostics or OS-bound workflows. They do not decide the canonical visual identity of
Dekzer browse rows.

### No platform cosplay

Windows must not look like Explorer. macOS must not look like Finder.

Dekzer should be native enough to orient the user and authored enough to be its own instrument.

### No icon zoo

The system must avoid one-off icons for every possible state.

Use base icons plus a small, consistent badge system.

## Data model boundary

Rows expose stable icon roles and row states. They do not expose rendered artwork as durable identity.

A row projection may include:

| Field class      | Meaning                                                                                                                                                                           |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Icon base role   | Stable canonical base role resolved by the icon registry                                                                                                                          |
| Row kind         | Place, volume, source root, hierarchy node, media candidate, product object                                                                                                       |
| Source row state | Proposed, registered, scanning, missing, permission required, offline                                                                                                             |
| Applicable badge | The single highest-priority badge selected by the composition law                                                                                                                 |
| Root class       | Normal music root, broad drive root, system volume root, user profile root, external volume root, network root, cloud-backed root, protected root, indirection root, unknown root |
| Media role       | Audio candidate, video candidate, companion metadata, sleeve candidate                                                                                                            |
| Readiness state  | Ready, degraded, pending, blocked, unavailable, unsupported, failed                                                                                                               |
| File format      | MP3, WAV, FLAC, AIFF, M4A, CUE, image format, etc.                                                                                                                                |
| Platform label   | User-facing OS label such as Desktop, Downloads, Macintosh HD, Local Disk (C:)                                                                                                    |

The renderer resolves icon base role, source row state, applicable badge, size, and row visual state through the Dekzer
icon registry.

The durable substrate never stores the drawing.

## Canonical icon naming

Icon names are stable role identifiers. They are not descriptions of the current drawing.

The drawing may evolve. The canonical name must remain stable unless the role changes.

Use prefix-dot-role naming.

## Base icon roles

Base icon roles identify the underlying object or place before state badges are applied.

### Places

| Canonical name    | Current drawing intent               |
| ----------------- | ------------------------------------ |
| `place.desktop`   | Small monitor or workspace rectangle |
| `place.downloads` | Arrow entering tray or folder        |
| `place.music`     | Folder with subtle waveform mark     |
| `place.documents` | Document stack or page               |
| `place.home`      | Home or user-location mark           |
| `place.folder`    | Generic folder                       |

The Music folder remains a filesystem place. It must not look like a Dekzer crate, prepared room, or canonical
collection object.

### Volumes

| Canonical name    | Current drawing intent               |
| ----------------- | ------------------------------------ |
| `volume.internal` | Drive slab                           |
| `volume.system`   | Drive slab with subtle system marker |
| `volume.external` | Detachable drive glyph               |
| `volume.network`  | Drive with network node mark         |
| `volume.cloud`    | Cloud-backed volume or folder root   |

`volume.cloud` applies only when the source root classification is `cloud_backed_root`.

Path heuristics, folder names, and vendor-looking labels are not sufficient.

Volume labels remain platform-specific. The icon roles remain product-owned.

### Media and file roles

| Canonical name          | Current drawing intent                         |
| ----------------------- | ---------------------------------------------- |
| `media.audio`           | Audio file or audio candidate glyph            |
| `media.video`           | Video file or video candidate glyph            |
| `media.companion`       | Companion metadata file, such as `.cue`        |
| `media.sleeveCandidate` | Image or sleeve candidate glyph                |
| `media.unsupported`     | Media candidate known to be unsupported        |
| `media.rejected`        | Candidate rejected by classification or policy |

Default browsing should show media-relevant entries. Raw unrelated files belong in an explicit diagnostic or details
mode.

### Product object roles

Product objects never use OS icons.

| Canonical name            | Meaning                                                    |
| ------------------------- | ---------------------------------------------------------- |
| `product.track`           | Canonical track object                                     |
| `product.crate`           | Manual crate                                               |
| `product.smartList`       | Query-backed or rule-backed list                           |
| `product.sleeve`          | Sleeve object                                              |
| `product.preparedRoom`    | Prepared room surface                                      |
| `product.route`           | Route or live path                                         |
| `product.history`         | History surface                                            |
| `product.analysisSurface` | Surface, tab, or panel for analysis results and inspection |

`product.analysisSurface` is not a per-file analysis artifact and not a readiness status indicator.

Readiness states are expressed through readiness columns, facets, or row-level treatment, not by replacing the object
glyph.

## Source row states

Source canonical names are row states, not standalone icon assets.

The icon registry composes source presentation as:

1. Resolve the root-class base icon.
2. Apply the source row state.
3. Select the applicable badge by the badge composition law.
4. Render the result at the requested size and row visual state.

Most source states are therefore compositions, not separate glyphs.

| Source row state            | Actual icon composition                                                    |
| --------------------------- | -------------------------------------------------------------------------- |
| `source.registered`         | Root-class base + `badge.source`                                           |
| `source.scanning`           | Root-class base + `badge.scanActive` when no higher-priority badge applies |
| `source.missing`            | Root-class base + `badge.warning`                                          |
| `source.permissionRequired` | Root-class base + `badge.lock`                                             |
| `source.offline`            | Root-class base + `badge.offline`                                          |
| `source.proposed`           | Root-class base + pending treatment, with no badge                         |

`source.relocatable` is not a primary source row state. Relocation is represented by `badge.locate` when it is the
selected icon badge, or by an adjacent row action when a higher-priority status badge wins.

This rule prevents implementers from creating separate source glyph assets for every source state.

## Root class base resolution

The source icon must preserve what kind of root was registered or proposed.

A source rooted at a normal music folder uses a folder-like base.

A source rooted at a drive or broad volume uses a volume-like base.

| Root class             | Base icon                                                                      |
| ---------------------- | ------------------------------------------------------------------------------ |
| `normal_music_root`    | `place.folder` or a specific place role such as `place.music`                  |
| `system_volume_root`   | `volume.system`                                                                |
| `broad_drive_root`     | `volume.internal` or `volume.external`, depending on the volume                |
| `user_profile_root`    | `place.home`                                                                   |
| `external_volume_root` | `volume.external`                                                              |
| `network_root`         | `volume.network`                                                               |
| `cloud_backed_root`    | `volume.cloud`                                                                 |
| `protected_root`       | Root-class base, then permission or protection state                           |
| `indirection_root`     | Resolved target base, with indirection disclosure outside the icon when needed |
| `unknown_root`         | Conservative base plus `badge.unknown` where applicable                        |

The source badge applies to the base icon after root class selection.

This lets users distinguish a registered Music folder from a registered broad drive root without relying only on the
label.

## Registration proposal state

A registration proposal is not an ordinary folder and not a registered source root.

For durable proposal states such as `PlanProposed`, use the same base glyph that would be used after confirmation, but
apply the pending treatment.

### Required first treatment

The first implementation uses:

| Property            | Treatment                                                                          |
| ------------------- | ---------------------------------------------------------------------------------- |
| Base glyph          | Resolved from root class                                                           |
| Foreground          | Secondary foreground token                                                         |
| Badge               | None                                                                               |
| Size                | Same size as normal row icon                                                       |
| Row label           | Normal label, with proposal action or secondary text outside the icon where needed |
| Accessibility label | Includes pending source proposal state                                             |

This signals “not yet committed” without implying structural failure, permission denial, or unavailability.

A registered source shows `badge.source` unless a higher-priority badge applies.

A proposed source never shows `badge.source`.

If proposed sources appear inline with ordinary hierarchy rows, the row text or available action must make the proposal
state explicit. The icon cannot carry that disambiguation alone.

## Badge system

Badges communicate state on top of a base icon. They prevent combinatorial icon growth and keep the language learnable.

### Badge canon

| Badge               | Meaning                                                                                  |
| ------------------- | ---------------------------------------------------------------------------------------- |
| `badge.source`      | Registered source root                                                                   |
| `badge.scanActive`  | Scan active                                                                              |
| `badge.warning`     | Missing, unreachable, lost identity, failed structural state, or urgent structural issue |
| `badge.lock`        | Permission denied or permission required                                                 |
| `badge.offline`     | Removable or external volume unavailable but not lost                                    |
| `badge.unknown`     | Unknown or unclassified                                                                  |
| `badge.unsupported` | Known media or candidate type that Dekzer cannot use                                     |
| `badge.locate`      | Relocation action available                                                              |

### Composition law

At most one badge appears on an icon.

When multiple states apply, the highest-priority badge wins.

| Priority | Badge               | Condition                                            |
| -------- | ------------------- | ---------------------------------------------------- |
| 1        | `badge.warning`     | Missing, unreachable, lost identity                  |
| 2        | `badge.lock`        | Permission denied or required                        |
| 3        | `badge.offline`     | Removable volume unavailable but not lost            |
| 4        | `badge.unknown`     | Unknown or unclassified                              |
| 5        | `badge.unsupported` | Known but unsupported media or candidate             |
| 6        | `badge.locate`      | Relocation action available                          |
| 7        | `badge.scanActive`  | Scan active                                          |
| 8        | `badge.source`      | Registered source root with no higher-priority state |

`badge.warning` outranks every other badge because missing identity is more urgent than activity or base registration.

`badge.unknown` outranks `badge.unsupported` because unclassified material is actionably uncertain, while unsupported
material has already been identified and rejected from normal use.

`badge.source` is the base registered-source state. It appears only when no more urgent state applies.

`badge.scanActive` appears only on the specific source being scanned and only when no higher-priority badge applies. A
source that is both scanning and permission-blocked shows `badge.lock`, not the scan badge.

### Action semantics

Most badges are informational. `badge.locate` is actionable.

When `badge.locate` is displayed, clicking the badge or its associated row action opens the source relocation flow for
that source. The relocation flow must be explicit. The badge must not imply relocation is automatic.

When a higher-priority badge wins, relocation remains available as a row action if the source has a relocation
observation or locate action. The icon badge priority must not hide the user's recovery path.

## Readiness states and icon badges

Readiness states are not an open-ended source of icon badges.

The badge vocabulary covers source, classification, and high-salience structural state. Readiness beyond that is
expressed through row-level color treatment, columns, facets, or status text.

| Readiness state | Icon badge behavior                                                                                   |
| --------------- | ----------------------------------------------------------------------------------------------------- |
| `ready`         | No readiness badge by default                                                                         |
| `degraded`      | Row-level treatment or readiness column, not an icon badge                                            |
| `pending`       | Row-level treatment or readiness column, not `badge.scanActive` unless scan is actually active        |
| `blocked`       | May map to `badge.lock` only when the block is permission/access related                              |
| `unavailable`   | May map to `badge.offline`, `badge.lock`, or `badge.warning` only when the source condition matches   |
| `unsupported`   | May map to `badge.unsupported` for media or candidate-result entries                                  |
| `failed`        | May map to `badge.warning` only for structural or source-level failure; otherwise row-level treatment |

Do not create new icon badges just because a readiness state exists.

## Empty and non-media territory

Muted is the default. Hidden is only appropriate when the user explicitly filters to music-relevant nodes.

The browser must not silently collapse empty territory.

| Case                                                | Default treatment                                      |
| --------------------------------------------------- | ------------------------------------------------------ |
| Folder exists but has no media-relevant descendants | Show as muted                                          |
| Folder was skipped by policy                        | Show as muted or explainable, depending on browse mode |
| Folder is unavailable due to permissions            | Show with permission state                             |
| User enables a music-only filter                    | Hidden is allowed                                      |
| Diagnostic or raw filesystem mode                   | Show raw territory according to that mode’s contract   |

Muted means the icon inherits the row’s secondary or disabled foreground token. It does not mean applying arbitrary
opacity to the primary color.

Opacity alone is too fragile in dense dark UI.

## Visual style

The local-source icon set should be compact, monochrome, and designed for dense professional tooling.

| Property                 | Decision                                                        |
| ------------------------ | --------------------------------------------------------------- |
| Base dense size          | 16px                                                            |
| Larger panel size        | 20px                                                            |
| Style                    | Mostly filled or semi-filled silhouettes with small cut details |
| Stroke                   | Consistent and not hairline                                     |
| Color                    | Inherit from row foreground token                               |
| Selected state           | Row selection controls icon color and contrast                  |
| Muted state              | Secondary or disabled foreground token                          |
| Missing or blocked state | Badge plus appropriate foreground treatment                     |
| Badge position           | Consistent overlay position per icon size                       |
| Platform variance        | Labels and ordering may vary; glyph family remains Dekzer-owned |

## Size-specific drawings

16px and 20px icons require separate design passes.

Do not rely on automated scale-down from 20px to 16px.

Dense browse rows are a primary surface. At 16px, small details such as connector marks, folder tabs, drive boundaries,
and waveform cuts need optical adjustment to remain legible.

The icon registry may expose the same canonical name at both sizes, but the underlying asset may differ.

## Platform behavior

### Windows

Windows labels may include Desktop, Downloads, Music, Documents, Local Disk (C:), removable drive names, and user folder
names.

Dekzer resolves these to canonical roles such as `place.desktop`, `place.downloads`, `place.music`, `volume.system`,
`volume.internal`, or `volume.external`.

The rendered glyphs remain Dekzer-authored.

### macOS

macOS labels may include Desktop, Downloads, Music, Documents, Macintosh HD, external volume names, and network
locations.

Dekzer resolves these to the same canonical roles.

The rendered glyphs do not copy Finder icons.

### Linux later

Linux labels may include Home, Music, mounted volumes, removable media, and network mounts.

Dekzer resolves these through the same icon role system.

## Implementation shape

### Substrate responsibilities

The Rust/library substrate owns:

- Durable source identity
- Root path and platform location observations
- Root classification
- File and folder hierarchy
- Media-relevant observations
- Source registration state
- Availability and permission state
- Readiness and classification outcomes
- Registration proposal durability

It does not own rendered icons.

### Renderer projection responsibilities

The renderer projection owns:

- Row kind
- Canonical base icon role
- Source row state
- Applicable badge
- Muted, selected, disabled, pending, and active visual state
- Whether the current browse mode may hide non-media territory

It does not invent durable source observations.

### Icon registry responsibilities

The icon registry owns:

- Canonical name to asset mapping
- Size-specific asset selection
- Badge overlay mapping
- Foreground token behavior
- Selected, muted, disabled, pending, and active state rendering

It does not decide whether a source is missing, registered, proposed, broad, cloud-backed, or permission-blocked.

## Acceptance bar

An implementation satisfies this doctrine when:

1. Normal source browsing does not use raw operating-system icon assets.
2. Product objects never use file-association icons.
3. The durable substrate stores icon roles and source observations, not rendered icon blobs.
4. Source row states are composed from root-class base icon, source row state, selected badge, and visual state.
5. Badge composition is deterministic and follows the priority table.
6. Proposed source roots use the required pending treatment with no badge.
7. System and broad volume roots use volume base icons after registration.
8. Empty and skipped territory is muted by default, not silently hidden.
9. Readiness states do not expand the badge vocabulary.
10. Canonical icon names remain stable when drawings evolve.
11. 16px and 20px glyphs are treated as separate design targets.
12. `badge.locate` opens an explicit relocation flow or is preserved as a row action when a higher-priority badge wins.

## First minimal icon set

The first implementation should cover these canonical roles.

### Places

- `place.desktop`
- `place.downloads`
- `place.music`
- `place.documents`
- `place.home`
- `place.folder`

### Volumes

- `volume.internal`
- `volume.system`
- `volume.external`
- `volume.network`
- `volume.cloud`

### Source row states and badges

Source row states are not standalone glyph assets, but the renderer and icon registry must recognize them as composition
inputs.

- `source.proposed`
- `source.registered`
- `source.scanning`
- `source.missing`
- `source.permissionRequired`
- `source.offline`
- `badge.source`
- `badge.scanActive`
- `badge.warning`
- `badge.lock`
- `badge.offline`
- `badge.unknown`
- `badge.unsupported`
- `badge.locate`

### Media roles

- `media.audio`
- `media.video`
- `media.companion`
- `media.sleeveCandidate`
- `media.unsupported`
- `media.rejected`

### Product roles

- `product.track`
- `product.crate`
- `product.smartList`
- `product.sleeve`
- `product.preparedRoom`
- `product.route`
- `product.history`
- `product.analysisSurface`

## Final position

Serato proves that users need instant filesystem recognition.

Dekzer should not copy the native icon look.

Dekzer should copy the recognition contract and own the drawing language.

The result is familiar enough to navigate immediately and strict enough to preserve Dekzer’s product model.
