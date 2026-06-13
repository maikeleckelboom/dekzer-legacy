use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params_from_iter, types::Value};
use thiserror::Error;

use crate::authority::promotion::{InspectSourcePromotionInput, InspectSourcePromotionTx};
use crate::authority::sources::{
    CommitAcceptedSourceFactsInput, CommitAcceptedSourceFactsMergePolicy, ContentHashEvidence,
};
use crate::authority::work::{
    ArtifactsAuthorityTx, ClaimSpecificMachineWorkInput, CompleteMachineWorkInput,
    FinishWorkRunInput, QueueInspectSourceWorkInput, RecordArtifactInput,
    RecordInlineArtifactInput, StartWorkRunInput, WorkItemsAuthorityTx, WorkRunsAuthorityTx,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::store::sources::source_observation_basis_fingerprint;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    ArtifactKind, ArtifactRole, SourceFileId, WorkItemState, WorkPriorityClass, WorkRunOutcome,
};

use super::SqliteDurableStore;

pub const SOURCE_FILE_BLAKE3_ALGORITHM: &str = "blake3";

const HASH_READ_BUFFER_BYTES: usize = 64 * 1024;
const HASH_JOB_ADAPTER_KEY: &str = "dekzer.source_file_hash.blake3";
const HASH_JOB_ADAPTER_VERSION: &str = "1";
const HASH_JOB_LEASE_DURATION_MS: i64 = 30_000;
pub const DEFAULT_HASH_BATCH_LIMIT: usize = 32;
pub const MAX_HASH_BATCH_LIMIT: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
struct HashSourceFileBlake3Input {
    source_file_id: SourceFileId,
    source_file_path: PathBuf,
    observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HashSourceFileBlake3Result {
    source_file_id: SourceFileId,
    content_hash_algorithm: String,
    content_hash_value: String,
    accepted_artifact_id: i64,
    work_item_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFileBlake3HashAdmissionScope {
    Source { source_id: i64 },
    SourceFiles { source_file_ids: Vec<SourceFileId> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadSourceFileBlake3HashCandidatesInput {
    pub scope: SourceFileBlake3HashAdmissionScope,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileBlake3HashCandidate {
    pub source_file_id: SourceFileId,
    pub source_id: i64,
    pub relative_path: String,
    pub file_class: String,
    pub file_kind: String,
    pub reason: SourceFileBlake3HashCandidateReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileBlake3HashCandidateReason {
    MissingFacts,
    MissingContentHash,
    NonBlake3ContentHash,
    StaleFacts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashSourceFileBlake3BatchInput {
    pub scope: SourceFileBlake3HashAdmissionScope,
    pub limit: Option<usize>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashSourceFileBlake3BatchResult {
    pub requested_limit: Option<usize>,
    pub effective_limit: usize,
    pub outcomes: Vec<HashSourceFileBlake3BatchOutcome>,
    pub hashed_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub remaining_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashSourceFileBlake3BatchOutcome {
    pub source_file_id: SourceFileId,
    pub source_id: i64,
    pub relative_path: String,
    pub status: HashSourceFileBlake3BatchOutcomeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashSourceFileBlake3BatchOutcomeStatus {
    Hashed {
        content_hash_value: String,
        accepted_artifact_id: i64,
        work_item_id: i64,
    },
    Skipped {
        reason: SourceFileBlake3HashSkipReason,
    },
    Failed {
        failure: SourceFileBlake3HashFailure,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileBlake3HashSkipReason {
    WorkAlreadyActive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFileBlake3HashFailure {
    SourceFileNotFound,
    SourceFileUnavailable {
        presence_state: String,
    },
    SourceRootUnavailable {
        mount_status: Option<String>,
        access_state: Option<String>,
        access_issue_kind: Option<String>,
    },
    SourceRootMissing {
        detail: Option<String>,
    },
    SourceRootBlocked {
        access_issue_kind: Option<String>,
        detail: Option<String>,
    },
    InvalidRelativePath {
        reason: String,
    },
    SourceFilePathEscapesRoot,
    PhysicalFileMissing {
        path: PathBuf,
    },
    PhysicalFileBlocked {
        path: PathBuf,
        detail: String,
    },
    FileOpen {
        path: PathBuf,
        detail: String,
    },
    FileRead {
        path: PathBuf,
        detail: String,
    },
    BasisChanged,
    Store {
        detail: String,
    },
}

#[derive(Debug, Error)]
enum HashSourceFileBlake3Error {
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
pub(super) struct SourceFileHashBasis {
    pub(super) source_file_id: SourceFileId,
    pub(super) source_id: i64,
    pub(super) relative_path: String,
    pub(super) size_bytes: Option<i64>,
    pub(super) mtime_ns: Option<i64>,
    pub(super) presence_state: String,
    pub(super) file_kind: String,
    pub(super) updated_at: i64,
}

impl SourceFileHashBasis {
    pub(super) fn basis_fingerprint(&self) -> String {
        source_observation_basis_fingerprint(
            self.source_file_id.get(),
            &self.relative_path,
            self.size_bytes,
            self.mtime_ns,
            self.updated_at,
        )
    }

    pub(super) fn media_kind(&self) -> String {
        self.file_kind.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedSourceFilePath {
    pub(super) basis: SourceFileHashBasis,
    pub(super) path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFilePathResolutionRow {
    basis: SourceFileHashBasis,
    source_class: String,
    root_path: Option<String>,
    mount_status: Option<String>,
    access_state: Option<String>,
    access_issue_kind: Option<String>,
}

#[derive(Debug, Error)]
pub(super) enum ResolveSourceFilePathError {
    #[error(transparent)]
    Store(#[from] LibrarySqliteError),
    #[error("source file {source_file_id} does not exist")]
    SourceFileNotFound { source_file_id: i64 },
    #[error(
        "source file {source_file_id} is not resolvable because presence_state is {presence_state}"
    )]
    SourceFileUnavailable {
        source_file_id: i64,
        presence_state: String,
    },
    #[error("source root is unavailable for source file {source_file_id}")]
    SourceRootUnavailable {
        source_file_id: i64,
        source_id: i64,
        mount_status: Option<String>,
        access_state: Option<String>,
        access_issue_kind: Option<String>,
    },
    #[error("source root is missing for source file {source_file_id}: {detail:?}")]
    SourceRootMissing {
        source_file_id: i64,
        source_id: i64,
        detail: Option<String>,
    },
    #[error("source root is blocked for source file {source_file_id}: {detail:?}")]
    SourceRootBlocked {
        source_file_id: i64,
        source_id: i64,
        access_issue_kind: Option<String>,
        detail: Option<String>,
    },
    #[error("source file {source_file_id} has invalid relative_path {relative_path:?}: {reason}")]
    InvalidRelativePath {
        source_file_id: i64,
        relative_path: String,
        reason: String,
    },
    #[error("source file {source_file_id} path escapes the source root")]
    SourceFilePathEscapesRoot { source_file_id: i64 },
    #[error("physical source file {source_file_id} is missing at {path:?}")]
    PhysicalFileMissing { source_file_id: i64, path: PathBuf },
    #[error("physical source file {source_file_id} is blocked at {path:?}: {source}")]
    PhysicalFileBlocked {
        source_file_id: i64,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl SqliteDurableStore {
    pub fn read_source_file_blake3_hash_candidates(
        &self,
        input: ReadSourceFileBlake3HashCandidatesInput,
    ) -> LibrarySqliteResult<Vec<SourceFileBlake3HashCandidate>> {
        let effective_limit = effective_hash_batch_limit(input.limit);
        let connection = self.open_read_connection()?;
        read_source_file_blake3_hash_candidates_for_scope(
            &connection,
            &input.scope,
            effective_limit,
        )
    }

    pub fn count_source_file_blake3_hash_candidates(
        &self,
        scope: SourceFileBlake3HashAdmissionScope,
    ) -> LibrarySqliteResult<usize> {
        let connection = self.open_read_connection()?;
        count_source_file_blake3_hash_candidates_for_scope(&connection, &scope)
    }

    pub fn hash_source_file_blake3_batch(
        &self,
        input: HashSourceFileBlake3BatchInput,
    ) -> LibrarySqliteResult<HashSourceFileBlake3BatchResult> {
        let effective_limit = effective_hash_batch_limit(input.limit);
        let candidates = self.read_source_file_blake3_hash_candidates(
            ReadSourceFileBlake3HashCandidatesInput {
                scope: input.scope.clone(),
                limit: Some(effective_limit),
            },
        )?;

        let mut outcomes = Vec::with_capacity(candidates.len());
        let mut source_scope_unavailable = false;

        for candidate in candidates {
            let source_file_id = candidate.source_file_id;
            let outcome = match self.resolve_source_file_path_for_observation(source_file_id) {
                Ok(resolved) => {
                    let source_id = resolved.basis.source_id;
                    let relative_path = resolved.basis.relative_path.clone();
                    match self.hash_source_file_blake3(HashSourceFileBlake3Input {
                        source_file_id,
                        source_file_path: resolved.path,
                        observed_at_ms: input.observed_at_ms,
                    }) {
                        Ok(result) => HashSourceFileBlake3BatchOutcome {
                            source_file_id,
                            source_id,
                            relative_path,
                            status: HashSourceFileBlake3BatchOutcomeStatus::Hashed {
                                content_hash_value: result.content_hash_value,
                                accepted_artifact_id: result.accepted_artifact_id,
                                work_item_id: result.work_item_id,
                            },
                        },
                        Err(HashSourceFileBlake3Error::WorkAlreadyActive { .. }) => {
                            HashSourceFileBlake3BatchOutcome {
                                source_file_id,
                                source_id,
                                relative_path,
                                status: HashSourceFileBlake3BatchOutcomeStatus::Skipped {
                                    reason: SourceFileBlake3HashSkipReason::WorkAlreadyActive,
                                },
                            }
                        }
                        Err(error) => HashSourceFileBlake3BatchOutcome {
                            source_file_id,
                            source_id,
                            relative_path,
                            status: HashSourceFileBlake3BatchOutcomeStatus::Failed {
                                failure: SourceFileBlake3HashFailure::from(error),
                            },
                        },
                    }
                }
                Err(error) => {
                    source_scope_unavailable = is_source_scope_unavailable_resolution_error(&error)
                        && matches!(
                            input.scope,
                            SourceFileBlake3HashAdmissionScope::Source { .. }
                        );
                    HashSourceFileBlake3BatchOutcome {
                        source_file_id,
                        source_id: candidate.source_id,
                        relative_path: candidate.relative_path,
                        status: HashSourceFileBlake3BatchOutcomeStatus::Failed {
                            failure: SourceFileBlake3HashFailure::from(error),
                        },
                    }
                }
            };

            outcomes.push(outcome);

            if source_scope_unavailable {
                break;
            }
        }

        let remaining_candidates = count_source_file_blake3_hash_candidates_for_scope(
            &self.open_read_connection()?,
            &input.scope,
        )?;

        let hashed_count = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.status,
                    HashSourceFileBlake3BatchOutcomeStatus::Hashed { .. }
                )
            })
            .count();
        let skipped_count = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.status,
                    HashSourceFileBlake3BatchOutcomeStatus::Skipped { .. }
                )
            })
            .count();
        let failed_count = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.status,
                    HashSourceFileBlake3BatchOutcomeStatus::Failed { .. }
                )
            })
            .count();

        Ok(HashSourceFileBlake3BatchResult {
            requested_limit: input.limit,
            effective_limit,
            outcomes,
            hashed_count,
            skipped_count,
            failed_count,
            remaining_candidates,
        })
    }

    fn hash_source_file_blake3(
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

    pub(super) fn resolve_source_file_path_for_observation(
        &self,
        source_file_id: SourceFileId,
    ) -> Result<ResolvedSourceFilePath, ResolveSourceFilePathError> {
        let connection = self.open_read_connection()?;
        let row = connection
            .query_row(
                "SELECT sf.source_id,
                        sf.relative_path,
                        sf.size_bytes,
                        sf.mtime_ns,
                        sf.presence_state,
                        sf.file_kind,
                        sf.updated_at,
                        s.source_class,
                        COALESCE(ss.effective_path, sl.absolute_path) AS root_path,
                        ss.mount_status,
                        ss.access_state,
                        ss.access_issue_kind
                 FROM source_files sf
                 JOIN sources s
                   ON s.source_id = sf.source_id
                 LEFT JOIN source_locators sl
                   ON sl.source_id = sf.source_id
                 LEFT JOIN source_state ss
                   ON ss.source_id = sf.source_id
                 WHERE sf.source_file_id = ?1",
                [source_file_id.get()],
                |row| {
                    Ok(SourceFilePathResolutionRow {
                        basis: SourceFileHashBasis {
                            source_file_id,
                            source_id: row.get(0)?,
                            relative_path: row.get(1)?,
                            size_bytes: row.get(2)?,
                            mtime_ns: row.get(3)?,
                            presence_state: row.get(4)?,
                            file_kind: row.get(5)?,
                            updated_at: row.get(6)?,
                        },
                        source_class: row.get(7)?,
                        root_path: row.get(8)?,
                        mount_status: row.get(9)?,
                        access_state: row.get(10)?,
                        access_issue_kind: row.get(11)?,
                    })
                },
            )
            .optional()
            .map_err(LibrarySqliteError::from)?
            .ok_or(ResolveSourceFilePathError::SourceFileNotFound {
                source_file_id: source_file_id.get(),
            })?;

        if row.basis.presence_state != "present" {
            return Err(ResolveSourceFilePathError::SourceFileUnavailable {
                source_file_id: source_file_id.get(),
                presence_state: row.basis.presence_state,
            });
        }

        if row.source_class != "internal" && row.mount_status.as_deref() != Some("mounted") {
            return Err(ResolveSourceFilePathError::SourceRootUnavailable {
                source_file_id: source_file_id.get(),
                source_id: row.basis.source_id,
                mount_status: row.mount_status,
                access_state: row.access_state,
                access_issue_kind: row.access_issue_kind,
            });
        }

        match row.access_state.as_deref() {
            Some("accessible") => {}
            Some("missing") => {
                return Err(ResolveSourceFilePathError::SourceRootMissing {
                    source_file_id: source_file_id.get(),
                    source_id: row.basis.source_id,
                    detail: row.access_issue_kind,
                });
            }
            Some("blocked") => {
                return Err(ResolveSourceFilePathError::SourceRootBlocked {
                    source_file_id: source_file_id.get(),
                    source_id: row.basis.source_id,
                    access_issue_kind: row.access_issue_kind,
                    detail: None,
                });
            }
            _ => {
                return Err(ResolveSourceFilePathError::SourceRootUnavailable {
                    source_file_id: source_file_id.get(),
                    source_id: row.basis.source_id,
                    mount_status: row.mount_status,
                    access_state: row.access_state,
                    access_issue_kind: row.access_issue_kind,
                });
            }
        }

        let root_path =
            row.root_path
                .ok_or_else(|| ResolveSourceFilePathError::SourceRootUnavailable {
                    source_file_id: source_file_id.get(),
                    source_id: row.basis.source_id,
                    mount_status: row.mount_status.clone(),
                    access_state: row.access_state.clone(),
                    access_issue_kind: row.access_issue_kind.clone(),
                })?;
        validate_source_file_relative_path(&row.basis.relative_path).map_err(|reason| {
            ResolveSourceFilePathError::InvalidRelativePath {
                source_file_id: source_file_id.get(),
                relative_path: row.basis.relative_path.clone(),
                reason,
            }
        })?;

        let joined_path = join_source_root_and_relative_path(&root_path, &row.basis.relative_path);
        let canonical_root = std::fs::canonicalize(&root_path).map_err(|source| {
            map_root_canonicalize_error(source_file_id, row.basis.source_id, source)
        })?;
        let canonical_file = std::fs::canonicalize(&joined_path).map_err(|source| {
            map_file_canonicalize_error(source_file_id, joined_path.clone(), source)
        })?;

        if !canonical_file.starts_with(&canonical_root) {
            return Err(ResolveSourceFilePathError::SourceFilePathEscapesRoot {
                source_file_id: source_file_id.get(),
            });
        }

        Ok(ResolvedSourceFilePath {
            basis: row.basis,
            path: canonical_file,
        })
    }
}

pub fn effective_hash_batch_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_HASH_BATCH_LIMIT)
        .clamp(1, MAX_HASH_BATCH_LIMIT)
}

fn read_source_file_blake3_hash_candidates_for_scope(
    connection: &rusqlite::Connection,
    scope: &SourceFileBlake3HashAdmissionScope,
    limit: usize,
) -> LibrarySqliteResult<Vec<SourceFileBlake3HashCandidate>> {
    let (scope_predicate, mut values) = hash_candidate_scope_predicate(scope);
    values.push(Value::Integer(i64::try_from(limit).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "hash candidate limit {limit} exceeds i64 range"
        ))
    })?));
    let sql = format!(
        "SELECT sf.source_file_id,
                sf.source_id,
                sf.relative_path,
                sf.file_class,
                sf.file_kind,
                CASE
                    WHEN facts.source_file_id IS NULL THEN 'missing_facts'
                    WHEN facts.content_hash_algorithm IS NULL
                      OR facts.content_hash_value IS NULL THEN 'missing_content_hash'
                    WHEN facts.content_hash_algorithm != ?1 THEN 'non_blake3_content_hash'
                    ELSE 'stale_facts'
                END AS reason
         FROM source_files sf
         LEFT JOIN SourceFacts facts
           ON facts.source_file_id = sf.source_file_id
         WHERE {scope_predicate}
           AND sf.presence_state = 'present'
           AND {}
           AND {}
         ORDER BY lower(sf.relative_path) ASC,
                  sf.source_file_id ASC
         LIMIT ?{}",
        media_relevant_source_file_predicate_sql("sf"),
        needs_blake3_hash_predicate_sql("sf", "facts"),
        values.len() + 1
    );
    let mut statement = connection.prepare(&sql)?;
    let mut query_values = vec![Value::Text(SOURCE_FILE_BLAKE3_ALGORITHM.to_string())];
    query_values.extend(values);
    let rows = statement
        .query_map(params_from_iter(query_values.iter()), |row| {
            let source_file_id = row.get::<_, i64>(0)?;
            Ok(SourceFileBlake3HashCandidate {
                source_file_id: SourceFileId::new(source_file_id)
                    .ok_or_else(|| rusqlite::Error::IntegralValueOutOfRange(0, source_file_id))?,
                source_id: row.get(1)?,
                relative_path: row.get(2)?,
                file_class: row.get(3)?,
                file_kind: row.get(4)?,
                reason: parse_hash_candidate_reason(&row.get::<_, String>(5)?).map_err(
                    |reason| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(LibrarySqliteError::MalformedSchemaState(reason)),
                        )
                    },
                )?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn count_source_file_blake3_hash_candidates_for_scope(
    connection: &rusqlite::Connection,
    scope: &SourceFileBlake3HashAdmissionScope,
) -> LibrarySqliteResult<usize> {
    let (scope_predicate, values) = hash_candidate_scope_predicate(scope);
    let sql = format!(
        "SELECT COUNT(*)
         FROM source_files sf
         LEFT JOIN SourceFacts facts
           ON facts.source_file_id = sf.source_file_id
         WHERE {scope_predicate}
           AND sf.presence_state = 'present'
           AND {}
           AND {}",
        media_relevant_source_file_predicate_sql("sf"),
        needs_blake3_hash_predicate_sql("sf", "facts")
    );
    let mut query_values = vec![Value::Text(SOURCE_FILE_BLAKE3_ALGORITHM.to_string())];
    query_values.extend(values);
    let count = connection.query_row(&sql, params_from_iter(query_values.iter()), |row| {
        row.get::<_, i64>(0)
    })?;
    usize::try_from(count).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "hash candidate count {count} cannot fit in usize"
        ))
    })
}

fn hash_candidate_scope_predicate(
    scope: &SourceFileBlake3HashAdmissionScope,
) -> (String, Vec<Value>) {
    match scope {
        SourceFileBlake3HashAdmissionScope::Source { source_id } => (
            "sf.source_id = ?2".to_string(),
            vec![Value::Integer(*source_id)],
        ),
        SourceFileBlake3HashAdmissionScope::SourceFiles { source_file_ids } => {
            if source_file_ids.is_empty() {
                return ("0 = 1".to_string(), Vec::new());
            }
            let mut ids = source_file_ids
                .iter()
                .map(|source_file_id| source_file_id.get())
                .collect::<Vec<_>>();
            ids.sort_unstable();
            ids.dedup();
            let placeholders = (0..ids.len())
                .map(|index| format!("?{}", index + 2))
                .collect::<Vec<_>>()
                .join(", ");
            (
                format!("sf.source_file_id IN ({placeholders})"),
                ids.into_iter().map(Value::Integer).collect(),
            )
        }
    }
}

fn media_relevant_source_file_predicate_sql(alias: &str) -> String {
    format!(
        "({alias}.file_class IN ('audio', 'video', 'image')
          OR ({alias}.file_class = 'unsupported' AND {alias}.file_kind = 'cue_sheet'))"
    )
}

fn needs_blake3_hash_predicate_sql(source_file_alias: &str, facts_alias: &str) -> String {
    format!(
        "({facts_alias}.source_file_id IS NULL
          OR {facts_alias}.content_hash_algorithm IS NULL
          OR {facts_alias}.content_hash_value IS NULL
          OR {facts_alias}.content_hash_algorithm != ?1
          OR NOT (
              {source_file_alias}.source_id = {facts_alias}.basis_source_id
              AND {source_file_alias}.relative_path = {facts_alias}.basis_relative_path
              AND {source_file_alias}.size_bytes IS {facts_alias}.basis_size_bytes
              AND {source_file_alias}.mtime_ns IS {facts_alias}.basis_mtime_ns
              AND {source_file_alias}.presence_state = {facts_alias}.basis_presence_state
          ))"
    )
}

fn parse_hash_candidate_reason(
    reason: &str,
) -> Result<SourceFileBlake3HashCandidateReason, String> {
    match reason {
        "missing_facts" => Ok(SourceFileBlake3HashCandidateReason::MissingFacts),
        "missing_content_hash" => Ok(SourceFileBlake3HashCandidateReason::MissingContentHash),
        "non_blake3_content_hash" => Ok(SourceFileBlake3HashCandidateReason::NonBlake3ContentHash),
        "stale_facts" => Ok(SourceFileBlake3HashCandidateReason::StaleFacts),
        other => Err(format!("unknown hash candidate reason {other:?}")),
    }
}

fn validate_source_file_relative_path(relative_path: &str) -> Result<(), String> {
    if relative_path.trim().is_empty() {
        return Err("path is empty".to_string());
    }
    if relative_path.contains("://") {
        return Err("path contains a URI scheme".to_string());
    }
    if has_drive_prefix(relative_path) {
        return Err("path contains a drive prefix".to_string());
    }
    if relative_path.contains('\\') {
        return Err("path contains backslashes".to_string());
    }
    if relative_path.starts_with('/') {
        return Err("path is absolute".to_string());
    }
    if relative_path.contains("//") {
        return Err("path contains an empty segment".to_string());
    }

    for segment in relative_path.split('/') {
        if segment.is_empty() {
            return Err("path contains an empty segment".to_string());
        }
        if segment == "." || segment == ".." {
            return Err("path contains a traversal segment".to_string());
        }
    }

    Ok(())
}

fn join_source_root_and_relative_path(root_path: &str, relative_path: &str) -> PathBuf {
    let mut path = PathBuf::from(root_path);
    for segment in relative_path.split('/') {
        path.push(segment);
    }
    path
}

fn has_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn map_root_canonicalize_error(
    source_file_id: SourceFileId,
    source_id: i64,
    source: std::io::Error,
) -> ResolveSourceFilePathError {
    match source.kind() {
        std::io::ErrorKind::NotFound => ResolveSourceFilePathError::SourceRootMissing {
            source_file_id: source_file_id.get(),
            source_id,
            detail: Some(source.to_string()),
        },
        _ => ResolveSourceFilePathError::SourceRootBlocked {
            source_file_id: source_file_id.get(),
            source_id,
            access_issue_kind: None,
            detail: Some(source.to_string()),
        },
    }
}

fn map_file_canonicalize_error(
    source_file_id: SourceFileId,
    path: PathBuf,
    source: std::io::Error,
) -> ResolveSourceFilePathError {
    match source.kind() {
        std::io::ErrorKind::NotFound => ResolveSourceFilePathError::PhysicalFileMissing {
            source_file_id: source_file_id.get(),
            path,
        },
        _ => ResolveSourceFilePathError::PhysicalFileBlocked {
            source_file_id: source_file_id.get(),
            path,
            source,
        },
    }
}

pub(super) fn is_source_scope_unavailable_resolution_error(
    error: &ResolveSourceFilePathError,
) -> bool {
    matches!(
        error,
        ResolveSourceFilePathError::SourceRootUnavailable { .. }
            | ResolveSourceFilePathError::SourceRootMissing { .. }
            | ResolveSourceFilePathError::SourceRootBlocked { .. }
    )
}

impl From<ResolveSourceFilePathError> for SourceFileBlake3HashFailure {
    fn from(error: ResolveSourceFilePathError) -> Self {
        match error {
            ResolveSourceFilePathError::Store(error) => Self::Store {
                detail: error.to_string(),
            },
            ResolveSourceFilePathError::SourceFileNotFound { .. } => Self::SourceFileNotFound,
            ResolveSourceFilePathError::SourceFileUnavailable { presence_state, .. } => {
                Self::SourceFileUnavailable { presence_state }
            }
            ResolveSourceFilePathError::SourceRootUnavailable {
                mount_status,
                access_state,
                access_issue_kind,
                ..
            } => Self::SourceRootUnavailable {
                mount_status,
                access_state,
                access_issue_kind,
            },
            ResolveSourceFilePathError::SourceRootMissing { detail, .. } => {
                Self::SourceRootMissing { detail }
            }
            ResolveSourceFilePathError::SourceRootBlocked {
                access_issue_kind,
                detail,
                ..
            } => Self::SourceRootBlocked {
                access_issue_kind,
                detail,
            },
            ResolveSourceFilePathError::InvalidRelativePath { reason, .. } => {
                Self::InvalidRelativePath { reason }
            }
            ResolveSourceFilePathError::SourceFilePathEscapesRoot { .. } => {
                Self::SourceFilePathEscapesRoot
            }
            ResolveSourceFilePathError::PhysicalFileMissing { path, .. } => {
                Self::PhysicalFileMissing { path }
            }
            ResolveSourceFilePathError::PhysicalFileBlocked { path, source, .. } => {
                Self::PhysicalFileBlocked {
                    path,
                    detail: source.to_string(),
                }
            }
        }
    }
}

impl From<HashSourceFileBlake3Error> for SourceFileBlake3HashFailure {
    fn from(error: HashSourceFileBlake3Error) -> Self {
        match error {
            HashSourceFileBlake3Error::Store(error) => Self::Store {
                detail: error.to_string(),
            },
            HashSourceFileBlake3Error::SourceFileNotFound { .. } => Self::SourceFileNotFound,
            HashSourceFileBlake3Error::SourceFileUnavailable { presence_state, .. } => {
                Self::SourceFileUnavailable { presence_state }
            }
            HashSourceFileBlake3Error::FileOpen { path, source } => Self::FileOpen {
                path,
                detail: source.to_string(),
            },
            HashSourceFileBlake3Error::FileRead { path, source } => Self::FileRead {
                path,
                detail: source.to_string(),
            },
            HashSourceFileBlake3Error::BasisChanged { .. } => Self::BasisChanged,
            HashSourceFileBlake3Error::WorkAlreadyActive { .. } => Self::Store {
                detail: "inspect-source work is already active".to_string(),
            },
        }
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
            source_facts_merge_policy:
                CommitAcceptedSourceFactsMergePolicy::preserve_current_probe_fields(),
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

pub(super) fn load_source_file_hash_basis(
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
        HashSourceFileBlake3BatchInput, HashSourceFileBlake3BatchOutcomeStatus,
        HashSourceFileBlake3Error, HashSourceFileBlake3Input,
        ReadSourceFileBlake3HashCandidatesInput, SOURCE_FILE_BLAKE3_ALGORITHM,
        SourceFileBlake3HashAdmissionScope, SourceFileBlake3HashCandidateReason,
        SourceFileBlake3HashFailure,
    };

    struct HashJobFixture {
        tempdir: TempDir,
        store: SqliteDurableStore,
    }

    impl HashJobFixture {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("create temp dir");
            let db_path = tempdir.path().join("library.sqlite3");
            let root_path = fs::canonicalize(tempdir.path()).expect("canonicalize temp root");
            let root_path_text = root_path.to_string_lossy().to_string();
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
                    write.execute(
                        "INSERT INTO source_locators (
                             source_id,
                             locator_kind,
                             absolute_path,
                             device_identity_kind,
                             device_identity_value,
                             relative_suffix
                         )
                         VALUES (1, 'absolute_path', ?1, NULL, NULL, '')",
                        [&root_path_text],
                    )?;
                    write.execute(
                        "INSERT INTO source_state (
                             source_id,
                             mount_status,
                             mount_epoch,
                             access_state,
                             access_issue_kind,
                             access_error_detail,
                             access_checked_at,
                             mount_root,
                             effective_path,
                             observed_volume_label,
                             filesystem_type,
                             last_seen_at,
                             updated_at
                         )
                         VALUES (1, 'mounted', 0, 'accessible', NULL, NULL, 2, NULL, ?1, NULL, NULL, 2, 2)",
                        [&root_path_text],
                    )?;
                    write.execute(
                        "INSERT INTO source_scan_state (
                             source_id,
                             scan_phase,
                             last_scan_started_at,
                             last_scan_finished_at,
                             last_successful_scan_at,
                             scan_issue_kind,
                             error_detail,
                             updated_at
                         )
                         VALUES (1, 'complete', 3, 4, 4, NULL, NULL, 4)",
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
            let name_browse_sort_key = crate::browse_sort_key::compute_name_browse_sort_key(name);
            let relative_path_browse_sort_key =
                crate::browse_sort_key::compute_relative_path_browse_sort_key(relative_path);
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO source_files (
                             source_file_id,
                             source_id,
                             parent_source_directory_id,
                             name,
                             name_browse_sort_key,
                             relative_path_browse_sort_key,
                             relative_path,
                             size_bytes,
                             mtime_ns,
                             file_kind,
                             file_class,
                             presence_state,
                             first_discovered_at,
                             last_observed_at,
                             last_presence_change_at,
                             created_at,
                             updated_at
                         )
                        VALUES (?1, 1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'present', 10, 10, 10, 10, 10)",
                        params![
                            source_file_id,
                            name,
                            name_browse_sort_key,
                            relative_path_browse_sort_key,
                            relative_path,
                            size_bytes,
                            mtime_ns,
                            file_kind_for_path(relative_path),
                            file_class_for_path(relative_path)
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert source file");
        }

