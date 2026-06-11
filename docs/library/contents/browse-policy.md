# Library Contents Browse Policy

**Role:** Implementation companion — current product-filter to implemented backend contents policy mapping, current renderer/backend fallback state, omission metadata mapping, and empty-state semantics for implemented policies.

The canonical product/domain browse policy, filter registry, classification vocabulary, row universes, and facets are owned by `docs/library/browse/policy-and-classification.md`. This document does not override that canon.

## Product Filter Mapping

The product's initial active workflow filter is **Audio**. Product filters map to backend-owned policies:

```ts
Audio           -> { policy: { kind: 'audioBrowse' }, scopeDepth: 'recursive' }
Media           -> { policy: { kind: 'playableMediaBrowse' }, scopeDepth: 'recursive' }
All Files       -> { policy: { kind: 'sourceFileInventory', fileClasses: [...] }, scopeDepth: 'recursive' }
Companion Files -> { policy: { kind: 'sourceFileInventory', fileClasses: [...] }, scopeDepth: 'recursive' }
```

`playableMediaBrowse` includes durable `source_files.file_class` values `audio` and `video`. It excludes images,
unsupported raw files, diagnostics-only files, generic metadata companions, and CUE sheets.

`audioBrowse` remains a separate audio-only policy. It includes `audio` and excludes `video`, including extension-only
`.mp4` files. Extension classification treats `.m4a` as audio and `.mp4` as video until media-probe authority establishes
stronger facts for a specific file.

**All Files** is raw source inventory. It is not interpreted content and not a problems view. The exact admitted
inventory classes remain owned by `sourceFileInventory`.

**Companion Files** is explicit admitted companion inventory. It is not a workflow media filter and not a problems view.
The canonical product semantics are owned by `policy-and-classification.md`.

The canonical built-in registry also includes **Video** as a distinct workflow filter and **Media** as the audio+video
workflow filter. This implementation companion must not be read as dropping either built-in. Any renderer activation of
Video must use distinct read policy/filter identity and must not reuse Audio or Media rows, cursors, retained snapshots,
or verified-empty state.

The renderer requests a policy and presents returned facts. It does not supply `['audio', 'video']` as a default
`sourceFileInventory` filter, inspect raw source inventory, filter rows, sort rows, or infer hidden content.

Active filter selection participates in contents identity. Switching filter must schedule a re-keyed contents read for
the same selected scope and selected depth. It must not mutate tree containment, selected scope, expansion, scroll, or
focus. It may change content rows, folder facets, omission metadata, and empty-state eligibility.

## Omission Metadata

Every contents result carries required service-owned `hasPolicyOmittedRows: boolean`.

For resolved scopes, the store computes the value across the requested scope and scopeDepth mode, independently of the
current page:

- `playableMediaBrowse` reports browse-relevant image rows as omitted.
- `audioBrowse` reports browse-relevant video and image rows as omitted.
- `sourceFileInventory` reports requestable browse classes omitted by its explicit class filter.
- unsupported raw junk, diagnostics-only files, and unrequested internal classifications do not trigger the value.
- `primaryMedia` returns `false` in this slice; it does not use raw `source_files` as a proxy for the primary-media row
  universe.

Non-browsable and unavailable result states return `false` and let result state drive presentation.

## Empty Presentation

Coverage and omission metadata jointly define zero-row presentation:

- incomplete coverage: `Still indexing` or equivalent;
- complete coverage plus omissions: policy-empty copy such as `No audio tracks in this view`;
- complete coverage without omissions: true empty copy for the active policy.

Verified-empty language is legal only for a matching accepted result whose selected scope, active policy/filter,
scopeDepth, complete coverage, and zero-row count prove absence. Pending, retained-pending, incomplete, blocked, failed,
missing, unavailable, stale, cursor-invalid, and unsupported-selection states must not render verified-empty language.

A folder containing only MP4 files returns video rows under `playableMediaBrowse`. The same folder returns zero rows and
`hasPolicyOmittedRows: true` under `audioBrowse`; it is not presented as truly empty.

## Other Policies

`sourceFileInventory.fileClasses` remains an explicit non-default inventory policy. It exposes requestable `audio`,
`video`, `image`, and admitted `unsupported` companion rows; the admitted unsupported subset is currently CUE sheets.
Persisted `none` and diagnostics-only unsupported kinds remain non-requestable.

`primaryMedia.mediaKinds` remains separate primary-media vocabulary and authority.

## Product Boundaries

Contents rows reuse the existing file-row payload and do not synthesize duration, BPM, musical key, codec, container,
artwork, source label, provenance, or canonical track identity.

The tree remains navigation-only. Audio, video, and file rows stay in contents. Source registration may perform only its
existing scoped first-source activation; scan events refresh data and never select browse scope.

Pending contents remain retained and threshold-gated. Branch loading does not clear selected contents. Enter selects
tree rows; Space remains reserved outside tree selection.
