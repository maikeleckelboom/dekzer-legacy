# Library Tree Row Action Surface and Drag Scope Contract

**Revision:** 2026-06-08

## Purpose

This document separates two concerns that are easy to confuse:

1. The hierarchical tree action surface used by local sources, folders, library scopes, and browse rows.
2. The future drag-and-drop geometry used for crates, playlists, and user-authored ordering workflows.

These are not the same feature.

The immediate problem is the row action surface. The current tree row feels hostile because the actionable area is
fragmented into a small disclosure button, icon/label spans, padding, and dead space. The row should behave like a
professional tree row, not like a group of tiny unrelated controls.

The future drag-and-drop model is important, but it belongs to crates/playlists and authored library organization
surfaces. It must not be used to justify complicated local-filesystem row behavior.

## Core Decision

Local source hierarchy rows are not drag-and-drop reorder surfaces.

Crates and playlists may become drag-and-drop reorder/reparent surfaces.

The row action surface must be fixed first. The fix must preserve the pro-DJ tree contract: row body selects the browse
scope; reveal/collapse is a separate intent; tracks stay in the content pane; parent rows can be both selectable and
expandable.

## Final Product Law

A tree row is a full-width browse-scope selection surface with explicit secondary intent zones.

For local source hierarchy, the full row must be usable for selection. No padding, icon span, label span, or empty
right-side space may block the primary row action. Reveal/collapse is a separate secondary intent carried by a dedicated
reveal lane, not by ordinary row-body click.

For crates and playlists, the same row foundation may later expose drag-and-drop geometry during a drag session, but
those drop zones are temporary interaction geometry, not permanent DOM controls.

## Scope

### In scope now

- Tree row HTML structure
- Full-row pointer surface
- Disclosure/expand hit geometry
- Roving focus target
- Keyboard behavior
- ARIA tree semantics
- Elimination of dead click areas
- Separation between local hierarchy actions and future crate/playlist drag-and-drop

### Out of scope now

- Implementing crate/playlist drag-and-drop
- Reordering local filesystem folders
- Reparenting local files/folders
- Native file drop into the library
- Backend hierarchy semantics
- Source scanning behavior
- Contents read behavior

## Two Different Interaction Classes

## 1. Local Source Hierarchy Rows

Examples:

- local source root
- source location
- literal filesystem directory
- system Music folder
- system Videos folder

These rows are browse/navigation rows.

They need:

- full-width row action surface;
- clear selected/active state;
- clear expanded/collapsed state;
- large, forgiving but distinct reveal/collapse hit area;
- keyboard tree behavior;
- no drag-and-drop reorder zones.

The user must not have to click a tiny 24px disclosure button to interact with the hierarchy.

## 2. Crate and Playlist Rows

Examples:

- crate
- playlist
- playlist folder
- prepared room sections later
- user-authored organization nodes later

These rows may later need:

- reorder above;
- reorder below;
- make child;
- reparent to ancestor level;
- authored grouping behavior;
- drag preview;
- explicit domain validation.

This is where the multi-dimensional drag-and-drop model belongs.

The `dekz-multi-dimensional-drag-and-drop-tree.jpg` reference applies to this class, not to local media folder browsing.

## Row Surface Contract

Each visible tree row has one row foundation:

- one roving focus target;
- one full-width row background;
- one full-width hover state;
- one full-width selected/active state;
- one row controller that routes pointer intent.

Do not create multiple keyboard stops inside a normal tree row.

Do not allow the visible label/icon/right-side empty space to be non-clickable. These regions route to the primary row
action, which is browse-scope selection for local hierarchy rows.

## Target DOM Concept

Use one row shell and one action surface.

Recommended conceptual structure:

    div.tree-row-shell
      data-node-id
      data-depth
      data-kind
      data-selected
      data-expanded
      data-expandable

      div.tree-row-action
        role treeitem
        tabindex 0 or -1
        aria-level
        aria-posinset
        aria-setsize
        aria-selected
        aria-expanded when applicable

        span.tree-row-indent
        span.tree-row-disclosure
        span.tree-row-icon
        span.tree-row-label
        span.tree-row-meta

      div.tree-row-drag-overlay
        aria-hidden true
        only active during supported drag sessions

The important part is not the exact element names. The important part is that the row action is full-width and owns the
row interaction.

## Disclosure / Reveal Surface

Do not implement reveal/collapse as a tiny chevron-only control.

The visible disclosure icon may remain visually small. The reveal hit target must be forgiving, but it must remain a
distinct secondary intent from row-body selection.

For local hierarchy rows, the row controller must support a dedicated reveal lane. The reveal lane may be wider than the
visible chevron and may include the disclosure gutter/indent region, but it must not consume the ordinary icon, label,
metadata, or empty right-side row body. Those regions select the browse scope.

Implementation rule:

- the visible chevron is presentation;
- reveal/collapse intent is handled by the row controller;
- the primary row action surface spans the row width and selects the browse scope;
- the reveal lane is a distinct pointer zone owned by the row controller;
- click routing must not depend on tiny inner button geometry;
- ordinary row-body click must never select and reveal in the same gesture.

Do not name the full row action `expandButton` in code. The full row is not an expand button. It is the tree row
selection surface with an explicit secondary reveal lane.

## Local Hierarchy Primary Action

For local source hierarchy rows, the product intent is locked:

- ordinary row-body click selects the browse scope only;
- icon, label, metadata, and empty right-side row space all route to browse-scope selection;
- reveal/collapse is a separate secondary intent exposed through a forgiving reveal lane;
- reveal-lane click reveals/collapses only and does not select as a hidden side effect;
- keyboard ArrowRight/ArrowLeft own deterministic expand/collapse behavior;
- selected and expanded state remain visually distinguishable;
- no padding or empty space blocks the primary selection action.

There is no implementation option where a normal single click both selects and reveals. There is no implementation
option where double-click becomes the primary reveal contract. Double-click may remain a benign no-op or repeat
selection unless a later, explicit product contract assigns it to a different non-conflicting action.

The non-negotiable part: the current tiny chevron-only hit target and dead row padding are unacceptable, but the fix is
not row-click expansion. The fix is a full-width selection surface plus a distinct, forgiving reveal lane.

Tests must prove:

- clicking row body selects only;
- clicking icon/label/metadata/empty right-side row area selects only;
- clicking reveal lane reveals/collapses only;
- selection and expansion state stay independently coherent;
- there is no dead row space;
- the reveal lane is forgiving and not limited to the visible chevron pixels.

## Crate / Playlist Drag-and-Drop Scope

Drag-and-drop does not apply to local filesystem rows in the first implementation.

Drag-and-drop applies only to user-authored organization surfaces such as crates and playlists once those surfaces
exist.

Allowed future drag intents:

- reorder above;
- reorder below;
- make child;
- reparent to ancestor level;
- reject.

These are proposed intents from pointer geometry. The domain model validates them before commit.

## Multi-Dimensional Drag Geometry

The reference image describes drag geometry for authored hierarchy rows.

For crates/playlists, a hovered row may expose temporary zones during a drag session:

- top strip: reorder above;
- bottom strip: reorder below;
- body: make child if allowed;
- left ancestor columns on last-in-group rows: reparent to ancestor level.

These zones are not permanent DOM controls.

They are computed during a drag session from:

- row rectangle;
- pointer position;
- row depth;
- indentation width;
- row expanded state;
- last-in-group state;
- allowed target operations.

## Last-in-Group Reparenting

The multi-dimensional model depends on last-in-group rows.

A row that is last in its current group can expose left-side ancestor columns during drag hover. Those columns allow the
user to move an item out to an ancestor level without awkward multi-step dragging.

This belongs to crate/playlist authoring.

It does not belong to local filesystem source browsing.

## ARIA Contract

The tree must keep normal tree semantics:

- role tree on the container;
- role treeitem on the row action;
- roving tabindex;
- aria-level;
- aria-posinset;
- aria-setsize;
- aria-selected;
- aria-expanded when expandable.

Do not create a focusable button for every disclosure icon in the normal tree row.

Do not create focusable drop zones.

Do not expose drag geometry to assistive tech as permanent controls.

## Keyboard Contract

Keyboard behavior belongs to the focused tree row.

Required behavior:

- ArrowDown moves to next visible row.
- ArrowUp moves to previous visible row.
- ArrowRight expands/reveals expandable collapsed row.
- ArrowRight on expanded row may move to first child.
- ArrowLeft collapses expanded row.
- ArrowLeft on collapsed or leaf row may move to parent.
- Enter activates/selects the row according to the current tree contract.
- Space remains reserved unless deliberately assigned later.

Keyboard drag-and-drop is out of scope for now. The DOM must not prevent it later.

## CSS / Hitbox Contract

The row must behave as one continuous hit surface.

Requirements:

- full-row hover background;
- full-row selected background;
- full-row focus ring;
- no pointer-dead padding between disclosure, icon, label, and right-side empty space;
- leaf rows keep alignment without showing fake disabled chevrons;
- icon and label clicks route to the row action;
- empty right-side row area routes to the row action.

If an element exists only for layout, it must not block pointer events.

## Data Attributes

Recommended row attributes:

- data-node-id
- data-depth
- data-kind
- data-selected
- data-expanded
- data-expandable
- data-action-mode

Recommended drag-only attributes:

- data-drag-active
- data-drop-intent
- data-drop-valid
- data-drop-zone

Drag attributes must appear only during supported drag sessions.

## Renderer Ownership

Tree projection owns:

- visible rows;
- row depth;
- row kind;
- expanded/collapsed projection;
- selected/active projection;
- last-in-group metadata for future D&D.

Tree controller owns:

- row activation;
- selection requests;
- expansion/reveal requests;
- keyboard navigation;
- click routing;
- future drag session lifecycle.

Drag geometry owns:

- pointer-coordinate to candidate intent translation during supported drag sessions.

Domain model owns:

- whether an operation is legal;
- final commit semantics.

The DOM owns none of these facts. It realizes them.

## Rejection Cases

Reject these designs:

### Tiny disclosure-only interaction

A local hierarchy row must not require clicking only the small chevron to reveal. The reveal lane must be forgiving.

### Dead row padding

No layout spacer, padding, icon span, label span, or right-side empty space may block the row action.

### Single-click select-and-reveal

A normal local hierarchy row-body click must never select and reveal in the same gesture. Selection and reveal are
separate intents.

### Double-click as primary reveal

Double-click must not be required for reveal/collapse. Keyboard ArrowRight/ArrowLeft and the reveal lane own
reveal/collapse.

### Focusable disclosure button per row

This creates two keyboard targets per tree row and makes focus behavior worse.

### Drag-and-drop for local filesystem hierarchy

Do not reorder or reparent local filesystem folders through this tree.

### Permanent drop target DOM

Drop zones are temporary drag-session geometry, not permanent focusable controls.

### Two row implementations

Do not keep an old row and a new row alive as compatibility paths.

### Icon owns expansion

The icon communicates structural/media meaning. Expansion is row/disclosure state.

## Implementation Sequence

### Phase 1: Fix row action surface

- Make the row action full-width.
- Remove dead pointer areas.
- Keep one roving focus target per row.
- Keep ARIA tree semantics valid.
- Ensure icon/label/empty-space clicks route to row action.
- Keep leaf alignment without fake disclosure.

### Phase 2: Implement and test local hierarchy activation

Implement the locked policy:

- row body selects only;
- icon, label, metadata, and empty right-side row area select only;
- reveal lane reveals/collapses only;
- ArrowRight/ArrowLeft reveal/collapse deterministically;
- double-click is not a primary reveal contract.

Then test it.

The current tiny chevron-only behavior is not acceptable, but row-click expansion is also unacceptable.

### Phase 3: Prepare row metadata for future D&D

- Add row kind metadata.
- Add depth metadata if missing.
- Add last-in-group metadata only if needed by future D&D.
- Do not implement crate/playlist drag behavior yet.

### Phase 4: Crate/playlist drag-and-drop

Only when crates/playlists need authored ordering:

- add drag session model;
- add candidate drop intents;
- add row geometry measurement;
- add last-in-group ancestor reparent zones;
- add domain validation;
- add visual preview.

## Acceptance Criteria

The row action surface work is acceptable when:

- clicking icon/label/right-side empty row area selects the browse scope and is not blocked;
- local hierarchy rows can be revealed through a distinct, forgiving reveal lane without targeting a tiny chevron;
- tab focus lands on the row, not an inner disclosure button;
- focus ring covers the row;
- hover and selected background cover the row;
- leaf rows do not show fake disabled disclosure;
- ArrowRight and ArrowLeft keep deterministic tree behavior;
- ARIA tree semantics remain valid;
- no local filesystem drag-and-drop behavior is introduced;
- no old row implementation remains as a compatibility path.

The future D&D work is acceptable only when:

- it is scoped to crates/playlists or other authored organization rows;
- it exposes exactly one candidate intent at a time;
- it supports ancestor-level reparent zones for last-in-group rows;
- invalid targets reject explicitly;
- drop zones are not focusable;
- domain validation owns final legality.

## Source Guards

After row-surface implementation, search for and reject:

- focusable disclosure button in tree rows;
- role button on disclosure affordance;
- pointer-blocking layout spans inside the row;
- row click handlers split across unrelated child controls;
- row-body click that both selects and reveals;
- double-click as the primary reveal/collapse path;
- permanent drop target elements;
- native draggable attributes on local hierarchy rows;
- old row component wrappers kept for compatibility.

## Stop Rules

Stop and report instead of continuing if:

- fixing hitboxes requires changing backend hierarchy semantics;
- local filesystem rows start acquiring reorder/reparent behavior;
- ARIA becomes invalid;
- multiple tab stops per row are introduced;
- implementation needs two competing tree-row components;
- tests require weakening keyboard navigation.

## Final Product Statement

Dekzer local hierarchy rows are full-width browse-scope selection surfaces with a distinct, forgiving reveal lane. They
are not tiny chevron targets, and they are not row-click expansion surfaces. Crate and playlist rows may later become
authored drag-and-drop surfaces with multi-dimensional reparent geometry. These two interaction classes share a row
foundation but must not be confused.
