---

status: draft
doctrine-version: 0.1
last-reviewed: 2026-06-10
owner: library-browser-architecture
canonical-context:

* product-doctrine
* library-browser-representation-contract
* source-activation-and-navigation-readiness
* first-slice-substrate-map
* prepared-room-model
* workspace-topology-model
* row-action-and-dnd-scope-contract
  scope:
* library-representation-composition
* representation-root-realization
* unified-library-sidebar
* independent-library-panels
* grouped-library-panels
* topology-hosted-library-surfaces
* panel-instance-state
* composition-ownership

---

# Library Representation Composition Contract

## Purpose

This document defines how Dekzer library representation roots may be realized, grouped, split, and placed in the
workspace.

The Library Browser Representation Contract defines what library representations mean.

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
- whether they are shown as tabs, accordions, split panes, or stacked sections;
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

## Required Realization Forms

The architecture must allow the following realization forms over time.

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

A row being shown inside a Prepared Room-related group does not mean it has become a Prepared Room membership.

Drag-and-drop must ask:

- What is the source representation kind?
- What is the target representation kind?
- What source capability is present?
- What target capability is present?
- Is this an import, copy, membership add, reorder, reparent, fork, snapshot, route operation, or file operation?
- Does the operation mutate a Dekzer-owned representation, an external representation, a source record, or only create a
  new reference?

The composition shell cannot grant permissions.

The topology host cannot grant permissions.

Only representation capabilities and explicit product contracts can grant permissions.

## Local Files Composition Rule

Local Files may appear:

- inside the unified Library sidebar;
- as its own panel;
- as part of a Sources group;
- beside External Libraries;
- beside Collection;
- beside Crates or Playlists for copy/add workflows;
- compacted in performance-forward layouts.

Local Files remains raw source representation in all forms.

Local Files must not expose:

- authored order;
- playlist-like membership editing;
- drag reparenting of filesystem folders;
- manual ordering as durable user order;
- hidden conversion into a crate or playlist.

A composition shell may make Local Files convenient. It must not make Local Files authored.

## Collection Composition Rule

Collection may appear:

- inside the unified Library sidebar;
- as its own canonical material panel;
- beside Local Files;
- beside Crates or Playlists;
- beside Smart Lists;
- in a preparation-forward layout.

Collection remains a projection over admitted musical material.

Collection is not Local Files.

Collection is not a playlist.

Collection may later expose metadata, identity, duplicate, preparation, and review workflows according to its own
contracts.

## Authored Organization Composition Rule

Crates, playlists, sleeves, and other authored organization roots may appear:

- inside the unified Library sidebar;
- as independent panels;
- in grouped organization panels;
- beside Collection;
- beside Local Files;
- beside Smart Lists;
- inside preparation-forward layouts.

Authored organization representations may own membership and order where their domain contract allows it.

Their authored behavior does not leak into Local Files, Collection projections, Smart List results, External Libraries,
or History records.

## Generated Representation Composition Rule

Smart Lists and search-like generated representations may appear:

- inside the unified Library sidebar;
- as independent panels;
- grouped with Crates or Playlists;
- beside Collection;
- beside Prepared Room or Sleeves later.

The user may own the definition.

The live result set remains generated unless an explicit snapshot, priority overlay, pinned exception, or conversion
model exists.

Composition must not imply manual ordering of generated results.

## External Library Composition Rule

External Libraries may appear:

- inside the unified Library sidebar;
- as independent adapter panels;
- beside Local Files;
- beside Collection;
- beside Dekzer-owned Crates or Playlists;
- in import/review layouts.

External rows preserve provenance in all realization forms.

A Serato crate, Traktor playlist, or Rekordbox-style collection is not automatically Dekzer-owned because it appears
beside Dekzer-owned surfaces.

Composition may support import, copy, mirror, or inspect actions only through explicit product contracts.

## History Composition Rule

History may appear:

- inside the unified Library sidebar;
- as an independent performance record panel;
- beside Collection;
- beside Crates or Playlists;
- beside Prepared Room;
- in a post-set review layout.

History remains a record representation.

Composition may support annotation, copy, fork, conversion, and review workflows.

It must not silently turn the source history record into an editable playlist.

## Prepared Room, Sleeves, and Routes Composition Rule

Prepared Rooms, Sleeves, Routes, Live Path, Shadow Paths, and related workflow objects may appear:

- inside the unified Library sidebar;
- as independent workflow panels;
- grouped into preparation surfaces;
- beside Crates, Playlists, Smart Lists, Collection, or History;
- as focused preparation-forward surfaces;
- as future VR/AR or spatial projections over the same domain model.

These objects are workflow representations, not prettier playlists.

Composition must preserve semantics such as:

- readiness;
- proximity;
- reserve depth;
- alternatives;
- commitment;
- performed state;
- post-set memory;
- route or transition meaning.

A panel group may make these objects easier to use. It must not reduce them to generic list folders.

## Panel Identity

Every panel instance should have a stable identity separate from the representation root identity.

A representation root answers:

Which library representation is this?

A panel instance answers:

Which live realization of that representation is this?

This allows:

- multiple Local Files panels;
- multiple Crates panels;
- multiple History panels;
- a compact sidebar and a focused panel showing the same root;
- independent scroll and expansion state;
- explicit linked-panel behavior later if designed.

Panel identity must not become durable musical identity.

Closing a panel must not delete the representation root.

