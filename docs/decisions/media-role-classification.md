# Media Role Classification

_Canonical architecture note. Governs library substrate, classification pipeline, browser projection, and all
file-to-product-object mapping in Dekzer._

---

Dekzer does not browse files directly.

Dekzer discovers source files, records media facts, assigns product roles, computes target-specific readiness, and
projects context-specific browser rows.

```
A file is inventory.
A library item is a product-addressable object derived from inventory.
A role is a product meaning assigned to a library item.
Readiness is target-specific usability.
Visibility is a projection.
```

---

## Companion Documents

This document governs what media means: inventory, evidence, library items, roles, readiness, and projection.

`source-access-and-scan-coverage.md` governs source access, platform permission outcomes, directory enumeration
outcomes, blocked/missing coverage, and the rule that access failure never becomes empty contents.

`work-scheduling.md` governs scheduling, priority, resource budgets, checkpointing, cancellation, leases, foreground
interest, and large-file fairness.

Do not duplicate those concerns here. Reference the companion documents when implementation work crosses those
boundaries.

---

## Core Laws

### 1. Files are not product objects

A source file is a discovered filesystem object. It carries path observation facts, extension claims, availability
state, scan lineage, and size/modified facts. It holds an optional link to a resolved file identity — it does not own
the identity itself.

A library item is created when Dekzer has a product-relevant reason to represent a media object. Library items are
either source-backed (bound to a `source_files` row via `item_origins`) or derived (from an embedded stream, a generated
asset, or a user-created object). Not every source file becomes a library item. Not every library item has a direct
source file — embedded artwork extracted from an audio container, for example, gets its own library item whose origin is
`embedded_stream`, not a fake `source_files` row.

### 2. Classification assigns roles. It does not define identity.

Path and identity are distinct. `source_files` is a mutable path-level scan observation — updated in place when the
scanner revisits a path. `file_identities` is a stable content identity anchored to a content hash. A library item must
not silently acquire the identity of a different file merely because that file appears at a known path.

Identity rules:

```
Rename or move of the same content → may reuse the existing file_identities row
Replacement of content at the same path → must produce a new file_identities row;
                                           prior identity-bound assignments do not transfer
```

A library item may hold multiple roles simultaneously.

Valid roles:

| Role                 | Meaning                                                                                                                                                                                                                                          |
|----------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `performance_item`   | Candidate for deck or transport use. Actual loadability is target-specific readiness.                                                                                                                                                            |
| `visual_asset`       | Candidate for visual output or visual workflows. Actual use is target-specific readiness.                                                                                                                                                        |
| `artwork_candidate`  | Candidate for attachment as artwork to a track, album, folder, or crate.                                                                                                                                                                         |
| `companion_metadata` | A recognized sidecar or structured companion to another item — CUE sheets, structured playlists, disc image companions. Not every adjacent text file. Random `.log` files, screenshots, and unrecognized adjacents are not `companion_metadata`. |

Do not add `primary_role`. A music video is simultaneously a `performance_item` and a `visual_asset`. A cover image is
an `artwork_candidate`. A role conflict is a projection problem, not a reason to introduce a false hierarchy.

Roles name what an item appears to be. They do not grant permission to use it for a target. Readiness gates use.

### 3. Diagnostic visibility is not a role

`diagnostic` is a browser surface, not a product role. Do not model it as a library item role.

The diagnostic surface may show source files with no accepted role, failed probes, rejected candidates, unsupported
formats, permission failures, and items hidden from normal browse projections. These objects need no special role to be
visible there — absence of an accepted role is sufficient criterion.

### 4. Role membership is unique per library item

For a given library item, a role is either held or not held. State transitions happen on the single existing row for
that `(library_item_id, role)` pair.

Schema invariant:

```sql
UNIQUE (library_item_id, role)
```

Do not insert duplicate role rows to represent disagreement between classifier and user. The row carries the current
assignment state and origin. Disagreement is a state, not a second row.

