---
status: draft
doctrine-version: 0.1
last-reviewed: 2026-06-09
owner: library-browser-architecture
canonical-context:
  - product-doctrine
  - first-slice-substrate-map
  - source-hierarchy-contract
  - browse-policy-and-classification
  - tree-contract
  - library-browser-representation-contract
  - row-action-and-dnd-scope-contract
scope:
  - source-activation
  - navigation-readiness
  - child-readiness
  - contents-coverage
  - retained-browser-state
  - no-false-empty-states
---

# Source Activation and Navigation Readiness

## Purpose

This document defines the readiness behavior that makes Dekzer’s Local Files browser usable before a full recursive scan
completes.

This is not A-4 collection health.

This is the missing bridge between the current library substrate and the professional browser feel Dekzer needs.

The core principle:

A source becomes browsable before it becomes fully scanned.

A folder becomes selectable before it becomes fully classified.

A branch becomes expandable only when child-readiness justifies it.

A scope becomes empty only when coverage proves it.

This contract exists because a DJ browser must feel honest under incomplete knowledge. A short explicit readiness state
is acceptable. A false empty, disappearing branch, stolen selection, or twitchy tree is not.

## Product Law

Scan is not user intent.

A scan may discover data, refresh facts, publish progress, and invalidate affected projections. It must never take
ownership of selection, expansion, scroll, visible focus, or user navigation.

Disclosure is not selection.

The row body selects a browse scope. The reveal lane expands or collapses a branch. These paths must remain separate in
pointer handling, keyboard handling, projection, and tests.

Recursive scan is not the gate for basic browsing.

A source root and its immediate child directories need a bounded activation/readiness path that can make the source
browsable before recursive classification finishes.

Empty is a proven state.

“No Songs” or equivalent empty copy is only valid when the requested scope, recursion mode, and active browse policy
have sufficient coverage to prove that no matching rows exist.

Absence of rows is not proof of emptiness.

## Why This Exists

Professional DJ software often fails in small but damaging ways:

- a folder appears empty while discovery is incomplete;
- a source cannot be meaningfully browsed until too much scanning finishes;
- child branches appear one by one, causing visual twitch;
- background activity changes what the user selected;
- expansion and selection compete for the same gesture;
- the UI hides whether it is blocked, probing, stale, incomplete, or actually empty.

Dekzer should be stricter.

The browser must expose readiness honestly instead of pretending the filesystem and media inventory are fully known.

## Relationship To Representation Contract

[The Library Browser Representation Contract](../library/browse/representation-contract.md) owns the umbrella model for library representations.

This document owns readiness behavior for the Local Files/raw source representation.

Local Files is a raw source representation. It is not the whole library. It may show filesystem hierarchy,
media-relevant descendants, source availability, scan coverage, file/folder relevance, and readiness state.

This document defines how Local Files becomes selectable, expandable, and meaningfully browsable while source discovery
is incomplete.

It does not define crates, playlists, smart lists, external adapters, history, or Prepared Room semantics.

## Readiness Model

A source, branch, or contents scope may be in one of these readiness classes.

### Unknown

The system has not yet attempted to establish the necessary fact.

Unknown must not be rendered as leaf, empty, or complete.

### Probing

A bounded read or activation check is in progress.

The renderer may show a pending/probing affordance, but it must retain accepted prior state when available.

### Ready

The bounded readiness requirement for the current operation has succeeded.

Ready does not mean fully scanned.

A source may be navigation-ready while recursive scan is still running.

A branch may be child-ready while descendant media classification is incomplete.

A contents scope may be selectable while recursive contents coverage is incomplete.

### Incomplete

The system has enough information to show some useful state, but not enough to prove completeness.

Incomplete must not produce verified empty copy.

### Blocked

The system could not read the source, folder, or scope because of permissions, unavailable mount, missing path, or
equivalent access condition.

Blocked is distinct from empty.

### Failed

The system attempted the readiness operation and failed due to an internal or operational error.

Failed is distinct from leaf, empty, blocked, and unknown.

### Complete

The relevant coverage for the requested operation is complete.

Only complete coverage may support verified empty states.

## Source Activation

Source activation is the bounded path that makes a registered source navigable before recursive scan completion.

Activation may establish:

- source exists;
- source is reachable;
- root path is readable;
- first navigation window can be read;
- basic child-directory facts are available;
- source should be shown as blocked or failed if activation cannot proceed.

