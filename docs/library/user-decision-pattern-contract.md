# A-6 User Decision Pattern Contract

**Status:** Ratified doctrine gate
**Scope:** Reusable decision pattern for future decision-bearing library features
**Non-goals:** Product UI, cleanup, relocation acceptance, CUE association, track identity merge behavior, Prepared Room behavior

This document is a contract, not a schema migration. It defines the rule set every later decision-bearing layer must
use so evidence, derived candidates, user decisions, and read projections do not collapse into one mutable product row.

## Core Concepts

### Evidence

Evidence is a persisted observed or derived fact from the substrate, scanner, probes, source inventory, attachment
identity, source integrity, or occurrence reads.

Evidence records what the system observed or derived under a basis. Evidence does not say what the user chose, which
copy is preferred, whether two items are the same musical track, whether cleanup is safe, or whether a relocation was
accepted.

Current examples include `source_files`, `SourceFacts`, `content_attachments`, `source_file_attachment_links`, source
lifecycle rows, attachment occurrence status projected from those rows, and track identity candidate evidence snapshots.

### Candidate

A candidate is a computed proposal derived from evidence.

A candidate is not a user decision. A candidate may appear, disappear, change status, or be superseded when evidence
changes. Candidate generation owns proposals only; it must not mutate evidence into acceptance, cleanup, merge, or
preference.

Current examples include source-file hash candidates, media-probe candidates, `primary_media_candidates`, and exact
content `track_identity_candidates`.

### Decision

A decision is an explicit user-authored or system-authored durable record with provenance, scope, target identity,
policy meaning, timestamp, and recomputation behavior.

A decision may resolve, suppress, rank, preserve, override, merge, or split a target only according to its declared verb
and feature contract. Decisions never rewrite evidence. Decisions never hide the fact that evidence changed underneath
them.

Current implemented decision records are the narrow exact-content track identity decisions in
`track_identity_decisions` and their evidence/source-scope snapshots.

### Projection

A projection is a read-side presentation that combines evidence, candidates, and decisions without becoming the owner
of any of them.

Projections may compute effective state, current/stale labels, review state, counts, summaries, and display rows. A
projection must be rebuildable from its owners and must not be treated as durable decision truth.

Current examples include attachment occurrence reads, track identity review candidates, source integrity facets,
contents reads, navigation rows, and generated renderer-facing contract shapes.

## Decision Verbs

These verbs are the reusable vocabulary. A feature may expose only a subset, but if it uses one of these words it must
use the meaning below or explicitly declare a narrower compatible meaning.

