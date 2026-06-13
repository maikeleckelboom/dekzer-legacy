# Source Root Admission Policy

## Purpose

Source root admission classifies a user-selected path before it becomes a registered library source. Admission runs before registration, before scan, and before any source appears in the browser. A path that fails admission is never persisted as a source root and never scanned.

The policy exists to distinguish three real cases that look identical at the filesystem level:

- a 2TB external drive dedicated to DJ music — legitimate, must work
- `C:\` — the operating system volume, must be blocked
- `D:\` — could be either of the above, requires classification

Admission is owned by the service layer. The renderer displays results; it does not own the policy.

---

## Displayable Local Entry Points

A displayable local browse entry point does not imply an admissible source root. Entry points are pre-admission browse
roots until the user selects a path and the service runs this admission policy.

The OS drive can be shown as a local browse entry point while remaining rejected as a source root. Broad roots remain
confirmation-required or rejected according to this policy, regardless of where they appeared in a local entry point
surface.

Admission remains the only path to source persistence and scan. A rejected path must not create a source, source
lifecycle row, source location, source scan root, or scan job.

---

## Admission pipeline

Admission runs in four sequential stages. A rejection at any stage terminates the pipeline and returns a result immediately.

```
selected path
  → 1. path classification
  → 2. access capability preflight
  → 3. traversal policy assignment
  → 4. scan policy assignment
  → admission result
