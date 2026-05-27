# Dekzer Product Doctrine

_Internal doctrine draft. This document defines product position, substrate obligations, system contracts, and modeling
laws for Dekzer._

_Version 2. Expanded from v1 to include identity stack, claim lifecycle, readiness state model, evidence grades,
conflict resolution, catalog governance, performance mode, recovery doctrine, privacy laws, job system doctrine, and RT
Flight Deck connection._

---

## 1. The Spine

Most DJ software competes at the performance surface: decks, waveforms, effects, skins, mixer workflows, and controller
mappings.

The durable pain lives below that surface.

DJs lose trust when libraries decay, preparation becomes trapped, metadata drifts, streaming behaves differently from
local files, exports fail, devices disagree, and the system cannot explain what is actually ready.

The DJ app space does not only compete on the wrong layer. It also traps the right layer inside the wrong product
boundary.

- Rekordbox has preparation, but it is welded to the Pioneer and AlphaTheta booth path.
- Serato has crates and performance speed, but not a serious long-term library substrate.
- Traktor has power, routing, and metadata depth, but leans into user-managed complexity.
- VirtualDJ has immense flexibility, but not a clean professional preparation doctrine.
- djay has modern access and surfaces, but not durable pro-library authority.

Each platform hard-codes a worldview. The unresolved pain lives underneath.

**Dekzer owns that lower layer: a local-first, inspectable library and preparation substrate that treats a DJ's
collection and prep work as long-lived professional assets.**

The fader is not where readiness begins. The gig is won or lost in the music room before the first track loads.

---

## 2. The Position

Dekzer is a local-first preparation and performance-readiness OS for DJs.

It treats the library as a long-term musical asset, preparation as durable work, compatibility as something to prove
before the gig, and live trust as a product requirement rather than a lucky outcome.

More precisely:

> Dekzer is a local-first, inspectable preparation ledger for live performance.

Ledger does not mean blockchain. It means durable record, provenance, change history, evidence, auditability,
reversibility, and accountable state.

A normal DJ app stores what the current value is.

Dekzer must know why that value exists, where it came from, whether it still applies, and whether it can be trusted
tonight.

The plain promise:

> Dekzer helps DJs know what they have, what is ready, what changed, and what will work tonight.

That sentence is the whole product in human language. Everything else must serve it.

---

## 3. The Flag

**Will it work tonight?**

This is the design test.

Every schema decision, UI surface, preparation model, export path, compatibility check, and runtime behavior must answer
to it.

- Does this schema help answer it?
- Does this UI make it visible?
- Does this preparation model preserve the work?
- Does this export path prove compatibility?
- Does this runtime surface avoid inventing confidence it does not have?
- Does this feature expose uncertainty early enough for the DJ to act?

If the answer is no, the feature is decorative fog.

---

## 4. The Product Stack

```text
Performance surface    Where DJs act.
Dekzer substrate       Where DJs become ready.
```

The deck is the cockpit. The aircraft is underneath.

Waveforms, decks, mixers, skins, pads, effects, gestures, and controller workflows matter. They are downstream of the
substrate. They must not become the place where the system first discovers whether the music is safe.

Dekzer owns four substrate layers.

### 4.1 Library

What do I have? Where is it? What is it? Which version is it? What is missing? What changed? What can still be trusted?

The library is not a list of files. It is a durable representation of musical assets, source locations, attachments,
identities, versions, fingerprints, and collection structure.

### 4.2 Preparation

What work have I done? What was inferred? What did I approve? What changed? What is ready? What is stale? What is still
suspect?

Preparation is not one object, one mode, or one status. It is a workflow plane made of independent facets: grids, cues,
loops, phrases, key, BPM, loudness, stems, sleeves, tags, notes, crates, transition candidates, practice history, export
readiness, and source readiness.

### 4.3 Compatibility

Will this work on this device, export target, streaming source, operating system, controller, firmware version, room, or
event setup?

Compatibility is not a boolean. It is an evaluated result against a target profile, produced from local state plus
versioned external knowledge.

### 4.4 Live Trust