| Verb | Meaning | May target | Durable state | Candidate suppression | Recompute behavior | Invalidation or conflict |
| --- | --- | --- | --- | --- | --- | --- |
| `accept` | Authoritatively choose the target for the declared policy scope. | Candidate, occurrence, source location, association, facet target, future stable item reference. | Yes. | May mark that target resolved and may block incompatible automatic decisions in the same scope. | Survives recomputation as a decision; effective state may become stale, conflicted, invalid, or orphaned. | Target disappears, basis changes, same target is rejected/deferred, or another target is accepted in an exclusive scope. |
| `reject` | Authoritatively say the target must not be used for the declared policy scope. | Candidate, association, occurrence interpretation, facet target, future stable item reference. | Yes. | Yes, within the declared scope and generator. It must not delete evidence. | Survives recomputation and blocks reacceptance by automation unless superseded. | Target basis changes, a newer user decision supersedes it, or feature rules declare the rejection no longer applicable. |
| `defer` | Postpone a decision while preserving that the target was reviewed. | Candidate, review item, ambiguous association, future facet target. | Yes when exposed as a decision, as in current track identity v0. | May suppress system acceptance or repeated prompts in the declared scope; it is not rejection. | Survives recomputation and may become stale or shadowed. | Basis changes, target resolves to a different candidate, or a newer accept/reject supersedes it. |
| `ignore` | Stop surfacing a target or proposal in a bounded review context without deciding the underlying evidence is false. | Candidate, proposed source location, occurrence interpretation, future warning/conflict. | Yes when the suppression must survive sessions. | Yes, within the declared generator/version/scope. | Survives recomputation only for the declared heuristic, target, and basis scope. | Generator version changes, evidence basis changes beyond the declared scope, or a user explicitly unignores/supersedes it. |
| `override` | User/system authority selects a result that differs from a derived default. | Candidate, policy result, classification, association, facet target. | Yes. | May shadow derived candidates or projections; must not erase them. | Survives recomputation but must be rechecked against its evidence or candidate basis. | Basis changes, target no longer resolves, or another explicit decision has higher declared precedence. |
| `prefer` | Rank or select one target over alternatives without claiming alternatives are invalid. | Source file, attachment occurrence, playable source, association option, future facet target. | Yes when it affects durable product behavior. | No deletion. It may shadow lower-ranked alternatives in projections. | Survives recomputation while the preferred target resolves and remains compatible. | Preferred target becomes unavailable/invalid, exclusive scope has another preferred target, or evidence basis changes. |
| `pin` | Preserve a target's position, visibility, or selected role against automatic replacement. | Source, source location, source file, attachment occurrence, candidate, future stable item reference. | Yes. | Not by itself. It can block automatic replacement in the declared scope. | Survives recomputation until unpinned or invalidated. | Target no longer resolves, scope is deleted, or a conflicting unpin/split/merge/override supersedes it. |
| `unpin` | Explicitly remove or supersede a durable pin. | Existing pin decision or pinned target. | Yes when pin history is retained; otherwise it must at least supersede the current pin with provenance. | No. | Survives as the reason the pin is no longer current when history is retained. | Conflicts with a simultaneous current pin over the same target/scope. |
| `merge` | Declare that multiple decision targets should be treated as one product identity in the declared scope. | Future track identities, future stable item references, candidate groups. | Yes. | Suppresses incompatible split and duplicate candidates in the same scope. | Survives recomputation but must surface stale/conflict if member evidence or identity basis changes. | Any member target invalidates, a split decision targets the same set, evidence basis changes, or destructive consequences would follow without confirmation. |
| `split` | Declare that one target or candidate group must be separated into multiple product identities or parts. | Future track identities, CUE/media candidates, future facet targets. | Yes. | Suppresses incompatible merge candidates in the same scope. | Survives recomputation but must surface stale/conflict if basis or member targets change. | Merge decision targets the same members, basis changes, or resulting identities cannot resolve. |

No verb creates product UI behavior by itself. UI flows may request commands, but verb meaning is owned by the backend
contract and persisted decision records.

## Decision Target Identity

A decision must target a stable substrate identity, a declared candidate identity, or a future stable item reference. It
must not target a loose display label, row index, transient renderer row, sort key, mutable path string by itself, or
projection-only grouping.

Allowed target classes:

| Target class | Current support | Targeting rule |
| --- | --- | --- |
| Source | Implemented as `sources.source_id`. | Target by source id and, when relevant, source identity key/provenance. |
| Source locator | Implemented as `source_locators` keyed by source id; no separate locator id. | Future locator decisions must snapshot locator kind and locator identity. Do not target only display path text. |
| Source file | Implemented as `source_files.source_file_id`. | Target by source file id plus evidence basis when a decision depends on current bytes/path state. |
| Content attachment | Implemented as `content_attachments.attachment_id`. | Target exact-byte attachment identity, not a path or title. |
| Source-file attachment link | Implemented as `source_file_attachment_links.source_file_attachment_link_id`. | Target the durable link when the decision concerns one source-file occurrence of an attachment. |
| Occurrence evidence row/context | A-5 is read-projected from durable link/source/source-file evidence; no separate occurrence table exists. | Target the durable link/source-file/attachment tuple plus evidence basis/status snapshot. If a later occurrence table is added, target its stable row id. |
| Candidate identity | Implemented for primary-media and exact-content track identity candidates. | Target the candidate id and candidate kind/basis/key. Do not target display text for the candidate. |
| Track identity candidate | Implemented as `track_identity_candidates.track_identity_candidate_id` for exact primary-media content only. | Current accept/reject/defer commands target candidate id and snapshot candidate/evidence basis. Future track identity candidates must preserve the same rule. |
| Future prep facet target | Future-only. | Must define stable subject kind, subject id, facet key, policy/version, and recompute basis before product exposure. |

Future stable item references must be declared before use. If a feature needs decisions over canonical tracks, CUE
associations, prep facets, source-file preference, or imported identities, its first schema/read contract must define
the target identity class rather than borrowing a renderer row or legacy asset id.

## Provenance

Every decision record must capture enough provenance to audit who or what made it, why it applied, and how it should be
re-evaluated.

Required provenance:

- Actor: user, system, importer, maintenance job, or future service identity. Current track identity v0 encodes this
  narrowly through `decision_source` values such as `user_local_v0` and `system_exact_content_v0`.
