# Dekzer Product Roadmap and Substrate Authority Plan

**Status:** Living document — maintained alongside implementation
**Scope:** Full product spine from substrate to performance
**Rule:** Every layer has one owner, one durable representation, one read contract, and one reason to exist. Anything
stale or misleading gets deleted once its replacement exists.

**This document is not itself a schema contract. Layer-specific documents remain authoritative for implementation
details. This document defines sequence, dependency, vetoes, and long-term ownership.**

---

## Product Phasing

V0 is a minimal, reliable local DJ foundation:

- local source registration;
- local scan/indexing;
- source and folder browsing;
- descendant media scope by default;
- search, filter, and sort;
- clear missing and offline states;
- deterministic reads;
- stable renderer projection;
- later basic preparation and local performance workflow.

V0 has no hardware support, no streaming services, no product-backed Prepared Room UI, and no audience, social, AR, VR,
or hardware room surfaces.

Future room-based concepts are architectural compatibility targets, not V0 product scope. Current substrate work
supports them by preserving identity, availability, provenance, source isolation, stable references, evidence versus
decision boundaries, and future runtime diagnostic integrity. The first product bar remains a minimal, reliable local DJ
workflow.

---

## Next Executable Queue

The legacy deletion sequence is complete. Public legacy surfaces, internal store/schema/projection/domain residue,
dormant browser preference state, and stale audit documentation have been removed or revised. `source_navigation_user_order`
remains current as source/source-location navigation ordering.

The next sequence is documentation and contract authority first, then bounded implementation slices:

1. Documentation authority alignment over the post-deletion substrate.
2. Local browse entry point and local browse item contracts, including Default Music as a Music-specific
   companion and source admission handoff.
3. Performance bounding for current reads, maintenance, and projection paths.
4. Local browse renderer projection and UI only after entry point reads, item reads, and admission contracts stay
   distinct from admitted source rows.
5. Workspace topology stays later because the related design work is not ready yet.
6. Analysis and waveform contracts stay later over current attachment, playable-media, candidate, and decision
   authority.
7. Implementation slices only after the relevant authority contract exists.

This keeps the V0 local DJ foundation first: visible local entry points, explicit admission, explicit scan, deterministic
reads, and stable projection before broader workspace, preparation, waveform, or runtime surfaces.

The implemented local browse entry point read boundary is `readLocalBrowseEntryPoints`. It is a read-only entry point
boundary with shallow platform status resolution. The resolver is hardened for Windows V0 entry points and non-Windows
unsupported behavior. Admission actions from this read are display/action recommendations only; they are not admission
results and do not persist source state.

This slice adds the local browse item read boundary, `readLocalBrowseItems`, as a bounded pre-admission item read. It
reads immediate child items for a validated entry point root or local descendant parent path, marks exact admitted-source
duplicates as status, and does not register sources, create source substrate rows, or start scans. Renderer projection
and visible local browse UI follow after this slice. Workspace topology remains later. Waveform and analysis remain
later over current attachment, playable-media, candidate, and decision authority.

A-5 attachment occurrence remains the accepted evidence-only substrate feeding A-6. It does not decide duplicate song,
safe deletion, preferred copy, accepted relocation, canonical track, cleanup, or track merge.

Media probe observations, source integrity / collection health v0, playable-media promotion, exact-content track identity
candidates, candidate decisions, explicit user decision commands, and the review-candidates read model are already
implemented. Their accepted contracts are narrower than the later canonical media-candidate, canonical-track,
preparation, analysis, waveform, playlist/crate, workspace, and performance roadmap layers and do not satisfy those
future gates by themselves.
Current playable-media promotion and exact-content track-identity candidate/decision production are source-scoped bounded
maintenance units. Candidate reads and remaining counts are SQL-level bounded reads/counts, not full source-wide
candidate vectors truncated in Rust.

---

## Ratified Substrate

The following layers are canonical.

1. **Source lifecycle** — The app knows whether a source is known, mounted, accessible, scanning, failed, or blocked.
2. **Media-relevant file inventory** — The app knows which source files exist and which are media-relevant by current
   policy.
3. **Observed file observations** — The app can store basis-bound evidence about a source file. Staleness is tracked.
4. **BLAKE3 evidence** — The app can compute content evidence through backend-owned path resolution, bounded and
   scan-triggered in one maintenance unit; scheduler/drain behavior remains future work.
