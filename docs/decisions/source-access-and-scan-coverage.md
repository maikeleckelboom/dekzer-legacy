# Source Access and Scan Coverage

_Companion to `media-role-classification.md` and `work-scheduling.md`. Governs how Dekzer resolves source access,
classifies filesystem and OS access outcomes, records directory enumeration coverage, and prevents access failures from
becoming fake empty results._

---

## Core Law

```text
Access failure never becomes empty contents.
```

Empty means a selected scope was accessible, readable, scanned, and contained no rows relevant to the active projection.
Missing, unavailable, blocked, failed, partial, pending, and unknown are separate product states.

Dekzer starts source-rooted discovery with an access contract, not a raw path. The scanner commits observed facts and
enumeration outcomes. It must not infer empty, missing, blocked, or complete state from the absence of observed rows
alone.

---

## Ownership

The Source Access Authority owns:

- resolving a source locator to an effective root;
- using platform access grants where required;
- probing whether the source root is accessible;
- classifying OS and product-policy access outcomes into product issue kinds;
- opening a bounded access session for scanner use;
- recording source-level access state and directory enumeration outcomes.

It does not own media identity, role assignment, readiness, browser projection, renderer state, or UI copy.

---

## Access Boundary

```text
source locator
  -> optional access grant
  -> source access probe
  -> bounded access session
  -> directory enumeration outcome
  -> discovery commit
  -> scan coverage update
  -> projection/read result
```

A scan may commit only outcomes it has proven. Failure to enumerate a directory is not proof that the directory is
empty.

---

## Access Grants

Path strings are not durable access authority.

Dekzer keeps an explicit access-grant slot per source even when the current platform/build does not require a grant.

Minimum shape:

```sql
source_access_grants (
  source_id       INTEGER PRIMARY KEY REFERENCES sources(source_id) ON DELETE CASCADE,
  grant_kind      TEXT NOT NULL,
    -- none | macos_security_scoped_bookmark
  grant_payload   BLOB,
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL,
  CHECK (
    (grant_kind = 'none' AND grant_payload IS NULL)
    OR (grant_kind <> 'none' AND grant_payload IS NOT NULL)
  )
)
```

Non-sandboxed builds may store `grant_kind = 'none'`. The boundary still exists so platform grant support can be added
without redesigning source identity.

---

## Product Issue Kinds

Raw platform errors may be retained as diagnostics, but product behavior keys off product issue kinds.

```text
missing
not_directory
permission_denied
privacy_permission_required
unavailable_mount
resource_busy
stale_network_handle
symlink_loop
symlink_escape_blocked
unsupported_path
invalid_path
io_interrupted
timed_out
unknown_io
```

OS permission and Dekzer policy are distinct:

```text
OS denies read/list access        -> permission_denied
macOS privacy grant missing       -> privacy_permission_required
source root is not mounted        -> unavailable_mount
symlink escapes effective root    -> symlink_escape_blocked
file locked by another process    -> resource_busy
```

---

## Source Access Probe Result

```text
accessible
  effective_root
  observed_at_ms
  filesystem_type?
  access_epoch

blocked
  issue_kind
  platform_code?
  diagnostic_detail?
  checked_at_ms

missing
  issue_kind
  checked_at_ms
```

The probe result updates source access state. It does not directly classify media or project browser rows.

---

## Directory Enumeration Outcome

Directory enumeration commits outcomes, not just lists of discovered files.

```text
enumerated
  directory_relative_path
  entries
  mtime_ns?
  scanned_at_ms

blocked
  directory_relative_path
  issue_kind
  platform_code?
  diagnostic_detail?
  checked_at_ms

missing
  directory_relative_path
  checked_at_ms
```

Directory states map to scan coverage:

```text
enumerated -> present + complete coverage for that directory enumeration
blocked    -> present + blocked coverage with issue kind
missing    -> missing + complete proof that the directory is gone
failed     -> failed coverage with error details
```

A missing directory is not a scan failure. It is a successful observation that the directory no longer exists.

---

## Coverage and Projection Semantics

Selected contents and hierarchy reads must distinguish:

```text
complete
pending
scanning
blocked
failed
source_unavailable
location_missing
```

Rules:

```text
source root inaccessible       -> source_unavailable or blocked
source location missing        -> location_missing
some descendant blocked        -> blocked/partial coverage, not empty
scanner still active           -> scanning or pending coverage, not empty
readable and scanned, no rows   -> empty with complete coverage
```

`empty` is valid only when coverage proves that the selected scope was accessible and fully scanned for the active
projection.

Directory relevance facts must use precise names. Do not use a single broad media-descendant flag. Current and near-term
directory facts include:

```text
has_child_directories
has_primary_media_descendant
has_image_media_descendant
future role/readiness-derived rollups
```

---

## Symlink and Traversal Policy

MVP rule:

```text
Do not follow symlinks outside the effective source root.
```

If a symlink escapes the effective source root, classify the outcome as `symlink_escape_blocked`. That is a Dekzer
policy block, not an OS permission failure.

Symlinked directories inside the effective root are an explicit product-policy decision. Do not leave traversal behavior
accidental.

---

## Scheduler Relationship

Source-wide discovery is tracked as scan sessions, not ordinary per-file work items. Scan sessions are long-running,
cancelable, budgeted, and recoverable. Per-file sniff/probe/hash/classify/readiness/analysis work follows from committed
observations and is governed by `work-scheduling.md`.

Access and enumeration outcomes are inputs to scheduling. A blocked directory may create diagnostic or retry work. It
must not create fake empty projection state.

---

## Implementation Gates

Recursive selected contents and hierarchy projection may not claim complete product behavior until these exist:

- source access issue kinds;
- source access probe result mapping;
- directory enumeration outcomes;
- source access grant storage or an explicit non-sandboxed deferral;
- directory blocked/missing/failed state persistence;
- OS error to product issue mapping tests;
- selected contents cannot return `empty` for access failure;
- symlink escape policy is enforced;
- blocked and missing coverage can be surfaced in diagnostics.

---

## Acceptance Bar

The implementation is correct only when all of the following hold:

- permission failure never returns empty contents;
- an unmounted source returns source unavailable or blocked state, not empty rows;
- a blocked descendant makes recursive coverage blocked or partial, not complete;
- a readable, fully scanned empty scope is the only path to authoritative empty;
- missing directories are represented as missing, not failed scan errors;
- symlink escape is classified as product-policy blocked;
- raw OS errors are mapped to product issue kinds before crossing projection boundaries;
- renderer code does not infer access state from missing rows;
- scan finalization does not mark unobserved rows missing when access was blocked before enumeration could prove
  absence.