### 5. User assignments are sticky, scoped to file identity

User-originated role decisions survive scans, probes, source refreshes, and re-discovery — **as long as the underlying
file identity is unchanged.**

File identity is determined by content hash, not path alone. If a file at a known path changes its identity (hash
differs from the one associated with the original assignment), the user assignment from the previous identity does not
transfer automatically. The classifier treats the new file as a new object and creates fresh provisional claims. The
prior user assignment is archived, not deleted, so Dekzer can surface it if needed.

State preservation rules:

```
user accepted    → stays accepted on re-scan, same file identity
user rejected    → stays rejected on re-scan, same file identity
user excluded    → stays excluded on re-scan, same file identity

classifier provisional → may be refreshed, replaced, or removed on re-scan
```

"user promoted" is not a state. It is an action. The result of user promotion is
`assignment_state = accepted, assignment_origin = user`. See Law 7 schema for role assignment states.

The classifier may add new provisional claims when new evidence appears. It may not overwrite, downgrade, or remove a
user-originated assignment on the same file identity.

**Provisional identity and the hash dependency.** Content hash is the strongest file identity signal but is not always
available at discovery time. The scanner does not hash all files synchronously. Hash computation is deferred and
scheduled by policy; on large video files over slow external media it must not block user interaction.

The substrate handles this in two phases:

```
provisional identity   source_id + path + size + modified_at
                       sufficient for classifier-originated assignments
                       not sufficient for durable user-originated assignments

resolved identity      content_hash
                       required before a user-originated assignment is considered sticky
```

When a user makes a durable role decision (accept, reject, exclude) before content hash is available, the substrate must
schedule hash computation and bind the assignment to `file_identity_id` when it resolves. Until that bind occurs, the
assignment carries a null `file_identity_id` and is non-sticky (provisional-path-bound). If identity resolution reveals
the file at that path is a different object than expected, the assignment must be surfaced for review rather than
silently transferred.

User assignments carry a `file_identity_id` foreign key. If that key is null, the assignment is not yet sticky. A
user-originated assignment is sticky for the library item and origin it was made against, bound to the resolved file
identity. File identity alone does not make the assignment global across every source occurrence of the same bytes — an
exclusion of one copy does not silently exclude identical content found elsewhere.

### 6. Extension affects priority. It does not determine final recognition and does not permanently exclude a file from future sniffing or probing.

The file extension is a cheap claim made by the filename. It is the first available signal and the weakest one.

Extension claims are used to:

- create low-confidence provisional role assignments immediately on discovery
- **prioritize** sniff and probe scheduling (known media extensions are scheduled sooner)

Extension claims are never used to:

- permanently exclude a file from media recognition
- make final classification decisions
- determine deck load eligibility
- allow renderer code to infer product meaning

**All source files are eligible for future media recognition. Full media probing is scheduled only when media-relevance
evidence exists.** An unknown or absent extension is not evidence of non-media status. It is absence of evidence, which
is different. A file with no known extension may be a renamed audio file, a misextensioned container, or a non-media
file — the distinction requires a sniff or probe, not an extension check.

### 7. Media probe facts are separate from extension claims

Extension claims and probe results are distinct facts with different reliability, availability timing, and eviction
policy. They must be tracked separately.

```
extension_claim    cheap, immediate, unreliable
probe result       expensive, deferred, authoritative when present
```

A `.mp4` may contain only an audio stream.
A `.mov` may contain multiple audio streams and no video.
A `.wav` may have corrupt or missing metadata.
A renamed file may lie.
A broken container may have a plausible extension and no usable stream.

The classifier uses probe facts when available. It falls back to extension claims only as a bounded provisional signal.
It never treats extension as final truth.

