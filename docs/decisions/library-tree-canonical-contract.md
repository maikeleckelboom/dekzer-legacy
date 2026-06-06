# Dekzer Library Tree — Architectural Synthesis and Canonical Contract

**Document type:** Architectural decision record
**Source corpus:** Seven-product pro-DJ competitive research (Serato DJ Pro, rekordbox, Traktor Pro, VirtualDJ, djay
Pro, Engine DJ / Engine Desktop, Mixxx) with screenshot evidence and official manual citations
**Confidence level per finding:** labeled inline

---

## 1. What the Research Proves Without Qualification

These are not recommendations. They are invariants that every researched product upholds simultaneously. Violating any
of them creates a product that DJs correctly identify as "wrong" on first use.

**Invariant 1 — The left pane is a scope tree, not a content tree.**
Tracks and audio rows live exclusively in the right-hand content pane. The left pane holds navigation scopes: folders,
crates, playlists, service roots, library sections. Not one product mixes track rows into the tree. Confidence:
universal/high.

**Invariant 2 — Selected state and expanded state are orthogonal.**
A node can be selected-and-collapsed, selected-and-expanded, unselected-and-expanded, or unselected-and-collapsed. All
four combinations are valid at runtime and must be visually distinguishable. This is the most important architectural
property of the left pane and the one most commonly violated by naive implementations. Traktor's controller manager
documents `Tree > Select Up/Down` and `Tree > Select Expand/Collapse` as wholly independent commands. VirtualDJ
documents a separate left-side expand button. Serato documents a separate small arrow for expansion. Engine DJ's
keyboard docs state Left/Right expand/collapse independently of Up/Down selection movement. Confidence: very high.

**Invariant 3 — Parent nodes are valid browse scopes, not just containers.**
Selecting a parent node must update the content pane with that parent's own aggregated scope — its own direct content
plus, where applicable, descendant scope content. This dual-role is not a preference toggle; it is the expected
behavior. rekordbox's manual states explicitly that selecting a playlist folder displays the tracks of playlists
contained under it. Serato's "Include Subcrate Tracks" preference exists, but that is the product being cautious about
backward compatibility — the correct design decision is to make it the invariant default. Traktor says "select the
iTunes folder to display its content in the Track List" while separately documenting expansion. Engine DJ says
sub-playlist tracks appear in the parent playlist. Confidence: high.

**Invariant 4 — Leaf nodes have no disclosure affordance.**
Not a disabled triangle. Not a placeholder. Nothing. The complete absence of a disclosure icon is the signal to the user
that this node has no child scopes. Traktor's official screenshots show "All Tracks," "All Stems," and "All Samples" as
leaf rows with no disclosure elements. rekordbox screenshots show leaf playlists without disclosure triangles. Engine
DJ's keyboard docs explicitly say expand/collapse "only applies to folders and playlists that contain sub-folders or
sub-playlists." Mixxx's GoToItem control moves focus to the track table when invoked on a leaf rather than attempting
expansion. Confidence: high.

**Invariant 5 — Double-click must not carry any primary contract behavior.**
The inconsistency across products is prohibitive. Serato uses double-click to rename a crate. Traktor and rekordbox use
double-click to expand certain root/library nodes. djay uses double-click to edit song metadata in the library table.
Mixxx documents double-click primarily for track-table actions. A user migrating from any of these products will bring a
muscular double-click expectation that conflicts with the others. Anything essential to Dekzer's navigation contract
that requires double-click is inaccessible to keyboard-primary users and will produce accidental misfires in the DJ
booth. Confidence: definitive.

---

## 2. The Best Patterns to Adopt, by Product

### From Traktor Pro — the clearest interaction model