5. **Attachment identity v0** — The app can say: these source-file rows have the same bytes and map to the same durable
   content attachment.
6. **Media probe observations v0** — The app records accepted basis-bound audio probe summary observations through backend
   inspection work.
7. **Playable-media promotion v0** — The app promotes current audio evidence into backend-owned playable-media observations.
8. **Exact-content track identity candidate and decision v0** — The app groups exact current evidence, records
   backend/user candidate decisions, resolves effective decision precedence, and exposes review candidates without
   claiming canonical track identity.
9. **Source integrity / collection health v0** — The app can read source-scoped lifecycle, coverage, inventory,
   evidence-maintenance, attachment-integrity, and runtime-maintenance facets without creating a second health authority.

### Canonical schema

`content_attachments` — exact-byte identity keyed by `(content_hash_algorithm, content_hash_value)`. No `file_kind` and
no track columns. `first_observed_at` is frozen at first insert.

`source_file_attachment_links` — links source files to attachments. `UNIQUE(source_file_id)` enforces one current
materialized attachment per source file. `source_id` is copied source scope and is schema-guarded to match the linked
`source_files.source_id`. Staleness computed by join against current `source_file_observations`, not stored as a flag.

Materialization: `materialize_attachments_for_source(source_id, limit)` — store authority, called by bounded
service-owned scan/manual hash maintenance. Candidate admission applies source-scoped SQL ordering and limits before
Rust mutation work.

Read boundary: `readSourceFileAttachment`, `readAttachmentSourceFiles`, and `readSourceAttachmentSummary` — explicit
read-only identity reads. They do not hash, materialize, populate product contents projections, or claim a maintained
snapshot invalidation scope.

Outcome fields: `attachments_created`, `attachments_refreshed`, `links_created`, `links_replaced`, `links_refreshed`,
`skipped_stale_observations`, `skipped_no_blake3`, `skipped_no_observations`; remaining counts report bounded maintenance backlog
without requiring full source-wide candidate materialization in Rust.

### Deleted Non-Current Surfaces

The current baseline is greenfield. Removed asset, browser, preparation, capability, playlist, and segment surfaces are
not current, not aliases, not fallback views, and not implementation targets. Future media, track, preparation,
analysis, playlist, waveform, workspace, or performance work must build on the current source-file, `source_file_observations`,
attachment, playable-media, track-candidate, and track-decision substrate instead of reviving deleted tables.

`playableMedia` is current as a narrow read policy over `playable_media`; it remains non-default product
doctrine until workflow-filter ownership explicitly chooses it. It is not track identity and not a fallback source-file
browser.

### Explicitly deferred

- Orphaned `content_attachments` rows after link replacement — attachment garbage collection is future work
- Cascade behavior when a source file is removed from inventory
- durable scheduler/drain behavior for attachment materialization beyond one bounded service-owned maintenance unit
- precise attachment read invalidation scope

---

## End-State Layer Model

Seven layers. Each answers a distinct question.

### Layer 1 — Source

_Where music may live._

Sources, source lifecycle, source locations, source directories, source files, source scan events, access state.

> "Where is the user's music, can we access it, what did we observe there?"

### Layer 2 — Observed Observations

_Evidence about a source file._

File basis, BLAKE3 hash, media/container observations, tag observations, CUE parse observations, probe status,
stale/current evidence status.

> "What do we know about this exact file observation, and is that knowledge still valid?"

### Layer 3 — Attachment Identity

_Stable content evidence._

Content attachment (BLAKE3 keyed), source-file attachment link, attachment occurrence model (exact duplicate /
relocation / offline / backup-copy views), source-file occurrence preference.

> "Which source-file rows represent the same bytes?"

### Layer 4 — Media Candidate

_Playable or interpretable units derived from attachment evidence._

Audio/video file candidates, CUE document candidates, CUE-to-audio association evidence, split-track candidates from
CUE, multi-file association candidates.

Acoustic fingerprint evidence is a later enrichment over audio media candidates that feeds track identity — it is not
part of initial media candidate creation. See Layer 5 and C-2.

> "What media object could this evidence represent?"

### Layer 5 — Track Identity

_User and product musical identity — not a file._

Track candidates, canonical tracks, duplicate track evidence, user-confirmed decisions, provenance, conflict state,
merge/split decisions.

> "What musical item is this, and why do we believe that?"