Three tiers of recognition facts exist and are tracked in separate structures: `extension_claim` on `source_files` (
filename-derived), `signature_sniff_results` rows (cheap bounded file-header sniff with structured kind and family
fields), and `media_probe_results`/`media_streams` rows (expensive authoritative container parse). Stream-level facts
are bound to a specific `probe_result_id`, not to the source file directly — this ensures superseded stream rows cannot
survive a re-probe that contradicts them. See **Substrate Schema** below.

### 8. Readiness is separate from role

Classification says what an item appears to be. Readiness says whether Dekzer can use that item for a specific target.

A file can hold a role and still be blocked for a specific target. These are independent facts.

Readiness targets:

```
audio_deck_load
video_deck_load
visual_output
artwork_attachment
waveform_preview
broadcast_metadata_projection
```

Analysis work kinds such as `beatgrid_analysis` are capabilities or work
products, not deck-load readiness targets. They may appear as scheduler
work kinds or as derived artifacts, but they do not answer whether an item
is usable for a target now.

Protocol/API casing may expose camelCase representations such as
`audioDeckLoad`, but those are representations of the canonical target
key, not separate targets.

Readiness states:

```
ready         usable for this target
degraded      usable with known limitations; permitted only when target
              policy explicitly allows degraded runtime use
pending       required evidence or work is not yet complete
blocked       not usable; reason is known
unavailable   required source or resource is not accessible
unsupported   target type not applicable to this item
failed        readiness evaluation or required evidence evaluation failed
              unexpectedly (distinct from blocked/unavailable)
```

Readiness reason codes:

```
probe_pending
codec_unsupported
no_audio_stream
no_video_stream
corrupt_file
missing_file
permission_denied
user_excluded
```

Examples:

```
music-video.mp4
  performance_item              role
  visual_asset                  role
  audio_deck_load: ready        readiness (has usable audio stream)
  video_deck_load: ready        readiness (has usable video stream)

silent-loop.mp4
  visual_asset                  role
  audio_deck_load: blocked      readiness (no_audio_stream)
  video_deck_load: ready        readiness

cover.jpg
  artwork_candidate             role
  audio_deck_load: unsupported  readiness (not applicable)
  artwork_attachment: ready     readiness
```

### 9. Readiness is invalidated by substrate events, not queried by the renderer

Readiness is not recomputed on every browse query. Readiness is not fetched lazily by the deck at load time.

**Invalidation** means: the current readiness row for that `(library_item_id, target)` pair has its status set to
`pending`. It does not mean evaluation runs immediately. Evaluation is scheduled and runs asynchronously.

Readiness is invalidated when:

- source file availability changes (present → missing or missing → present)
- a file's identity changes (hash, size, or modification time differ after re-scan)
- a new probe result is written for the source file
- a role assignment changes in a way that affects eligibility
- a user excludes or re-accepts an item
- supported codec or policy rules change

The deck must not discover basic format failure at load time. The substrate is responsible for knowing this earlier.

### 10. Visibility is never stored

Visibility is derived at query time from role membership, assignment state, readiness, source context, and browser
context.

Forbidden columns and flags:

```
library_items.is_visible
library_items.hidden
library_items.browser_visible
source_files.excluded_from_browser
source_files.show_in_browser
```

Correct model:

```
source_files       → inventory facts
role assignments   → product meaning
readiness          → target-specific usability
projections        → what appears in each context
```

Visibility is a query result. It is not durable identity.

---

## Substrate Schema

In this document, `library_items` means source-backed or derived media items only — tracks, visual clips, artwork
assets, companion files. It does not mean crates, playlists, routes, or workspace objects. Those are separate substrate
concerns.

Implementations must not invent columns that contradict these structures. Comments on each table describe the allowed
value sets.

