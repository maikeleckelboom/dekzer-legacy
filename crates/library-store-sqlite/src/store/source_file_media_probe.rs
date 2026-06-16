use std::fs::File;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params_from_iter, types::Value};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::{AudioCodecParameters, CODEC_ID_NULL_AUDIO};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, Track, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use thiserror::Error;

use crate::authority::promotion::{InspectSourceFilePromotionInput, InspectSourceFilePromotionTx};
use crate::authority::sources::{
    CommitAcceptedSourceFileObservationInput, CommitAcceptedSourceFileObservationMergePolicy,
};
use crate::authority::work::{
    ClaimSpecificMachineWorkInput, CompleteMachineWorkInput, FinishWorkRunInput,
    QueueInspectSourceFileWorkInput, RecordArtifactInput, RecordInlineArtifactInput,
    StartWorkRunInput, WorkArtifactAuthorityTx, WorkItemAuthorityTx, WorkRunAuthorityTx,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::store::source_file_hash::{
    ResolveSourceFilePathError, SourceFileHashBasis, is_source_scope_unavailable_resolution_error,
    load_source_file_hash_basis,
};
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    ArtifactKind, SourceFileId, WorkItemState, WorkPriorityClass, WorkRunOutcome,
};

use super::SqliteDurableStore;

const MEDIA_PROBE_JOB_ADAPTER_KEY: &str = "dekzer.source_file_media_probe.symphonia";
const MEDIA_PROBE_JOB_ADAPTER_VERSION: &str = "1";
const MEDIA_PROBE_JOB_LEASE_DURATION_MS: i64 = 30_000;
const MEDIA_PROBE_READ_BUFFER_BYTES: usize = 64 * 1024;
pub const DEFAULT_MEDIA_PROBE_BATCH_LIMIT: usize = 16;
pub const MAX_MEDIA_PROBE_BATCH_LIMIT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFileMediaProbeAdmissionScope {
    Source { source_id: i64 },
    SourceFiles { source_file_ids: Vec<SourceFileId> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadSourceFileMediaProbeCandidatesInput {
    pub scope: SourceFileMediaProbeAdmissionScope,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileMediaProbeCandidate {
    pub source_file_id: SourceFileId,
    pub source_id: i64,
    pub relative_path: String,
    pub file_class: String,
    pub file_kind: String,
    pub reason: SourceFileMediaProbeCandidateReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileMediaProbeCandidateReason {
    MissingObservations,
    MissingProbeFields,
    StaleObservations,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSourceFileMediaBatchInput {
    pub scope: SourceFileMediaProbeAdmissionScope,
    pub limit: Option<usize>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSourceFileMediaBatchResult {
    pub requested_limit: Option<usize>,
    pub effective_limit: usize,
    pub outcomes: Vec<ProbeSourceFileMediaBatchOutcome>,
    pub probed_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub remaining_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSourceFileMediaBatchOutcome {
    pub source_file_id: SourceFileId,
    pub source_id: i64,
    pub relative_path: String,
    pub status: ProbeSourceFileMediaBatchOutcomeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeSourceFileMediaBatchOutcomeStatus {
    Probed {
        observations: SourceFileMediaProbeObservations,
        accepted_artifact_id: i64,
        work_item_id: i64,
    },
    Skipped {
        reason: SourceFileMediaProbeSkipReason,
    },
    Failed {
        failure: SourceFileMediaProbeFailure,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileMediaProbeSkipReason {
    WorkAlreadyActive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFileMediaProbeFailure {
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
    UnsupportedMediaKind {
        file_kind: String,
    },
    UnsupportedFormat {
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
    Probe {
        detail: String,
    },
    BasisChanged,
    Store {
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileMediaProbeObservations {
    pub media_kind: String,
    pub mime_type: Option<String>,
    pub duration_ms: Option<i64>,
    pub sample_rate_hz: Option<i64>,
    pub channels: Option<i64>,
    pub bit_depth: Option<i64>,
    pub codec: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeSourceFileMediaInput {
    source_file_id: SourceFileId,
    source_file_path: PathBuf,
    observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeSourceFileMediaResult {
    source_file_id: SourceFileId,
    observations: SourceFileMediaProbeObservations,
    accepted_artifact_id: i64,
    work_item_id: i64,
}

#[derive(Debug, Error)]
enum ProbeSourceFileMediaError {
    #[error(transparent)]
    Store(#[from] LibrarySqliteError),
    #[error("source file {source_file_id} does not exist")]
    SourceFileNotFound { source_file_id: i64 },
    #[error(
        "source file {source_file_id} is not probeable because presence_state is {presence_state}"
    )]
    SourceFileUnavailable {
        source_file_id: i64,
        presence_state: String,
    },
    #[error("source file {source_file_id} has unsupported media file_kind {file_kind}")]
    UnsupportedMediaKind {
        source_file_id: i64,
        file_kind: String,
    },
    #[error("unsupported media format at {path:?}: {detail}")]
    UnsupportedFormat { path: PathBuf, detail: String },
    #[error("failed to open source file for media probe at {path:?}: {source}")]
    FileOpen {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read source file during media probe at {path:?}: {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to probe source file media at {path:?}: {detail}")]
    Probe { path: PathBuf, detail: String },
    #[error("source file {source_file_id} basis changed while probing media")]
    BasisChanged { source_file_id: i64 },
    #[error("inspect-source media probe work for source file {source_file_id} is already active")]
    WorkAlreadyActive { source_file_id: i64 },
}

type ProbeSourceFileMediaJobResult<T> = Result<T, ProbeSourceFileMediaError>;

impl SqliteDurableStore {
    pub fn read_source_file_media_probe_candidates(
        &self,
        input: ReadSourceFileMediaProbeCandidatesInput,
    ) -> LibrarySqliteResult<Vec<SourceFileMediaProbeCandidate>> {
        let effective_limit = effective_media_probe_batch_limit(input.limit);
        let connection = self.open_read_connection()?;
        read_source_file_media_probe_candidates_for_scope(
            &connection,
            &input.scope,
            effective_limit,
        )
    }

    pub fn count_source_file_media_probe_candidates(
        &self,
        scope: SourceFileMediaProbeAdmissionScope,
    ) -> LibrarySqliteResult<usize> {
        let connection = self.open_read_connection()?;
        count_source_file_media_probe_candidates_for_scope(&connection, &scope)
    }

    pub fn probe_source_file_media_batch(
        &self,
        input: ProbeSourceFileMediaBatchInput,
    ) -> LibrarySqliteResult<ProbeSourceFileMediaBatchResult> {
        let effective_limit = effective_media_probe_batch_limit(input.limit);
        let candidates = self.read_source_file_media_probe_candidates(
            ReadSourceFileMediaProbeCandidatesInput {
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
                    match self.probe_source_file_media(ProbeSourceFileMediaInput {
                        source_file_id,
                        source_file_path: resolved.path,
                        observed_at_ms: input.observed_at_ms,
                    }) {
                        Ok(result) => ProbeSourceFileMediaBatchOutcome {
                            source_file_id,
                            source_id,
                            relative_path,
                            status: ProbeSourceFileMediaBatchOutcomeStatus::Probed {
                                observations: result.observations,
                                accepted_artifact_id: result.accepted_artifact_id,
                                work_item_id: result.work_item_id,
                            },
                        },
                        Err(ProbeSourceFileMediaError::WorkAlreadyActive { .. }) => {
                            ProbeSourceFileMediaBatchOutcome {
                                source_file_id,
                                source_id,
                                relative_path,
                                status: ProbeSourceFileMediaBatchOutcomeStatus::Skipped {
                                    reason: SourceFileMediaProbeSkipReason::WorkAlreadyActive,
                                },
                            }
                        }
                        Err(error) => ProbeSourceFileMediaBatchOutcome {
                            source_file_id,
                            source_id,
                            relative_path,
                            status: ProbeSourceFileMediaBatchOutcomeStatus::Failed {
                                failure: SourceFileMediaProbeFailure::from(error),
                            },
                        },
                    }
                }
                Err(error) => {
                    source_scope_unavailable = is_source_scope_unavailable_resolution_error(&error)
                        && matches!(
                            input.scope,
                            SourceFileMediaProbeAdmissionScope::Source { .. }
                        );
                    ProbeSourceFileMediaBatchOutcome {
                        source_file_id,
                        source_id: candidate.source_id,
                        relative_path: candidate.relative_path,
                        status: ProbeSourceFileMediaBatchOutcomeStatus::Failed {
                            failure: SourceFileMediaProbeFailure::from(error),
                        },
                    }
                }
            };

            outcomes.push(outcome);

            if source_scope_unavailable {
                break;
            }
        }

        let remaining_candidates = count_source_file_media_probe_candidates_for_scope(
            &self.open_read_connection()?,
            &input.scope,
        )?;
        let probed_count = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.status,
                    ProbeSourceFileMediaBatchOutcomeStatus::Probed { .. }
                )
            })
            .count();
        let skipped_count = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.status,
                    ProbeSourceFileMediaBatchOutcomeStatus::Skipped { .. }
                )
            })
            .count();
        let failed_count = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.status,
                    ProbeSourceFileMediaBatchOutcomeStatus::Failed { .. }
                )
            })
            .count();

        Ok(ProbeSourceFileMediaBatchResult {
            requested_limit: input.limit,
            effective_limit,
            outcomes,
            probed_count,
            skipped_count,
            failed_count,
            remaining_candidates,
        })
    }

    fn probe_source_file_media(
        &self,
        input: ProbeSourceFileMediaInput,
    ) -> ProbeSourceFileMediaJobResult<ProbeSourceFileMediaResult> {
        self.probe_source_file_media_with_after_probe(input, || Ok(()))
    }

    fn probe_source_file_media_with_after_probe<F>(
        &self,
        input: ProbeSourceFileMediaInput,
        after_probe: F,
    ) -> ProbeSourceFileMediaJobResult<ProbeSourceFileMediaResult>
    where
        F: FnOnce() -> ProbeSourceFileMediaJobResult<()>,
    {
        let initial_basis = self.load_probeable_source_file_basis(input.source_file_id)?;
        if initial_basis.file_kind != "audio" {
            return Err(ProbeSourceFileMediaError::UnsupportedMediaKind {
                source_file_id: input.source_file_id.get(),
                file_kind: initial_basis.file_kind,
            });
        }
        let observations =
            probe_audio_metadata(&input.source_file_path, &initial_basis.relative_path)?;
        after_probe()?;

        let file_store_root = self.app_owned_state.artifact_file_store_root().clone();
        self.with_write(|write| {
            let current_basis = load_source_file_hash_basis(write, input.source_file_id)?;
            if current_basis != initial_basis {
                return Err(LibrarySqliteError::WriteInvariant(
                    "source file basis changed while probing media".to_string(),
                ));
            }

            let result = commit_media_probe_observation(
                write,
                file_store_root.clone(),
                &initial_basis,
                &observations,
                input.observed_at_ms,
            )?;
            Ok(result)
        })
        .map_err(|error| match error {
            LibrarySqliteError::WriteInvariant(detail)
                if detail == "source file basis changed while probing media" =>
            {
                ProbeSourceFileMediaError::BasisChanged {
                    source_file_id: input.source_file_id.get(),
                }
            }
            LibrarySqliteError::WriteInvariant(detail)
                if detail == "inspect-source media probe work is already active" =>
            {
                ProbeSourceFileMediaError::WorkAlreadyActive {
                    source_file_id: input.source_file_id.get(),
                }
            }
            other => ProbeSourceFileMediaError::Store(other),
        })
    }

    fn load_probeable_source_file_basis(
        &self,
        source_file_id: SourceFileId,
    ) -> ProbeSourceFileMediaJobResult<SourceFileHashBasis> {
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
            .ok_or(ProbeSourceFileMediaError::SourceFileNotFound {
                source_file_id: source_file_id.get(),
            })?;

        if basis.presence_state != "present" {
            return Err(ProbeSourceFileMediaError::SourceFileUnavailable {
                source_file_id: source_file_id.get(),
                presence_state: basis.presence_state,
            });
        }

        Ok(basis)
    }
}

pub fn effective_media_probe_batch_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_MEDIA_PROBE_BATCH_LIMIT)
        .clamp(1, MAX_MEDIA_PROBE_BATCH_LIMIT)
}

fn read_source_file_media_probe_candidates_for_scope(
    connection: &rusqlite::Connection,
    scope: &SourceFileMediaProbeAdmissionScope,
    limit: usize,
) -> LibrarySqliteResult<Vec<SourceFileMediaProbeCandidate>> {
    let (scope_predicate, mut values) = media_probe_candidate_scope_predicate(scope);
    values.push(Value::Integer(i64::try_from(limit).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "media probe candidate limit {limit} exceeds i64 range"
        ))
    })?));
    let sql = format!(
        "SELECT sf.source_file_id,
                sf.source_id,
                sf.relative_path,
                sf.file_class,
                sf.file_kind,
                CASE
                    WHEN observations.source_file_id IS NULL THEN 'missing_observations'
                    WHEN NOT (
                        sf.source_id = observations.basis_source_id
                        AND sf.relative_path = observations.basis_relative_path
                        AND sf.size_bytes IS observations.basis_size_bytes
                        AND sf.mtime_ns IS observations.basis_mtime_ns
                        AND sf.presence_state = observations.basis_presence_state
                    ) THEN 'stale_observations'
                    ELSE 'missing_probe_fields'
                END AS reason
         FROM source_files sf
         LEFT JOIN source_file_observations observations
           ON observations.source_file_id = sf.source_file_id
         WHERE {scope_predicate}
           AND sf.presence_state = 'present'
           AND sf.file_class = 'audio'
           AND {}
          ORDER BY lower(sf.relative_path) ASC,
                  sf.source_file_id ASC
         LIMIT ?{}",
        needs_media_probe_predicate_sql("sf", "observations"),
        values.len()
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            let source_file_id = row.get::<_, i64>(0)?;
            Ok(SourceFileMediaProbeCandidate {
                source_file_id: SourceFileId::new(source_file_id)
                    .ok_or_else(|| rusqlite::Error::IntegralValueOutOfRange(0, source_file_id))?,
                source_id: row.get(1)?,
                relative_path: row.get(2)?,
                file_class: row.get(3)?,
                file_kind: row.get(4)?,
                reason: parse_media_probe_candidate_reason(&row.get::<_, String>(5)?).map_err(
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

fn count_source_file_media_probe_candidates_for_scope(
    connection: &rusqlite::Connection,
    scope: &SourceFileMediaProbeAdmissionScope,
) -> LibrarySqliteResult<usize> {
    let (scope_predicate, values) = media_probe_candidate_scope_predicate(scope);
    let sql = format!(
        "SELECT COUNT(*)
         FROM source_files sf
         LEFT JOIN source_file_observations observations
           ON observations.source_file_id = sf.source_file_id
                   WHERE {scope_predicate}
           AND sf.presence_state = 'present'
           AND sf.file_class = 'audio'
           AND {}",
        needs_media_probe_predicate_sql("sf", "observations")
    );
    let count = connection.query_row(&sql, params_from_iter(values.iter()), |row| {
        row.get::<_, i64>(0)
    })?;
    usize::try_from(count).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "media probe candidate count {count} cannot fit in usize"
        ))
    })
}

fn media_probe_candidate_scope_predicate(
    scope: &SourceFileMediaProbeAdmissionScope,
) -> (String, Vec<Value>) {
    match scope {
        SourceFileMediaProbeAdmissionScope::Source { source_id } => (
            "sf.source_id = ?1".to_string(),
            vec![Value::Integer(*source_id)],
        ),
        SourceFileMediaProbeAdmissionScope::SourceFiles { source_file_ids } => {
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
                .map(|index| format!("?{}", index + 1))
                .collect::<Vec<_>>()
                .join(", ");
            (
                format!("sf.source_file_id IN ({placeholders})"),
                ids.into_iter().map(Value::Integer).collect(),
            )
        }
    }
}

fn needs_media_probe_predicate_sql(source_file_alias: &str, observations_alias: &str) -> String {
    format!(
        "({observations_alias}.source_file_id IS NULL
          OR NOT (
              {source_file_alias}.source_id = {observations_alias}.basis_source_id
              AND {source_file_alias}.relative_path = {observations_alias}.basis_relative_path
              AND {source_file_alias}.size_bytes IS {observations_alias}.basis_size_bytes
              AND {source_file_alias}.mtime_ns IS {observations_alias}.basis_mtime_ns
              AND {source_file_alias}.presence_state = {observations_alias}.basis_presence_state
          )
          OR (
              {observations_alias}.mime_type IS NULL
              AND {observations_alias}.duration_ms IS NULL
              AND {observations_alias}.sample_rate_hz IS NULL
              AND {observations_alias}.channels IS NULL
              AND {observations_alias}.bit_depth IS NULL
              AND {observations_alias}.codec IS NULL
          ))"
    )
}

fn parse_media_probe_candidate_reason(
    reason: &str,
) -> Result<SourceFileMediaProbeCandidateReason, String> {
    match reason {
        "missing_observations" => Ok(SourceFileMediaProbeCandidateReason::MissingObservations),
        "missing_probe_fields" => Ok(SourceFileMediaProbeCandidateReason::MissingProbeFields),
        "stale_observations" => Ok(SourceFileMediaProbeCandidateReason::StaleObservations),
        other => Err(format!("unknown media probe candidate reason {other:?}")),
    }
}

fn probe_audio_metadata(
    path: &Path,
    relative_path: &str,
) -> ProbeSourceFileMediaJobResult<SourceFileMediaProbeObservations> {
    let file = File::open(path).map_err(|source| ProbeSourceFileMediaError::FileOpen {
        path: path.to_path_buf(),
        source,
    })?;
    let mss = MediaSourceStream::new(
        Box::new(file),
        MediaSourceStreamOptions {
            buffer_len: MEDIA_PROBE_READ_BUFFER_BYTES,
        },
    );
    let mut hint = Hint::new();
    if let Some(extension) = Path::new(relative_path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        hint.with_extension(extension);
    }

    let probed = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|source| map_symphonia_probe_error(path, source))?;
    let format = probed;
    let container_name = container_name_for_path(relative_path);
    let track = format
        .default_track(TrackType::Audio)
        .or_else(|| {
            format.tracks().iter().find(|track| {
                track
                    .codec_params
                    .as_ref()
                    .and_then(CodecParameters::audio)
                    .is_some_and(|params| params.codec != CODEC_ID_NULL_AUDIO)
            })
        })
        .ok_or_else(|| ProbeSourceFileMediaError::UnsupportedFormat {
            path: path.to_path_buf(),
            detail: "no supported audio track".to_string(),
        })?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(CodecParameters::audio)
        .ok_or_else(|| ProbeSourceFileMediaError::UnsupportedFormat {
            path: path.to_path_buf(),
            detail: "no supported audio track".to_string(),
        })?;

    Ok(SourceFileMediaProbeObservations {
        media_kind: "audio".to_string(),
        mime_type: mime_type_for_format(&container_name, relative_path).map(str::to_string),
        duration_ms: duration_ms_from_track(track),
        sample_rate_hz: params.sample_rate.map(i64::from),
        channels: params
            .channels
            .as_ref()
            .and_then(|channels| i64::try_from(channels.count()).ok()),
        bit_depth: params.bits_per_sample.map(i64::from),
        codec: codec_label_for_format(&container_name, params),
    })
}

