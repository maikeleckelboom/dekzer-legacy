# Browse Policy Integrity and Policy Omission Metadata

**Status:** Locked
**Applies to:** contents read service, contents read protocol, generated renderer contract, contents projection,
renderer empty-state copy
**Does not apply to:** source registration activation, source-root admission, default music source discovery,
playable-media policy implementation

---

## Core Decision

The contents service must distinguish four cases that the renderer cannot distinguish on its own:

1. A scope contains no browse-relevant inventory of any class — it is genuinely empty.
2. A scope contains browse-relevant inventory, but the active contents policy excludes it.
3. A scope contains raw filesystem objects that Dekzer does not model as contents inventory — neither included nor
   excluded by policy.
4. A scope has incomplete scan coverage — no authoritative empty statement is valid yet.

The service owns this distinction. The renderer presents it. The renderer has no authority to derive it independently.

---

## Rules

### Rule 1 — `audioBrowse` is audio only, permanently

`audioBrowse` includes audio-class contents rows. It does not include MP4. It does not include video. It does not
include containers whose audio content has not been proven by a media probe authority. Extension alone is not sufficient
proof.

This is not a V0 constraint. It is the permanent meaning of the name. If Dekzer ever needs a policy that includes audio
and video, that policy gets a new name. `audioBrowse` does not change meaning.

### Rule 2 — `.mp4` is video/container-class at the extension level

At extension-classification level: `.m4a` is audio-class. `.mp4` is video/container-class.

An MP4 container may carry audio streams, but extension alone cannot honestly classify it as an audio file. Until a
media probe authority establishes stream-level facts for a specific file, `.mp4` does not enter `audioBrowse`.

### Rule 3 — Authoritative empty requires both complete coverage and no browse-relevant inventory

"This folder is empty" is valid only when both conditions hold simultaneously:

- The service's scan/read coverage for the requested scope is authoritative enough to make an empty claim.
- The scope contains no browse-relevant inventory of any class under any policy.

A scope containing MP4 files is not empty. Under `audioBrowse` it is empty for the active policy. Those are different
statements and must not be collapsed into the same copy.

A scope with incomplete scan coverage and zero known rows is also not empty. It is still being indexed.

### Rule 4 — The service owns policy omission; the renderer presents it

The contents read service applies the active browse policy. When it omits browse-relevant inventory because the
inventory does not match the active policy, that omission is part of the service-owned result. It crosses the service
boundary as a contract field.

The renderer must not inspect raw source inventory, file extensions, tree state, source files, or any row not returned
to it in order to infer omissions. The renderer's authority is limited to:

- The returned rows.
- The returned `hasRowsOmittedByPolicy` value.
- The returned coverage/completeness state.

### Rule 5 — `hasRowsOmittedByPolicy` is a boolean in V0

The field is `hasRowsOmittedByPolicy: boolean`. It is not a count.

A count would carry obligations that are not yet cleanly answerable: exact across recursive scopes, across pagination,
across unavailable source rows, across ignored inventory, across source-file rows versus canonical track rows, and
stable under partial scan coverage. A boolean answers the only question V0 needs: did this view exclude browse-relevant
inventory because of the active policy? Upgrade to a count only when counting authority, scope semantics, and coverage
guarantees are explicitly defined.

### Rule 6 — Exact semantics of `hasRowsOmittedByPolicy`

`hasRowsOmittedByPolicy: true` means:

> Within the requested contents scope, recursion mode, and authoritative read snapshot, at least one persisted
> browse-relevant potential contents row exists but is excluded by the active contents policy.

The field is:

- Scoped to the request's `scope` parameter, not the current page.
- Scoped to the request's `recursion` mode — recursive reads consider descendants; immediate reads do not report beyond
  immediate children.
- Evaluated at the service/protocol boundary, not derived by the renderer.
- Independent of page size. A page that returns rows may still carry `hasRowsOmittedByPolicy: true` at the scope level.
- Not a claim that scan coverage is complete.
- Not triggered by ignored filesystem objects.