        fn set_source_state(
            &self,
            source_class: &str,
            authority: &str,
            mount_status: &str,
            access_state: &str,
            access_issue_kind: Option<&str>,
            effective_path: Option<&str>,
        ) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "UPDATE sources
                         SET source_class = ?1,
                             authority = ?2,
                             updated_at = 30
                         WHERE source_id = 1",
                        params![source_class, authority],
                    )?;
                    write.execute(
                        "UPDATE source_state
                         SET mount_status = ?1,
                             access_state = ?2,
                             access_issue_kind = ?3,
                             effective_path = ?4,
                             updated_at = 30
                         WHERE source_id = 1",
                        params![
                            mount_status,
                            access_state,
                            access_issue_kind,
                            effective_path
                        ],
                    )?;
                    Ok(())
                })
                .expect("update source state");
        }

        fn read_candidates_for_source(
            &self,
            limit: usize,
        ) -> Vec<super::SourceFileBlake3HashCandidate> {
            self.store
                .read_source_file_blake3_hash_candidates(ReadSourceFileBlake3HashCandidatesInput {
                    scope: SourceFileBlake3HashAdmissionScope::Source { source_id: 1 },
                    limit: Some(limit),
                })
                .expect("read hash candidates")
        }

        fn run_hash_batch_for_source(
            &self,
            limit: usize,
        ) -> super::HashSourceFileBlake3BatchResult {
            self.store
                .hash_source_file_blake3_batch(HashSourceFileBlake3BatchInput {
                    scope: SourceFileBlake3HashAdmissionScope::Source { source_id: 1 },
                    limit: Some(limit),
                    observed_at_ms: 40,
                })
                .expect("run hash batch")
        }

        fn run_hash_batch_for_source_files(
            &self,
            source_file_ids: &[i64],
            limit: usize,
        ) -> super::HashSourceFileBlake3BatchResult {
            self.store
                .hash_source_file_blake3_batch(HashSourceFileBlake3BatchInput {
                    scope: SourceFileBlake3HashAdmissionScope::SourceFiles {
                        source_file_ids: source_file_ids
                            .iter()
                            .map(|id| SourceFileId::new(*id).expect("positive source file id"))
                            .collect(),
                    },
                    limit: Some(limit),
                    observed_at_ms: 40,
                })
                .expect("run hash batch")
        }

        fn insert_source_fact(
            &self,
            artifact_id: i64,
            source_file_id: i64,
            content_hash_algorithm: Option<&str>,
            content_hash_value: Option<&str>,
        ) {
            let connection = self.store.open_read_connection().expect("open read");
            let (relative_path, size_bytes, mtime_ns, presence_state, file_kind): (
                String,
                Option<i64>,
                Option<i64>,
                String,
                String,
            ) = connection
                .query_row(
                    "SELECT relative_path,
                            size_bytes,
                            mtime_ns,
                            presence_state,
                            file_kind
                     FROM source_files
                     WHERE source_file_id = ?1",
                    [source_file_id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .expect("load source file basis");
            drop(connection);

            let basis_fingerprint = format!("test:basis:{source_file_id}:{artifact_id}");
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO WorkItems (
                             work_item_id,
                             subject_kind,
                             subject_id,
                             work_kind,
                             priority_class,
                             basis_fingerprint,
                             state,
                             attempt_count,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, 'source_file', ?2, 'inspect_source', 'interactive', ?3, 'completed', 1, 20, 20)",
                        params![artifact_id, source_file_id.to_string(), basis_fingerprint],
                    )?;
                    write.execute(
                        "INSERT INTO WorkRuns (
                             work_run_id,
                             work_item_id,
                             adapter_key,
                             adapter_version,
                             started_at,
                             finished_at,
                             outcome
                         )
                         VALUES (?1, ?1, 'test.hash.fixture', '1', 21, 21, 'completed')",
                        [artifact_id],
                    )?;
                    write.execute(
                        "INSERT INTO Artifacts (
                             artifact_id,
                             work_run_id,
                             subject_kind,
                             subject_id,
                             artifact_kind,
                             artifact_role,
                             adapter_key,
                             adapter_version,
                             basis_fingerprint,
                             media_type,
                             storage_kind,
                             payload_hash,
                             created_at
                         )
                         VALUES (?1, ?1, 'source_file', ?2, 'inspection_result', 'primary_result', 'test.hash.fixture', '1', ?3, 'application/json', 'inline_payload', ?4, 22)",
                        params![
                            artifact_id,
                            source_file_id.to_string(),
                            basis_fingerprint,
                            format!("fixture:artifact:{artifact_id}")
                        ],
                    )?;
                    write.execute(
                        "INSERT INTO SourceFacts (
                             source_file_id,
                             fact_kind,
                             basis_fingerprint,
                             basis_source_id,
                             basis_relative_path,
                             basis_size_bytes,
                             basis_mtime_ns,
                             basis_presence_state,
                             observed_at_ms,
                             content_hash_algorithm,
                             content_hash_value,
                             media_kind,
                             mime_type,
                             duration_ms,
                             sample_rate_hz,
                             channels,
                             bit_depth,
                             codec,
                             updated_at,
                             accepted_artifact_id
                         )
                         VALUES (?1, 'source_inspection', ?2, 1, ?3, ?4, ?5, ?6, 23, ?7, ?8, ?9, NULL, NULL, NULL, NULL, NULL, NULL, 24, ?10)",
                        params![
                            source_file_id,
                            basis_fingerprint,
                            relative_path,
                            size_bytes,
                            mtime_ns,
                            presence_state,
                            content_hash_algorithm,
                            content_hash_value,
                            file_kind,
                            artifact_id,
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert source fact");
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

        fn content_hash_value(&self, source_file_id: i64) -> String {
            let connection = self.store.open_read_connection().expect("open read");
            read_observed_file_facts_for_source_file(&connection, source_file_id)
                .expect("read observed facts")
                .expect("facts exist")
                .content_hash
                .expect("content hash exists")
                .value
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
    }

    #[test]
    fn batch_resolver_joins_source_root_with_source_file_relative_path() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "Album/track.flac", b"root relative bytes");

        let result = fixture.run_hash_batch_for_source_files(&[100], 10);

        assert_eq!(result.hashed_count, 1);
        assert_eq!(result.failed_count, 0);
        assert_eq!(
            fixture.content_hash_value(100),
            blake3::hash(b"root relative bytes").to_hex().to_string()
        );
    }

    #[test]
    fn batch_rejects_traversal_relative_path_without_writing_source_facts() {
        let fixture = HashJobFixture::new();
        fixture.insert_source_file(100, "../escape.flac", 11, 1000);

        let result = fixture.run_hash_batch_for_source_files(&[100], 10);

        assert_eq!(result.hashed_count, 0);
        assert_eq!(result.failed_count, 1);
        assert!(matches!(
            result.outcomes[0].status,
            HashSourceFileBlake3BatchOutcomeStatus::Failed {
                failure: SourceFileBlake3HashFailure::InvalidRelativePath { .. }
            }
        ));
        assert_eq!(fixture.count_rows("SourceFacts"), 0);
    }

    #[test]
    fn batch_hashes_resolved_row_path_not_bytes_from_another_source_file() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "Album/track.flac", b"authoritative bytes");
        fixture.write_source_file(101, "Other/track.flac", b"wrong bytes");

        let result = fixture.run_hash_batch_for_source_files(&[100], 10);

        assert_eq!(result.hashed_count, 1);
        assert_eq!(
            fixture.content_hash_value(100),
            blake3::hash(b"authoritative bytes").to_hex().to_string()
        );
        assert!(
            fixture
                .store
                .read_observed_file_facts_for_source_file(101)
                .expect("read facts")
                .is_none(),
            "batch scoped to source_file_id 100 must not hash row 101"
        );
    }

    #[test]
    fn batch_missing_physical_file_returns_typed_failure_and_writes_no_source_facts() {
        let fixture = HashJobFixture::new();
        fixture.insert_source_file(100, "Album/missing.flac", 12, 1000);

        let result = fixture.run_hash_batch_for_source_files(&[100], 10);

        assert_eq!(result.hashed_count, 0);
        assert_eq!(result.failed_count, 1);
        assert!(matches!(
            result.outcomes[0].status,
            HashSourceFileBlake3BatchOutcomeStatus::Failed {
                failure: SourceFileBlake3HashFailure::PhysicalFileMissing { .. }
            }
        ));
        assert_eq!(fixture.count_rows("SourceFacts"), 0);
    }

    #[test]
    fn batch_unmounted_source_returns_typed_failure_without_writing_source_facts() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "Album/track.flac", b"unmounted bytes");
        fixture.set_source_state(
            "external_mounted",
            "device",
            "unmounted",
            "unknown",
            Some("unavailable_mount"),
            None,
        );

        let result = fixture.run_hash_batch_for_source(10);

        assert_eq!(result.hashed_count, 0);
        assert_eq!(result.failed_count, 1);
        assert!(matches!(
            result.outcomes[0].status,
            HashSourceFileBlake3BatchOutcomeStatus::Failed {
                failure: SourceFileBlake3HashFailure::SourceRootUnavailable { .. }
            }
        ));
        assert_eq!(fixture.count_rows("SourceFacts"), 0);
    }

    #[test]
    fn candidate_selection_includes_media_relevant_files_and_admitted_cue_only() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "Album/audio.flac", b"audio");
        fixture.write_source_file(101, "Album/video.mp4", b"video");
        fixture.write_source_file(102, "Album/cover.jpg", b"image");
        fixture.write_source_file(103, "Album/album.cue", b"cue");
        fixture.write_source_file(104, "Album/notes.txt", b"text");
        fixture.write_source_file(105, "Album/archive.zip", b"archive");
        fixture.write_source_file(106, "Album/data.bin", b"other");
        fixture.write_source_file(107, "Album/unknown.nope", b"unknown");

        let candidate_ids = fixture
            .read_candidates_for_source(20)
            .into_iter()
            .map(|candidate| candidate.source_file_id.get())
            .collect::<Vec<_>>();

        assert_eq!(candidate_ids, vec![103, 100, 102, 101]);
    }

    #[test]
    fn current_blake3_evidence_is_not_a_candidate() {
        let fixture = HashJobFixture::new();
        let path = fixture.write_source_file(100, "Album/current.flac", b"current");
        fixture.run_hash(100, path).expect("seed current blake3");

        let candidates = fixture.read_candidates_for_source(10);

        assert!(candidates.is_empty());
    }

    #[test]
    fn stale_blake3_evidence_is_rehashed_against_current_basis() {
        let fixture = HashJobFixture::new();
        let path = fixture.write_source_file(100, "Album/stale.flac", b"old bytes");
        fixture.run_hash(100, path.clone()).expect("seed old hash");
        fs::write(&path, b"new bytes").expect("rewrite physical file");
        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "UPDATE source_files
                     SET size_bytes = ?1,
                         mtime_ns = 2000,
                         updated_at = 30
                     WHERE source_file_id = 100",
                    [i64::try_from(b"new bytes".len()).expect("len fits i64")],
                )?;
                Ok(())
            })
            .expect("mark source file basis stale");

        let candidates = fixture.read_candidates_for_source(10);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].reason,
            SourceFileBlake3HashCandidateReason::StaleFacts
        );

        let result = fixture.run_hash_batch_for_source(10);

        assert_eq!(result.hashed_count, 1);
        assert_eq!(
            fixture.content_hash_value(100),
            blake3::hash(b"new bytes").to_hex().to_string()
        );
    }

    #[test]
    fn current_sha256_evidence_is_candidate_for_blake3() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "Album/sha.flac", b"hash me with blake3");
        fixture.insert_source_fact(9000, 100, Some("sha256"), Some("fixture-sha"));

        let candidates = fixture.read_candidates_for_source(10);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].reason,
            SourceFileBlake3HashCandidateReason::NonBlake3ContentHash
        );

        let result = fixture.run_hash_batch_for_source(10);

        assert_eq!(result.hashed_count, 1);
        let facts_hash = fixture.content_hash_value(100);
        assert_ne!(facts_hash, "fixture-sha");
        assert_eq!(
            facts_hash,
            blake3::hash(b"hash me with blake3").to_hex().to_string()
        );
    }

    #[test]
    fn bounded_batch_respects_limit_and_deterministic_relative_path_order() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "z.flac", b"z");
        fixture.write_source_file(101, "A.flac", b"a");
        fixture.write_source_file(102, "b.flac", b"b");

        let result = fixture.run_hash_batch_for_source(2);
        let outcome_paths = result
            .outcomes
            .iter()
            .map(|outcome| outcome.relative_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(result.effective_limit, 2);
        assert_eq!(result.hashed_count, 2);
        assert_eq!(result.remaining_candidates, 1);
        assert_eq!(outcome_paths, vec!["A.flac", "b.flac"]);
        assert!(
            fixture
                .store
                .read_observed_file_facts_for_source_file(100)
                .expect("read z facts")
                .is_none(),
            "third deterministic candidate must remain unhashed under limit"
        );
    }

    #[test]
    fn per_file_failure_continues_and_preserves_inventory_and_unrelated_facts() {
        let fixture = HashJobFixture::new();
        fixture.insert_source_file(100, "Album/missing.flac", 7, 1000);
        fixture.write_source_file(101, "Album/ok.flac", b"ok bytes");
        let current_path = fixture.write_source_file(102, "Album/current.flac", b"already current");
        let current_before = fixture
            .run_hash(102, current_path)
            .expect("seed current unrelated hash")
            .content_hash_value;

        let result = fixture.run_hash_batch_for_source(10);

        assert_eq!(result.failed_count, 1);
        assert_eq!(result.hashed_count, 1);
        assert_eq!(fixture.count_rows("source_files"), 3);
        let current_after = fixture
            .store
            .read_observed_file_facts_for_source_file(102)
            .expect("read unrelated facts")
            .expect("unrelated facts exist");
        assert_eq!(current_after.status, StoreObservedFileFactStatus::Current);
        assert_eq!(
            current_after.content_hash.expect("content hash").value,
            current_before
        );
    }

    #[test]
    fn batch_same_bytes_in_distinct_files_do_not_create_identity_rows() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "Album/a.flac", b"same batch bytes");
        fixture.write_source_file(101, "Album/b.flac", b"same batch bytes");

        let result = fixture.run_hash_batch_for_source(10);

        assert_eq!(result.hashed_count, 2);
        assert_eq!(
            fixture.content_hash_value(100),
            fixture.content_hash_value(101)
        );
    }

    #[test]
    fn batch_cue_file_hashes_itself_without_pairing_or_identity_rows() {
        let fixture = HashJobFixture::new();
        fixture.write_source_file(100, "Album/track.flac", b"audio bytes");
        fixture.write_source_file(101, "Album/album.cue", b"FILE track.flac WAVE");

        let result = fixture.run_hash_batch_for_source_files(&[101], 10);

        assert_eq!(result.hashed_count, 1);
        let connection = fixture.store.open_read_connection().expect("open read");
        let cue_facts = read_observed_file_facts_for_source_file(&connection, 101)
            .expect("read cue facts")
            .expect("cue facts exist");
        assert_eq!(cue_facts.basis_relative_path, "Album/album.cue");
        assert_eq!(cue_facts.media_kind, "cue_sheet");
        assert!(
            read_observed_file_facts_for_source_file(&connection, 100)
                .expect("read audio facts")
                .is_none()
        );
    }

    fn file_kind_for_path(path: &str) -> &'static str {
        crate::browse_media::file_kind_str_from_path(path)
    }

    fn file_class_for_path(path: &str) -> &'static str {
        crate::browse_media::file_class_str_from_path(path)
    }
}