fn map_symphonia_probe_error(path: &Path, source: SymphoniaError) -> ProbeSourceFileMediaError {
    match source {
        SymphoniaError::IoError(source) => ProbeSourceFileMediaError::FileRead {
            path: path.to_path_buf(),
            source,
        },
        SymphoniaError::Unsupported(detail) => ProbeSourceFileMediaError::UnsupportedFormat {
            path: path.to_path_buf(),
            detail: detail.to_string(),
        },
        other => ProbeSourceFileMediaError::Probe {
            path: path.to_path_buf(),
            detail: other.to_string(),
        },
    }
}

fn duration_ms_from_track(track: &Track) -> Option<i64> {
    let duration = track
        .duration
        .or_else(|| track.num_frames.map(symphonia::core::units::Duration::new))?;
    let time_base = track.time_base?;
    let timestamp = symphonia::core::units::Timestamp::try_from(duration.get()).ok()?;
    let time = time_base.calc_time(timestamp)?;
    i64::try_from(time.as_millis()).ok()
}

fn mime_type_for_format(format_name: &str, relative_path: &str) -> Option<&'static str> {
    match format_name {
        "aiff" => Some("audio/aiff"),
        "flac" => Some("audio/flac"),
        "isomp4" | "mp4" => Some("audio/mp4"),
        "mp3" => Some("audio/mpeg"),
        "ogg" => Some("audio/ogg"),
        "wav" | "wave" => Some("audio/wav"),
        _ => mime_type_for_extension(relative_path),
    }
}

