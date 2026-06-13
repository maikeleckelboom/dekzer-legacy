# Default Music Source Discovery

## Scope

This document is scoped to the `music` entry point class under
[`local-browser-entry-points-contract.md`](local-browser-entry-points-contract.md). The local browser entry points
contract is the canonical owner for the general local filesystem entry point model, entry point identity, candidate
status, browse-versus-scan boundary, and source admission handoff.

This document owns the Music-specific implementation companion behavior: platform Music path resolution, Music candidate
production, Music-specific deduplication application, and admission handoff as `defaultMusicFolder`.

## Purpose

On each launch before the platform Music folder has been admitted as a source, Dekzer presents the platform Music folder
as an immediately visible browse candidate. This gives the user an obvious first local choice without silently importing,
indexing, or scanning anything.

The Music candidate is not a source root, not a registered source, and not part of source lifecycle until the user acts
and source root admission succeeds.

## Platform Music Folder Resolution

| Platform | Canonical path        | Resolution method                                                |
| -------- | --------------------- | ---------------------------------------------------------------- |
| Windows  | `%USERPROFILE%\Music` | `SHGetKnownFolderPath(FOLDERID_Music)`; not string construction  |
| macOS    | `~/Music`             | `FileManager.default.urls(for: .musicDirectory, ...)`            |
| Linux    | `~/Music`             | XDG user dirs (`xdg-user-dir MUSIC`) with `~/Music` fallback     |

**V0 implementation targets Windows.** macOS and Linux resolution rules are included for cross-platform design intent
and must not be implemented accidentally in the Windows V0 slice.

Resolution must use platform-native APIs. On Windows, `%USERPROFILE%` concatenation is not acceptable because user
profiles can be relocated. Use the Known Folder API.

If the platform API fails to return a path, discovery produces no candidate. Do not hard-code fallback strings.

## Discovery Behavior

The implemented V0 read path for this candidate is `readLocalBrowserEntryPoints`. The Music row returned by that read is
the `music` local browser entry point with the `defaultMusicFolder` admission hint when the platform Music folder is
resolved. It remains a candidate until the user acts and admission succeeds.

Discovery runs as a bounded, non-scanning read. It must not block first shell render. It may complete shortly after the
library surface mounts. Even a cheap status check can be slow on cloud-redirected, network-backed, or broken shell-folder
paths, so production status resolution must prefer platform attribute/status APIs that avoid content reads and directory
enumeration. Do not add thread-per-path timeout behavior unless the service has a safe cancellation pattern for that
exact operation. It is a shallow status check, not a scan.

Steps:

1. Resolve the platform Music folder path using the platform API.
2. Resolve shallow path status for existence and directory type only.
3. Produce a Music local browser entry point candidate when resolution succeeds.

If the path exists and is a directory, the candidate status is `available`. No scan starts automatically.

If the path does not exist, the candidate status is `missing`. V0 hides the candidate entirely. Do not create the folder
and do not show a "create Music folder" prompt.

If the path exists but cannot be statted or enumerated cheaply, the candidate status is `permissionBlocked` or
`unavailable`, depending on the resolved failure. Do not scan.

## Presentation

The Music candidate is presented as a local browser entry point or local candidate row, distinct from admitted and
scanned sources until the user explicitly acts on it.

Must show:

- source name: "Music" or localized platform equivalent;
- candidate state, such as `available`, `permissionBlocked`, or `unavailable`;
- primary action to request admission and start scan after successful registration;
- secondary action to choose another folder;
- dismissal action such as "Not now".

Must not show:

- audio row contents;
- a scanning state;
- a label implying that the folder has been indexed;
- an implication that the folder is a confirmed library root;
- source lifecycle state before admission.

The browse-versus-scan boundary for any shallow local browse behavior is owned by
[`local-browser-entry-points-contract.md`](local-browser-entry-points-contract.md).

## Admission

When the user chooses to use the Music candidate, the service runs source root admission.

The platform Music folder is classified as `defaultMusicFolder` by the source root admission policy. It passes admission
without warning or confirmation only when platform resolution and admission classification agree. See
[`root-admission-policy.md`](root-admission-policy.md).

Discovery does not pre-admit or pre-persist the source. The `defaultMusicFolder` admission hint is display/action
guidance only; it is not an admission result and does not bypass source root admission. The source is admitted and
persisted only when the user acts and the service admission path accepts registration.

Renderer code must not treat the candidate as a half-source, pass it to source lifecycle reads, create source-location
state for it, or attach source IPC subscriptions to it before admission.

The implemented `readLocalBrowserEntryPoints` boundary returns this candidate and its status only. It does not implement
local child browsing, renderer filesystem crawling, source registration, or scan start.

## Deduplication

