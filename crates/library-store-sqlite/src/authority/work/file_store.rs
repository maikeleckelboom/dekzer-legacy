// Artifact file-store materialization and reconciliation are retained as
// substrate behind promotion/artifact APIs, while the maintained boundary center
// does not expose reconciliation commands.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};

use crate::authority::write_lane::AdmittedWrite;
use crate::source_media::AppOwnedStorageKind;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::ArtifactStorageKind;

const ARTIFACT_FILE_STORE_DIRECTORY_SUFFIX: &str = ".artifact-file-store";
const ARTIFACT_FILE_STORE_FILE_EXTENSION: &str = "artifact";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFileStoreRoot {
    kind: AppOwnedStorageKind,
    path: PathBuf,
}

impl ArtifactFileStoreRoot {
    pub fn from_durable_store_path(store_path: impl AsRef<Path>) -> Self {
        let store_path = store_path.as_ref();
        let store_parent = store_path.parent().unwrap_or_else(|| Path::new("."));
        let store_file_name = store_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "library.sqlite3".to_string());
        let directory_name = format!("{store_file_name}{ARTIFACT_FILE_STORE_DIRECTORY_SUFFIX}");

        Self {
            kind: AppOwnedStorageKind::LibraryStoreSqliteState,
            path: store_parent.join(directory_name),
        }
    }

    pub fn for_store_path(store_path: impl AsRef<Path>) -> Self {
        Self::from_durable_store_path(store_path)
    }

    pub const fn kind(&self) -> AppOwnedStorageKind {
        self.kind
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn resolve_relative_path(
        &self,
        relative_path: &ArtifactFileStoreRelativePath,
    ) -> PathBuf {
        self.path.join(relative_path.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactFileStoreRelativePath(String);

impl ArtifactFileStoreRelativePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn for_artifact(
        artifact_id: i64,
        created_at_ms: i64,
    ) -> ArtifactFileStoreRelativePath {
        Self(format!(
            "artifact-{artifact_id}-{created_at_ms}.{ARTIFACT_FILE_STORE_FILE_EXTENSION}"
        ))
    }

    pub(crate) fn from_db_value(
        value: String,
        context: impl Into<String>,
    ) -> LibrarySqliteResult<Self> {
        let context = context.into();
        let mut components = Path::new(&value).components();
        let Some(Component::Normal(_)) = components.next() else {
            return Err(LibrarySqliteError::MalformedSchemaState(format!(
                "{} has malformed artifact file-store relative path: {value}",
                context.as_str()
            )));
        };
        if components.next().is_some() {
            return Err(LibrarySqliteError::MalformedSchemaState(format!(
                "{} has malformed artifact file-store relative path: {value}",
                context.as_str()
            )));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFileStorePath {
    pub root_kind: AppOwnedStorageKind,
    pub relative_path: ArtifactFileStoreRelativePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArtifactFileStoreReconciliationResult {
    pub orphan_files_deleted: Vec<ArtifactFileStoreRelativePath>,
    pub integrity_findings: Vec<ArtifactFileStoreIntegrityFinding>,
    pub limit_reached: bool,
}

impl ArtifactFileStoreReconciliationResult {
    pub fn total_count(&self) -> usize {
        self.orphan_files_deleted.len() + self.integrity_findings.len()
    }

    pub fn health(&self) -> ArtifactFileStoreHealth {
        if !self.integrity_findings.is_empty() {
            ArtifactFileStoreHealth::Degraded
        } else if !self.orphan_files_deleted.is_empty() {
            ArtifactFileStoreHealth::Healed
        } else {
            ArtifactFileStoreHealth::Healthy
        }
    }

    pub fn missing_owned_file_count(&self) -> usize {
        self.integrity_findings
            .iter()
            .filter(|finding| {
                matches!(
                    finding,
                    ArtifactFileStoreIntegrityFinding::MissingOwnedFile { .. }
                )
            })
            .count()
    }

    pub fn missing_referenced_artifact_count(&self) -> usize {
        self.integrity_findings
            .iter()
            .filter(|finding| matches!(
                finding,
                ArtifactFileStoreIntegrityFinding::MissingReferencedArtifactFile { .. }
                    | ArtifactFileStoreIntegrityFinding::MissingReferencedClaimedArtifactFile { .. }
            ))
            .count()
    }

    pub fn missing_claimed_artifact_count(&self) -> usize {
        self.integrity_findings
            .iter()
            .filter(|finding| matches!(
                finding,
                ArtifactFileStoreIntegrityFinding::MissingClaimedArtifactFile { .. }
                    | ArtifactFileStoreIntegrityFinding::MissingReferencedClaimedArtifactFile { .. }
            ))
            .count()
    }

    pub fn missing_referenced_claimed_artifact_count(&self) -> usize {
        self.integrity_findings
            .iter()
            .filter(|finding| {
                matches!(
                    finding,
                    ArtifactFileStoreIntegrityFinding::MissingReferencedClaimedArtifactFile { .. }
                )
            })
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactFileStoreHealth {
    Healthy,
    Healed,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
// Findings intentionally read as complete diagnostics at API/test call sites.
#[allow(clippy::enum_variant_names)]
pub enum ArtifactFileStoreIntegrityFinding {
    MissingOwnedFile {
        artifact_id: i64,
        file_store_path: ArtifactFileStorePath,
    },
    MissingReferencedArtifactFile {
        artifact_id: i64,
        file_store_path: ArtifactFileStorePath,
    },
    MissingClaimedArtifactFile {
        artifact_id: i64,
        file_store_path: ArtifactFileStorePath,
    },
    MissingReferencedClaimedArtifactFile {
        artifact_id: i64,
        file_store_path: ArtifactFileStorePath,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactFileStoreOwnedFileRow {
    pub artifact_id: i64,
    pub file_store_path: ArtifactFileStorePath,
    pub has_live_reference: bool,
    pub has_active_claim: bool,
}

impl ArtifactFileStoreOwnedFileRow {
    fn into_integrity_finding(self) -> ArtifactFileStoreIntegrityFinding {
        match (self.has_live_reference, self.has_active_claim) {
            (true, true) => {
                ArtifactFileStoreIntegrityFinding::MissingReferencedClaimedArtifactFile {
                    artifact_id: self.artifact_id,
                    file_store_path: self.file_store_path,
                }
            }
            (true, false) => ArtifactFileStoreIntegrityFinding::MissingReferencedArtifactFile {
                artifact_id: self.artifact_id,
                file_store_path: self.file_store_path,
            },
            (false, true) => ArtifactFileStoreIntegrityFinding::MissingClaimedArtifactFile {
                artifact_id: self.artifact_id,
                file_store_path: self.file_store_path,
            },
            (false, false) => ArtifactFileStoreIntegrityFinding::MissingOwnedFile {
                artifact_id: self.artifact_id,
                file_store_path: self.file_store_path,
            },
        }
    }
}

pub(crate) fn persist_artifact_file_payload(
    write: &mut AdmittedWrite<'_>,
    file_store_root: &ArtifactFileStoreRoot,
    artifact_id: i64,
    created_at_ms: i64,
    payload_bytes: &[u8],
) -> LibrarySqliteResult<ArtifactFileStorePath> {
    let relative_path = ArtifactFileStoreRelativePath::for_artifact(artifact_id, created_at_ms);
    let file_store_path = ArtifactFileStorePath {
        root_kind: file_store_root.kind(),
        relative_path,
    };
    let absolute_path = file_store_root.resolve_relative_path(&file_store_path.relative_path);
    write_payload_to_artifact_file_store(&absolute_path, payload_bytes)?;
    let rollback_path = absolute_path.clone();
    write.register_rollback_cleanup(move || {
        let _ = remove_artifact_file_store_file_if_present(&rollback_path);
    });
    Ok(file_store_path)
}

pub(crate) fn retire_artifact_if_unreferenced_and_unclaimed(
    write: &mut AdmittedWrite<'_>,
    file_store_root: &ArtifactFileStoreRoot,
    artifact_id: i64,
) -> LibrarySqliteResult<bool> {
    if artifact_has_live_reference(write, artifact_id)?
        || artifact_has_active_claim(write, artifact_id)?
    {
        return Ok(false);
    }

    let Some(storage_kind) = load_artifact_storage_kind(write, artifact_id)? else {
        return Ok(false);
    };
    if storage_kind == ArtifactStorageKind::FileStore {
        let file_store_path =
            load_artifact_file_store_path(write, artifact_id)?.ok_or_else(|| {
                LibrarySqliteError::MalformedSchemaState(format!(
                    "file-backed artifact {artifact_id} is missing from ArtifactFileStoreEntries"
                ))
            })?;
        if file_store_path.root_kind != file_store_root.kind() {
            return Err(LibrarySqliteError::MalformedSchemaState(format!(
                "artifact {artifact_id} uses unsupported artifact file-store root kind {}",
                file_store_path.root_kind.as_str()
            )));
        }
        let absolute_path = file_store_root.resolve_relative_path(&file_store_path.relative_path);
        write.register_post_commit(move || {
            let _ = remove_artifact_file_store_file_if_present(&absolute_path);
        });
    }

    write.execute(
        "DELETE FROM Artifacts WHERE artifact_id = ?1",
        [artifact_id],
    )?;
    Ok(true)
}

pub(crate) fn reconcile_artifact_file_store(
    connection: &Connection,
    file_store_root: &ArtifactFileStoreRoot,
    limit: usize,
) -> LibrarySqliteResult<ArtifactFileStoreReconciliationResult> {
    if limit == 0 {
        return Ok(ArtifactFileStoreReconciliationResult::default());
    }

    let owned_files = load_owned_file_store_rows(connection)?;
    let mut result = ArtifactFileStoreReconciliationResult::default();
    let mut owned_relative_paths = BTreeSet::new();

    for owned_file in owned_files {
        if owned_file.file_store_path.root_kind != file_store_root.kind() {
            return Err(LibrarySqliteError::MalformedSchemaState(format!(
                "artifact {} uses unsupported artifact file-store root kind {}",
                owned_file.artifact_id,
                owned_file.file_store_path.root_kind.as_str()
            )));
        }

        owned_relative_paths.insert(
            owned_file
                .file_store_path
                .relative_path
                .as_str()
                .to_string(),
        );
        if result.total_count() >= limit {
            result.limit_reached = true;
            continue;
        }

        let absolute_path =
            file_store_root.resolve_relative_path(&owned_file.file_store_path.relative_path);
        match fs::metadata(&absolute_path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => {
                return Err(LibrarySqliteError::MalformedSchemaState(format!(
                    "artifact {} artifact file-store path {:?} must resolve to a file",
                    owned_file.artifact_id, absolute_path
                )));
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                result
                    .integrity_findings
                    .push(owned_file.into_integrity_finding());
            }
            Err(source) => {
                return Err(LibrarySqliteError::ArtifactFileStoreIo {
                    action: "read file metadata",
                    path: absolute_path,
                    source,
                });
            }
        }
    }

    if result.total_count() >= limit {
        result.limit_reached = true;
        return Ok(result);
    }

    let root_path = file_store_root.path();
    let mut root_entries = match fs::read_dir(root_path) {
        Ok(entries) => entries
            .map(|entry| {
                let entry = entry.map_err(|source| LibrarySqliteError::ArtifactFileStoreIo {
                    action: "list file-store root",
                    path: root_path.to_path_buf(),
                    source,
                })?;
                Ok(entry.file_name())
            })
            .collect::<LibrarySqliteResult<Vec<_>>>()?,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(result),
        Err(source) => {
            return Err(LibrarySqliteError::ArtifactFileStoreIo {
                action: "list file-store root",
                path: root_path.to_path_buf(),
                source,
            });
        }
    };
    root_entries.sort();

    for entry_name in root_entries {
        if result.total_count() >= limit {
            result.limit_reached = true;
            break;
        }
        let Some(entry_name) = entry_name.to_str() else {
            return Err(LibrarySqliteError::MalformedSchemaState(format!(
                "artifact file-store root {:?} contains a non-utf8 entry name",
                root_path
            )));
        };
        let relative_path = ArtifactFileStoreRelativePath::from_db_value(
            entry_name.to_string(),
            format!("artifact file-store root {:?}", root_path),
        )?;
        if owned_relative_paths.contains(relative_path.as_str()) {
            continue;
        }

        let absolute_path = file_store_root.resolve_relative_path(&relative_path);
        let metadata = match fs::symlink_metadata(&absolute_path) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(LibrarySqliteError::ArtifactFileStoreIo {
                    action: "read orphan file metadata",
                    path: absolute_path.clone(),
                    source,
                });
            }
        };
        if metadata.is_dir() {
            return Err(LibrarySqliteError::MalformedSchemaState(format!(
                "artifact file-store root {:?} contains unexpected directory entry {:?}",
                root_path, absolute_path
            )));
        }
        if remove_artifact_file_store_file_if_present(&absolute_path)? {
            result.orphan_files_deleted.push(relative_path);
        }
    }

    Ok(result)
}

pub(crate) fn write_payload_to_artifact_file_store(
    path: &Path,
    payload_bytes: &[u8],
) -> LibrarySqliteResult<()> {
    let parent = path.parent().unwrap_or(path);
    ensure_artifact_file_store_directory(parent)?;

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "artifact".to_string());
    let temp_path = path.with_file_name(format!("{file_name}.staging"));
    remove_artifact_file_store_file_if_present(&temp_path)?;

    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|source| LibrarySqliteError::ArtifactFileStoreIo {
            action: "open staging file",
            path: temp_path.clone(),
            source,
        })?;
    if let Err(source) = temp_file
        .write_all(payload_bytes)
        .and_then(|_| temp_file.sync_all())
    {
        let _ = remove_artifact_file_store_file_if_present(&temp_path);
        return Err(LibrarySqliteError::ArtifactFileStoreIo {
            action: "write staging file",
            path: temp_path,
            source,
        });
    }
    drop(temp_file);

    fs::rename(&temp_path, path).map_err(|source| {
        let _ = remove_artifact_file_store_file_if_present(&temp_path);
        LibrarySqliteError::ArtifactFileStoreIo {
            action: "persist file",
            path: path.to_path_buf(),
            source,
        }
    })?;
    if let Err(error) = sync_artifact_file_store_directory(parent, "sync directory after rename") {
        let _ = remove_artifact_file_store_file_if_present(path);
        return Err(error);
    }

    Ok(())
}

pub(crate) fn remove_artifact_file_store_file_if_present(path: &Path) -> LibrarySqliteResult<bool> {
    match fs::remove_file(path) {
        Ok(()) => {
            let parent = path.parent().unwrap_or(path);
            sync_artifact_file_store_directory(parent, "sync directory after delete")?;
            Ok(true)
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(LibrarySqliteError::ArtifactFileStoreIo {
            action: "delete file",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn ensure_artifact_file_store_directory(directory_path: &Path) -> LibrarySqliteResult<()> {
    let existed_before = directory_path.exists();
    fs::create_dir_all(directory_path).map_err(|source| {
        LibrarySqliteError::ArtifactFileStoreIo {
            action: "create directory",
            path: directory_path.to_path_buf(),
            source,
        }
    })?;
    if !existed_before {
        let parent_directory = directory_path.parent().unwrap_or_else(|| Path::new("."));
        sync_artifact_file_store_directory(
            parent_directory,
            "sync file-store root parent after create",
        )?;
    }
    Ok(())
}

fn sync_artifact_file_store_directory(
    directory_path: &Path,
    action: &'static str,
) -> LibrarySqliteResult<()> {
    match open_directory_for_sync(directory_path).and_then(|directory| directory.sync_all()) {
        Ok(()) => Ok(()),
        Err(source) if directory_sync_is_not_supported_on_host(&source) => Ok(()),
        Err(source) => Err(LibrarySqliteError::ArtifactFileStoreIo {
            action,
            path: directory_path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(unix)]
fn open_directory_for_sync(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

#[cfg(windows)]
fn open_directory_for_sync(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_directory_for_sync(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

#[cfg(windows)]
fn directory_sync_is_not_supported_on_host(source: &std::io::Error) -> bool {
    matches!(
        source.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::InvalidInput
    )
}

#[cfg(not(windows))]
fn directory_sync_is_not_supported_on_host(_source: &std::io::Error) -> bool {
    false
}

fn load_artifact_storage_kind(
    connection: &Connection,
    artifact_id: i64,
) -> LibrarySqliteResult<Option<ArtifactStorageKind>> {
    let persisted_storage_kind = connection
        .query_row(
            "SELECT storage_kind
             FROM Artifacts
             WHERE artifact_id = ?1",
            [artifact_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    persisted_storage_kind
        .map(|storage_kind| {
            ArtifactStorageKind::parse(&storage_kind).ok_or_else(|| {
                LibrarySqliteError::MalformedSchemaState(format!(
                    "artifact {artifact_id} has malformed storage_kind: {storage_kind}"
                ))
            })
        })
        .transpose()
}

fn load_artifact_file_store_path(
    connection: &Connection,
    artifact_id: i64,
) -> LibrarySqliteResult<Option<ArtifactFileStorePath>> {
    let persisted_entry = connection
        .query_row(
            "SELECT root_kind, relative_path
             FROM ArtifactFileStoreEntries
             WHERE artifact_id = ?1",
            [artifact_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    persisted_entry
        .map(|(persisted_root_kind, persisted_relative_path)| {
            let Some(root_kind) = AppOwnedStorageKind::from_db_value(&persisted_root_kind) else {
                return Err(LibrarySqliteError::MalformedSchemaState(format!(
                    "artifact {artifact_id} has malformed artifact file-store root kind: {persisted_root_kind}"
                )));
            };
            Ok(ArtifactFileStorePath {
                root_kind,
                relative_path: ArtifactFileStoreRelativePath::from_db_value(
                    persisted_relative_path,
                    format!("artifact {artifact_id}"),
                )?,
            })
        })
        .transpose()
}

fn artifact_has_active_claim(
    connection: &Connection,
    artifact_id: i64,
) -> LibrarySqliteResult<bool> {
    let active_claim_count: i64 = connection.query_row(
        "SELECT COUNT(*)
         FROM ArtifactClaims
         WHERE artifact_id = ?1
           AND released_at IS NULL",
        [artifact_id],
        |row| row.get(0),
    )?;
    Ok(active_claim_count > 0)
}

fn artifact_has_live_reference(
    connection: &Connection,
    artifact_id: i64,
) -> LibrarySqliteResult<bool> {
    let has_live_reference: i64 = connection.query_row(
        "SELECT CASE
                    WHEN EXISTS(
                        SELECT 1
                        FROM SourceFacts
                        WHERE accepted_artifact_id = ?1
                    ) THEN 1
                    ELSE 0
                END",
        [artifact_id],
        |row| row.get(0),
    )?;
    Ok(has_live_reference != 0)
}

fn load_owned_file_store_rows(
    connection: &Connection,
) -> LibrarySqliteResult<Vec<ArtifactFileStoreOwnedFileRow>> {
    let mut statement = connection.prepare(
        "SELECT entry.artifact_id,
                entry.root_kind,
                entry.relative_path,
                CASE
                    WHEN EXISTS(
                        SELECT 1
                        FROM SourceFacts
                        WHERE accepted_artifact_id = entry.artifact_id
                    ) THEN 1
                    ELSE 0
                END AS has_live_reference,
                EXISTS(
                    SELECT 1
                    FROM ArtifactClaims claim
                    WHERE claim.artifact_id = entry.artifact_id
                      AND claim.released_at IS NULL
                ) AS has_active_claim
         FROM ArtifactFileStoreEntries entry
         ORDER BY entry.artifact_id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    rows
        .map(|row| {
            let (
                artifact_id,
                persisted_root_kind,
                persisted_relative_path,
                has_live_reference,
                has_active_claim,
            ) = row?;
            let Some(root_kind) = AppOwnedStorageKind::from_db_value(&persisted_root_kind) else {
                return Err(LibrarySqliteError::MalformedSchemaState(format!(
                    "artifact {artifact_id} has malformed artifact file-store root kind: {persisted_root_kind}"
                )));
            };
            Ok(ArtifactFileStoreOwnedFileRow {
                artifact_id,
                file_store_path: ArtifactFileStorePath {
                    root_kind,
                    relative_path: ArtifactFileStoreRelativePath::from_db_value(
                        persisted_relative_path,
                        format!("artifact {artifact_id}"),
                    )?,
                },
                has_live_reference: has_live_reference != 0,
                has_active_claim: has_active_claim != 0,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};
    use tempfile::TempDir;

    use crate::authority::work::artifacts::{
        ArtifactsAuthorityTx, PersistArtifactPayloadInput, PersistedArtifactPayload,
        PersistedArtifactStorage, RecordArtifactInput,
    };
    use crate::authority::write_lane::admit_write;
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{ArtifactKind, ArtifactRole, WorkRunId};

    use super::{
        ArtifactFileStoreHealth, ArtifactFileStoreIntegrityFinding, ArtifactFileStoreRoot,
        reconcile_artifact_file_store,
    };

    fn open_test_connection() -> (TempDir, std::path::PathBuf, Connection) {
        let tempdir = TempDir::new().expect("create tempdir");
        let db_path = tempdir.path().join("artifact-file-store.sqlite3");
        let mut connection = Connection::open(&db_path).expect("open test connection");
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")
            .expect("configure test connection");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");
        (tempdir, db_path, connection)
    }

    fn insert_open_inspection_run(
        write: &Connection,
        basis_fingerprint: &str,
        started_at: i64,
    ) -> WorkRunId {
        write
            .execute(
                "INSERT INTO WorkItems (
                     subject_kind,
                     subject_id,
                     work_kind,
                     capability_kind,
                     target_profile_key,
                     target_quality,
                     priority_class,
                     basis_fingerprint,
                     state,
                     leased_until,
                     attempt_count,
                     blocked_reason,
                     failure_kind,
                     error_detail,
                     created_at,
                     updated_at
                 )
                 VALUES (
                     'source_file',
                     '300',
                     'inspect_source',
                     NULL,
                     NULL,
                     NULL,
                     'interactive',
                     ?1,
                     'leased',
                     ?2,
                     1,
                     NULL,
                     NULL,
                     NULL,
                     ?3,
                     ?3
                 )",
                params![basis_fingerprint, started_at + 10_000, started_at],
            )
            .expect("insert work item");
        let work_item_id = write.last_insert_rowid();
        write
            .execute(
                "INSERT INTO WorkRuns (
                     work_item_id,
                     adapter_key,
                     adapter_version,
                     started_at,
                     finished_at,
                     outcome,
                     failure_kind,
                     error_detail
                 )
                 VALUES (?1, 'test.adapter', '1.0.0', ?2, NULL, 'running', NULL, NULL)",
                params![work_item_id, started_at],
            )
            .expect("insert work run");
        WorkRunId::new(write.last_insert_rowid()).expect("inserted work run id")
    }

    fn persist_inspection_artifact(
        write: &mut crate::authority::write_lane::AdmittedWrite<'_>,
        file_store_root: &ArtifactFileStoreRoot,
        work_run_id: WorkRunId,
        basis_fingerprint: &str,
        payload_hash: &str,
        created_at: i64,
        payload: Vec<u8>,
    ) -> PersistedArtifactPayload {
        let mut artifacts = ArtifactsAuthorityTx::new(write);
        artifacts
            .persist_artifact_payload(
                file_store_root,
                &PersistArtifactPayloadInput {
                    artifact: RecordArtifactInput {
                        work_run_id,
                        artifact_kind: ArtifactKind::InspectionResult,
                        artifact_role: ArtifactRole::PrimaryResult,
                        media_type: "application/octet-stream".to_string(),
                        basis_fingerprint: basis_fingerprint.to_string(),
                        payload_hash: payload_hash.to_string(),
                        created_at,
                    },
                    payload,
                },
            )
            .expect("persist capability artifact")
    }

    #[test]
    fn generic_inline_persistence_uses_artifact_rows_and_inline_payloads() {
        let (_tempdir, db_path, mut connection) = open_test_connection();
        let file_store_root = ArtifactFileStoreRoot::for_store_path(&db_path);

        admit_write(&mut connection, |write| {
            let work_run_id = insert_open_inspection_run(write, "basis:v1", 10);
            let persisted = persist_inspection_artifact(
                write,
                &file_store_root,
                work_run_id,
                "basis:v1",
                "hash:inline:v1",
                11,
                b"{}".to_vec(),
            );

            assert_eq!(persisted.artifact.storage_kind.as_str(), "inline_payload");
            Ok(())
        })
        .expect("persist inline artifact");

        let storage_row: String = connection
            .query_row(
                "SELECT storage_kind
                 FROM Artifacts
                 ORDER BY artifact_id ASC
                 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("read artifact storage kind");
        let inline_payload_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM ArtifactInlinePayloads", [], |row| {
                row.get(0)
            })
            .expect("count inline payload rows");
        let file_store_entry_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM ArtifactFileStoreEntries", [], |row| {
                row.get(0)
            })
            .expect("count file-store rows");

        assert_eq!(storage_row, "inline_payload");
        assert_eq!(inline_payload_count, 1);
        assert_eq!(file_store_entry_count, 0);
        assert!(!file_store_root.path().exists());
    }

    #[test]
    fn generic_file_store_persistence_writes_bytes_and_generic_rows() {
        let (_tempdir, db_path, mut connection) = open_test_connection();
        let file_store_root = ArtifactFileStoreRoot::for_store_path(&db_path);
        let payload = vec![7_u8; 1_048_577];

        admit_write(&mut connection, |write| {
            let work_run_id = insert_open_inspection_run(write, "basis:v1", 10);
            let persisted = persist_inspection_artifact(
                write,
                &file_store_root,
                work_run_id,
                "basis:v1",
                "hash:file:v1",
                11,
                payload.clone(),
            );

            match persisted.storage {
                PersistedArtifactStorage::FileStore(path) => {
                    assert_eq!(path.root_kind.as_str(), "library_store_sqlite_state");
                }
                other => panic!("expected file-store payload, found {other:?}"),
            }
            Ok(())
        })
        .expect("persist file-store artifact");

        let (artifact_id, relative_path, payload_bytes): (i64, String, i64) = connection
            .query_row(
                "SELECT artifact_id, relative_path, payload_bytes
                 FROM ArtifactFileStoreEntries
                 ORDER BY artifact_id ASC
                 LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read file-store entry");
        let storage_row: String = connection
            .query_row(
                "SELECT storage_kind
                 FROM Artifacts
                 WHERE artifact_id = ?1",
                [artifact_id],
                |row| row.get(0),
            )
            .expect("read artifact storage kind");
        let absolute_path = file_store_root.path().join(&relative_path);

        assert_eq!(storage_row, "file_store");
        assert_eq!(payload_bytes, payload.len() as i64);
        assert_eq!(
            std::fs::read(&absolute_path).expect("read persisted artifact file"),
            payload
        );
    }

    #[test]
    fn reconciliation_deletes_orphan_files_without_a_cleanup_queue() {
        let (_tempdir, db_path, _connection) = open_test_connection();
        let file_store_root = ArtifactFileStoreRoot::for_store_path(&db_path);
        std::fs::create_dir_all(file_store_root.path()).expect("create artifact file-store root");
        let first_orphan = file_store_root.path().join("orphan-a.artifact");
        let second_orphan = file_store_root.path().join("orphan-b.artifact");
        std::fs::write(&first_orphan, b"a").expect("write first orphan");
        std::fs::write(&second_orphan, b"b").expect("write second orphan");
        let connection = Connection::open(&db_path).expect("reopen connection");
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")
            .expect("configure reopened connection");

        let first_reconciliation = reconcile_artifact_file_store(&connection, &file_store_root, 1)
            .expect("run first bounded reconciliation");
        assert_eq!(
            first_reconciliation.health(),
            ArtifactFileStoreHealth::Healed
        );
        assert!(first_reconciliation.limit_reached);
        assert_eq!(
            first_reconciliation
                .orphan_files_deleted
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>(),
            vec!["orphan-a.artifact"]
        );
        assert!(first_reconciliation.integrity_findings.is_empty());
        assert!(!first_orphan.exists());
        assert!(second_orphan.exists());

        let second_reconciliation = reconcile_artifact_file_store(&connection, &file_store_root, 1)
            .expect("run second bounded reconciliation");
        assert_eq!(
            second_reconciliation.health(),
            ArtifactFileStoreHealth::Healed
        );
        assert!(!second_reconciliation.limit_reached);
        assert_eq!(
            second_reconciliation
                .orphan_files_deleted
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>(),
            vec!["orphan-b.artifact"]
        );
        assert!(second_reconciliation.integrity_findings.is_empty());
        assert!(!second_orphan.exists());
    }

    #[test]
    fn reconciliation_reports_missing_claimed_file_using_generic_claims() {
        let (_tempdir, db_path, mut connection) = open_test_connection();
        let file_store_root = ArtifactFileStoreRoot::for_store_path(&db_path);
        let mut artifact_id = 0;

        admit_write(&mut connection, |write| {
            let work_run_id = insert_open_inspection_run(write, "basis:v1", 10);
            let persisted = persist_inspection_artifact(
                write,
                &file_store_root,
                work_run_id,
                "basis:v1",
                "hash:file:missing",
                11,
                vec![5_u8; 1_048_577],
            );
            artifact_id = persisted.artifact.artifact_id.get();
            write.execute(
                "INSERT INTO ArtifactClaims (
                     artifact_id,
                     claimant_kind,
                     claimant_key,
                     release_policy,
                     claimed_at,
                     released_at
                 )
                 VALUES (?1, 'source_export', ?2, 'manual', 12, NULL)",
                params![artifact_id, format!("claim:{artifact_id}")],
            )?;
            Ok(())
        })
        .expect("persist claimed file-store artifact");

        let relative_path: String = connection
            .query_row(
                "SELECT relative_path
                 FROM ArtifactFileStoreEntries
                 WHERE artifact_id = ?1",
                [artifact_id],
                |row| row.get(0),
            )
            .expect("read artifact file-store path");
        std::fs::remove_file(file_store_root.path().join(relative_path))
            .expect("remove referenced artifact file");

        let reconciliation = reconcile_artifact_file_store(&connection, &file_store_root, 10)
            .expect("reconcile missing referenced file");

        assert_eq!(reconciliation.health(), ArtifactFileStoreHealth::Degraded);
        assert_eq!(reconciliation.integrity_findings.len(), 1);
        match &reconciliation.integrity_findings[0] {
            ArtifactFileStoreIntegrityFinding::MissingClaimedArtifactFile {
                artifact_id: found_artifact_id,
                ..
            } => assert_eq!(*found_artifact_id, artifact_id),
            other => panic!("expected missing claimed file finding, got {other:?}"),
        }
    }
}