### Layer 6 — Preparation

_Independent readiness facets — not one status._

Beatgrid, BPM, key, waveform, cues, loops, phrases, loudness, energy, stems, notes, tags. Each facet: owner, basis,
artifact/evidence, current/stale status, user-approved vs. machine-generated, readiness, conflict handling, provenance.

> "What is ready for performance, what is missing, what changed, and what can be trusted?"

### Layer 7 — Workflow / Product

_Local DJ workflow first; room and performance memory later._

V0 workflow starts with local source browsing, local library correctness, search/filter/sort, missing/offline clarity,
and stable reads. Later workflow layers add basic preparation, local performance workflow, and then the room-shaped
model: cold archive → nearby reserve → prepared room → prepared crates → hot table → live path → shadow paths. Sleeves,
routes, performance history, and RT Flight Deck remain future layers.

> "How does the DJ find, prepare, trust, perform, recover, and eventually evolve a collection?"

---

## Roadmap Architecture Decisions

These decisions are ratified as roadmap constraints. They are not schema contracts until their layer-specific contract
lands. Do not re-open without a written rationale and architectural review.

### Identity channels are separate and must not contaminate each other

| Channel           | Table                                          | Status             | Means                                           |
| ----------------- | ---------------------------------------------- | ------------------ | ----------------------------------------------- |
| Byte identity     | `content_attachments`                          | Current            | Same BLAKE3 hash = identical bytes              |
| Acoustic identity | future `acoustic_fingerprint_evidence`         | Planned (C-2)      | Same audio content, possibly different encoding |
| Track identity    | future `track_candidates` / `canonical_tracks` | Planned (D-1, D-2) | Same musical item                               |

Acoustic fingerprinting does not use `content_attachments`. It runs on known audio media candidates, not raw
attachments. It produces its own evidence table consumed by track identity.

### Staleness is computed, not stored — for observed-observation and attachment-link validity

No `is_current` or `is_stale` boolean column on observed-observation or attachment-link tables. Link status is computed by
joining to the current `source_file_observations` row and comparing `source_file_observations.content_hash_value` to the linked
`content_attachments.content_hash_value`. `source_file_attachment_links` does not store a duplicate hash copy. If
current `source_file_observations` for a source file has a different hash or no current BLAKE3 observation, the link is stale.

This law applies to evidence and link validity. It does not prohibit stored lifecycle state fields for entities where
state is an authority, not a cache — for example, user decision state, job status, or scan lifecycle.

### Hash-change merge policy

When a source file's BLAKE3 hash changes: delete the old link, insert a new link to the correct attachment. Old
`content_attachments` row survives. No soft-marking, no link history in v0.

### `file_kind` belongs on the link, not the attachment

`content_attachments` is pure content identity. `file_kind` is interpretation context from the source file; it goes on
`source_file_attachment_links`.

### Probe observations are required before product-facing duplicate/relocation surfaces

Exact-byte occurrence queries can be built internally before probe observations exist. No product-facing duplicate or
relocation view surfaces until probe observations are available. Without probe observations, BLAKE3 cannot distinguish a
playable audio file from a corrupt container.

### Duplicate and relocation are one substrate, not two

Both are filter views on the attachment occurrence model:

- **Exact duplicate** — one attachment, multiple current present source-file occurrences
- **Relocation** — one attachment, old occurrence stale/missing, new occurrence current
- **Offline** — known occurrence, source currently unavailable
- **Backup/copy** — occurrences across different source roots

Build one occurrence model. Apply filters. Do not build two separate models sequentially.

### User decisions are always separate records from evidence

Machine evidence and user decisions must never share a row. Every layer that accepts user input must follow this
pattern:

- **Evidence record** — what the system observed or inferred
- **Decision record** — user accepts / rejects / overrides / prefers / pins
- Decisions reference the evidence row and basis they acted on
- Decisions carry provenance (when, by what)
- Machine recomputation does not silently overwrite user decisions
- Conflict behavior when evidence changes under an existing decision must be defined per layer

This is a doctrine constraint, not a single shared table. Each layer implements it in its own scope, conforming to the
pattern contract.

### Acoustic fingerprinting comes after media candidates

Acoustic fingerprinting requires decoded audio. It runs on known audio media candidates, not raw attachments. Placing it
before media candidates is wrong. Its output is an enrichment input into track identity evidence.