Rearranging a panel must not mutate the represented musical material.

## Composition Persistence

Workspace composition may be persisted as layout/session state.

Examples:

- which representation panels are open;
- where they are placed;
- grouped or split arrangement;
- compact or expanded state;
- panel-local session state where appropriate.

Persisted composition state must not be confused with durable library substrate state.

A saved workspace layout may remember that a Crates panel and Local Files panel were open side by side.

That does not change crate membership, file paths, source registration, or history records.

## Relationship To Existing Docs

The Library Browser Representation Contract owns representation semantics.

This document owns realization and composition rules for representation roots.

Source Activation and Navigation Readiness owns Local Files readiness behavior before recursive scan completion.

First Slice Substrate Map owns current first-slice scope and active read-boundary ownership.

Prepared Room docs own Prepared Room and workflow semantics.

Workspace topology docs own host topology, size negotiation, placement, and layout realization.

Row action and drag-and-drop contracts own pointer surfaces, action legality, and drag/drop behavior.

This document sits between library representation semantics and workspace topology realization.

It prevents layout placement from becoming domain meaning.

## Implementation Notes for Library V0

Library V0 may implement only Local Files/source hierarchy and contents.

That is acceptable.

But the implementation must not name or structure the browser as if Local Files is the whole library.

Prefer names that leave room for representation roots and panel realizations.

Good conceptual names:

- representation root;
- representation kind;
- browser surface;
- panel instance;
- composition shell;
- library panel;
- source browser;
- contents projection;
- capability profile.

Avoid names that imply permanent placement when the concept is not placement-owned.

Bad conceptual names:

- sidebar item as the only identity;
- folder browser as the whole library;
- source tree as the whole library;
- local files root as the library root;
- playlist panel as a separate snowflake architecture;
- crates sidebar as a unique subsystem unrelated to representation panels.

First implementation may expose Local Files in a unified sidebar.

That sidebar must be treated as one realization, not the library’s permanent architecture.

## Suggested Future Substrate Shape

A future library panel substrate should be able to describe a panel instance with at least:

- panel instance identity;
- representation root identity;
- representation kind;
- realization kind;
- current scope;
- capability profile;
- local view state;
- read model binding;
- composition group membership;
- topology host placement where applicable.

This is not a mandate to implement these fields now.

It is a direction marker to prevent hard-coding one sidebar-only model.

## Rejection Cases

### Treating Sidebar Placement As Domain Meaning

A representation root appearing in the sidebar does not make it a sidebar-only concept.

### Treating Panel Placement As Domain Meaning

A representation root appearing as an independent panel does not give it new semantic powers.

### Building One Snowflake Panel Per Representation

Do not create unrelated architectural systems for Local Files, Crates, Playlists, Smart Lists, History, and Prepared
Room when a shared representation-panel substrate can carry the common concerns.

### Letting Composition Grant Actions

A composition shell cannot grant reorder, reparent, import, delete, edit, fork, or snapshot behavior.

Actions must come from representation capabilities.

### Letting Topology Grant Actions

A topology host cannot make a raw source row authored, make a Smart List result reorderable, or make an external row
Dekzer-owned.

### Sharing Panel State Accidentally

Two panels showing the same representation root must not accidentally share scroll, focus, expansion, selection,
pagination, or local filters.

Any shared state must be explicit.

### Collapsing Representation Into One Library Tree

The library may be composed into one sidebar, but it must not become one generic tree where all roots share the same
semantics.

### Treating Prepared Room As Playlist Placement

A Prepared Room panel may appear beside playlists or crates. That does not make it a playlist.

### Treating External Libraries As Native Objects

An external library panel may appear beside Dekzer-native organization panels. That does not make external objects
Dekzer-owned.

### Treating History As Editable Authored Order

History may appear in a panel, group, or sidebar. It remains a record representation unless explicitly forked or copied
into an authored object.

### Making First-Slice Local Files The Permanent Model

Library V0 may begin with Local Files. It must not encode that the whole library is Local Files.

## Acceptance Criteria

The composition model is acceptable when:

- Collection, Local Files, Playlists, Crates, Smart Lists, External Libraries, History, Prepared Rooms, Sleeves, and
  Routes are treated as representation roots, not sidebar-only entries;
- a representation root can be realized inside a unified sidebar;
- a representation root can later be opened as an independent workspace panel;
- multiple representation roots can be grouped without merging their semantics;
- the same representation root can have multiple panel instances with independent local state;
- sidebar, panel, group, split, and topology host are realization forms, not domain owners;
- topology placement changes size and authority, not representation meaning;
- actions and drag/drop behavior come from representation capabilities, not placement;
- Local Files remains raw source representation in every realization;
- authored organization remains authored in every realization;
- generated views remain generated in every realization;
- external library rows preserve provenance in every realization;
- history rows remain record representations in every realization;
- Prepared Room and workflow objects preserve workflow semantics in every realization;
- Library V0 can remain narrow without hard-coding a sidebar-only future.

## Final Statement

Dekzer’s library representations must be composable.

They can come together into one Library sidebar.

They can split apart into separate workspace panels.

They can be grouped, focused, compacted, compared, and topology-hosted.

The composition changes how the user works with the representation.

It does not change what the representation means.

Representation kind owns meaning.

Panel instance owns local view and session state.

Composition shell owns grouping.

Topology owns placement and size.

That is the contract.