```sql
-- Path-level scan observations. Mutable. Updated in place when the scanner revisits a path.
-- This is NOT a stable identity anchor. Identity lives in file_identities.
-- discovery_state tracks path-observation status only (not availability, sniff state, probe state, or policy exclusion).
-- Availability is tracked in the availability column. Sniff state is in signature_sniff_results.
-- Probe state is in media_probe_results. Policy exclusion is a separate concern.
source_files
(
  source_file_id    INTEGER PRIMARY KEY,
  source_id         INTEGER NOT NULL,
  path              TEXT NOT NULL,
  extension_claim   TEXT,                -- lowercased, from filename
  availability      TEXT NOT NULL,       -- present | missing | inaccessible
  discovery_state   TEXT NOT NULL,       -- discovered | removed_from_scan | stale
  size_bytes        INTEGER,
  modified_at       INTEGER,             -- unix epoch
  file_identity_id  INTEGER REFERENCES file_identities(file_identity_id),  -- null until hash resolved
  UNIQUE (source_id, path)
)

-- Stable content identity. Created when content hash is first computed.
-- Survives path changes, renames, and moves of the same content.
-- Renames and moves of the same content may share one file_identities row.
-- Replacement of content at the same path must produce a new file_identities row;
-- prior identity-bound assignments from the previous row do not carry over.
--
-- file_identities identifies content, not necessarily a single user-facing library item.
-- Multiple source_files rows and multiple library_items may reference the same
-- file_identity_id (identical bytes in different folders, for example).
-- Content identity must not collapse distinct source occurrences into one product
-- item unless a separate canonical-track merge policy explicitly says to do so.
-- Source browsing is hierarchy-sensitive: a file must not vanish from one folder
-- because its hash already appears under another.
file_identities (
  file_identity_id INTEGER PRIMARY KEY,
  content_hash    BLOB NOT NULL UNIQUE,
  size_bytes      INTEGER NOT NULL,
  first_seen_at   INTEGER NOT NULL
)

-- Product-addressable media objects.
-- Bound to file_identity when hash is resolved; provisional otherwise.
-- Origins tracked in item_origins. No source_file_id here.
library_items (
  library_item_id           INTEGER PRIMARY KEY,
  file_identity_id          INTEGER REFERENCES file_identities(file_identity_id),  -- null when provisional
  provisional_identity_key  TEXT,   -- source_id:path:size:modified_at composite; used before hash resolves
  identity_state            TEXT NOT NULL  -- provisional | identity_resolved
  -- No primary_role. No is_visible. No hidden.
)

-- Origin of a library item. Supports source files, embedded streams, and derived assets.
-- Embedded artwork extracted from an audio file gets its own library_item
-- with origin_kind = embedded_stream, not a fake source_files row.
item_origins (
  item_origin_id    INTEGER PRIMARY KEY,
  library_item_id   INTEGER NOT NULL REFERENCES library_items(library_item_id),
  origin_kind       TEXT NOT NULL,     -- source_file | embedded_stream | derived_artifact | user_created
  source_file_id    INTEGER REFERENCES source_files(source_file_id),         -- present for source_file origins
  probe_result_id   INTEGER REFERENCES media_probe_results(media_probe_result_id),  -- present for embedded_stream origins
  stream_index      INTEGER                                       -- present for embedded_stream origins
)

-- Many-to-many role membership. One row per (item, role). Current state only.
-- History is in library_item_role_events.
library_item_roles (
  library_item_id   INTEGER NOT NULL REFERENCES library_items(library_item_id),
  role              TEXT NOT NULL,          -- performance_item | visual_asset | artwork_candidate | companion_metadata
  assignment_state  TEXT NOT NULL,          -- proposed | accepted | rejected | excluded
  assignment_origin TEXT NOT NULL,          -- classifier | probe | filename_rule | embedded_metadata | user
  basis             TEXT NOT NULL,          -- extension_claim | signature_claim | media_probe | sidecar_rule | embedded_artwork | user_override
  confidence        TEXT NOT NULL,          -- low | medium | high
  file_identity_id  INTEGER REFERENCES file_identities(file_identity_id),
    -- Required when assignment_origin = user. Null means assignment is not yet sticky.
    -- Classifier-originated assignments may carry null until identity resolves.
  assigned_at       INTEGER NOT NULL,
  UNIQUE (library_item_id, role)
)

-- Append-only history of all role state changes. "Archived, not deleted" is enforced here.
library_item_role_events (
  library_item_role_event_id INTEGER PRIMARY KEY,
  library_item_id   INTEGER NOT NULL REFERENCES library_items(library_item_id),
  role              TEXT NOT NULL,
  event_kind        TEXT NOT NULL,     -- proposed | accepted | rejected | excluded | archived | superseded
  assignment_origin TEXT NOT NULL,
  basis             TEXT NOT NULL,
  confidence        TEXT NOT NULL,
  file_identity_id  INTEGER REFERENCES file_identities(file_identity_id),
  occurred_at       INTEGER NOT NULL
)

-- Signature sniff results. Cheap bounded header inspection. Separate from extension and probe.
-- One current row per file; re-sniff replaces the row.
-- Sniff history is not retained here. Role event history does not carry sniff diagnostic detail.
-- Sniff diagnostics must be read from the current signature_sniff_results row and its issue_code.
signature_sniff_results (
  signature_sniff_result_id INTEGER PRIMARY KEY,
  source_file_id    INTEGER NOT NULL REFERENCES source_files(source_file_id),
  sniff_state       TEXT NOT NULL,     -- complete | failed | skipped
  signature_kind    TEXT,              -- id3 | flac | riff_wave | mp4_ftyp | jpeg | png | gif | zip | unknown
  signature_family  TEXT,              -- audio | video_container | image | archive | unknown
  issue_code        TEXT,
  sniffer_version   TEXT NOT NULL,
  sniffed_at        INTEGER NOT NULL
)

-- Container-level media facts. One row per probe pass.
-- superseded_at marks historical rows when a re-probe produces a newer result.
media_probe_results (
  media_probe_result_id INTEGER PRIMARY KEY,
  source_file_id        INTEGER NOT NULL REFERENCES source_files(source_file_id),
  probe_state           TEXT NOT NULL,     -- complete | failed | partial
  container_kind        TEXT,
  duration_ms           INTEGER,
  issue_code            TEXT,
  probe_tool            TEXT NOT NULL,
  probe_tool_version    TEXT NOT NULL,
  probe_policy_version  TEXT NOT NULL,
  probed_at             INTEGER NOT NULL,
  superseded_at         INTEGER            -- null if current; set when a newer probe replaces this result
)

-- Stream-level media facts. Bound to a specific probe result, not the source file.
-- Old stream rows cannot survive a re-probe: they belong to their probe_result_id.
media_streams (
  media_stream_id   INTEGER PRIMARY KEY,
  probe_result_id   INTEGER NOT NULL REFERENCES media_probe_results(media_probe_result_id),
  stream_index      INTEGER NOT NULL,
  stream_kind       TEXT NOT NULL,     -- audio | video | image | subtitle | data
  codec             TEXT,
  duration_ms       INTEGER,
  width             INTEGER,
  height            INTEGER,
  frame_rate_num    INTEGER,
  frame_rate_den    INTEGER,
  sample_rate       INTEGER,
  channel_count     INTEGER,
  disposition_flags TEXT               -- default | attached_pic | forced | …
)

-- Target-specific usability. One row per (item, target).
-- pending status means invalidated_at is set but last_evaluated_at reflects the prior result.
-- Do not fabricate evaluated_at timestamps for rows that have never been evaluated.
item_readiness (
  library_item_id   INTEGER NOT NULL REFERENCES library_items(library_item_id),
  target            TEXT NOT NULL,        -- audio_deck_load | video_deck_load | visual_output | artwork_attachment | waveform_preview | broadcast_metadata_projection
  status            TEXT NOT NULL,        -- ready | degraded | pending | blocked | unavailable | unsupported | failed
  reason            TEXT,                 -- readiness reason code; populated for blocked, degraded, unavailable, or failed statuses
  invalidated_at    INTEGER,              -- when last invalidated; null if never invalidated
  last_evaluated_at INTEGER,              -- when last evaluation completed; null if never evaluated
  evaluator_version TEXT,
  policy_version    TEXT,
  UNIQUE (library_item_id, target)
)
```

