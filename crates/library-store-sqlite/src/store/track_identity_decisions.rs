use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use crate::read_models::track_identity_decisions::{
    StoreTrackIdentityDecisionState, StoreTrackIdentityEffectiveDecisionSummary,
    read_effective_track_identity_decision_for_candidate,
};
use crate::store::source_file_hash::SOURCE_FILE_BLAKE3_ALGORITHM;
use crate::time::unix_time_ms;
use crate::track_identity_evidence_predicates::current_track_identity_candidate_evidence_predicate;

use super::SqliteDurableStore;

pub const DEFAULT_TRACK_IDENTITY_DECISION_LIMIT: usize = 4;
pub const MAX_TRACK_IDENTITY_DECISION_LIMIT: usize = 128;
const TRACK_IDENTITY_CANDIDATE_KIND: &str = "exact_primary_media_content";
const TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS: &str = "current_primary_media_exact_blake3";
const TRACK_IDENTITY_DECISION_STATE_ACCEPTED: &str = "accepted";
const TRACK_IDENTITY_DECISION_STATE_REJECTED: &str = "rejected";
const TRACK_IDENTITY_DECISION_STATE_DEFERRED: &str = "deferred";
pub const TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0: &str = "system_exact_content_v0";
pub const TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0: &str = "user_local_v0";
const TRACK_IDENTITY_DECISION_BASIS_SYSTEM_EXACT_CONTENT_V0: &str =
    "active_exact_content_candidate_current_evidence_v0";
const TRACK_IDENTITY_DECISION_REASON_SYSTEM_EXACT_CONTENT_V0: &str =
    "accepted active exact-content track identity candidate from current primary-media evidence";
const TRACK_IDENTITY_DECISION_BASIS_USER_LOCAL_V0: &str = "explicit_user_local_decision_v0";
const TRACK_IDENTITY_DECISION_REASON_USER_ACCEPTED_V0: &str =
    "user explicitly accepted track identity candidate";
const TRACK_IDENTITY_DECISION_REASON_USER_REJECTED_V0: &str =
    "user explicitly rejected track identity candidate";
const TRACK_IDENTITY_DECISION_REASON_USER_DEFERRED_V0: &str =
    "user explicitly deferred track identity candidate";