fn container_name_for_path(relative_path: &str) -> String {
    Path::new(relative_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| "unknown".to_string())
}

fn mime_type_for_extension(relative_path: &str) -> Option<&'static str> {
    match Path::new(relative_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("aif" | "aiff" | "aifc") => Some("audio/aiff"),
        Some("flac") => Some("audio/flac"),
        Some("m4a" | "mp4" | "aac" | "alac") => Some("audio/mp4"),
        Some("mp3") => Some("audio/mpeg"),
        Some("ogg" | "oga" | "opus") => Some("audio/ogg"),
        Some("wav") => Some("audio/wav"),
        _ => None,
    }
}

fn codec_label_for_format(format_name: &str, params: &AudioCodecParameters) -> Option<String> {
    let label = match format_name {
        "aiff" | "wav" | "wave" => "pcm",
        "flac" => "flac",
        "isomp4" | "mp4" => "isomp4",
        "mp3" => "mp3",
        "ogg" => "ogg",
        _ if params.codec != CODEC_ID_NULL_AUDIO => return Some(format!("{:?}", params.codec)),
        _ => return None,
    };
    Some(label.to_string())
}

fn commit_media_probe_observation(
    write: &mut AdmittedWrite<'_>,
    file_store_root: crate::authority::work::ArtifactFileStoreRoot,
    basis: &SourceFileHashBasis,
    observations: &SourceFileMediaProbeObservations,
    observed_at_ms: i64,
) -> LibrarySqliteResult<ProbeSourceFileMediaResult> {
    let basis_fingerprint = basis.basis_fingerprint();
    let queued = WorkItemAuthorityTx::new(write).queue_inspect_source_file_work(
        &QueueInspectSourceFileWorkInput {
            source_file_id: basis.source_file_id,
            basis_fingerprint: basis_fingerprint.clone(),
            priority_class: WorkPriorityClass::Interactive,
            queued_at: observed_at_ms,
        },
    )?;
    if queued.state != WorkItemState::Queued {
        return Err(LibrarySqliteError::WriteInvariant(
            "inspect-source media probe work is already active".to_string(),
        ));
    }
    let claimed = WorkItemAuthorityTx::new(write).claim_specific_machine_work(
        &ClaimSpecificMachineWorkInput {
            work_item_id: queued.work_item_id,
            lease_duration_ms: MEDIA_PROBE_JOB_LEASE_DURATION_MS,
            claimed_at: observed_at_ms,
        },
    )?;
    let work_run = WorkRunAuthorityTx::new(write).start_work_run(&StartWorkRunInput {
        work_item_id: claimed.work_item_id,
        adapter_key: MEDIA_PROBE_JOB_ADAPTER_KEY.to_string(),
        adapter_version: MEDIA_PROBE_JOB_ADAPTER_VERSION.to_string(),
        started_at: observed_at_ms,
    })?;

    let payload = serde_json::json!({
        "adapterKey": MEDIA_PROBE_JOB_ADAPTER_KEY,
        "adapterVersion": MEDIA_PROBE_JOB_ADAPTER_VERSION,
        "sourceFileId": basis.source_file_id.get(),
        "basisFingerprint": basis_fingerprint,
        "mediaKind": observations.media_kind,
        "mimeType": observations.mime_type,
        "durationMs": observations.duration_ms,
        "sampleRateHz": observations.sample_rate_hz,
        "channels": observations.channels,
        "bitDepth": observations.bit_depth,
        "codec": observations.codec,
    })
    .to_string()
    .into_bytes();
    let payload_hash = format!("blake3:{}", blake3::hash(&payload).to_hex());
    let artifact =
        WorkArtifactAuthorityTx::new(write).record_inline_artifact(&RecordInlineArtifactInput {
            artifact: RecordArtifactInput {
                work_run_id: work_run.work_run_id,
                artifact_kind: ArtifactKind::InspectionResult,
                media_type: "application/json".to_string(),
                basis_fingerprint: basis_fingerprint.clone(),
                payload_hash,
                created_at: observed_at_ms,
            },
            payload,
        })?;

    InspectSourceFilePromotionTx::new(write, file_store_root).inspect_source_file(
        &InspectSourceFilePromotionInput {
            source_file_observations: CommitAcceptedSourceFileObservationInput {
                source_file_id: basis.source_file_id,
                accepted_artifact_id: artifact.artifact_id,
                basis_fingerprint,
                observed_at_ms,
                content_hash: None,
                media_kind: observations.media_kind.clone(),
                mime_type: observations.mime_type.clone(),
                duration_ms: observations.duration_ms,
                sample_rate_hz: observations.sample_rate_hz,
                channels: observations.channels,
                bit_depth: observations.bit_depth,
                codec: observations.codec.clone(),
                updated_at: observed_at_ms,
            },
            source_file_observation_merge_policy:
                CommitAcceptedSourceFileObservationMergePolicy::preserve_current_content_hash(),
            rebuild_projection_domains: vec![],
            rebuild_priority: WorkPriorityClass::Interactive,
        },
    )?;

    WorkRunAuthorityTx::new(write).finish_work_run(&FinishWorkRunInput {
        work_run_id: work_run.work_run_id,
        finished_at: observed_at_ms,
        outcome: WorkRunOutcome::Completed,
        failure_kind: None,
        error_detail: None,
    })?;
    WorkItemAuthorityTx::new(write).complete_machine_work_item(&CompleteMachineWorkInput {
        work_item_id: claimed.work_item_id,
        completed_at: observed_at_ms,
    })?;

    Ok(ProbeSourceFileMediaResult {
        source_file_id: basis.source_file_id,
        observations: observations.clone(),
        accepted_artifact_id: artifact.artifact_id.get(),
        work_item_id: claimed.work_item_id.get(),
    })
}

