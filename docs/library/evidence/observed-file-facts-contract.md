---
status: accepted
last-reviewed: 2026-06-01
owner: library-substrate-boundary
canonical-context:
  - media-relevant-file-inventory-contract
  - media-identity-schema-authority
  - media-probe-observations-contract
scope:
  - observed-file-facts
  - content-hash-placement
  - source-file-evidence
---

# Observed File Facts Contract

## Purpose

Observed file facts are durable evidence produced by reading or probing a source file. They are attached to a
`source_file_id` and a captured file basis. They are not product identity, track identity, attachment identity, artwork
intelligence, waveform state, stems state, or preparation readiness.

Source files remain attachment inventory. Observed facts describe what was observed about a file row at a point in
time; they do not promote that file into playable media.

## Current Storage

`source_file_facts` is the accepted observed-file-facts table for the current substrate. It stores the accepted
source-file facts for a `source_file_id`. The row includes:

- `source_file_id`
- `basis_fingerprint`
- `basis_source_id`
- `basis_relative_path`
- `basis_size_bytes`
- `basis_mtime_ns`
- `basis_presence_state`
- `observed_at_ms`
- optional `content_hash_algorithm` and `content_hash_value`
- media/probe summary fields such as `media_kind`, `mime_type`, duration, sample rate, channels, bit depth, and codec
- `accepted_artifact_id`

The basis columns are copied from `source_files` when the accepted facts are committed. A read compares the copied
basis with the current `source_files` row and returns `current` only when source id, relative path, size, mtime, and
presence still match. Otherwise the fact is `stale`.

## Content Hash Policy

Content hash evidence belongs here because hashing reads file bytes. Hash values must always be stored with an explicit
algorithm tag.

SHA-256 appears only in tests and fixtures as explicit evidence. It is not declared the product content-hash algorithm.

BLAKE3 source-file hashing is implemented in `library-store-sqlite` as a store-side evidence job. The low-level commit
path streams file bytes through BLAKE3 and commits accepted `source_file_facts` through the existing inspect-source artifact
and observed-facts authority path. The production batch path does not accept renderer- or caller-resolved filesystem
paths. It resolves each `source_file_id` from durable source state before reading bytes. The algorithm string is
`blake3`, and the value is the canonical lowercase hex digest.

BLAKE3 is content evidence, not identity by itself. The hash job does not create attachment identity, primary-media
candidates, track identity rows, preparation rows, waveform artifacts, or CUE-to-audio pairing.

The job captures the current `source_files` basis before reading bytes and re-reads it before committing. If the basis
changed while hashing was in flight, the job rejects the commit with a typed basis-change result instead of recording
evidence that could appear current for the wrong row state. Missing or unreadable filesystem paths fail as typed IO
errors and do not create `source_file_facts`.

## Source-File Path Resolution

Source-file filesystem path resolution is backend-owned by `library-store-sqlite`.

The current root path authority is:

- `source_state.effective_path` when present.
- `source_locators.absolute_path` as the absolute-path fallback, matching the existing root scan and navigation path
  reads.

The resolver input is `source_file_id`. It loads the current `source_files` row, requires
`source_files.presence_state = 'present'`, requires usable source lifecycle state, joins the effective source root with
`source_files.relative_path`, canonicalizes the root and file, and rejects any path that escapes the source root. It
also rejects malformed source-file relative paths containing absolute prefixes, URI schemes, backslashes, empty
segments, or traversal segments.

Missing physical files, blocked files, unavailable/unmounted roots, missing roots, invalid relative paths, and root
escape attempts are typed failures. They do not erase source-file inventory and do not write `source_file_facts`.

`source_locations` can represent user-visible registered subpaths and contents scopes. Production source-file hashing
does not currently narrow path resolution through source-location subpath prefixes; the row's `source_id` root plus
`source_files.relative_path` is the durable file path authority. Future source-location-scoped hash admission can reuse
the same resolver after a scoped candidate read is added.

## BLAKE3 Admission And Batch Behavior