```

---

## Stage 1: Path classification

### Hard-rejected paths

These paths are rejected with no user override available in normal mode. They are not music source roots under any interpretation.

Advanced override, if ever added, is outside V0 scope and must be separately designed. V0 has no override path for system roots or system directories.

| Category                        | Windows examples                                               | macOS examples                                         |
| ------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------ |
| System volume root              | `C:\` (when C is the OS drive)                                 | `/`                                                    |
| OS system directories           | `C:\Windows`, `C:\Windows\System32`                            | `/System`, `/Library`, `/usr`, `/bin`, `/etc`, `/sbin` |
| Application install directories | `C:\Program Files`, `C:\Program Files (x86)`, `C:\ProgramData` | `/Applications`                                        |
| User accounts container         | `C:\Users`                                                     | `/Users`                                               |
| Known system recovery paths     | `C:\Recovery`, `C:\System Volume Information`, `C:\$SysReset`  | —                                                      |

Determining whether a drive is the system/OS volume must use platform APIs, not string guessing. On Windows, compare against `GetSystemDrive()` or the `CSIDL_WINDOWS`/`FOLDERID_Windows` resolved volume. Do not assume `C:` is always the system drive.

### Confirmation-required paths

These can proceed only after explicit user confirmation. Admission returns `acceptedWithWarning` + `requiresConfirmation: true`.

| Category                      | Examples                                                                                                                | Reason                                                                                 |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| User home root                | `C:\Users\<name>`, `~`                                                                                                  | Contains Desktop, Downloads, AppData, dev repos, caches                                |
| Downloads, Desktop, Documents | `C:\Users\<name>\Downloads`                                                                                             | High noise-to-music ratio                                                              |
| Dev/project roots             | Any path whose top-level sample contains `node_modules`, `.git`, `src`, `packages`, `target`, `build`, `dist`, `vendor` | Package managers and build artifacts                                                   |
| Data/external volume root     | `D:\`, `E:\` (non-system drive letters on Windows); `/Volumes/<name>` on macOS                                          | May be a 2TB DJ music disk or a general backup disk — cannot tell without confirmation |
| Cloud-backed root             | OneDrive root, iCloud Drive root, Dropbox root, Google Drive root                                                       | Placeholder files, offline risk, auto-download triggers                                |
| Network share root            | `\\NAS\Share`, SMB mounts, network-mapped drives                                                                        | Credential expiry, offline, latency, slow enumeration                                  |

### Directly accepted paths

These pass admission without warning or confirmation.

| Category                            | Examples                                                                                             |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Platform default music folder       | `%USERPROFILE%\Music`, `~/Music`                                                                     |
| Named music/DJ subfolder            | Any path ending in or containing: Music, DJ Library, Audio, Records, Tracks, Samples, Sets, Releases |
| Specific user-selected subdirectory | Any path not matching a rejected or warned category and passing access preflight                     |

### Duplicate and overlap paths

These are checked after category classification.

| Situation                                                      | Decision                                                                                                                                    |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Canonical path matches an existing admitted source exactly     | `rejected`, reason: `exactDuplicate`. Restore existing source visibility.                                                                   |
| Selected path is a subdirectory of an existing admitted source | `rejected` by default, reason: `nestedChildOfExistingSource`. Warn and explain overlap risk.                                                |
| Selected path is a parent of an existing admitted source       | `acceptedWithWarning`, reason: `nestedParentOfExistingSource`. Require confirmation. Warn that parent and child may produce duplicate rows. |

---

## Stage 2: Access capability preflight

Preflight must be cheap. It must not scan the source or traverse the directory tree.

**Preflight budget:**

- Stat the selected root (existence, type, basic metadata) — must complete in < 1s
- Enumerate top-level directory entries only — at most 50 entries, hard timeout 3s total
- No recursive traversal at any depth
- Cloud placeholder enumeration must not trigger content download

**Preflight checks in order:**

1. Path exists → if not: `rootNotFound`, `rejected`
2. Path is a directory → if not: `rejected`
3. Can stat metadata → if not: `rootMetadataUnreadable`, `rejected`
4. Can enumerate root directory → if not: `rootEnumerationDenied`, `rejected`
5. Top-level sample contains hard-rejected folder names (`Windows`, `Program Files`, etc.) → escalate category accordingly
6. If path is a drive root: classify as system or data volume via platform API
7. Detect reparse point at root → flag as `reparsePointAtRoot` in traversal policy
8. Detect network/removable/cloud signals

If preflight times out entirely: `acceptedWithWarning` for non-system paths, `rejected` for unverified system-adjacent paths + `preflightTimeout` reason.

---

## Stage 3: Traversal policy

Traversal policy governs what the scanner does at runtime. It is derived from the source's admission classification and current policy version at scan time. The source persists its admission classification and admission policy version — not a frozen traversal configuration. Policy upgrades may recompute traversal behavior for existing sources without requiring re-admission.

**Rules applying to all sources:**

- Never follow symlinks or junctions that escape the admitted root.
- Detect reparse points. Default: skip with a blocked-subtree record. Do not silently traverse.
- Detect and halt on symlink/junction loops.
- Skip these folder names at every depth regardless of source type: `$RECYCLE.BIN`, `System Volume Information`, `$SysReset`, `Recovery`, `WindowsImageBackup`, `Windows`, `Program Files`, `Program Files (x86)`, `ProgramData`.
- Skip `.git` directories.
- Skip `node_modules` directories.
- Never require admin elevation to continue traversal. If a subtree is access-denied, record it as blocked and continue siblings.
- Never escape the admitted root boundary.

**Additional rules for data/external volume roots:** apply the folder name skip list aggressively. Do not assume folder names are safe because the volume passed admission.

---

## Stage 4: Scan policy

Scan policy governs how the scanner operates on an admitted source. It is derived from the source's admission classification and current policy version at scan time. The source persists its admission classification; the policy itself may be recomputed from that classification as policy versions evolve.

Do not persist scan policy as an unversioned permanent enum. Policy upgrades must be able to recompute behavior for existing sources without requiring re-admission.

Each admitted source should carry an `admissionPolicyVersion` so future policy changes can identify which sources were admitted under older rules.

| Policy            | When assigned                                                           | Behavior                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ----------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `normal`          | Default music folder, named music subfolder, specific user subdirectory | Standard traversal. Permission failures mark subtree blocked, scan continues. Incremental hierarchy publication.                                                                                                                                                                                                                                                                                                          |
| `broadConfirmed`  | User home root or broad user folder admitted with explicit confirmation | Apply name-based skip list aggressively. Media-relevance preflight per top-level folder. Warn if audio density is very low after initial pass. Cancel allowed without source removal.                                                                                                                                                                                                                                     |
| `dedicatedVolume` | Data/external volume root admitted with confirmation                    | System folder name skip list applied. Chunked traversal with bounded batch sizes. Top-level hierarchy published first. Permission failures mark subtree blocked. Cancel without source removal. Reopen preserves partial hierarchy.                                                                                                                                                                                       |
| `removable`       | Removable drive root                                                    | Same as `dedicatedVolume`. Drive disconnection during scan → source unavailable state, not scan failure. On reconnect, verify path identity before resuming.                                                                                                                                                                                                                                                              |
| `network`         | Network share root                                                      | Slow enumeration tolerance. Credential failure → mark subtree blocked with `networkCredentialFailure`, not missing. Timeout failures mark subtrees blocked, not absent.                                                                                                                                                                                                                                                   |
| `cloud`           | Cloud-backed root                                                       | Enumerate metadata only. Do not trigger content download. Mark placeholder files as `cloudOnly` where the platform API can identify them safely. If cloud placeholder detection is unavailable on the current platform, cloud roots remain warning-classified and scan must avoid file-content reads entirely. V0 behavior is platform-API-dependent; do not invent placeholder detection without a reliable API surface. |

---

## Admission result model

The admission result is a structured value produced by the service and consumed by the renderer.

```
AdmissionResult {
  decision:                  "accepted" | "acceptedWithWarning" | "rejected"
  reason:                    AdmissionReason       -- see reason codes below
  admissionClassification:   AdmissionCategory     -- stored with source for policy recomputation
  admissionPolicyVersion:    u32                   -- policy version at admission time
  requiresConfirmation:      bool                  -- must renderer get explicit user approval?
  canPersist:                bool                  -- may service persist this as a source?
  canScanAfterAdmission:     bool                  -- may scan begin? always false when requiresConfirmation is true
  suggestedAlternatives:     PathCandidate[]       -- renderer-displayable alternative paths
  messageKey:                string                -- renderer copy key, not backend prose
}
```

The renderer does not generate admission logic. It receives this result, displays it using `messageKey`-keyed copy, and either proceeds (if `decision == accepted` and `requiresConfirmation == false`) or shows a confirmation dialog (if `requiresConfirmation == true`) before calling the registration command.

`canScanAfterAdmission` is always `false` when `requiresConfirmation` is `true` — the renderer must not initiate scan before the user has confirmed. Initial admission results requiring confirmation must not allow scan. After user confirmation, the service may issue a confirmed acceptance or repeat admission in confirmed mode; only that confirmed service path may allow registration and scan.

A `rejected` result must never result in source registration or scan.

---

## Reason codes

### Path-based reasons

| Code                    | Meaning                                    |
| ----------------------- | ------------------------------------------ |
| `defaultMusicFolder`    | Platform music folder                      |
| `namedMusicFolder`      | Named music subfolder                      |
| `specificUserFolder`    | Specific accepted subdirectory             |
| `dedicatedVolumeRoot`   | Whole external/data drive root             |
| `userHomeRoot`          | User home directory                        |
| `broadUserFolder`       | Downloads, Desktop, Documents              |
| `devProjectRoot`        | Path with dev signals in top level         |
| `systemVolumeRoot`      | Root of OS installation volume             |
| `systemDirectory`       | OS system directory                        |
| `appInstallDirectory`   | Application install directory              |
| `userAccountsContainer` | Parent directory containing all user homes |
| `cloudBackedRoot`       | Cloud sync root                            |
| `networkShareRoot`      | Network share or mapped drive              |

### Access-based reasons

| Code                          | Meaning                                       |
| ----------------------------- | --------------------------------------------- |
| `rootNotFound`                | Path does not exist                           |
| `rootNotDirectory`            | Path exists but is not a directory            |
| `rootMetadataUnreadable`      | Cannot stat the path                          |
| `rootEnumerationDenied`       | Root directory cannot be enumerated           |
| `permissionDenied`            | General access denied                         |
| `networkCredentialsRequired`  | Network share requires authentication         |
| `cloudPlaceholderUnavailable` | Cloud root cannot be accessed offline         |
| `removableDriveUnavailable`   | Removable drive is not present                |
| `preflightTimeout`            | Preflight did not complete within budget      |
| `reparsePointAtRoot`          | Root path is a reparse point/junction/symlink |

### Overlap reasons

| Code                           | Meaning                                         |
| ------------------------------ | ----------------------------------------------- |
| `exactDuplicate`               | Canonical path matches existing admitted source |
| `nestedChildOfExistingSource`  | Path is a subdirectory of an existing source    |
| `nestedParentOfExistingSource` | Path contains an existing admitted source       |

---

## Permission failures during scan

Permission failures during scan must not abort the source scan. They must not cause unproven files or subtrees to be marked as missing or deleted.

**Required behavior on permission failure:**

- Record a `blockedSubtree` or `blockedFile` observation per affected path with a reason code.
- Continue scanning siblings and other branches.
- Source scan final state becomes `partial` (not `completed`) if blocked subtrees exist.
- Blocked state is retryable via explicit rescan.
- Blocked subtrees are visible in source lifecycle state.
- Renderer displays blocked state using reason codes, not raw error messages.

**Blocked-subtree reason codes during scan:**

- `permissionDenied`
- `accessDenied`
- `networkTimeout`
- `networkCredentialFailure`
- `cloudFileUnavailable`
- `cloudDownloadRequired`
- `reparsePointSkipped`
- `reparseLoopDetected`
- `longPathExceeded`
- `filenameTooLong`

---

## Live-performance safety

The admission and scan system must remain safe when Dekzer is used in a performance context.

- No automatic scans on source registration without explicit user action.
- No cloud file download triggered by scan during performance mode (when implemented).
- Source lifecycle state changes during scan must not disrupt active deck operation.
- Scan must be cancellable at any point without source removal.

---

## Edge case decisions

| Situation                                      | Decision                                                                                                                                                             |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| User selects `C:\`                             | Hard rejected. Reason: `systemVolumeRoot`. Suggest Music folder.                                                                                                     |
| User selects `E:\` (non-system drive)          | Warning + confirmation required. Reason: `dedicatedVolumeRoot`. Scan policy: `dedicatedVolume`.                                                                      |
| User selects `~/Music`                         | Accepted. Reason: `defaultMusicFolder`. Scan policy: `normal`.                                                                                                       |
| User selects `~/Downloads`                     | Warning + confirmation. Reason: `broadUserFolder`. Suggest Music folder.                                                                                             |
| User selects `D:\DJ Library`                   | Accepted. Reason: `namedMusicFolder`. Scan policy: `normal`.                                                                                                         |
| Drive letter changes (`E:\` → `F:\`)           | Existing source becomes unavailable. Not removed. User can relink. Path string identity is weak for removables — documented, deferred to future volume UUID pairing. |
| Removable drive disconnects mid-scan           | Source unavailable state. Scan stops without corruption. Partial hierarchy preserved. On reconnect, verify path identity before resuming.                            |
| OneDrive root selected                         | Warning + confirmation. Reason: `cloudBackedRoot`. Scan policy: `cloud`.                                                                                             |
| NAS share selected                             | Warning + confirmation. Reason: `networkShareRoot`. Scan policy: `network`.                                                                                          |
| Symlink inside source escapes root             | Skip with `reparsePointSkipped` record. Do not follow.                                                                                                               |
| `$RECYCLE.BIN` found inside data volume        | Always skipped at traversal level. Not recorded as a library folder.                                                                                                 |
| `node_modules` found inside data volume        | Skipped by default.                                                                                                                                                  |
| Selected path already registered (exact match) | Rejected. Reason: `exactDuplicate`. Restore existing source visibility.                                                                                              |
| Selected path is parent of existing source     | Warning + confirmation. Reason: `nestedParentOfExistingSource`.                                                                                                      |
| Selected path is child of existing source      | Rejected by default. Reason: `nestedChildOfExistingSource`.                                                                                                          |
| Preflight times out (> 3s)                     | Accepted with warning for non-system paths. Reason includes `preflightTimeout`.                                                                                      |
| Root enumerates but is empty                   | Accepted. Scan completes. Source state: authoritative empty.                                                                                                         |
| Root contains no audio files                   | Accepted. Scan completes. Source state: authoritative empty.                                                                                                         |
| Permission denied on root directory            | Rejected. Reason: `rootEnumerationDenied`. Suggest alternatives.                                                                                                     |
| Permission denied on one subfolder             | Admitted. That subtree is marked `blockedSubtree` during scan. Siblings continue.                                                                                    |
| Cloud placeholder read would trigger download  | Preflight must not trigger download. Enumerate metadata only.                                                                                                        |
| Very large drive with millions of files        | `dedicatedVolume` policy. Top-level published first. Chunked traversal. Cancel without removal.                                                                      |

---

## Drive letter instability

Windows drive letters for removable and external drives are assigned dynamically. `E:\` today may be a different physical device than `E:\` tomorrow.

**V0:** source identity is the canonical path string (case-insensitive on Windows). No volume UUID pairing.

**Source lifecycle contract:** if a source path becomes inaccessible between sessions (drive not connected, letter changed), the source transitions to `unavailable`, not `removed`. The source record is preserved. The user can rescan when the drive is reconnected, or relink if the letter changed.

**Future:** optionally pair source path with volume label and/or filesystem UUID for automatic relink. Not V0.

---

## Canonical path identity and deduplication

On Windows, path comparison for duplicate detection must be case-insensitive and normalized.

- Trailing separators stripped.
- Forward/backward slash normalized.
- Case-folded for comparison on Windows.
- UNC paths normalized for network shares.

Network and share path identity may be provider-dependent. Case sensitivity can vary between local NTFS, SMB servers, and NAS devices. V0 normalizes UNC strings using case-insensitive comparison but does not guarantee physical identity equivalence for network paths.

V0 deduplication uses normalized path string comparison only. Junction/symlink resolution during deduplication is not V0.

---

## Renderer contract

The renderer:

- calls the admission command with the user-selected path
- receives an `AdmissionResult`
- displays copy using `messageKey`, not backend-generated prose
- shows `suggestedAlternatives` from the result
- shows a confirmation dialog if `requiresConfirmation: true` and waits for user decision before proceeding
- calls registration only if the user confirms and `canPersist: true`
- calls scan only if `canScanAfterAdmission: true` and the user initiates it

The renderer must not:

- perform path classification itself
- block or warn based on renderer-side path pattern matching
- show confirmation when backend did not require it
- proceed with registration after a `rejected` result
- invent alternative paths not present in `AdmissionResult.suggestedAlternatives`

---

## Admission proof and confirmation integrity

Admission must be unforgeable. The renderer must not be able to bypass a warning result by simply calling registration directly with the same path.

For paths that require confirmation (`requiresConfirmation: true`), the service must enforce that registration is accompanied by proof of confirmed admission. Acceptable mechanisms:

- a short-lived admission decision id issued by the service at admission time, consumed once at registration
- a confirmed-mode flag in the registration command that triggers a repeat admission check, and the service validates the path still meets the conditions under which confirmation is appropriate
- any equivalent that prevents the renderer from calling registration without having gone through the admission + user-confirmation flow

The exact mechanism is an implementation detail. The invariant is: for warning-classified paths, registration without proof of user confirmation must fail at the service layer, not just at the renderer.

---

## Policy evolution and existing sources

Admission rules will change as Dekzer matures. New categories will be added. Existing category boundaries may shift. Cloud and network handling may improve.

Policy changes must not automatically delete or invalidate existing admitted sources. An existing source that would be flagged under a newer policy version should instead be marked for review, not removed.

Required behavior when policy version advances:

- existing sources with an older `admissionPolicyVersion` remain valid until explicitly reviewed or rescanned
- sources whose stored `admissionClassification` is no longer compatible with current policy may be surfaced as "needs review" in source lifecycle state
- the user decides whether to keep, rescan, or remove them
- Dekzer must never silently remove a source because the admission policy changed

---

## Test matrix

The following invariants must be covered by admission unit/integration tests.

**Path classification:**

- `C:\` (system volume) → hard rejected, `systemVolumeRoot`
- `C:\Windows` → hard rejected, `systemDirectory`
- `C:\Program Files` → hard rejected, `appInstallDirectory`
- `C:\Users` → hard rejected, `userAccountsContainer`
- `D:\` (non-system data drive) → warning + confirmation, `dedicatedVolumeRoot`
- `%USERPROFILE%\Music` → accepted, `defaultMusicFolder`
- `D:\DJ Library` → accepted, `namedMusicFolder`
- `~` user home root → warning + confirmation, `userHomeRoot`
- `~/Downloads` → warning + confirmation, `broadUserFolder`
- Path with `node_modules` in top-level → warning + confirmation, `devProjectRoot`

**Access capability:**

- Non-existent path → rejected, `rootNotFound`
- Path that is a file → rejected
- Path where root enumeration is denied → rejected, `rootEnumerationDenied`
- Path where subfolder is denied → admitted, subfolder marked blocked during scan

**Deduplication:**

- Same path registered twice → second admission rejected, `exactDuplicate`
- Child path of existing source → rejected, `nestedChildOfExistingSource`
- Parent path of existing source → warning + confirmation, `nestedParentOfExistingSource`

**Scan policy:**

- `dedicatedVolumeRoot` admission → scan policy is `dedicatedVolume`
- `defaultMusicFolder` admission → scan policy is `normal`
- `cloudBackedRoot` admission → scan policy is `cloud`

**Admission proof:**

- Registering a warning-classified path without a valid admission proof fails at the service layer
- Confirmed admission for a `dedicatedVolumeRoot` cannot be reused to register a different path
- A `rejected` admission result cannot lead to a persisted source record

**Permission failures during scan:**

- Blocked subtree → scan continues, subtree marked `blockedSubtree`
- Source scan final state is `partial` when blocked subtrees exist
- Blocked subtrees are not marked `missing`
- Scan state survives reopen

---

## What is not covered here

- Track identity and audio deduplication: see identity docs
- Contents browse policy: see `contents-browse-policy.md`
- Audio browse row: see `audio-browse-row-implementation.md`
- Source scan implementation details: service-level
- File classification and media class taxonomy: separate doc
- Default music source discovery behavior: see `default-music-source-discovery.md`
- Exclave boundary adoption: deferred
