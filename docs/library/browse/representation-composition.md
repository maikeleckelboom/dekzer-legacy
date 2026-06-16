---
status: accepted
doctrine-version: 0.1
last-reviewed: 2026-06-09
owner: library-browser-architecture
canonical-context:
  - product-doctrine
  - library-browser-representation-contract
  - source-activation-and-navigation-readiness
  - first-slice-substrate-map
  - prepared-room-model
  - workspace-topology-model
  - row-action-and-dnd-scope-contract
scope:
  - library-representation-composition
  - representation-root-realization
  - unified-library-sidebar
  - browse-column-view
  - independent-library-panels
  - grouped-library-panels
  - topology-hosted-library-surfaces
  - panel-instance-state
  - composition-ownership
---

# Library Representation Composition Contract

## Purpose

This document defines how Dekzer library representation roots may be realized, grouped, split, and placed in the
workspace.

[The Library Browse Representation Contract](representation-contract.md) defines what library representations mean.

This document defines how those representations may appear.

The distinction matters.

Collection, Local Files, Playlists, Crates, Smart Lists, External Libraries, History, Prepared Rooms, Sleeves, and
Routes are not inherently sidebar items. They are library representation roots.

A representation root may appear:

- as an item in a unified Library sidebar;
- as its own workspace panel;
- as part of a grouped panel;
- as a focused preparation surface;
- as a compact navigation surface;
- as a topology-hosted surface inside a preparation or adjunct area.

The first visual placement of a representation root must not become its architecture.

This contract prevents a common implementation collapse:

Representation equals sidebar item.

That collapse is false.

A sidebar is one composition shell. It is not the owner of library meaning.

## Product Law

Library representation roots are workspace-capable surfaces.

They may be composed together or separated apart without changing their semantic ownership.

The same representation root can be realized in multiple ways:

- Local Files inside the default Library sidebar;
- Local Files as a standalone source-browsing panel;
- Crates beside Local Files in a preparation layout;
- History beside a Prepared Room during post-set review;
- External Libraries beside Collection during import or comparison;
- Smart Lists grouped with Crates during preparation;
- Prepared Rooms as a focused workspace surface.

Composition must not change meaning.

A Local Files row remains a raw source representation whether it appears in a sidebar, a standalone panel, a grouped
preparation panel, or a split layout.

A playlist membership remains an authored representation whether it appears in a compact sidebar, a full playlist panel,
or beside another playlist.

An external crate remains provenance-aware whether it appears under External Libraries, in an import panel, or beside a
Dekzer-native crate.

A history row remains a record representation whether it appears in a history panel, a review surface, or a Prepared
Room history view.

## Core Ownership Rule

Representation kind owns meaning.

Panel instance owns local view and session state.

Composition shell owns grouping and presentation arrangement.

Topology owns placement and size.

No layer may silently take ownership from another layer.

### Representation Kind Owns Meaning

A representation kind owns:

- what the row or root represents;
- who owns the representation;
- provenance;
- legal actions;
- drag-and-drop semantics;
- whether ordering is authored, generated, external, or recorded;
- whether rows are raw source occurrences, canonical material projections, memberships, generated results, external
  references, history records, or workflow objects.

Representation kind answers:

- What am I looking at?
- What relationship does this row have to the music?
- What actions are legal here?

### Panel Instance Owns Local View State

A panel instance owns the local state needed to render and operate one realization of a representation root.

Examples:

- selected scope within that panel;
- expanded rows within that panel;
- selected path within a column-browser realization;
- focused column within a column-browser realization;
- local scroll position;
- local column visibility;
- local sort selection where the representation allows it;
- local filter controls where the representation allows them;
- pending, retained, blocked, failed, stale-safe, or empty projection state;
- focused row within that panel;
- pagination state for that panel’s active read;
- panel-specific inspector visibility;
- panel-specific density or compactness.

Panel state is not durable substrate state.

Panel state is not musical ownership.

Panel state must not be confused with the underlying representation.

Two panels may show the same representation root with different local state.

Example:

One Local Files panel may be expanded into an external drive while another Local Files panel is focused on the system
Music folder.

One Crates panel may show crate folders while another panel focuses a specific crate’s contents.

One History panel may show a performed set list while another shows a performance review scope.

Those panel instances must not accidentally share scroll, selection, expansion, pagination, or focused row unless an
explicit linked-panel model is designed.

#### Browse Session Context

A panel instance owns or references a browse session context. This session context carries the operational parameters
that define what the panel renders and what local interaction state is active.

In V0 there may be one effective library browse session. Future multiple panels need independent:

- selected scope;
- active browse policy;
- expansion state;
- selected path for column-browser realizations;
- focused column for column-browser realizations;
- pagination state;
- scroll position;
- focused row;
- local filter, sort, and column state;
- pending, retained, and error projection state.

These must be per-panel unless an explicit linked-panel model is designed. Linked panels that intentionally share browse
session state must be explicit in both code and product behavior. Do not accidentally share session state because two
panels render the same representation root.

### Composition Shell Owns Grouping

A composition shell owns how representation roots and panel instances are arranged together inside a visible browser
surface.