If the service cannot compute this boolean honestly for a requested scope, it must not fabricate a result. It returns
the conservative contract-defined fallback value or stops the implementation and surfaces the authority gap.

### Rule 7 — Three-tier inventory classification

Every persisted inventory item considered by the contents read service falls into exactly one tier for a given policy
request.

**Included** — matches the active contents policy; returned as a contents row.

**Omitted by policy** — is browse-relevant inventory that does not match the active contents policy. This tier sets
`hasRowsOmittedByPolicy: true`. Examples under `audioBrowse`: a video-class `.mp4` file, any future playable-media class
not included by the active policy.

**Ignored** — is not browse-relevant contents inventory for the product surface under any policy. Examples: OS noise,
temporary files, arbitrary binary objects, files Dekzer does not model as contents rows. Ignored items do not set
`hasRowsOmittedByPolicy`. The product must not say "files hidden by this view" for items in this tier.

The boundary between omitted-by-policy and ignored is not inferred from filesystem presence. It requires an explicit
product decision per inventory class, owned by the classification authority, not by the renderer and not by inference
from extension alone.

### Rule 8 — Companion metadata requires explicit classification

Metadata sidecars, artwork files, CUE sheets, and future analysis artifacts are not automatically ignored and are not
automatically browse-relevant contents rows. Their tier must be decided explicitly per file class.

A companion file triggers `hasRowsOmittedByPolicy` only if the active contents policy defines it as browse-relevant
potential contents inventory and excludes it. Until explicitly classified, companion files do not default into either
the omitted or the ignored tier — their tier is undefined and must be resolved before the service makes any claim about
them.

### Rule 9 — Coverage completeness gates empty-state copy

The renderer must consult both the returned coverage state and `hasRowsOmittedByPolicy` before rendering any empty-state
copy. Authoritative empty copy is only valid when coverage is complete.

| Coverage   | Returned rows | hasRowsOmittedByPolicy | Correct renderer meaning                                                                         |
|------------|:-------------:|:----------------------:|--------------------------------------------------------------------------------------------------|
| complete   |       0       |         false          | The scope has no browse-relevant inventory. Render authoritative empty copy.                     |
| complete   |       0       |          true          | Browse-relevant inventory exists but is excluded by the active policy.                           |
| incomplete |       0       |         false          | No rows known yet. Coverage incomplete. Do not render authoritative empty.                       |
| incomplete |       0       |          true          | Omitted inventory is known, but coverage is still incomplete. Do not render authoritative empty. |
| any        |      > 0      |         false          | Show returned rows. No omission note needed.                                                     |
| any        |      > 0      |          true          | Show returned rows. Optional compact policy-omission note; not required now.                     |

### Rule 10 — Copy is policy-driven, not hardcoded

Empty-state copy must be driven by the active policy's content description, not by a hardcoded string table in the
renderer.

Under `audioBrowse`, when coverage is complete, rows are zero, and `hasRowsOmittedByPolicy` is true:

> "No audio tracks in this view."

Not: "This folder is empty."

When a future playable-media policy is active, the same code path uses that policy's content description. The omission
metadata slice may implement a minimal policy-description lookup keyed to the existing policy kind. It must not migrate
the policy system into first-class configuration objects in the same slice.

### Rule 11 — MP4-only folder: canonical behavior

Given a scope containing only `.mp4` files, under `audioBrowse`:

- Returned rows: zero.
- `hasRowsOmittedByPolicy: true`.
- Correct copy: "No audio tracks in this view."

Under no active policy may this scope render "This folder is empty" as long as it contains any browse-relevant
inventory.

### Rule 12 — `hasRowsOmittedByPolicy` is not a filter escape hatch

The field is presentation metadata. The renderer must not use it to:

- Fetch additional inventory from any source.
- Reveal rows not returned by the service.
- Toggle renderer-side filters or sorting.
- Infer counts or file classes.
- Mutate selected scope or tree expansion state.
- Bypass the active contents policy in any way.