Hash admission is bounded and deterministic. The store-owned candidate read supports source scope and explicit
`source_file_id` lists. It applies the media-relevant source-file inventory policy:

- include present `audio`, `video`, and `image` rows;
- include present `unsupported` rows only when `file_kind = 'cue_sheet'`;
- exclude docs, logs, archives, other unsupported files, unknown files, missing rows, and removed rows.

A candidate needs BLAKE3 evidence when no `source_file_facts` row exists, the content hash is absent, the stored hash
algorithm is not `blake3`, or the observed facts read model would be stale against the current `source_files` basis.

The batch path uses a default bounded limit when none is supplied and caps oversized requests. It orders candidates by
lowercased `relative_path`, then `source_file_id`. Each candidate is resolved through the backend resolver and then
hashed through the existing BLAKE3 evidence commit path. Per-file failures are recorded as outcomes and the batch
continues. If a source-scope batch discovers the source root itself is unavailable, missing, or blocked, it reports that
typed failure and stops the source batch without treating the source as an empty success.

## Boundary Command

The current product boundary exposes bounded source-file hash evidence maintenance through:

| Layer                   | Command                                                      |
| ----------------------- | ------------------------------------------------------------ |
| Rust protocol           | `SourceFileHash.HashSourceFilesBlake3`                       |
| Generated TS contract   | `hashSourceFilesBlake3`                                      |
| Desktop IPC/preload API | `library.hashing.hashSourceFilesBlake3({ sourceId, limit })` |

The public request is source-scoped: `sourceId` is required and `limit` is optional. The renderer never supplies
filesystem paths, never resolves a `source_file_id` to a path, and never calls store internals. Explicit source-file-id
admission remains store-only until a separate boundary use case needs it.

The command is always bounded. A missing limit uses the backend default; oversized limits are capped by backend
behavior. Non-positive source ids and zero limits are invalid requests.

The reply includes `effectiveLimit`, per-file `outcomes`, `hashedCount`, `skippedCount`, `failedCount`,
`remainingCandidates`, and optional `sourceFailure`.

Per-file outcomes are typed as `hashed`, `skipped`, or `failed`. Successful `hashed` outcomes expose
`contentHashAlgorithm = blake3`, the lowercase hex digest, and the accepted artifact/work ids. They do not expose local
filesystem paths. Failures are typed as source-file unavailable, source-root unavailable/missing/blocked, invalid
relative path, root escape, physical-file missing/blocked, file open/read failure, basis changed, or store failure.

`sourceFailure` distinguishes source-level problems from an empty candidate set. A source that is unknown, unavailable,
missing, or blocked must not be reported as an empty success.

Hash evidence commits still go through the inspect-source artifact and observed-facts authority path. Hash evidence
commits do not claim a maintained contents or navigation scope by themselves; attachment, primary-media, and track
identity maintenance are explicit follow-on phases. The service also runs one bounded internal attachment
materialization unit for the same source after a successful
non-source-failure manual hash batch. The public hash reply remains hash-only and does not report attachment work.

## Media Probe Policy

Media probe observations v0 are store-owned source-file evidence jobs. The v0 adapter is
`dekzer.source_file_media_probe.symphonia` version `1`, using the pure Rust Symphonia metadata/container reader for audio
headers and container facts. The job does not decode packets into waveform buffers and does not generate analysis output.

Probe commits use the same inspect-source work/run/artifact plus observed-facts authority path as BLAKE3 hashing. The
accepted artifact is an inline JSON `inspection_result` that records adapter key/version, source file id, basis
fingerprint, and the observed probe fields.

`source_file_facts` remains one accepted row per `source_file_id`, so current compatible evidence must be merged:

- a probe commit preserves current BLAKE3 hash evidence only when the previous `source_file_facts` basis still matches the
  current `source_files` row;
- a BLAKE3 hash commit preserves current probe fields only when the previous `source_file_facts` basis still matches the
  current `source_files` row;
- stale hash evidence is not resurrected by probing;
- stale probe fields are not resurrected by hashing.

The probe captures source-file basis before reading metadata and re-reads basis before committing. If the basis changes
while probing is in flight, the job rejects the commit with a typed basis-change result.