---

## Artwork Candidate Rules

An image becomes an `artwork_candidate` only when at least one of the following is true:

1. It is embedded artwork extracted from an audio or video file.
2. It lives in the same directory as at least one `performance_item` candidate.
3. Its filename matches a recognized convention: `cover`, `folder`, `front`, `back`, `artwork`, `albumart`.
4. Its basename or structured sidecar name matches a track, album, artist, or folder identity in the library.
5. The user manually promotes it as artwork.

**An image is not an artwork candidate merely because it shares an ancestor directory with music.** Proximity is
same-directory, not same-source.

Forbidden example:

```
/Music
  /Albums
    /Artist
      /Album
        track.flac
  /Artwork Archive
    random-event-poster.png
```

The image in `Artwork Archive` does not become an `artwork_candidate` for any track, album, or folder in the library. It
shares a source, not a directory, and the rule does not extend transitively through source ancestry.

Artwork candidates carry target scope:

```
track_artwork_candidate
album_folder_artwork_candidate
source_artwork_candidate
crate_sleeve_artwork_candidate
```

A `folder.jpg` beside an album is a candidate for that folder only. It does not silently attach to sibling directories,
parent folders, or other albums in the same source.

---

## Classification Pipeline

The pipeline has three stages. Each stage is cheaper than the next. A file advances to the next stage only when evidence
justifies it.

