---
status: archived-evidence
canonical-contract: docs/source-root-scan-admission-contract.md
date: 2026-05-27
owner: product-architecture
authority: evidence-only
do-not-use-as-architecture-authority: true
purpose: Repo reconnaissance evidence for broad-root source scan implementation.
source-inputs:
  - deepseek-final-source-root-scan-pipeline-reconnaissance
  - sub-agent-windows-reparse-junction-behavior
  - sub-agent-work-executor-inspection-adapter
  - sub-agent-database-pollution-persistence-path
  - sub-agent-source-identity
---

# Source Root Scan Reconnaissance Evidence

This file records repo reconnaissance evidence for the broad-root scan failure, especially the case where a user selects
a whole Windows drive such as C:\.

This file is not architecture authority. The canonical design authority is
docs/source-root-scan-admission-contract.md.

Use this file to answer:

- What does the current code appear to do?
- Where does the C:\ failure path come from?
- Which implementation gaps are proven by repo inspection?
- Which fixes should the next implementation prompt target?

Do not use this file to redefine source-root scanning, candidate admission, root identity, work budgeting, reparse
behavior, or evidence vocabulary. Those belong in the canonical contract.

## Executive evidence verdict

The current scanner appears adequate for curated music folders, but it is not safe for broad roots such as C:\.

| Finding                                           | Evidence summary                                                                                                               |
|---------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------|
| Traversal-first                                   | The scanner walks the selected root before root classification, density sampling, or broad-root scan planning.                 |
| Not candidate-gated before persistence            | Files are persisted to source_files before the inspection gate decides whether work should be queued.                          |
| No magic-number admission                         | No 8 to 16 byte signature read exists before queuing inspect_source work.                                                      |
| Decode/analysis not directly coupled to traversal | Inspection work is queued as work items; full byte-reading/decode adapter was not found.                                       |
| Broad-root unsafe                                 | C:\ can cause unbounded traversal, source_files pollution, source_directories growth, and poor cancellation/progress behavior. |
| Root identity is path-shaped in production        | Durable identity schema slots exist, but production registration uses absolute_path/degraded_enrollment style identity.        |
| Windows reparse handling is incomplete            | Junctions/mount points are not detected by the existing is_symlink checks.                                                     |

Important nuance: fs::metadata is not content byte-reading and does not decode audio. It is still an expensive
filesystem operation at whole-drive scale, especially on Windows where antivirus, cloud placeholders, permission checks,
and reparse points can amplify cost.

## Evidence source map

| Evidence area                               | Source report                        | Use                                                                       |
|---------------------------------------------|--------------------------------------|---------------------------------------------------------------------------|
| Current scan pipeline and C:\ failure chain | DeepSeek final reconnaissance report | High-level map and current behavior summary.                              |
| Windows reparse/junction behavior           | Sub-agent report                     | Exact Windows-specific filesystem gap.                                    |
| Work executor / inspection adapter          | Sub-agent report                     | Confirms infrastructure exists but audio adapter/work loop does not.      |
| Database pollution / persistence path       | Sub-agent report                     | Confirms every file is inserted/updated before probe gating.              |
| Source identity                             | Sub-agent report                     | Confirms path-only production identity despite forward-compatible schema. |

## Current pipeline evidence

Observed current flow, summarized from the final reconnaissance report:

1. User selects a local root in the desktop app.
2. The app registers the root through the local-root boundary.
3. The Rust service stores the root using path-shaped identity.
4. The app triggers a root scan.
5. The store probes source access.
6. The filesystem walker traverses entries under the root.
7. For each entry, filesystem metadata is collected.
8. Directories are persisted into source directory state.
9. Files are persisted into source file state.
10. Extension classification decides whether inspect_source work is queued.
11. Inspection work is deferred to work items.
12. Projection refresh/reseed follows discovery.

The good part: traversal is not currently doing full audio decode or analysis.

The failing part: candidate admission happens too late to protect the durable substrate from broad-root noise.

## C:\ failure path evidence

When C:\ is selected, the current code treats it as an ordinary source root.

Expected failure chain:

1. C:\ registers successfully because it exists and is a directory.
2. The scan starts a recursive traversal over the system volume.
3. No source-root classification marks it as a system_volume_root.
4. No broad-root scan plan applies system, application, cache, dependency, protected-folder, or reparse exclusions.
5. The walker visits huge irrelevant subtrees.
6. Non-media files are persisted into source_files before candidate admission meaningfully narrows the set.
7. Directories are persisted into source_directories.
8. Extension-matched files can queue inspect_source work without magic-number verification.
9. Permission errors may be recorded per entry, but they do not solve scale, persistence pollution, or reparse behavior.
10. Cancellation and progress are too coarse for whole-drive behavior.

This is not merely slow scanning. It is a substrate-shape bug: operating-system debris becomes durable library state.

## Sub-agent evidence: Windows reparse and junction behavior

### Finding

The current code cannot detect Windows junctions correctly.

The sub-agent identified two relevant gates that rely on file_type().is_symlink:

| Location                                                                 | Reported behavior with Windows junctions                                                                      |
|--------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------|
| filesystem_walk.rs:121, symlink escape check in walk iterator            | is_symlink returns false for junctions. The entry falls through to is_dir and can be descended into normally. |
| source_access.rs:85, root-level symlink detection in probe_source_access | is_symlink returns false for junctions. The root is treated as a normal directory.                            |

The report states that Rust on Windows distinguishes IO_REPARSE_TAG_SYMLINK from IO_REPARSE_TAG_MOUNT_POINT, so a
junction/mount point is not caught by a symlink-only check.

### Additional evidence

| Area                             | Finding                                                                                                                     |
|----------------------------------|-----------------------------------------------------------------------------------------------------------------------------|
| Walkdir config                   | FOLLOW_FILESYSTEM_SYMLINKS is false. This is not enough if custom reparse handling misses junctions.                        |
| read_link behavior               | Windows read_link can resolve junctions to final targets, but target-inside-root checks do not prevent duplicate inventory. |
| Primary risk                     | Duplicate inventory and loop/double-traversal risk, not only root escape.                                                   |
| Cloud placeholders               | No code was found for FILE_ATTRIBUTE_RECALL_ON_DATA, placeholders, or OneDrive handling.                                    |
| Windows-specific filesystem code | No cfg(windows) reparse taxonomy code was found for scanning.                                                               |

### Implementation implication

The first implementation may conservatively skip or record reparse points, but the model must distinguish file symlink,
directory symlink, junction, mount point, cloud placeholder, and unknown reparse point. The canonical contract owns that
taxonomy.

## Sub-agent evidence: work executor and inspection adapter

### Finding

No audio inspection adapter or work executor loop was found in the codebase.

What exists:

| Existing surface         | Reported status                                                                                   |
|--------------------------|---------------------------------------------------------------------------------------------------|
| WorkItems table          | Full lease/claim/complete/fail lifecycle exists.                                                  |
| WorkRuns table           | adapter_key and adapter_version columns exist.                                                    |
| claim_machine_work_batch | Exists and works, but nothing calls it in a loop.                                                 |
| InspectSourcePromotionTx | Commits already-computed source facts and rebuilds projections. It does not inspect bytes itself. |
| Visual media inspection  | Exists for PNG/JPEG/GIF/MP4/MOV/M4V/WebM dimensions/content hashing.                              |

What does not exist:

| Missing surface                  | Reported status                                                                            |
|----------------------------------|--------------------------------------------------------------------------------------------|
| Audio dependency                 | No symphonia, rodio, cpal references found.                                                |
| Work-loop binary                 | No binary that continuously claims work items was found.                                   |
| Audio byte inspection            | No code found that reads audio bytes, parses ID3/FLAC/Vorbis tags, or decode-probes audio. |
| TypeScript work item integration | No TypeScript code touches work items or inspection.                                       |

### Implementation implication

The separation is directionally correct: store/discovery queues work, adapter performs byte-reading later. But C:\
safety still needs a cheap magic-number gate before inspect_source work is queued, otherwise extension-matched non-audio
files can enter future inspection work once the adapter exists.

Do not invent a duplicate audio adapter during broad-root safety work. Add the queue-boundary admission guard and leave
deeper audio inspection as its own adapter work.