const MAX_TRACK_IDENTITY_DECISION_REASON_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProduceTrackIdentityDecisionsForSourceResult {
    pub decisions_created: usize,
    pub decision_evidence_created: usize,
    pub skipped_stale_candidates: usize,
    pub skipped_existing_current_decisions: usize,
    pub skipped_user_blocked_candidates: usize,
    pub remaining_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackIdentityDecisionChangeSuccess {
    pub track_identity_decision_id: i64,
    pub track_identity_candidate_id: i64,
    pub decision_state: StoreTrackIdentityDecisionState,
    pub decision_source: String,
    pub evidence_snapshot_count: usize,
    pub decision_created: bool,
    pub effective_decision: StoreTrackIdentityEffectiveDecisionSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackIdentityDecisionChangeFailure {
    CandidateNotFound,
    CandidateStaleForAccept,
    NoCurrentEvidenceForAccept,
    NoSourceScopeForDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackIdentityDecisionChangeResult {
    Written(TrackIdentityDecisionChangeSuccess),
    Failed(TrackIdentityDecisionChangeFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackIdentityDecisionProductionCandidate {
    track_identity_candidate_id: i64,
    candidate_kind: String,
    candidate_evidence_basis: String,
    evidence_key_algorithm: String,
    evidence_key_value: String,
    candidate_status: String,
}

impl SqliteDurableStore {
    pub fn accept_track_identity_candidate(
        &self,
        track_identity_candidate_id: i64,
        reason: Option<String>,
    ) -> LibrarySqliteResult<TrackIdentityDecisionChangeResult> {
        let decided_at = unix_time_ms()?;
        self.with_write(|write| {
            write_user_track_identity_decision(
                write,
                track_identity_candidate_id,
                UserTrackIdentityDecisionState::Accepted,
                reason,
                decided_at,
            )
        })
    }

    pub fn reject_track_identity_candidate(
        &self,
        track_identity_candidate_id: i64,
        reason: Option<String>,
    ) -> LibrarySqliteResult<TrackIdentityDecisionChangeResult> {
        let decided_at = unix_time_ms()?;
        self.with_write(|write| {
            write_user_track_identity_decision(
                write,
                track_identity_candidate_id,
                UserTrackIdentityDecisionState::Rejected,
                reason,
                decided_at,
            )
        })
    }

    pub fn defer_track_identity_candidate(
        &self,
        track_identity_candidate_id: i64,
        reason: Option<String>,
    ) -> LibrarySqliteResult<TrackIdentityDecisionChangeResult> {
        let decided_at = unix_time_ms()?;
        self.with_write(|write| {
            write_user_track_identity_decision(
                write,
                track_identity_candidate_id,
                UserTrackIdentityDecisionState::Deferred,
                reason,
                decided_at,
            )
        })
    }

    pub fn produce_track_identity_decisions_for_source(
        &self,
        source_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<ProduceTrackIdentityDecisionsForSourceResult> {
        let decided_at = unix_time_ms()?;
        self.with_write(|write| {
            produce_track_identity_decisions_for_source(write, source_id, limit, decided_at)
        })
    }

    pub fn count_track_identity_decision_production_candidates(
        &self,
        source_id: i64,
    ) -> LibrarySqliteResult<usize> {
        let connection = self.open_read_connection()?;
        count_track_identity_decision_production_candidates(&connection, source_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UserTrackIdentityDecisionState {
    Accepted,
    Rejected,
    Deferred,
}

impl UserTrackIdentityDecisionState {
    fn as_store_state(self) -> StoreTrackIdentityDecisionState {
        match self {
            Self::Accepted => StoreTrackIdentityDecisionState::Accepted,
            Self::Rejected => StoreTrackIdentityDecisionState::Rejected,
            Self::Deferred => StoreTrackIdentityDecisionState::Deferred,
        }
    }

    fn as_sql(self) -> &'static str {
        match self {
            Self::Accepted => TRACK_IDENTITY_DECISION_STATE_ACCEPTED,
            Self::Rejected => TRACK_IDENTITY_DECISION_STATE_REJECTED,
            Self::Deferred => TRACK_IDENTITY_DECISION_STATE_DEFERRED,
        }
    }

    fn default_reason(self) -> &'static str {
        match self {
            Self::Accepted => TRACK_IDENTITY_DECISION_REASON_USER_ACCEPTED_V0,
            Self::Rejected => TRACK_IDENTITY_DECISION_REASON_USER_REJECTED_V0,
            Self::Deferred => TRACK_IDENTITY_DECISION_REASON_USER_DEFERRED_V0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentDecision {
    track_identity_decision_id: i64,
    decision_state: StoreTrackIdentityDecisionState,
}

pub fn effective_track_identity_decision_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_TRACK_IDENTITY_DECISION_LIMIT)
        .clamp(1, MAX_TRACK_IDENTITY_DECISION_LIMIT)
}

fn write_user_track_identity_decision(
    write: &mut AdmittedWrite<'_>,
    track_identity_candidate_id: i64,
    decision_state: UserTrackIdentityDecisionState,
    reason: Option<String>,
    decided_at: i64,
) -> LibrarySqliteResult<TrackIdentityDecisionChangeResult> {
    let Some(candidate) =
        read_track_identity_candidate_for_decision(write, track_identity_candidate_id)?
    else {
        return Ok(TrackIdentityDecisionChangeResult::Failed(
            TrackIdentityDecisionChangeFailure::CandidateNotFound,
        ));
    };

    let current_evidence_count =
        count_current_track_identity_candidate_evidence(write, track_identity_candidate_id)?;
    if decision_state == UserTrackIdentityDecisionState::Accepted {
        if candidate.candidate_status != "active" {
            return Ok(TrackIdentityDecisionChangeResult::Failed(
                TrackIdentityDecisionChangeFailure::CandidateStaleForAccept,
            ));
        }
        if current_evidence_count == 0 {
            return Ok(TrackIdentityDecisionChangeResult::Failed(
                TrackIdentityDecisionChangeFailure::NoCurrentEvidenceForAccept,
            ));
        }
    }

    if current_evidence_count == 0 {
        let candidate_evidence_count =
            count_candidate_evidence_rows(write, track_identity_candidate_id)?;
        if candidate_evidence_count == 0 {
            return Ok(TrackIdentityDecisionChangeResult::Failed(
                TrackIdentityDecisionChangeFailure::NoSourceScopeForDecision,
            ));
        }
    }

    let current_user_decision = read_current_user_decision(write, track_identity_candidate_id)?;
    if let Some(current_decision) = &current_user_decision
        && current_decision.decision_state == decision_state.as_store_state()
    {
        return write_existing_user_decision_result(
            write,
            current_decision.track_identity_decision_id,
            track_identity_candidate_id,
            decision_state,
        );
    }

    let track_identity_decision_id = next_track_identity_decision_id(write)?;
    // The current-source unique index allows only one current user decision per candidate.
    // When replacing a current user decision, insert the new row temporarily as superseded
    // by the old row, then point the old row at the new row and clear the new row. That
    // preserves history without dropping the unique guard during the correction.
    insert_user_decision(
        write,
        track_identity_decision_id,
        &candidate,
        decision_state,
        reason,
        current_user_decision
            .as_ref()
            .map(|decision| decision.track_identity_decision_id),
        decided_at,
    )?;
    if current_user_decision.is_some() {
        supersede_current_user_decision(
            write,
            track_identity_candidate_id,
            track_identity_decision_id,
            decided_at,
        )?;
        make_user_decision_current(write, track_identity_decision_id, decided_at)?;
    }
    let evidence_snapshot_count = insert_decision_evidence_snapshot(
        write,
        track_identity_decision_id,
        track_identity_candidate_id,
        decided_at,
    )?;
    let source_scope_count = insert_decision_source_scope(
        write,
        track_identity_decision_id,
        track_identity_candidate_id,
        decided_at,
    )?;
    if source_scope_count == 0 {
        // The preflight guard (current_evidence == 0 && candidate_evidence == 0) should
        // prevent reaching this point, but enforce the contract as a safety net.
        // Delete the decision row (cascades to evidence snapshot and source scope).
        write.execute(
            "DELETE FROM track_identity_decisions
             WHERE track_identity_decision_id = ?1",
            params![track_identity_decision_id],
        )?;
        // Restore any previously-current decision that was superseded.
        if let Some(old_decision) = &current_user_decision {
            write.execute(
                "UPDATE track_identity_decisions
                 SET superseded_by_decision_id = NULL,
                     updated_at = ?1
                 WHERE track_identity_decision_id = ?2",
                params![decided_at, old_decision.track_identity_decision_id],
            )?;
        }
        return Ok(TrackIdentityDecisionChangeResult::Failed(
            TrackIdentityDecisionChangeFailure::NoSourceScopeForDecision,
        ));
    }
    let effective_decision =
        read_effective_track_identity_decision_for_candidate(write, track_identity_candidate_id)?;

    Ok(TrackIdentityDecisionChangeResult::Written(
        TrackIdentityDecisionChangeSuccess {
            track_identity_decision_id,
            track_identity_candidate_id,
            decision_state: decision_state.as_store_state(),
            decision_source: TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0.to_string(),
            evidence_snapshot_count,
            decision_created: true,
            effective_decision,
        },
    ))
}

fn write_existing_user_decision_result(
    write: &mut AdmittedWrite<'_>,
    track_identity_decision_id: i64,
    track_identity_candidate_id: i64,
    decision_state: UserTrackIdentityDecisionState,
) -> LibrarySqliteResult<TrackIdentityDecisionChangeResult> {
    let evidence_snapshot_count =
        count_decision_evidence_snapshot_rows(write, track_identity_decision_id)?;
    let effective_decision =
        read_effective_track_identity_decision_for_candidate(write, track_identity_candidate_id)?;

    Ok(TrackIdentityDecisionChangeResult::Written(
        TrackIdentityDecisionChangeSuccess {
            track_identity_decision_id,
            track_identity_candidate_id,
            decision_state: decision_state.as_store_state(),
            decision_source: TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0.to_string(),
            evidence_snapshot_count,
            decision_created: false,
            effective_decision,
        },
    ))
}

fn produce_track_identity_decisions_for_source(
    write: &mut AdmittedWrite<'_>,
    source_id: i64,
    limit: usize,
    decided_at: i64,
) -> LibrarySqliteResult<ProduceTrackIdentityDecisionsForSourceResult> {
    let skipped_stale_candidates = count_stale_track_identity_candidates(write, source_id)?;
    let skipped_existing_current_decisions =
        count_existing_current_system_decisions(write, source_id)?;
    let skipped_user_blocked_candidates = count_user_blocked_candidates(write, source_id)?;
    let candidates = read_track_identity_decision_production_candidates(write, source_id)?;
    let mut result = ProduceTrackIdentityDecisionsForSourceResult {
        skipped_stale_candidates,
        skipped_existing_current_decisions,
        skipped_user_blocked_candidates,
        remaining_candidates: candidates.len().saturating_sub(limit),
        ..Default::default()
    };

    for candidate in candidates.into_iter().take(limit) {
        let track_identity_decision_id =
            insert_system_exact_content_decision(write, &candidate, decided_at)?;
        result.decisions_created += 1;
        result.decision_evidence_created += insert_decision_evidence_snapshot(
            write,
            track_identity_decision_id,
            candidate.track_identity_candidate_id,
            decided_at,
        )?;
        insert_decision_source_scope(
            write,
            track_identity_decision_id,
            candidate.track_identity_candidate_id,
            decided_at,
        )?;
    }

    result.remaining_candidates =
        count_track_identity_decision_production_candidates(write, source_id)?;
    Ok(result)
}

fn count_track_identity_decision_production_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    read_track_identity_decision_production_candidates(connection, source_id).map(|rows| rows.len())
}

fn read_track_identity_candidate_for_decision(
    connection: &rusqlite::Connection,
    track_identity_candidate_id: i64,
) -> LibrarySqliteResult<Option<TrackIdentityDecisionProductionCandidate>> {
    connection
        .query_row(
            "SELECT track_identity_candidate_id,
                    candidate_kind,
                    evidence_basis,
                    evidence_key_algorithm,
                    evidence_key_value,
                    status
             FROM track_identity_candidates
             WHERE track_identity_candidate_id = ?1",
            [track_identity_candidate_id],
            |row| {
                Ok(TrackIdentityDecisionProductionCandidate {
                    track_identity_candidate_id: row.get(0)?,
                    candidate_kind: row.get(1)?,
                    candidate_evidence_basis: row.get(2)?,
                    evidence_key_algorithm: row.get(3)?,
                    evidence_key_value: row.get(4)?,
                    candidate_status: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn count_current_track_identity_candidate_evidence(
    connection: &rusqlite::Connection,
    track_identity_candidate_id: i64,
) -> LibrarySqliteResult<usize> {
    let current_evidence_predicate = current_track_identity_candidate_evidence_predicate("?2");
    connection
        .query_row(
            &format!(
                "SELECT COUNT(*)
                 FROM track_identity_candidate_evidence evidence
                 JOIN source_files file
                   ON file.source_file_id = evidence.source_file_id
                 LEFT JOIN SourceFacts facts
                   ON facts.source_file_id = evidence.source_file_id
                 LEFT JOIN source_file_attachment_links link
                   ON link.source_file_attachment_link_id =
                      evidence.source_file_attachment_link_id
                 LEFT JOIN content_attachments attachment
                   ON attachment.attachment_id = evidence.attachment_id
                 WHERE evidence.track_identity_candidate_id = ?1
                   AND {current_evidence_predicate}",
                current_evidence_predicate = current_evidence_predicate,
            ),
            params![track_identity_candidate_id, SOURCE_FILE_BLAKE3_ALGORITHM],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn count_candidate_evidence_rows(
    connection: &rusqlite::Connection,
    track_identity_candidate_id: i64,
) -> LibrarySqliteResult<usize> {
    connection
        .query_row(
            "SELECT COUNT(*)
             FROM track_identity_candidate_evidence
             WHERE track_identity_candidate_id = ?1",
            [track_identity_candidate_id],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn read_current_user_decision(
    connection: &rusqlite::Connection,
    track_identity_candidate_id: i64,
) -> LibrarySqliteResult<Option<CurrentDecision>> {
    connection
        .query_row(
            "SELECT track_identity_decision_id,
                    decision_state
             FROM track_identity_decisions
             WHERE track_identity_candidate_id = ?1
               AND decision_source = ?2
               AND superseded_by_decision_id IS NULL
               AND decision_state != 'superseded'",
            params![
                track_identity_candidate_id,
                TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
            ],
            |row| {
                Ok(CurrentDecision {
                    track_identity_decision_id: row.get(0)?,
                    decision_state: map_store_decision_state(row.get::<_, String>(1)?.as_str()),
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn next_track_identity_decision_id(connection: &rusqlite::Connection) -> LibrarySqliteResult<i64> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(track_identity_decision_id), 0) + 1
             FROM track_identity_decisions",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn supersede_current_user_decision(
    write: &mut AdmittedWrite<'_>,
    track_identity_candidate_id: i64,
    superseded_by_decision_id: i64,
    superseded_at: i64,
) -> LibrarySqliteResult<usize> {
    write
        .execute(
            "UPDATE track_identity_decisions
             SET superseded_by_decision_id = ?2,
                 updated_at = ?3
             WHERE track_identity_candidate_id = ?1
               AND decision_source = ?4
               AND superseded_by_decision_id IS NULL
               AND decision_state != 'superseded'",
            params![
                track_identity_candidate_id,
                superseded_by_decision_id,
                superseded_at,
                TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
            ],
        )
        .map_err(Into::into)
}

fn insert_user_decision(
    write: &mut AdmittedWrite<'_>,
    track_identity_decision_id: i64,
    candidate: &TrackIdentityDecisionProductionCandidate,
    decision_state: UserTrackIdentityDecisionState,
    reason: Option<String>,
    temporary_superseded_by_decision_id: Option<i64>,
    decided_at: i64,
) -> LibrarySqliteResult<()> {
    let decision_reason = normalized_decision_reason(reason, decision_state.default_reason());
    write.execute(
        "INSERT INTO track_identity_decisions (
             track_identity_decision_id,
             track_identity_candidate_id,
             decision_state,
             decision_source,
             decision_basis,
             decision_reason,
             candidate_kind,
             candidate_evidence_basis,
             candidate_status_at_decision,
             evidence_key_algorithm,
             evidence_key_value,
             superseded_by_decision_id,
             created_at,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
        params![
            track_identity_decision_id,
            candidate.track_identity_candidate_id,
            decision_state.as_sql(),
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
            TRACK_IDENTITY_DECISION_BASIS_USER_LOCAL_V0,
            decision_reason,
            candidate.candidate_kind,
            candidate.candidate_evidence_basis,
            candidate.candidate_status,
            candidate.evidence_key_algorithm,
            candidate.evidence_key_value,
            temporary_superseded_by_decision_id,
            decided_at,
        ],
    )?;
    Ok(())
}

fn make_user_decision_current(
    write: &mut AdmittedWrite<'_>,
    track_identity_decision_id: i64,
    updated_at: i64,
) -> LibrarySqliteResult<usize> {
    write
        .execute(
            "UPDATE track_identity_decisions
             SET superseded_by_decision_id = NULL,
                 updated_at = ?2
             WHERE track_identity_decision_id = ?1",
            params![track_identity_decision_id, updated_at],
        )
        .map_err(Into::into)
}

fn normalized_decision_reason(reason: Option<String>, default_reason: &str) -> String {
    let Some(reason) = reason else {
        return default_reason.to_string();
    };
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return default_reason.to_string();
    }
    trimmed
        .chars()
        .take(MAX_TRACK_IDENTITY_DECISION_REASON_CHARS)
        .collect()
}

fn count_decision_evidence_snapshot_rows(
    connection: &rusqlite::Connection,
    track_identity_decision_id: i64,
) -> LibrarySqliteResult<usize> {
    connection
        .query_row(
            "SELECT COUNT(*)
             FROM track_identity_decision_evidence
             WHERE track_identity_decision_id = ?1",
            [track_identity_decision_id],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn count_stale_track_identity_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    connection
        .query_row(
            "SELECT COUNT(DISTINCT candidate.track_identity_candidate_id)
             FROM track_identity_candidates candidate
             JOIN track_identity_candidate_evidence evidence
               ON evidence.track_identity_candidate_id = candidate.track_identity_candidate_id
             WHERE evidence.source_id = ?1
               AND candidate.status != 'active'",
            [source_id],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn count_user_blocked_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    connection
        .query_row(
            "SELECT COUNT(DISTINCT candidate.track_identity_candidate_id)
             FROM track_identity_candidates candidate
             JOIN track_identity_candidate_evidence evidence
               ON evidence.track_identity_candidate_id = candidate.track_identity_candidate_id
             JOIN track_identity_decisions decision
               ON decision.track_identity_candidate_id = candidate.track_identity_candidate_id
             WHERE evidence.source_id = ?1
               AND decision.decision_source = ?2
               AND decision.decision_state IN (?3, ?4)
               AND decision.superseded_by_decision_id IS NULL",
            params![
                source_id,
                TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
                TRACK_IDENTITY_DECISION_STATE_REJECTED,
                TRACK_IDENTITY_DECISION_STATE_DEFERRED,
            ],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn count_existing_current_system_decisions(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    connection
        .query_row(
            "SELECT COUNT(DISTINCT decision.track_identity_candidate_id)
             FROM track_identity_decisions decision
             JOIN track_identity_decision_evidence evidence
               ON evidence.track_identity_decision_id = decision.track_identity_decision_id
             WHERE evidence.source_id = ?1
               AND decision.decision_source = ?2
               AND decision.superseded_by_decision_id IS NULL",
            params![
                source_id,
                TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
            ],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn read_track_identity_decision_production_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<Vec<TrackIdentityDecisionProductionCandidate>> {
    let current_evidence_predicate = current_track_identity_candidate_evidence_predicate("?2");
    connection
        .prepare(&format!(
            "SELECT DISTINCT candidate.track_identity_candidate_id,
                    candidate.candidate_kind,
                    candidate.evidence_basis,
                    candidate.evidence_key_algorithm,
                    candidate.evidence_key_value,
                    candidate.status
             FROM track_identity_candidates candidate
             JOIN track_identity_candidate_evidence evidence
               ON evidence.track_identity_candidate_id = candidate.track_identity_candidate_id
             JOIN source_files file
               ON file.source_file_id = evidence.source_file_id
             LEFT JOIN SourceFacts facts
               ON facts.source_file_id = evidence.source_file_id
             LEFT JOIN source_file_attachment_links link
               ON link.source_file_attachment_link_id =
                  evidence.source_file_attachment_link_id
             LEFT JOIN content_attachments attachment
               ON attachment.attachment_id = evidence.attachment_id
             WHERE evidence.source_id = ?1
               AND candidate.status = 'active'
               AND candidate.candidate_kind = ?3
               AND candidate.evidence_basis = ?4
               AND candidate.evidence_key_algorithm = ?2
               AND {current_evidence_predicate}
               AND NOT EXISTS (
                   SELECT 1
                   FROM track_identity_decisions decision
                   WHERE decision.track_identity_candidate_id =
                         candidate.track_identity_candidate_id
                     AND decision.decision_source = ?5
                     AND decision.superseded_by_decision_id IS NULL
               )
               AND NOT EXISTS (
                   SELECT 1
                   FROM track_identity_decisions user_decision
                   WHERE user_decision.track_identity_candidate_id =
                         candidate.track_identity_candidate_id
                     AND user_decision.decision_source = ?6
                     AND user_decision.decision_state IN (?7, ?8)
                     AND user_decision.superseded_by_decision_id IS NULL
               )
             ORDER BY candidate.track_identity_candidate_id ASC",
            current_evidence_predicate = current_evidence_predicate,
        ))?
        .query_map(
            params![
                source_id,
                SOURCE_FILE_BLAKE3_ALGORITHM,
                TRACK_IDENTITY_CANDIDATE_KIND,
                TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS,
                TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
                TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
                TRACK_IDENTITY_DECISION_STATE_REJECTED,
                TRACK_IDENTITY_DECISION_STATE_DEFERRED,
            ],
            |row| {
                Ok(TrackIdentityDecisionProductionCandidate {
                    track_identity_candidate_id: row.get(0)?,
                    candidate_kind: row.get(1)?,
                    candidate_evidence_basis: row.get(2)?,
                    evidence_key_algorithm: row.get(3)?,
                    evidence_key_value: row.get(4)?,
                    candidate_status: row.get(5)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn insert_system_exact_content_decision(
    write: &mut AdmittedWrite<'_>,
    candidate: &TrackIdentityDecisionProductionCandidate,
    decided_at: i64,
) -> LibrarySqliteResult<i64> {
    write.execute(
        "INSERT INTO track_identity_decisions (
             track_identity_candidate_id,
             decision_state,
             decision_source,
             decision_basis,
             decision_reason,
             candidate_kind,
             candidate_evidence_basis,
             candidate_status_at_decision,
             evidence_key_algorithm,
             evidence_key_value,
             created_at,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            candidate.track_identity_candidate_id,
            TRACK_IDENTITY_DECISION_STATE_ACCEPTED,
            TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
            TRACK_IDENTITY_DECISION_BASIS_SYSTEM_EXACT_CONTENT_V0,
            TRACK_IDENTITY_DECISION_REASON_SYSTEM_EXACT_CONTENT_V0,
            candidate.candidate_kind,
            candidate.candidate_evidence_basis,
            candidate.candidate_status,
            candidate.evidence_key_algorithm,
            candidate.evidence_key_value,
            decided_at,
        ],
    )?;

    write
        .query_row(
            "SELECT track_identity_decision_id
             FROM track_identity_decisions
             WHERE track_identity_candidate_id = ?1
               AND decision_source = ?2
               AND superseded_by_decision_id IS NULL",
            params![
                candidate.track_identity_candidate_id,
                TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
            ],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn insert_decision_evidence_snapshot(
    write: &mut AdmittedWrite<'_>,
    track_identity_decision_id: i64,
    track_identity_candidate_id: i64,
    decided_at: i64,
) -> LibrarySqliteResult<usize> {
    let current_evidence_predicate = current_track_identity_candidate_evidence_predicate("?4");
    write
        .execute(
            &format!(
                "INSERT INTO track_identity_decision_evidence (
                     track_identity_decision_id,
                     track_identity_candidate_id,
                     track_identity_candidate_member_id,
                     track_identity_candidate_evidence_id,
                     primary_media_candidate_id,
                     attachment_id,
                     source_file_attachment_link_id,
                     source_file_id,
                     source_id,
                     evidence_basis_fingerprint,
                     content_hash_algorithm,
                     content_hash_value,
                     probe_accepted_artifact_id,
                     created_at,
                     updated_at
                 )
                 SELECT ?1,
                        evidence.track_identity_candidate_id,
                        member.track_identity_candidate_member_id,
                        evidence.track_identity_candidate_evidence_id,
                        evidence.primary_media_candidate_id,
                        evidence.attachment_id,
                        evidence.source_file_attachment_link_id,
                        evidence.source_file_id,
                        evidence.source_id,
                        evidence.evidence_basis_fingerprint,
                        evidence.content_hash_algorithm,
                        evidence.content_hash_value,
                        evidence.probe_accepted_artifact_id,
                        ?3,
                        ?3
                 FROM track_identity_candidate_evidence evidence
                 JOIN track_identity_candidate_members member
                   ON member.track_identity_candidate_id =
                      evidence.track_identity_candidate_id
                  AND member.primary_media_candidate_id = evidence.primary_media_candidate_id
                 JOIN source_files file
                   ON file.source_file_id = evidence.source_file_id
                 LEFT JOIN SourceFacts facts
                   ON facts.source_file_id = evidence.source_file_id
                 LEFT JOIN source_file_attachment_links link
                   ON link.source_file_attachment_link_id =
                      evidence.source_file_attachment_link_id
                 LEFT JOIN content_attachments attachment
                   ON attachment.attachment_id = evidence.attachment_id
                 WHERE evidence.track_identity_candidate_id = ?2
                   AND {current_evidence_predicate}
                 ORDER BY evidence.source_id ASC,
                          evidence.source_file_id ASC",
            ),
            params![
                track_identity_decision_id,
                track_identity_candidate_id,
                decided_at,
                SOURCE_FILE_BLAKE3_ALGORITHM,
            ],
        )
        .map_err(Into::into)
}

fn insert_decision_source_scope(
    write: &mut AdmittedWrite<'_>,
    track_identity_decision_id: i64,
    track_identity_candidate_id: i64,
    decided_at: i64,
) -> LibrarySqliteResult<usize> {
    let count = insert_decision_source_scope_from_evidence_snapshot(
        write,
        track_identity_decision_id,
        decided_at,
    )?;
    if count > 0 {
        return Ok(count);
    }
    insert_decision_source_scope_from_candidate_provenance(
        write,
        track_identity_decision_id,
        track_identity_candidate_id,
        decided_at,
    )
}

fn insert_decision_source_scope_from_evidence_snapshot(
    write: &mut AdmittedWrite<'_>,
    track_identity_decision_id: i64,
    decided_at: i64,
) -> LibrarySqliteResult<usize> {
    write
        .execute(
            "INSERT INTO track_identity_decision_source_scope (
                 track_identity_decision_id,
                 track_identity_candidate_id,
                 source_id,
                 scope_basis,
                 created_at,
                 updated_at
             )
             SELECT DISTINCT ?1,
                    snapshot.track_identity_candidate_id,
                    snapshot.source_id,
                    'current_decision_evidence_source_v0',
                    ?3,
                    ?3
             FROM track_identity_decision_evidence snapshot
             WHERE snapshot.track_identity_decision_id = ?2",
            params![
                track_identity_decision_id,
                track_identity_decision_id,
                decided_at
            ],
        )
        .map_err(Into::into)
}

fn insert_decision_source_scope_from_candidate_provenance(
    write: &mut AdmittedWrite<'_>,
    track_identity_decision_id: i64,
    track_identity_candidate_id: i64,
    decided_at: i64,
) -> LibrarySqliteResult<usize> {
    write
        .execute(
            "INSERT INTO track_identity_decision_source_scope (
                 track_identity_decision_id,
                 track_identity_candidate_id,
                 source_id,
                 scope_basis,
                 created_at,
                 updated_at
             )
             SELECT DISTINCT ?1,
                    ?2,
                    evidence.source_id,
                    'candidate_source_provenance_v0',
                    ?4,
                    ?4
             FROM track_identity_candidate_evidence evidence
             WHERE evidence.track_identity_candidate_id = ?3",
            params![
                track_identity_decision_id,
                track_identity_candidate_id,
                track_identity_candidate_id,
                decided_at,
            ],
        )
        .map_err(Into::into)
}

fn read_count(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

fn map_store_decision_state(raw: &str) -> StoreTrackIdentityDecisionState {
    match raw {
        TRACK_IDENTITY_DECISION_STATE_ACCEPTED => StoreTrackIdentityDecisionState::Accepted,
        TRACK_IDENTITY_DECISION_STATE_REJECTED => StoreTrackIdentityDecisionState::Rejected,
        TRACK_IDENTITY_DECISION_STATE_DEFERRED => StoreTrackIdentityDecisionState::Deferred,
        _ => StoreTrackIdentityDecisionState::Superseded,
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use tempfile::TempDir;

    use crate::read_models::contents::{
        StoreContentsMediaClass, StoreContentsReadPolicy, StoreContentsRecursion,
        StoreContentsRowProfile, StoreContentsScope, StoreContentsState,
    };
    use crate::read_models::track_identity_decisions::{
        StoreTrackIdentityDecisionCurrentStatus, StoreTrackIdentityDecisionState,
        StoreTrackIdentityEffectiveDecisionCurrentStatus,
        StoreTrackIdentityEffectiveDecisionPrecedence, StoreTrackIdentityUserBlockingDecisionState,
    };
    use crate::{
        ProduceTrackIdentityDecisionsForSourceResult, SqliteDurableStore,
        TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0, TrackIdentityDecisionChangeFailure,
        TrackIdentityDecisionChangeResult,
    };

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    struct TrackIdentityDecisionFixture {
        _tempdir: TempDir,
        store: SqliteDurableStore,
        source_id: i64,
    }

    impl TrackIdentityDecisionFixture {
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
                         VALUES (?1, 'internal', 'system', 'source:track-decision-test', 'Track Decision Test', 1, 1)",
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
            let media_class = crate::browse_media::media_class_str_from_path(relative_path);
            let relative_path_browse_sort_key =
                crate::browse_sort_key::compute_relative_path_browse_sort_key(relative_path);
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO source_files (
                             source_file_id,
                             source_id,
                             name,
                             name_browse_sort_key,
                             relative_path_browse_sort_key,
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
                         VALUES (?1, ?2, ?3, '', ?4, ?5, 10, 100, ?6, ?7, 'present', 1, 1, 1, 1, 1)",
                        params![
                            source_file_id,
                            self.source_id,
                            file_name,
                            relative_path_browse_sort_key,
                            relative_path,
                            file_kind,
                            media_class
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert source file");
        }

        fn commit_current_facts(&self, source_file_id: i64, hash_value: &str) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT OR IGNORE INTO WorkItems (
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
                         VALUES (1, 'source_file', 'fixture', 'inspect_source', 'fixture', 'completed', 'interactive', 1, 1)",
                        [],
                    )?;
                    write.execute(
                        "INSERT OR IGNORE INTO WorkRuns (
                             work_run_id,
                             work_item_id,
                             adapter_key,
                             adapter_version,
                             started_at,
                             outcome
                         )
                         VALUES (1, 1, 'test.track_identity_decision', '1', 1, 'ok')",
                        [],
                    )?;
                    let artifact_id = 10_000 + source_file_id;
                    write.execute(
                        "INSERT OR REPLACE INTO Artifacts (
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
                         VALUES (?1, 1, 'source_file', ?2, 'inspection_result', 'primary_result', 'test.track_identity_decision', '1', ?3, 'application/json', 'inline_payload', ?4, 1)",
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
                         VALUES (?1, 'source_inspection', ?2, ?3, ?4, ?5, ?6, ?7, 1, 'blake3', ?8, 'audio', 'audio/wav', 100, 44100, 2, 16, 'pcm', 1, ?9)
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
                            hash_value,
                            artifact_id,
                        ],
                    )?;
                    Ok(())
                })
                .expect("commit facts");
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

        fn promote_and_candidate(&self) {
            self.store
                .promote_primary_media_for_source(self.source_id, 10)
                .expect("promote primary media");
            self.store
                .produce_track_identity_candidates_for_source(self.source_id, 10)
                .expect("produce track identity candidates");
        }

        fn produce_decisions(&self, limit: usize) -> ProduceTrackIdentityDecisionsForSourceResult {
            self.store
                .produce_track_identity_decisions_for_source(self.source_id, limit)
                .expect("produce track identity decisions")
        }

        fn single_candidate_id(&self) -> i64 {
            let candidates = self
                .store
                .read_track_identity_candidates_for_source(self.source_id, 10)
                .expect("read candidates");
            assert_eq!(candidates.len(), 1);
            candidates[0].track_identity_candidate_id
        }

        fn accept_candidate(
            &self,
            track_identity_candidate_id: i64,
        ) -> TrackIdentityDecisionChangeResult {
            self.store
                .accept_track_identity_candidate(
                    track_identity_candidate_id,
                    Some(" user accepted ".to_string()),
                )
                .expect("accept candidate")
        }

        fn reject_candidate(
            &self,
            track_identity_candidate_id: i64,
        ) -> TrackIdentityDecisionChangeResult {
            self.store
                .reject_track_identity_candidate(
                    track_identity_candidate_id,
                    Some(" user rejected ".to_string()),
                )
                .expect("reject candidate")
        }

        fn defer_candidate(
            &self,
            track_identity_candidate_id: i64,
        ) -> TrackIdentityDecisionChangeResult {
            self.store
                .defer_track_identity_candidate(
                    track_identity_candidate_id,
                    Some(" user deferred ".to_string()),
                )
                .expect("defer candidate")
        }

        fn expect_written(
            result: TrackIdentityDecisionChangeResult,
        ) -> crate::TrackIdentityDecisionChangeSuccess {
            match result {
                TrackIdentityDecisionChangeResult::Written(success) => success,
                TrackIdentityDecisionChangeResult::Failed(failure) => {
                    panic!("expected written decision, got {failure:?}")
                }
            }
        }

        fn expect_failure(
            result: TrackIdentityDecisionChangeResult,
        ) -> TrackIdentityDecisionChangeFailure {
            match result {
                TrackIdentityDecisionChangeResult::Failed(failure) => failure,
                TrackIdentityDecisionChangeResult::Written(success) => {
                    panic!("expected decision failure, got {success:?}")
                }
            }
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

        fn count_table_if_exists(&self, table: &str) -> Option<i64> {
            let connection = self.store.open_read_connection().expect("open read");
            let exists = connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM sqlite_master
                     WHERE type = 'table'
                       AND name = ?1",
                    [table],
                    |row| row.get::<_, i64>(0),
                )
                .expect("check table")
                > 0;
            exists.then(|| {
                connection
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .expect("count table")
            })
        }

        fn change_file_basis(&self, source_file_id: i64) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "UPDATE source_files
                         SET size_bytes = size_bytes + 1,
                             mtime_ns = mtime_ns + 1,
                             updated_at = updated_at + 1
                         WHERE source_file_id = ?1",
                        [source_file_id],
                    )?;
                    Ok(())
                })
                .expect("change file basis");
        }
    }

    #[test]
    fn active_candidate_receives_system_exact_content_decision_with_provenance() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        let attachment_id = fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();

        let result = fixture.produce_decisions(10);

        assert_eq!(result.decisions_created, 1);
        assert_eq!(result.decision_evidence_created, 1);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 1);
        assert_eq!(fixture.count_rows("track_identity_decision_evidence"), 1);

        let candidates = fixture
            .store
            .read_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("read candidates");
        let candidate = &candidates[0];
        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions");
        assert_eq!(decisions.len(), 1);
        let decision = &decisions[0];
        assert_eq!(
            decision.track_identity_candidate_id,
            candidate.track_identity_candidate_id
        );
        assert_eq!(
            decision.decision_state,
            StoreTrackIdentityDecisionState::Accepted
        );
        assert_eq!(decision.decision_source, "system_exact_content_v0");
        assert_eq!(
            decision.current_status,
            StoreTrackIdentityDecisionCurrentStatus::Current
        );
        assert_eq!(decision.evidence_key_algorithm, "blake3");
        assert_eq!(decision.evidence_key_value, HASH_A);
        assert!(
            decision
                .proves
                .contains("exact-content track identity candidate")
        );
        assert!(decision.does_not_prove.contains("canonical track identity"));
        assert_eq!(decision.evidence.len(), 1);
        let evidence = &decision.evidence[0];
        assert_eq!(
            evidence.track_identity_candidate_member_id,
            candidate.members[0].track_identity_candidate_member_id
        );
        assert_eq!(
            evidence.track_identity_candidate_evidence_id,
            candidate.evidence[0].track_identity_candidate_evidence_id
        );
        assert_eq!(evidence.attachment_id, attachment_id);
        assert_eq!(evidence.source_file_id, 100);
        assert_eq!(evidence.content_hash_value, HASH_A);
    }

    #[test]
    fn user_accept_creates_accepted_decision_with_evidence_snapshot() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();

        let success =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));

        assert_eq!(success.track_identity_candidate_id, candidate_id);
        assert_eq!(
            success.decision_state,
            StoreTrackIdentityDecisionState::Accepted
        );
        assert_eq!(success.decision_source, "user_local_v0");
        assert_eq!(success.evidence_snapshot_count, 1);
        assert!(success.decision_created);
        assert_eq!(
            success.effective_decision.effective_decision_id,
            Some(success.track_identity_decision_id)
        );
        assert_eq!(
            success.effective_decision.effective_decision_current_status,
            StoreTrackIdentityEffectiveDecisionCurrentStatus::Current
        );
        assert_eq!(
            success.effective_decision.effective_decision_precedence,
            StoreTrackIdentityEffectiveDecisionPrecedence::User
        );

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_candidate(candidate_id, 10)
            .expect("read candidate decisions");
        assert_eq!(decisions.len(), 1);
        assert_eq!(
            decisions[0].decision_state,
            StoreTrackIdentityDecisionState::Accepted
        );
        assert_eq!(decisions[0].decision_reason, "user accepted");
        assert_eq!(decisions[0].evidence.len(), 1);
        assert_eq!(
            decisions[0]
                .candidate_effective_decision
                .effective_decision_id,
            Some(success.track_identity_decision_id)
        );
    }

    #[test]
    fn user_reject_and_defer_create_explicit_user_decisions() {
        let reject_fixture = TrackIdentityDecisionFixture::new();
        reject_fixture.insert_source_file(100, "Album/reject.wav");
        reject_fixture.link_attachment(100, HASH_A);
        reject_fixture.commit_current_facts(100, HASH_A);
        reject_fixture.promote_and_candidate();
        let reject_candidate_id = reject_fixture.single_candidate_id();

        let rejected = TrackIdentityDecisionFixture::expect_written(
            reject_fixture.reject_candidate(reject_candidate_id),
        );

        assert_eq!(
            rejected.decision_state,
            StoreTrackIdentityDecisionState::Rejected
        );
        assert_eq!(rejected.decision_source, "user_local_v0");
        assert_eq!(rejected.evidence_snapshot_count, 1);

        let defer_fixture = TrackIdentityDecisionFixture::new();
        defer_fixture.insert_source_file(100, "Album/defer.wav");
        defer_fixture.link_attachment(100, HASH_A);
        defer_fixture.commit_current_facts(100, HASH_A);
        defer_fixture.promote_and_candidate();
        let defer_candidate_id = defer_fixture.single_candidate_id();

        let deferred = TrackIdentityDecisionFixture::expect_written(
            defer_fixture.defer_candidate(defer_candidate_id),
        );

        assert_eq!(
            deferred.decision_state,
            StoreTrackIdentityDecisionState::Deferred
        );
        assert_eq!(deferred.decision_source, "user_local_v0");
        assert_eq!(deferred.evidence_snapshot_count, 1);
    }

    #[test]
    fn user_reject_blocks_system_decision_from_being_effective() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/reject-blocks.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let system = fixture.produce_decisions(10);
        assert_eq!(system.decisions_created, 1);

        let rejected =
            TrackIdentityDecisionFixture::expect_written(fixture.reject_candidate(candidate_id));

        assert_eq!(
            rejected.decision_state,
            StoreTrackIdentityDecisionState::Rejected
        );
        let effective = fixture
            .store
            .read_effective_track_identity_decision_for_candidate(candidate_id)
            .expect("read effective decision");
        assert_eq!(
            effective.effective_decision_id,
            Some(rejected.track_identity_decision_id)
        );
        assert_eq!(
            effective.effective_decision_state,
            Some(StoreTrackIdentityDecisionState::Rejected)
        );
        assert_eq!(
            effective.effective_decision_precedence,
            StoreTrackIdentityEffectiveDecisionPrecedence::User
        );
        assert_eq!(
            effective.user_blocking_decision_state,
            StoreTrackIdentityUserBlockingDecisionState::Rejected
        );
        assert!(
            effective.masked_system_decision_id.is_some(),
            "existing current system decision should be identified as masked"
        );
    }

    #[test]
    fn user_defer_blocks_system_decision_from_being_effective() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/defer-blocks.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let system = fixture.produce_decisions(10);
        assert_eq!(system.decisions_created, 1);

        let deferred =
            TrackIdentityDecisionFixture::expect_written(fixture.defer_candidate(candidate_id));

        let effective = fixture
            .store
            .read_effective_track_identity_decision_for_candidate(candidate_id)
            .expect("read effective decision");
        assert_eq!(
            effective.effective_decision_id,
            Some(deferred.track_identity_decision_id)
        );
        assert_eq!(
            effective.effective_decision_state,
            Some(StoreTrackIdentityDecisionState::Deferred)
        );
        assert_eq!(
            effective.user_blocking_decision_state,
            StoreTrackIdentityUserBlockingDecisionState::Deferred
        );
        assert!(
            effective.masked_system_decision_id.is_some(),
            "existing current system decision should be identified as masked"
        );
    }

    #[test]
    fn user_accept_after_reject_supersedes_reject_and_preserves_history() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/user-correction.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();

        let rejected =
            TrackIdentityDecisionFixture::expect_written(fixture.reject_candidate(candidate_id));
        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));

        assert_ne!(
            rejected.track_identity_decision_id,
            accepted.track_identity_decision_id
        );
        let repeated =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert_eq!(
            repeated.track_identity_decision_id, accepted.track_identity_decision_id,
            "same-source same-state user decision is idempotent"
        );
        assert!(!repeated.decision_created);

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_candidate(candidate_id, 10)
            .expect("read candidate decisions");
        assert_eq!(decisions.len(), 2);
        let historical_reject = &decisions[0];
        assert_eq!(
            historical_reject.decision_state,
            StoreTrackIdentityDecisionState::Rejected
        );
        assert_eq!(
            historical_reject.superseded_by_decision_id,
            Some(accepted.track_identity_decision_id)
        );
        assert_eq!(
            historical_reject.current_status,
            StoreTrackIdentityDecisionCurrentStatus::Superseded
        );
        assert_eq!(
            historical_reject
                .candidate_effective_decision
                .effective_decision_id,
            Some(accepted.track_identity_decision_id)
        );
        assert_ne!(
            historical_reject
                .candidate_effective_decision
                .effective_decision_id,
            Some(rejected.track_identity_decision_id),
            "superseded user decision remains historical but not effective"
        );
        assert_eq!(historical_reject.evidence.len(), 1);
        let current_accept = &decisions[1];
        assert_eq!(
            current_accept.decision_state,
            StoreTrackIdentityDecisionState::Accepted
        );
        assert_eq!(current_accept.superseded_by_decision_id, None);
        assert_eq!(
            current_accept
                .candidate_effective_decision
                .effective_decision_precedence,
            StoreTrackIdentityEffectiveDecisionPrecedence::User
        );
        let current_user_decisions = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .query_row(
                "SELECT COUNT(*)
                 FROM track_identity_decisions
                 WHERE track_identity_candidate_id = ?1
                   AND decision_source = ?2
                   AND superseded_by_decision_id IS NULL
                   AND decision_state != 'superseded'",
                params![candidate_id, TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0],
                |row| row.get::<_, i64>(0),
            )
            .expect("count current user decisions");
        assert_eq!(current_user_decisions, 1);
    }

    #[test]
    fn system_maintenance_does_not_override_current_user_blocking_decision() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/user-blocked.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let deferred =
            TrackIdentityDecisionFixture::expect_written(fixture.defer_candidate(candidate_id));

        let result = fixture.produce_decisions(10);

        assert_eq!(result.decisions_created, 0);
        assert_eq!(result.decision_evidence_created, 0);
        assert_eq!(result.skipped_user_blocked_candidates, 1);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 1);
        let effective = fixture
            .store
            .read_effective_track_identity_decision_for_candidate(candidate_id)
            .expect("read effective decision");
        assert_eq!(
            effective.effective_decision_id,
            Some(deferred.track_identity_decision_id)
        );
        assert_eq!(
            effective.effective_decision_precedence,
            StoreTrackIdentityEffectiveDecisionPrecedence::User
        );
        assert_eq!(
            effective.user_blocking_decision_state,
            StoreTrackIdentityUserBlockingDecisionState::Deferred
        );
        assert_eq!(effective.masked_system_decision_id, None);
    }

    #[test]
    fn user_accept_requires_active_candidate_with_current_evidence() {
        let missing_fixture = TrackIdentityDecisionFixture::new();
        let missing_failure =
            TrackIdentityDecisionFixture::expect_failure(missing_fixture.accept_candidate(99_999));
        assert_eq!(
            missing_failure,
            TrackIdentityDecisionChangeFailure::CandidateNotFound
        );

        let stale_fixture = TrackIdentityDecisionFixture::new();
        stale_fixture.insert_source_file(100, "Album/stale-accept.wav");
        stale_fixture.link_attachment(100, HASH_A);
        stale_fixture.commit_current_facts(100, HASH_A);
        stale_fixture.promote_and_candidate();
        let stale_candidate_id = stale_fixture.single_candidate_id();
        stale_fixture.change_file_basis(100);
        stale_fixture
            .store
            .produce_track_identity_candidates_for_source(stale_fixture.source_id, 10)
            .expect("mark candidate stale");

        let stale_failure = TrackIdentityDecisionFixture::expect_failure(
            stale_fixture.accept_candidate(stale_candidate_id),
        );
        assert_eq!(
            stale_failure,
            TrackIdentityDecisionChangeFailure::CandidateStaleForAccept
        );

        let no_evidence_fixture = TrackIdentityDecisionFixture::new();
        no_evidence_fixture.insert_source_file(100, "Album/no-current-evidence.wav");
        no_evidence_fixture.link_attachment(100, HASH_A);
        no_evidence_fixture.commit_current_facts(100, HASH_A);
        no_evidence_fixture.promote_and_candidate();
        let no_evidence_candidate_id = no_evidence_fixture.single_candidate_id();
        no_evidence_fixture.change_file_basis(100);

        let no_evidence_failure = TrackIdentityDecisionFixture::expect_failure(
            no_evidence_fixture.accept_candidate(no_evidence_candidate_id),
        );
        assert_eq!(
            no_evidence_failure,
            TrackIdentityDecisionChangeFailure::NoCurrentEvidenceForAccept
        );
    }

    #[test]
    fn user_reject_and_defer_do_not_fabricate_stale_evidence_snapshots() {
        let reject_fixture = TrackIdentityDecisionFixture::new();
        reject_fixture.insert_source_file(100, "Album/reject-no-evidence.wav");
        reject_fixture.link_attachment(100, HASH_A);
        reject_fixture.commit_current_facts(100, HASH_A);
        reject_fixture.promote_and_candidate();
        let reject_candidate_id = reject_fixture.single_candidate_id();
        reject_fixture.change_file_basis(100);

        let rejected = TrackIdentityDecisionFixture::expect_written(
            reject_fixture.reject_candidate(reject_candidate_id),
        );
        assert_eq!(rejected.evidence_snapshot_count, 0);

        let defer_fixture = TrackIdentityDecisionFixture::new();
        defer_fixture.insert_source_file(100, "Album/defer-stale-no-evidence.wav");
        defer_fixture.link_attachment(100, HASH_A);
        defer_fixture.commit_current_facts(100, HASH_A);
        defer_fixture.promote_and_candidate();
        let defer_candidate_id = defer_fixture.single_candidate_id();
        defer_fixture.change_file_basis(100);
        defer_fixture
            .store
            .produce_track_identity_candidates_for_source(defer_fixture.source_id, 10)
            .expect("mark candidate stale");

        let deferred = TrackIdentityDecisionFixture::expect_written(
            defer_fixture.defer_candidate(defer_candidate_id),
        );
        assert_eq!(deferred.evidence_snapshot_count, 0);
        let decisions = defer_fixture
            .store
            .read_track_identity_decisions_for_candidate(defer_candidate_id, 10)
            .expect("read candidate decisions");
        assert_eq!(decisions[0].evidence.len(), 0);
        assert_eq!(
            decisions[0]
                .candidate_effective_decision
                .effective_decision_current_status,
            StoreTrackIdentityEffectiveDecisionCurrentStatus::Stale
        );
    }

    #[test]
    fn decision_evidence_snapshots_are_immutable_after_later_facts_change() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/immutable.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert_eq!(accepted.evidence_snapshot_count, 1);

        fixture.commit_current_facts(100, HASH_B);
        fixture.link_attachment(100, HASH_B);

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_candidate(candidate_id, 10)
            .expect("read candidate decisions");
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].evidence.len(), 1);
        assert_eq!(
            decisions[0].evidence[0].content_hash_value, HASH_A,
            "old decision evidence snapshot must not be rewritten by later facts"
        );
    }

    #[test]
    fn decision_evidence_snapshot_survives_live_candidate_evidence_delete() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/copied-provenance.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert_eq!(accepted.evidence_snapshot_count, 1);

        let candidate_evidence_id = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .query_row(
                "SELECT track_identity_candidate_evidence_id
                 FROM track_identity_decision_evidence
                 WHERE track_identity_decision_id = ?1",
                [accepted.track_identity_decision_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("read copied evidence id");
        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_candidate_evidence
                     WHERE track_identity_candidate_evidence_id = ?1",
                    [candidate_evidence_id],
                )?;
                Ok(())
            })
            .expect("delete live candidate evidence");

        assert_eq!(fixture.count_rows("track_identity_candidate_evidence"), 0);
        assert_eq!(
            fixture.count_rows("track_identity_decision_evidence"),
            1,
            "historical decision evidence must not cascade from live candidate evidence"
        );
    }

    #[test]
    fn stale_candidate_cannot_receive_new_accepted_decision() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/stale.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        fixture.change_file_basis(100);
        fixture
            .store
            .produce_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("mark candidate stale");

        let result = fixture.produce_decisions(10);

        assert_eq!(result.decisions_created, 0);
        assert_eq!(result.decision_evidence_created, 0);
        assert_eq!(result.skipped_stale_candidates, 1);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 0);
    }

    #[test]
    fn repeated_decision_production_does_not_duplicate_current_decision() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();

        let first = fixture.produce_decisions(10);
        let second = fixture.produce_decisions(10);

        assert_eq!(first.decisions_created, 1);
        assert_eq!(second.decisions_created, 0);
        assert_eq!(second.skipped_existing_current_decisions, 1);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 1);
        assert_eq!(fixture.count_rows("track_identity_decision_evidence"), 1);
    }

    #[test]
    fn decision_production_does_not_group_by_path_or_equivalence_fingerprint() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.insert_source_file(101, "Album/track copy.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.link_attachment(101, HASH_B);
        fixture.commit_current_facts(100, HASH_A);
        fixture.commit_current_facts(101, HASH_B);
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
                     VALUES (1, 'same-looking-track', 1, 1)",
                    [],
                )?;
                Ok(())
            })
            .expect("insert legacy asset");
        fixture.promote_and_candidate();

        fixture.produce_decisions(10);

        assert_eq!(fixture.count_rows("track_identity_candidates"), 2);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 2);
        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions");
        let mut hashes = decisions
            .iter()
            .map(|decision| decision.evidence_key_value.as_str())
            .collect::<Vec<_>>();
        hashes.sort_unstable();
        assert_eq!(hashes, vec![HASH_A, HASH_B]);
    }

    #[test]
    fn decision_production_leaves_contents_cue_metadata_and_prep_surfaces_unchanged() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.insert_source_file(101, "Album/album.cue");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        fixture.produce_decisions(10);

        let result = fixture
            .store
            .read_contents(
                StoreContentsScope::Source {
                    source_id: fixture.source_id,
                },
                StoreContentsReadPolicy {
                    media_classes: vec![
                        StoreContentsMediaClass::Audio,
                        StoreContentsMediaClass::Video,
                        StoreContentsMediaClass::Image,
                        StoreContentsMediaClass::Unsupported,
                    ],
                    row_profile: StoreContentsRowProfile::SourceFile,
                },
                StoreContentsRecursion::Recursive,
                10,
                None,
            )
            .expect("read contents");
        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 2);
        assert!(result.rows.iter().all(|row| row.primary_media.is_none()));

        for table in [
            "LibraryAssetAttachments",
            "LibraryBrowserRows",
            "SourceSegmentSets",
            "SourceSegments",
            "PrepAssignments",
            "ResolvedLibraryAssetPrepTargets",
            "Playlists",
            "PlaylistEntries",
        ] {
            assert_eq!(fixture.count_rows(table), 0, "{table} must remain empty");
        }
        for absent_or_future_table in [
            "canonical_tracks",
            "tracks",
            "library_tracks",
            "track_identities",
            "Tracks",
            "TrackRows",
            "LibraryTracks",
            "CueAudioAssociations",
            "Waveforms",
            "Stems",
            "PrepRows",
            "PreparationRows",
        ] {
            assert!(
                matches!(
                    fixture.count_table_if_exists(absent_or_future_table),
                    None | Some(0)
                ),
                "{absent_or_future_table} must be absent or empty"
            );
        }

        let columns = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .prepare("PRAGMA table_info(track_identity_decisions)")
            .expect("prepare pragma")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query pragma")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect columns");
        for forbidden in ["title", "artist", "album", "bpm", "key"] {
            assert!(
                !columns.iter().any(|column| column == forbidden),
                "decision schema must not contain metadata column {forbidden}"
            );
        }
    }

    #[test]
    fn decision_evidence_snapshot_includes_only_current_evidence_rows() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.insert_source_file(101, "Album/track copy.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.link_attachment(101, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.commit_current_facts(101, HASH_A);
        fixture.promote_and_candidate();

        assert_eq!(
            fixture.count_rows("track_identity_candidate_evidence"),
            2,
            "both source files share the same hash and group into one candidate"
        );

        fixture.change_file_basis(100);
        fixture
            .store
            .produce_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("re-produce candidates to mark file 100 evidence stale");

        assert_eq!(
            fixture.count_rows("track_identity_candidate_evidence"),
            2,
            "stale evidence row is preserved but no longer current"
        );

        let result = fixture.produce_decisions(10);
        assert_eq!(result.decisions_created, 1);
        assert_eq!(
            result.decision_evidence_created, 1,
            "only the current evidence row must be snapshotted"
        );

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions");
        assert_eq!(decisions.len(), 1);
        let evidence = &decisions[0].evidence;
        assert_eq!(
            evidence.len(),
            1,
            "decision evidence must contain only the current supporting evidence row"
        );
        assert_eq!(
            evidence[0].source_file_id, 101,
            "snapshot evidence must reference the still-current source file"
        );
    }

    #[test]
    fn source_scoped_historical_decision_survives_live_candidate_evidence_deletion() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/source-scoped-survival.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert_eq!(accepted.evidence_snapshot_count, 1);

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_candidate_evidence
                     WHERE track_identity_candidate_id = ?1",
                    [candidate_id],
                )?;
                Ok(())
            })
            .expect("delete live candidate evidence");

        assert_eq!(fixture.count_rows("track_identity_candidate_evidence"), 0);
        assert_eq!(
            fixture.count_rows("track_identity_decision_evidence"),
            1,
            "historical decision evidence must survive live candidate evidence deletion"
        );

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions for source");
        assert_eq!(
            decisions.len(),
            1,
            "historical decision must still appear in source-scoped read after live candidate evidence is deleted"
        );
        assert_eq!(
            decisions[0].current_status,
            StoreTrackIdentityDecisionCurrentStatus::Stale,
            "decision current_status must be stale because live candidate evidence is gone"
        );
        assert_eq!(
            decisions[0].evidence.len(),
            1,
            "snapshot evidence must still appear after live candidate evidence is deleted"
        );
        assert_eq!(
            decisions[0].evidence[0].content_hash_value, HASH_A,
            "snapshot evidence content hash must be preserved"
        );
    }

    #[test]
    fn candidate_scoped_historical_decision_survives_live_candidate_deletion() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/candidate-scoped-survival.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert_eq!(accepted.evidence_snapshot_count, 1);

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_candidates
                     WHERE track_identity_candidate_id = ?1",
                    [candidate_id],
                )?;
                Ok(())
            })
            .expect("delete live candidate");

        assert_eq!(fixture.count_rows("track_identity_candidates"), 0);
        assert_eq!(
            fixture.count_rows("track_identity_candidate_members"),
            0,
            "candidate members cascade-delete with candidate"
        );
        assert_eq!(
            fixture.count_rows("track_identity_candidate_evidence"),
            0,
            "candidate evidence cascade-deletes with candidate"
        );
        assert_eq!(
            fixture.count_rows("track_identity_decision_evidence"),
            1,
            "historical decision evidence must survive live candidate deletion"
        );

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_candidate(candidate_id, 10)
            .expect("read decisions for candidate");
        assert_eq!(
            decisions.len(),
            1,
            "historical decision must still appear in candidate-scoped read after candidate is deleted"
        );
        assert_eq!(
            decisions[0].current_status,
            StoreTrackIdentityDecisionCurrentStatus::Stale,
            "decision current_status must be stale because candidate is gone"
        );
        assert!(
            decisions[0]
                .candidate_effective_decision
                .effective_decision_id
                .is_some(),
            "candidate_effective_decision must be present when there is a current decision row"
        );
        assert_eq!(
            decisions[0]
                .candidate_effective_decision
                .effective_decision_current_status,
            StoreTrackIdentityEffectiveDecisionCurrentStatus::Stale,
            "effective_decision_current_status must be stale because candidate is gone"
        );
    }

    #[test]
    fn effective_decision_status_tolerates_missing_live_candidate() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/effective-stale-missing.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert_eq!(
            accepted
                .effective_decision
                .effective_decision_current_status,
            StoreTrackIdentityEffectiveDecisionCurrentStatus::Current
        );

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_candidates
                     WHERE track_identity_candidate_id = ?1",
                    [candidate_id],
                )?;
                Ok(())
            })
            .expect("delete live candidate");

        let effective = fixture
            .store
            .read_effective_track_identity_decision_for_candidate(candidate_id)
            .expect("read effective decision after candidate deletion");

        assert_eq!(
            effective.effective_decision_id,
            Some(accepted.track_identity_decision_id),
            "effective decision id must still resolve"
        );
        assert_eq!(
            effective.effective_decision_current_status,
            StoreTrackIdentityEffectiveDecisionCurrentStatus::Stale,
            "effective_decision_current_status must be stale because candidate is gone"
        );
        assert_eq!(
            effective.effective_decision_precedence,
            StoreTrackIdentityEffectiveDecisionPrecedence::User
        );
    }

    #[test]
    fn snapshot_evidence_not_cascade_deleted_by_live_candidate_deletion() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/snapshot-survival.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        fixture.produce_decisions(10);
        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert!(accepted.evidence_snapshot_count > 0);

        let decision_evidence_count_before = fixture.count_rows("track_identity_decision_evidence");
        assert!(
            decision_evidence_count_before > 0,
            "snapshot evidence must exist before candidate deletion"
        );

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_candidates
                     WHERE track_identity_candidate_id = ?1",
                    [candidate_id],
                )?;
                Ok(())
            })
            .expect("delete live candidate");

        assert_eq!(fixture.count_rows("track_identity_candidates"), 0);
        assert_eq!(
            fixture.count_rows("track_identity_candidate_evidence"),
            0,
            "live candidate evidence must cascade-delete"
        );
        assert_eq!(
            fixture.count_rows("track_identity_decision_evidence"),
            decision_evidence_count_before,
            "snapshot evidence rows must survive live candidate cascade-deletion"
        );
    }

    #[test]
    fn no_canonical_track_shell_exists_in_schema() {
        let fixture = TrackIdentityDecisionFixture::new();

        for forbidden_table in [
            "canonical_tracks",
            "tracks",
            "library_tracks",
            "track_identities",
        ] {
            assert!(
                fixture.count_table_if_exists(forbidden_table).is_none(),
                "forbidden table {forbidden_table} must be absent"
            );
        }
    }

    #[test]
    fn reject_with_zero_evidence_still_source_readable_through_source_scope() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/reject-zero-evidence.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();

        fixture.change_file_basis(100);
        fixture
            .store
            .produce_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("mark candidate stale");

        let rejected =
            TrackIdentityDecisionFixture::expect_written(fixture.reject_candidate(candidate_id));

        assert_eq!(rejected.evidence_snapshot_count, 0);
        assert_eq!(
            fixture.count_rows("track_identity_decision_evidence"),
            0,
            "no decision evidence rows for zero-current-evidence reject"
        );
        let source_scope_count = fixture.count_rows("track_identity_decision_source_scope");
        assert!(
            source_scope_count >= 1,
            "source scope must be populated from candidate provenance, got {source_scope_count}"
        );

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions for source");
        assert_eq!(
            decisions.len(),
            1,
            "reject decision must appear in source-scoped read"
        );
        assert_eq!(
            decisions[0].current_status,
            StoreTrackIdentityDecisionCurrentStatus::Stale
        );
        let effective = fixture
            .store
            .read_effective_track_identity_decision_for_candidate(candidate_id)
            .expect("read effective decision");
        assert_eq!(
            effective.effective_decision_id,
            Some(rejected.track_identity_decision_id)
        );
        assert_eq!(
            effective.effective_decision_current_status,
            StoreTrackIdentityEffectiveDecisionCurrentStatus::Stale
        );
        assert_eq!(
            effective.effective_decision_precedence,
            StoreTrackIdentityEffectiveDecisionPrecedence::User
        );
        assert_eq!(
            effective.user_blocking_decision_state,
            StoreTrackIdentityUserBlockingDecisionState::Rejected
        );

        let scope_bases = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .prepare(
                "SELECT DISTINCT scope_basis
                 FROM track_identity_decision_source_scope
                 WHERE track_identity_decision_id = ?1",
            )
            .expect("prepare scope basis query")
            .query_map([rejected.track_identity_decision_id], |row| {
                row.get::<_, String>(0)
            })
            .expect("query scope bases")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect scope bases");
        assert_eq!(
            scope_bases,
            vec!["candidate_source_provenance_v0".to_string()]
        );
    }

    #[test]
    fn defer_with_zero_evidence_still_source_readable_through_source_scope() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/defer-zero-evidence.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();

        fixture.change_file_basis(100);
        fixture
            .store
            .produce_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("mark candidate stale");

        let deferred =
            TrackIdentityDecisionFixture::expect_written(fixture.defer_candidate(candidate_id));

        assert_eq!(deferred.evidence_snapshot_count, 0);
        assert_eq!(fixture.count_rows("track_identity_decision_evidence"), 0);

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions for source");
        assert_eq!(
            decisions.len(),
            1,
            "defer decision with zero evidence must appear in source-scoped read"
        );
        assert_eq!(
            decisions[0].current_status,
            StoreTrackIdentityDecisionCurrentStatus::Stale
        );
        assert_eq!(
            decisions[0]
                .candidate_effective_decision
                .user_blocking_decision_state,
            StoreTrackIdentityUserBlockingDecisionState::Deferred
        );
    }

    #[test]
    fn source_scope_survives_live_candidate_deletion() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/source-scope-survival.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let _accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));

        assert!(
            fixture.count_rows("track_identity_decision_source_scope") >= 1,
            "source scope must exist after accept"
        );

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_candidates
                     WHERE track_identity_candidate_id = ?1",
                    [candidate_id],
                )?;
                Ok(())
            })
            .expect("delete live candidate");

        assert_eq!(fixture.count_rows("track_identity_candidates"), 0);
        assert_eq!(fixture.count_rows("track_identity_candidate_evidence"), 0);
        assert!(
            fixture.count_rows("track_identity_decision_source_scope") >= 1,
            "source scope must survive live candidate cascade-deletion"
        );

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions for source");
        assert_eq!(
            decisions.len(),
            1,
            "decision must still appear in source-scoped read after candidate deletion"
        );
        assert_eq!(
            decisions[0].current_status,
            StoreTrackIdentityDecisionCurrentStatus::Stale
        );
    }

    #[test]
    fn accepted_decision_has_both_evidence_snapshot_and_source_scope() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/accept.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();

        let system = fixture.produce_decisions(10);
        assert_eq!(system.decisions_created, 1);
        assert_eq!(system.decision_evidence_created, 1);
        assert_eq!(
            fixture.count_rows("track_identity_decision_source_scope"),
            1,
            "system decision must have source scope"
        );

        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert!(accepted.evidence_snapshot_count >= 1);
        assert!(
            fixture.count_rows("track_identity_decision_source_scope") >= 2,
            "user accept must also have source scope"
        );

        let scope_rows = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .prepare(
                "SELECT scope_basis, source_id
                 FROM track_identity_decision_source_scope
                 WHERE track_identity_decision_id = ?1",
            )
            .expect("prepare scope query")
            .query_map([accepted.track_identity_decision_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .expect("query scope rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect scope rows");
        assert_eq!(scope_rows.len(), 1);
        assert_eq!(scope_rows[0].0, "current_decision_evidence_source_v0");
        assert_eq!(scope_rows[0].1, fixture.source_id);
    }

    #[test]
    fn reject_and_defer_with_no_source_scope_return_typed_failure_and_create_no_rows() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/no-source-scope.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_candidate_evidence
                     WHERE track_identity_candidate_id = ?1",
                    [candidate_id],
                )?;
                Ok(())
            })
            .expect("delete candidate evidence");

        assert_eq!(fixture.count_rows("track_identity_candidate_evidence"), 0);
        assert_eq!(fixture.count_rows("track_identity_candidates"), 1);

        let rejected = fixture.reject_candidate(candidate_id);
        let failure = TrackIdentityDecisionFixture::expect_failure(rejected);
        assert_eq!(
            failure,
            TrackIdentityDecisionChangeFailure::NoSourceScopeForDecision
        );

        assert_eq!(
            fixture.count_rows("track_identity_decisions"),
            0,
            "no decision row must be created on no-source-scope failure"
        );
        assert_eq!(
            fixture.count_rows("track_identity_decision_evidence"),
            0,
            "no decision evidence row must be created on no-source-scope failure"
        );
        assert_eq!(
            fixture.count_rows("track_identity_decision_source_scope"),
            0,
            "no source-scope row must be created on no-source-scope failure"
        );

        let deferred = fixture.defer_candidate(candidate_id);
        let failure2 = TrackIdentityDecisionFixture::expect_failure(deferred);
        assert_eq!(
            failure2,
            TrackIdentityDecisionChangeFailure::NoSourceScopeForDecision
        );

        assert_eq!(
            fixture.count_rows("track_identity_decisions"),
            0,
            "no decision row must remain after no-source-scope defer failure"
        );
        assert_eq!(
            fixture.count_rows("track_identity_decision_evidence"),
            0,
            "no decision evidence row must remain after no-source-scope defer failure"
        );
        assert_eq!(
            fixture.count_rows("track_identity_decision_source_scope"),
            0,
            "no source-scope row must remain after no-source-scope defer failure"
        );
    }

    #[test]
    fn no_source_scope_reject_does_not_supersede_prior_decision() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/prior-decision.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();

        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));
        assert!(accepted.decision_created);

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_candidate_evidence
                     WHERE track_identity_candidate_id = ?1",
                    [candidate_id],
                )?;
                Ok(())
            })
            .expect("delete candidate evidence");

        let rejected = fixture.reject_candidate(candidate_id);
        let failure = TrackIdentityDecisionFixture::expect_failure(rejected);
        assert_eq!(
            failure,
            TrackIdentityDecisionChangeFailure::NoSourceScopeForDecision
        );

        let superseded: Option<i64> = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .query_row(
                "SELECT superseded_by_decision_id
                 FROM track_identity_decisions
                 WHERE track_identity_decision_id = ?1",
                [accepted.track_identity_decision_id],
                |row| row.get(0),
            )
            .expect("read superseded_by");
        assert_eq!(
            superseded, None,
            "prior decision must not be superseded by failed no-source-scope reject"
        );
        assert_eq!(
            fixture.count_rows("track_identity_decision_evidence"),
            1,
            "prior decision evidence snapshot must be intact"
        );
        assert_eq!(
            fixture.count_rows("track_identity_decisions"),
            1,
            "only the prior decision must exist"
        );
    }

    #[test]
    fn multi_source_candidate_yields_multi_source_scope() {
        let fixture = TrackIdentityDecisionFixture::new();

        fixture.insert_source_file(100, "Album/source-a.wav");
        let attachment_id = fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);

        let source2_id: i64 = 2;

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "INSERT INTO sources (
                         source_id, source_class, authority, identity_key,
                         display_name, created_at, updated_at
                     )
                     VALUES (2, 'internal', 'system', 'source:test-multi',
                             'Multi Source', 1, 1)",
                    [],
                )?;
                write.execute(
                    "INSERT INTO source_state (
                         source_id, mount_status, mount_epoch, access_state,
                         access_checked_at, effective_path, updated_at
                     )
                     VALUES (2, 'mounted', 1, 'accessible', 1, 'root', 1)",
                    [],
                )?;
                write.execute(
                    "INSERT INTO source_scan_state (
                         source_id, scan_phase, last_scan_started_at,
                         last_scan_finished_at, last_successful_scan_at,
                         updated_at
                     )
                     VALUES (2, 'complete', 1, 2, 2, 2)",
                    [],
                )?;
                let src_b_rpath = "Album/source-b.wav";
                let src_b_rpath_key =
                    crate::browse_sort_key::compute_relative_path_browse_sort_key(src_b_rpath);
                write.execute(
                    "INSERT INTO source_files (
                         source_file_id, source_id, name, name_browse_sort_key,
                         relative_path_browse_sort_key, relative_path,
                         size_bytes, mtime_ns, file_kind, media_class,
                         presence_state, first_discovered_at,
                         last_observed_at, last_presence_change_at,
                         created_at, updated_at
                     )
                      VALUES (?1, 2, 'source-b.wav', 'v1|tstotutrctet-tbt.twtatv', ?2, ?3,
                             10, 100, 'audio', 'audio', 'present', 1, 1, 1,
                             1, 1)",
                    params![200, src_b_rpath_key, src_b_rpath],
                )?;
                write.execute(
                    "INSERT OR IGNORE INTO WorkItems (
                         work_item_id, subject_kind, subject_id, work_kind,
                         basis_fingerprint, state, priority_class,
                         created_at, updated_at
                     )
                     VALUES (10, 'source_file', 'fixture-multi',
                             'inspect_source', 'fixture-multi',
                             'completed', 'interactive', 1, 1)",
                    [],
                )?;
                write.execute(
                    "INSERT OR IGNORE INTO WorkRuns (
                         work_run_id, work_item_id, adapter_key,
                         adapter_version, started_at, outcome
                     )
                     VALUES (10, 10, 'test.multi_source', '1', 1, 'ok')",
                    [],
                )?;
                write.execute(
                    "INSERT OR REPLACE INTO Artifacts (
                         artifact_id, work_run_id, subject_kind,
                         subject_id, artifact_kind, artifact_role,
                         adapter_key, adapter_version, basis_fingerprint,
                         media_type, storage_kind, payload_hash, created_at
                     )
                     VALUES (20000, 10, 'source_file', '200',
                             'inspection_result', 'primary_result',
                             'test.multi_source', '1',
                             'basis:200', 'application/json',
                             'inline_payload', 'payload:200', 1)",
                    [],
                )?;
                write.execute(
                    "INSERT INTO SourceFacts (
                         source_file_id, fact_kind, basis_fingerprint,
                         basis_source_id, basis_relative_path,
                         basis_size_bytes, basis_mtime_ns,
                         basis_presence_state, observed_at_ms,
                         content_hash_algorithm, content_hash_value,
                         media_kind, mime_type, duration_ms,
                         sample_rate_hz, channels, bit_depth, codec,
                         updated_at, accepted_artifact_id
                     )
                     VALUES (200, 'source_inspection', 'basis:200', 2,
                             'Album/source-b.wav', 10, 100, 'present', 1,
                             'blake3', ?1, 'audio', 'audio/wav', 100,
                             44100, 2, 16, 'pcm', 1, 20000)
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
                    params![HASH_A],
                )?;
                write.execute(
                    "INSERT OR REPLACE INTO source_file_attachment_links (
                         attachment_id, source_file_id, source_id,
                         file_kind, created_at, updated_at
                     )
                     VALUES (?1, 200, 2, 'audio', 1, 1)",
                    params![attachment_id],
                )?;
                Ok(())
            })
            .expect("set up source 2");

        fixture.promote_and_candidate();
        assert_eq!(fixture.count_rows("track_identity_candidates"), 1);

        fixture
            .store
            .promote_primary_media_for_source(source2_id, 10)
            .expect("promote source 2");
        fixture
            .store
            .produce_track_identity_candidates_for_source(source2_id, 10)
            .expect("candidates source 2");

        assert_eq!(
            fixture.count_rows("track_identity_candidates"),
            1,
            "same hash groups into same candidate"
        );
        assert_eq!(
            fixture.count_rows("track_identity_candidate_evidence"),
            2,
            "candidate must have evidence from both sources"
        );

        let result = fixture.produce_decisions(10);
        assert_eq!(result.decisions_created, 1);

        let scope_rows = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .prepare(
                "SELECT source_id, scope_basis
                 FROM track_identity_decision_source_scope
                 ORDER BY source_id ASC",
            )
            .expect("prepare scope query")
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query scope rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect scope rows");

        assert_eq!(
            scope_rows.len(),
            2,
            "source scope must contain both source ids, got {scope_rows:?}"
        );
        assert_eq!(scope_rows[0].0, 1);
        assert_eq!(scope_rows[1].0, 2);
        assert_eq!(scope_rows[0].1, "current_decision_evidence_source_v0");

        let decisions_a = fixture
            .store
            .read_track_identity_decisions_for_source(1, 10)
            .expect("read decisions for source 1");
        let decisions_b = fixture
            .store
            .read_track_identity_decisions_for_source(2, 10)
            .expect("read decisions for source 2");
        assert_eq!(decisions_a.len(), 1);
        assert_eq!(decisions_b.len(), 1);
        assert_eq!(
            decisions_a[0].track_identity_decision_id, decisions_b[0].track_identity_decision_id,
            "both source-scoped reads must return the same decision"
        );

        let distinct_scope_count: i64 = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .query_row(
                "SELECT COUNT(*)
                 FROM (
                     SELECT DISTINCT track_identity_decision_id, source_id
                     FROM track_identity_decision_source_scope
                 )",
                [],
                |row| row.get(0),
            )
            .expect("count distinct scope rows");
        assert_eq!(
            distinct_scope_count, 2,
            "unique guard must prevent duplicate (decision_id, source_id) scope rows"
        );
    }

    #[test]
    fn source_scope_has_no_live_fk_to_candidate_or_evidence() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/fk-check.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        let candidate_id = fixture.single_candidate_id();
        let accepted =
            TrackIdentityDecisionFixture::expect_written(fixture.accept_candidate(candidate_id));

        let foreign_keys = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .prepare("PRAGMA foreign_key_list(track_identity_decision_source_scope)")
            .expect("prepare pragma")
            .query_map([], |row| {
                Ok((row.get::<_, String>(2)?, row.get::<_, String>(3)?))
            })
            .expect("query foreign keys")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect foreign keys");

        assert_eq!(
            foreign_keys,
            vec![(
                "track_identity_decisions".to_string(),
                "track_identity_decision_id".to_string(),
            )],
            "only track_identity_decisions should be FK parent for source scope"
        );

        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM track_identity_decisions
                     WHERE track_identity_decision_id = ?1",
                    [accepted.track_identity_decision_id],
                )?;
                Ok(())
            })
            .expect("delete decision");

        assert_eq!(
            fixture.count_rows("track_identity_decision_source_scope"),
            0,
            "source scope must cascade-delete with decision"
        );
    }
}
