---
status: future-architecture
doctrine-version: 0.1
last-reviewed: 2026-05-28
amended: 2026-05-30
owner: renderer-substrate-boundary
canonical-context:
  - product-doctrine-shortened
  - library-tree-frame-stability-contract
  - library-row-profile-contract
  - source-hierarchy-contract
  - first-slice-substrate-map
scope:
  - track-rows-in-tree
  - segment-rows-in-tree
  - companion-rows-in-tree
  - node-kind-interaction-rules
  - child-count-semantics-per-kind
---

# Library Tree Track and Segment Rows Contract

**Status:** FUTURE ARCHITECTURE — This document describes tree row kinds beyond the current first-slice
implementation. The current first-slice library tree is navigation-only and projects sources, source locations,
directories, and hierarchy state/action rows. Source-file rows remain available through contents and diagnostic
inventory paths, not as current tree rows. `primary_media`, `segment`, `companion`, `blocked`, `excluded`, and
`unsupported` are future tree-projection row kinds that are not active in the current tree. The product rules below
remain valid for future implementation. Do not use them as current first-slice implementation targets.

## Core law (future)

**Tree children may be hierarchy nodes, media rows, companion rows, or segment rows. Each kind declares its own
authority, interaction rules, and contribution to child counts.**

A tree is not a file browser. A tree is not a flat track list. It is a hybrid navigation surface where the visible
node kinds depend on the active row profile. The frame stability rules from the frame-stability contract apply to
all node kinds without exception.

## Node kind taxonomy (v1)

| Kind            | Authority                                                              | Can have children    |
|-----------------|------------------------------------------------------------------------|----------------------|
| `source_root`   | Substrate source identity. Root of a mounted source.                   | Yes                  |
| `directory`     | Substrate directory node. File system directory within a source.       | Yes                  |
| `primary_media` | Track attachment. A playable primary media file.                       | No (v1)              |
| `companion`     | Track attachment. A companion artifact alongside primary media.        | No                   |
| `segment`       | Source segment. A CUE/chapter-defined segment within a container file. | No                   |
| `blocked`       | Scan observation. A path that could not be traversed.                  | No                   |
| `excluded`      | Scan policy observation. A path deliberately skipped.                  | No                   |
| `unsupported`   | Scan observation. A file present but not classifiable as media.        | No (diagnostic only) |

`primary_media`, `companion`, `segment`, `blocked`, and `excluded` are leaf
nodes. They do not expand further.

## When tracks appear in the tree

Tracks appear as direct children of directories when the active row profile
permits them.

| Row profile                     | Track rows visible in tree |
|---------------------------------|----------------------------|
| `directories_only`              | No                         |
| `primary_media`                 | Yes                        |
| `primary_media_with_companions` | Yes                        |
| `segments_enabled`              | Yes                        |
| `diagnostic_all_candidates`     | Yes                        |

A directory may have both subdirectory children and track children simultaneously.
The renderer must display both in the same branch without treating tracks as a
separate second panel.

Example: `/Music/Techno/Producer Name/` may contain:

- 3 subdirectories (album folders)
- 12 track files at the directory level

Under `primary_media`, that directory expands to show 3 directory rows and 12
primary_media rows. `knownChildCount = 15`. `isCountComplete` reflects scan
coverage state.

## When segments appear in the tree

Segments appear as children of their container file's parent directory, or as
children of a `source_root` if the disc image is the root, when the active row
profile includes `segments_enabled`.

Segments are not independent track authority. A segment row is a projection of
a `source_segment` substrate record. Its playable authority requires that the
segmentation has been accepted (i.e., a `SourceSegmentSets` record exists for
the parent file with an accepted segmentation artifact).

| Segment condition                  | Tree behavior                                                                       |
|------------------------------------|-------------------------------------------------------------------------------------|
| Segmentation accepted              | Segment rows appear as children; each is independently loadable.                    |
| Segmentation pending / not yet run | Segment rows do not appear; parent shows a single unsegmented row or pending state. |
| Segmentation failed                | Segment rows do not appear; parent shows failed segmentation state.                 |

Segments inherit all frame stability rules: stable node ID keys, no empty-frame
flicker on load, epoch guards on reads.

## Child count semantics by node kind