---

## Future Product Direction

For Dekzer as a professional DJ platform, the long-term default browse policy should be playable media: audio and video.
Audio-only browse is a filtered view, not the universal product default. This conclusion follows from competitive
research — rekordbox, Serato, and VirtualDJ all treat music video as first-class library content.

This future policy requires explicit naming, an explicit included-class contract, and an explicit empty-state
description. It must not be implemented by widening `audioBrowse`. It is not part of the omission metadata slice.

---

## Implementation Sequencing

### Slice N — Source-add activation

Merge independently. Source-add activation owns first-run browse activation behavior. The MP4-only folder issue is a
discovered product honesty gap, not a regression from this commit.

### Slice N+1 — Contents policy omission metadata

In scope:

- Store/service query support for `hasRowsOmittedByPolicy`
- Protocol/result contract update
- Generated contract update
- Coverage completeness signal in result (if not already present)
- Renderer empty-state copy using returned metadata and coverage state
- Focused tests covering all cases in the Rule 9 truth table

Out of scope:

- Widening `audioBrowse`
- Playable-media default policy
- First-class policy object migration
- Source-root admission or default music source discovery
- Renderer-side filtering or sorting
- Any new row-level fields (duration, BPM, key, codec, artwork, canonical track identity, analysis readiness)

### Slice N+2 — Browse policy as first-class configuration objects

Promote browse policy definitions into first-class objects carrying: policy key, included media/content classes, content
description label, empty-state copy label, default status. Must not be combined with Slice N+1.

---

## Required Tests for Slice N+1

**Service/protocol:**

- Audio-only scope under `audioBrowse` → rows returned, `hasRowsOmittedByPolicy: false`
- MP4-only scope under `audioBrowse` → zero rows, `hasRowsOmittedByPolicy: true`
- Mixed audio + MP4 scope under `audioBrowse` → audio rows returned, `hasRowsOmittedByPolicy: true`
- Ignored-only scope (OS noise, tmp files) → zero rows, `hasRowsOmittedByPolicy: false`
- Recursive read with MP4 only in a descendant → `hasRowsOmittedByPolicy: true` at scope level
- Immediate read → descendants outside immediate scope do not contribute to the field
- Paginated read → field is scope-level, not page-local; additional pages do not flip the value
- Incomplete scan coverage + zero known rows → coverage state is incomplete; authoritative empty not claimed

**Renderer:**

- Renderer does not inspect file extensions, raw inventory, or tree state to derive omission
- Zero rows + complete coverage + `hasRowsOmittedByPolicy: false` → "This folder is empty."
- Zero rows + complete coverage + `hasRowsOmittedByPolicy: true` → policy-specific copy
- Zero rows + incomplete coverage → no authoritative empty copy, regardless of omission flag
- Rows present + `hasRowsOmittedByPolicy: true` → rows shown normally; no rows hidden
- Empty-state copy string sourced from policy description, not hardcoded

**Regression guard — the following must not be present in the Slice N+1 diff:**

- MP4 added to `audioBrowse`
- Count field added to result
- Renderer filtering or sorting added
- Raw inventory inspection in renderer
- Playable-media policy implementation
- Policy object migration
- Source admission or default discovery changes
- Ignored filesystem objects triggering `hasRowsOmittedByPolicy`
- Authoritative empty copy rendered under incomplete coverage or active omission

---

## Hard Lines

`audioBrowse` does not include MP4.

A folder containing only MP4 files is not empty.

`hasRowsOmittedByPolicy` crosses the service boundary. The renderer does not derive it.

Ignored filesystem objects do not trigger `hasRowsOmittedByPolicy`.

Incomplete scan coverage must not produce authoritative empty copy.

Companion metadata class assignment must be explicit, not defaulted.

Slice N+1 and Slice N+2 are separate commits with no shared scope.