## Sub-agent evidence: database pollution and persistence path

### Finding

Every file under the scan root is persisted to source_files unconditionally before inspection gating.

Reported call chain:

```text
FilesystemWalkEntries::next()
  -> yields kernel32.dll as DiscoveredLocationKind::File
  -> execute_root_scan_materialization_with_observer() pushes to pending_chunk
  -> commit_discovery_chunk -> commit_chunk()
    -> process_discovered_file()
      -> record_source_file_observation()
        -> media_class = "none" for .dll
      -> file_needs_inspection()
        -> returns false for .dll
        -> no inspect_source work queued
```

The report identified process_discovered_file as the central location:

| Operation                      | Reported order                                                        |
|--------------------------------|-----------------------------------------------------------------------|
| record_source_file_observation | Called unconditionally before the inspection gate.                    |
| file_needs_inspection          | Called after source file observation.                                 |
| queue_inspect_source_work      | Gated by file_needs_inspection, but too late to protect source_files. |

The gate exists, but it gates work queueing, not durable file persistence.

### Directory persistence

The sub-agent also reports that folders are persisted unconditionally through source directory chain/upsert behavior.
This is not automatically wrong: source hierarchy is first-slice authority. The problem is treating every irrelevant
file as durable source file state by default.

### Missing-file finalization

The report says finalize_scan treats previously persisted non-audio files like other present files. If a .dll is
recorded during scan 1 and absent during scan 2, it can be marked missing. That is coherent change detection for
whatever was persisted, but wrong admission for broad roots: the .dll should not have become per-file library substrate
state in the first place.

### Implementation implication

Move or add candidate admission before per-file source_files persistence for broad roots. Preserve source hierarchy and
aggregate ignored-file counts, but do not write every non-candidate file as durable source_files state by default.

## Sub-agent evidence: source identity

### Finding

Source roots are identified by path only in production, while durable identity schema slots already exist.

Reported production registration behavior:

| Field                                 | Production behavior                               |
|---------------------------------------|---------------------------------------------------|
| sources.identity_key                  | degraded_enrollment plus absolute/canonical path. |
| source_locators.locator_kind          | absolute_path.                                    |
| source_locators.device_identity_kind  | NULL.                                             |
| source_locators.device_identity_value | NULL.                                             |
| source_locators.relative_suffix       | Empty.                                            |

Reported forward-compatible schema/design already present:

| Existing shape               | Meaning                                                                           |
|------------------------------|-----------------------------------------------------------------------------------|
| source_locators variants     | absolute_path and removable_volume variants exist.                                |
| removable_volume constraints | Requires device_identity_kind, device_identity_value, and relative_suffix.        |
| RootIdentityKind enum        | Includes WindowsVolumeGuid, FilesystemUuid, MacVolumeUuid, DeviceSerialPartition. |
| Unique indexes               | Exist for absolute path and durable identity tuple.                               |

Reported missing implementation:

1. Platform-specific volume identity resolution at root registration time.
2. Population of device_identity_kind and device_identity_value from platform APIs.
3. Switching locator kind to removable_volume when durable identity is available.
4. Deduplicating identity by durable volume identity rather than path.

### Implementation implication

Durable root identity is primarily a data-population gap, not a schema redesign. The broad-root safety slice may keep
path identity provisional, but it must not harden path strings as permanent source identity.

## Corrections and cautions before implementation

The reconnaissance reports are valuable evidence, but implementation should obey the canonical contract over raw
suggestions.

| Raw suggestion or wording                           | Correction                                                                                                |
|-----------------------------------------------------|-----------------------------------------------------------------------------------------------------------|
| Treat fs::metadata as a byte read                   | fs::metadata is filesystem metadata, not content byte-reading. It remains expensive at whole-drive scale. |
| Use hard max depth/file-count limits as main safety | Use work budgets and candidate policy. Hard guards may exist only as pathological safety brakes.          |
| Skip all reparse points forever                     | A conservative first slice may skip, but the model must distinguish reparse kinds.                        |
| Persist only candidates and ignore hierarchy        | Preserve source hierarchy and coverage observations; avoid per-file rows for irrelevant files by default. |
| Implement audio adapter as part of scan safety      | Keep adapter separate. Add cheap magic gate before queued inspection work.                                |

