---
status: candidate
canonical: true
doctrine-version: 0.3
last-reviewed: 2026-05-28
owner: product-architecture
authority: canonical product doctrine
supersedes:
  - docs/archive/product-doctrine.md
---

# Dekzer Product Doctrine

_Internal doctrine draft. Shortened version. Defines Dekzer's product position, substrate obligations, system contracts,
and modeling laws._

## 1. Core position

Most DJ software competes at the performance surface: decks, waveforms, effects, skins, mixer workflows, and controller
mappings. The durable pain lives below that surface.

DJs lose trust when libraries decay, preparation becomes trapped, metadata drifts, streaming behaves differently from
local files, exports fail, devices disagree, and the system cannot explain what is actually ready.

Current platforms each hard-code a worldview:

- Rekordbox has preparation, but it is welded to the Pioneer and AlphaTheta booth path.
- Serato has crates and performance speed, but not a serious long-term library substrate.
- Traktor has power, routing, and metadata depth, but leans into user-managed complexity.
- VirtualDJ has immense flexibility, but not a clean professional preparation doctrine.
- djay has modern access and surfaces, but not durable pro-library authority.

Dekzer owns the lower layer: a local-first, inspectable library and preparation substrate that treats a DJ's collection
and prep work as long-lived professional assets.

The fader is not where readiness begins. The gig is won or lost in the music room before the first track loads.

## 2. Product promise

Dekzer is a local-first preparation and performance-readiness OS for DJs.

More precisely:

> Dekzer is a local-first, inspectable preparation ledger for live performance.

Ledger does not mean blockchain. It means durable record, provenance, change history, evidence, auditability,
reversibility, and accountable state.

A normal DJ app stores the current value. Dekzer must know why that value exists, where it came from, whether it still
applies, and whether it can be trusted tonight.

Plain promise:

> Dekzer helps DJs know what they have, what is ready, what changed, and what will work tonight.

Design flag:

> Will it work tonight?

Every schema, UI surface, preparation model, export path, compatibility check, and runtime behavior must answer that
question. If it does not help answer it, it is decorative fog.

## 3. Product stack

Dekzer has two visible layers:

- Performance surface: where DJs act.
- Dekzer substrate: where DJs become ready.

Decks, waveforms, mixers, skins, pads, effects, gestures, and controller workflows matter, but they are downstream of
the substrate. They must not become the first place the system discovers whether the music is safe.

Dekzer owns four substrate layers.

### Library

Answers: what do I have, where is it, what is it, which version is it, what is missing, what changed, and what can still
be trusted?

The library is not a list of files. It is a durable representation of musical assets, source locations, attachments,
identities, versions, fingerprints, and collection structure.

### Preparation

Answers: what work have I done, what was inferred, what did I approve, what changed, what is ready, what is stale, and
what is still suspect?

Preparation is not one object, mode, or status. It is a workflow plane made of independent facets: grids, cues, loops,
phrases, key, BPM, loudness, stems, sleeves, tags, notes, crates, transition candidates, practice history, export
readiness, and source readiness.

### Compatibility

Answers: will this work on this device, export target, streaming source, operating system, controller, firmware version,
room, or event setup?

Compatibility is not a boolean. It is an evaluated result against a target profile, produced from local state plus
versioned external knowledge.

### Live trust

Answers: can I depend on this under pressure, can the system explain risk before it hurts me, and can it distinguish
what it knows from what it does not know?

Live trust begins before performance. Runtime surfaces inherit confidence from preparation, compatibility, and verified
media state. They must not manufacture it.

## 4. Identity stack

Many future bugs hide inside vague use of the word "track." Dekzer must maintain a canonical identity ladder. Confusing
rungs is a data integrity error.

| Rung                 | Meaning                                                                   |
|----------------------|---------------------------------------------------------------------------|
| Musical work         | Abstract song or composition. May have many recordings.                   |
| Recording / master   | Specific realized version. A remix is a different recording.              |
| Track in library     | User's representation of a recording inside their collection.             |
| Media attachment     | Specific media object attached to the track. May change over time.        |
| File instance        | Specific file on disk or stream, tied to path, host, and mount state.     |
| Audio fingerprint    | Perceptual audio identity. Survives re-encoding.                          |
| File fingerprint     | Byte-level or structural hash. Changes with byte edits.                   |
| Analysis baseline    | Analysis state produced against a specific attachment at a point in time. |
| Preparation artifact | Cue, grid, loop, phrase, tag, or note scoped to a baseline.               |
| Export projection    | Target-specific representation of a track or collection.                  |
| Runtime use          | Actual instance loaded into a deck during performance.                    |