Music discovery inherits the general deduplication contract from
[`local-browser-entry-points-contract.md`](local-browser-entry-points-contract.md). The rules below are the Music-specific
application.

If the user has already manually registered the platform Music folder path, the Music candidate is suppressed or marked
as `duplicateOfAdmittedSource` according to the active surface's projection rules. It must not appear as a second source.

Deduplication rule: if the normalized canonical form of the discovered path matches the normalized canonical form of
any existing admitted source path, discovery does not produce a second source row.

On Windows, path comparison is case-insensitive and normalized by stripping trailing separators and normalizing slashes.

If the Music folder is a parent or child of an existing registered source, it is not automatically suppressed. Standard
overlap rules from source root admission apply when the user chooses to act on it.

## Unavailable and Inaccessible States

**Folder exists, read denied:**
Show the candidate as `permissionBlocked`. Offer retry or choose another folder. Do not scan. Do not persist as admitted.

**Folder is missing after previously being available:**
On the next launch before the Music folder has been admitted as a source, re-run discovery. If the folder is now missing,
treat the candidate as `missing`. If the Music folder was previously admitted, it follows normal source unavailable
lifecycle instead of discovery candidate behavior.

**Folder is a junction or reparse point:**
If the resolved Music folder path is itself a reparse point, detect the reparse at preflight. Apply the appropriate
admission warning from [`root-admission-policy.md`](root-admission-policy.md), such as cloud-backed or removable rules.
Do not silently admit it as a plain `defaultMusicFolder`.

## Scan Behavior

Scan is always explicit. The Music candidate does not auto-scan on launch or on any subsequent launch.

When the user initiates scan from the Music candidate:

- run source root admission;
- require `accepted` with reason `defaultMusicFolder`;
- persist the source record;
- start scan only because the user explicitly initiated it;
- use `normal` scan policy;
- publish hierarchy and contents through normal admitted-source paths;
- use standard permission handling, where blocked subfolders are marked blocked and scan continues;
- finish as `completed` or `partial` if blocked subtrees exist.

## State After Admission

Once the Music folder is admitted and scanned, it becomes a normal library source with full lifecycle behavior. It is no
longer a discovery candidate.

On subsequent launches, the admitted Music folder appears through admitted-source navigation. Discovery does not
re-present it as a candidate.

## Session Dismissal

If the user dismisses the candidate without acting:

- do not persist any source record;
- do not scan;
- on the next launch before the Music folder has been admitted, re-run discovery.

Dekzer does not track "the user dismissed the Music folder suggestion" as a persistent preference in V0. The candidate
reappears each launch until the user either admits it or registers a different source.

Future: allow persistent "do not show this" preference. Not V0.

## What This Is Not

Default Music source discovery is not:

- the owner of the general local filesystem entry point contract;
- a silent background index;
- a forced library location;
- a one-time wizard the app cannot show again;
- a guarantee that the Music folder is scanned on every launch;
- a substitute for source root admission policy;
- an import from iTunes, Rekordbox, Traktor, or any third-party library format.

It is: Dekzer noticing that the platform Music folder exists and making it an obvious first local choice, while still
requiring explicit user action and source root admission.

## Future Candidates

V0 covers Windows Music discovery only. macOS and Linux paths are defined above but may not ship in V0.

The general model for additional local browser entry points is owned by
[`local-browser-entry-points-contract.md`](local-browser-entry-points-contract.md).

Future Music-specific companion behavior may include:

- source recovery for a previously admitted Music source;
- persistent dismissal preference;
- user-configured default Music path.

## Test Matrix

**Discovery:**

- Platform Music folder exists and is a directory -> candidate is `available`.
- Platform Music folder does not exist -> no visible candidate, or candidate is `missing` if a future surface chooses to
  show missing candidates.
- Platform Music folder exists but stat/access check fails -> candidate is `permissionBlocked` or `unavailable`.

**Deduplication:**

- Music folder already registered as source -> discovery produces no duplicate source row.
- Unrelated source registered, Music folder not registered -> Music candidate appears.
- Music folder is a child of a registered source -> deduplication does not suppress by itself; overlap rules apply on
  act.

**Session dismissal:**

- User dismisses candidate -> no source persisted, no scan started.
- On next launch after dismissal -> candidate reappears while the Music folder remains unadmitted.

**Admission on act:**

- User starts scan from candidate -> admission runs, `defaultMusicFolder` is returned, source is persisted, explicit scan
  starts.
- User chooses folder picker from candidate -> folder picker opens, user selects a different path, and that path goes
  through normal admission.

**Post-admission:**

- Admitted Music folder appears as normal library source, not as discovery candidate, on next launch.
- Discovery does not re-present an already admitted path.