`knownChildCount` in a child summary counts the node kinds visible under the
current profile.

| Counting rule                                                                       |
|-------------------------------------------------------------------------------------|
| `directories_only`: count subdirectories only.                                      |
| `primary_media`: count subdirectories + primary media files.                        |
| `primary_media_with_companions`: count subdirectories + primary media + companions. |
| `segments_enabled`: count subdirectories + primary media + accepted segments.       |
| Blocked and excluded nodes: counted if the profile makes them visible.              |

A renderer must not display a count that mixes profile semantics. A count from a
`directories_only` read must not be reused as the count for a `primary_media` read.

## Interaction rules by node kind

| Kind            | Double-click / enter     | Drag behavior                           | Right-click / context menu        |
|-----------------|--------------------------|-----------------------------------------|-----------------------------------|
| `source_root`   | Expand/collapse          | Not a drag source.                      | Source options, rescan, remove.   |
| `directory`     | Expand/collapse + select | Not a drag source (v1).                 | Browse options, prep this folder. |
| `primary_media` | Load to focused deck     | Drag source. Subject: track attachment. | Load, add to crate, prep, locate. |
| `companion`     | Open or reveal           | Not a drag source.                      | Reveal in finder, info.           |
| `segment`       | Load to focused deck     | Drag source. Subject: source segment.   | Load, add to crate, prep.         |
| `blocked`       | No action.               | Not a drag source.                      | Show reason, retry access.        |
| `excluded`      | No action.               | Not a drag source.                      | Show reason, include in scan.     |

## Track authority in tree rows

A `primary_media` tree row represents a track attachment projected into the tree.
It carries enough state to load the track to a deck without requiring a separate
track detail fetch.

Required fields on a `primary_media` tree row:

| Field             | Meaning                                                  |
|-------------------|----------------------------------------------------------|
| nodeId            | Substrate hierarchy node ID (stable key).                |
| trackId           | Track identity in the substrate.                         |
| attachmentId      | Track attachment identity.                               |
| label             | Display title (from track metadata).                     |
| artistDisplay     | Artist string for display.                               |
| durationMs        | Duration in milliseconds, if known.                      |
| readinessState    | Projected readiness from preparation facet states.       |
| availabilityState | Whether the backing source file is currently accessible. |

A track row in the tree is not a full track detail record. It is a projection
sufficient for navigation and load. The contents panel provides richer detail.

## Segment authority in tree rows

A `segment` tree row represents a source segment projected into the tree.
Segment rows are not accepted track identity. They are accepted source material.

Required fields on a `segment` tree row:

| Field             | Meaning                                        |
|-------------------|------------------------------------------------|
| nodeId            | Substrate hierarchy node ID (stable key).      |
| segmentId         | Source segment identity.                       |
| segmentSetId      | Parent segment set identity.                   |
| label             | Display title from segment metadata.           |
| ordinal           | Position within the segment set.               |
| startOffsetMs     | Start offset within the container file.        |
| durationMs        | Duration in milliseconds.                      |
| availabilityState | Whether the backing source file is accessible. |

## Frame stability applies to all kinds

The frame stability laws from the frame-stability contract apply without
exception to `primary_media`, `companion`, and `segment` rows.

| Rule                               | Applies to track/segment rows |
|------------------------------------|-------------------------------|
| Stable substrate node ID keys      | Yes                           |
| No empty-frame flicker on refresh  | Yes                           |
| Epoch guards on reads              | Yes                           |
| Branch patching (not full rebuild) | Yes                           |
| Drag stability during refresh      | Yes                           |
| Source unavailable: keep rows      | Yes (show degraded state)     |

## v1 scope boundary

Track and segment row rendering is in scope for v1.

The following are explicitly out of scope for v1:

```text
In-tree inline waveform display
In-tree prep progress indicators beyond readinessState
In-tree BPM/key overlays
In-tree multi-select visual treatment (logic is in scope; visual spec is not)
Drag-to-reorder within tree branches
```

## Non-goals

This contract does not define how the DJ prepares tracks from the tree, how
crate/playlist organization nodes appear in the tree (those are organization
domain nodes, not hierarchy nodes), or how search results modify the visible
tree. Those are governed by separate contracts.
