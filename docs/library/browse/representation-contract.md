---

status: accepted
doctrine-version: 0.1
last-reviewed: 2026-06-10
owner: library-browser-architecture
canonical-context:

* product-doctrine
* prepared-room-model
* source-hierarchy-contract
* browse-policy-and-classification
* tree-contract
* row-action-and-dnd-scope-contract
  scope:
* library-browser-representation-model
* representation-ownership
* repeated-material-semantics
* row-capability-model
* authored-organization-boundaries
* external-library-provenance

---

# Library Browser Representation Contract

## Purpose

This document defines the canonical representation model for the Dekzer library browser.

The library is not a local file tree. Local Files is one representation inside the library, and it is the rawest one.
Dekzer also needs collection projections, crates, playlists, smart lists, external library adapters, history, sleeves,
Prepared Room objects, and future performance workflow surfaces.

This contract prevents a common architectural collapse:

Library equals Local Files.

That collapse is false.

The library is the product domain where musical material can be found, inspected, organized, prepared, referenced,
performed, and revisited through multiple representation classes.

The same physical file or musical material may appear in many places. Each appearance must make clear:

- why it exists there;
- who owns that representation;
- what relationship it has to the underlying material;
- what actions are legal in that context.

## Product Law

Dekzer’s library is a multi-representation browser over shared musical material.

Local Files is the raw source representation, governed by filesystem structure plus Dekzer browse policy.

Crates, playlists, sleeves, and Prepared Room structures are authored representations, governed by user-owned
membership, grouping, and order.

Smart lists and search views are generated representations, governed by definitions, filters, and query semantics.

External library rows are provenance-aware representations. A Serato crate, Traktor playlist, or Rekordbox-style
collection shown in Dekzer is not automatically Dekzer-owned.

History and performance rows are record representations. They may be browsed, annotated, copied, or forked, but their
source record is not an ordinary playlist.

The UI must expose each representation’s ownership and allowed actions so repeated appearances of the same material are
explainable, not confusing.

## Core Distinction

A visible row is not always “a track” in the same sense.

A row may represent:

- a source or inventory occurrence;
- a canonical track or material projection;
- a playlist membership;
- a crate membership;
- a smart-list result;
- a search result;
- an external-library reference;
- a history or performance reference;
- a preparation or workflow-object reference.

These are different representation classes, not accidental duplicates.

The same physical file may appear under Local Files because it exists at a path, in Collection because Dekzer admitted
it as musical material, in a crate because the user added a membership reference, in a smart list because it matches a
rule, in history because it was played, and in an external crate because another application references it.

The UI must make those meanings visible.

## Representation Classes

### 1. Raw Source Representations

Examples:

- Local Files;
- local source root;
- filesystem folder;
- system Music folder;
- system Videos folder;
- mounted drive source;
- removable source;
- network source;
- cloud-backed local folder;
- protected or permission-limited folder.

Owner:

- filesystem structure;
- source admission policy;
- scan and discovery state;
- Dekzer media relevance rules;
- active browse policy;
- active filters and sorts.

User can:

- register a source;
- unregister a source;
- scan or rescan;
- expand and collapse folders;
- select a folder or source as a browse scope;
- sort and filter table contents;
- inspect source availability;
- inspect blocked, failed, pending, or degraded source state;
- later apply explicit Dekzer hide/exclude policy if that product model is designed.

User cannot:

- reorder filesystem folders;
- drag folders into new filesystem positions;
- treat raw source hierarchy as an authored playlist;
- manually reorder source table contents as durable user order;
- edit playlist-like membership inside Local Files.

Local Files is raw, literal, and externally governed. Dekzer presents it through product policy, but Dekzer does not
pretend the user owns the filesystem’s order.

### 2. Collection Projections

Examples:

- all tracks;
- albums;
- artists;
- genres;
- labels;
- years;
- keys;
- BPM ranges;
- canonical track views;
- attachment inventory summaries;
- duplicate or identity review projections;
- preparation-readiness projections.

Owner:

- Dekzer’s canonical material model;
- metadata and analysis;
- projection definitions;
- sort and grouping policy;
- identity and evidence contracts where relevant.

User can:

- browse;
- filter;
- sort;
- inspect;
- edit allowed metadata when supported;
- add material to authored representations such as crates and playlists;
- resolve identity or duplicate questions where supported;
- start preparation or review workflows.

User usually cannot:

- manually reorder the projection as a durable sequence;
- reparent projection groups;
- treat generated grouping as authored hierarchy.

A collection projection is not a playlist. It is a view over admitted musical material.

### 3. Authored Organization Representations

Examples:

- crates;
- playlists;
- playlist folders;
- crate folders;
- sleeves;
- prepared-room groups;
- set-planning containers;
- user-authored route or transition containers later.

Owner:

- the user;
- Dekzer’s authored organization model;
- membership records;
- manual order;
- grouping and folder structure;
- user edits.

User can:

- create;
- rename;
- delete;
- duplicate;
- add material;
- remove membership;
- reorder;
- reparent;
- group;
- drag between authored containers;
- snapshot from generated views into authored containers.

Important:

The user edits the membership/reference and order inside the authored representation. That does not necessarily move the
physical file or change canonical track identity.

Moving a track in a playlist changes playlist membership order. It does not move the file.

### 4. Generated Query Representations

Examples:

- smart lists;
- search results;
- recently added;
- missing files;
- unprepared tracks;
- tracks needing beatgrid review;
- tracks with missing artwork;
- energy, key, BPM, genre, or readiness filters;
- preparation-state views.

Owner:

- query definition;
- filter definition;
- generated projection logic;
- sort policy;
- current library state.

User can:

- create and edit smart-list definitions;
- rename smart lists;
- delete smart-list definitions;
- pin or organize smart-list definitions;
- adjust filters and default sort;
- add generated results into authored representations;
- snapshot or freeze results into crates, playlists, sleeves, or Prepared Room objects when supported.

User cannot by default:

- manually reorder live generated results as if they were an authored sequence;
- reparent query result rows;
- persist arbitrary manual ordering inside a live query without an explicit overlay model.

Smart lists are authored definitions with generated contents.

The user owns the rule. The user does not automatically own the live result order.

Manual ordering inside a smart list requires a designed model, such as:

- a snapshot;
- a pinned exception layer;
- a manual priority overlay;
- conversion into a crate or playlist.

Without one of those models, drag-reordering a smart-list result is a semantic lie.

### 5. External Library Adapter Representations

Examples:

- Serato crates;
- Traktor playlists;
- Rekordbox-style collections;
- Engine or other DJ app libraries;
- imported or linked preparation setups;
- future adapter surfaces for other music libraries.

Owner:

- external application or source adapter;
- Dekzer provenance model;
- import or mirror policy if the user chooses one.

User can initially:

- browse;
- inspect provenance;
- add referenced material to Dekzer-owned authored representations;
- import, copy, or mirror only through explicit product flow.

User cannot by default:

- mutate the external source as if Dekzer owns it;
- reorder external crates unless Dekzer has imported or mirrored them into a Dekzer-owned representation;
- hide provenance;
- confuse an external object with a Dekzer-native authored object.

External rows may look similar to Dekzer rows, but they do not have the same ownership.

The UI must distinguish:

- read-only external reference;
- imported copy;
- synchronized mirror;
- Dekzer-native authored object.

Edit rights and drag-and-drop behavior depend on that ownership state.

### 6. History and Performance Representations

Examples:

- play history;
- performed sets;
- live path;
- shadow paths;
- route history;
- post-set review;
- performance incident records;
- runtime diagnostic records tied to a performance.

Owner:

- performance record;
- runtime and performance event model;
- user annotations where supported;
- immutable or append-only record rules where appropriate.

User can:

- browse;
- annotate;
- inspect context;
- fork into new authored plans;
- add material to crates or playlists;
- copy a performed sequence into an authored object;
- attach notes or review decisions where supported.

User usually cannot:

- arbitrarily reorder history as if it never happened;
- erase causality without explicit destructive edit semantics;
- treat a source performance record as a normal playlist.

History is not a normal playlist.

It may be copied, annotated, forked, or converted, but its source record semantics differ from authored organization.

### 7. Prepared Room and Workflow Representations

Examples:

- Prepared Room;
- sleeves;
- nearby reserve;
- prepared crates;
- hot table;
- live path;
- shadow paths;
- planned transitions;
- preparation zones;
- post-performance room state.

Owner:

- user-authored workflow model;
- preparation state;
- performance plan and post-performance record;
- Dekzer workflow contracts.

User can:

- create;
- edit;
- arrange;
- group;
- move material between zones;
- preserve performed state;
- revisit before, during, and after states when supported;
- fork future plans from prior performance rooms.

Prepared Room objects are not just playlists with better names.

They are workflow representations with stronger semantics around:

- readiness;
- proximity;
- alternatives;
- reserve depth;
- live commitment;
- performance memory;
- post-set review.

A Prepared Room that has been performed on may preserve the prepared state, performed set, runtime timeline, decisions,
Live Path, Shadow Paths, transitions, incident traces, and post-set history. That object must not collapse into a flat
playlist.

## Ordering Ownership Matrix