### CUE schema requires corpus audit first

CUE parse observation schema must not be designed from the spec alone. Real-world cases: single-file, multi-file,
embedded FLAC CUESHEET, wrong filename casing, missing referenced files, INDEX/GAP edge cases, non-standard encodings,
hidden pregaps, multi-disc archives. The Thunderdome FLAC+CUE collection is a canonical audit corpus.

### Search/filter substrate requires a design contract before browser surfaces

SQLite FTS5 or equivalent must be a deliberate architectural decision. Indexing strategy, filter composition,
rebuild/staleness behavior, and pagination must be defined before any product browser surface is implemented. The
renderer must not own search truth.

### Import interoperability is a design constraint on track identity and prep facets

Serato/rekordbox/Traktor import data (cues, loops, beatgrids, playlists, ratings) must be receivable as provenance-bound
evidence. Imported cue/loop/beatgrid/playlist data affects both track identity candidate schema and prep facet schema —
not just prep. The import interoperability contract must land before track identity candidates and prep facets harden.
Import data is never silently canonical; user confirmation is required for promotion.

### RT Flight Deck and performance sessions are future compatibility constraints

The RT Flight Deck creates requirements for: event log shape, evidence provenance, runtime observation history, user
decisions, performance sessions, incident reconstruction. These are not V0 product surfaces and do not justify backend
slices by themselves. A performance session entity (what was loaded, what played, what cues fired, what transitions
happened, whether prep observations held, anomalies, recovery) must be sketched in a doctrine doc before runtime event design
begins.

### No automatic cleanup, removal, or merge without a user decision record

Dekzer may surface evidence of duplicate bytes or moved content, but must never perform cleanup, removal, merge, or
source-file forgetting without an explicit user decision record. This applies at every layer. Occurrence model surfaces
are diagnostic evidence — not action recommendations. The duplicate and relocation views exist to inform the user, not
to drive system action.

---

## Implementation Roadmap

### Slice type markers

- **[CODE]** — implement store/service/read model/tests
- **[DOCTRINE]** — write contract/doc that blocks future code from drifting; no implementation; produces a named
  deliverable doc
- **[AUDIT]** — inspect, classify, and decide before code changes; produces a doc artifact that constrains future schema
- **[MODEL]** — formalize states/transitions/membership before implementation

### Implementation status labels

Each slice carries one of:

- **Ratified** — already implemented and accepted
- **In progress** — currently being implemented
- **Planned** — approved direction, not yet implemented
- **Doctrine gate** — must be written before dependent implementation begins
- **Blocked** — depends on a named earlier slice or gate

Later items must not begin before the items they depend on are ratified.

---

### Phase A1 — Durable Collection Evidence

Goal: Dekzer can know what exists locally and preserve file/content continuity.

---

**A-1 [CODE] Service-owned attachment materialization maintenance** [Ratified]

Wire bounded attachment materialization into service-owned maintenance after BLAKE3 hash evidence is produced.

- Bounded — same batch discipline as hash maintenance
- Service-owned — not test-only store method
- Not a draining synchronous unit on scan completion
- Manual `hashSourceFilesBlake3` can trigger materialization for the same source
- No UI, no track identity, no CUE pairing, no playableMedia activation
- Scan-triggered maintenance performs at most one hash pass and one attachment materialization pass, then clears the
  pending source request.
- Remaining hash and attachment materialization candidates are explicit-command or future-scheduler work.

Completes: `scan → source files → BLAKE3 observations → attachment links`

`feat(library): wire bounded attachment materialization into service maintenance`

---

**A-2 [CODE] Attachment identity read boundary** [Ratified]

Expose read-only attachment identity state through service/protocol boundary.

- Source file → current attachment link and status
- Attachment → all source-file occurrences with status
- Source → attachment materialization summary (current / stale / missing counts)

No materialization command exposed. No UI. No browser row population. No maintained snapshot invalidation scope is
claimed for attachment-only link changes.

**Note:** This is a prerequisite substrate surface, not a product-facing surface. Collection health (A-4) is the first
product-trust read model. No attachment-detail UI may be built before A-4 lands.

`feat(library): expose attachment identity read boundary`

---

**A-3 [CODE] Media probe observations v0** [Ratified]

Basis-bound probe evidence stored alongside source-file observations.

Fields: container, codec, duration, sample rate, channels, bit depth if available, bitrate if cheap, probe status,
adapter/version, stale/current status.

