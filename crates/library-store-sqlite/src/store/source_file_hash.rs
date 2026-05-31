use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use thiserror::Error;

use crate::authority::promotion::{InspectSourcePromotionInput, InspectSourcePromotionTx};
use crate::authority::sources::{CommitAcceptedSourceFactsInput, ContentHashEvidence};
use crate::authority::work::{
    ArtifactsAuthorityTx, ClaimSpecificMachineWorkInput, CompleteMachineWorkInput,
    FinishWorkRunInput, QueueInspectSourceWorkInput, RecordArtifactInput,
    RecordInlineArtifactInput, StartWorkRunInput, WorkItemsAuthorityTx, WorkRunsAuthorityTx,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::publication;
use crate::store::sources::source_observation_basis_fingerprint;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    ArtifactKind, ArtifactRole, ProjectionDomain, SourceFileId, WorkItemState, WorkPriorityClass,
    WorkRunOutcome,
};

use super::SqliteDurableStore;

pub const SOURCE_FILE_BLAKE3_ALGORITHM: &str = "blake3";

const HASH_READ_BUFFER_BYTES: usize = 64 * 1024;
const HASH_JOB_ADAPTER_KEY: &str = "dekzer.source_file_hash.blake3";
const HASH_JOB_ADAPTER_VERSION: &str = "1";
const HASH_JOB_LEASE_DURATION_MS: i64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashSourceFileBlake3Input {
    pub source_file_id: SourceFileId,
    pub source_file_path: PathBuf,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashSourceFileBlake3Result {
    pub source_file_id: SourceFileId,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    pub accepted_artifact_id: i64,
    pub work_item_id: i64,
}

#[derive(Debug, Error)]
pub enum HashSourceFileBlake3Error {
    #[error(transparent)]
    Store(#[from] LibrarySqliteError),
    #[error("source file {source_file_id} does not exist")]
    SourceFileNotFound { source_file_id: i64 },
    #[error(
        "source file {source_file_id} is not hashable because presence_state is {presence_state}"
    )]
    SourceFileUnavailable {
        source_file_id: i64,
        presence_state: String,
    },
    #[error("failed to open source file bytes at {path:?}: {source}")]
    FileOpen {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read source file bytes at {path:?}: {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("source file {source_file_id} basis changed while hashing")]
    BasisChanged { source_file_id: i64 },
    #[error("inspect-source work for source file {source_file_id} is already active")]
    WorkAlreadyActive { source_file_id: i64 },
}

type HashSourceFileBlake3JobResult<T> = Result<T, HashSourceFileBlake3Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFileHashBasis {
    source_file_id: SourceFileId,
    source_id: i64,
    relative_path: String,
    size_bytes: Option<i64>,
    mtime_ns: Option<i64>,
    presence_state: String,
    file_kind: String,
    updated_at: i64,
}

impl SourceFileHashBasis {
    fn basis_fingerprint(&self) -> String {
        source_observation_basis_fingerprint(
            self.source_file_id.get(),
            &self.relative_path,
            self.size_bytes,
            self.mtime_ns,
            self.updated_at,
        )
    }

    fn media_kind(&self) -> String {
        self.file_kind.clone()
    }
}

impl SqliteDurableStore {
    pub fn hash_source_file_blake3(
        &self,
        input: HashSourceFileBlake3Input,
    ) -> HashSourceFileBlake3JobResult<HashSourceFileBlake3Result> {
        self.hash_source_file_blake3_with_after_hash(input, || Ok(()))
    }

    fn hash_source_file_blake3_with_after_hash<F>(
        &self,
        input: HashSourceFileBlake3Input,
        after_hash: F,
    ) -> HashSourceFileBlake3JobResult<HashSourceFileBlake3Result>
    where
        F: FnOnce() -> HashSourceFileBlake3JobResult<()>,
    {
        let initial_basis = self.load_hashable_source_file_basis(input.source_file_id)?;
        let content_hash_value = hash_file_blake3(&input.source_file_path)?;
        after_hash()?;

        let file_store_root = self.app_owned_state.artifact_file_store_root().clone();
        self.with_write(|write| {
            let current_basis = load_source_file_hash_basis(write, input.source_file_id)?;
            if current_basis != initial_basis {
                return Err(LibrarySqliteError::WriteInvariant(
                    "source file basis changed while hashing".to_string(),
                ));
            }

            let result = commit_blake3_hash_evidence(
                write,
                file_store_root.clone(),
                &initial_basis,
                &content_hash_value,
                input.observed_at_ms,
            )?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(result)
        })
        .map_err(|error| match error {
            LibrarySqliteError::WriteInvariant(detail)
                if detail == "source file basis changed while hashing" =>
            {
                HashSourceFileBlake3Error::BasisChanged {
                    source_file_id: input.source_file_id.get(),
                }
            }
            LibrarySqliteError::WriteInvariant(detail)
                if detail == "inspect-source hash work is already active" =>
            {
                HashSourceFileBlake3Error::WorkAlreadyActive {
                    source_file_id: input.source_file_id.get(),
                }
            }
            other => HashSourceFileBlake3Error::Store(other),
        })
    }

    fn load_hashable_source_file_basis(
        &self,
        source_file_id: SourceFileId,
    ) -> HashSourceFileBlake3JobResult<SourceFileHashBasis> {
        let connection = self.open_read_connection()?;
        let basis = connection
            .query_row(
                "SELECT source_id,
                        relative_path,
                        size_bytes,
                        mtime_ns,
                        presence_state,
                        file_kind,
                        updated_at
                 FROM source_files
                 WHERE source_file_id = ?1",
                [source_file_id.get()],
                |row| {
                    Ok(SourceFileHashBasis {
                        source_file_id,
                        source_id: row.get(0)?,
                        relative_path: row.get(1)?,
                        size_bytes: row.get(2)?,
                        mtime_ns: row.get(3)?,
                        presence_state: row.get(4)?,
                        file_kind: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(LibrarySqliteError::from)?
            .ok_or(HashSourceFileBlake3Error::SourceFileNotFound {
                source_file_id: source_file_id.get(),
            })?;

        if basis.presence_state != "present" {
            return Err(HashSourceFileBlake3Error::SourceFileUnavailable {
                source_file_id: source_file_id.get(),
                presence_state: basis.presence_state,
            });
        }

        Ok(basis)
    }
}

fn commit_blake3_hash_evidence(
    write: &mut AdmittedWrite<'_>,
    file_store_root: crate::authority::work::ArtifactFileStoreRoot,
    basis: &SourceFileHashBasis,
    content_hash_value: &str,
    observed_at_ms: i64,
) -> LibrarySqliteResult<HashSourceFileBlake3Result> {
    let basis_fingerprint = basis.basis_fingerprint();
    let queued = WorkItemsAuthorityTx::new(write).queue_inspect_source_work(
        &QueueInspectSourceWorkInput {
            source_file_id: basis.source_file_id,
            basis_fingerprint: basis_fingerprint.clone(),
            priority_class: WorkPriorityClass::Interactive,
            queued_at: observed_at_ms,
        },
    )?;
    if queued.state != WorkItemState::Queued {
        return Err(LibrarySqliteError::WriteInvariant(
            "inspect-source hash work is already active".to_string(),
        ));
    }
    let claimed = WorkItemsAuthorityTx::new(write).claim_specific_machine_work(
        &ClaimSpecificMachineWorkInput {
            work_item_id: queued.work_item_id,
            lease_duration_ms: HASH_JOB_LEASE_DURATION_MS,
            claimed_at: observed_at_ms,
        },
    )?;
    let work_run = WorkRunsAuthorityTx::new(write).start_work_run(&StartWorkRunInput {
        work_item_id: claimed.work_item_id,
        adapter_key: HASH_JOB_ADAPTER_KEY.to_string(),
        adapter_version: HASH_JOB_ADAPTER_VERSION.to_string(),
        started_at: observed_at_ms,
    })?;

    let payload = serde_json::json!({
        "algorithm": SOURCE_FILE_BLAKE3_ALGORITHM,
        "contentHashValue": content_hash_value,
        "sourceFileId": basis.source_file_id.get(),
        "basisFingerprint": basis_fingerprint,
    })
    .to_string()
    .into_bytes();
    let payload_hash = format!("blake3:{}", blake3::hash(&payload).to_hex());
    let artifact =
        ArtifactsAuthorityTx::new(write).record_inline_artifact(&RecordInlineArtifactInput {
            artifact: RecordArtifactInput {
                work_run_id: work_run.work_run_id,
                artifact_kind: ArtifactKind::InspectionResult,
                artifact_role: ArtifactRole::PrimaryResult,
                media_type: "application/json".to_string(),
                basis_fingerprint: basis_fingerprint.clone(),
                payload_hash,
                created_at: observed_at_ms,
            },
            payload,
        })?;

    InspectSourcePromotionTx::new(write, file_store_root).inspect_source(
        &InspectSourcePromotionInput {
            source_facts: CommitAcceptedSourceFactsInput {
                source_file_id: basis.source_file_id,
                accepted_artifact_id: artifact.artifact_id,
                basis_fingerprint,
                observed_at_ms,
                content_hash: Some(ContentHashEvidence {
                    algorithm: SOURCE_FILE_BLAKE3_ALGORITHM.to_string(),
                    value: content_hash_value.to_string(),
                }),
                media_kind: basis.media_kind(),
                mime_type: None,
                duration_ms: None,
                sample_rate_hz: None,
                channels: None,
                bit_depth: None,
                codec: None,
                updated_at: observed_at_ms,
            },
            rebuild_projection_domains: vec![],
            rebuild_priority: WorkPriorityClass::Interactive,
        },
    )?;

    WorkRunsAuthorityTx::new(write).finish_work_run(&FinishWorkRunInput {
        work_run_id: work_run.work_run_id,
        finished_at: observed_at_ms,
        outcome: WorkRunOutcome::Completed,
        failure_kind: None,
        error_detail: None,
    })?;
    WorkItemsAuthorityTx::new(write).complete_machine_work_item(&CompleteMachineWorkInput {
        work_item_id: claimed.work_item_id,
        completed_at: observed_at_ms,
    })?;

    Ok(HashSourceFileBlake3Result {
        source_file_id: basis.source_file_id,
        content_hash_algorithm: SOURCE_FILE_BLAKE3_ALGORITHM.to_string(),
        content_hash_value: content_hash_value.to_string(),
        accepted_artifact_id: artifact.artifact_id.get(),
        work_item_id: claimed.work_item_id.get(),
    })
}

fn load_source_file_hash_basis(
    write: &AdmittedWrite<'_>,
    source_file_id: SourceFileId,
) -> LibrarySqliteResult<SourceFileHashBasis> {
    write
        .query_row(
            "SELECT source_id,
                    relative_path,
                    size_bytes,
                    mtime_ns,
                    presence_state,
                    file_kind,
                    updated_at
             FROM source_files
             WHERE source_file_id = ?1",
            [source_file_id.get()],
            |row| {
                Ok(SourceFileHashBasis {
                    source_file_id,
                    source_id: row.get(0)?,
                    relative_path: row.get(1)?,
                    size_bytes: row.get(2)?,
                    mtime_ns: row.get(3)?,
                    presence_state: row.get(4)?,
                    file_kind: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            LibrarySqliteError::WriteInvariant(format!(
                "source file {} does not exist",
                source_file_id.get()
            ))
        })
}

fn hash_file_blake3(path: &Path) -> HashSourceFileBlake3JobResult<String> {
    let mut file = File::open(path).map_err(|source| HashSourceFileBlake3Error::FileOpen {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; HASH_READ_BUFFER_BYTES];

    loop {
        let bytes_read =
            file.read(&mut buffer)
                .map_err(|source| HashSourceFileBlake3Error::FileRead {
                    path: path.to_path_buf(),
                    source,
                })?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use library_domain::SourceFileId;
    use rusqlite::params;
    use tempfile::TempDir;

    use crate::read_models::observed_file_facts::{
        StoreObservedFileFactStatus, read_observed_file_facts_for_source_file,
    };
    use crate::store::{SqliteDurableStore, SqliteDurableStoreAppOwnedState};

    use super::{
        HashSourceFileBlake3Error, HashSourceFileBlake3Input, SOURCE_FILE_BLAKE3_ALGORITHM,
    };

    struct HashJobFixture {
        tempdir: TempDir,
        store: SqliteDurableStore,
    }

    impl HashJobFixture {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("create temp dir");
            let db_path = tempdir.path().join("library.sqlite3");
            let app_owned_state =
                SqliteDurableStoreAppOwnedState::from_durable_store_path(&db_path);
            SqliteDurableStore::bootstrap_or_validate_app_owned_state(&app_owned_state)
                .expect("bootstrap store");
            let store = SqliteDurableStore::open_app_owned_state(app_owned_state)
                .expect("open bootstrapped store");
            store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO sources (
                             source_id,
                             source_class,
                             authority,
                             identity_key,
                             display_name,
                             is_user_visible,
                             created_at,
                             updated_at
                         )
                         VALUES (1, 'internal', 'system', 'source:hash:test', 'Hash Test', 1, 1, 1)",
                        [],
                    )?;
                    Ok(())
                })
                .expect("seed source");

            Self { tempdir, store }
        }

        fn write_source_file(
            &self,
            source_file_id: i64,
            relative_path: &str,
            bytes: &[u8],
        ) -> std::path::PathBuf {
            let path = self.tempdir.path().join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent directory");
            }
            fs::write(&path, bytes).expect("write source bytes");
            self.insert_source_file(source_file_id, relative_path, bytes.len() as i64, 1000);
            path
        }

        fn insert_source_file(
            &self,
            source_file_id: i64,
            relative_path: &str,
            size_bytes: i64,
            mtime_ns: i64,
        ) {
            let name = relative_path.rsplit('/').next().expect("file name");
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO source_files (
                             source_file_id,
                             source_id,
                             parent_source_directory_id,
                             name,
                             relative_path,
                             size_bytes,
                             mtime_ns,
                             file_kind,
                             media_class,
                             presence_state,
                             first_discovered_at,
                             last_observed_at,
                             last_presence_change_at,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, 1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, 'present', 10, 10, 10, 10, 10)",
                        params![
                            source_file_id,
                            name,
                            relative_path,
                            size_bytes,
                            mtime_ns,
                            file_kind_for_path(relative_path),
                            media_class_for_path(relative_path)
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert source file");
        }

        fn run_hash(
            &self,
            source_file_id: i64,
            path: std::path::PathBuf,
        ) -> Result<super::HashSourceFileBlake3Result, HashSourceFileBlake3Error> {
            self.store
                .hash_source_file_blake3(HashSourceFileBlake3Input {
                    source_file_id: SourceFileId::new(source_file_id)
                        .expect("positive source file"),
                    source_file_path: path,
                    observed_at_ms: 20,
                })
        }

        fn count_rows(&self, table: &str) -> i64 {
            let connection = self.store.open_read_connection().expect("open read");
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count rows")
        }
    }

    #[test]
    fn blake3_hash_is_deterministic_lowercase_hex_and_algorithm_tagged() {
        let fixture = HashJobFixture::new();
        let first_path = fixture.write_source_file(100, "Album/track-a.flac", b"dekzer bytes");
        let second_path = fixture.write_source_file(101, "Album/track-b.flac", b"dekzer bytes");

        let first = fixture.run_hash(100, first_path).expect("hash first file");
        let second = fixture
            .run_hash(101, second_path)
            .expect("hash second file");

        assert_eq!(first.content_hash_value, second.content_hash_value);
        assert_eq!(first.content_hash_algorithm, SOURCE_FILE_BLAKE3_ALGORITHM);
        assert_eq!(first.content_hash_algorithm, "blake3");
        assert_eq!(first.content_hash_value.len(), 64);
        assert!(
            first
                .content_hash_value
                .chars()
                .all(|c| { c.is_ascii_digit() || matches!(c, 'a'..='f') })
        );
    }

    #[test]
    fn hash_job_stores_current_observed_file_facts() {
        let fixture = HashJobFixture::new();
        let path = fixture.write_source_file(100, "Album/track.flac", b"current facts");

        let result = fixture.run_hash(100, path).expect("hash file");
        let connection = fixture.store.open_read_connection().expect("open read");
        let facts = read_observed_file_facts_for_source_file(&connection, 100)
            .expect("read observed facts")
            .expect("facts exist");

        assert_eq!(facts.source_file_id, 100);
        assert_eq!(
            facts.content_hash.expect("content hash").algorithm,
            "blake3"
        );
        assert_eq!(
            facts.accepted_artifact_id, result.accepted_artifact_id,
            "facts must point at the inspection artifact created by the job"
        );
        assert_eq!(facts.status, StoreObservedFileFactStatus::Current);
    }

    #[test]
    fn same_bytes_in_distinct_source_files_do_not_create_identity_rows() {
        let fixture = HashJobFixture::new();
        let first_path = fixture.write_source_file(100, "Album/a.flac", b"same bytes");
        let second_path = fixture.write_source_file(101, "Album/b.flac", b"same bytes");

        let first = fixture.run_hash(100, first_path).expect("hash first");
        let second = fixture.run_hash(101, second_path).expect("hash second");

        assert_eq!(first.content_hash_value, second.content_hash_value);
        assert_eq!(fixture.count_rows("LibraryAssets"), 0);
        assert_eq!(fixture.count_rows("LibraryAssetAttachments"), 0);
        assert_eq!(fixture.count_rows("SourceSegments"), 0);
        assert_eq!(fixture.count_rows("SourceSegmentSets"), 0);
        assert_eq!(fixture.count_rows("LibraryBrowserRows"), 0);
    }

    #[test]
    fn missing_file_returns_typed_failure_and_writes_no_source_facts() {
        let fixture = HashJobFixture::new();
        fixture.insert_source_file(100, "Album/missing.flac", 12, 1000);
        let missing_path = fixture.tempdir.path().join("Album/missing.flac");

        let error = fixture
            .run_hash(100, missing_path)
            .expect_err("missing file must fail");
        assert!(matches!(error, HashSourceFileBlake3Error::FileOpen { .. }));
        assert_eq!(fixture.count_rows("SourceFacts"), 0);
    }

    #[test]
    fn basis_change_after_hashing_does_not_commit_current_evidence() {
        let fixture = HashJobFixture::new();
        let path = fixture.write_source_file(100, "Album/changing.flac", b"old bytes");

        let error = fixture
            .store
            .hash_source_file_blake3_with_after_hash(
                HashSourceFileBlake3Input {
                    source_file_id: SourceFileId::new(100).expect("positive source file"),
                    source_file_path: path,
                    observed_at_ms: 20,
                },
                || {
                    fixture
                        .store
                        .with_write(|write| {
                            write.execute(
                                "UPDATE source_files
                                 SET size_bytes = 99,
                                     mtime_ns = 2000,
                                     updated_at = 30
                                 WHERE source_file_id = 100",
                                [],
                            )?;
                            Ok(())
                        })
                        .map_err(HashSourceFileBlake3Error::Store)
                },
            )
            .expect_err("basis change must fail");

        assert!(matches!(
            error,
            HashSourceFileBlake3Error::BasisChanged { .. }
        ));
        assert_eq!(fixture.count_rows("SourceFacts"), 0);
    }

    #[test]
    fn equivalence_fingerprint_is_not_used_as_blake3_hash_evidence() {
        let fixture = HashJobFixture::new();
        let path = fixture.write_source_file(100, "Album/track.flac", b"hash me");
        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "INSERT INTO LibraryAssets (
                         library_asset_id,
                         equivalence_fingerprint,
                         created_at,
                         updated_at
                     )
                     VALUES (1, 'eq:not-a-content-hash', 1, 1)",
                    [],
                )?;
                Ok(())
            })
            .expect("insert library asset");

        let result = fixture.run_hash(100, path).expect("hash file");

        assert_ne!(result.content_hash_value, "eq:not-a-content-hash");
        assert_eq!(result.content_hash_algorithm, "blake3");
    }

    #[test]
    fn cue_file_is_hashed_as_its_own_source_file_without_audio_pairing() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "Album/track.flac", b"audio bytes");
        let cue_path = fixture.write_source_file(101, "Album/album.cue", b"FILE track.flac WAVE");

        fixture.run_hash(101, cue_path).expect("hash cue file");
        let connection = fixture.store.open_read_connection().expect("open read");
        let cue_facts = read_observed_file_facts_for_source_file(&connection, 101)
            .expect("read cue facts")
            .expect("cue facts exist");

        assert_eq!(cue_facts.basis_relative_path, "Album/album.cue");
        assert_eq!(cue_facts.media_kind, "cue_sheet");
        assert!(cue_facts.content_hash.is_some());
        assert!(
            read_observed_file_facts_for_source_file(&connection, 100)
                .expect("read audio facts")
                .is_none(),
            "hashing a CUE file must not attach evidence to adjacent audio"
        );
        assert_eq!(fixture.count_rows("LibraryAssets"), 0);
        assert_eq!(fixture.count_rows("LibraryAssetAttachments"), 0);
        assert_eq!(fixture.count_rows("SourceSegments"), 0);
        assert_eq!(fixture.count_rows("SourceSegmentSets"), 0);
    }

    fn file_kind_for_path(path: &str) -> &'static str {
        if path.ends_with(".cue") {
            "cue_sheet"
        } else {
            "audio"
        }
    }

    fn media_class_for_path(path: &str) -> &'static str {
        if path.ends_with(".cue") {
            "unsupported"
        } else {
            "audio"
        }
    }
}
