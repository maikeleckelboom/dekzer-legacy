# Analysis Artifact Substrate V1

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define the proposed substrate that future waveform, beatgrid, key, loudness, phrase marker, stem separation, and stem playback artifacts use in Dekzer.

This document is a proposal. It does not implement migrations or generated contracts.

## Vocabulary baseline

Use current dev-branch vocabulary:

| Current vocabulary | Meaning in this proposal |
| --- | --- |
| `source_files` | Observed filesystem entries and file inventory. |
| `source_file_observations` | Evidence/probe/observation layer about source files. |
| `content_attachments` | Content-addressable attachment inventory used as byte identity/provenance input. |
| `source_file_attachment_links` | Occurrence-to-attachment link. This may supply bytes, but does not own analysis artifacts. |
| `playable_media` | Current playable media substrate target for full-mix V0 analysis. |
| `track_identity_candidates` | Candidate identity evidence, not canonical track identity. |
| `track_identity_decisions` | Human or decision records over candidates, not automatic semantic identity. |
| `work_items`, `work_runs`, `work_artifacts` | Existing general work/artifact substrate to extend unless human review rejects. |

Stale planning terms such as `primary_media_candidate`, `primaryMedia`, `primary_media_facts`, `SourceFacts`, and `source_file_facts` must not appear in target schemas. If encountered in older notes, translate them to the current substrate before using them.

## Core decision

Extend the current `work_*` substrate instead of introducing parallel `analysis_work_*` tables by default.

Reason:

- The repo already has work item, run, artifact, inline payload, file-store entry, and claim concepts.
- A second `analysis_*` family would likely duplicate scheduling, claiming, artifact state, and file-store ownership.
- Waveform and stem artifacts are not special enough to justify a parallel authority.
- The current substrate can become the generic artifact/work substrate if its target vocabulary, priority model, claims, and dependency model are expanded deliberately.

Human review may still veto this if the current `work_*` implementation is too source-maintenance-specific. That must be proven from repo code, not assumed.

Before extending current `work_*`, stop for a repo audit of existing migration files, schema modules, store access
paths, generated contracts, and worker/scheduler behavior. The extension remains a proposal until that audit proves
`work_*` can carry analysis artifacts without duplicating source-maintenance assumptions or becoming a hidden parallel
analysis schema.

## Target ownership

### V0 full-mix waveform target

The V0 full-mix waveform target should be:

- `target_kind = playable_media_full_mix`
- `playable_media_id`
- `content_attachment_id` or equivalent content attachment identity from the current schema

This replaces the old planning target:

- stale: `primary_media_candidate_id + attachment_id`
- current V0: `playable_media_id + content_attachment_id`

The content attachment identifies the content basis. Any source-file occurrence linked to that attachment may supply bytes. The artifact is not owned by a source-file occurrence.

### Future audio component target

`audio_component` is a future contract layer above `playable_media`, not a canonical track table.

It should represent analyzable/playable components such as:

- full mix component
- generated stem member component
- dedicated stem file component
- source-range/subrange component from future CUE or long-file material

Do not introduce `audio_components` as durable schema until the store schema arc has accepted:

- ownership
- target IDs
- source-range semantics
- stem set/member semantics
- relationship to `playable_media`
- relationship to `track_identity_decisions`

Until then, docs may say “future audio component target” or “logical audio component target.”

## Work item model changes

Future extension of current `work_items` needs these concepts:

| Concept | Required decision |
| --- | --- |
| Subject/target vocabulary | Must support `playable_media_full_mix`, future `audio_component`, `content_attachment`, `source_range`, and artifact targets. |
| `priority_order` | Numeric queue priority. Do not sort text priority names. |
| `enqueue_seq` | Stable FIFO order within priority. Do not sort queued work by `updated_at`. |
| `not_before_at` | Separate scheduled-readiness filter/index, not part of the main claim ordering index. |
| `compute_backend` | `cpu`, `gpu`, `cpu_or_gpu`, `io_bound`, or `hybrid`. Stem separation is GPU/hybrid, not a normal CPU waveform worker. |
| Lease fields | `lease_owner`, `lease_id`, `lease_until` or equivalent. Running jobs must recover if workers die. |
| Output artifact linkage | Runs need a clear producer relationship to artifacts. |
| Artifact dependency DAG | Artifacts need dependency rows and reverse dependency indexes for invalidation cascades. |
| Supersession | Artifacts can be current, stale, superseded, failed, partial, or ready. |
| Manifest/file-store fields | Large payloads live in artifact files, not SQLite JSON blobs. |

