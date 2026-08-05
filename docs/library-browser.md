# Library browser

The browser is the surface where Dekzer's honesty rules become visible. It has to feel fast while knowing that the
filesystem underneath it is only partially explored, and it has to do that without ever claiming more than it has
established.

This document owns the current browser model: scope, readiness, coverage, selection, refresh, and the boundary between
backend truth and renderer interaction state.

## Sources and browse scopes

A **local browse entry point** is a place worth looking, before anything is registered. On Windows these resolve from
known folders and from fixed and removable volumes. The default Music folder is one such entry point. Entry points and
their child items can be read and previewed without registering or scanning anything, and a browse item that matches an
already-admitted source is marked as such rather than offered again.

Browsing is not registration. Registration is not scanning. Keeping the three separate is what stops a user from
accidentally indexing a whole drive by looking at it.

A **source** is registered and durable. A **source location** is a registered subpath within one. A **directory** is a
scanned folder inside a source.

A **browse scope** is what the contents pane is currently showing, and it is one of exactly three things: a source, a
source location, or a directory. These stay distinct identity values even when they resolve to overlapping filesystem
prefixes, because selecting a source and selecting the directory at its root are different user intents that can
diverge later.

## Activation is not recursive scanning

Registering a source persists it and establishes a bounded first navigation window. The source and its immediate
folders become browsable as soon as that window is readable.

Activation establishes that the source exists, is reachable, its root is readable, and its first child window can be
read. It does not require recursive scan completion, full descendant classification, every media row indexed, or any
metadata enrichment.

Recursive discovery continues behind that. A source is navigable while it is still being scanned, and this is the
normal case rather than a transitional one. If activation cannot proceed, the source appears as blocked or failed, with
an explanation. It does not disappear and it does not appear empty.

## Readiness states

A source, branch, or scope is in one of these. They are not interchangeable and collapsing any pair of them into
"loading" or "empty" is the failure this model exists to prevent.

| State      | Meaning                                                                              |
| ---------- | ------------------------------------------------------------------------------------ |
| Unknown    | No attempt has been made yet. Not a leaf, not empty, not complete.                   |
| Probing    | A bounded read is in progress. Accepted prior state stays visible.                   |
| Ready      | The bounded requirement for this operation succeeded. Not the same as fully scanned. |
| Incomplete | Enough to show something useful, not enough to prove completeness.                   |
| Blocked    | Unreadable due to permissions, unavailable mount, or missing path.                   |
| Failed     | The operation was attempted and errored.                                             |
| Complete   | Coverage for the requested operation is complete.                                    |

Ready is the one that gets misread. A source can be navigation-ready while its recursive scan runs. A branch can be
child-ready while descendant classification is incomplete. A scope can be selectable while its recursive coverage is
partial. Ready means the current operation can proceed, not that the system is finished.

Coverage is tracked at two levels. Source scan phase covers the source lifecycle, where `idle` means no active worker
rather than fully scanned, and `partial` means the scan finished with gaps remaining. Directory scan state covers
subtree coverage. Recursive completeness for a selected scope derives from the directory states under that scope, not
from source-level phase alone.

## Child readiness and disclosure

Whether a branch can be expanded is a separate question from whether it contains media.

A branch must not become expandable because the renderer guesses children might exist, and must not become a leaf
because children are not yet known. The renderer derives expandability from a single navigable-child-scope state with
three values: unknown, has navigable child scopes, or none. Only the second warrants a disclosure control. Unknown
renders without one.

The store maintains descendant observations for child directories, playable media, and image media as internal
evidence for coverage and query planning. These are not renderer affordance authority. The renderer does not read them
to decide whether to draw a chevron, and it does not fetch a level of children for every visible row to find out.

Disclosure is about child browse scopes, which for local files means child directories. A folder full of audio with no
subfolders is selectable and useful, and correctly has no disclosure control.

The first accepted child window commits atomically. Folders do not appear one at a time during the initial read. The
user sees the previous accepted state, a clear probing state, or the complete first child set.

## Contents, coverage, and the verified-empty rule

A contents read is parameterized by scope, policy, scope depth, limit, cursor, and a renderer-owned request key and
generation.

Scope depth is `immediate` or `recursive`, and it is an identity value rather than a display option. Tree expansion
shows immediate children for navigation. Tree selection projects recursive contents for work. The two do not share
cursors, page accumulators, retained snapshots, coverage state, or empty-state eligibility.

**A scope is verified empty only when all of the following hold:**

- The scope identity is known
- The browse policy is known
- The selected depth is known
- Access is not blocked
- The read did not fail
- The cursor and response identity match the current request generation
- Coverage is complete for the requested policy
- The accepted result contains no matching rows

If any one is false, the state is not verified empty. Unknown, probing, incomplete, blocked, failed, cursor-invalid,
stale, and retained-pending never collapse into empty.

Only verified empty may use absence-proven language. Every other state gets copy that says what is actually true, which
is usually that the system does not know yet. A zero-row result with complete coverage where the active policy omitted
rows is empty for that policy only, and says so.