- Belongs to source files, not attachments
- Probe status distinguishes: probed / failed / unsupported / pending
- No tags yet, no track identity

`feat(library): add media probe observations v0`

---

**A-4 [CODE] Collection health / source integrity read model** [Ratified]

**Collection health is the first product-trust surface. The attachment identity read boundary (A-2) is a prerequisite
substrate surface, not a user-facing product. No attachment-detail UI may be built before this slice lands.**

First product-trust read model. Backend only.

Summarizes per source in v0. The collection-level aggregate is deferred as a pure rollup over source-scoped rows:

- Sources by lifecycle/access state
- Source files by presence state (present / missing / removed)
- Media-relevant files with no BLAKE3 evidence
- Media-relevant files with no probe observations
- Source files with stale source-file observations
- Source files with stale attachment links
- Unsupported/blocked/unreadable files
- CUE files pending future parse
- Attachment materialization backlog estimate

Implemented as the source-scoped `readSourceIntegrity` boundary. No collection-level aggregate is introduced in v0.

Must exist before any product-facing duplicate or relocation surface.

`feat(library): add collection health and source integrity read model`

---

**A-5 [CODE] Attachment occurrence model** [Implemented — ready for substrate audit]

One substrate evidence read over existing attachment identity, source-file inventory, probe evidence, and source
integrity. No product-facing occurrence views may surface from this slice. The implementation guide is
`docs/library/evidence/attachment-occurrence-model-readiness.md`.

Primary motivation: local library correctness. The same audio/content may appear in multiple places because users copy
folders, attach backup drives, rotate external drives, or temporarily lose access to known sources. A-5 must preserve
offline occurrences, identify relocation candidates as evidence only, keep unavailable from collapsing into absent, and
avoid unsafe claims about what content represents or what should happen to it.

Secondary motivation: future room compatibility. Prepared Room, Performed Room, and RT Flight Deck references will later
need identity, availability, provenance, source isolation, stable references, and evidence/decision boundaries, but they
are not the product scope of A-5.

Interpretation filters over evidence:

- Same-content occurrences — one attachment, multiple current present occurrences
- Relocation continuity — same attachment, old occurrence stale/missing, new current
- Offline occurrences — known occurrence, source unavailable
- Backup/copy — occurrences across different source roots

Rules:

- Stale links excluded from current views by default
- CUE same-content evidence is CUE evidence, not audio evidence
- Language: "same content", "same bytes", "same attachment" — never product track claims
- No action recommendations

`feat(library): add attachment occurrence model`

---

### Phase A2 — Collection Access Foundations

Goal: Dekzer can serve product-facing access patterns and user decisions over the substrate.

A-6 must land before CUE association (B-4), track identity (Phase D), and prep facet schemas are designed. A-7 and A-8
may proceed in parallel with Phase B.

---

**A-6 [DOCTRINE] User decision pattern contract** [Ratified]

Not a table. A law that every subsequent layer must conform to.

Defines:

- Evidence is always a separate record from decision
- Decision kinds: accept / reject / defer / ignore / override / prefer / pin / unpin / merge / split
- Decisions reference the evidence row and basis they acted on
- Decisions carry provenance
- Machine recomputation does not silently overwrite decisions
- Conflict behavior when evidence changes under an existing decision

Deliverable: `docs/library/user-decision-pattern-contract.md`

**This gate is now in force before CUE association, canonical track identity, duplicate/relocation handling, and prep
facet schemas are designed.**

---

**A-7 [DOCTRINE] Search/filter substrate contract** [Doctrine gate]

Design decision, not implementation.

Defines:

- FTS approach (SQLite FTS5 or alternative) and rationale
- What gets indexed: source file paths/names, file kinds, attachment metadata; tags and tracks later
- Stale record handling in the index
- Rebuild strategy on source change
- Filter composition model (how filters compose with scopes and pagination)
- No renderer-owned search truth
- How search results cite their authority layer

Deliverable: `docs/library/search-filter-substrate-contract.md`

**This gate must land before any product browser surface is implemented.**

---

**A-8 [CODE] Search/filter index v0** [Blocked — requires A-7]

After A-7.

Initial index: source file path/name, file kind, attachment hash/summary, probe observations when present. Tags and track
metadata extend it later.

`feat(library): add search and filter index v0`

---