Can I depend on this under pressure? Can the system explain risk before it hurts me? Can it say what it knows, what it
does not know, and what changed since the last trusted state?

Live trust begins before performance. Runtime surfaces inherit confidence from preparation, compatibility, and verified
media state. They must not manufacture it.

---

## 5. Identity Stack

Many future bugs hide inside vague use of the word "track."

Dekzer must maintain a canonical identity ladder. Each rung is distinct. Confusing them is not a naming error — it is a
data integrity error.

```text
musical work          The abstract song or composition. May have many recordings.
recording / master    A specific realized version. A remix is a different recording.
track in library      The user's representation of a recording inside their collection.
media attachment      A specific media object attached to the track. May change over time.
file instance         A specific file on disk or stream. Tied to a path, host, and mount state.
audio fingerprint     Perceptual identity of the audio content itself. Survives re-encoding.
file fingerprint      Cryptographic or structural hash. Changes on any byte-level edit.
analysis baseline     The analysis state produced against a specific attachment at a point in time.
preparation artifact  Cue, grid, loop, phrase, tag, or note. Scoped to a specific baseline.
export projection     The representation of a track as it will appear on a specific target.
runtime use           The actual instance loaded into a deck during performance.
```

Laws that follow from this stack:

```text
A preparation artifact is not attached to a track. It is attached to an analysis baseline.
An analysis baseline is not attached to a track. It is attached to a media attachment.
A media attachment is not attached to a file. It is attached to a track, and resolved to a file instance.
A file instance is not an identity. It is a location claim about a specific moment.
A runtime use is a projection, not a source of authority about library state.
```

The question "Is this cue attached to the song?" has no single answer until the rung is specified. The system must
always know which rung a claim belongs to.

---

## 6. Claim Lifecycle

The doctrine says facts are claims with provenance. That is correct. But a claim without a lifecycle is just a row with
extra columns.

Every claim in the Dekzer substrate has a defined lifecycle phase.

```text
observed      The system noticed something. No judgment yet.
inferred      The system derived a value from evidence (analysis, heuristic, pattern).
imported      The value arrived from an external system.
suggested     The system is offering a candidate for user review.
edited        The user modified a value.
accepted      The user approved a value as authoritative for this scope.
verified      The value has been confirmed by additional evidence beyond its origin.
invalidated   The value no longer applies to its target scope (e.g., attachment changed).
rejected      The user explicitly refused this value.
superseded    A newer accepted value replaces this one. The old one is retained as history.
exported      The value was committed to a specific export projection.
```

The key law:

> A claim is not native authority until its source, scope, confidence, and acceptance state are known.

A Rekordbox cue, an AI-detected cue candidate, a user-set cue, and a drift-invalidated cue cannot all live as morally
equal facts. They occupy different phases of the lifecycle and carry different authority.

Collapsing phases into one flat table loses the ledger.

The lifecycle also defines what audit means. A rejected claim is not garbage. It is evidence. A superseded claim is not
an error. It is history. The system must retain lifecycle phase transitions as part of its record — not just the current
phase.

---

## 7. Readiness and Evidence Model

### 7.1 Readiness is a projection, not a belief

**Readiness is not stored as belief. Readiness is projected from evidence.**

A track does not have a readiness value stamped on it. Readiness is computed from the current state of its identity,
attachments, analysis baselines, preparation artifacts, compatibility evaluations, and export projections — against a
specific target context.

This means readiness can change without any user action, because the evidence underneath it changed.

### 7.2 Readiness states

```text
ready         All required checks pass for this scope and target.
verified      Ready, plus confirmed by additional evidence (e.g., test export, device load).
degraded      Usable, but at reduced confidence. Some checks passed, some are uncertain.
stale         Was ready. A dependency changed. Re-evaluation required.
suspect       Imported or inferred without sufficient verification.
unverified    Checks not yet run.
blocked       A hard dependency is unresolvable (missing file, unsupported format).
unknown       Insufficient information to produce a verdict.
failed        A check produced a definitive negative result.
```

### 7.3 Readiness transitions

Transitions are not arbitrary. They follow from evidence events.

