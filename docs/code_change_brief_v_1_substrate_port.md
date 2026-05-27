# Code Change Brief: V1 Substrate Port

**Status:** Draft implementation brief
**Scope:** Rust substrate, boundary crates, generated contract fallout, and immediate desktop integration points
**Authority:** `docs/decisions/library-preparation-substrate-v1.md`
**Companion:** `docs/schema-rewrite-prompt.md`

This brief turns the v1 library + preparation substrate decision into an implementation plan. It is not a design session
and it is not a permission slip for a broad rewrite. The schema decision owns the vocabulary. This brief owns the order
of attack.

The goal is to replace the legacy asset/capability/prep-policy epoch with a substrate centered on source inventory,
track identity, attachments, locations, preparation facets, work/artifact evidence, and disposable projections.

---

## 1. Outcome

After the port, the active implementation should read as if the old model never became product substrate.

The repo should use:

```text
source_files                       observed source inventory
tracks                             durable collection identity
track_attachments                  track-owned media payloads
track_attachment_locations         observed source-material links
track_identity_*                   resolver audit and decisions
preparation_facets                 governed preparation dimensions
preparation_facet_states           subject-scoped evidence/readiness state
work_runs                          execution context
work_items                         units of work inside a run
artifacts                          evidence produced by work items
artifact_supersessions             immutable supersession edges
track_browser_rows                 disposable browser projection cache
```

The repo should not expose active implementation concepts named:

```text
LibraryAssets
LibraryAssetId
library_asset_id
LibraryAssetAttachments
LibraryAssetCapabilities
LibraryBrowserRows
CapabilitySpecs
PrepPolicies
PrepAssignments
ResolvedLibraryAssetPrepTargets
```

Historical or future-architecture docs may mention old vocabulary only when clearly marked as non-authoritative for v1
implementation.

---

## 2. Non-negotiable laws

```text
No compatibility aliases.
No dual model.
No LibraryAssetId alias to TrackId.
No old and new public vocabulary living together.
No wrapper endpoint that preserves the old boundary shape.
No migration bridge for data that does not exist yet.
No broad product-domain expansion while doing the substrate port.
No renderer-owned preparation facts.
No projection row treated as durable authority.
```

This is a greenfield baseline replacement. If a file still needs old vocabulary to compile, that file has not been
ported. Do not paper over it with aliases.

---

## 3. Implementation posture

Do not start by inventing a perfect future crate graph. Start by making the current repo honest.

The implementation should proceed in small slices:

```text
Slice 1: Schema baseline + schema validation
Slice 2: Domain vocabulary and IDs
Slice 3: Store authority and read/write paths
Slice 4: Boundary protocol and service
Slice 5: Desktop/generated contract fallout
Slice 6: Legacy vocabulary deletion sweep
```

Each slice should compile or produce a deliberately bounded failure frontier. Do not try to land every Rust, TypeScript,
renderer, and future product-domain change in one mega-pass.

---

## 4. Read-first files

Before changing code, read:

```text
docs/decisions/library-preparation-substrate-v1.md
docs/schema-rewrite-prompt.md
docs/implementation-structure.md
docs/product-doctrine.md
```

If the neutral filenames have not landed yet, use their current equivalents and fix references as part of docs cleanup,
not as part of the substrate port.

Then inspect the actual repo shape:

```text
Cargo.toml
crates/library-store-sqlite/
crates/library-boundary-protocol/
crates/library-boundary-service/
packages/library-boundary-contract/
packages/library-boundary-client/
apps/desktop/src/main/
apps/desktop/src/shared/
apps/desktop/src/preload/
apps/desktop/src/renderer/library/
```

Do not assume a crate, module, or file exists because an older plan named it.

---

## 5. Slice 1: Schema baseline

### Goal

Land `20260527000000_substrate_v1.sql` as the active canonical baseline.

### Required work

```text
Add the new baseline schema from docs/schema-rewrite-prompt.md.
Point schema bootstrap/baseline loading at the new epoch.
Remove the old asset-centered baseline from active use.
Update schema checks and seeds.
Add or update invariant validation for INV-01 through INV-08.
```

### Acceptance bar

```text
SQLite parses the new schema.
All FK references resolve.
STRICT tables are preserved.
No active schema references library_asset_id.
No active schema creates LibraryAssets, LibraryBrowserRows, CapabilitySpecs, PrepPolicies, or related legacy tables.
Schema validation covers the cross-table constraints SQLite cannot express directly.
```

### Do not do in this slice

```text
Do not port renderer UI.
Do not add organization tables.
Do not add sleeves, requests, devices, imports, exports, or history.
Do not introduce compatibility views.
```

---

## 6. Slice 2: Domain vocabulary and IDs

### Goal