Traktor is the strongest reference. Its documentation is unusually explicit: selecting a subfolder causes the Track List
to update; a double-click separately expands the folder in the Browser Tree. The controller manager exposes
`Tree > Select Up/Down` and `Tree > Select Expand/Collapse` as independent commands, making the conceptual model visible
at the API level. Screenshots show the selected-row highlight and disclosure triangle as visually separate elements.
Traktor is the product that most closely pre-validates Dekzer's current contract.

Adopt: the explicit conceptual separation of "which scope the content pane is showing" from "which branches are open in
the tree."

### From VirtualDJ — the clearest pointer affordance

VirtualDJ explicitly documents "clicking on the button to the left of any folder will expand it and show its related
sub-folders." That sentence is the clearest disclosure-click specification found across all seven products. The physical
placement — a button left of the folder, not overlapping the row body — makes the separation spatially unambiguous.

Adopt: the spatial separation principle. The disclosure affordance is a distinct, smaller hit target sitting to the left
of or preceding the row content. It is not the same zone as the row body. Users who click the row body cannot
accidentally trigger expansion.

Do not adopt: VirtualDJ's History special case, where clicking the History folder row expands it rather than selecting
it as a scope. Both research documents flag this explicitly as the anti-pattern — an exception that creates a two-tier
interaction model and trains users to expect row-click expansion in some cases. If Dekzer ever special-cases any node
type to expand on row-body click, it breaks the invariant that users learn once and apply everywhere.

### From Engine DJ — the clearest keyboard contract

Engine DJ's keyboard documentation states: Up/Down navigate tree or track list; Left/Right expand/collapse only folders
and playlists that contain sub-children. This is the cleanest specification of the keyboard contract found in the
corpus. The explicit constraint that Left/Right are no-ops on leaf nodes is correct and must be preserved.

Adopt: Left/Right as the canonical expand/collapse keys. Expand fires only on collapsed containers; collapse fires only
on expanded containers; both are no-ops on leaves.

### From Mixxx — the clearest leaf behavioral distinction

Mixxx's GoToItem control separates its behavior by node type: on a collapsed container, it expands; on a leaf, it moves
focus to the track table. This is the most explicit documentation of what "activating" a leaf node means in practice.
The leaf does not have expansion behavior — it has focus-hand-off behavior.

Adopt: the principle that leaf activation is conceptually different from container activation, and that difference must
be encoded in the interaction model rather than just the visual rendering.

### From Serato — the clearest rename-vs-navigate separation

Serato's explicit use of double-click for crate renaming, and its explicit separate small arrow for expansion, makes it
the clearest model for how to separate these three distinct actions (select, expand, rename) without overloading any
single gesture. The implication for Dekzer is that inline rename, if supported, needs its own explicit affordance — a
dedicated UI element — rather than borrowing a gesture from navigation.

---

## 3. What to Explicitly Reject and Why

**Reject: Row-body click triggering expansion (the VirtualDJ History failure mode)**
This is the most dangerous anti-pattern. Once any node type expands on row-body click, users are training a split mental
model. When they encounter a different node that doesn't expand on row-body click, they'll assume it's a bug.
VirtualDJ's History exception is documented in the research precisely because it makes the overall model less
predictable. Dekzer's nodes must behave identically by structural role — containers respond to disclosure clicks for
expansion, all nodes respond to row-body clicks for scope selection.

**Reject: Disabled or placeholder disclosure triangles on leaf nodes**
A greyed-out triangle on a leaf node tells the user "there might be something here but I'm not letting you get to it."
It communicates contingency where there is none. The correct signal is pure structural absence: no triangle means no
children. Any implementation that adds layout placeholders for disclosure alignment should solve the alignment problem
through padding or fixed-width indentation, not through fake interactive elements.

**Reject: Expansion side-effecting the content pane**
Expanding a node must not change what the content pane is showing. This is the most subtle violation of Invariant 2. If
pressing Right to expand a collapsed parent also refreshes the content pane to show that parent's scope, you've tied
expansion to selection. The content pane must update only on selection events. Expansion is a pure tree-visibility
operation.