```text
unverified -> ready        when all required checks pass
ready -> stale             when attachment fingerprint changes
ready -> stale             when compatibility catalog version changes for the active target
ready -> degraded          when target profile changes and local use remains safe
ready -> blocked           when source file becomes unreadable
degraded -> stale          when a previously uncertain check resolves negatively
imported -> suspect        when source file cannot be matched to current attachment
suspect -> accepted        when user reviews and approves
stale -> ready             when re-evaluation passes
stale -> blocked           when re-evaluation reveals hard failure
verified -> stale          when any verified dependency changes
failed -> blocked          when failure is confirmed as irrecoverable without user action
```

Without defined transitions, readiness becomes a label the system applies optimistically. That is the failure mode
Dekzer exists to prevent.

### 7.4 Evidence grades

"Will it work tonight?" requires proof. Not all proof is equal. Dekzer must track the grade of evidence behind any
readiness verdict.

```text
declared      The file extension or container says it. No verification performed.
detected      The system identified the format by reading actual bytes.
computed      Analysis produced a value (BPM, key, waveform, fingerprint).
verified      A secondary check confirmed a computed or declared value.
tested        A dry-run or simulated path confirmed the operation would succeed.
exported      The value or file was committed to a specific export projection.
confirmed     The output was loaded and confirmed on the actual target device.
```

These are not interchangeable. A file extension declaring MP3 is not equivalent to decoding the audio. A successful
export is not equivalent to loading on a CDJ. A compatibility catalog saying "supported" is not equivalent to a device
test.

Every readiness verdict must record its evidence grade. The UI may simplify that for the user, but the substrate cannot.

---

## 8. Conflict Resolution Doctrine

When repatriating trapped work or integrating multiple analyses, imported ecosystems will disagree.

Example:

```text
Rekordbox says BPM 128.00.
Serato says BPM 127.99.
Audio analysis says 128.02.
The user manually approved 128.00 last year.
The file fingerprint changed last week.
```

Dekzer's behavior in this situation must be defined, not improvised.

**Conflicts are preserved, not overwritten.**

**User acceptance resolves product-facing authority, not historical evidence.**

**Rejected claims remain audit evidence unless explicitly purged by the user.**

**Imported facts never silently replace accepted native facts.**

**Conflicts between imported sources are surfaced for user resolution, not resolved by system preference.**

**A conflict is a fact about disagreement. It is not an error to be hidden.**

This is ledger behavior. The ledger does not forget that a disagreement existed. It records what the user decided, when,
and against what evidence state.

The system may propose resolutions. It may rank candidates by confidence. It must not apply resolutions silently.

---

## 9. The Prepared Room Model

Dekzer's core mental model is the music room, not the deck.

```text
Cold archive
Nearby reserve
Prepared room
Prepared crates
Hot table
Live path
Shadow paths
```

This is not decorative language. It describes how DJs actually work.

### 9.1 Semantic contracts

Each zone has a defined meaning.

**Cold archive** — Known material. Not necessarily performance-ready. May include unanalyzed files, archived sets,
reference material, or music under consideration. The system knows it exists. It does not claim it is ready.

**Nearby reserve** — Material intentionally kept close. Curated for potential use. Not necessarily prepared. The DJ has
made a proximity decision, not a readiness decision.

**Prepared room** — Material that has been inspected, analyzed, organized, and made useful. Readiness is visible. Drift
is surfaced. This is the primary working space.

**Prepared crate** — A deliberate, purpose-bound subset of prepared material. May be scoped to an event, a venue, a
genre, a client, or a mood. Has an explicit reason for existing.

**Hot table** — A near-immediate candidate pool. Material close to actual use. May feed a live path. Ephemeral by
default, but snapshottable.

**Live path** — The actual or planned performance sequence. What is playing and what is next. May be improvised or
pre-ordered. Has real-time state.

**Shadow paths** — Plausible alternatives, recovery routes, and transition branches maintained alongside the live path.
Not hypotheticals — actionable options the DJ has chosen to keep available.

### 9.2 Rules

A hot table is ephemeral by default. It may be promoted to a snapshot.

A prepared crate is durable. It persists across sessions.