**A-9 [CODE] Source-file occurrence preference** [Blocked — requires A-4, A-5]

When one attachment has multiple source-file occurrences, define which is preferred for playback and prep. Source health
context from A-4 is required — occurrence preference cannot be resolved coherently without knowing whether a source is
unavailable, stale, blocked, missing, or backup-like.

- Source priority policy (local vs. unavailable vs. backup; documented default)
- User override support (designed, may not be fully UI-exposed yet)
- Playback and prep systems consume this preference
- No track identity

Note: the default preference policy must define how the system distinguishes "main library source" from "backup
source" — source trust/priority metadata may be required.

`feat(library): add source-file occurrence preference model`

---

### Phase B — Media Observations

Goal: Dekzer can inspect files and explain what it knows, with provenance.

B-1 and B-2 may proceed independently after A-3 — tag observations and the CUE corpus audit do not depend on each other.

---

**B-1 [CODE] Tag observation ledger** [Planned]

Raw tag evidence only. Not canonical metadata.

Fields: raw artist/title/album/date/genre/comment, embedded artwork reference if present, tag source/provenance,
adapter/version, stale status, conflicts between tag sources.

- Conflicting tag observations are stored, not resolved automatically
- Tags are evidence, not user metadata, not track identity

`feat(library): add tag observation ledger`

---

**B-2 [AUDIT] CUE corpus audit** [Planned]

Before any CUE parse schema. Use real collection material (Thunderdome FLAC+CUE archive).

Classify:

- Single-file CUE
- Multi-file CUE
- Embedded FLAC CUESHEET
- Wrong filename casing
- Missing referenced audio files
- INDEX/GAP edge cases
- Non-standard encodings
- Hidden pregaps
- Multi-disc archives

Output: `docs/library/cue-corpus-audit.md` listing real cases found and constraints on parse schema.

---

**B-3 [CODE] CUE parse observations** [Blocked — requires B-2]

After B-2. CUE source file owns its parse evidence.

Schema driven by audit output. Minimum: parse status, referenced file declarations, track entries, indices,
titles/artists from CUE, parse warnings, adapter/version, basis binding.

No pairing yet. No track identity.

`feat(library): add CUE parse observations`

---

**B-4 [CODE] CUE-to-audio association evidence** [Blocked — requires B-3, A-6]

After B-3 and after the A-6 user decision pattern contract is in force.

Association is evidence, not inference. Supports:

- One CUE attachment → zero, one, or multiple candidate audio attachments
- Evidence kind and confidence
- Rejection reason
- User override (per A-6 user decision pattern)
- Missing audio file state
- Ambiguous audio file state

Path proximity may be one evidence input. It is never sole authority.

`feat(library): add CUE-to-audio association evidence`

---

### Phase C — Attachment-to-Media Candidate Layer

Goal: Group evidence into candidate media units without declaring final tracks.

---

**C-1 [CODE] Media candidate layer** [Planned]

Create playable/interpretable media candidates from attachment evidence.

Inputs: audio/video attachments, probe observations, CUE parse observations, CUE association evidence.

Candidate types: audio file candidate, video file candidate, CUE-derived track candidates, unsupported/blocked
candidates.

Does not create canonical tracks.

`feat(library): add media candidate layer`

---

**C-2 [CODE] Acoustic fingerprint evidence** [Blocked — requires C-1]

After C-1. Runs on audio media candidates, not raw attachments.

- `acoustic_fingerprint_evidence` table — separate from `content_attachments`
- Acoustic similarity groups are a separate relation
- Track identity consumes both BLAKE3 grouping and acoustic grouping

`feat(library): add acoustic fingerprint evidence`

---

### C/D Gate — Import Interoperability Contract [Doctrine gate]

**Before track identity candidate schema and prep facets harden.**

This gate belongs between C and D because imported cue/loop/beatgrid/playlist data affects both track identity candidate
schema and prep facet schema — not only prep. Placing it after D-1 or D-2 risks closing off the foreign-data ingestion
path before the schema that must receive it is designed.

Defines how Serato/rekordbox/Traktor import data becomes evidence and decisions, not silent canonical truth.

Constraints:

- Prep facets must support imported evidence
- Imported data is provenance-bound
- User confirmation required for promotion to canonical
- Foreign playlist/crate structures do not become Dekzer's internal doctrine
- Imported beatgrid/cue data can be stale if media candidate basis differs
- Mapping errors must be representable