Bounded probe admission is deterministic and source-owned. v0 source-scope media probe admission is audio-only: it
admits present `audio` rows ordered by lowercased relative path then `source_file_id`. Video files remain media-relevant
inventory rows but are not admitted to media probe v0 until a video-capable adapter is selected. Direct probing of a
video file returns a typed `UnsupportedMediaKind` failure and does not write `source_file_facts`. Images are not admitted for
v0 media probing. CUE sheets are excluded from probe admission and are not parsed or paired.

Missing physical files, source lifecycle failures, invalid relative paths, root escapes, unreadable files, unsupported
formats, and basis changes are typed outcomes. They do not write fake facts.

## Service-Owned Source Maintenance

The boundary service owns the current source maintenance orchestration path. After a root scan publishes its terminal
`SourceScanCompleted` event and the maintained read-model invalidations for that scan, the service requests one bounded
source maintenance unit for that same durable source. The scan command field is still named `rootId` because the public
roots API uses root language, but registered local roots are source rows and the `rootId` value is the durable
`source_id` used by source maintenance. The renderer does not schedule this work, does not retry batches to keep the
source current, and does not resolve filesystem paths.

The current trigger model is intentionally narrow:

- successful root scan completion requests one bounded maintenance unit for the completed source;
- duplicate source maintenance requests are deduped while pending or active;
- blocked, failed, or cancelled scans do not automatically request source maintenance;
- maintained snapshot invalidation alone does not schedule maintenance unless it came from the successful scan
  completion path or an explicit maintenance command.

Maintenance uses the existing store-owned `hash_source_file_blake3_batch`,
`materialize_attachments_for_source`, and `probe_source_file_media_batch` authority paths in that order. Scan-triggered
maintenance performs only one small bounded unit; it does not synchronously drain a large source. Remaining candidates
stay discoverable through source maintenance candidate counts and can be handled by a later explicit command or future
scheduler request. A source-level unavailable, missing, blocked, or unknown-source state is reported as a typed
maintenance outcome and stops the source run; it is not treated as an empty success. A per-file hash or probe failure
can leave candidates behind, but the bounded unit does not spin on the same failing candidate.

The controller is service-owned scheduling state only. It does not persist durable truth, create identity rows, or
promote source files. In the current implementation it is synchronous on the service/scan execution path after terminal
scan publication; it does not create a standalone scheduler thread or hidden background drain loop. The current
scan-triggered path clears the in-memory pending source request after one bounded run, so future scheduler or explicit
commands must request additional work. Service shutdown requests maintenance stop, cancels active scan work through the
existing root-work cancellation path, and joins active scan threads. If shutdown happens while a file is being hashed, the
current file read may finish before the next stop check; no additional hash pass starts after the stop request.

After the bounded hash phase, the same service-owned cycle runs one bounded attachment materialization phase for that
source. It can run even when the hash phase finds no remaining hash work, so already-current BLAKE3 facts can still
receive missing attachment links. The cycle then runs one bounded audio-only media probe phase.

Source maintenance emits existing maintained snapshot invalidations when phase commits advance maintained revisions.
There is no public source-maintenance event family yet. The public command/read contract is defined in
`docs/library/source/maintenance-orchestration-contract.md`. Future event variants should distinguish maintenance
activity from durable observed-facts truth and must not expose filesystem paths.

## CUE Ownership

A CUE file owns future CUE parse observations on its own `source_file_id`. An adjacent audio file owns future
audio/container/probe observations on its own source-file row. This contract does not infer CUE-to-audio pairing,
does not parse CUE sheets, and does not create segment, attachment, or track rows from CUE evidence.

## Future Work

Future work remains separate:

- production scheduler policy beyond the current scan-completion trigger
- source-location-scoped hash admission
- video-capable media probing beyond the audio-only v0 adapter
- CUE parsing
- CUE-to-audio association evidence
- attachment identity read boundary and later promotion beyond the Rust/store/service maintenance foundation
- track identity
- artwork intelligence
- waveform, stems, and preparation jobs