## Queue ordering law

Queue ordering must use:

1. `state = queued`
2. `priority_order ASC`
3. `enqueue_seq ASC`

`updated_at` must never participate in runnable queue ordering. Progress updates, lease heartbeats, and retries can update `updated_at` without moving a queued work item ahead or behind another item.

Recommended indexes at decision level:

    CREATE INDEX work_items_claim_idx
    ON work_items(state, priority_order, enqueue_seq);

    CREATE INDEX work_items_scheduled_idx
    ON work_items(not_before_at)
    WHERE state = 'queued' AND not_before_at IS NOT NULL;

Do not put `not_before_at` between `priority_order` and `enqueue_seq` in the main claim index.

## Work claim and recovery law

A running work item must be leased. Startup and supervisor recovery must:

1. detect stale workers,
2. expire old leases,
3. mark orphaned runs failed or cancelled with a named reason,
4. requeue retryable work if attempts remain,
5. mark temp artifact directories as garbage.

Without this, a waveform or stem job crash can permanently strand work in `running`.

## Progress write law

Progress writes to SQLite are coarse snapshots:

- at most once per 500 ms per worker,
- on phase transitions,
- on completion/failure/cancellation.

Never write progress per decoded PCM block, FFT frame, waveform column, tile parse, or render frame.

## Artifact dependency DAG

The artifact dependency graph must support forward and reverse lookup.

A future dependency table needs at least:

- artifact id
- dependency kind
- dependency id
- dependency role
- dependency basis hash if dependency is another artifact
- dependency content hash when content bytes are part of the basis

It also needs a reverse index:

    CREATE INDEX work_artifact_dependencies_dep_idx
    ON work_artifact_dependencies(dependency_kind, dependency_id);

This is mandatory for invalidation cascades, such as a stem separation model upgrade invalidating dependent stem waveform artifacts.

## Artifact ownership laws

- Source-file occurrence may supply bytes.
- Content attachment supplies content identity/evidence.
- Playable media or future audio component owns the analysis target.
- Artifact dependencies record the source/provenance chain.
- Browser rows do not own waveform state.
- Preparation rows do not exist as artifact owners.
- Track identity candidates do not become canonical tracks.
- Automatic track identity decisions do not become semantic identity.

## Artifact kind set

Initial artifact kinds to reserve in planning:

- `waveform`
- `beatgrid`
- `key`
- `loudness`
- `energy_profile`
- `phrase_markers`
- `stem_separation_bundle`
- `stem_playback_bundle`
- `stem_alignment_profile`
- `source_range_profile`

Not all need implementation now. The DAG must not make waveform-specific assumptions that prevent these later artifacts.

## Resource governor contract

The work queue is not the resource governor. A future ResourceGovernor must own:

- CPU worker leases
- GPU compute leases
- disk read tokens
- disk write tokens
- SQLite writer budget
- memory budget
- GPU upload budget
- live/performance mode gate

Priority hierarchy:

1. audio callback, outside the queue and absolute priority,
2. audio prefetch/decode for loaded decks,
3. render frame work,
4. visible waveform tile reads,
5. user-requested analysis,
6. background waveform analysis,
7. stem separation,
8. maintenance and garbage collection.

## Stop conditions

Do not implement schema until these are reviewed:

- the V0 target decision,
- whether current `work_*` can be extended cleanly,
- artifact dependency DAG shape,
- work claim/recovery model,
- artifact manifest/file-store ownership,
- generated boundary contract placement.
