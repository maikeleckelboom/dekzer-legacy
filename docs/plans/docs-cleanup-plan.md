# Dekzer Documentation Cleanup Plan

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** This is a plan only. This intake pass does not move, delete, archive, merge, or clean up existing docs.

## Purpose

Reduce documentation sprawl while preserving ownership, history, and implementation safety.

The target is not a totally flat docs directory. The target is a flat-enough structure with a small number of durable domains and predictable filenames.

## Problem signals

Current docs have grown through migrations, planning, architecture exploration, and implementation repairs. Risks:

- too many nested folders,
- stale vocabulary surviving in active docs,
- old branch history reading like current doctrine,
- implementation prompts scattered across active docs,
- unclear distinction between proposal, accepted decision, and historical note,
- hard-to-find canonical docs.

## Target structure

Recommended top-level docs groups:

| Folder              | Purpose                                                           |
| ------------------- | ----------------------------------------------------------------- |
| `docs/product`      | Product doctrine, user-facing mental models, workflow principles. |
| `docs/architecture` | Active architecture proposals/specs/contracts not yet code.       |
| `docs/library`      | Current library substrate contracts and accepted behavior.        |
| `docs/workspace`    | Workspace topology/layout contracts.                              |
| `docs/runtime`      | Runtime/audio/rendering/performance contracts once accepted.      |
| `docs/decisions`    | Accepted durable decisions and implementation discipline.         |
| `docs/plans`        | Temporary plans, cleanup plans, roadmaps, migration plans.        |
| `docs/archive`      | Historical/deprecated material only if retained deliberately.     |

Rule: avoid more than one nested level unless the directory is an explicit domain with many accepted docs. Prefer filenames to carry specificity.

Example:

- prefer `docs/library/playable-media-promotion-contract.md`
- avoid `docs/library/evidence/promotion/playable/media/contract.md`

This follows the project convention that docs use kebab-case filenames.

## Status labels

Every architecture/planning doc should begin with one status line:

- proposal, not implemented
- proposal, accepted for implementation
- accepted contract
- implementation note
- migration history
- archived reference

If a document lacks status, cleanup should add one or move it to archive.

## Cleanup phases

### Phase 0 — Inventory only

Use Codex for inventory, not edits.

Output:

- list all docs,
- classify by folder,
- detect status labels,
- detect stale vocabulary,
- detect broken links,
- detect duplicate subjects,
- identify docs that appear current but mention deprecated substrate.

No file moves.

### Phase 1 — Ownership map

Create a proposed map:

- canonical docs to keep,
- docs to merge,
- docs to archive,
- docs to delete,
- docs to rename,
- links that must update.

No file moves until human review.

### Phase 2 — Shallow structure migration

After review, move docs into the target structure.

Rules:

- keep filenames kebab-case,
- prefer stable filenames,
- update internal links in same commit/slice,
- do not rewrite technical content unless needed for path/status fixes,
- do not mix content rewrite with mass file movement.

### Phase 3 — Vocabulary cleanup

Search and classify stale terms:

- primaryMedia
- primary_media_candidate
- primary_media_facts
- SourceFacts
- source_file_facts
- library assets as current substrate
- prep as internal substrate
- deprecated/browser-row waveform ownership

Allowed occurrences:

- stale terminology translation tables,
- migration history,
- explicit archive docs.

Not allowed:

- active architecture/contract docs describing target implementation.

### Phase 4 — Canonical indexes

Create or update small index docs:

- `docs/architecture/index.md`
- `docs/library/index.md`
- `docs/workspace/index.md`
- `docs/product/index.md`
- `docs/decisions/index.md`

Indexes should point to canonical docs only, not every historical note.

## Codex cleanup prompt order

### Prompt 1 — Docs inventory and classification

Reasoning: High.

Scope:

- docs only,
- no edits except optional generated report under `docs/plans/docs-inventory.md`.

Acceptance:

- complete inventory,
- stale vocabulary list,
- proposed status classification,
- duplicate subject list,
- broken link list.

### Prompt 2 — Proposed move/merge map

Scope:

- produce plan only,
- no moves.

Acceptance:

- target paths for each doc,
- delete/archive/merge candidates,
- risk notes.

### Prompt 3 — Mechanical path migration

Scope:

- move/rename docs only,
- update links,
- no content rewrites besides status headers.

Acceptance:

- link check passes or broken links listed,
- stale vocabulary count unchanged except path/status edits,
- no content doctrine changes.

### Prompt 4 — Content cleanup in small batches

Scope:

- one domain at a time, starting with waveform/library identity docs.

Acceptance:

- active docs use current vocabulary,
- archive/history docs are clearly labeled,
- no implementation doctrine is lost.

## Target folder count rule

Prefer 6–8 durable folders. Avoid creating a folder for every small concept.

Good:

- `docs/architecture/waveform-column-v1.md`
- `docs/architecture/analysis-basis-hashing-v1.md`
- `docs/runtime/realtime-audio-page-handoff.md`

Bad:

- `docs/architecture/waveform/binary/column/v1/spec.md`
- `docs/library/evidence/playable/media/promotion/contract.md`

## Human review gates

Do not let Codex perform bulk deletion without a reviewed deletion list.

For each cleanup slice, require:

- files moved,
- files edited,
- stale vocabulary changes,
- broken link status,
- docs status labels changed,
- risks.

## Acceptance for full cleanup

- Active docs have clear status.
- Canonical docs are easy to locate.
- Folder structure has a small number of durable domains.
- Stale vocabulary survives only in translation/history/archive context.
- No broken links from moved docs.
- Planning docs do not masquerade as accepted contracts.
- Implementation prompts are not stored as active doctrine.
