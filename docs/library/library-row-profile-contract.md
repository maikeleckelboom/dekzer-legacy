---
status: candidate
doctrine-version: 0.1
last-reviewed: 2026-05-28
owner: renderer-substrate-boundary
canonical-context:
  - library-tree-frame-stability-contract
  - library-tree-track-segment-rows-contract
  - source-hierarchy-contract (TODO: not yet written)
scope:
  - hierarchy-projection-policy
  - row-profile-definitions
  - cache-key-profile-component
  - profile-change-invalidation
---

# Library Row Profile Contract

## Core law

**A row profile is a projection policy, not a renderer preference.**

The profile is owned by the projection read model. It determines which node kinds
appear in a read result, what child counts include, and what coverage state means.
The renderer passes a profile as a read parameter. It does not transform results
to match a different profile after the fact.

## What a row profile governs

| Axis                       | Governed by row profile | Not governed by row profile |
|----------------------------|-------------------------|-----------------------------|
| Which node kinds appear    | Yes                     |                             |
| Whether tracks appear      | Yes                     |                             |
| Whether segments appear    | Yes                     |                             |
| Whether companions appear  | Yes                     |                             |
| Child count semantics      | Yes                     |                             |
| Sort order within a branch |                         | Governed by sort policy     |
| Prep readiness filter      |                         | Governed by filter policy   |
| Search/text filter         |                         | Governed by filter policy   |
| Visual presentation        |                         | Governed by renderer        |

A row profile change must trigger full cache clear for that profile key. Sort and
filter policy changes do not change the profile key.

## Defined profiles (v1)

| Profile key                     | Node kinds visible                                                  | Child count includes             |
|---------------------------------|---------------------------------------------------------------------|----------------------------------|
| `directories_only`              | source_root, directory, blocked, excluded                           | Directories only                 |
| `primary_media`                 | source_root, directory, primary_media, blocked, excluded            | Directories + primary media      |
| `primary_media_with_companions` | source_root, directory, primary_media, companion, blocked, excluded | Directories + media + companions |
| `segments_enabled`              | source_root, directory, primary_media, segment, blocked, excluded   | Directories + media + segments   |
| `diagnostic_all_candidates`     | All node kinds including unsupported, rejected, errored             | All observed entries             |

`diagnostic_all_candidates` is for debug and diagnostic views only. It must not
be the default profile for any user-facing tree surface.

Analyzed-only filtering is a filter policy applied over `primary_media`, not a
separate row profile. It does not change the profile key.

## Profile and cache key relationship

The renderer branch cache key must include the row profile key and the sort policy key.

```
cacheKey = (sourceRootNodeId, parentNodeId, rowProfile, sortPolicyKey, projectionPolicyKey)
```

Sort policy is not part of `rowProfile`, but it is part of the branch cache identity. A cache entry produced under one
sort policy must not satisfy a request under a different sort policy. Sort changes keep existing rows visible while a
replacement order is read, then patch the branch without an empty frame.

Two reads for the same parent under different profiles produce separate cache
entries. Switching profiles invalidates the cache entries for the old profile.
It does not invalidate entries for other profiles on the same parent.

## Profile change propagation

| Event                                      | Required behavior                                               |
|--------------------------------------------|-----------------------------------------------------------------|
| User switches active row profile           | Full clear of all cache entries for the old profile key.        |
| Profile definition changes (schema update) | Full clear of all cache entries for that profile key.           |
| Profile is added                           | No invalidation needed; no existing cache entries reference it. |
| Profile is removed                         | Clear cache entries for that profile key.                       |

Profile changes do not trigger scan epoch advances. They affect projection shape only.

## Profile and child count honesty

The `knownChildCount` and `isCountComplete` in a child summary are always
relative to the active profile.

A directory with 14 audio files and 3 companion files:

- Under `primary_media`: `knownChildCount = 14`
- Under `primary_media_with_companions`: `knownChildCount = 17`
- Under `directories_only`: `knownChildCount = 0` (or no children if none)

A renderer must not reuse a child summary produced under one profile to satisfy
a request under another.

## Profile and the read API

All child read surfaces accept `rowProfile` as a required parameter.

```
readChildren(parentNodeId, rowProfile, sortPolicy, limit, cursor)
readChildSummary(parentNodeIds, rowProfile)
readChildrenBatch(parentNodeIds, rowProfile, sortPolicy, limit)
readVisibleFrontier(rootNodeId, expandedNodeIds, viewportHint, rowProfile, sortPolicy)
```

The projection substrate produces different row sets per profile. The renderer
never post-filters profile output.

## Profile and the stale response guard

The `rowProfile` field in the request guard (defined in the frame stability
contract) ensures a response produced under one profile is never accepted by
a cache entry keyed to another.

A response whose `rowProfile` does not match the current active profile for that
branch is silently discarded.

## What is not a row profile

The following are not profiles and must not be encoded as profile variants:

| Excluded concept              | Correct home        |
|-------------------------------|---------------------|
| BPM range filter              | Filter policy       |
| Key filter                    | Filter policy       |
| Prep readiness filter         | Filter policy       |
| Text/search filter            | Filter policy       |
| Sort order (BPM, title, etc.) | Sort policy         |
| Waveform display resolution   | Renderer preference |
| Column visibility             | Renderer preference |

Adding these to the profile enum would require a cache clear on every filter
change. That is wrong.

## Non-goals

This contract does not define how profiles are surfaced in the product UI, how
a user switches profiles, or how profiles interact with the contents panel.
Those belong to the browsing policy domain and the selection-contents contract.