## Implementation implications for the next coding pass

The next coding pass should be narrow and safety-first.

Target changes:

1. Add source-root classification for obvious Windows system volume roots and broad drive roots.
2. Add a broad-root scan plan with default exclusions for system, application, cache, dependency, build, VCS, and
   protected folders.
3. Move/add candidate admission before per-file source_files persistence for broad roots.
4. Store aggregate ignored-file counts for irrelevant files instead of per-file durable rows by default.
5. Add magic-number admission before inspect_source work is queued.
6. Add Windows reparse detection sufficient to avoid blind junction/mount traversal.
7. Preserve existing per-entry inaccessible/blocked handling.
8. Keep path identity provisional but use the existing durable identity schema slots when implementing relocation
   support.
9. Add tests proving C:\-like roots do not pollute source_files or queue arbitrary binaries for inspection.

## Minimal implementation prompt payload

Use the following as the distilled implementation direction for a coding agent. Do not land this prompt as architecture
doctrine.

```text
Make source root scanning safe for broad Windows roots.

Mode:
You are working locally in the Dekzer repo on Windows. Do not commit, push, create a PR, rewrite history, or create tags. Leave changes staged or unstaged. Report exact files changed, validation run, current git status, risks, and suggested commit message.

Goal:
Prevent broad roots such as C:\ from causing unbounded traversal, source_files pollution, arbitrary future inspection work, or UI/job mystery state. This is not a full ingestion rewrite.

Read first:
- docs/source-root-scan-admission-contract.md
- current source registration code
- current filesystem walk/discovery code
- current source file persistence code
- current work item queueing code
- current source identity/locator code

Known evidence:
- Windows junctions are missed by is_symlink-only checks.
- No audio inspection adapter/work executor loop currently exists.
- record_source_file_observation is called before file_needs_inspection.
- Production source identity is path-only, but durable identity schema slots exist.

Scope:
1. Classify obvious Windows system/broad roots before scanning.
2. Apply safe broad-root exclusions before traversal/persistence work becomes expensive.
3. Ensure non-candidate files under broad roots do not create per-file source_files records by default.
4. Add cheap magic-number admission before inspect_source work is queued.
5. Avoid following Windows reparse points blindly.
6. Add focused tests for C:\-like roots, huge binary folders, renamed .mp3 executable, permission-denied folders, and reparse/junction behavior.

Non-negotiable laws:
- Do not treat C:\ as an ordinary music folder.
- Do not store every ignored system file.
- Do not queue inspection for extension-matched files without a cheap signature gate.
- Do not collapse rejected candidate, unsupported media, and damaged media.
- Do not create a parallel scanner until existing owners are proven unusable.
- Reuse existing helpers and schema slots where available.

Stop rules:
- Stop if a safe fix requires a larger schema migration than expected; report the migration shape.
- Stop if Windows durable identity requires OS wrappers not yet present; keep identity provisional without blocking later durable identity.
- Stop if the missing audio adapter becomes necessary; keep this pass at the queue/admission boundary and report remaining adapter risk.
```

## Retention decision

Keep this file as archived evidence only.

Do not keep separate standing documents for:

- source_root_scan_reconnaissance_revised.md
- individual sub-agent reports
- overlapping implementation-plan variants

If detailed raw transcripts are needed, keep them outside active docs or attach them to the work item/issue. This file
is the single evidence summary. The canonical contract remains the single architecture source.

## Final sanity check

| Question                                                                     | Answer                   |
|------------------------------------------------------------------------------|--------------------------|
| Does this file define architecture?                                          | No. It records evidence. |
| Does this file supersede the admission contract?                             | No.                      |
| Does this file keep the revised reconnaissance plan alive as a parallel doc? | No.                      |
| Does this file preserve the important repo findings?                         | Yes.                     |
| Does this file give the coding agent enough concrete deltas?                 | Yes.                     |
| Is the current broad-root bug now traceable to code-level evidence?          | Yes.                     |