impl From<ResolveSourceFilePathError> for SourceFileMediaProbeFailure {
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

impl From<ProbeSourceFileMediaError> for SourceFileMediaProbeFailure {
    fn from(error: ProbeSourceFileMediaError) -> Self {
        match error {
            ProbeSourceFileMediaError::Store(error) => Self::Store {
                detail: error.to_string(),
            },
            ProbeSourceFileMediaError::SourceFileNotFound { .. } => Self::SourceFileNotFound,
            ProbeSourceFileMediaError::SourceFileUnavailable { presence_state, .. } => {
                Self::SourceFileUnavailable { presence_state }
            }
            ProbeSourceFileMediaError::UnsupportedMediaKind { file_kind, .. } => {
                Self::UnsupportedMediaKind { file_kind }
            }
            ProbeSourceFileMediaError::UnsupportedFormat { detail, .. } => {
                Self::UnsupportedFormat { detail }
            }
            ProbeSourceFileMediaError::FileOpen { path, source } => Self::FileOpen {
                path,
                detail: source.to_string(),
            },
            ProbeSourceFileMediaError::FileRead { path, source } => Self::FileRead {
                path,
                detail: source.to_string(),
            },
            ProbeSourceFileMediaError::Probe { detail, .. } => Self::Probe { detail },
            ProbeSourceFileMediaError::BasisChanged { .. } => Self::BasisChanged,
            ProbeSourceFileMediaError::WorkAlreadyActive { .. } => Self::Store {
                detail: "inspect-source media probe work is already active".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use library_domain::SourceFileId;
    use rusqlite::params;
    use tempfile::TempDir;

    use crate::read_models::attachment_identity::{
        StoreSourceFileAttachmentLinkStatus, get_attachment_for_source_file,
    };
    use crate::read_models::source_file_observations::{
        StoreSourceFileObservationStatus, read_source_file_observation,
    };
    use crate::store::source_file_hash::{
        HashSourceFileBlake3BatchInput, HashSourceFileBlake3BatchOutcomeStatus,
        SourceFileBlake3HashAdmissionScope,
    };
    use crate::store::{SqliteDurableStore, SqliteDurableStoreAppOwnedState};

    use super::{
        ProbeSourceFileMediaBatchInput, ProbeSourceFileMediaBatchOutcomeStatus,
        ProbeSourceFileMediaError, ProbeSourceFileMediaInput,
        ReadSourceFileMediaProbeCandidatesInput, SourceFileMediaProbeAdmissionScope,
        SourceFileMediaProbeCandidateReason, SourceFileMediaProbeFailure,
    };

    struct MediaProbeFixture {
        tempdir: TempDir,
        store: SqliteDurableStore,
    }

    impl MediaProbeFixture {
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
                         VALUES (1, 'internal', 'system', 'source:media-probe:test', 'Probe Test', 1, 1, 1)",
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
            let name_sort_key = crate::browse_sort_key::compute_name_sort_key(name);
            let path_sort_key = crate::browse_sort_key::compute_path_sort_key(relative_path);
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO source_files (
                             source_file_id,
                             source_id,
                             parent_source_directory_id,
                             name,
                             name_sort_key,
                             path_sort_key,
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
                            name_sort_key,
                            path_sort_key,
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

        fn mark_source_file_basis_changed(
            &self,
            source_file_id: i64,
            size_bytes: i64,
            mtime_ns: i64,
            updated_at: i64,
        ) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "UPDATE source_files
                         SET size_bytes = ?2,
                             mtime_ns = ?3,
                             updated_at = ?4
                         WHERE source_file_id = ?1",
                        params![source_file_id, size_bytes, mtime_ns, updated_at],
                    )?;
                    Ok(())
                })
                .expect("update source file basis");
        }

        fn read_candidates_for_source(
            &self,
            limit: usize,
        ) -> Vec<super::SourceFileMediaProbeCandidate> {
            self.store
                .read_source_file_media_probe_candidates(ReadSourceFileMediaProbeCandidatesInput {
                    scope: SourceFileMediaProbeAdmissionScope::Source { source_id: 1 },
                    limit: Some(limit),
                })
                .expect("read media probe candidates")
        }

        fn run_probe_batch_for_source_files(
            &self,
            source_file_ids: &[i64],
            limit: usize,
        ) -> super::ProbeSourceFileMediaBatchResult {
            self.store
                .probe_source_file_media_batch(ProbeSourceFileMediaBatchInput {
                    scope: SourceFileMediaProbeAdmissionScope::SourceFiles {
                        source_file_ids: source_file_ids
                            .iter()
                            .map(|id| SourceFileId::new(*id).expect("positive source file id"))
                            .collect(),
                    },
                    limit: Some(limit),
                    observed_at_ms: 40,
                })
                .expect("run media probe batch")
        }

        fn hash_source_file(&self, source_file_id: i64) {
            let result = self
                .store
                .hash_source_file_blake3_batch(HashSourceFileBlake3BatchInput {
                    scope: SourceFileBlake3HashAdmissionScope::SourceFiles {
                        source_file_ids: vec![
                            SourceFileId::new(source_file_id).expect("positive source file id"),
                        ],
                    },
                    limit: Some(10),
                    observed_at_ms: 30,
                })
                .expect("hash source file");
            assert_eq!(result.hashed_count, 1);
            assert!(matches!(
                result.outcomes[0].status,
                HashSourceFileBlake3BatchOutcomeStatus::Hashed { .. }
            ));
        }

        fn observations(&self, source_file_id: i64) -> crate::StoreSourceFileObservation {
            let connection = self.store.open_read_connection().expect("open read");
            read_source_file_observation(&connection, source_file_id)
                .expect("read observations")
                .expect("observations exist")
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
    fn media_probe_records_wav_source_file_observations() {
        let fixture = MediaProbeFixture::new();
        let bytes = tiny_wav_bytes(44_100, 2, 16, 4_410);
        fixture.write_source_file(100, "Album/track.wav", &bytes);

        let result = fixture.run_probe_batch_for_source_files(&[100], 10);

        assert_eq!(result.probed_count, 1);
        assert_eq!(result.failed_count, 0);
        let ProbeSourceFileMediaBatchOutcomeStatus::Probed { observations, .. } =
            &result.outcomes[0].status
        else {
            panic!("expected probed outcome");
        };
        assert_eq!(observations.media_kind, "audio");
        assert_eq!(observations.mime_type.as_deref(), Some("audio/wav"));
        assert_eq!(observations.duration_ms, Some(100));
        assert_eq!(observations.sample_rate_hz, Some(44_100));
        assert_eq!(observations.channels, Some(2));
        assert_eq!(observations.bit_depth, Some(16));
        assert_eq!(observations.codec.as_deref(), Some("pcm"));

        let stored = fixture.observations(100);
        assert_eq!(stored.status, StoreSourceFileObservationStatus::Current);
        assert_eq!(stored.media_kind, "audio");
        assert_eq!(stored.mime_type.as_deref(), Some("audio/wav"));
        assert_eq!(stored.duration_ms, Some(100));
        assert_eq!(stored.sample_rate_hz, Some(44_100));
        assert_eq!(stored.channels, Some(2));
        assert_eq!(stored.bit_depth, Some(16));
        assert_eq!(stored.codec.as_deref(), Some("pcm"));
        assert_eq!(stored.content_hash, None);
    }

    #[test]
    fn unsupported_audio_bytes_return_typed_failure_and_write_no_observations() {
        let fixture = MediaProbeFixture::new();
        fixture.write_source_file(100, "Album/broken.wav", b"not a wave file");

        let result = fixture.run_probe_batch_for_source_files(&[100], 10);

        assert_eq!(result.probed_count, 0);
        assert_eq!(result.failed_count, 1);
        assert!(matches!(
            result.outcomes[0].status,
            ProbeSourceFileMediaBatchOutcomeStatus::Failed {
                failure: SourceFileMediaProbeFailure::UnsupportedFormat { .. }
                    | SourceFileMediaProbeFailure::Probe { .. }
                    | SourceFileMediaProbeFailure::FileRead { .. }
            }
        ));
        assert_eq!(fixture.count_rows("source_file_observations"), 0);
    }

    #[test]
    fn missing_physical_file_returns_typed_failure_and_writes_no_observations() {
        let fixture = MediaProbeFixture::new();
        fixture.insert_source_file(100, "Album/missing.wav", 12, 1000);

        let result = fixture.run_probe_batch_for_source_files(&[100], 10);

        assert_eq!(result.probed_count, 0);
        assert_eq!(result.failed_count, 1);
        assert!(matches!(
            result.outcomes[0].status,
            ProbeSourceFileMediaBatchOutcomeStatus::Failed {
                failure: SourceFileMediaProbeFailure::PhysicalFileMissing { .. }
            }
        ));
        assert_eq!(fixture.count_rows("source_file_observations"), 0);
    }

    #[test]
    fn basis_change_during_probe_rejects_commit() {
        let fixture = MediaProbeFixture::new();
        let bytes = tiny_wav_bytes(44_100, 1, 16, 4_410);
        let path = fixture.write_source_file(100, "Album/changing.wav", &bytes);

        let error = fixture
            .store
            .probe_source_file_media_with_after_probe(
                ProbeSourceFileMediaInput {
                    source_file_id: SourceFileId::new(100).expect("positive source file"),
                    source_file_path: path,
                    observed_at_ms: 40,
                },
                || {
                    fixture
                        .store
                        .with_write(|write| {
                            write.execute(
                                "UPDATE source_files
                                 SET size_bytes = 99,
                                     mtime_ns = 2000,
                                     updated_at = 50
                                 WHERE source_file_id = 100",
                                [],
                            )?;
                            Ok(())
                        })
                        .map_err(ProbeSourceFileMediaError::Store)
                },
            )
            .expect_err("basis change must reject probe commit");

        assert!(matches!(
            error,
            ProbeSourceFileMediaError::BasisChanged { .. }
        ));
        assert_eq!(fixture.count_rows("source_file_observations"), 0);
    }

    #[test]
    fn current_blake3_evidence_survives_media_probe_commit() {
        let fixture = MediaProbeFixture::new();
        let bytes = tiny_wav_bytes(44_100, 2, 16, 4_410);
        fixture.write_source_file(100, "Album/track.wav", &bytes);
        fixture.hash_source_file(100);
        let hash_before = fixture.observations(100).content_hash.expect("hash before");

        fixture.run_probe_batch_for_source_files(&[100], 10);

        let observations = fixture.observations(100);
        assert_eq!(
            observations.status,
            StoreSourceFileObservationStatus::Current
        );
        assert_eq!(observations.content_hash, Some(hash_before));
        assert_eq!(observations.duration_ms, Some(100));
        assert_eq!(observations.sample_rate_hz, Some(44_100));
    }

    #[test]
    fn stale_blake3_evidence_is_not_resurrected_by_media_probe_commit() {
        let fixture = MediaProbeFixture::new();
        let bytes = tiny_wav_bytes(44_100, 1, 16, 4_410);
        fixture.write_source_file(100, "Album/stale.wav", &bytes);
        fixture.hash_source_file(100);
        fixture.mark_source_file_basis_changed(100, bytes.len() as i64 + 1, 2000, 50);

        fixture.run_probe_batch_for_source_files(&[100], 10);

        let observations = fixture.observations(100);
        assert_eq!(
            observations.status,
            StoreSourceFileObservationStatus::Current
        );
        assert_eq!(observations.content_hash, None);
        assert_eq!(observations.duration_ms, Some(100));
    }

    #[test]
    fn blake3_commit_after_probe_preserves_current_probe_fields() {
        let fixture = MediaProbeFixture::new();
        let bytes = tiny_wav_bytes(48_000, 2, 16, 4_800);
        fixture.write_source_file(100, "Album/probe-first.wav", &bytes);
        fixture.run_probe_batch_for_source_files(&[100], 10);

        fixture.hash_source_file(100);

        let observations = fixture.observations(100);
        assert_eq!(
            observations.status,
            StoreSourceFileObservationStatus::Current
        );
        assert_eq!(
            observations
                .content_hash
                .expect("hash after probe")
                .algorithm,
            "blake3"
        );
        assert_eq!(observations.mime_type.as_deref(), Some("audio/wav"));
        assert_eq!(observations.duration_ms, Some(100));
        assert_eq!(observations.sample_rate_hz, Some(48_000));
        assert_eq!(observations.channels, Some(2));
        assert_eq!(observations.bit_depth, Some(16));
    }

    #[test]
    fn cue_files_are_not_probe_candidates_or_paired_with_audio() {
        let fixture = MediaProbeFixture::new();
        let bytes = tiny_wav_bytes(44_100, 1, 16, 4_410);
        fixture.write_source_file(100, "Album/track.wav", &bytes);
        let cue_path = fixture.write_source_file(101, "Album/album.cue", b"FILE track.wav WAVE");

        let cue_only = fixture.run_probe_batch_for_source_files(&[101], 10);
        assert!(cue_only.outcomes.is_empty());
        let candidates = fixture.read_candidates_for_source(10);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.source_file_id.get())
                .collect::<Vec<_>>(),
            vec![100]
        );

        let direct_error = fixture
            .store
            .probe_source_file_media(ProbeSourceFileMediaInput {
                source_file_id: SourceFileId::new(101).expect("positive source file"),
                source_file_path: cue_path,
                observed_at_ms: 40,
            })
            .expect_err("cue media probe is unsupported");
        assert!(matches!(
            direct_error,
            ProbeSourceFileMediaError::UnsupportedMediaKind { .. }
        ));
        assert!(
            fixture
                .store
                .read_source_file_observation(101)
                .expect("read cue observations")
                .is_none()
        );
        assert!(
            fixture
                .store
                .read_source_file_observation(100)
                .expect("read audio observations")
                .is_none()
        );
    }