**Reject: Loading state rendered on the row body**
If deferred child loading shows a spinner or busy state on the row highlight, the visual language conflates "this row is
the active browse scope" with "this row is fetching children." These are independent states. The loading affordance
belongs in the disclosure slot, not the row body.

**Reject: Double-click for any primary navigation or expansion behavior**
Covered in Invariant 5. Worth restating as an explicit reject because it's tempting to add it as a "shortcut." If it's
there, power users will discover it and rely on it, and then it's part of the contract.

---

## 4. Where Dekzer Can Be Better Than All of Them

These are the gaps in every product that Dekzer gets to define correctly from first principles.

### Deferred child loading as a first-class affordance

No product in the corpus clearly defines what happens in the disclosure slot when children are being loaded lazily. This
is a real and important state, especially for large libraries with folder hierarchies that are expensive to walk. The
correct pattern:

- User clicks disclosure → spinner appears in the disclosure slot (replacing the static triangle)
- Parent node remains fully selectable and browsable during the fetch
- Content pane is unaffected by the in-progress child fetch
- When children arrive, the spinner is replaced by the expanded disclosure state (▼) and child rows appear
- If the fetch fails, the disclosure returns to collapsed state (▶) with an optional error signal

The parent's browsability during loading is architecturally critical. It means the content pane update path (triggered
by selection) and the child fetch path (triggered by disclosure) are fully independent async operations with no shared
mutable state. This is not just good UX — it is the correct architecture for the system's retained-read / warm-prefetch
model.

### Parent dual-role as the default, not a preference toggle

Serato's "Include Subcrate Tracks" preference exists because Serato shipped without it and had to add it without
breaking existing behavior. Dekzer has no such legacy constraint. Parent nodes should always aggregate their children's
scope content. This should be an invariant of the data model, not a runtime preference. A parent playlist folder shows
its own tracks plus all descendant tracks. A parent crate shows its own tracks plus all subcrate tracks. The user should
never discover a settings page to get this behavior.

### Explicit visual language for all four selection×expansion state combinations

None of the products does this cleanly enough. The requirement is:

| State                  | Visual                                                   |
|------------------------|----------------------------------------------------------|
| Unselected + collapsed | Neutral row background, ▶ disclosure                     |
| Unselected + expanded  | Neutral row background, ▼ disclosure                     |
| Selected + collapsed   | Highlighted row background, ▶ disclosure                 |
| Selected + expanded    | Highlighted row background, ▼ disclosure                 |
| Leaf (any)             | Highlighted or neutral background, no disclosure element |

The selected row background and the disclosure orientation must be visually independent signals. A user who is
navigating by keyboard must be able to see, at a glance, both which node is their current browse scope and which
branches are open, without one piece of information obscuring the other.

### The keyboard contract for Space

Every product either documents Space as a playback control (djay: Space = Play/Pause) or leaves its tree behavior
undocumented. This means there is no cross-product expectation to honor. Dekzer must make an explicit decision: either
Space is bound to transport and is completely absent from the tree keyboard contract, or it is a focus-scoped alias for
Enter that only activates when tree focus is unmistakably held. The worst outcome is an ambiguous Space binding that
fires differently depending on where the application believes focus currently lives, because focus tracking is hard and
DJs operate in high-stress environments where accidental playback state changes have consequences. If in doubt, Space
belongs to transport and Enter is the canonical tree activation key.

---

## 5. The Canonical Dekzer Library Tree Contract

This is the normative specification. It supersedes any prior informal description.

### Node taxonomy

Every node in the left-side tree is one of three structural roles:

**Container node** — has known navigable child scopes. Has a disclosure affordance. Is
selectable as a browse scope that aggregates its own content and all descendant content. Is expandable independently of
selection. Unknown child scope existence must not be projected as a known expandable branch.