Examples:

- unified Library sidebar;
- accordion group;
- tab group;
- stacked panel group;
- split browser group;
- pinned surface group;
- compact source group;
- preparation group;
- history/review group.

A composition shell may decide:

- which panel instances are visible;
- their order in the group;
- whether they are collapsed or expanded;
- whether they are shown as tabs, accordions, split panes, stacked sections, or column-browser panes;
- local group density;
- local group header labels;
- local group affordances such as pin, close, split, or open separately.

A composition shell must not decide:

- whether a row is reorderable;
- whether a row is authored;
- whether a row is external;
- whether a row is generated;
- whether a history row can be rewritten;
- whether Local Files can expose playlist-like membership editing;
- whether a Smart List result can be manually reordered;
- whether an external adapter row can be mutated;
- whether a Prepared Room row is just a playlist row.

Composition changes placement and grouping. It does not change representation semantics.

### Topology Owns Placement and Size

Topology owns where a panel or panel group is placed and how much spatial authority it receives.

Examples:

- preparation band;
- adjunct band;
- compact side surface;
- focused lower workspace;
- split preparation surface;
- performance-forward layout;
- preparation-forward layout.

Topology may decide:

- placement;
- size;
- ratio;
- minimum size;
- default target size;
- split orientation;
- host occupancy;
- whether a panel group is compact or expanded due to available space.

Topology must not decide:

- representation ownership;
- row capability;
- source availability;
- scan coverage;
- media classification;
- authored ordering;
- external provenance;
- history record mutability;
- Prepared Room workflow semantics.

Topology hosts surfaces. It does not own library meaning.

Persistent app chrome remains separate from authored workspace topology unless a surface is explicitly modeled as a
workspace-hosted working surface rather than always-present shell chrome.

## Vocabulary

### Representation Root

A representation root is a top-level library representation entry.

Examples:

- Collection;
- Local Files;
- Playlists;
- Crates;
- Smart Lists;
- External Libraries;
- History;
- Prepared Rooms;
- Sleeves;
- Routes.

A representation root owns or references a representation kind. It is not inherently a sidebar item, tab, panel, or
topology host.

### Representation Kind

A representation kind defines the semantic class of a row, root, or surface.

Examples:

- raw source representation;
- collection projection;
- authored organization;
- generated query representation;
- external library adapter representation;
- history or performance record;
- Prepared Room or workflow representation.

Representation kind owns legal action semantics.

### Panel Realization

A panel realization is one concrete visual and interaction form for a representation root.

Examples:

- compact tree panel;
- column browser panel;
- browser plus contents panel;
- contents-only panel;
- inspector-heavy panel;
- import comparison panel;
- history review panel;
- Prepared Room workspace panel;
- compact navigation strip.

A representation root may support multiple panel realizations.

### Panel Instance

A panel instance is a live occurrence of a panel realization in the workspace.

The same representation root may have multiple panel instances open at once.

Panel instances have local state.

### Composition Shell

A composition shell groups one or more panel instances.

Examples:

- unified Library sidebar;
- preparation panel group;
- source panel group;
- import/review group;
- tabbed group;
- split group.

A composition shell owns grouping and arrangement, not semantic meaning.

### Topology Host

A topology host is a workspace placement target.

It gives a panel or group spatial authority.

It does not define the panel’s musical meaning.

## Supported Realization Forms Over Time

These forms are architecture-supported over time. Not every form is a V0 implementation requirement.

### Unified Library Sidebar

A unified Library sidebar may contain multiple representation roots in one familiar surface.

Example:

Library

- Collection
- Local Files
- Playlists
- Crates
- Smart Lists
- External Libraries
- History
- Prepared Rooms / Sleeves / Routes

This is the simple default composition.

It is useful for first-run experience, compact layouts, and performance-forward layouts where preparation needs to stay
available but not dominate the workstation.

The unified sidebar is not the library model.

It is a composition shell over representation roots.

### Column View

Column view is v1-seeded as a first-class view mode. A representation root may support a column-browser realization.

A column view presents the current path as adjacent child-window columns. Each column represents one path segment and
projects the readable children for that segment.

Example:

Sources

- Local Files;
- External Libraries;
- Imports later.

Local Files

- Music;
- DJ Edits;
- Downloads;
- Recordings.

Music

- House;
- Techno;
- Minimal;
- Disco.

House

- Deep House;
- Tech House;
- Afro House;
- Progressive House.

Column view is useful for fast source digging, deep folder navigation, external drive browsing, import review, and
preparation workflows where sibling scopes need to remain visible.

It is a realization of the same representation and hierarchy contracts. It is not a second hierarchy model.
It must not create a second source truth.

Column view may start narrow in v1, but it must remain first-class in architecture.

For Local Files, Column view uses the same source readiness, child-readiness, hierarchy coverage, child-window,
retained-pending, blocked, failed, incomplete, and empty-state contracts as the tree realization.

Each column projects an accepted read result for one path segment. A column must not infer durable hierarchy observations from
an empty child array, a stale cache, a visible row count, or another panel’s projected rows.