Make Rust domain vocabulary match the schema.

### Required concepts

Add or port domain types for:

```text
Track / TrackId
TrackMetadata
TrackAttachment / TrackAttachmentId
TrackAttachmentKind
TrackAttachmentLocation / LocationId
TrackIdentityCandidate / CandidateId
TrackIdentityResolutionResult / ResolutionResultId
TrackIdentityResultOption / OptionId
TrackIdentityDecision / DecisionId
PreparationFacet / FacetKey
PreparationFacetState
PreparationFacetSubject
WorkRun / WorkRunId
WorkItem / WorkItemId
Artifact / ArtifactId
ArtifactSupersession
TrackBrowserRow
```

Remove or replace domain types for:

```text
LibraryAsset
LibraryAssetId
LibraryAssetAttachment
LibraryAssetCapability
CapabilitySpec
CapabilityKind
PrepPolicy
PrepAssignment
ResolvedLibraryAssetPrepTarget
LibraryBrowserRow
```

### Naming rules

```text
Track is collection identity.
Attachment is track-owned media payload.
Location connects an attachment to observed source material.
Preparation is durable substrate vocabulary.
Prep is allowed only in API/folder shorthand where already established.
Readiness is projected verdict, not work execution state.
Facet is a governed preparation dimension.
```

### Acceptance bar

```text
No public Rust type aliases old asset IDs to new track IDs.
Compiler errors point to real porting work, not missing aliases.
New enums match schema value sets exactly.
```

---

## 7. Slice 3: Store authority and read/write paths

### Goal

Move store logic from asset/capability ownership to track/preparation ownership.

### Work areas

Inspect actual files first. Then port in this order:

```text
schema bootstrap and invariant checks
source inventory code that remains valid
track creation and metadata writes
track attachment writes
track attachment location writes
identity resolver writes
work run writes
work item writes
artifact writes
facet registry and facet state writes
track browser projection reads
contents reads that previously joined asset/browser rows
```

### Required ownership changes

```text
work_runs owns work_items.
work_items owns artifacts.
artifacts do not point directly to work_runs.
artifact supersession is an edge table.
preparation_facet_states owns the current_artifact_id pointer.
track_browser_rows is disposable projection state.
source discovery creates source inventory, not tracks by side effect unless a resolver path explicitly does so.
```

### Store logic that must exist

```text
A referenced source_file cannot be physically removed from an available attachment location without first updating the location availability state.
A current_artifact_id update must verify subject_kind, subject_id, and facet_key match the referenced artifact.
Identity decisions may exist without a track unless decision_kind is accepted.
Accepted identity decisions require a track.
Segment attachment locations must be anchored through a source file.
```

### Acceptance bar

```text
library-store-sqlite compiles.
Schema tests pass.
Store tests prove source_file -> track_attachment_location resilience.
Store tests prove work_runs -> work_items -> artifacts ownership.
Store tests prove preparation_facet_states current_artifact_id validation.
Store tests prove track_browser_rows replaces LibraryBrowserRows.
```

---

## 8. Slice 4: Boundary protocol and service

### Goal

Expose the new substrate vocabulary across Rust service boundaries without preserving old names.

### Required work

```text
Update protocol DTOs from asset vocabulary to track vocabulary.
Update service handlers that call store/read-model paths.
Regenerate boundary contract if the repo uses generated TS packages.
Remove old commands/events that only make sense for LibraryAssets or CapabilitySpecs.
Remove playlist writes if they are out of v1 scope and currently tied to the old model.
```

### Guardrails

```text
Do not keep old commands as wrappers.
Do not add deprecated fields.
Do not keep old generated contract names alive as compatibility surfaces.
If a renderer path still needs old names, report the required follow-up instead of preserving the old protocol.
```

### Acceptance bar

```text
library-boundary-protocol tests pass.
library-boundary-service tests pass.
Generated contract export/check passes if applicable.
No generated public contract still exposes LibraryAsset as active v1 vocabulary.
```

---

## 9. Slice 5: Desktop and generated contract fallout

### Goal

Update the immediate TypeScript/main/preload/shared fallout caused by the Rust boundary change.

### Required work

```text
Update generated contract package.
Update boundary client package.
Update desktop main handlers that consume changed service replies.
Update shared DTOs that mirror old asset/browser rows.
Update renderer only where required to compile or consume renamed fields.
```

### Guardrails

```text
Do not redesign renderer architecture in this slice.
Do not build the full preparation UI.
Do not add organization, sleeves, requests, devices, imports, exports, or history surfaces.
Do not use renderer state to repair missing substrate facts.
```

### Acceptance bar

```text
Boundary contract typecheck passes.
Boundary client typecheck passes.
Desktop typecheck passes.
Existing library browser flow compiles against track_browser_rows-shaped DTOs.
```

