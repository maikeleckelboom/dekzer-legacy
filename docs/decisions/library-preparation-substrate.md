# Library Preparation Substrate

**Status:** HISTORICAL / FUTURE ARCHITECTURE - not current schema authority
**Location:** `docs/decisions/library-preparation-substrate.md`

---

## Current Verdict

This document is retained only as historical and future architecture reference. It is not a canonical owner for current
schema, current table names, generated contracts, Rust types, TypeScript types, or implementation sequence.

The post-deletion baseline is authoritative through the accepted current substrate docs:

- source files and source lifecycle/integrity;
- `SourceFacts` observed evidence;
- `content_attachments`;
- `source_file_attachment_links`;
- primary-media candidates;
- exact-content track-identity candidates;
- track-identity decisions and review projections;
- contents/search/filter and navigation contracts.

Deleted asset, browser, preparation, capability, playlist, and segment surfaces are not current, not aliases, not
fallback paths, and not implementation targets.

Future preparation, analysis, waveform, work scheduling, workspace, playlist/crate, CUE, canonical track, room, hardware,
RT Flight Deck, and performance surfaces must define new contracts after this authority cleanup. They must consume the
current substrate instead of reviving deleted preparation/capability-era models.

---

## Historical Context

This document came from an earlier schema epoch that tried to describe a broad v1 substrate for source inventory, track
identity, attachments, preparation work, readiness, and browser projections in one decision.

Useful lessons remain:

- files are not tracks;
- source-file observation, content identity, musical identity, preparation evidence, user decisions, and renderer
  projection need separate owners;
- readiness is a projection from evidence and decisions, not a durable truth column;
- preparation dimensions should be independent facets, not one status field;
- work outputs need basis, producer, version, and invalidation metadata;
- identity proposals and user decisions must be auditable and reversible;
- browser rows and workspace surfaces are projections, not source authority.

Those lessons are architectural pressure only. They do not ratify any schema shown by the old document and do not grant
authority to old vocabulary.

---

## What This Document No Longer Owns

This document does not own:

- current schema authority;
- current file, attachment, primary-media, or track-identity table names;
- preparation facet table names;
- waveform, analysis, artifact, work-run, or work-item table names;
- browser projection tables;
- workspace topology;
- playlist, crate, CUE, canonical track, or Prepared Room schema;
- implementation migration order;
- generated protocol or TypeScript contract names.

Any future document may cite these notes for rationale, but it must restate the current target identity, evidence basis,
decision behavior, read projection, invalidation rules, and implementation boundaries in its own contract.

---

## Non-Authoritative Architecture Notes

### Files Are Not Tracks

A source file is an observed inventory fact. It can have path, size, mtime, presence, and observed evidence. It is not a
track and must not be promoted into musical identity by path, filename, or hash alone.

Current authority for this separation lives in the source-file, observed-facts, attachment identity, primary-media, and
track-identity candidate/decision contracts.

### Content Identity Is Not Musical Identity

Exact bytes can support attachment identity and exact-content candidates. They do not prove same-song identity across
encodes, metadata variants, CUE structures, imports, or user intent.

Current exact-content candidates and decisions are deliberately non-canonical. Canonical track identity remains future
work and must be contracted separately.

### Preparation Is Multi-Facet

Preparation should eventually model independent readiness dimensions such as BPM, key, beatgrid, waveform, cues, loops,
phrases, loudness, energy, stems, notes, and tags.

That future model must define stable target identity, evidence basis, artifact ownership, user decision behavior,
readiness projection, invalidation, conflict handling, and read boundaries. It must not inherit table names or state
vocabulary from deleted preparation/capability substrate.

### Analysis And Waveform Are Backend-Owned Evidence

Waveform and analysis work should eventually be artifact-backed, basis-bound, stale-aware, and attributable to producer
versions. Renderer-generated throwaway state is not durable preparation authority.

Detailed waveform and analysis contracts are intentionally absent here. They come after this authority cleanup and must
project the current attachment, primary-media, candidate, and decision substrate.

### Work Outputs Need Provenance

Future work scheduling may need runs, items, artifacts, supersession, cancellation, retry, and bounded execution
semantics. Those concepts are not current schema authority. Any future work model must declare its owner, durability,
failure states, invalidation, retention, and relationship to source maintenance.

### Readiness Is A Projection

Readiness should remain a target-specific projection from evidence, artifacts, decisions, and policy. It should not be a
stored belief that silently drifts from its basis.

The exact readiness vocabulary belongs to the future domain contract that owns the target. This document does not define
current readiness states.

### Browser And Workspace Rows Are Projections

Navigation, contents rows, browser representations, and workspace layout surfaces should project authoritative substrate
state. They must not become hidden owners of source identity, media identity, preparation, track identity, or user
decisions.

Current navigation and contents authority lives in the library tree, browse, contents, and source ordering contracts.
Future workspace layout contracts must project those authorities instead of creating parallel state.

---

## Re-Contracting Requirements

Before future preparation, analysis, waveform, workspace, playlist/crate, CUE, canonical track, Prepared Room, RT Flight
Deck, hardware, or performance work is implemented, its contract must define:

- current substrate inputs;
- durable owner, if any;
- evidence basis and staleness behavior;
- candidate generation, if any;
- user decision model, if any, conforming to A-6;
- read projection and invalidation behavior;
- renderer limits;
- deletion and retention rules;
- tests proving the contract does not depend on deleted substrate.

No future slice may treat this historical document as permission to add tables, restore old vocabulary, or bypass the
accepted current substrate contracts.

---

## Safe Citation Rule

It is safe to cite this document only for high-level lessons:

- why file identity, content identity, track identity, preparation evidence, decisions, and projections must stay
  separated;
- why readiness should be projected from evidence;
- why future preparation and analysis need provenance and invalidation.

It is not safe to cite this document as current schema authority, current vocabulary authority, implementation backlog,
or a source of table shapes.