- Origin surface or command: the backend command, importer, maintenance phase, or system production path that authored
  the decision.
- `created_at` and `updated_at`.
- Evidence basis or candidate basis: enough snapshot data to know what evidence/candidate the decision acted on.
- Reason or note only when the feature supports it. Current track identity v0 supports bounded optional user reason
  text and stores a non-empty normalized decision reason.
- Source command identity when applicable, such as request id, import run id, scan run id, work item id, or future
  command event id.

There are no hidden decisions. There is no silent merge. There is no automatic cleanup. No destructive decision may be
created without an explicit command whose contract names the destructive meaning.

## Recompute Behavior

Evidence changes do not rewrite decisions. Recompute may change effective state, conflict state, projection labels, or
candidate membership, but the decision record and its provenance remain inspectable until explicitly superseded or
removed by a contract that preserves auditability.

Existing implemented names:

- Track identity candidates currently use `active`, `stale`, and `superseded`.
- Track identity decisions currently use `accepted`, `rejected`, `deferred`, and `superseded`.
- Track identity read projections currently expose current/stale/no-current and review states such as `staleDecision`.

A-6 also uses these contract vocabulary states. They are not implemented enums unless a feature explicitly adds them:

- `active`: target resolves and the decision basis remains compatible.
- `shadowed`: decision still exists but a higher-precedence decision or narrower scope controls the projection.
- `conflicted`: decision cannot be applied without contradicting another decision, target, or evidence basis.
- `stale`: target still resolves but the evidence or candidate basis changed.
- `invalid`: target class or policy meaning is no longer valid for the feature contract.
- `superseded`: a newer explicit decision replaces this one.
- `orphaned`: the target no longer resolves.

Required recompute outcomes:

| Change | Required behavior |
| --- | --- |
| Candidate disappears | Preserve decisions. Mark or project them as stale, invalid, or orphaned depending on whether the candidate row remains addressable. Do not delete evidence or assume rejection. |
| Candidate basis changes | Keep the old decision basis. Do not transfer the decision to the new basis unless an explicit feature rule says how and records provenance. |
| Source becomes unavailable | Preserve source, source-file, attachment, candidate, and decision records. Projections may show unavailable or shadowed effective state. |
| Source returns | Recompute candidates/projections from current evidence. Decisions become active again only when their target and basis still match; otherwise surface stale/conflict. |
| Occurrence status changes | Treat status as context over evidence. Do not reinterpret an available/missing/offline occurrence as duplicate, relocation, preference, or cleanup. |
| Source file becomes missing | Preserve link/attachment evidence and decisions. Projections may show missing/stale. No deletion or safe-cleanup decision is implied. |
| Attachment hash evidence changes | A link may move to a new attachment or become stale. Decisions tied to the old attachment/link become stale/conflicted/orphaned; they do not transfer by path alone. |
| Decision target no longer resolves | Preserve the decision as orphaned/invalid unless a feature contract has an explicit audited retirement rule. |

## Conflict Rules

A conflict exists when a decision, candidate, or projection would assign incompatible meaning, ownership, mutation, or
resolution behavior to the same substrate target or to targets that imply incompatible product results.

Conflict classes:

- Same target, conflicting verb: for example current accept and reject, pin and unpin, merge and split, or two mutually
  exclusive overrides.
- Different targets imply incompatible result: for example two preferred source files in an exclusive playback-source
  scope, one merge decision and one split decision over overlapping members, or two accepted CUE associations where the
  feature contract allows only one.
- Evidence basis changed: the decision's evidence snapshot no longer matches current evidence.
- Candidate regenerated with different basis: the displayed candidate looks similar but has a new basis/key/member set.
- User decision versus system-derived candidate: the candidate may still be generated, but it must not override the
  user decision. Current track identity v0 gives user decisions precedence over system decisions.
- Future destructive action requiring confirmation: removal, cleanup, purge, merge with destructive side effects, or
  relocation acceptance must stop until an explicit confirmation command writes the decision.

No automatic conflict resolution is allowed unless the feature contract already declares the rule, precedence, and
provenance. A read model may choose the effective winner only as a projection; it does not erase losing decisions.

## Layer Boundaries

- SQLite/Rust durable substrate owns decision records when implemented.
- Candidate generation owns proposals only.
- Evidence authorities own observed and derived facts only.
- Read models project resolved decision state, conflict state, stale state, and summaries.
- Renderer surfaces may display evidence/candidates/decisions and request commands, but never own durable decision
  truth.
