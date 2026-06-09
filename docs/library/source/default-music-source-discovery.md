# Default Music Source Discovery

## Purpose

On first launch, Dekzer presents the platform music folder as an immediately visible source candidate without requiring the user to navigate a folder picker or understand Dekzer's source model. This is a product-first-run experience, not a silent import or background index.

---

## Platform music folder resolution

| Platform | Canonical path        | Resolution method                                                |
| -------- | --------------------- | ---------------------------------------------------------------- |
| Windows  | `%USERPROFILE%\Music` | `SHGetKnownFolderPath(FOLDERID_Music)` — not string construction |
| macOS    | `~/Music`             | `FileManager.default.urls(for: .musicDirectory, ...)`            |
| Linux    | `~/Music`             | XDG user dirs (`xdg-user-dir MUSIC`) with `~/Music` fallback     |

**V0 implementation targets Windows.** macOS and Linux resolution rules are included for cross-platform design intent and must not be implemented accidentally in the Windows V0 slice.

Resolution must use platform-native APIs, not path string construction. On Windows, `%USERPROFILE%` concatenation is not acceptable because user profiles can be relocated. Use the Known Folder API.

If the platform API fails to return a path, discovery produces no candidate. Do not hard-code fallback strings.

---

## Discovery behavior

Discovery runs during startup as a bounded, non-scanning startup task. It must not block first shell render. It may complete shortly after the library panel mounts. Even a cheap stat can hang on cloud-redirected, network-backed, or broken shell-folder paths, so discovery must be async with a hard timeout (suggested: 2s). It is a stat check, not a scan.

Steps:

1. Resolve the platform music folder path using the platform API.
2. Stat the resolved path (existence and directory type only).
3. Produce a `DiscoveredMusicSource` candidate.

**If the path exists and is a directory:**
Candidate is `present`. Visible in the library navigation as a default local source. No scan starts automatically.

**If the path does not exist:**
Candidate is `absent`. V0: hide entirely. Do not create the folder. Do not show a "create Music folder" prompt.

**If the path exists but cannot be statted:**
Candidate is `unavailable`. Show with inaccessible state. Do not scan.

---

## Presentation

The discovered default source is presented in the library navigation tree as a **suggested candidate**, distinct from admitted/scanned sources until the user explicitly acts on it.

**Must show:**

- Source name: "Music" (or localized platform equivalent)
- State: not scanned — no audio rows are available yet
- Primary action: "Start scan"
- Secondary action: "Choose another folder"
- Dismissal: "Not now" or equivalent

**Must not show:**

- Audio row contents — the source is not scanned and contains no accepted rows
- A "scanning" or "loading" state — no automatic process has run
- A "ready to browse" label — there is nothing to browse until scanned
- Any implication that the folder has been indexed or is a confirmed library root

If V0 later adds shallow top-level browse before scan (showing immediate child folders of the Music folder without full scan), that is a separate design decision with its own admission and partial-state semantics. Do not implement it by accident.

---

## Admission

When the user chooses to use the Music folder candidate (by starting a scan or explicitly accepting it), the service runs source root admission.

The platform music folder is classified as `defaultMusicFolder` by the source root admission policy. It passes admission without warning or confirmation. See `source-root-admission-policy.md`.

Discovery does not pre-admit or pre-persist the source. The source is admitted and persisted only when the user acts.

**A discovery candidate is not a source root, not a registered source, and not part of source lifecycle until admitted.** Renderer code must not treat the candidate as a half-source, pass it to source lifecycle reads, or attach IPC subscriptions to it before admission.

---

## Deduplication

If the user has already manually registered the platform music folder path, the discovery candidate is suppressed. It does not appear as a second source.

Deduplication rule: if the normalized canonical form of the discovered path matches the normalized canonical form of any existing admitted source path, discovery does not produce a visible candidate.

On Windows, path comparison is case-insensitive and normalized (trailing separators stripped, slashes normalized).