Selecting a row in one column may open the next column and update the active contents scope for that panel instance.

Column selection must not mutate the representation root’s meaning.

Column selection must not mutate another panel instance unless an explicit linked-panel model is designed.

Column view may render counts, readiness, loading, retained, blocked, failed, incomplete, or empty states only from the
same read contracts that authorize those states in the tree realization.

Filter and browse-policy changes affect contents projection. They must not change column containment,
child-readiness, source structure, or leaf/disclosure meaning.

A composition shell may place a column browser beside a contents table, details panel, preparation panel, or import
comparison panel. The placement does not change source ownership, row capability, external provenance, or authored
semantics.

Column view is v1-seeded. It is not built by this contract change unless a sprint explicitly scopes implementation.

### Independent Workspace Panel

Any major representation root may be opened as an independent workspace panel.

Examples:

- Local Files as its own source browsing panel;
- Collection as its own canonical material panel;
- Crates as its own authored organization panel;
- History as its own performance record panel;
- External Libraries as its own adapter/provenance panel;
- Prepared Room as its own workflow panel.

Independent panels allow serious preparation work without forcing every task through one sidebar.

### Grouped Workspace Panel

Multiple related representation roots may be grouped into one panel group.

Examples:

Sources group:

- Local Files;
- External Libraries;
- Imports later.

Organization group:

- Playlists;
- Crates;
- Smart Lists.

Preparation group:

- Crates;
- Smart Lists;
- Prepared Rooms;
- Sleeves;
- Routes.

Review group:

- History;
- performed sets;
- incident traces later;
- post-set notes later.

The group owns composition. Each root keeps its representation semantics.

### Focused Preparation Surface

A representation root may take most or all of a preparation area.

Examples:

- Prepared Room occupies the full preparation band;
- Collection occupies the full lower workspace during tagging or review;
- History occupies the full preparation area during post-set analysis;
- External Libraries occupies the full area during import.

Focused mode changes spatial authority. It does not change ownership.

### Split and Compare Surface

Two or more representation roots may be opened side by side.

Examples:

- Local Files beside Collection;
- External Libraries beside Crates;
- History beside Prepared Room;
- Smart List beside a crate;
- two crates beside each other;
- playlist beside route later.

This is necessary for professional preparation workflows.

The split does not make the two surfaces share action semantics.

Drag-and-drop, copy, import, fork, snapshot, or membership operations must still come from source and target
representation capabilities.

### Compact Surface

A representation root may be shown in a compact form when the topology gives it little space.

Examples:

- compact Local Files source strip;
- compact Crates list;
- compact History list;
- compact Prepared Room selector.

Compact rendering may reduce visible columns or controls. It must not change ownership rules.

## Composition Examples

### Default First-Run Library

The default first-run Library surface may be one unified sidebar plus a contents pane.

It may show only the currently implemented Local Files representation in Library V0.

Even then, naming and architecture must not hard-code that the whole library is Local Files.

### Source Digging Layout

A preparation-forward layout may show:

- Local Files panel;
- External Libraries panel;
- contents/details panel.

This supports finding music from raw and external sources.

### Source Column Digging Layout

A preparation-forward layout may show:

- Local Files column browser;
- contents table;
- details panel;
- source readiness or import review surface.

This supports fast navigation through deep source hierarchies without making the tree realization the only serious
source-browsing surface.

The column browser and tree browser must remain different views over the same source/hierarchy read contracts.

### Organization Layout

A preparation-forward layout may show:

- Collection panel;
- Crates panel;
- Playlists panel;
- Smart Lists panel;
- contents/details panel.

This supports organizing admitted material.

### Performance Review Layout

A review layout may show:

- History panel;
- Prepared Room panel;
- performed set details;
- notes or incident surfaces later.

This supports post-set memory and future planning.

### Performance-Forward Layout

A performance-forward layout may show:

- decks and waveforms with most spatial authority;
- a compact library sidebar or compact preparation panel;
- hidden or collapsed secondary library surfaces.

This supports live performance without destroying preparation availability.

## Data and Read Boundaries

Composition does not replace read ownership.

A panel realization must use the read boundary owned by its representation.

Examples:

- Local Files tree uses hierarchy reads for child rows;
- Local Files column browser uses hierarchy reads for path-segment child windows;
- Local Files contents uses contents reads for selected scopes;
- Collection projections use collection/material projection reads when implemented;
- Crates and playlists use authored membership reads when implemented;
- Smart Lists use generated query reads when implemented;
- External Libraries use adapter/provenance reads when implemented;
- History uses performance record reads when implemented;
- Prepared Room uses workflow-object reads when implemented.

A composition shell must not synthesize representation data by scraping another panel’s rows.

A topology host must not synthesize representation data.

A panel may project accepted read results. It must not become the owner of the durable substrate.

## Action and Drag-and-Drop Rules

Actions come from representation kind and capability profile, not from placement.

A row being visible in a panel does not mean it is editable.

A row being inside a sidebar does not mean it is a navigation-only row.

A row being inside a large panel does not mean it is authored.

A row being in a split comparison does not mean it can be dragged into the other side.
