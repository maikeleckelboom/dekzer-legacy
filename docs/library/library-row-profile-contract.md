---
status: superseded
doctrine-version: 0.2
last-reviewed: 2026-06-06
owner: renderer-substrate-boundary
superseded-by:
  - source-hierarchy-contract
  - library-contents-browse-policy
  - audio-browse-row-v0
scope:
  - superseded-tree-row-profile-framing
---

# Library Row Profile Contract

## Status

This document is superseded.

The current library tree does not expose renderer-selectable row profiles and does not use row profiles to place audio,
track, companion, or segment rows in navigation. The current `readLibraryTreeChildren` surface is navigation-only from
the product renderer perspective: sources, source locations, directories, and hierarchy state/action rows.

## Current Authority

| Question                             | Authority                                                  |
| ------------------------------------ | ---------------------------------------------------------- |
| Tree navigation row admission        | `docs/library/source-hierarchy-contract.md`                |
| Default contents browse policy       | `docs/library/library-contents-browse-policy.md`           |
| Contents scope from tree selection   | `docs/library/library-tree-selection-contents-contract.md` |
| Proposed audio browse row read model | `docs/library/audio-browse-row-v0.md`                      |

## Rejection Rule

Do not use this document to add renderer-controlled tree row profiles, renderer-side filtering/sorting authority, or
file/audio rows in tree navigation.