### Stage 1 — Discovery

```
1.  Scanner discovers regular filesystem files in registered sources.
2.  source_files row is inserted immediately. No extension-based filtering.
3.  Records path, extension_claim, availability, size, modified_at,
    and identity facts (hash when available).
4.  Extension claims produce low-confidence provisional role assignments
    for files whose extension_claim matches known media types
    (audio, video, image, sidecar).
    Assignment state: proposed. Confidence: low. Basis: extension_claim.
5.  Files with known media extensions are queued for Stage 2 or Stage 3
    at elevated priority.
    Files with unknown or absent extensions are eligible for Stage 2.
    No file is permanently excluded from future media recognition.
```

Discovery does not classify final product meaning. It records facts and creates provisional claims where extension
evidence exists.

### Stage 2 — Signature Sniff

A cheap, bounded inspection of file headers and magic bytes. Not a full container parse.

```
6.  Triggered for files that lack a known media extension or where
    extension-based classification is uncertain.
    May also be triggered by policy, source folder rules, or prior
    library identity indicating the file may be media.
7.  Reads only a small bounded prefix or metadata-safe amount.
    Does not open, decode, or demux the container.
8.  Writes a signature_sniff_results row:
      signature_kind:   id3 | flac | riff_wave | mp4_ftyp | jpeg | png | gif | zip | unknown
      signature_family: audio | video_container | image | archive | unknown
      sniff_state:      complete | failed | skipped
9.  A signature_family of audio, video_container, or image
    promotes the file into the Stage 3 probe queue.
    A signature_family of archive or a sniff_state of failed does not
    permanently exclude the file. It lowers probe priority.
    The file remains eligible if other evidence appears in a subsequent pass.
```

Signature sniff prevents extension from being the sole media discovery gate. A renamed `.flac` with no extension, or a
`.dat` wrapping audio, can be recognized without a full probe.

### Stage 3 — Media Probe

An expensive container and stream inspection. Scheduled only when media-relevance evidence exists.

