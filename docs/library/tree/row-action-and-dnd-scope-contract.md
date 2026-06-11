# Library Tree Row Action Surface and Drag Scope Contract

**Revision:** 2026-06-11

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

## Visual Dimension Contract

A row may show several visual layers, but each layer has one meaning:

| Visual layer                         | Meaning                                                              |
| ------------------------------------ | -------------------------------------------------------------------- |
| Full-row selected background or rail | Selected browse scope driving the contents pane                      |
| Full-row focus outline               | Keyboard command target                                              |
| Disclosure/reveal lane               | Expand, collapse, or load branch                                     |
| Folder/source icon                   | Structural/media/source identity and child-readiness reinforcement   |
| Ambient status slot                  | Sparse readiness, refresh, availability, blocked, or failed signal   |
| Drag-hover overlay                   | Future temporary drop-target feedback during supported drag sessions |

No row state may steal another visual layer. Focus is not selection. Drag hover is not selection. Ambient status is not
child existence. Folder icon fill is not expanded state.

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
      data-focused
      data-child-readiness
      data-ambient-status

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
        span.tree-row-status

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
visible chevron and may include the disclosure gutter/indent region, padding, and border assigned to that lane, but it
must not consume the ordinary icon, label, metadata, status slot, or empty right-side row body. Those regions select the
browse scope or remain informational/action-specific as described below.

The reveal hit target is rectangular, not glyph-shaped. Any pointer event inside the reveal lane is reveal/collapse/load
intent. Pointer routing must not depend on hitting the chevron pixels exactly.

Implementation rule:

- the visible chevron is presentation;
- the folder/source icon is not the expansion control for local hierarchy rows;
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
- status-slot indicators are informational unless a specific explicit action is present;
- reveal/collapse is a separate secondary intent exposed through a forgiving reveal lane;
- reveal-lane click reveals/collapses only and does not select as a hidden side effect;
- keyboard ArrowRight/ArrowLeft own deterministic expand/collapse behavior;
- selected and expanded state remain visually distinguishable;
- no padding or empty space blocks the primary selection action;
- no ambient status indicator changes selection, expansion, focus, or contents scope.

There is no implementation option where a normal single click both selects and reveals. There is no implementation
option where double-click becomes the primary reveal contract. Double-click may remain a benign no-op or repeat
selection unless a later, explicit product contract assigns it to a different non-conflicting action.

The non-negotiable part: the current tiny chevron-only hit target and dead row padding are unacceptable, but the fix is
not row-click expansion. The fix is a full-width selection surface plus a distinct, forgiving reveal lane.

## Ambient Status Slot

The status slot is a sparse informational layer for readiness, refresh, availability, blocked, or failed signals. It is
not a default decoration applied to every row.

Rules:

- no indicator means normal;
- blue or cyan may indicate active refresh, probe, scan, or retained/pending activity;
- amber may indicate incomplete, degraded, partial, or permission-warning state;
- red may indicate blocked or failed state;
- green may indicate ready/healthy only where that status is useful enough to display;
- status indicators must not decide child existence, emptiness, scan completion, or source availability;
- status indicators must not select, reveal, focus, or update contents unless a later explicit action contract assigns a
  specific control to that slot.

The status slot projects state owned by upstream readiness/lifecycle/projection owners. The DOM does not infer status.

Tests must prove:

- clicking row body selects only;
- clicking icon/label/metadata/empty right-side row area selects only;
- clicking reveal lane reveals/collapses only;
- clicking status indicator does not accidentally select, reveal, or focus unless an explicit action exists;
- selection, focus, and expansion state stay independently coherent;
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

## Drag-Hover Auto-Expand for Future Authored Trees

Auto-expand during drag is future-scope and applies only to supported authored hierarchy rows, such as crates/playlists
or prepared-room organization surfaces. It does not apply to local filesystem hierarchy rows in the first local-library
implementation.

If implemented later, the contract is:

- drag hover is drag intent, not selection intent;
- drag hover must not change the selected browse scope;
- drag hover must not update the contents pane;
- drag hover must not masquerade as keyboard focus;
- valid/denied drop target feedback appears immediately, usually within 0-100ms;
- auto-expand arms only after a stable dwell, roughly 250-350ms;
- collapsed branch expansion or child-readiness probing fires only after a longer stable dwell, roughly 600-800ms;
- the dwell resets when the pointer leaves the row, moves to another row, drag mode/modifier changes, the row becomes
  invalid, or the drag cancels;
- a short cooldown after auto-expand prevents dragging down a tree from exploding every branch open;
- branches opened only by drag are tagged in a drag expansion session;
- drag-opened branches may be restored on cancel/leave without drop;
- a successful drop inside a drag-opened branch may keep that branch open;
- explicit user expansion during a drag commits as normal expansion.

Auto-expand never changes selection and never causes contents refresh. It is a temporary branch visibility operation
inside a drag session.

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