| Representation                | Ordering owner                               | User CRUD                             | Manual reorder          | D&D reparent             |
| ----------------------------- | -------------------------------------------- | ------------------------------------- | ----------------------- | ------------------------ |
| Local Files                   | filesystem and Dekzer browse policy          | source admission only                 | no                      | no                       |
| Source table contents         | sort/filter policy                           | no membership CRUD                    | no durable manual order | no                       |
| Collection projections        | Dekzer projection/sort policy                | metadata/prep actions where supported | usually no              | no                       |
| Crates                        | user-authored order                          | yes                                   | yes                     | yes, when hierarchical   |
| Playlists                     | user-authored order                          | yes                                   | yes                     | yes, when folders exist  |
| Playlist/crate folders        | user-authored hierarchy                      | yes                                   | yes                     | yes                      |
| Smart lists                   | user-authored definition, generated contents | definition CRUD                       | not by default          | not by default           |
| Search results                | query                                        | no                                    | no                      | no                       |
| External app crates/playlists | external/provenance owner                    | browse/import/mirror only             | not until Dekzer-owned  | not until Dekzer-owned   |
| History/performed sets        | performance record                           | annotate/fork/copy                    | not as source record    | different rules          |
| Prepared Room/sleeves         | user-authored workflow object                | yes                                   | yes                     | yes, but domain-specific |

## Capability Model

Every browser row should expose a representation kind and capability profile.

Example capabilities:

- canSelect;
- canExpand;
- canReveal;
- canCreateChild;
- canRename;
- canDelete;
- canAddMembership;
- canRemoveMembership;
- canReorder;
- canReparent;
- canEditDefinition;
- canSnapshot;
- canImport;
- canInspectProvenance;
- isReadOnlyExternal;
- isGeneratedResult;
- isRawSource;
- isAuthoredObject;
- isHistoryRecord;
- isWorkflowObject.

The UI must not infer allowed actions from visual style alone.

Representation kind and capability profile decide what the user can do.

A row being in a tree does not mean it is draggable.

A row being expandable does not mean it is authored.

A row showing a track title does not mean the row is the canonical track itself.

A row that references a file does not mean the action should move that file.

## Repeated Material Rule

The same physical file or canonical track may appear in many representations.

Examples:

- A file appears under Local Files because it exists at a filesystem path.
- The same material appears under Collection because Dekzer admitted it into the library material model.
- The same material appears in a crate because the user added a membership reference.
- The same material appears in a playlist because the user added a membership reference with sequence order.
- The same material appears in a smart list because it matches a rule.
- The same material appears in search results because it matches a query.
- The same material appears in history because it was played.
- The same material appears in an external crate because another application references it.
- The same material appears in a Prepared Room because it was staged for a workflow.

These rows should not look like accidental duplication.

The UI should make representation context visible through:

- section labels;
- representation icons;
- provenance indicators;
- detail columns;
- context menu grouping;
- absent commands;
- inspector/details surfaces;
- empty-state copy;
- row capability metadata.

## Local Files Rule

Local Files is not the library. It is one representation inside the library.

Local Files may show:

- filesystem folders;
- displayable media descendants;
- source availability state;
- scan coverage state;
- file/folder relevance under active policy;
- table contents filtered by media relevance and browse policy.

Local Files must not expose:

- reorder;
- reparent;
- manual ordering;
- playlist-like membership editing;
- authored hierarchy operations.

Local Files is allowed to be fast, musical, filtered, and useful. It is not allowed to pretend the filesystem is a
user-authored DJ structure.

## Authored List Rule

Crates, playlists, sleeves, Prepared Room lists, and similar objects are authored representations.

They may expose:

- create;
- rename;
- delete;
- duplicate;
- add membership;
- remove membership;
- reorder;
- reparent;
- drag-and-drop;
- grouping;
- snapshot/import from generated views.

Authored list operations mutate the authored representation. They do not move source files unless an explicit
file-management operation exists.

## Smart List Rule

Smart lists are authored definitions with generated contents.

The user can CRUD the smart-list object and definition. The result set is generated.

Smart-list result rows are not manually reorderable unless Dekzer introduces an explicit model for:

- snapshots;
- pinned exceptions;
- manual priority overlays;
- conversion into authored objects.

A smart list can produce a crate or playlist snapshot. That snapshot is authored and reorderable. The live smart list
remains generated.

## External Adapter Rule

External library representations must preserve provenance.

A Serato crate, Traktor playlist, or Rekordbox-style collection shown in Dekzer is not automatically a Dekzer-owned
crate or playlist.

The UI must distinguish:

- read-only external reference;
- imported copy;
- synchronized mirror;
- Dekzer-native authored object.

Edit rights and drag-and-drop behavior depend on ownership state.

An external adapter row may support “import into Dekzer” or “copy to crate” without supporting direct mutation of the
external source.

## History and Performance Rule

History and performed sets are records.

They may support:

- browsing;
- annotation;
- copy;
- fork;
- export;
- conversion into an authored playlist or room;
- post-performance review.

They must not silently become ordinary playlists.