Activation must not require:

- recursive scan completion;
- full descendant classification;
- all media rows to be indexed;
- all metadata/enrichment to finish;
- album/artist/genre projections to exist.

A newly registered source may become visible and selectable as soon as source activation proves enough for navigation.

If activation is pending, the source may appear with a probing state.

If activation is blocked or failed, the source must remain explainable instead of disappearing or appearing empty.

## Navigation Readiness

Navigation readiness is the ability to show and interact with source and folder rows as browse scopes.

A navigation-ready row may be:

- selected;
- inspected;
- refreshed;
- expanded if child-readiness allows;
- shown with incomplete scan coverage;
- shown with blocked or failed access state.

Navigation readiness must not imply full media coverage.

A folder can be selectable before Dekzer knows every descendant track.

Selecting a folder establishes user intent. Later scan events must not override that selection.

## Child Readiness

Child readiness answers whether expanding a branch is currently valid and what child state can be shown.

A branch must not become expandable merely because the renderer guesses that children may exist.

A branch must not become a leaf merely because children are not yet known.

Child-readiness states should distinguish:

- known leaf;
- unloaded but child-capable;
- probing;
- loaded;
- retained while refresh is pending;
- blocked;
- failed;
- incomplete.

The first accepted child window should commit atomically.

Do not render child folders one by one during the initial child-readiness window. The user should see either the
previous accepted state, a clear pending/probing state, or the first accepted child set.

## Contents Readiness and Coverage

Contents readiness answers whether the selected scope can produce a meaningful table projection.

Contents coverage answers whether absence of rows is enough to prove emptiness.

Contents projection must distinguish:

- no accepted rows yet and pending;
- accepted retained rows while refresh is pending;
- accepted rows with incomplete coverage;
- accepted rows with complete coverage;
- verified empty;
- blocked;
- failed.

“No Songs” is only valid for verified empty.

A selected source or folder with incomplete recursive audio coverage must not show false empty copy.

If the active policy is audio browse with recursive scope, verified empty requires enough recursive coverage to prove
that no matching audio rows exist under that scope.

## Retention Rule

Pending work must not wipe accepted state.

If a branch has accepted children and a refresh begins, keep the accepted children visible as retained stale-safe state
until a replacement result is accepted or a terminal blocked/failed state requires a different projection.

If contents have accepted rows and a refresh begins for the same logical scope and policy, keep the accepted rows
visible as retained stale-safe state until a replacement result is accepted.

Retention does not mean pretending state is fresh.

The projection should be able to express retained, pending, incomplete, or stale-safe status.

## No False Empty Rule

A scope is verified empty only when all of these are true:

- the requested scope identity is known;
- the requested browse policy is known;
- the requested recursion mode is known;
- the relevant source/folder access is not blocked;
- the read did not fail;
- coverage is complete enough for the requested policy;
- the accepted result contains no matching rows.

If any of these are false, the state is not verified empty.

Unknown, probing, incomplete, blocked, failed, cursor-invalid, stale, and retained-pending states must not collapse into
empty.

## Intent Safety

The following events are not user intent:

- scan started;
- scan progressed;
- scan completed;
- scan cancelled;
- scan failed;
- source lifecycle refresh;
- invalidation replay;
- background child refresh;
- background contents refresh;
- event pump gap recovery.

These events may refresh loaded or visible projections. They must not select a different source, expand a branch,
collapse a branch, scroll a table, or move focus.

Only explicit user actions may mutate selection, expansion, scroll, or focus.

Examples of explicit user actions:

- selecting a source row;
- selecting a folder row;
- activating reveal lane;
- pressing supported tree expansion keys;
- invoking a retry command;
- invoking refresh/rescan commands;
- choosing a browse policy/filter.

## Disclosure and Selection

Disclosure and selection are separate contracts.

Row body:

- selects the represented browse scope;
- does not expand or collapse unless a specific keyboard convention says otherwise.

Reveal lane:

- expands or collapses a branch;
- does not select the row unless explicitly designed and tested as a separate behavior.

Keyboard behavior must preserve this distinction.

ArrowRight and ArrowLeft may operate disclosure according to tree rules.

Enter may select/activate according to row rules.

Space must not be stolen for tree selection if it conflicts with transport or broader DJ workflow expectations.