- Generated contracts expose commands and reads. They do not create semantics; semantics live in the source contract,
  store authority, and service mapping.
- App main/preload/shared TypeScript adapters may validate transport shape and forward commands, but they must not add
  product decision meaning.

## A-5 Attachment Occurrence Relationship

A-5 attachment occurrence evidence feeds A-6 by providing targetable occurrence context:

- attachment identity (`content_attachments`);
- source-file attachment links (`source_file_attachment_links`);
- source-file inventory and presence (`source_files`);
- source lifecycle and access state;
- occurrence status and link freshness in `readAttachmentSourceFiles`.

A-5 can say where the same bytes are currently or previously observed and whether an occurrence is available,
unavailable, missing, removed, blocked, current, or stale.

A-6 must not reinterpret A-5 occurrence evidence as:

- duplicate song;
- safe deletion;
- preferred copy;
- accepted relocation;
- canonical track;
- cleanup candidate;
- track merge.

Those meanings require explicit candidates and explicit decisions with A-6 target identity, provenance, recompute
behavior, and conflict rules.

## Future Feature Gates

Before CUE association, track identity, duplicate/relocation UI, prep facets, source-file preference, user overrides,
or future stable item references become product-facing, each feature must declare:

- evidence source;
- candidate generator, if any;
- decision verbs used;
- decision target identity;
- persistence owner;
- recomputation behavior;
- conflict behavior;
- read projection;
- renderer limitations;
- validation tests.

Minimum test coverage for any decision-bearing feature:

- decision command rejects invalid or mutable display targets;
- decision record snapshots actor/source/basis/provenance;
- recompute after evidence change preserves decision history and marks stale/conflict/orphan state;
- system candidates do not override current user decisions;
- conflicting decisions are surfaced, not silently resolved;
- destructive actions require an explicit command and do not run from evidence alone.

## Existing Implementation Inventory

This inventory was compiled from the targeted docs/code searches and the required store/protocol/service audit. It
classifies current and legacy decision-like concepts against A-6. It is not a migration plan.