A performed order is a record of what happened. Editing a derived copy is different from rewriting the source record.

## Prepared Room and Workflow Rule

Prepared Room, sleeves, nearby reserve, prepared crates, hot table, live path, shadow paths, and preparation zones are
workflow representations.

They carry product semantics beyond list membership:

- readiness;
- proximity;
- reserve depth;
- alternatives;
- commitment;
- performance state;
- post-set memory.

Prepared Room and sleeve rows may support authored ordering and drag-and-drop, but their operations must preserve
workflow meaning. Do not reduce them to generic playlist folders unless the product explicitly defines a playlist
projection of them.

## Relationship To Existing Docs

This contract owns the umbrella representation model.

It does not replace narrower contracts.

Source hierarchy contracts still own:

- local source tree containment;
- selectable folders;
- disclosure rules;
- exclusion of file rows from the tree.

Browse policy and classification still own:

- file classification;
- row universes;
- content predicates;
- facets;
- filter-aware content projection.

Tree row action contracts still own:

- HTML row surface;
- pointer hit zones;
- focus ownership;
- selection/reveal separation;
- local hierarchy versus future authored drag scope.

Prepared Room docs still own:

- Prepared Room domain model;
- workflow stack;
- performance memory;
- room-specific laws.

This document sits above those contracts and tells them what kind of representation they are part of.

## UI Requirements

Every library view should answer three questions:

1. What kind of representation am I looking at?
2. What is this row’s relationship to the actual media?
3. What actions are legal here?

A UI surface may answer these through:

- section labels;
- row icons;
- badges;
- provenance chips;
- table columns;
- context menus;
- inspector panes;
- disabled or absent commands;
- empty-state text;
- breadcrumb language;
- details and source panels.

If the same material appears twice, the user should be able to understand why.

## Rejection Cases

### Treating Local Files as the whole library

Local Files is one representation. It must not own the meaning of crates, playlists, smart lists, collection views,
external adapters, history, or Prepared Room.

### Manual reorder in raw source views

Raw filesystem hierarchy and source table contents are not authored sequences.

### Smart-list result reordering without an overlay model

The user owns the definition. The live results are generated.

### External adapter mutation without ownership

Do not edit external app structures unless the product has an explicit import, mirror, or writeback model.

### Duplicate-looking rows without representation context

Repeated material must be explainable by representation context.

### One generic row action model for everything

Allowed actions must come from representation kind and capabilities.

### Vague Collection authority

Do not use Collection as a bucket that hides ownership. Collection projections must say whether they project canonical
material, source occurrence, metadata, preparation state, identity review, or another defined model.

### Reducing workflow objects to playlists

Prepared Room, sleeves, routes, Live Path, and Shadow Paths may expose list-like behavior, but they are not
automatically playlists.

### Treating history as editable authored order

History is a record. A copied or forked history-derived object may be authored. The source record remains a record.

## Implementation Notes for Library V0

The first implementation may only support a subset of these representations.

For current Library V0:

- Local Files/source hierarchy is the primary implemented representation.
- Contents table applies media relevance and browse policy.
- Source rows and directory rows are raw source browse scopes.
- Contents rows are scoped source-file browse rows until a richer read model explicitly defines otherwise.
- Crates, playlists, smart lists, external adapters, and Prepared Room rows may remain future scope.
- Row and projection naming should leave room for typed representation kinds.
- Capability metadata should be shaped so authored representations can be added without rewriting the local source
  model.

Do not add fake authored behavior before the substrate supports it.

Do not let the V0 implementation hard-code “library equals local filesystem.”

## Acceptance Criteria

A library representation model is acceptable when:

- every tree/list row concept has an explicit representation kind;
- Local Files is modeled as raw source representation, not the whole library;
- authored representations can own order and membership;
- generated representations can own definitions without pretending live result rows are manually ordered;
- external representations preserve provenance and editability state;
- repeated appearances of material are explainable;
- allowed actions are derived from representation kind and capabilities;
- local source rows do not expose reorder or reparent behavior;
- crates, playlists, sleeves, and Prepared Room rows can later expose authored drag-and-drop without changing Local
  Files semantics;
- history/performance records remain distinguishable from editable authored copies;
- Prepared Room and workflow objects retain workflow semantics instead of collapsing into playlist semantics.

## Final Statement

Dekzer’s library is not a file tree and not a flat track list. It is a set of typed representations over shared musical
material.

Local Files shows raw source reality.

Collection views show canonical material projections.

Crates and playlists show user-authored organization.

Smart lists show generated results from user-owned definitions.

External adapters show provenance-aware references.

History surfaces show performance records.

Prepared Room surfaces show workflow memory, readiness, alternatives, commitment, and post-performance state.

The UI must make those meanings visible so repeated appearances of the same musical material feel powerful instead of
confusing.