## Renderer Responsibility

The renderer projects accepted substrate state.

The renderer may represent:

- unknown;
- probing;
- ready;
- incomplete;
- retained;
- stale-safe;
- blocked;
- failed;
- verified empty;
- selected;
- expanded;
- unloaded;
- loading.

The renderer must not invent:

- filesystem completeness;
- media coverage;
- descendant existence;
- verified empty;
- source availability;
- scan completion;
- authored ownership.

The renderer may hold interaction state such as selected row, expanded rows, focus, and visible scroll position.
Background source and scan activity must not own those states.

## Backend Responsibility

The backend owns source, filesystem, inventory, and coverage facts.

Where the backend cannot prove a fact, it must expose an honest state rather than relying on row absence.

Backend read models should provide enough information for the renderer to distinguish:

- blocked from empty;
- failed from empty;
- incomplete from complete;
- unknown from leaf;
- probing/pending from accepted;
- source navigation readiness from scan completion.

## Rejection Cases

### Gating source browsing on recursive scan completion

A source must be able to become browsable before recursive scan completion.

### Treating absent rows as empty

No rows is not the same as verified empty.

### Treating unknown children as leaf

A branch with unknown child-readiness must not be projected as a known leaf.

### Twitchy first child rendering

Initial child windows should commit atomically. Do not show folders appearing one by one during the first accepted child
read.

### Scan stealing user intent

Scan activity must not select, expand, collapse, scroll, or focus anything.

### Refresh wiping accepted state

Pending refresh must retain accepted tree and contents state where the logical scope remains valid.

### Hiding blocked state

Permission failures, unavailable mounts, and unreadable folders must be explainable.

### Renderer-only readiness lies

Do not fake readiness with timers, optimistic guesses, or presentation-only state.

### Collapsing all failures into generic loading

Blocked, failed, probing, incomplete, and empty are different states.

## Implementation Notes for Library V0

Library V0 may implement only the Local Files/raw source representation.

That is acceptable if the readiness model leaves room for future representation kinds.

Recommended V0 implementation shape:

- extend existing source lifecycle/readiness models rather than creating parallel state;
- keep source activation separate from recursive scan;
- expose navigation readiness and coverage status in read models;
- preserve retained rows during pending reads;
- teach tree projection not to collapse unknown child state into leaf;
- teach contents projection not to emit verified empty without coverage proof;
- keep selection/disclosure tests strict;
- add blocked/failed distinction where current contracts flatten it.

Do not add fake crates, playlists, smart lists, or Prepared Room rows as part of this sprint.

Do not hard-code “library equals filesystem.”

## Acceptance Criteria

This contract is implemented well enough when:

- a newly registered source can become navigation-ready before recursive scan completion;
- a source or folder can be selected while deeper classification is incomplete;
- branch expansion depends on child-readiness, not full scan completion;
- first child windows commit atomically;
- pending refresh/probe does not wipe accepted tree children;
- pending contents refresh does not wipe accepted contents rows for the same logical scope;
- contents projection distinguishes pending, retained, incomplete, blocked, failed, and verified empty;
- “No Songs” or equivalent empty copy appears only for verified empty;
- scan and lifecycle events do not steal selection, expansion, scroll, or focus;
- disclosure and selection remain separate in pointer and keyboard paths;
- renderer state remains projection/interaction state, not filesystem authority;
- backend contracts expose honest readiness and coverage state instead of making the renderer guess.

## Suggested First Implementation Slice

Implement the smallest vertical slice that proves the doctrine:

1. Source activation can mark a registered source as navigation-ready before recursive scan completion.
2. Tree projection can represent probing, retained, blocked, failed, loaded, unloaded, and known leaf without conflating
   them.
3. Contents projection can represent verified empty only when coverage is complete.
4. Tests prove scan/lifecycle/invalidation refresh does not mutate selection or expansion.

This is enough to unlock the next sprint safely.

A-4 collection health should come after this because collection health depends on the same honesty machinery: readiness,
coverage, retained reads, blocked states, failed states, and no false empty paths.

## Final Statement

Dekzer’s library browser must feel fast because it is honest, not because it pretends to know more than it does.

A source can be browsable before it is fully scanned.

A folder can be useful before all descendants are classified.

A branch can wait honestly before exposing children.

A scope can be empty only when coverage proves it.

That is the contract.