Laws:

- A preparation artifact belongs to an analysis baseline, not directly to a track.
- An analysis baseline belongs to a media attachment, not directly to a track.
- A media attachment belongs to a track and resolves to a file instance.
- A file instance is not identity. It is a location claim at a moment in time.
- A runtime use is a projection, not authority over library state.

The system must always know which identity rung a claim belongs to.

## 5. Claims and provenance

Many important Dekzer facts are claims, not plain values. A cue point may be engine-detected, imported, user-created,
edited, accepted, stale, rejected, exported, or scoped to an older media version.

Every substrate claim needs a lifecycle:

| Phase       | Meaning                                                  |
|-------------|----------------------------------------------------------|
| observed    | System noticed something, without judgment.              |
| inferred    | System derived a value from evidence.                    |
| imported    | Value came from an external system.                      |
| suggested   | Candidate offered for review.                            |
| edited      | User modified the value.                                 |
| accepted    | User approved the value for this scope.                  |
| verified    | Additional evidence confirmed it.                        |
| invalidated | Value no longer applies to its scope.                    |
| rejected    | User explicitly refused it.                              |
| superseded  | Newer accepted value replaced it, while history remains. |
| exported    | Value was committed to a specific export projection.     |

Core law:

> A claim is not native authority until its source, scope, confidence, and acceptance state are known.

A Rekordbox cue, an AI-detected cue candidate, a user-set cue, and a drift-invalidated cue cannot live as equal facts.
Collapsing phases into one flat table loses the ledger.

## 6. Readiness and evidence

Readiness is not stored as belief. It is projected from evidence.

A track does not own a readiness value. Readiness is computed from identity, attachments, analysis baselines,
preparation artifacts, compatibility evaluations, and export projections against a specific target context.

Readiness can change without user action when underlying evidence changes.

Readiness states:

| State      | Meaning                                                                      |
|------------|------------------------------------------------------------------------------|
| ready      | Required checks pass for this scope and target.                              |
| verified   | Ready, plus confirmed by additional evidence.                                |
| degraded   | Usable at reduced confidence. Some checks passed, some remain uncertain.     |
| stale      | Was ready, but a dependency changed. Re-evaluation required.                 |
| suspect    | Imported or inferred without enough verification.                            |
| unverified | Checks have not yet run.                                                     |
| blocked    | Hard dependency is unresolvable, such as missing file or unsupported format. |
| unknown    | Insufficient information to produce a verdict.                               |
| failed     | Check produced a definitive negative result.                                 |

Important transitions:

- unverified to ready when required checks pass.
- ready to stale when an attachment fingerprint changes.
- ready to stale when a compatibility catalog version changes for the active target.
- ready to blocked when the source file becomes unreadable.
- suspect to accepted when the user approves.
- stale to ready when re-evaluation passes.
- verified to stale when any verified dependency changes.

Evidence grades:

| Grade     | Meaning                                                      |
|-----------|--------------------------------------------------------------|
| declared  | Extension or container says it. No verification.             |
| detected  | System identified format by reading bytes.                   |
| computed  | Analysis produced a value.                                   |
| verified  | Secondary check confirmed a value.                           |
| tested    | Dry-run or simulated path confirmed operation.               |
| exported  | Value or file was committed to an export projection.         |
| confirmed | Output was loaded and confirmed on the actual target device. |

These grades are not interchangeable. File extension is not decoding. Export success is not device confirmation. The UI
may simplify this, but the substrate cannot.

## 7. Conflict resolution

Imported ecosystems and analysis engines will disagree. Dekzer must preserve conflict rather than overwrite it.

Rules:

- Conflicts are preserved, not overwritten.
- User acceptance resolves product-facing authority, not historical evidence.
- Rejected claims remain audit evidence unless the user purges them.
- Imported facts never silently replace accepted native facts.
- Conflicts between imported sources are surfaced for user resolution.
- A conflict is a fact about disagreement, not an error to hide.

The system may propose and rank resolutions. It must not apply them silently.

## 8. Prepared Room model

Dekzer's core mental model is the music room, not the deck.

Zones:

| Zone           | Meaning                                                                   |
|----------------|---------------------------------------------------------------------------|
| Cold archive   | Known material. Not necessarily performance-ready.                        |
| Nearby reserve | Material intentionally kept close, but not necessarily prepared.          |
| Prepared room  | Inspected, analyzed, organized material with visible readiness and drift. |
| Prepared crate | Durable, purpose-bound subset of prepared material.                       |
| Hot table      | Near-immediate candidate pool. Ephemeral by default, snapshottable.       |
| Live path      | Actual or planned performance sequence with runtime state.                |
| Shadow paths   | Actionable alternatives, recovery routes, and transition branches.        |

Rules:

- A hot table is ephemeral unless promoted to a snapshot.
- A prepared crate is durable.
- A smartlist may feed a prepared crate, but the crate owns its membership.
- A shadow path may become a live path at any moment.
- A live path is not a playlist. It has runtime state, history, and evidence.
- A request queue is incoming intent. It may influence a live path but does not own it.

Organization objects must not collapse into one type with a kind field. Manual collections, query collections,
materialized snapshots, ordered sequences, event queues, performance paths, and export projections have different
durability, mutability, authority, and semantic weight.

## 9. Core doctrine

Inspectable is mandatory. Dekzer stores not only cue points, grids, tags, and BPM values, but also observations, claims,
approvals, edits, invalidations, readiness projections, and evidence.

Library decay is a first-class enemy. Tracks can have versions, attachments, masters, edits, exports, remasters,
streaming substitutions, corrupted copies, transcoded copies, moved files, and baselines that no longer apply. Cue
points surviving a file drift of 8 ms may be unsafe.

Trust asymmetry sets the bar. A visual bug is a bug. A silent readiness lie is betrayal. Dekzer must surface uncertainty
early, while it is still fixable.

Local-first and externally aware are both required. The user owns the library and preparation state. External
compatibility knowledge arrives as versioned, updateable catalogs.

Import is repatriation, not middleware. Bring trapped work home from Rekordbox, Serato, Traktor, VirtualDJ, Engine DJ,
and others. Imported facts become claims first, not native authority.

The deck must never be the first safety check. By the hot table or live path, Dekzer should already know source state,
readability, format support, analysis state, cue readiness, sleeve state, streaming limits, compatibility, export
status, and degraded conditions.

Runtime surfaces are projections, not authorities. They may show, consume, and explain readiness. They must not invent
it.

## 10. Catalogs, performance mode, recovery, privacy, and jobs

### Compatibility catalog governance

Compatibility catalogs are published by Dekzer with curation and accountability. They are versioned, updateable,
cacheable, and pin-able.

A DJ must be able to freeze the compatibility knowledge set for a gig. Catalog updates must not silently downgrade a
prepared crate. If an update affects previous verdicts, affected projections become stale rather than silently
reassigned.

Hard rule:

> Compatibility catalogs are updateable, but compatibility state used for a gig must be pin-able, inspectable, and
> protected from silent mutation.

### Performance mode

Performance mode is a substrate commitment, not a UI mode.

When active:

- No automatic catalog updates are applied.
- No destructive metadata migration runs in the background.
- No analysis rewrite is triggered by file change detection.
- No background job may downgrade live-path readiness without surfacing the condition.
- No import session modifies accepted native facts.
- No schema migration runs that could affect in-flight state.

Passive observations may continue. The system may notice change, but it must not act on it until the DJ exits
performance mode.

### Backup, rollback, and recovery

A ledger without recovery is half a ledger. Operations that modify accepted preparation state require recovery coverage.

Required recovery coverage includes import sessions, batch metadata edits, scans and rescans, export snapshots,
preflight reports, database backup, device export, catalog pins, and analysis rewrites.

Destructive operations require explicit confirmation. Background operations that could affect accepted preparation must
be staged and surfaced before commit. Backups need manifests with schema and catalog versions.

### Privacy and local sovereignty

The DJ's library is sensitive professional material: promos, private edits, setlists, gig history, requests, sources,
folder structure, and working logic.

Dekzer must not require remote indexing to function. It must not send library contents, file paths, preparation state,
or play history to external services without explicit consent. Cloud sync, catalog lookup, external analysis, backup,
and collaboration are additive. They do not replace local authority.

### Job system doctrine

Jobs are substrate actors. Scanning, fingerprinting, audio analysis, imports, drift detection, stem generation,
compatibility evaluation, export, backup, and preflight generation all produce evidence, claims, artifacts, or
projections.

Jobs must be resumable, observable, cancelable, attributable, diagnostic on failure, and respectful of performance mode.
A job that detects drift does not resolve it. It produces a drift observation. Resolution requires user action or an
approved policy.

## 11. RT Flight Deck

Prepared Room answers:

> Was the music ready before the first track loaded?

RT Flight Deck answers:

> What happened during performance, why, and what evidence proves it?

RT Flight Deck is not the deck surface. It is the runtime evidence layer. It observes, records, and explains what
happens when preparation meets live execution.

It should eventually answer:

- What played?
- What was skipped?
- What was prepared but unused?
- What loaded cleanly versus under pressure?
- What did the system flag as suspect that turned out fine?
- What was flagged as ready but failed?
- What shadow path became a live path?
- Which transitions came from prepared crates, and which were improvised?

RT Flight Deck is not MVP, but the substrate must leave room for it now. Runtime evidence cannot be bolted on later if
the system was not built to carry it.

## 12. Modeling laws

Do not:

- Store important preparation values without provenance.
- Treat files as tracks.
- Treat track identity as file identity.
- Treat imported metadata as native authority.
- Treat compatibility as a boolean.
- Treat ready as a static property.
- Store readiness as belief.
- Let streaming sources masquerade as local attachments.
- Hide drift.
- Flatten candidates, accepted facts, rejected facts, stale facts, and exported facts into one bucket.
- Make the deck responsible for discovering substrate failure.
- Require the cloud to understand the user's own library.
- Hard-code external compatibility knowledge as timeless product truth.
- Use one organization object for crates, playlists, smartlists, snapshots, request queues, and live paths.
- Let a UI surface own durable meaning that belongs in the substrate.
- Allow jobs to silently overwrite accepted user work.
- Apply catalog updates during performance mode without explicit user action.
- Run destructive operations without a recovery path.
- Confuse claims across identity rungs.

## 13. Conceptual substrate families

These are not final table names. They define territory the data model must cover:

- source locations and literal source hierarchy
- media attachments and fingerprints
- audio fingerprints
- recording and track identities
- identity rung mappings
- claim records and lifecycle transitions
- collection objects: manual collections, query collections, materialized snapshots, ordered sequences, event queues,
  performance paths, export projections
- preparation artifacts, candidates, approvals, edits, and conflicts
- invalidations, drift detections, readiness projections, and evidence records
- import sessions and conflict resolutions
- job records and job evidence artifacts
- compatibility catalogs, versions, and target profiles
- preflight evaluations, export snapshots, and gig pins
- runtime evidence records
- backup manifests

Not all must be visible in the first UI. All must shape the schema.

## 14. Questions Dekzer must answer

A DJ should eventually be able to ask:

- Why do you think this track is ready?
- What changed since I last trusted it?
- Where did this cue come from?
- Which media version was this grid made against?
- Will this prep survive this export target?
- Which facts were imported, approved, suspect, or stale?
- What will break if I use this source tonight?
- What is safe locally but unsafe on this device?
- What did the system infer, and what did I explicitly confirm?
- What did I play last time, skip, or prepare but not use?
- Which catalog version produced this compatibility verdict?
- Can I return to the state before this import?

If Dekzer cannot answer these questions, it is not the product it claims to be.

## 15. First slice and MVP boundaries

The first serious Dekzer slice should not begin with decks. It should prove the substrate path:

- register a local root
- scan literal hierarchy
- persist source locations and hierarchy
- inventory media-relevant attachments
- classify readable media
- preserve durable file identity
- surface source hierarchy in the app
- reopen cleanly
- show what is known, unknown, and unsupported

The milestone is the first moment Dekzer can honestly say:

> I know where your music is. I know what I found. I know what I can read. I know what I cannot yet trust. I can reopen
> tomorrow and remember correctly.

Explicitly not in the first slice:

- deck runtime
- controller support
- streaming-first architecture
- AI claim authority
- full compatibility matrix
- event operations
- performance mode enforcement
- RT Flight Deck runtime layer
- cloud sync

These are sequenced, not abandoned. The substrate must be sound before higher surfaces carry weight.

## 16. What Dekzer is and is not

Dekzer is not:

- a skin over decks
- a streaming-first DJ toy
- middleware for other DJ apps
- a cloud account wrapped around a music folder
- a generic file browser
- a compatibility spreadsheet
- a database that happens to draw waveforms
- a system that hides uncertainty to look polished
- a product that discovers failure when failure is most expensive

Dekzer is:

> A professional memory system for music work.

Not a DJ app that also has a library. A library and preparation substrate that also has a performance surface.

Category:

> local-first, inspectable preparation ledger for live performance

Plain promise:

> Dekzer helps DJs know what they have, what is ready, what changed, and what will work tonight.

Flag:

> Will it work tonight?

Everything else serves that, or it is decorative fog.