---

## 10. Slice 6: Legacy vocabulary deletion sweep

### Goal

Remove old names from active code, tests, fixtures, and docs after the port compiles.

### Search terms

```text
LibraryAssets
LibraryAsset
LibraryAssetId
library_asset_id
LibraryBrowserRows
LibraryAssetCapabilities
CapabilitySpecs
CapabilityKind
PrepPolicies
PrepAssignments
ResolvedLibraryAssetPrepTargets
selectedContentsRead as final public boundary
```

### Rules

```text
Delete old tests that only prove the old model.
Port tests that prove still-valid behavior.
Do not keep compatibility tests.
Docs may mention old vocabulary only as explicitly historical or future-stale context.
```

### Acceptance bar

```text
No active implementation code references removed model names.
No active schema references removed model names.
No tests assert old aliases or compatibility behavior.
Docs that mention old vocabulary are marked as historical/future-stale or are outside implementation authority.
```

---

## 11. Suggested implementation commands

Use the repo’s actual scripts if these differ.

Rust-side checks:

```text
git diff --check
cargo fmt --all --check
cargo test -p library-store-sqlite
cargo test -p library-boundary-protocol
cargo test -p library-boundary-service
cargo clippy --workspace --all-targets -- -D warnings
```

Boundary / TS checks if touched:

```text
cargo run -p xtask -- export-boundary-contract
cargo run -p xtask -- check-boundary-contract
pnpm --filter @dekzer/library-boundary-contract run typecheck
pnpm --filter @dekzer/library-boundary-client run typecheck
pnpm --filter @dekzer/desktop run typecheck
pnpm --filter @dekzer/desktop run test
pnpm --filter @dekzer/desktop run lint
pnpm --filter @dekzer/desktop run format:check
```

Full validation, when the repo is ready:

```text
pnpm verify
```

---

## 12. Stop conditions

Stop and report before pushing deeper if:

```text
The schema rewrite has not landed or does not parse.
The actual crate structure differs enough that this brief no longer maps cleanly.
A new crate seems necessary to compile the current slice.
The implementation would require simultaneous schema, Rust, generated TS, renderer, and UI redesign in one pass.
A compatibility alias seems necessary to keep compiling.
An old public boundary would remain beside the new public boundary.
Validation failures point to unrelated repo breakage.
```

The right response to a stop condition is not to widen scope. Report the exact mismatch and propose the next smallest
slice.

---

## 13. Contents read v1 pagination law

The contents boundary remains first-page-only during the v1 substrate port. Cursor identity and virtualization-ready
continuation are later work, not substrate-port work.

```text
Contents reads remain first-page-only in v1.

The request may carry a cursor field only as deferred protocol shape.

If a cursor is provided, the boundary returns an explicit non-success cursorInvalid result.
It must not silently ignore the cursor, restart from page one, or treat the request as successful pagination.

Successful v1 contents reads do not emit nextCursor.

Renderer code must not render contents Load More rows or maintain contents-page accumulation state in v1.

Real cursor identity, ordering-bound pagination, and virtualization-ready continuation are a later slice.
```

The later cursor slice must bind cursor identity to:

```text
cursor version
scope
policy
recursion
row profile
media class set
query/order identity
last-row key tuple
```

This law belongs to the contents boundary, but it is repeated here because the substrate port will touch
browser/read-model and boundary surfaces. Do not let the substrate port accidentally smuggle cursor pagination back in.

---

## 14. What this brief does not cover

```text
Full organization model: crates, playlists, smart lists, folders, tags.
Sleeves implementation.
History and RT Flight Deck runtime evidence.
Requests and request queue.
Devices and preflight records.
Imports and exports.
Full Prepared Room UI.
Deck runtime.
Performance session.
Broadcast projection.
Renderer domain restructuring.
```

These are real product domains, but they are not part of the v1 substrate port.

---

## 15. First recommended implementation slice

The first implementation slice should be:

```text
Land schema baseline + schema validation only.
```

Expected output:

```text
20260527000000_substrate_v1.sql is active.
Schema bootstrap points at the new epoch.
Schema checks cover v1 invariants.
Seeds use preparation_facets instead of CapabilitySpecs/PrepPolicies.
No implementation compatibility bridge is added.
```

Suggested commit message:

```text
feat(library): introduce v1 preparation substrate schema
```

Only after that should the Rust vocabulary/store port begin.

---

## 16. Final report contract for agents

Keep final reports concise:

```text
files changed
files deleted
validation run
git status
remaining legacy vocabulary, if any
risks
suggested commit message
```

Do not paste large diffs. Do not narrate every refactor step. Report the invariant-relevant decisions and the evidence
that validation ran.