The exact strings are owned by the renderer and its projection tests, not by this document. What this document owns is
which state may use which class of statement.

## Selection and disclosure are separate

The row body selects a browse scope. The reveal lane expands or collapses a branch. These stay separate in pointer
handling, keyboard handling, projection, and tests.

Arrow keys operate disclosure. Enter selects. Space is not taken for tree selection, because it belongs to transport
expectations in a DJ application.

## Refresh retention

**Pending work does not wipe accepted state.**

If a branch has accepted children and a refresh begins, the accepted children stay visible as retained state until a
replacement is accepted or a terminal blocked or failed state requires something else. If contents have accepted rows
and a refresh begins for the same logical scope and policy, the rows stay.

Retention is not a claim of freshness. The projection can express retained, pending, incomplete, and stale-safe status
alongside the retained rows. The alternative, clearing the view on every refresh, makes a working library flicker
between content and emptiness for reasons the user cannot see.

Changing the filter or the depth changes contents identity. The next read uses a new request key or generation and must
not reuse rows, cursors, omission metadata, or empty-state eligibility from another policy. Rows from the previous
accepted identity may remain visible as retained-pending presentation until a matching response arrives.

## Background work does not own user intent

Scan started, progressed, completed, cancelled, and failed. Source lifecycle refresh. Invalidation replay. Background
child and contents refresh. Event pump gap recovery.

None of these are user intent. They may refresh loaded projections. They may not select a different source, expand or
collapse a branch, scroll the table, or move focus.

Only explicit user actions mutate selection, expansion, scroll, and focus: selecting a row, activating the reveal lane,
pressing a tree key, invoking retry or refresh, or choosing a filter. Choosing a filter changes contents identity only,
and leaves selection, expansion, scroll, and focus alone.

## Search, filters, and classification

Search runs against a SQLite FTS5 projection and stays within the selected library, source, or folder scope. Ordering
is backend-owned relevance. User-selectable sort controls are not implemented.

Workflow filters change the contents policy without changing the selected scope:

| Filter        | Policy                | Admits                                                  |
| ------------- | --------------------- | ------------------------------------------------------- |
| Audio         | `audioBrowse`         | Audio only                                              |
| Audio + Video | `playableMediaBrowse` | Audio and video                                         |
| All Files     | `sourceFileInventory` | Raw inventory, including images and admitted companions |

A fourth policy, `playableMedia`, reads promoted playable-media rows revalidated against current source files,
attachment links, and observations. It is a narrow evidence-backed profile and is not the product default.

Audio is the default. All Files is raw source inventory, not an interpreted view and not a problems view. Companion
files are admitted non-primary files associated with media workflows, and normal media-relevant inventory admits
unsupported rows only for CUE sheets.

Classification from a path is provisional. `.m4a` classifies as audio and `.mp4` as video until stronger probe evidence
exists. Backend and query code own policy filtering. Renderer mode names do not cross into protocol or query code, and
the renderer sends a typed policy rather than anything resembling a query.

## Pagination

Contents pagination uses keyset cursors, not offsets, because a live-updating scan result set shifts row positions
between page reads and offsets silently skip or repeat rows when it does.

A cursor encodes scope, the full policy discriminant with its canonicalized variant state, scope depth, and the last
row's ordering position. It is validated against the current request identity. A cursor from one depth, scope, or
policy is invalid for any other. A mismatched or undecodable cursor returns an explicit invalid-cursor result with no
next cursor, and a stale or invalid-cursor response never produces verified-empty presentation.

Contents pagination is distinct from tree load-more, which pages children of a branch.

## The boundary

**The backend owns** source state, filesystem observation, inventory, coverage, ordering, and policy filtering. Where
it cannot prove something, it exposes an honest state rather than relying on the absence of rows to imply it. Its read
models give the renderer enough to distinguish blocked from empty, failed from empty, incomplete from complete, unknown
from leaf, and probing from accepted.

Rows and coverage are read from one consistent snapshot, so scan progress cannot advance between the two and produce a
result that contradicts its own coverage.

**The renderer owns** projection and ephemeral interaction state: selected row, expanded rows, focus, and scroll
position. It may represent every readiness state above.

**The renderer may not invent** filesystem completeness, media coverage, descendant existence, verified empty, source
availability, or scan completion. It does not walk its own tree nodes to answer a recursive contents question, does not
treat expansion state as a filter on results, does not parse identity out of labels or path text, and does not fake
readiness with timers or optimistic guesses.

## Implementation

- Contents and search read models: `crates/library-store-sqlite/src/read_models/`
- Boundary protocol commands: `crates/library-boundary-protocol/src/commands/`
- Main-process library integration: `apps/desktop/src/main/library/`
- Renderer browser surfaces: `apps/desktop/src/renderer/library/`
- Renderer projection and integration tests: `apps/desktop/tests/`
- Electron acceptance scenarios: `apps/desktop/tests/e2e/`, run with
  `pnpm --filter @dekzer/desktop verify:e2e:library-v0`