```
10. Media-relevance evidence that justifies a full probe:
      - known media extension (audio, video, image, sidecar)
      - recognized file signature / magic bytes from Stage 2
      - same-directory artwork candidate rule
      - prior library identity (file seen before as media)
      - user action (explicit probe request)
      - source or folder policy override

11. Probe writes media_probe_results and media_streams rows.

12. The classifier revises role assignments using probe facts.
    Provisional claims are confirmed, upgraded, degraded, or retracted.
    User-originated assignments are not touched.

13. Readiness is evaluated for each relevant (library_item, target) pair.
    Each pair receives or updates a row in item_readiness.

14. Browser projections derive visible rows from roles, assignment states,
    readiness, source context, and browser context.
```

Not every discovered file receives a full media probe. A DJ drive containing logs, PDFs, screenshots, installers,
databases, and cache files does not cause a full probe pass on non-media content.

**Retraction handling:** If probe results retract all role claims for a library item, the library item is not deleted.
It transitions to a state with no accepted roles. It remains visible in diagnostic surfaces. It does not appear in
normal browse projections. This state is permanent until new evidence or user action changes it.

---

## Default Browser Policy

> A source row is visible when it is useful for DJ browsing, not merely because the filesystem contains it.

**Normal DJ browse context shows:**

- Folders that have audio or video-track descendants with an accepted `performance_item` role, **or** pending
  media-candidate descendants not yet confirmed by probe
- Audio performance items (accepted role)
- Pending media candidates (inferred role, probe not yet complete) — visible as pending, never shown as deck-ready
- Video performance items
- Source roots that the user has explicitly registered and not excluded
- Missing or unavailable items when they explain an otherwise confusing library state

Pending rows must carry a visible pending state. They must not be shown as loadable or offer deck controls. Hiding all
provisional items until probe completes produces empty-looking trees during active scan and is incorrect behavior.

**Normal DJ browse context hides by default:**

- Loose image files
- Cover/artwork sidecar files
- Video files with no accepted `performance_item` role
- Unsupported or unrecognized files
- Cache files, export artifacts, log files
- Files with no accepted role (visible in diagnostics only)

**Visual asset context shows:**

- Visual clips
- Accepted visual images
- Overlay assets
- Folders with visual asset descendants
- Visual readiness problems requiring attention

**Source diagnostic context shows:**

- All tracked files needed to explain scanner or classifier behavior
- Unsupported files
- Probe failures
- Rejected role assignments
- Permission issues
- Unavailable or missing files
- Files hidden from normal browse projections and why

---

## UI Timing Rule

The substrate must support visual asset roles before the product exposes a top-level visual asset surface. Substrate
correctness does not depend on UI surface availability.

Do not ship a top-level `VISUAL ASSETS` sidebar section until Dekzer has a real visual output or visual asset workflow
that the user can operate.

Until that surface exists:

- Video tracks appear under Collection / Video (deck-loadable items only, not every `.mp4`)
- Artwork candidates appear in artwork review and item detail surfaces
- Unresolved visual files appear in source diagnostics
- `visual_asset` roles exist in substrate without being promoted into primary navigation

No product navigation should name a surface that the product cannot yet deliver.

---

## Component Ownership Boundaries

Each component owns its outputs. No component may write into another component's domain.

```
Scanner
  Owns:  source_files rows, availability state, discovery_state
  Reads: registered sources, scan policy
  Must not: classify roles, evaluate readiness, determine visibility

Sniffer
  Owns:  signature_sniff_results rows
  Reads: source_files rows
  Must not: write probe results, assign roles, update availability

Probe Worker
  Owns:  media_probe_results rows, media_streams rows
  Reads: source_files rows, probe policy
  Must not: assign roles, evaluate readiness

Classifier
  Owns:  library_item_roles rows (classifier-originated only)
         library_item_role_events rows
         library_items rows (creation of new items)
         item_origins rows
  Reads: source_files, signature_sniff_results, media_probe_results, media_streams
  Must not: write user-originated role assignments, evaluate readiness,
            overwrite or remove user-originated assignments

Readiness Evaluator
  Owns:  item_readiness rows
  Reads: library_item_roles, media_probe_results, media_streams, source_files
  Must not: assign roles, modify probe results, trigger probing

Projection / Query Layer
  Owns:  browser visibility (derived at query time, never stored)
  Reads: all substrate tables
  Must not: infer classification from extension, write any substrate row,
            trigger probe or readiness work as a side effect of a browse query

Renderer
  Owns:  visual presentation of projection results
  Reads: projection output only
  Must not: access substrate tables directly, infer product meaning from
            file extension, path, or filename, trigger any substrate work
```

