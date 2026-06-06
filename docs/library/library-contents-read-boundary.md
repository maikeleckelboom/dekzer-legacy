---
status: pointer
doctrine-version: 0.1
last-reviewed: 2026-06-06
owner: renderer-substrate-boundary
canonical-target:
  - docs/decisions/library-contents-read-boundary.md
scope:
  - contents-read-boundary-discovery
---

# Library Contents Read Boundary

The canonical contents read boundary is `docs/decisions/library-contents-read-boundary.md`.

Use that document for the typed contents request, policy, recursion, cursor, and pagination contract. Use
`docs/library/contents-policy-shape.md` for the implemented profile-specific policy decision,
`docs/library/library-contents-browse-policy.md` for the current default audio-first browse policy and
`docs/library/library-tree-selection-contents-contract.md` for renderer coupling, retained-row behavior, refresh
planning, and panel containment.

The source-file inventory vocabulary is `file_kind` for detailed taxonomy and `file_class` for coarse persisted
classification. Boundary source-file rows expose `fileClass`; `sourceFileInventory` owns `fileClasses`.