**Leaf node** — has no navigable child scopes, or child scope existence is unknown. Has no disclosure affordance. Is selectable as a browse scope. Has no expansion
state. Progressive scan may transition a row from unknown/no-disclosure to known-branch/disclosure through invalidation.

**Service root / virtual node** — a container node whose children are fetched from an external or deferred source.
Behaves identically to container nodes except that first expansion triggers a deferred child fetch rather than revealing
already-loaded children.

Tracks and audio files are never rendered as nodes in this tree. They live in the content pane.

### Pointer contract

**Row body click:**
Sets the clicked node as the current browse scope. Triggers content pane refresh with that scope's content. Does not
change the node's expanded state. Applies identically to container nodes and leaf nodes.

**Disclosure click:**
Toggles the clicked node's expanded state (collapsed → expanded, or expanded → collapsed). On first expansion of an
unloaded container, triggers deferred child fetch and shows loading state in the disclosure slot. Does not change the
clicked node's selected state. Does not change the content pane.

**Double-click:**
Not part of the primary interaction contract. If implemented at all, it must be a non-destructive alias for an action
already accessible via single click or keyboard. It must never be the only way to reach any state. If used for inline
rename, it must be constrained to rename-capable custom node types and must not fire on read-only library nodes or
during active navigation.

**Hover:**
Subtle row highlight to communicate hit target. Must not be visually confusable with selected state. Must not trigger
any state change.

### Keyboard contract

**Up / Down:**
Move selection to the previous or next visible row in the tree. Each movement updates the content pane immediately,
because selection and content pane are always in sync. Invisible rows (collapsed children) are skipped.

**Right (ArrowRight):**
If the focused node is a collapsed container: expand it (same as disclosure click). Spinner in disclosure slot if
children are deferred. Does not change which node is selected. Does not move focus to first child.
If the focused node is an already-expanded container: no-op.
If the focused node is a leaf: no-op.

**Left (ArrowLeft):**
If the focused node is an expanded container: collapse it. Does not change selection. Does not change content pane.
If the focused node is a collapsed container: no-op (moving to parent is an optional enhancement, not canonical
behavior, because it triggers a scope change that must be explicit).
If the focused node is a leaf: no-op.

**Enter:**
Selects the currently focused node as browse scope and updates the content pane. Identical to row-body click. This is
the canonical keyboard activation key for tree selection.

**Space:**
Bound to transport/playback unless the tree has explicit, unambiguous keyboard focus AND Space is provably unmapped from
transport in the current focus context. If there is any ambiguity, Space is not a tree key. The implementation must not
rely on implicit focus assumptions to resolve this.

**Ctrl/Cmd+F:**
Move focus to the library search field. Standard and expected by users coming from every researched product.

### State model

Each node carries exactly four independent state bits relevant to rendering:

`selected: boolean` — this node is the current browse scope; drives content pane content
`expanded: boolean` — this node's children are currently visible in the tree (containers only; undefined for leaves)
`loading: boolean` — this node's children are currently being fetched (containers only; for deferred/service nodes)
`hovered: boolean` — the pointer is currently over this row

These four bits are orthogonal. No transition on one bit is allowed to automatically mutate another. Specifically:

- Selecting a node does not expand it.
- Expanding a node does not select it.
- Loading children does not change selection or expansion state (expansion is set when the load completes, not when it
  starts — the disclosure transitions: ▶ idle → ⟳ loading → ▼ expanded).

### Content pane update timing

The content pane updates only on selection change events. It does not update on expansion, collapse, hover, or child
loading. When selection changes:

- If warm/cached content exists for the new scope: swap content immediately, no loading state
- If content is being fetched: retain the currently displayed rows, mark them visually as stale (subtle), show a loading
  indicator attached to the content pane header or breadcrumb — not on the tree row
- When fresh content arrives: replace stale content with live content

The selected tree node and the content pane always agree on the current scope. There is no state where the tree
selection and the content pane scope are out of sync. If a selection event fires, the content pane must at minimum show
the stale-retained version of that scope immediately, even if a fresh fetch is still in flight.

