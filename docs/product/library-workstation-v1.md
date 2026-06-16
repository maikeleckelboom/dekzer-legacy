# Library Workstation V1

Status: binding product target for the Library Workstation v1 shell refactor
Scope: Library surface regions, vocabulary, view targets, preview boundary, and workspace-host authority
Non-goal: this document does not authorize building shell components, inspector components, preview components, Column view, backend substrate changes, or workspace-host runtime imports.

## Product Target

Dekzer Library is a consumer-grade Library Workstation, not a two-pane file browser.

The canonical chain is:

```text
source origins
-> browse roots
-> active scope
-> projection
-> view mode
-> contents
-> primary selection
-> inspector/action target
-> preview slot / loaded slot
-> workspace region
```

## Regions

The v1 workstation surface has these binding regions:

- Top bar: owns Scope, Projection, View, Search, and Filters controls.
- Left Browse pane: owns browse roots and scope navigation rows. Tree rows are browse scopes, not track rows.
- Center Contents pane: owns result rows for the active scope, projection, view, search, filters, sort, and cursor.
- Right Inspector pane: reflects the primary selection as TRACK, SOURCE, SLOT, or WORKSPACE context.
- Bottom Preview lane: a single-slot collapsed Hot Table v0 lane for preview/load/clear dispatch.

Primary selection drives inspector and action target context. Tree selection and tree expansion remain independent.

## Views

Projection means result family or query interpretation. View means renderer form.

Legal v1 view vocabulary includes List, Columns, Tree, Covers, Grid, and Compact. List, Columns, Tree, and Covers are view modes. They are not projection modes.

List is the first v1 table target for Contents.

Columns is a first-class view over the existing representation, hierarchy, and read contracts. It may start narrow, but it must be architected as a real view mode, not a prototype-only side path.

## Preview Lane

The Preview lane is a single-slot Hot Table v0 expression. It is not a disposable preview widget.

Preview and load actions must route through the shared slot dispatch model. Library must not create a Library-only deck/load path.

## Compact Visual Acceptance

- Contents must remain the visually dominant Library Workstation surface.
- Empty or deferred metadata columns must not create visible table noise.
- Source and status actions should collapse or move to Inspector when they would dominate Contents.
- Preview lane stays compact when empty.
- Column View terminal content must use a compact terminal presentation, not the full v1 List table when width is constrained.

## Workspace-Host Boundary

Workspace topology may host the Library surface later. It may provide bounds, visibility, and viewport hints only.

Workspace topology must not own Library scope, rows, selection, inspector context, preview slot state, or membership semantics.

This pass must not import workspace-host code or archive-workspace-layout runtime.

## Rejection Cases

Reject any design or implementation that:

- puts tracks in the tree
- selects a fake source row for aggregate scopes
- lets a view mode change membership semantics
- shows a generic `Details` inspector context
- creates a second hierarchy model or source truth for Columns
- creates a Library-only deck/load path
- imports workspace-host or archive-workspace-layout runtime in this pass