A smartlist may feed a prepared crate, but the crate's membership is its own — not automatically replaced by query
changes.

A shadow path may become a live path at any moment. The system must support promotion without friction.

A live path is not a playlist. It has runtime state, history, and evidence.

A request queue is not a live path. It is incoming intent. It may influence a live path but does not own it.

### 9.3 Organization taxonomy

Dekzer must not collapse all organizational objects into one type with a kind field. That is how the substrate loses
meaning.

```text
manual collection       user-authored membership, explicitly maintained
query collection        rule-authored membership, dynamically resolved
materialized snapshot   frozen result of a query or manual selection at a point in time
ordered sequence        deliberate play order over a collection
event queue             incoming request or intent, not user-authored
performance path        actual or planned runtime play route with live state
export projection       target-specific representation of a collection or sequence
```

Each of these is a different thing. Each has different durability, mutability, authority, and semantic weight. The
schema must reflect that.

---

## 10. Doctrine

### I. Inspectable is a hard requirement

A normal DJ database stores outcomes:

```text
cue_point = 00:32.14
bpm       = 128.4
key       = 8A
```

Dekzer stores claims with provenance:

```text
Who or what produced this?
When?
Against which media version?
With what confidence?
Was it accepted by the user?
Was it edited?
Was it invalidated?
Does it still apply to the current attachment?
Can it survive the selected export target?
```

Many important facts in Dekzer are not just values. They are claims.

A cue point may be: an engine-detected candidate, a user-created marker, an imported marker from another system, an
edited marker, an accepted marker, a stale marker, a rejected marker, an exported marker, or a marker that applies only
to a previous media version.

Collapsing those into one flat `cue_points` table loses the product.

The conceptual shift:

```text
Not only: tracks, cue points, beatgrids, tags.
Also: observations, claims, approvals, edits, invalidations, readiness projections.
```

### II. Library decay is a first-class enemy

A track is not one eternal object.

It can have versions, attachments, masters, edits, exports, remasters, streaming substitutions, corrupted copies,
transcoded copies, moved files, duration drift, and analysis baselines that no longer apply.

Dekzer must distinguish: musical work identity, track identity inside the user's collection, media attachment, file
fingerprint, audio fingerprint, analysis baseline, preparation state, export state, and target compatibility state.

The failure case:

```text
Cue points survive, but the file changed by 8 ms.
```

Current tools treat this as fine. Dekzer must treat it as a possible readiness downgrade.

The model must leave room for drift detection across: duration, audio fingerprint, file hash, container metadata, sample
rate, channel layout, loudness, waveform alignment, analysis baseline, and streaming source substitution.

Not all of this belongs in the MVP. All of it belongs in the runway.

### III. Trust asymmetry sets the engineering bar

A visual bug is a bug.

A silent readiness lie is betrayal.

Trust is slow to build and fast to destroy. One silent export error can erase months of confidence in the tool. The
substrate layer carries a higher correctness obligation than the performance surface.

Dekzer must express uncertainty without shame. Most software hides uncertainty. Dekzer exposes uncertainty early, while
it is still fixable.

### IV. Local-first and externally aware are both required

Local-first means: the library is the user's, preparation is the user's, the app works without cloud dependency, and
durable state does not disappear because a service changes terms.

Externally aware means: Dekzer models CDJ firmware behavior, export format constraints, streaming restrictions,
controller capabilities, OS audio behavior, driver caveats, known broken versions, and target-device media limitations.

These external facts cannot be hard-coded as eternal truths. They rot. The resolution:

```text
Local substrate owns the user's library and prep.
External compatibility knowledge arrives as versioned, updateable catalogs.
Compatibility results record which catalog version produced them.
```

Hard distinction:

> Dekzer must not require the cloud to know the user's own library.
> Dekzer may use updateable external catalogs to evaluate target compatibility.

### V. Import is repatriation, not middleware

Importing from Rekordbox XML, Serato crates, Traktor NML, VirtualDJ, Engine DJ, or other ecosystems is repatriation:

> Bring your trapped work home.

A Rekordbox cue is not instantly Dekzer authority. It is an imported claim that can become accepted preparation after
mapping, verification, and conflict resolution. Import sessions are provenance events, not magical conversions.