Deliverable: `docs/library/import-interoperability-contract.md`

---

### Phase D — Track Identity

Goal: Create canonical track candidates from attachment and media evidence.

---

**D-1 [CODE] Track identity candidates** [Blocked — requires C-1, C/D gate]

Evidence-backed candidate tracks. The C/D import interoperability gate must land before this schema hardens — imported
track identity evidence must be receivable by the candidate schema from the start.

Consumes: media candidates, tag observations, acoustic fingerprints, CUE-derived candidates, user decisions (per A-6
pattern).

- No silent merging
- No filename-only or path-only authority
- Candidates carry full provenance
- Conflict state is representable

`feat(library): add track identity candidates`

---

**D-2 [CODE] Canonical track authority** [Blocked — requires D-1, A-6]

After D-1 and after A-6 user decision pattern is in force.

Canonical tracks: product-owned musical identities. Have one or more media candidates, one selected playable source,
alternates, user metadata, provenance, merge/split history, durable decisions.

`feat(library): add canonical track authority`

---

### D/E Gate — Future Prepared Room Formal Model [Doctrine gate]

**Before future room/preparation workflow schemas harden.**

This gate does not block substrate and probe work in Phases A–C, but it must land before Phase E implementation begins.
The formal model defines what "prepared," "nearby reserve," "hot table," and "shadow paths" mean in the data layer —
future concepts that will silently distort the prep facet schema and track identity UX if left implicit until Phase F.

Defines:

- States: cold archive / nearby reserve / prepared room / prepared crates / hot table / live path / shadow paths
- What "shadow path" means in performance context (fallback tracks? alternative routes?)
- What "hot table" means
- What "nearby reserve" means (curation act or prediction?)
- State transitions and what triggers each
- What moves automatically vs. what requires explicit user action
- Membership criteria for "prepared room" (which facets, which readiness level)
- Readiness relation between prep facets and room membership
- User decision interaction with room membership

Deliverable: `docs/library/prepared-room-formal-model.md`

---

### Phase E — Preparation, Analysis, and Waveform Contracts

Goal: Know what is ready for performance, what is missing, what changed, and what can be trusted.

---

**E-1 [DOCTRINE] Preparation, analysis, and waveform contracts** [Doctrine gate]

After the post-deletion authority cleanup, preparation/work/analysis/waveform concepts must be contracted again before
implementation. The future contracts must consume the current source-file, `source_file_observations`, attachment, playable-media,
track-candidate, and track-decision substrate. They must not restore deleted preparation, capability, waveform, or
segment models from earlier schema epochs.

Deliverables are intentionally separate contracts, not a single revived substrate:

- analysis work and artifact ownership;
- waveform artifact basis, storage, invalidation, and read behavior;
- preparation facet target identity, evidence, decisions, and readiness projection.

**E-2 [CODE] Preparation facets** [Blocked — requires D-2, C/D gate, D/E gate, E-1]

Independent facets: beatgrid, BPM, key, waveform, cues, loops, phrases, loudness, energy, stems, notes, tags.

Each facet requires: owner, basis, artifact/evidence, current/stale status, user-approved vs. machine-generated,
readiness, conflict handling, provenance.

Import evidence from the C/D gate contract must be receivable by each facet before this schema hardens.

Waveform is not a renderer-generated throwaway. It is artifact-backed, attachment/media-candidate based, basis-bound,
stale-aware, generated by backend worker, streamed/resource-backed to renderer.

`feat(library): add preparation facets`

---

### Phase F — Workflow / Product

Goal: product workflow after the local library foundation is trustworthy.

---

**F-1 [DOCTRINE] Performance session + RT Flight Deck constraints** [Doctrine gate]

Before runtime event design.

This is a future compatibility gate, not V0 product scope.

Defines:

- Performance session entity: what was loaded, what played, what cue/loop actions fired, what transitions happened,
  whether prep observations held under live use, anomalies, recovery actions
- Runtime event log shape
- Anomaly chain structure
- Incident evidence and reconstruction
- Feedback loops: session evidence → preparation intelligence; session evidence → route planning
- RT Flight Deck read model requirements

This document constrains Phase G runtime architecture. Design it early enough that event/evidence/session schema
decisions don't close off the Flight Deck.

Deliverable: `docs/library/performance-session-and-flight-deck-constraints.md`

---

