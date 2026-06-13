---
status: accepted
doctrine-version: 0.1
last-reviewed: 2026-06-09
owner: renderer-substrate-boundary
canonical-context:
  - selection-contents-contract
  - browse-policy
  - media-relevant-file-inventory-contract
scope:
  - source-hierarchy-tree
  - navigation-only-tree-rows
  - contents-scope-selection
---

# Source Hierarchy Contract

## Core Law

The library tree is a navigation surface. It projects sources, source locations, directories, and navigation state or action rows. It does not project source-file rows.

Audio, video, image, CUE, metadata companion, unsupported, and unknown source files belong to contents or diagnostic inventory reads, not to the tree hierarchy.

Image-only folders are not default navigation rows unless a later product policy explicitly admits them. Directory
navigation exists to browse source structure and reach contents scopes, not to surface artwork-only inventory.

## Tree Rows

Tree hierarchy rows may include:

- source rows;
- source-location rows;
- directory rows;
- read-state rows;
- load-more rows for explicit hierarchy continuation.

Tree hierarchy rows must not include source-file rows, even when the backend or a diagnostic read model can still represent those files.

## Leaf Directories

A directory with no navigable child directories is a valid selectable destination node. Selecting it creates the contents scope for that folder.

Leaf directories do not show a disclosure affordance. A directory may show disclosure only when it has navigable child directories or an explicit hierarchy continuation action.

## Disclosure Affordance Law

Tree disclosure is earned only by confirmed child browse-scope presence.

For Local Files sources, child browse scopes are child directory scopes. A source or directory row may show a disclosure
affordance only when the hierarchy authority has confirmed child directory scopes or returned an explicit hierarchy
continuation action. A folder that contains matching media files but no child directory scopes is selectable and useful,
but it is not expandable.

Content/media presence must not create a chevron. Folder usefulness, folder admission, content rows, and facets may be
filter-aware, but they do not decide tree disclosure. Audio files, video files, companion files, artwork, raw inventory,
or known descendant media observations do not make a row expandable unless child browse scopes are confirmed.

Unknown child-scope readiness must not be rendered as known expandable. A row whose child browse-scope state is unknown
must show honest pending, probing, retained, blocked, failed, or absent disclosure state according to the active tree
readiness contract; it must not show a disclosure affordance that implies confirmed children.

## Contents and Inventory

The contents pane owns file rows for the selected source or directory scope. Selecting a tree row establishes browse
scope only. The product's initial active workflow filter is **Audio**, which uses the audio contents policy over the
selected scope with recursive coverage. The separate **Media** filter uses the playable-media policy. Scope selection,
active filter, contents policy, and scope depth remain separate owners.

Explicit source-file inventory remains available through contents or diagnostic paths. Companion files such as CUE sheets and artwork remain source-file inventory rows; they are not tree children.

Contents rows are not canonical tracks. They are scoped source-file browse rows until a richer read model explicitly
defines otherwise.