This boundary is a correctness constraint, not a style preference. Implementation pressure will push classification
logic into the query layer and renderer. That must be treated as a defect, not a shortcut.

---

## Forbidden Implementation Patterns

```
library_items.primary_role
library_items.is_visible
library_items.hidden
library_items.browser_visible
library_items.source_file_id          -- origins belong in item_origins
source_files.excluded_from_browser
source_files.show_in_browser
source_files.signature_claim          -- sniff facts belong in signature_sniff_results
item_readiness.evaluated_at           -- use invalidated_at + last_evaluated_at instead
media_streams.source_file_id          -- streams must bind to probe_result_id

duplicate (library_item_id, role) rows
assignment_state = promoted           -- not a state; result of user action is accepted + origin = user
renderer-side extension classification
deck load eligibility derived from extension alone
probe or readiness work triggered by browse rendering or deck load requests
embedded artwork represented as a fake source_files row
top-level Visual Assets navigation before a real visual workflow exists
```

---

## Acceptance Bar

The implementation is correct only when all of the following hold:

- A music video can simultaneously hold `performance_item` and `visual_asset` roles
- A user-excluded item stays excluded after re-scan when file identity is unchanged
- A user-excluded item is not sticky if the file at that path has changed content hash
- Replacement of content at the same path produces a new `file_identities` row; prior identity-bound assignments are
  archived, not transferred
- A library item does not silently acquire the identity of a different file that appears at the same path
- A user-originated assignment with a null `file_identity_id` is treated as provisional, not sticky
- Content hash computation is scheduled and deferred; it does not block discovery or user interaction; user decisions
  made before hash resolution carry a null `file_identity_id` until the bind completes
- Readiness is set to `pending` (invalidated_at written) when invalidated; evaluation runs asynchronously
- A readiness row that has never been evaluated has a null `last_evaluated_at`, not a fabricated timestamp
- The deck never receives a format failure at load time for a condition the substrate could have evaluated earlier
- Extension-based classification is visibly provisional; no probe-pending item is shown as deck-ready
- No renderer code infers product meaning from file extension or path
- No stored visibility flag exists anywhere in the schema
- Normal DJ browsing does not show loose cover images as track-equivalent rows
- Pending media-candidate descendants make their parent folder visible during active scan
- Pending rows are never presented as deck-ready
- A library item whose probe retracts all role claims remains in the substrate and is visible in diagnostics
- Embedded artwork extracted from an audio file becomes a `library_item` via `item_origins`, not a fake `source_files`
  row
- `media_streams` rows belong to a `probe_result_id`; a re-probe does not leave orphaned stream rows from the prior
  result; prior probe result rows carry a non-null `superseded_at`
- Sniff diagnostics are readable from `signature_sniff_results.issue_code` and `sniff_state`; role event history does
  not carry sniff diagnostic detail
- Prior role decisions are recoverable from `library_item_role_events` even after the current row is updated
- An extensionless media file can be recognized through signature sniff and subsequently promoted to full probe
- A non-media file with no media-relevance signal is not full-probed by default
- Unknown or absent extension never means permanently ignored; it means media-relevance is unconfirmed
- Full media probe scheduling is policy-driven and substrate-owned; it is never triggered by browse rendering or deck
  load
- Diagnostics can explain hidden, rejected, unsupported, missing, probe-failed, and policy-excluded files
