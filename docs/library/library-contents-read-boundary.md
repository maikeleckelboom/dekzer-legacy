---
status: pointer
doctrine-version: 0.2
last-reviewed: 2026-06-06
owner: renderer-substrate-boundary
canonical-target:
  - docs/decisions/library-contents-read-boundary.md
scope:
  - contents-read-boundary-discovery
---

# Library Contents Read Boundary

The canonical typed boundary is `docs/decisions/library-contents-read-boundary.md`.

The active policy union contains `playableMediaBrowse`, `audioBrowse`, `sourceFileInventory { fileClasses }`, and
`primaryMedia { mediaKinds }`. The default product request uses `playableMediaBrowse`; `audioBrowse` remains audio-only.

Every result carries required `hasPolicyOmittedRows`. The store/service computes it for the requested scope and
scopeDepth mode. The renderer presents the returned rows, scopeCoverage, and omission fact without inspecting raw inventory.
Incomplete zero-row coverage is not authoritative empty.

Source-file inventory uses `file_kind` for detailed taxonomy and durable `file_class` for coarse classification.
Extension classification treats `.m4a` as audio and `.mp4` as video. Boundary source-file rows expose `fileClass`;
`sourceFileInventory` alone owns caller-supplied `fileClasses`.

Tree navigation remains directory/navigation-only. Audio, video, and file rows stay in contents.