    #[test]
    fn probing_writes_only_source_observations_and_artifacts() {
        let fixture = MediaProbeFixture::new();
        let bytes = tiny_wav_bytes(44_100, 2, 16, 4_410);
        fixture.write_source_file(100, "Album/track.wav", &bytes);

        fixture.run_probe_batch_for_source_files(&[100], 10);

        assert_eq!(fixture.count_rows("source_file_observations"), 1);
        assert_eq!(fixture.count_rows("work_artifacts"), 1);
        assert_eq!(fixture.count_rows("content_attachments"), 0);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 0);
        assert_eq!(fixture.count_rows("playable_media"), 0);
        assert_eq!(fixture.count_rows("track_identity_candidates"), 0);
    }

    #[test]
    fn probe_only_update_does_not_break_existing_attachment_link() {
        let fixture = MediaProbeFixture::new();
        let bytes = tiny_wav_bytes(44_100, 2, 16, 4_410);
        fixture.write_source_file(100, "Album/track.wav", &bytes);
        fixture.hash_source_file(100);
        let materialized = fixture
            .store
            .materialize_attachments_for_source(1, 10)
            .expect("materialize attachment");
        assert_eq!(materialized.links_created, 1);

        fixture.run_probe_batch_for_source_files(&[100], 10);

        let connection = fixture.store.open_read_connection().expect("open read");
        let link = get_attachment_for_source_file(&connection, 100)
            .expect("read source-file attachment")
            .expect("attachment link exists");
        assert_eq!(
            link.link_status,
            StoreSourceFileAttachmentLinkStatus::Current
        );
        assert_eq!(fixture.count_rows("content_attachments"), 1);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 1);
    }

    #[test]
    fn source_scope_candidate_admission_is_bounded_and_audio_only() {
        let fixture = MediaProbeFixture::new();
        fixture.write_source_file(100, "Album/audio.wav", &tiny_wav_bytes(44_100, 1, 16, 441));
        fixture.write_source_file(101, "Album/video.mp4", b"video");
        fixture.write_source_file(102, "Album/cover.jpg", b"image");
        fixture.write_source_file(103, "Album/album.cue", b"cue");
        fixture.write_source_file(104, "Album/notes.txt", b"text");

        let candidates = fixture.read_candidates_for_source(2);

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.source_file_id.get())
                .collect::<Vec<_>>(),
            vec![100]
        );
        assert_eq!(
            candidates[0].reason,
            SourceFileMediaProbeCandidateReason::MissingObservations
        );
    }

    #[test]
    fn source_file_id_list_admission_excludes_video() {
        let fixture = MediaProbeFixture::new();
        fixture.write_source_file(100, "Album/track.wav", &tiny_wav_bytes(44_100, 1, 16, 441));
        fixture.write_source_file(101, "Album/video.mp4", b"video");

        let result = fixture.run_probe_batch_for_source_files(&[100, 101], 10);

        assert_eq!(result.probed_count, 1);
        assert_eq!(result.failed_count, 0);
        assert_eq!(result.outcomes.len(), 1);
        let ProbeSourceFileMediaBatchOutcomeStatus::Probed { observations, .. } =
            &result.outcomes[0].status
        else {
            panic!("expected probed outcome for audio");
        };
        assert_eq!(observations.media_kind, "audio");
    }

    #[test]
    fn direct_video_probe_returns_unsupported_and_writes_no_observations() {
        let fixture = MediaProbeFixture::new();
        let video_path = fixture.write_source_file(101, "Album/video.mp4", b"video");

        let error = fixture
            .store
            .probe_source_file_media(ProbeSourceFileMediaInput {
                source_file_id: SourceFileId::new(101).expect("positive source file"),
                source_file_path: video_path,
                observed_at_ms: 40,
            })
            .expect_err("direct video probe is unsupported");

        assert!(matches!(
            error,
            ProbeSourceFileMediaError::UnsupportedMediaKind { file_kind, .. }
            if file_kind == "video"
        ));
        assert_eq!(fixture.count_rows("source_file_observations"), 0);
        assert_eq!(fixture.count_rows("work_artifacts"), 0);
    }

    #[test]
    fn bounded_admission_capacity_not_consumed_by_video_rows() {
        let fixture = MediaProbeFixture::new();
        fixture.write_source_file(100, "Album/video.mp4", b"video");
        fixture.write_source_file(101, "Album/video2.mkv", b"video");
        fixture.write_source_file(102, "Album/video3.avi", b"video");

        let candidates = fixture.read_candidates_for_source(10);

        assert!(
            candidates.is_empty(),
            "video rows must not consume bounded probe admission capacity"
        );
        let remaining = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .query_row(
                "SELECT COUNT(*) FROM source_files sf
                 LEFT JOIN source_file_observations observations ON observations.source_file_id = sf.source_file_id
                 WHERE sf.source_id = 1
                   AND sf.presence_state = 'present'
                   AND sf.file_class = 'audio'
                   AND (observations.source_file_id IS NULL
                        OR NOT (
                            sf.source_id = observations.basis_source_id
                            AND sf.relative_path = observations.basis_relative_path
                            AND sf.size_bytes IS observations.basis_size_bytes
                            AND sf.mtime_ns IS observations.basis_mtime_ns
                            AND sf.presence_state = observations.basis_presence_state
                        )
                        OR (
                            observations.mime_type IS NULL
                            AND observations.duration_ms IS NULL
                            AND observations.sample_rate_hz IS NULL
                            AND observations.channels IS NULL
                            AND observations.bit_depth IS NULL
                            AND observations.codec IS NULL
                        ))",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count remaining candidates");
        assert_eq!(
            remaining, 0,
            "video-less source must report zero candidates"
        );
    }

    fn tiny_wav_bytes(
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
        frames: u32,
    ) -> Vec<u8> {
        let bytes_per_sample = bits_per_sample / 8;
        let block_align = channels * bytes_per_sample;
        let byte_rate = sample_rate * u32::from(block_align);
        let data_len = frames * u32::from(block_align);
        let riff_len = 36 + data_len;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_len.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.resize(
            bytes.len() + usize::try_from(data_len).expect("data length fits usize"),
            0,
        );
        bytes
    }

    fn file_kind_for_path(path: &str) -> &'static str {
        crate::browse_media::file_kind_str_from_path(path)
    }

    fn file_class_for_path(path: &str) -> &'static str {
        crate::browse_media::file_class_str_from_path(path)
    }
}
