use rusqlite::{OptionalExtension, params};

use crate::authority::artifact_rows::require_source_artifact;
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{ArtifactId, SourceFileId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitAcceptedSourceFileObservationInput {
    pub source_file_id: SourceFileId,
    pub accepted_artifact_id: ArtifactId,
    pub basis_fingerprint: String,
    pub observed_at_ms: i64,
    pub content_hash: Option<ContentHashEvidence>,
    pub media_kind: String,
    pub mime_type: Option<String>,
    pub duration_ms: Option<i64>,
    pub sample_rate_hz: Option<i64>,
    pub channels: Option<i64>,
    pub bit_depth: Option<i64>,
    pub codec: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitAcceptedSourceFileObservationMergePolicy {
    pub preserve_current_content_hash_when_unspecified: bool,
    pub preserve_current_probe_fields_when_unspecified: bool,
}

impl CommitAcceptedSourceFileObservationMergePolicy {
    pub const fn replacement() -> Self {
        Self {
            preserve_current_content_hash_when_unspecified: false,
            preserve_current_probe_fields_when_unspecified: false,
        }
    }

    pub const fn preserve_current_content_hash() -> Self {
        Self {
            preserve_current_content_hash_when_unspecified: true,
            preserve_current_probe_fields_when_unspecified: false,
        }
    }

    pub const fn preserve_current_probe_fields() -> Self {
        Self {
            preserve_current_content_hash_when_unspecified: false,
            preserve_current_probe_fields_when_unspecified: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHashEvidence {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFileBasis {
    source_id: i64,
    relative_path: String,
    size_bytes: Option<i64>,
    mtime_ns: Option<i64>,
    presence_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentSourceFileObservationForMerge {
    content_hash: Option<ContentHashEvidence>,
    mime_type: Option<String>,
    duration_ms: Option<i64>,
    sample_rate_hz: Option<i64>,
    channels: Option<i64>,
    bit_depth: Option<i64>,
    codec: Option<String>,
}

pub struct SourceFileObservationAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceFileObservationAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn commit_accepted_source_file_observation_with_merge(
        &self,
        input: &CommitAcceptedSourceFileObservationInput,
        merge_policy: CommitAcceptedSourceFileObservationMergePolicy,
    ) -> LibrarySqliteResult<()> {
        validate_input(input)?;
        require_source_artifact(
            self.tx,
            input.accepted_artifact_id.get(),
            input.source_file_id.get(),
            "inspection_result",
            &input.basis_fingerprint,
        )?;
        let source_file_basis = load_source_file_basis(self.tx, input.source_file_id)?;
        let merged_input =
            merge_with_current_observation(self.tx, input, merge_policy, &source_file_basis)?;
        validate_input(&merged_input)?;
        let (content_hash_algorithm, content_hash_value) = merged_input
            .content_hash
            .as_ref()
            .map(|hash| (Some(hash.algorithm.as_str()), Some(hash.value.as_str())))
            .unwrap_or((None, None));

        self.tx.execute(
            "INSERT INTO source_file_observations (
                 source_file_id,
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
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
             ON CONFLICT(source_file_id) DO UPDATE
             SET basis_fingerprint = excluded.basis_fingerprint,
                 basis_source_id = excluded.basis_source_id,
                 basis_relative_path = excluded.basis_relative_path,
                 basis_size_bytes = excluded.basis_size_bytes,
                 basis_mtime_ns = excluded.basis_mtime_ns,
                 basis_presence_state = excluded.basis_presence_state,
                 observed_at_ms = excluded.observed_at_ms,
                 content_hash_algorithm = excluded.content_hash_algorithm,
                 content_hash_value = excluded.content_hash_value,
                 media_kind = excluded.media_kind,
                 mime_type = excluded.mime_type,
                 duration_ms = excluded.duration_ms,
                 sample_rate_hz = excluded.sample_rate_hz,
                 channels = excluded.channels,
                 bit_depth = excluded.bit_depth,
                 codec = excluded.codec,
                 updated_at = excluded.updated_at,
                 accepted_artifact_id = excluded.accepted_artifact_id",
            params![
                input.source_file_id.get(),
                input.basis_fingerprint,
                source_file_basis.source_id,
                source_file_basis.relative_path,
                source_file_basis.size_bytes,
                source_file_basis.mtime_ns,
                source_file_basis.presence_state,
                merged_input.observed_at_ms,
                content_hash_algorithm,
                content_hash_value,
                merged_input.media_kind,
                merged_input.mime_type,
                merged_input.duration_ms,
                merged_input.sample_rate_hz,
                merged_input.channels,
                merged_input.bit_depth,
                merged_input.codec,
                merged_input.updated_at,
                merged_input.accepted_artifact_id.get(),
            ],
        )?;
        Ok(())
    }
}

fn merge_with_current_observation(
    tx: &AdmittedWrite<'_>,
    input: &CommitAcceptedSourceFileObservationInput,
    merge_policy: CommitAcceptedSourceFileObservationMergePolicy,
    source_file_basis: &SourceFileBasis,
) -> LibrarySqliteResult<CommitAcceptedSourceFileObservationInput> {
    if !merge_policy.preserve_current_content_hash_when_unspecified
        && !merge_policy.preserve_current_probe_fields_when_unspecified
    {
        return Ok(input.clone());
    }

    let Some(current) = load_current_source_file_observation_for_merge(
        tx,
        input.source_file_id,
        source_file_basis,
    )?
    else {
        return Ok(input.clone());
    };

    let mut merged = input.clone();
    if merge_policy.preserve_current_content_hash_when_unspecified && merged.content_hash.is_none()
    {
        merged.content_hash = current.content_hash;
    }
    if merge_policy.preserve_current_probe_fields_when_unspecified {
        if merged.mime_type.is_none() {
            merged.mime_type = current.mime_type;
        }
        if merged.duration_ms.is_none() {
            merged.duration_ms = current.duration_ms;
        }
        if merged.sample_rate_hz.is_none() {
            merged.sample_rate_hz = current.sample_rate_hz;
        }
        if merged.channels.is_none() {
            merged.channels = current.channels;
        }
        if merged.bit_depth.is_none() {
            merged.bit_depth = current.bit_depth;
        }
        if merged.codec.is_none() {
            merged.codec = current.codec;
        }
    }

    Ok(merged)
}

fn validate_input(input: &CommitAcceptedSourceFileObservationInput) -> LibrarySqliteResult<()> {
    require_non_empty("basis_fingerprint", &input.basis_fingerprint)?;
    require_non_empty("media_kind", &input.media_kind)?;
    if input.updated_at < input.observed_at_ms {
        return Err(LibrarySqliteError::WriteInvariant(
            "source observations updated_at must be greater than or equal to observed_at_ms"
                .to_string(),
        ));
    }
    if let Some(content_hash) = &input.content_hash {
        require_non_empty("content_hash.algorithm", &content_hash.algorithm)?;
        require_non_empty("content_hash.value", &content_hash.value)?;
    }
    Ok(())
}

fn require_non_empty(field_name: &str, value: &str) -> LibrarySqliteResult<()> {
    if value.trim().is_empty() {
        Err(LibrarySqliteError::WriteInvariant(format!(
            "source observations {field_name} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn load_source_file_basis(
    tx: &AdmittedWrite<'_>,
    source_file_id: SourceFileId,
) -> LibrarySqliteResult<SourceFileBasis> {
    tx.query_row(
        "SELECT source_id,
                relative_path,
                size_bytes,
                mtime_ns,
                presence_state
         FROM source_files
         WHERE source_file_id = ?1",
        [source_file_id.get()],
        |row| {
            Ok(SourceFileBasis {
                source_id: row.get(0)?,
                relative_path: row.get(1)?,
                size_bytes: row.get(2)?,
                mtime_ns: row.get(3)?,
                presence_state: row.get(4)?,
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

fn load_current_source_file_observation_for_merge(
    tx: &AdmittedWrite<'_>,
    source_file_id: SourceFileId,
    source_file_basis: &SourceFileBasis,
) -> LibrarySqliteResult<Option<CurrentSourceFileObservationForMerge>> {
    tx.query_row(
        "SELECT content_hash_algorithm,
                content_hash_value,
                mime_type,
                duration_ms,
                sample_rate_hz,
                channels,
                bit_depth,
                codec
         FROM source_file_observations
         WHERE source_file_id = ?1
           AND basis_source_id = ?2
           AND basis_relative_path = ?3
           AND basis_size_bytes IS ?4
           AND basis_mtime_ns IS ?5
           AND basis_presence_state = ?6",
        params![
            source_file_id.get(),
            source_file_basis.source_id,
            source_file_basis.relative_path,
            source_file_basis.size_bytes,
            source_file_basis.mtime_ns,
            source_file_basis.presence_state,
        ],
        |row| {
            let content_hash_algorithm = row.get::<_, Option<String>>(0)?;
            let content_hash_value = row.get::<_, Option<String>>(1)?;
            Ok(CurrentSourceFileObservationForMerge {
                content_hash: content_hash_algorithm
                    .zip(content_hash_value)
                    .map(|(algorithm, value)| ContentHashEvidence { algorithm, value }),
                mime_type: row.get(2)?,
                duration_ms: row.get(3)?,
                sample_rate_hz: row.get(4)?,
                channels: row.get(5)?,
                bit_depth: row.get(6)?,
                codec: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use super::{
        CommitAcceptedSourceFileObservationInput, CommitAcceptedSourceFileObservationMergePolicy,
        ContentHashEvidence, SourceFileObservationAuthorityTx,
    };
    use crate::authority::write_lane::{AdmittedWrite, admit_write};
    use crate::read_models::source_file_observations::{
        StoreSourceFileObservationStatus, read_source_file_observation,
    };
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{ArtifactId, SourceFileId};

    fn install_test_baseline() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install canonical baseline");
        connection
    }

    fn insert_source(connection: &Connection) {
        connection
            .execute(
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
                 VALUES (1, 'internal', 'system', 'source:observations:test', 'Observations Test', 1, 1, 1)",
                [],
            )
            .expect("insert source");
    }

    fn insert_source_file(
        connection: &Connection,
        source_file_id: i64,
        relative_path: &str,
        size_bytes: i64,
        mtime_ns: i64,
    ) {
        let name = relative_path
            .rsplit('/')
            .next()
            .expect("relative path has file name");
        let name_sort_key = crate::browse_sort_key::compute_name_sort_key(name);
        let path_sort_key = crate::browse_sort_key::compute_path_sort_key(relative_path);
        connection
            .execute(
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
                 VALUES (?1, 1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, 'audio', 'audio', 'present', 10, 10, 10, 10, 10)",
                params![
                    source_file_id,
                    name,
                    name_sort_key,
                    path_sort_key,
                    relative_path,
                    size_bytes,
                    mtime_ns,
                ],
            )
            .expect("insert source file");
    }

    fn insert_artifact(
        write: &AdmittedWrite<'_>,
        artifact_id: i64,
        source_file_id: i64,
        basis_fingerprint: &str,
    ) {
        write
            .execute(
                "INSERT INTO work_items (
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
                 VALUES (?1, 'source_file', ?2, 'inspect_source_file', 'interactive', ?3, 'completed', 1, 20, 20)",
                params![artifact_id, source_file_id.to_string(), basis_fingerprint],
            )
            .expect("insert work item");
        write
            .execute(
                "INSERT INTO work_runs (
                     work_run_id,
                     work_item_id,
                     adapter_key,
                     adapter_version,
                     started_at,
                     finished_at,
                     outcome
                 )
                 VALUES (?1, ?1, 'test.adapter', '1', 21, 21, 'completed')",
                [artifact_id],
            )
            .expect("insert work run");
        write
            .execute(
                "INSERT INTO work_artifacts (
                     artifact_id,
                     work_run_id,
                     subject_kind,
                     subject_id,
                     artifact_kind,
                     adapter_key,
                     adapter_version,
                     basis_fingerprint,
                     media_type,
                     storage_kind,
                     payload_hash,
                     created_at
                 )
                 VALUES (?1, ?1, 'source_file', ?2, 'inspection_result', 'test.adapter', '1', ?3, 'application/json', 'inline_payload', ?4, 22)",
                params![
                    artifact_id,
                    source_file_id.to_string(),
                    basis_fingerprint,
                    format!("hash:artifact:{artifact_id}")
                ],
            )
            .expect("insert artifact");
    }

    fn commit_source_observation(
        connection: &mut Connection,
        artifact_id: i64,
        source_file_id: i64,
        basis_fingerprint: &str,
        media_kind: &str,
        content_hash: Option<ContentHashEvidence>,
    ) {
        admit_write(connection, |write| {
            insert_artifact(write, artifact_id, source_file_id, basis_fingerprint);
            SourceFileObservationAuthorityTx::new(write)
                .commit_accepted_source_file_observation_with_merge(
                    &CommitAcceptedSourceFileObservationInput {
                        source_file_id: SourceFileId::new(source_file_id)
                            .expect("positive source_file_id"),
                        accepted_artifact_id: ArtifactId::new(artifact_id)
                            .expect("positive artifact"),
                        basis_fingerprint: basis_fingerprint.to_string(),
                        observed_at_ms: 23,
                        content_hash,
                        media_kind: media_kind.to_string(),
                        mime_type: None,
                        duration_ms: None,
                        sample_rate_hz: None,
                        channels: None,
                        bit_depth: None,
                        codec: None,
                        updated_at: 24,
                    },
                    CommitAcceptedSourceFileObservationMergePolicy::replacement(),
                )
        })
        .expect("commit source observations");
    }

    #[test]
    fn accepted_hash_evidence_is_algorithm_tagged_and_basis_bound() {
        let mut connection = install_test_baseline();
        insert_source(&connection);
        insert_source_file(&connection, 100, "Album/track.flac", 123, 456);

        commit_source_observation(
            &mut connection,
            200,
            100,
            "basis:track:100",
            "audio",
            Some(ContentHashEvidence {
                algorithm: "sha256".to_string(),
                value: "fixture-digest".to_string(),
            }),
        );

        let observations = read_source_file_observation(&connection, 100)
            .expect("read source-file observations")
            .expect("observations exist");
        assert_eq!(observations.source_file_id, 100);
        assert_eq!(observations.basis_source_id, 1);
        assert_eq!(observations.basis_relative_path, "Album/track.flac");
        assert_eq!(observations.basis_size_bytes, Some(123));
        assert_eq!(observations.basis_mtime_ns, Some(456));
        assert_eq!(observations.basis_presence_state, "present");
        assert_eq!(observations.observed_at_ms, 23);
        assert_eq!(
            observations.content_hash.expect("content hash"),
            crate::read_models::source_file_observations::StoreContentHashEvidence {
                algorithm: "sha256".to_string(),
                value: "fixture-digest".to_string(),
            }
        );
        assert_eq!(
            observations.status,
            StoreSourceFileObservationStatus::Current
        );
    }

    #[test]
    fn source_file_basis_change_makes_existing_evidence_stale() {
        let mut connection = install_test_baseline();
        insert_source(&connection);
        insert_source_file(&connection, 100, "Album/track.flac", 123, 456);
        commit_source_observation(
            &mut connection,
            200,
            100,
            "basis:track:100",
            "audio",
            Some(ContentHashEvidence {
                algorithm: "sha256".to_string(),
                value: "fixture-digest".to_string(),
            }),
        );

        connection
            .execute(
                "UPDATE source_files
                 SET size_bytes = 124,
                     mtime_ns = 789,
                     updated_at = 30
                 WHERE source_file_id = 100",
                [],
            )
            .expect("change source file basis");

        let observations = read_source_file_observation(&connection, 100)
            .expect("read source-file observations")
            .expect("observations exist");
        assert_eq!(observations.basis_size_bytes, Some(123));
        assert_eq!(observations.basis_mtime_ns, Some(456));
        assert_eq!(observations.status, StoreSourceFileObservationStatus::Stale);
    }

    #[test]
    fn cue_source_file_holds_evidence_without_audio_pairing_or_identity_creation() {
        let mut connection = install_test_baseline();
        insert_source(&connection);
        insert_source_file(&connection, 100, "Album/track.flac", 123, 456);
        insert_source_file(&connection, 101, "Album/album.cue", 12, 457);
        connection
            .execute(
                "UPDATE source_files
                 SET file_kind = 'cue_sheet',
                     file_class = 'unsupported'
                 WHERE source_file_id = 101",
                [],
            )
            .expect("mark cue source file");

        commit_source_observation(
            &mut connection,
            201,
            101,
            "basis:cue:101",
            "cue_sheet",
            None,
        );

        let cue_observation = read_source_file_observation(&connection, 101)
            .expect("read cue observations")
            .expect("cue observations exist");
        assert_eq!(cue_observation.source_file_id, 101);
        assert_eq!(cue_observation.basis_relative_path, "Album/album.cue");
        assert_eq!(cue_observation.media_kind, "cue_sheet");
        assert_eq!(cue_observation.content_hash, None);
        assert_eq!(
            cue_observation.status,
            StoreSourceFileObservationStatus::Current
        );
        assert!(
            read_source_file_observation(&connection, 100)
                .expect("read audio observations")
                .is_none(),
            "CUE evidence must not be copied to the adjacent audio file"
        );
    }
}