### VI. The deck must never be the first safety check

By the time a track reaches the hot table or live path, Dekzer should already know: source state, file readability,
format support, analysis presence, grid confidence, cue readiness, stem availability, sleeve state, metadata provenance,
streaming limitations, target-device compatibility, export status, and known degraded conditions.

Current tools reveal failure at the worst possible moment. Dekzer drags failure into preparation time, where it can be
understood, fixed, accepted, or rejected.

### VII. Runtime surfaces are projections, not authorities

The performance surface may show readiness. It may consume readiness. It may explain readiness.

It must not invent readiness.

The renderer is not the owner of library state, preparation state, compatibility state, or runtime evidence. It presents
bounded projections from authoritative local state and live runtime publications.

---

## 11. Compatibility Catalog Governance

The doctrine requires versioned, updateable compatibility catalogs. That raises a governance question the doctrine must
answer explicitly.

**Who publishes catalogs?** Catalogs are published by Dekzer. They are not crowd-sourced, community-contributed, or
automatically generated from third-party sources without curation. This maintains accountability.

**Can users pin catalog versions?** Yes. A DJ must be able to freeze the compatibility knowledge set used for a specific
gig. A catalog update that arrives the night before a performance cannot be allowed to silently downgrade a prepared
crate.

**Can users run offline with stale catalogs?** Yes. The system must operate without external network access. Offline use
with a pinned or cached catalog version is a supported mode, not a degraded one.

**How are known-bad catalog entries corrected?** Catalog corrections arrive as versioned updates. When a correction
affects a previously evaluated verdict, the system flags affected tracks and readiness projections as stale — it does
not silently re-evaluate and reassign status.

**Are catalog updates automatic?** Updates are not applied automatically during performance mode. Outside performance
mode, the user controls when catalog updates are applied and can preview what would change.

**Can a gig freeze a compatibility knowledge set?** Yes. This is the Gig Pin. When a DJ locks a gig configuration, the
catalog version in use at that moment is recorded alongside it. Verdicts from that evaluation remain stable for the gig
regardless of subsequent catalog updates.

The hard rule:

> Compatibility catalogs are updateable, but compatibility state used for a gig must be pin-able, inspectable, and not
> subject to silent mutation.

If external knowledge can silently update and downgrade tonight's crate, Dekzer becomes the thing it warned against.

---

## 12. Performance Mode and Stable Mode

Given the "Will it work tonight?" flag, Dekzer must explicitly define what the system will and will not do during
performance.

**Performance mode is a substrate commitment, not a UI mode.**

When performance mode is active:

```text
No automatic compatibility catalog updates are applied.
No destructive metadata migration runs in the background.
No analysis rewrite is triggered by file change detection.
No background job may downgrade live-path readiness without surfacing the condition.
No import session modifies accepted native facts.
No schema migration runs that could affect in-flight state.
```

The live path, hot table, and shadow paths are protected from silent mutation.

The system may continue passive observations (noting that a file changed, noting that a catalog update is available). It
must not act on those observations until the DJ exits performance mode.

DJs distrust automatic change for good reason. Every major DJ software failure story involves something the system did
that the DJ did not ask for. Dekzer names this and refuses it.

---

## 13. Backup, Rollback, and Recovery Doctrine

A preparation ledger without recovery is only half a ledger.

Reversibility is a core property — not a nice-to-have. Every operation that modifies accepted preparation state must
have a defined recovery story.

**Operations that require recovery coverage:**

```text
import session          must be rollbackable as a unit
metadata batch edit     must produce an undo record before applying
scan or rescan          must record previous state for comparison, not replace it silently
export snapshot         must be preserved as a named artifact
preflight report        must be snapshotted at evaluation time
library database        must support durable backup with integrity verification
device export           must be restorable from export snapshot
catalog version pin     must be restorable independently of library state
analysis rewrite        must be staged, not applied without approval
```

**Laws:**

Destructive operations require explicit user confirmation.

Background operations that could affect accepted preparation state must be staged and surfaced before commit.

