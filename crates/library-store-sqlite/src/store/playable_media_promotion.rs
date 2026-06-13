use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use crate::store::source_file_hash::SOURCE_FILE_BLAKE3_ALGORITHM;
use crate::time::unix_time_ms;

use super::SqliteDurableStore;

pub const DEFAULT_PLAYABLE_MEDIA_PROMOTION_LIMIT: usize = 4;
pub const MAX_PLAYABLE_MEDIA_PROMOTION_LIMIT: usize = 128;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PromotePlayableMediaForSourceResult {
    pub promoted_count: usize,
    pub refreshed_count: usize,
    pub skipped_unusable_source: usize,
    pub skipped_unsupported_media_kind: usize,
    pub skipped_no_observations: usize,
    pub skipped_stale_observations: usize,
    pub skipped_no_blake3: usize,
    pub skipped_no_probe_observations: usize,
    pub skipped_missing_attachment_link: usize,
    pub skipped_stale_attachment_link: usize,
    pub remaining_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayableMediaPromotionCandidate {
    attachment_id: i64,
    source_file_id: i64,
    basis_fingerprint: String,
    media_kind: String,
    mime_type: Option<String>,
    duration_ms: Option<i64>,
    sample_rate_hz: Option<i64>,
    channels: Option<i64>,
    bit_depth: Option<i64>,
    codec: Option<String>,
    change: PlayableMediaPromotionChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayableMediaPromotionChange {
    Create,
    Refresh,
}

impl SqliteDurableStore {
    pub fn promote_playable_media_for_source(
        &self,
        source_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<PromotePlayableMediaForSourceResult> {
        let promoted_at = unix_time_ms()?;
        self.with_write(|write| {
            promote_playable_media_for_source(write, source_id, limit, promoted_at)
        })
    }

    pub fn count_playable_media_promotion_candidates(
        &self,
        source_id: i64,
    ) -> LibrarySqliteResult<usize> {
        let connection = self.open_read_connection()?;
        let source_usable = source_is_usable(&connection, source_id)?;
        if !source_usable {
            return Ok(0);
        }
        count_playable_media_promotion_candidates(&connection, source_id)
    }
}

pub fn effective_playable_media_promotion_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_PLAYABLE_MEDIA_PROMOTION_LIMIT)
        .clamp(1, MAX_PLAYABLE_MEDIA_PROMOTION_LIMIT)
}

fn promote_playable_media_for_source(
    write: &mut AdmittedWrite<'_>,
    source_id: i64,
    limit: usize,
    promoted_at: i64,
) -> LibrarySqliteResult<PromotePlayableMediaForSourceResult> {
    let mut result = read_playable_media_promotion_skip_summary(write, source_id)?;
    if !source_is_usable(write, source_id)? {
        if source_exists(write, source_id)? {
            result.skipped_unusable_source = 1;
        }
        return Ok(result);
    }

    for candidate in read_playable_media_promotion_candidates(write, source_id, limit)? {
        upsert_playable_media(write, &candidate, promoted_at)?;
        match candidate.change {
            PlayableMediaPromotionChange::Create => result.promoted_count += 1,
            PlayableMediaPromotionChange::Refresh => result.refreshed_count += 1,
        }
    }

    result.remaining_candidates = count_playable_media_promotion_candidates(write, source_id)?;
    Ok(result)
}

fn source_exists(connection: &rusqlite::Connection, source_id: i64) -> LibrarySqliteResult<bool> {
    connection
        .query_row(
            "SELECT 1
             FROM sources
             WHERE source_id = ?1",
            [source_id],
            |_| Ok(true),
        )
        .optional()
        .map(|value| value.unwrap_or(false))
        .map_err(Into::into)
}

fn source_is_usable(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<bool> {
    connection
        .query_row(
            "SELECT COALESCE(state.mount_status, 'unknown'),
                    COALESCE(state.access_state, 'unknown')
             FROM sources source
             LEFT JOIN source_state state
               ON state.source_id = source.source_id
             WHERE source.source_id = ?1",
            [source_id],
            |row| {
                let mount_status: String = row.get(0)?;
                let access_state: String = row.get(1)?;
                Ok(matches!(
                    (access_state.as_str(), mount_status.as_str()),
                    ("accessible", status) if status != "unmounted"
                ) || matches!(
                    (access_state.as_str(), mount_status.as_str()),
                    ("unknown", "mounted")
                ))
            },
        )
        .optional()
        .map(|value| value.unwrap_or(false))
        .map_err(Into::into)
}

fn read_playable_media_promotion_skip_summary(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<PromotePlayableMediaForSourceResult> {
    connection
        .query_row(
            &format!(
                "SELECT
                    COALESCE(SUM(CASE
                        WHEN file.presence_state = 'present'
                         AND NOT (file.file_class = 'audio' AND file.file_kind = 'audio')
                        THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE
                        WHEN {present_audio_predicate}
                         AND observations.source_file_id IS NULL
                        THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE
                        WHEN {present_audio_predicate}
                         AND observations.source_file_id IS NOT NULL
                         AND NOT ({current_observations_predicate})
                        THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE
                        WHEN {present_audio_predicate}
                         AND observations.source_file_id IS NOT NULL
                         AND {current_observations_predicate}
                         AND NOT (
                             observations.content_hash_algorithm = ?2
                             AND observations.content_hash_value IS NOT NULL
                         )
                        THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE
                        WHEN {present_audio_predicate}
                         AND observations.source_file_id IS NOT NULL
                         AND {current_observations_predicate}
                         AND observations.content_hash_algorithm = ?2
                         AND observations.content_hash_value IS NOT NULL
                         AND NOT ({playable_probe_predicate})
                        THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE
                        WHEN {present_audio_predicate}
                         AND observations.source_file_id IS NOT NULL
                         AND {current_observations_predicate}
                         AND observations.content_hash_algorithm = ?2
                         AND observations.content_hash_value IS NOT NULL
                         AND {playable_probe_predicate}
                         AND link.source_file_id IS NULL
                        THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE
                        WHEN {present_audio_predicate}
                         AND observations.source_file_id IS NOT NULL
                         AND {current_observations_predicate}
                         AND observations.content_hash_algorithm = ?2
                         AND observations.content_hash_value IS NOT NULL
                         AND {playable_probe_predicate}
                         AND link.source_file_id IS NOT NULL
                         AND NOT (
                             attachment.content_hash_algorithm = observations.content_hash_algorithm
                             AND attachment.content_hash_value = observations.content_hash_value
                         )
                        THEN 1 ELSE 0 END), 0)
                 FROM source_files file
                 LEFT JOIN source_file_observations observations
                   ON observations.source_file_id = file.source_file_id
                 LEFT JOIN source_file_attachment_links link
                   ON link.source_file_id = file.source_file_id
                 LEFT JOIN content_attachments attachment
                   ON attachment.attachment_id = link.attachment_id
                 WHERE file.source_id = ?1",
                present_audio_predicate = PRESENT_AUDIO_PREDICATE,
                current_observations_predicate = CURRENT_OBSERVATIONS_PREDICATE,
                playable_probe_predicate = PLAYABLE_AUDIO_PROBE_PREDICATE,
            ),
            params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM],
            |row| {
                Ok(PromotePlayableMediaForSourceResult {
                    skipped_unsupported_media_kind: read_count(row, 0)?,
                    skipped_no_observations: read_count(row, 1)?,
                    skipped_stale_observations: read_count(row, 2)?,
                    skipped_no_blake3: read_count(row, 3)?,
                    skipped_no_probe_observations: read_count(row, 4)?,
                    skipped_missing_attachment_link: read_count(row, 5)?,
                    skipped_stale_attachment_link: read_count(row, 6)?,
                    ..Default::default()
                })
            },
        )
        .map_err(Into::into)
}

fn read_playable_media_promotion_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
    limit: usize,
) -> LibrarySqliteResult<Vec<PlayableMediaPromotionCandidate>> {
    let limit_i64 =
        i64::try_from(limit).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?;
    let sql = playable_media_promotion_candidates_sql(
        "SELECT candidate.attachment_id,
                candidate.source_file_id,
                candidate.basis_fingerprint,
                candidate.media_kind,
                candidate.mime_type,
                candidate.duration_ms,
                candidate.sample_rate_hz,
                candidate.channels,
                candidate.bit_depth,
                candidate.codec,
                CASE
                    WHEN existing.playable_media_id IS NULL THEN 'create'
                    ELSE 'refresh'
                END AS promotion_change",
        "ORDER BY lower(candidate.relative_path) ASC,
                  candidate.source_file_id ASC
         LIMIT ?3",
    );
    connection
        .prepare(&sql)?
        .query_map(
            params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM, limit_i64],
            |row| {
                let change = match row.get::<_, String>(10)?.as_str() {
                    "create" => PlayableMediaPromotionChange::Create,
                    _ => PlayableMediaPromotionChange::Refresh,
                };
                Ok(PlayableMediaPromotionCandidate {
                    attachment_id: row.get(0)?,
                    source_file_id: row.get(1)?,
                    basis_fingerprint: row.get(2)?,
                    media_kind: row.get(3)?,
                    mime_type: row.get(4)?,
                    duration_ms: row.get(5)?,
                    sample_rate_hz: row.get(6)?,
                    channels: row.get(7)?,
                    bit_depth: row.get(8)?,
                    codec: row.get(9)?,
                    change,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn count_playable_media_promotion_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    let sql = playable_media_promotion_candidates_sql("SELECT COUNT(*)", "");
    connection
        .query_row(
            &sql,
            params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn playable_media_promotion_candidates_sql(select_clause: &str, suffix: &str) -> String {
    format!(
        "WITH eligible AS (
             SELECT attachment.attachment_id,
                    file.source_file_id,
                    file.relative_path,
                    observations.basis_fingerprint,
                    observations.media_kind,
                    observations.mime_type,
                    observations.duration_ms,
                    observations.sample_rate_hz,
                    observations.channels,
                    observations.bit_depth,
                    observations.codec,
                    ROW_NUMBER() OVER (
                        PARTITION BY attachment.attachment_id
                        ORDER BY lower(file.relative_path) ASC,
                                 file.source_file_id ASC
                    ) AS attachment_rank
             FROM source_files file
             JOIN source_file_observations observations
               ON observations.source_file_id = file.source_file_id
             JOIN source_file_attachment_links link
               ON link.source_file_id = file.source_file_id
              AND link.source_id = file.source_id
             JOIN content_attachments attachment
               ON attachment.attachment_id = link.attachment_id
             WHERE file.source_id = ?1
               AND {present_audio_predicate}
               AND {current_observations_predicate}
               AND observations.content_hash_algorithm = ?2
               AND observations.content_hash_value IS NOT NULL
               AND attachment.content_hash_algorithm = observations.content_hash_algorithm
               AND attachment.content_hash_value = observations.content_hash_value
               AND {playable_probe_predicate}
         ),
         candidates AS (
             SELECT attachment_id,
                    source_file_id,
                    relative_path,
                    basis_fingerprint,
                    media_kind,
                    mime_type,
                    duration_ms,
                    sample_rate_hz,
                    channels,
                    bit_depth,
                    codec
             FROM eligible
             WHERE attachment_rank = 1
         )
         {select_clause}
         FROM candidates candidate
         LEFT JOIN playable_media existing
           ON existing.attachment_id = candidate.attachment_id
         WHERE existing.playable_media_id IS NULL
            OR existing.evidence_source_file_id != candidate.source_file_id
            OR existing.evidence_basis_fingerprint != candidate.basis_fingerprint
            OR existing.media_kind != candidate.media_kind
            OR existing.mime_type IS NOT candidate.mime_type
            OR existing.duration_ms IS NOT candidate.duration_ms
            OR existing.sample_rate_hz IS NOT candidate.sample_rate_hz
            OR existing.channels IS NOT candidate.channels
            OR existing.bit_depth IS NOT candidate.bit_depth
            OR existing.codec IS NOT candidate.codec
         {suffix}",
        present_audio_predicate = PRESENT_AUDIO_PREDICATE,
        current_observations_predicate = CURRENT_OBSERVATIONS_PREDICATE,
        playable_probe_predicate = PLAYABLE_AUDIO_PROBE_PREDICATE,
    )
}

fn upsert_playable_media(
    write: &mut AdmittedWrite<'_>,
    candidate: &PlayableMediaPromotionCandidate,
    promoted_at: i64,
) -> LibrarySqliteResult<()> {
    write.execute(
        "INSERT INTO playable_media (
             attachment_id,
             evidence_source_file_id,
             evidence_basis_fingerprint,
             media_kind,
             mime_type,
             duration_ms,
             sample_rate_hz,
             channels,
             bit_depth,
             codec,
             created_at,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
         ON CONFLICT(attachment_id) DO UPDATE SET
             evidence_source_file_id = excluded.evidence_source_file_id,
             evidence_basis_fingerprint = excluded.evidence_basis_fingerprint,
             media_kind = excluded.media_kind,
             mime_type = excluded.mime_type,
             duration_ms = excluded.duration_ms,
             sample_rate_hz = excluded.sample_rate_hz,
             channels = excluded.channels,
             bit_depth = excluded.bit_depth,
             codec = excluded.codec,
             updated_at = excluded.updated_at",
        params![
            candidate.attachment_id,
            candidate.source_file_id,
            candidate.basis_fingerprint,
            candidate.media_kind,
            candidate.mime_type,
            candidate.duration_ms,
            candidate.sample_rate_hz,
            candidate.channels,
            candidate.bit_depth,
            candidate.codec,
            promoted_at,
        ],
    )?;
    Ok(())
}

fn read_count(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

const PRESENT_AUDIO_PREDICATE: &str =
    "file.presence_state = 'present' AND file.file_class = 'audio' AND file.file_kind = 'audio'";

const CURRENT_OBSERVATIONS_PREDICATE: &str = "file.source_id = observations.basis_source_id
    AND file.relative_path = observations.basis_relative_path
    AND file.size_bytes IS observations.basis_size_bytes
    AND file.mtime_ns IS observations.basis_mtime_ns
    AND file.presence_state = observations.basis_presence_state";

const PLAYABLE_AUDIO_PROBE_PREDICATE: &str = "observations.media_kind = 'audio'
    AND (
        observations.mime_type IS NOT NULL
        OR observations.duration_ms IS NOT NULL
        OR observations.sample_rate_hz IS NOT NULL
        OR observations.channels IS NOT NULL
        OR observations.bit_depth IS NOT NULL
        OR observations.codec IS NOT NULL
    )";

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use tempfile::TempDir;

    use crate::read_models::contents::{
        StoreContentsFileClass, StoreContentsReadPolicy, StoreContentsScope,
        StoreContentsScopeDepth, StoreContentsState, StorePlayableMediaKind,
    };
    use crate::{PromotePlayableMediaForSourceResult, SqliteDurableStore};

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    struct PlayableMediaPromotionFixture {
        _tempdir: TempDir,
        store: SqliteDurableStore,
        source_id: i64,
    }

    impl PlayableMediaPromotionFixture {
        fn new() -> Self {
            let tempdir = TempDir::new().expect("create tempdir");
            let db_path = tempdir.path().join("library.sqlite3");
            let store = SqliteDurableStore::open(&db_path).expect("open store");
            let fixture = Self {
                _tempdir: tempdir,
                store,
                source_id: 1,
            };
            fixture.insert_source();
            fixture
        }

        fn insert_source(&self) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO sources (
                             source_id,
                             source_class,
                             authority,
                             identity_key,
                             display_name,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, 'internal', 'system', 'source:playable-media-test', 'Playable Media Test', 1, 1)",
                        [self.source_id],
                    )?;
                    write.execute(
                        "INSERT INTO source_state (
                             source_id,
                             mount_status,
                             mount_epoch,
                             access_state,
                             access_checked_at,
                             effective_path,
                             updated_at
                         )
                         VALUES (?1, 'mounted', 1, 'accessible', 1, 'root', 1)",
                        [self.source_id],
                    )?;
                    write.execute(
                        "INSERT INTO source_scan_state (
                             source_id,
                             scan_phase,
                             last_scan_started_at,
                             last_scan_finished_at,
                             last_successful_scan_at,
                             updated_at
                         )
                         VALUES (?1, 'complete', 1, 2, 2, 2)",
                        [self.source_id],
                    )?;
                    Ok(())
                })
                .expect("insert source");
        }

        fn insert_source_file(&self, source_file_id: i64, relative_path: &str) {
            let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
            let file_kind = crate::browse_media::file_kind_str_from_path(relative_path);
            let file_class = crate::browse_media::file_class_str_from_path(relative_path);
            let name_sort_key = crate::browse_sort_key::compute_name_sort_key(file_name);
            let path_sort_key = crate::browse_sort_key::compute_path_sort_key(relative_path);
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO source_files (
                             source_file_id,
                             source_id,
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
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 10, 100, ?7, ?8, 'present', 1, 1, 1, 1, 1)",
                        params![
                            source_file_id,
                            self.source_id,
                            file_name,
                            name_sort_key,
                            path_sort_key,
                            relative_path,
                            file_kind,
                            file_class,
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert source file");
        }

        fn commit_current_observations(
            &self,
            source_file_id: i64,
            hash_value: Option<&str>,
            media_kind: &str,
            with_probe: bool,
        ) {
            self.store
                .with_write(|write| {
                    let artifact_id = 10_000 + source_file_id;
                    write.execute(
                        "INSERT OR IGNORE INTO work_items (
                             work_item_id,
                             subject_kind,
                             subject_id,
                             work_kind,
                             basis_fingerprint,
                             state,
                             priority_class,
                             created_at,
                             updated_at
                         )
                         VALUES (1, 'source_file', 'fixture', 'inspect_source_file', 'fixture', 'completed', 'interactive', 1, 1)",
                        [],
                    )?;
                    write.execute(
                        "INSERT OR IGNORE INTO work_runs (
                             work_run_id,
                             work_item_id,
                             adapter_key,
                             adapter_version,
                             started_at,
                             outcome
                         )
                         VALUES (1, 1, 'test.playable_media_promotion', '1', 1, 'ok')",
                        [],
                    )?;
                    write.execute(
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
                         VALUES (?1, 1, 'source_file', ?2, 'inspection_result', 'test.playable_media_promotion', '1', ?3, 'application/json', 'inline_payload', ?4, 1)",
                        params![
                            artifact_id,
                            source_file_id.to_string(),
                            format!("basis:{source_file_id}"),
                            format!("payload:{source_file_id}")
                        ],
                    )?;
                    let (relative_path, size_bytes, mtime_ns, presence_state): (
                        String,
                        Option<i64>,
                        Option<i64>,
                        String,
                    ) = write.query_row(
                        "SELECT relative_path, size_bytes, mtime_ns, presence_state
                         FROM source_files
                         WHERE source_file_id = ?1",
                        [source_file_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )?;
                    write.execute(
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
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 1, ?17)
                         ON CONFLICT(source_file_id) DO UPDATE SET
                             basis_fingerprint = excluded.basis_fingerprint,
                             basis_source_id = excluded.basis_source_id,
                             basis_relative_path = excluded.basis_relative_path,
                             basis_size_bytes = excluded.basis_size_bytes,
                             basis_mtime_ns = excluded.basis_mtime_ns,
                             basis_presence_state = excluded.basis_presence_state,
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
                            source_file_id,
                            format!("basis:{source_file_id}"),
                            self.source_id,
                            relative_path,
                            size_bytes,
                            mtime_ns,
                            presence_state,
                            hash_value.map(|_| "blake3"),
                            hash_value,
                            media_kind,
                            with_probe.then_some("audio/wav"),
                            with_probe.then_some(100_i64),
                            with_probe.then_some(44_100_i64),
                            with_probe.then_some(2_i64),
                            with_probe.then_some(16_i64),
                            with_probe.then_some("pcm"),
                            artifact_id,
                        ],
                    )?;
                    Ok(())
                })
                .expect("commit observations");
        }

        fn link_attachment(&self, source_file_id: i64, hash_value: &str) -> i64 {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT OR IGNORE INTO content_attachments (
                             content_hash_algorithm,
                             content_hash_value,
                             first_observed_at,
                             updated_at
                         )
                         VALUES ('blake3', ?1, 1, 1)",
                        [hash_value],
                    )?;
                    let attachment_id: i64 = write.query_row(
                        "SELECT attachment_id
                         FROM content_attachments
                         WHERE content_hash_algorithm = 'blake3'
                           AND content_hash_value = ?1",
                        [hash_value],
                        |row| row.get(0),
                    )?;
                    let file_kind: String = write.query_row(
                        "SELECT file_kind
                         FROM source_files
                         WHERE source_file_id = ?1",
                        [source_file_id],
                        |row| row.get(0),
                    )?;
                    write.execute(
                        "INSERT OR REPLACE INTO source_file_attachment_links (
                             attachment_id,
                             source_file_id,
                             source_id,
                             file_kind,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, 1, 1)",
                        params![attachment_id, source_file_id, self.source_id, file_kind],
                    )?;
                    Ok(attachment_id)
                })
                .expect("link attachment")
        }

        fn promote(&self, limit: usize) -> PromotePlayableMediaForSourceResult {
            self.store
                .promote_playable_media_for_source(self.source_id, limit)
                .expect("promote playable media")
        }

        fn count_rows(&self, table: &str) -> i64 {
            self.store
                .open_read_connection()
                .expect("open read")
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count rows")
        }

        fn change_file_basis(&self, source_file_id: i64) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "UPDATE source_files
                         SET size_bytes = 11,
                             mtime_ns = 101,
                             updated_at = 2
                         WHERE source_file_id = ?1",
                        [source_file_id],
                    )?;
                    Ok(())
                })
                .expect("change source file basis");
        }
    }

    #[test]
    fn current_audio_attachment_and_probe_promotes_one_candidate() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        let attachment_id = fixture.link_attachment(100, HASH_A);
        fixture.commit_current_observations(100, Some(HASH_A), "audio", true);

        let result = fixture.promote(10);

        assert_eq!(
            result,
            PromotePlayableMediaForSourceResult {
                promoted_count: 1,
                ..Default::default()
            }
        );
        assert_eq!(fixture.count_rows("playable_media"), 1);
        let connection = fixture.store.open_read_connection().expect("open read");
        let stored: (i64, i64, Option<i64>, Option<i64>, Option<String>) = connection
            .query_row(
                "SELECT attachment_id,
                        evidence_source_file_id,
                        duration_ms,
                        sample_rate_hz,
                        codec
                 FROM playable_media",
                [],
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
            .expect("read promoted candidate");
        assert_eq!(
            stored,
            (
                attachment_id,
                100,
                Some(100),
                Some(44_100),
                Some("pcm".to_string())
            )
        );
    }

    #[test]
    fn duplicate_source_files_for_one_attachment_promote_one_candidate() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/a.wav");
        fixture.insert_source_file(101, "Album/b.wav");
        let attachment_id = fixture.link_attachment(100, HASH_A);
        fixture.link_attachment(101, HASH_A);
        fixture.commit_current_observations(100, Some(HASH_A), "audio", true);
        fixture.commit_current_observations(101, Some(HASH_A), "audio", true);

        let result = fixture.promote(10);

        assert_eq!(result.promoted_count, 1);
        assert_eq!(fixture.count_rows("playable_media"), 1);
        let connection = fixture.store.open_read_connection().expect("open read");
        let stored: (i64, i64) = connection
            .query_row(
                "SELECT attachment_id, evidence_source_file_id
                 FROM playable_media",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read candidate");
        assert_eq!(stored, (attachment_id, 100));
    }

    #[test]
    fn hash_without_probe_does_not_promote() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/no-probe.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_observations(100, Some(HASH_A), "audio", false);

        let result = fixture.promote(10);

        assert_eq!(result.promoted_count, 0);
        assert_eq!(result.skipped_no_probe_observations, 1);
        assert_eq!(fixture.count_rows("playable_media"), 0);
    }

    #[test]
    fn probe_without_attachment_link_does_not_promote() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/no-link.wav");
        fixture.commit_current_observations(100, Some(HASH_A), "audio", true);

        let result = fixture.promote(10);

        assert_eq!(result.promoted_count, 0);
        assert_eq!(result.skipped_missing_attachment_link, 1);
        assert_eq!(fixture.count_rows("playable_media"), 0);
    }

    #[test]
    fn stale_source_observations_do_not_promote() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/stale.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_observations(100, Some(HASH_A), "audio", true);
        fixture.change_file_basis(100);

        let result = fixture.promote(10);

        assert_eq!(result.promoted_count, 0);
        assert_eq!(result.skipped_stale_observations, 1);
        assert_eq!(fixture.count_rows("playable_media"), 0);
    }

    #[test]
    fn stale_attachment_link_does_not_promote() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/stale-link.wav");
        fixture.link_attachment(100, HASH_B);
        fixture.commit_current_observations(100, Some(HASH_A), "audio", true);

        let result = fixture.promote(10);

        assert_eq!(result.promoted_count, 0);
        assert_eq!(result.skipped_stale_attachment_link, 1);
        assert_eq!(fixture.count_rows("playable_media"), 0);
    }

    #[test]
    fn non_audio_files_do_not_promote() {
        let fixture = PlayableMediaPromotionFixture::new();
        for (id, path, hash) in [
            (100, "Album/video.mp4", HASH_A),
            (101, "Album/cover.jpg", HASH_B),
            (102, "Album/album.cue", HASH_C),
            (103, "Album/notes.txt", HASH_A),
            (104, "Album/archive.zip", HASH_B),
            (105, "Album/blob.bin", HASH_C),
        ] {
            fixture.insert_source_file(id, path);
            fixture.link_attachment(id, hash);
            fixture.commit_current_observations(id, Some(hash), "audio", true);
        }

        let result = fixture.promote(10);

        assert_eq!(result.promoted_count, 0);
        assert_eq!(result.skipped_unsupported_media_kind, 6);
        assert_eq!(fixture.count_rows("playable_media"), 0);
    }

    #[test]
    fn read_contents_playable_media_returns_only_promoted_evidence_backed_rows() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/promoted.wav");
        fixture.insert_source_file(101, "Album/fallback-removed.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_observations(100, Some(HASH_A), "audio", true);
        fixture.promote(10);

        let result = fixture
            .store
            .read_contents(
                StoreContentsScope::Source {
                    source_id: fixture.source_id,
                },
                playable_media_policy(),
                StoreContentsScopeDepth::Recursive,
                10,
                None,
            )
            .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 1);
        let row = &result.rows[0];
        assert_eq!(row.source_file_id, 100);
        let media = row.playable_media.as_ref().expect("playable-media record");
        assert!(media.playable_media_id > 0);
        assert!(media.attachment_id > 0);
        assert_eq!(media.content_hash_value, HASH_A);
        assert_eq!(media.codec.as_deref(), Some("pcm"));
    }

    #[test]
    fn source_file_inventory_default_remains_unchanged() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");

        let result = fixture
            .store
            .read_contents(
                StoreContentsScope::Source {
                    source_id: fixture.source_id,
                },
                StoreContentsReadPolicy::SourceFileInventory {
                    file_classes: vec![
                        StoreContentsFileClass::Audio,
                        StoreContentsFileClass::Video,
                        StoreContentsFileClass::Image,
                        StoreContentsFileClass::Unsupported,
                    ],
                },
                StoreContentsScopeDepth::Recursive,
                10,
                None,
            )
            .expect("read source file contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 1);
        assert!(result.rows[0].playable_media.is_none());
    }

    #[test]
    fn promotion_remaining_count_is_bounded_and_deterministic() {
        let fixture = PlayableMediaPromotionFixture::new();
        for (id, path, hash) in [
            (100, "Album/a.wav", HASH_A),
            (101, "Album/b.wav", HASH_B),
            (102, "Album/c.wav", HASH_C),
        ] {
            fixture.insert_source_file(id, path);
            fixture.link_attachment(id, hash);
            fixture.commit_current_observations(id, Some(hash), "audio", true);
        }

        let first = fixture.promote(2);
        assert_eq!(first.promoted_count, 2);
        assert_eq!(first.remaining_candidates, 1);
        assert_eq!(fixture.count_rows("playable_media"), 2);

        let second = fixture.promote(2);
        assert_eq!(second.promoted_count, 1);
        assert_eq!(second.remaining_candidates, 0);

        let connection = fixture.store.open_read_connection().expect("open read");
        let source_file_ids = connection
            .prepare(
                "SELECT evidence_source_file_id
                 FROM playable_media
                 ORDER BY evidence_source_file_id ASC",
            )
            .expect("prepare candidate read")
            .query_map([], |row| row.get::<_, i64>(0))
            .expect("query candidates")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect candidates");
        assert_eq!(source_file_ids, vec![100, 101, 102]);
    }

    #[test]
    fn promotion_writes_only_playable_media() {
        let fixture = PlayableMediaPromotionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_observations(100, Some(HASH_A), "audio", true);

        fixture.promote(10);

        assert_eq!(fixture.count_rows("playable_media"), 1);
        assert_eq!(fixture.count_rows("track_identity_candidates"), 0);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 0);
    }

    fn playable_media_policy() -> StoreContentsReadPolicy {
        StoreContentsReadPolicy::PlayableMedia {
            media_kinds: vec![StorePlayableMediaKind::Audio, StorePlayableMediaKind::Video],
        }
    }
}
