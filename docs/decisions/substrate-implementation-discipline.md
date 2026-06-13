---
status: candidate
source-status: mined-from-superseded-brief
source-file: code_change_brief_v_1_substrate_port.md
last-reviewed: 2026-05-28
owner: product-architecture
purpose: Preserve durable implementation laws and process rules mined from the superseded V1 Substrate Port brief before deleting that stale brief.
authority: This document is not a substrate architecture contract. It is a reusable implementation discipline note. Current product/domain contracts remain authoritative.
---

# Substrate Implementation Discipline

This document preserves the still-useful rules mined from the superseded V1 Substrate Port brief.

The original brief is stale as an active implementation plan. It referenced an older substrate-port moment, older
authority documents, and a specific schema epoch. Do not use it for current planning. Delete or archive the original
after this document is saved.

This file keeps only the durable process laws that remain useful across Dekzer substrate work.

## 1. Greenfield replacement law

When a substrate model is replaced before production data depends on it, treat the replacement as a greenfield baseline.

Do not preserve old public vocabulary merely to keep intermediate code compiling.

Do not add aliases, wrappers, compatibility views, or dual paths unless there is real production data or an external
contract that must be honored.

For current Dekzer work, this means:

| Rule                     | Meaning                                                                    |
| ------------------------ | -------------------------------------------------------------------------- |
| No compatibility aliases | Do not alias old identifiers to new concepts.                              |
| No dual model            | Do not keep old and new substrate concepts alive as equal public surfaces. |
| No wrapper endpoint      | Do not preserve an old boundary shape by wrapping new internals.           |
| No fake migration bridge | Do not build migration machinery for data that does not exist yet.         |
| No renderer repair path  | Renderer code must not invent or repair substrate observations.            |
| No projection authority  | Disposable projection rows are not durable state.                          |

If a file still needs old vocabulary to compile, that file has not been ported. Do not hide the failure with naming
plaster.

## 2. Migration slice discipline

Large substrate changes must be sliced by ownership layer, not by whatever happens to compile first.

Preferred order:

| Slice                         | Purpose                                                         |
| ----------------------------- | --------------------------------------------------------------- |
| Schema and validation         | Land the durable shape and cross-table invariants.              |
| Domain vocabulary and IDs     | Make Rust/domain names match the schema and ownership model.    |
| Store authority paths         | Port writes, reads, validation, and projection generation.      |
| Boundary protocol and service | Expose new vocabulary without preserving old public names.      |
| Desktop/generated fallout     | Update immediate TypeScript/main/preload/shared fallout.        |
| Legacy deletion sweep         | Remove stale names from active code, tests, fixtures, and docs. |

Each slice should either compile or produce a deliberately bounded failure frontier.

Do not let a substrate slice expand into renderer redesign, organization modeling, import/export modeling, sleeves,
requests, devices, decks, RT Flight Deck, or other product domains unless that is the explicit task.

## 3. Read-first rule

Before changing substrate code, read the current canonical docs and inspect the actual repo shape.

Use current equivalents, not stale filenames from old briefs.

Minimum read-first set for substrate work:

| Area                   | Current source of authority                 |
| ---------------------- | ------------------------------------------- |
| Product position       | Product doctrine.                           |
| First slice            | First-slice substrate map.                  |
| Source scanning        | Source-root scan admission contract.        |
| Job behavior           | Job authority map.                          |
| Current implementation | Actual crate/package/app files in the repo. |

Do not assume a crate, package, module, or file exists because an older plan named it.

## 4. No-old-public-vocabulary law

When a concept is replaced, old public names must disappear from active implementation surfaces.

This applies to:

| Surface              | Requirement                                                        |
| -------------------- | ------------------------------------------------------------------ |
| Schema               | No old table/column names in active baseline.                      |
| Rust domain types    | No old IDs/types kept as aliases.                                  |
| Boundary protocol    | No deprecated fields or wrapper commands.                          |
| Generated contracts  | No old public contract names.                                      |
| Renderer-facing DTOs | No old names unless the old concept is still truly active.         |
| Tests                | No compatibility tests proving stale behavior.                     |
| Docs                 | Old vocabulary only in clearly marked historical/archive material. |

Deletion is part of the migration. A port is not complete while stale names remain in active code.

## 5. Store authority laws

Substrate store logic must preserve ownership and invariant boundaries.

Durable lessons from the old brief:

| Law                                                | Meaning                                                                                              |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Work runs own work items                           | A work item belongs to an execution context.                                                         |
| Work items produce artifacts                       | work_artifacts are evidence outputs, not free-floating observations.                                 |
| Artifact supersession is an edge                   | Do not mutate history into disappearance.                                                            |
| Current pointers need validation                   | A current artifact pointer must match subject and facet/scope.                                       |
| Projections are disposable                         | Browser rows and derived views are rebuildable, not source authority.                                |
| Source discovery is not track creation by accident | Discovery may inventory source material, but identity creation requires explicit resolver ownership. |