Every backup must carry a manifest of what it contains, when it was created, and what catalog and schema version it was
made against.

The system must be able to restore a known-good state without requiring the user to reconstruct context from memory.

A DJ who loses a night of prep work because a background job ran silently will not trust Dekzer again. Recovery is not a
feature. It is a trust requirement.

---

## 14. Privacy and Local Sovereignty

The DJ's library is sensitive professional material. It contains unreleased promos, private edits, setlists, gig
history, client or event requests, play history, purchase sources, folder structure, and organizational logic that
reflects real professional relationships and work.

**Dekzer's sovereignty commitments:**

```text
Dekzer does not require remote indexing of the user's library to function.
Dekzer does not send library contents, file paths, preparation state, or play history to external services without explicit user consent.
Any cloud sync, catalog lookup, or external analysis must be opt-in, inspectable, and separable from local authority.
The local substrate remains sovereign even when external services are in use.
Revoking cloud connectivity must not degrade the user's local library state.
```

**The local-first guarantee is not a marketing claim. It is an architectural constraint.**

External features — catalog updates, streaming integration, cloud backup, collaborative features — are additive. They
extend local authority. They do not replace it or hold it hostage.

---

## 15. Job System Doctrine

Preparation implies substantial background work: scanning, fingerprinting, audio analysis, import processing, drift
detection, stem generation, compatibility evaluation, export operations, backup, and preflight generation.

Jobs are not implementation detail. They are substrate actors. Their behavior must be governed.

**Jobs produce evidence, claims, artifacts, or projections. They do not silently overwrite accepted user work.**

**Jobs are resumable.** An interrupted scan or analysis does not start over from scratch. Progress is preserved.

**Jobs are observable.** The user can see what is running, what it is doing, what it has found, and what it has changed.

**Jobs are cancelable.** The user can stop a job. A canceled job leaves the substrate in a valid partial state, not in
mystery.

**Jobs are attributable.** Every piece of evidence or claim produced by a job records which job produced it, at what
time, against which substrate version.

**Failed jobs leave diagnostic evidence, not mystery state.** A job that encounters a problem records what it found,
what it attempted, and where it stopped. It does not silently produce partial results that look complete.

**Jobs respect performance mode.** No job that could mutate accepted preparation state runs during performance mode
without explicit promotion.

**Job authority is bounded.** A job that detects drift does not resolve it. It produces a drift observation. Resolution
requires either user action or an explicitly configured policy that the user has approved.

---

## 16. RT Flight Deck

The doctrine so far addresses preparation — the work done before performance. The Prepared Room model answers: is the
music ready?

RT Flight Deck is the live-runtime sibling of the Prepared Room.

It answers a different question: **What happened during performance, why, and what evidence proves it?**

RT Flight Deck is not the deck surface. It is the runtime evidence layer — the system that observes, records, and makes
sense of what occurs when preparation meets live execution.

```text
Prepared Room        Was the music ready before the first track loaded?
RT Flight Deck       What happened after it did?
```

RT Flight Deck does not invent performance data. It captures it from the live path, runtime events, and performance
surface activity, and relates that evidence back to the preparation state.

A DJ should be able to ask, after a gig:

```text
What played?
What was skipped?
What was prepared but unused?
What loaded cleanly versus under pressure?
What did the system flag as suspect that turned out fine?
What was flagged as ready that failed?
What shadow path became a live path?
Which transitions came from prepared crates, and which were improvised?
```

That is the audit trail a professional preparation ledger should produce.

RT Flight Deck is not a MVP concern. It is a doctrine concern. Its existence must be accounted for in the substrate
design now — even if its UI surfaces come later. Runtime evidence must not be an afterthought bolted onto a system not
built to carry it.

---

## 17. Modeling Laws

Do not store important preparation values without provenance.

Do not treat files as tracks.

Do not treat track identity as file identity.

Do not treat imported metadata as native authority.

Do not treat compatibility as a boolean.

Do not treat ready as a static property.

Do not store readiness as belief. Project it from evidence.

Do not let streaming sources masquerade as local attachments.

Do not hide drift.

