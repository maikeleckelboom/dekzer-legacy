---
status: superseded
doctrine-version: 0.2
last-reviewed: 2026-06-06
owner: renderer-substrate-boundary
superseded-by:
  - source-hierarchy-contract
  - library-tree-selection-contents-contract
  - library-contents-browse-policy
scope:
  - superseded-in-tree-media-row-framing
---

# Library Tree Track And Segment Rows Contract

## Status

This document is superseded.

The current library tree is navigation-only. It projects source, source-location, directory, and hierarchy
state/action rows. It does not project audio rows, source-file rows, track rows, companion rows, or segment rows.

## Current Authority

Current implementation targets must use:

| Question                                        | Authority                                                  |
| ----------------------------------------------- | ---------------------------------------------------------- |
| What appears in tree navigation?                | `docs/library/source-hierarchy-contract.md`                |
| How selection creates a contents scope?         | `docs/library/library-tree-selection-contents-contract.md` |
| What appears in default contents browse?        | `docs/library/library-contents-browse-policy.md`           |
| What the next audio row read-model proposal is? | `docs/library/audio-browse-row-v0.md`                      |

## Rejection Rule

Do not use this file to restore file, audio, track, companion, or segment rows into tree navigation. Any future media
row surface needs a new contents/read-model contract and an explicit product decision.