**F-2 [CODE] Prepared Room first slice** [Blocked — requires D/E gate, D-2]

After the D/E gate (Prepared Room formal model) is ratified and D-2 canonical track authority exists.

- Cold archive / source inventory
- Prepared candidates
- Readiness facets
- Crate-like grouping
- "Not ready because…" explanations
- No live deck dependency yet

---

**F-3 [CODE] Sleeves and routes** [Blocked — requires F-2]

After Prepared Room first slice.

- Sleeves as workflow objects
- Routes as explainable performance paths
- Transition candidates
- Planned movement through energy/key/tempo/notes
- Not playlists

---

### Phase G — Deck / Runtime

Goal: The live system is trustworthy because the library substrate underneath it is trustworthy.

---

**G-1 [CODE] Deck runtime surface** [Blocked — requires E-1, F-1, F-2]

Only after library substrate is trustworthy.

- Deck runtime surface, waveform resources, transport state, cue/loop execution, controller integration
- Stable performance mode
- RT Flight Deck diagnostics

---

## Language Law

These substitutions are non-negotiable in product-facing surfaces, docs, and code comments.

| Do not use                                   | Use instead                                                                              |
| -------------------------------------------- | ---------------------------------------------------------------------------------------- |
| "same track"                                 | "same content", "same bytes", "same attachment"                                          |
| "duplicate song"                             | "same content evidence in multiple locations"                                            |
| "safe to delete"                             | Never recommend deletion                                                                 |
| "duplicate track"                            | Never. Describe occurrences only                                                         |
| "track moved"                                | "content attachment found at new source-file path"                                       |
| "cleanup", "remove duplicates", "auto-merge" | Never as system actions. Surface evidence only; require an explicit user decision record |

---

## Big Vetoes

| Do not do                                        | Until                                                                                                                                         |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Product-facing duplicate/relocation UI           | Attachment occurrence evidence (A-5) exists; actions also require A-6                                                                         |
| Track tables                                     | Media candidate layer (C-1) is ratified                                                                                                       |
| `playableMedia` activation                       | Media candidate / track identity layer is real                                                                                                |
| CUE-to-audio pairing                             | CUE parse (B-3) + association evidence (B-4) exist                                                                                            |
| Prep facets                                      | Canonical track/media identity (D-2) is durable and D/E gate is ratified                                                                      |
| Waveform UI                                      | Waveform artifact substrate is backend-owned                                                                                                  |
| Acoustic fingerprinting                          | Audio media candidates (C-1) exist                                                                                                            |
| Track identity schema hardened                   | C/D import interoperability gate lands first                                                                                                  |
| Auto-cleanup, removal, or merge                  | Never without explicit user decision flow and record                                                                                          |
| Product contents rows populated from attachments | Product contents projection is a future layer                                                                                                 |
| Renderer-owned library truth                     | Never                                                                                                                                         |
| Local browse projection/UI                       | Local browse entry point, local browse item, and root-admission contracts are in force; local browse rows stay distinct from admitted sources |
| Playlists as central workflow model              | Crates/sleeves/routes designed first                                                                                                          |
| Search from renderer                             | A-7 search/filter contract must land first                                                                                                    |
| Import as silent canonical                       | Always evidence; user confirmation required                                                                                                   |
| Import interoperability implementation           | C/D gate contract must land first                                                                                                             |
| Prepared Room implementation                     | D/E gate formal model must land first                                                                                                         |
| Runtime event design                             | F-1 Flight Deck constraints must land first                                                                                                   |
| Attachment-detail UI                             | Collection health (A-4) must land first                                                                                                       |

---

## Cleanup Policy

As new canonical layers land, dormant surfaces are deleted. No permanent placeholders.

**Rule:** When a layer is replaced, the old surface is deleted in the same slice — not queued for later, not aliased, not
silently kept.

---

## Strategic Principle

> **Local DJ foundation first. Exact byte identity first. Probe observations before product occurrence claims. Collection health
> before occurrence UI. Occurrence model before occurrence interpretation views. User decision pattern before track and
> prep schemas. Local browse entry points and items before local browse UI. Search/index contract before
> browse surfaces. Import interoperability contract before track identity and prep facet hardening. Future Prepared Room formal model before room
> workflow implementation. RT Flight Deck and performance session doctrine before runtime event design. No automatic
> removal or merge without an explicit user decision record.**