If the Music folder is a parent or child of an existing registered source, it is not automatically suppressed. Standard overlap rules from the source root admission policy apply when the user chooses to act on it.

---

## Unavailable and inaccessible states

**Folder exists, read denied:**
Show candidate as `permissionBlocked`. Display: cannot access Music folder. Offer retry or choose another folder. Do not scan. Do not persist as admitted.

**Folder is missing after previously being available:**
On next launch, re-run discovery. If the folder is now absent, treat as `absent`. The source is not marked deleted if it was previously admitted — it follows normal source unavailable lifecycle.

**Folder is a junction/reparse point:**
If the resolved Music folder path is itself a reparse point (e.g., the user redirected it to an external drive or cloud folder), detect the reparse at preflight. Apply the appropriate admission warning from `source-root-admission-policy.md` (cloud-backed or removable rules). Do not silently admit as a plain `defaultMusicFolder`.

---

## Scan behavior

Scan is always explicit. The discovery candidate does not auto-scan on launch or on any subsequent launch.

When the user initiates scan of the Music folder candidate:

- Run source root admission (produces `accepted`, `defaultMusicFolder`, scan policy: `normal`).
- Persist source record.
- Start scan using `normal` scan policy.
- Incremental hierarchy publication.
- Standard permission handling: blocked subfolders are marked blocked, scan continues.
- Source state after scan: `completed` or `partial` if blocked subtrees exist.

---

## State after the source is admitted

Once the Music folder is admitted and scanned, it becomes a normal library source with full lifecycle behavior. It is no longer a "discovery candidate." Discovery candidate presentation applies only until the source is admitted and persisted.

On subsequent launches, the admitted Music folder appears as a normal library root. Discovery does not re-present it as a candidate.

---

## Session dismissal

If the user dismisses the candidate without acting:

- Do not persist any source record.
- Do not scan.
- On next launch, re-run discovery. The candidate reappears.

Dekzer does not track "the user dismissed the Music folder suggestion" as a persistent preference in V0. The candidate reappears each launch until the user either admits it or registers a different source.

Future: allow persistent "don't show this" preference. Not V0.

---

## What this is not

Default music source discovery is not:

- a silent background index — no scan runs without user action
- a forced library location — the user can dismiss or choose differently
- a one-time wizard the app cannot show again
- a guarantee that the Music folder is scanned on every launch
- a substitute for the source root admission policy
- an import from iTunes, Rekordbox, Traktor, or any third-party library format

It is: Dekzer noticing that the platform music folder exists and making it the obvious first choice, requiring no configuration.

---

## Future: multi-platform and additional candidates

V0 covers Windows only. macOS and Linux paths are defined above but may not ship in V0.

Future additional candidates (post-V0):

- External drive roots detected as connected and containing music-named top-level folders
- Previously used sources from a prior Dekzer session (source recovery, not discovery)
- User-configured default source paths

These are not part of default music source discovery V0. They belong to a separate source recovery or quick-start feature.

---

## Test matrix

**Discovery:**

- Platform music folder exists and is a directory → candidate is `present`
- Platform music folder does not exist → no candidate
- Platform music folder exists but stat/access check fails → candidate is `unavailable`

**Deduplication:**

- Music folder already registered as source → discovery produces no visible candidate
- Unrelated source registered, Music folder not registered → discovery candidate appears
- Music folder is a child of a registered source → deduplication does not suppress (overlap rules apply on act)

**Session dismissal:**

- User dismisses candidate → no source persisted, no scan started
- On next launch after dismissal → candidate reappears (V0 has no persistent dismissal)

**Admission on act:**

- User starts scan from candidate → admission runs, `defaultMusicFolder`, `normal` policy, source persisted, scan starts
- User chooses folder picker from candidate → folder picker opens, user selects a different path, that path goes through normal admission

**Post-admission:**

- Admitted Music folder appears as normal library source, not as discovery candidate, on next launch
- Discovery does not re-present an already-admitted path