Do not flatten candidates, accepted facts, rejected facts, stale facts, and exported facts into the same semantic
bucket.

Do not make the deck responsible for discovering substrate failure.

Do not require the cloud to understand the user's own library.

Do not hard-code external compatibility knowledge as timeless product truth.

Do not use one organization object for crates, playlists, smartlists, snapshots, request queues, and live paths.

Do not let a UI surface own durable meaning that belongs in the substrate.

Do not allow jobs to silently overwrite accepted user work.

Do not apply catalog updates during performance mode without explicit user action.

Do not run destructive operations without a defined recovery path.

Do not confuse a claim at one identity rung with a claim at a different rung.

---

## 18. Conceptual Substrate Families

These are not final table names. They define territory the data model must cover.

```text
source locations
literal source hierarchy
media attachments
media fingerprints
audio fingerprints
recording / track identities
identity rung mappings
claim records
claim lifecycle transitions
collection organization objects
  manual collections
  query collections
  materialized snapshots
  ordered sequences
  event queues
  performance paths
  export projections
preparation artifacts
preparation claims / candidates
user approvals / edits
conflict records
invalidations
readiness projections
readiness evidence records
drift detections
import sessions
import conflict resolutions
job records
job evidence artifacts
compatibility catalogs
compatibility catalog versions
target profiles
preflight evaluations
export snapshots
gig pins
runtime evidence records
backup manifests
```

Not all must be visible in the first UI.

All must shape the schema.

---

## 19. Questions Dekzer Must Eventually Answer

A DJ should be able to ask:

```text
Why do you think this track is ready?
What changed since I last trusted it?
Where did this cue come from?
Which media version was this grid made against?
Will this prep survive this export target?
Which facts were imported?
Which facts did I approve?
Which facts are still suspect?
What will break if I use this source tonight?
What is safe locally but unsafe on this device?
What did the system infer, and what did I explicitly confirm?
What did I play last time, and what did I skip?
What was prepared but not used?
Which catalog version was used for this compatibility verdict?
Can I go back to the state my library was in before this import?
```

If Dekzer cannot answer these questions, it is not the product it claims to be.

---

## 20. First Slice and MVP Boundaries

The first serious Dekzer slice should not begin with decks.

It should prove the substrate path:

```text
register a local root
scan literal hierarchy
persist source locations and hierarchy
inventory media-relevant attachments
classify readable media
preserve durable file identity
surface source hierarchy in the app
reopen cleanly
show what is known, unknown, and unsupported
```

Early UI may look simple. That is acceptable.

The important milestone is not visual drama. It is the first moment Dekzer can say:

```text
I know where your music is.
I know what I found.
I know what I can read.
I know what I cannot yet trust.
I can reopen tomorrow and remember correctly.
```

That is the floor.

**What is explicitly not in the first slice:**

```text
No deck runtime.
No controller support.
No streaming-first architecture.
No AI claim authority.
No full compatibility matrix.
No event operations.
No performance mode enforcement (until there is a performance to protect).
No RT Flight Deck runtime layer.
No cloud sync.
```

These are not abandoned. They are sequenced. The substrate must be provably sound before the surfaces above it carry
weight.

The dragon stays in the cave until the foundation can hold its bones.

---

## 21. What Dekzer Is Not

Dekzer is not a skin over decks.

Dekzer is not a streaming-first DJ toy.

Dekzer is not middleware for other DJ apps.

Dekzer is not a cloud account wrapped around a music folder.

Dekzer is not a generic file browser.

Dekzer is not a compatibility spreadsheet.

Dekzer is not a database that happens to draw waveforms.

Dekzer is not a system that hides uncertainty to look polished.

Dekzer is not a product that discovers failure at the moment it is most expensive.

---

## 22. What Dekzer Is

Dekzer is a professional memory system for music work.

Not a DJ app that also has a library.

A library and preparation substrate that also has a performance surface.

The category:

```text
local-first, inspectable preparation ledger for live performance
```

The plain promise:

```text
Dekzer helps DJs know what they have, what is ready, what changed, and what will work tonight.
```

The flag:

```text
Will it work tonight?
```

Everything else serves that, or it is decorative fog.