These laws still apply even when the specific old table names are gone.

## 6. Boundary protocol laws

Boundary changes must not keep old concepts alive for convenience.

Rules:

| Rule                              | Meaning                                                                          |
| --------------------------------- | -------------------------------------------------------------------------------- |
| No old wrappers                   | A removed command should not survive as a wrapper around new internals.          |
| No deprecated fields              | Do not keep dead fields to make consumers temporarily happy.                     |
| No renderer dependency inversion  | Renderer needs must not force substrate authority into the UI.                   |
| Generated contracts must be clean | Public contract output should read like the new model was designed, not patched. |

If the renderer still needs old names, report the follow-up. Do not preserve the old protocol shape as a shortcut.

## 7. Contents boundary pagination law

Contents reads support cursor pagination. The following rules apply:

| Rule                                     | Requirement                                                                                |
| ---------------------------------------- | ------------------------------------------------------------------------------------------ |
| Contents reads support cursor pagination | Cursor identity is encoded, validated, and used for continuation.                          |
| Cursor field is active                   | It is real pagination support, not a deferred shape.                                       |
| Provided cursor must fail explicitly     | Return a non-success `cursorInvalid` result.                                               |
| Do not silently restart                  | A provided cursor must not be ignored and treated as page one.                             |
| Successful read may emit next cursor     | `nextCursor` is produced when more rows exist.                                             |
| Renderer may accumulate pages            | The contents boundary accumulates rows and sends cursors through `loadContentsPage`.       |
| Tree load-more is separate               | Tree load-more (`loadChildren`) and contents pagination (`loadContentsPage`) are distinct. |

Cursor identity is bound to:

| Cursor identity input | Why it matters                                                      |
| --------------------- | ------------------------------------------------------------------- |
| cursor version        | Prevents stale cursor interpretation.                               |
| scope                 | Cursor is only valid for the same browse/read scope.                |
| policy                | Visibility/filter policy changes invalidate cursor.                 |
| scopeDepth            | Descendant and immediate scope reads are not the same result space. |
| row profile           | Row shape affects continuation meaning.                             |
| media class set       | Media filters affect ordering and membership.                       |
| query/order identity  | Cursor depends on the exact result order.                           |
| last-row key tuple    | Continuation must resume from a deterministic ordering key.         |

Do not let unrelated substrate work break cursor pagination in the contents boundary.

## 8. Stop conditions for coding agents

A coding agent must stop and report instead of widening scope when:

| Stop condition                                                                       | Required response                                                             |
| ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| Canonical docs do not match current repo shape                                       | Report the mismatch and propose the smallest next slice.                      |
| A new crate/package seems necessary                                                  | Stop before inventing it. Explain why existing owners cannot absorb the work. |
| The task requires simultaneous schema, Rust, generated TS, renderer, and UI redesign | Split the work.                                                               |
| A compatibility alias seems necessary                                                | Stop. That is usually a sign of wrong slicing.                                |
| Old and new public boundaries would coexist                                          | Stop. The migration is not clean.                                             |
| Validation failures are unrelated                                                    | Report separately. Do not hide them inside the task.                          |
| Actual source files contradict the brief                                             | Trust the repo and current contracts, not stale prompt text.                  |

The right response to a stop condition is not to widen scope. It is to narrow the next move.

## 9. Final report contract for agents

Agent final reports should stay compact.

Required report:

| Item                       | Meaning                              |
| -------------------------- | ------------------------------------ |
| Files changed              | Exact paths.                         |
| Files deleted              | Exact paths.                         |
| Validation run             | Exact commands and results.          |
| Git status                 | Clean, staged, unstaged, or blocked. |
| Remaining stale vocabulary | Exact search terms or none.          |
| Risks                      | Only real unresolved risks.          |
| Suggested commit message   | Conventional commit format.          |

Do not paste large diffs. Do not narrate every refactor step. Report invariant-relevant decisions and validation
evidence.

## 10. What was intentionally not preserved

The superseded brief should not be kept alive through hidden fragments.

The following were intentionally not preserved as active instruction:

| Old material                           | Reason                                                        |
| -------------------------------------- | ------------------------------------------------------------- |
| Specific V1 substrate port sequence    | Stale implementation context.                                 |
| Specific schema epoch references       | Current schema/source docs must own that.                     |
| Old read-first filename list           | May point to stale docs.                                      |
| Old table/type target list             | Current schema/domain contracts must own names.               |
| Broad substrate-port scope             | Too wide for current source-root scan work.                   |
| Historical implementation command list | Repo scripts and current validation docs should own commands. |

Delete the old brief once this survivor doc is saved and any missing current references are handled.