- ArrowDown moves focus to next visible row.
- ArrowUp moves focus to previous visible row.
- ArrowRight expands/reveals expandable collapsed row.
- ArrowRight on expanded row is a no-op by default unless a later contract adds focus-to-first-child.
- ArrowLeft collapses expanded row.
- ArrowLeft on collapsed or leaf row is a no-op by default unless a later contract adds focus-to-parent.
- Enter activates/selects the focused row according to the current tree contract.
- Space remains reserved unless deliberately assigned later.

Focus movement is not selection. The selected browse scope and content pane remain unchanged until row-body click or
Enter selects a row.

Keyboard drag-and-drop is out of scope for now. The DOM must not prevent it later.

## CSS / Hitbox Contract

The row must behave as one continuous hit surface.

Requirements:

- full-row hover background;
- full-row selected background or selection rail;
- full-row focus ring that is visually distinct from selected state;
- no pointer-dead padding between disclosure, icon, label, status slot, and right-side empty space;
- leaf rows keep alignment without showing fake disabled chevrons;
- icon and label clicks route to the row action;
- empty right-side row area routes to the row action;
- status indicators are sparse and absent for normal rows;
- status indicators do not create extra tab stops.

If an element exists only for layout, it must not block pointer events.

## Data Attributes

Recommended row attributes:

- data-node-id
- data-depth
- data-kind
- data-selected
- data-expanded
- data-expandable
- data-focused
- data-child-readiness
- data-ambient-status
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
- focused projection;
- selected/active projection;
- child-readiness projection;
- ambient status projection;
- last-in-group metadata for future D&D.

Tree controller owns:

- row activation;
- selection requests;
- expansion/reveal requests;
- keyboard focus movement;
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

The icon communicates structural/media meaning and routes to row-body selection for local hierarchy rows. Expansion is
row/disclosure state.

### Always-on status LEDs

Status dots on every row create noise and dilute the meaning of real readiness, warning, and failure states. No status
indicator means normal.

### Focus equals selection

Keyboard focus and selected browse scope are separate. A focus ring must not replace selected state, and selected state
must not masquerade as focus.

### Drag hover selects or refreshes contents

Drag hover is future drag-session geometry. It must not select a row, change keyboard focus, or refresh the contents
pane.

## Implementation Sequence

### Phase 1: Fix row action surface

- Make the row action full-width.
- Remove dead pointer areas.
- Keep one roving focus target per row.
- Keep ARIA tree semantics valid.
- Ensure icon/label/empty-space clicks route to row action.
- Keep selected and focus visuals separate.
- Keep status indicators sparse and non-focusable.
- Keep leaf alignment without fake disclosure.

### Phase 2: Implement and test local hierarchy activation

Implement the locked policy:

- row body selects only;
- icon, label, metadata, and empty right-side row area select only;
- status slot does not accidentally select/reveal/focus;
- reveal lane reveals/collapses only;
- ArrowRight/ArrowLeft reveal/collapse deterministically;
- Up/Down move focus without changing selected scope;
- Enter selects focused row;
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
- focus ring covers the row and is visually distinct from selected state;
- hover and selected background cover the row;
- leaf rows do not show fake disabled disclosure;
- ArrowRight and ArrowLeft keep deterministic tree behavior;
- ARIA tree semantics remain valid;
- no local filesystem drag-and-drop behavior is introduced;
- no always-on row status-dot pattern is introduced;
- no old row implementation remains as a compatibility path.

The future D&D work is acceptable only when:

- it is scoped to crates/playlists or other authored organization rows;
- it exposes exactly one candidate intent at a time;
- it supports ancestor-level reparent zones for last-in-group rows;
- invalid targets reject explicitly;
- drop zones are not focusable;
- drag-hover auto-expand never selects or refreshes contents;
- drag-opened branches are session-scoped unless a successful drop or explicit user action commits them;
- domain validation owns final legality.

## Source Guards

After row-surface implementation, search for and reject:

- focusable disclosure button in tree rows;
- role button on disclosure affordance;
- pointer-blocking layout spans inside the row;
- row click handlers split across unrelated child controls;
- row-body click that both selects and reveals;
- icon click that expands local hierarchy rows;
- double-click as the primary reveal/collapse path;
- always-on status dots on normal rows;
- focus styling that is indistinguishable from selected styling;
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
- tests require weakening keyboard navigation;
- focus movement starts changing selected browse scope implicitly;
- visual status indicators start deciding substrate facts.

## Final Product Statement

Dekzer local hierarchy rows are full-width browse-scope selection surfaces with a distinct, forgiving reveal lane. They
are not tiny chevron targets, and they are not row-click expansion surfaces. The icon and label select; the reveal lane
reveals; status hints stay sparse and informational; keyboard focus remains distinct from selected browse scope. Crate
and playlist rows may later become authored drag-and-drop surfaces with multi-dimensional reparent geometry. These two
interaction classes share a row foundation but must not be confused.