### Disclosure hit target

The disclosure affordance is a dedicated hit target occupying a fixed-width zone at the start of the row, before the
node icon and label. Its width must be sufficient for comfortable pointer interaction (minimum effective tap/click size
applies) but should be clearly narrower than the remaining row body. It must not overlap with the node icon or label
zone. The disclosure zone is absent entirely on leaf nodes — no placeholder, no padding substitute, no disabled element.

### Loading state in disclosure slot

While deferred children are being fetched:

- The static disclosure triangle is replaced by a spinner or progress indicator in the same slot
- The row body remains clickable and selection-functional during the load
- The expanded state is considered in-transition; it resolves to `expanded: true` when the load completes successfully,
  or reverts to `expanded: false` (collapsed) if the load fails
- The content pane is unaffected throughout — a simultaneously selected node that is also loading children does not show
  a loading state in the content pane for the child fetch

---

## 6. Open Questions Requiring a Product Decision

**Left-arrow on collapsed node → move to parent:**
The full tree-widget keyboard convention (as in macOS Finder) moves focus to the parent node when pressing Left on an
already-collapsed node. None of the researched products clearly document this behavior for their DJ library trees, and
it carries a meaningful cost: moving to the parent node triggers a scope change (content pane update to the parent's
scope). That is a high-impact side effect of a navigation keystroke. The safer default is to make Left a no-op when
already collapsed, and only add the parent-navigation behavior if there is explicit user demand for it. Decide before
first keyboard implementation, not after.

**Space in transport context:**
Dekzer likely has a transport/performance layer. The binding contract between the tree focus context and the transport
shortcut layer must be defined explicitly, not resolved at runtime through implicit focus tracking. This is a
product-level decision that affects every keyboard interaction in the application.

**Node iconography for container vs. leaf:**
The research shows that the disclosure triangle's presence or absence is the primary structural signal. A secondary
signal via node iconography — e.g., a folder icon with a chevron/hierarchy indicator for containers, a flat list icon
for leaves — would reinforce the structural difference without adding complexity. This is a design decision, but one
worth making explicitly rather than defaulting to identical icons for all node types.

**Inline rename affordance:**
If Dekzer supports renaming user-created nodes (custom playlists, crates, folders), the rename trigger needs an explicit
home. It cannot be double-click (rejected above). Options: right-click context menu only, or a hover-revealed edit icon
in the row. Decide before implementing the context menu.

---

## 7. What This Research Validates About Dekzer's Current Contract

The research validates the following elements of the current Dekzer contract without modification:

- Row body click selects browse scope only
- Disclosure click reveals/collapses child nodes only
- Row body click does not expand
- Disclosure click does not select
- Leaf folders are selectable and have no disclosure
- Parent folders are both selectable and expandable
- Tracks live in the content pane, not the tree
- ArrowRight reveals (not "reveal then move to first child")
- ArrowLeft collapses

The research identifies the following elements that need explicit refinement:

- **Enter as the canonical keyboard select/load key** — make this explicit in the spec; do not leave it ambiguous
  between Enter and Space
- **Space requires scoping** — either it belongs to transport and is absent from the tree contract, or it is a
  focus-scoped alias; the current ambiguity must be resolved
- **Visual separation of selected vs. expanded** — the four-state combination table above must be fully implemented; any
  UI review should confirm all four combinations are distinguishable at a glance
- **Disclosure slot as the exclusive home for loading state** — if this is not currently specified, add it; do not let
  loading state bleed into the row body

---

*Synthesis based on seven-product competitive corpus: Serato DJ Pro 4.x, rekordbox 7.x, Traktor Pro 4.x, VirtualDJ
2025/2026, djay Pro 5.x, Engine DJ / Engine Desktop v5, Mixxx 2.5.x. Screenshot evidence and official manual citations
as documented in the source research PDFs.*