| Concept | File/path | Current owner | Target object | Kind | Classification |
| --- | --- | --- | --- | --- | --- |
| Source registration admission outcomes (`registered`, `proposalRequired`, `rejected`) | `crates/library-store-sqlite/src/store/sources.rs`; `crates/library-boundary-protocol/src/commands/library_roots.rs`; `docs/library/source/root-admission-policy.md` | Source root admission service/store | Requested/canonical source root path; future source | Candidate/policy outcome | Conforming but narrow. Not an A-6 decision record; future confirmation/override must declare A-6 target/provenance. |
| Source registration proposals | `source_registration_proposals`; `crates/library-store-sqlite/src/store/sources.rs` | Source root admission store | Requested/canonical source root path | Candidate/proposal | Conforming but narrow. Proposal is not user acceptance. |
| Local root unregister | `crates/library-store-sqlite/src/store/sources.rs`; `crates/library-boundary-protocol/src/commands/library_roots.rs` | Source lifecycle store/service | `sources.source_id` | Durable configuration command | Conforming but narrow. Not reusable as generic delete/cleanup decision. |
| Source lifecycle states and relocation vocabulary | `docs/library/source/lifecycle-visible-state-contract.md`; `crates/library-store-sqlite/src/authority/roots/lifecycle.rs` | Source lifecycle authority | Source and source locator | Evidence/lifecycle projection | Conforming but narrow. Relocation acceptance remains future and must use A-6. |
| Source locations observed vs registered | `source_locations`; `docs/decisions/source-locations-lifecycle-contract.md`; `crates/library-store-sqlite/src/authority/sources/source_locations.rs` | Source-location authority | Source id plus relative path/source_location row | Evidence plus durable configuration | Needs future migration if exposed as general accept/ignore decisions. Current registered subpath pattern is compatible but predates A-6. |
| Source-location proposal suppression | `docs/decisions/source-locations-lifecycle-contract.md` | Future source-location authority | Source id plus relative path plus heuristic key/version | Future decision/suppression | Conforming but future-only. Must not overload hidden source locations. |
| Source navigation user order | `source_navigation_user_order`; `crates/library-store-sqlite/src/authority/sources/source_records.rs`; `source_locations.rs` | Source/navigation ordering authority | Source or source location | Durable ordering preference | Unrelated to A-6 `prefer`/`pin` unless a future feature explicitly reclassifies it. |
| Source file inventory | `source_files`; `crates/library-store-sqlite/src/authority/sources/source_files.rs`; `authority/ingest/discovery.rs` | Scanner/source inventory authority | `source_files.source_file_id` | Evidence | Already conforming. |
| Accepted source facts and accepted artifacts | `SourceFacts`; `crates/library-store-sqlite/src/authority/sources/source_facts.rs`; `authority/promotion/inspect_source.rs` | Source inspection/hash/probe authority | Source file plus basis/artifact | Evidence | Conforming but narrow. "Accepted" means accepted inspection artifact, not a user decision. |
| Source-file hash candidates | `crates/library-store-sqlite/src/store/source_file_hash.rs`; boundary source-file hash commands | Maintenance/hash authority | Source file | Candidate/work proposal | Conforming but narrow. Not durable user decision. |
| Source-file media-probe candidates | `crates/library-store-sqlite/src/store/source_file_media_probe.rs` | Maintenance/probe authority | Source file | Candidate/work proposal | Conforming but narrow. Not durable user decision. |
| Attachment identity and links | `content_attachments`; `source_file_attachment_links`; `crates/library-store-sqlite/src/store/attachment_identity.rs` | Attachment identity store | Content attachment; source-file attachment link | Evidence | Already conforming. |
| Attachment occurrence read | `crates/library-store-sqlite/src/read_models/attachment_identity.rs`; `docs/library/evidence/attachment-occurrence-model-readiness.md` | Attachment identity read model | Attachment/link/source-file/source status | Projection | Already conforming. Evidence/status only; no duplicate/relocation/cleanup meaning. |
| Primary media candidates | `primary_media_candidates`; `crates/library-store-sqlite/src/store/primary_media_promotion.rs` | Primary-media promotion authority | Content attachment plus source-file evidence | Candidate | Conforming but narrow. Audio evidence candidate only; not canonical track identity. |
| Exact-content track identity candidates | `track_identity_candidates`, members, evidence; `crates/library-store-sqlite/src/store/track_identity_candidates.rs` | Track identity candidate authority | Track identity candidate id plus BLAKE3 evidence key | Candidate | Conforming but narrow. Exact current primary-media content only. |
| Track identity decisions | `track_identity_decisions`, decision evidence, decision source scope; `crates/library-store-sqlite/src/store/track_identity_decisions.rs` | Track identity decision authority | Exact-content track identity candidate | Decision | Already conforming but narrow. Supports accept/reject/defer only and no canonical track. |
| System exact-content decisions | `produce_track_identity_decisions_for_source`; `system_exact_content_v0` | Track identity decision maintenance | Exact-content track identity candidate | System-authored decision | Conforming but narrow. User reject/defer blocks system; future system decisions need explicit provenance. |
| Track identity review candidates | `crates/library-store-sqlite/src/read_models/track_identity_review.rs`; protocol snapshot reads | Track identity review read model | Candidate plus effective decision | Projection | Already conforming. Backend-derived read state only. |
| Track identity decision commands | `crates/library-boundary-protocol/src/commands/track_identity_decisions.rs`; `crates/library-boundary-service/src/track_identity_decisions.rs`; desktop trackIdentity adapters | Boundary protocol/service/app adapters | Candidate id | Command surface | Conforming but narrow. Transport exposes accept/reject/defer without adding semantics. |
| Search/filter index and contents omission metadata | `docs/library/search-filter-substrate-contract.md`; `crates/library-store-sqlite/src/read_models/search_filter`; contents read docs | Search/filter and contents read authorities | Source-file/content read rows | Projection/index | Unrelated. Must not become ignore/reject authority. |
| Source integrity/collection health facets | `docs/library/health/source-integrity-read-model-contract.md`; source integrity reads | Source integrity read model | Source-scoped health facets | Projection | Already conforming. Read-only health; no decision creation. |
| Long-term role/classification assignments | `docs/decisions/media-role-classification.md` | Future architecture only | Future library item/file identity | Future decision/classification | Future architecture. Compatible with A-6 separation but not current schema authority. |
| Historical preparation substrate identity-resolution notes | `docs/decisions/library-preparation-substrate.md` | Future architecture reference only | Future track identity/preparation facets | Future candidate/decision pattern | Compatible as high-level lessons only. Future implementation must use A-6 target/provenance/recompute gates and a new current contract. |

No audited current code path requires A-5 semantics to change. Existing conflicts are limited to dormant or future
vocabulary that must not be reused without an A-6-compliant implementation contract.
