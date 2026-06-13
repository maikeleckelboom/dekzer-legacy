use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::LibrarySqliteResult;
use crate::read_models::track_identity_candidates::StoreTrackIdentityCandidateStatus;
use crate::store::{
    SOURCE_FILE_BLAKE3_ALGORITHM, TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
    TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
};
use crate::track_identity_evidence_predicates::current_track_identity_candidate_evidence_predicate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreTrackIdentityDecision {
    pub track_identity_decision_id: i64,
    pub track_identity_candidate_id: i64,
    pub decision_state: StoreTrackIdentityDecisionState,
    pub decision_source: String,
    pub decision_basis: String,
    pub decision_reason: String,
    pub candidate_kind: String,
    pub candidate_evidence_basis: String,
    pub candidate_status_at_decision: StoreTrackIdentityCandidateStatus,
    pub evidence_key_algorithm: String,
    pub evidence_key_value: String,
    pub current_status: StoreTrackIdentityDecisionCurrentStatus,
    pub candidate_effective_decision: StoreTrackIdentityEffectiveDecisionSummary,
    pub proves: String,
    pub does_not_prove: String,
    pub superseded_by_decision_id: Option<i64>,
    pub evidence: Vec<StoreTrackIdentityDecisionEvidence>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreTrackIdentityDecisionState {
    Accepted,
    Rejected,
    Deferred,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreTrackIdentityDecisionCurrentStatus {
    Current,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreTrackIdentityEffectiveDecisionCurrentStatus {
    Current,
    Stale,
    NoCurrentDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreTrackIdentityEffectiveDecisionPrecedence {
    User,
    System,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreTrackIdentityUserBlockingDecisionState {
    None,
    Rejected,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreTrackIdentityEffectiveDecisionSummary {
    pub effective_decision_id: Option<i64>,
    pub effective_decision_state: Option<StoreTrackIdentityDecisionState>,
    pub effective_decision_source: Option<String>,
    pub effective_decision_current_status: StoreTrackIdentityEffectiveDecisionCurrentStatus,
    pub effective_decision_precedence: StoreTrackIdentityEffectiveDecisionPrecedence,
    pub user_blocking_decision_state: StoreTrackIdentityUserBlockingDecisionState,
    pub masked_system_decision_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreTrackIdentityDecisionEvidence {
    pub track_identity_decision_evidence_id: i64,
    pub track_identity_decision_id: i64,
    pub track_identity_candidate_id: i64,
    pub track_identity_candidate_member_id: i64,
    pub track_identity_candidate_evidence_id: i64,
    pub playable_media_id: i64,
    pub attachment_id: i64,
    pub source_file_attachment_link_id: i64,
    pub source_file_id: i64,
    pub source_id: i64,
    pub evidence_basis_fingerprint: String,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    pub probe_accepted_artifact_id: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn read_track_identity_decisions_for_source(
    connection: &Connection,
    source_id: i64,
    limit: usize,
) -> LibrarySqliteResult<Vec<StoreTrackIdentityDecision>> {
    let limit_i64 =
        i64::try_from(limit).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?;
    let current_evidence_predicate = current_track_identity_candidate_evidence_predicate("?2");
    let mut decisions = connection
        .prepare(&format!(
            "SELECT decision.track_identity_decision_id,
                    decision.track_identity_candidate_id,
                    decision.decision_state,
                    decision.decision_source,
                    decision.decision_basis,
                    decision.decision_reason,
                    decision.candidate_kind,
                    decision.candidate_evidence_basis,
                    decision.candidate_status_at_decision,
                    decision.evidence_key_algorithm,
                    decision.evidence_key_value,
                    CASE
                        WHEN decision.superseded_by_decision_id IS NOT NULL
                          OR decision.decision_state = 'superseded'
                        THEN 'superseded'
                        WHEN candidate.track_identity_candidate_id IS NOT NULL
                         AND candidate.status = 'active'
                         AND EXISTS (
                             SELECT 1
                             FROM track_identity_candidate_evidence evidence
                             JOIN source_files file
                               ON file.source_file_id = evidence.source_file_id
                             LEFT JOIN source_file_observations observations
                               ON observations.source_file_id = evidence.source_file_id
                             LEFT JOIN source_file_attachment_links link
                               ON link.source_file_attachment_link_id =
                                  evidence.source_file_attachment_link_id
                             LEFT JOIN content_attachments attachment
                               ON attachment.attachment_id = evidence.attachment_id
                             WHERE evidence.track_identity_candidate_id =
                                   decision.track_identity_candidate_id
                               AND {current_evidence_predicate}
                         )
                        THEN 'current'
                        ELSE 'stale'
                    END AS current_status,
                    decision.superseded_by_decision_id,
                    decision.created_at,
                    decision.updated_at
             FROM track_identity_decisions decision
             LEFT JOIN track_identity_candidates candidate
               ON candidate.track_identity_candidate_id = decision.track_identity_candidate_id
             WHERE EXISTS (
                 SELECT 1
                 FROM track_identity_decision_source_scope scope
                 WHERE scope.track_identity_decision_id = decision.track_identity_decision_id
                   AND scope.source_id = ?1
             )
             ORDER BY decision.track_identity_decision_id ASC
             LIMIT ?3",
            current_evidence_predicate = current_evidence_predicate,
        ))?
        .query_map(
            params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM, limit_i64],
            map_decision_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    hydrate_decisions(connection, &mut decisions)?;
    Ok(decisions)
}

pub fn read_track_identity_decisions_for_candidate(
    connection: &Connection,
    track_identity_candidate_id: i64,
    limit: usize,
) -> LibrarySqliteResult<Vec<StoreTrackIdentityDecision>> {
    let limit_i64 =
        i64::try_from(limit).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?;
    let current_evidence_predicate = current_track_identity_candidate_evidence_predicate("?2");
    let mut decisions = connection
        .prepare(&format!(
            "SELECT decision.track_identity_decision_id,
                    decision.track_identity_candidate_id,
                    decision.decision_state,
                    decision.decision_source,
                    decision.decision_basis,
                    decision.decision_reason,
                    decision.candidate_kind,
                    decision.candidate_evidence_basis,
                    decision.candidate_status_at_decision,
                    decision.evidence_key_algorithm,
                    decision.evidence_key_value,
                    CASE
                        WHEN decision.superseded_by_decision_id IS NOT NULL
                          OR decision.decision_state = 'superseded'
                        THEN 'superseded'
                        WHEN candidate.track_identity_candidate_id IS NOT NULL
                         AND candidate.status = 'active'
                         AND EXISTS (
                             SELECT 1
                             FROM track_identity_candidate_evidence evidence
                             JOIN source_files file
                               ON file.source_file_id = evidence.source_file_id
                             LEFT JOIN source_file_observations observations
                               ON observations.source_file_id = evidence.source_file_id
                             LEFT JOIN source_file_attachment_links link
                               ON link.source_file_attachment_link_id =
                                  evidence.source_file_attachment_link_id
                             LEFT JOIN content_attachments attachment
                               ON attachment.attachment_id = evidence.attachment_id
                             WHERE evidence.track_identity_candidate_id =
                                   decision.track_identity_candidate_id
                               AND {current_evidence_predicate}
                         )
                        THEN 'current'
                        ELSE 'stale'
                    END AS current_status,
                    decision.superseded_by_decision_id,
                    decision.created_at,
                    decision.updated_at
             FROM track_identity_decisions decision
             LEFT JOIN track_identity_candidates candidate
               ON candidate.track_identity_candidate_id = decision.track_identity_candidate_id
             WHERE decision.track_identity_candidate_id = ?1
             ORDER BY decision.track_identity_decision_id ASC
             LIMIT ?3",
            current_evidence_predicate = current_evidence_predicate,
        ))?
        .query_map(
            params![
                track_identity_candidate_id,
                SOURCE_FILE_BLAKE3_ALGORITHM,
                limit_i64
            ],
            map_decision_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    hydrate_decisions(connection, &mut decisions)?;
    Ok(decisions)
}

pub fn read_effective_track_identity_decision_for_candidate(
    connection: &Connection,
    track_identity_candidate_id: i64,
) -> LibrarySqliteResult<StoreTrackIdentityEffectiveDecisionSummary> {
    let current_user_decision = read_current_decision_for_source(
        connection,
        track_identity_candidate_id,
        TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
    )?;
    let current_system_decision = read_current_decision_for_source(
        connection,
        track_identity_candidate_id,
        TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
    )?;

    let Some(effective_decision) = current_user_decision.or(current_system_decision.clone()) else {
        return Ok(StoreTrackIdentityEffectiveDecisionSummary {
            effective_decision_id: None,
            effective_decision_state: None,
            effective_decision_source: None,
            effective_decision_current_status:
                StoreTrackIdentityEffectiveDecisionCurrentStatus::NoCurrentDecision,
            effective_decision_precedence: StoreTrackIdentityEffectiveDecisionPrecedence::None,
            user_blocking_decision_state: StoreTrackIdentityUserBlockingDecisionState::None,
            masked_system_decision_id: None,
        });
    };

    let effective_decision_current_status =
        read_effective_decision_current_status(connection, track_identity_candidate_id)?;
    let effective_decision_precedence =
        if effective_decision.decision_source == TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0 {
            StoreTrackIdentityEffectiveDecisionPrecedence::User
        } else {
            StoreTrackIdentityEffectiveDecisionPrecedence::System
        };
    let user_blocking_decision_state = user_blocking_decision_state(&effective_decision);
    let masked_system_decision_id =
        if user_blocking_decision_state != StoreTrackIdentityUserBlockingDecisionState::None {
            current_system_decision
                .as_ref()
                .map(|decision| decision.track_identity_decision_id)
        } else {
            None
        };

    Ok(StoreTrackIdentityEffectiveDecisionSummary {
        effective_decision_id: Some(effective_decision.track_identity_decision_id),
        effective_decision_state: Some(effective_decision.decision_state),
        effective_decision_source: Some(effective_decision.decision_source),
        effective_decision_current_status,
        effective_decision_precedence,
        user_blocking_decision_state,
        masked_system_decision_id,
    })
}

fn hydrate_decisions(
    connection: &Connection,
    decisions: &mut [StoreTrackIdentityDecision],
) -> LibrarySqliteResult<()> {
    for decision in decisions {
        decision.evidence =
            read_track_identity_decision_evidence(connection, decision.track_identity_decision_id)?;
        let effective = read_effective_track_identity_decision_for_candidate(
            connection,
            decision.track_identity_candidate_id,
        )?;
        decision.candidate_effective_decision = effective;
    }

    Ok(())
}

fn map_decision_row(row: &Row<'_>) -> rusqlite::Result<StoreTrackIdentityDecision> {
    Ok(StoreTrackIdentityDecision {
        track_identity_decision_id: row.get(0)?,
        track_identity_candidate_id: row.get(1)?,
        decision_state: map_decision_state(row.get::<_, String>(2)?.as_str()),
        decision_source: row.get(3)?,
        decision_basis: row.get(4)?,
        decision_reason: row.get(5)?,
        candidate_kind: row.get(6)?,
        candidate_evidence_basis: row.get(7)?,
        candidate_status_at_decision: map_candidate_status(row.get::<_, String>(8)?.as_str()),
        evidence_key_algorithm: row.get(9)?,
        evidence_key_value: row.get(10)?,
        current_status: map_current_status(row.get::<_, String>(11)?.as_str()),
        candidate_effective_decision: StoreTrackIdentityEffectiveDecisionSummary {
            effective_decision_id: None,
            effective_decision_state: None,
            effective_decision_source: None,
            effective_decision_current_status:
                StoreTrackIdentityEffectiveDecisionCurrentStatus::NoCurrentDecision,
            effective_decision_precedence: StoreTrackIdentityEffectiveDecisionPrecedence::None,
            user_blocking_decision_state: StoreTrackIdentityUserBlockingDecisionState::None,
            masked_system_decision_id: None,
        },
        proves: "a backend-owned decision classified the exact-content track identity candidate under its recorded evidence basis".to_string(),
        does_not_prove: "canonical track identity, same-song semantic identity, metadata reconciliation, CUE association, or multi-encode equivalence".to_string(),
        superseded_by_decision_id: row.get(12)?,
        evidence: Vec::new(),
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentDecision {
    track_identity_decision_id: i64,
    decision_state: StoreTrackIdentityDecisionState,
    decision_source: String,
}

fn read_current_decision_for_source(
    connection: &Connection,
    track_identity_candidate_id: i64,
    decision_source: &str,
) -> LibrarySqliteResult<Option<CurrentDecision>> {
    connection
        .query_row(
            "SELECT track_identity_decision_id,
                    decision_state,
                    decision_source
             FROM track_identity_decisions
             WHERE track_identity_candidate_id = ?1
               AND decision_source = ?2
               AND superseded_by_decision_id IS NULL
               AND decision_state != 'superseded'",
            params![track_identity_candidate_id, decision_source],
            |row| {
                Ok(CurrentDecision {
                    track_identity_decision_id: row.get(0)?,
                    decision_state: map_decision_state(row.get::<_, String>(1)?.as_str()),
                    decision_source: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn read_effective_decision_current_status(
    connection: &Connection,
    track_identity_candidate_id: i64,
) -> LibrarySqliteResult<StoreTrackIdentityEffectiveDecisionCurrentStatus> {
    let current_evidence_predicate = current_track_identity_candidate_evidence_predicate("?2");
    connection
        .query_row(
            &format!(
                "SELECT CASE
                    WHEN EXISTS (
                         SELECT 1
                         FROM track_identity_candidates c
                         WHERE c.track_identity_candidate_id = ?1
                           AND c.status = 'active'
                     )
                     AND EXISTS (
                         SELECT 1
                         FROM track_identity_candidate_evidence evidence
                         JOIN source_files file
                           ON file.source_file_id = evidence.source_file_id
                         LEFT JOIN source_file_observations observations
                           ON observations.source_file_id = evidence.source_file_id
                         LEFT JOIN source_file_attachment_links link
                           ON link.source_file_attachment_link_id =
                              evidence.source_file_attachment_link_id
                         LEFT JOIN content_attachments attachment
                           ON attachment.attachment_id = evidence.attachment_id
                         WHERE evidence.track_identity_candidate_id = ?1
                           AND {current_evidence_predicate}
                     )
                    THEN 'current'
                    ELSE 'stale'
                 END",
                current_evidence_predicate = current_evidence_predicate,
            ),
            params![track_identity_candidate_id, SOURCE_FILE_BLAKE3_ALGORITHM],
            |row| {
                Ok(match row.get::<_, String>(0)?.as_str() {
                    "current" => StoreTrackIdentityEffectiveDecisionCurrentStatus::Current,
                    _ => StoreTrackIdentityEffectiveDecisionCurrentStatus::Stale,
                })
            },
        )
        .map_err(Into::into)
}

fn user_blocking_decision_state(
    effective_decision: &CurrentDecision,
) -> StoreTrackIdentityUserBlockingDecisionState {
    if effective_decision.decision_source != TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0 {
        return StoreTrackIdentityUserBlockingDecisionState::None;
    }

    match effective_decision.decision_state {
        StoreTrackIdentityDecisionState::Rejected => {
            StoreTrackIdentityUserBlockingDecisionState::Rejected
        }
        StoreTrackIdentityDecisionState::Deferred => {
            StoreTrackIdentityUserBlockingDecisionState::Deferred
        }
        StoreTrackIdentityDecisionState::Accepted | StoreTrackIdentityDecisionState::Superseded => {
            StoreTrackIdentityUserBlockingDecisionState::None
        }
    }
}

fn read_track_identity_decision_evidence(
    connection: &Connection,
    track_identity_decision_id: i64,
) -> LibrarySqliteResult<Vec<StoreTrackIdentityDecisionEvidence>> {
    connection
        .prepare(
            "SELECT track_identity_decision_evidence_id,
                    track_identity_decision_id,
                    track_identity_candidate_id,
                    track_identity_candidate_member_id,
                    track_identity_candidate_evidence_id,
                    playable_media_id,
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
             FROM track_identity_decision_evidence
             WHERE track_identity_decision_id = ?1
             ORDER BY source_id ASC,
                      source_file_id ASC,
                      track_identity_candidate_evidence_id ASC",
        )?
        .query_map([track_identity_decision_id], map_evidence_row)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn map_evidence_row(row: &Row<'_>) -> rusqlite::Result<StoreTrackIdentityDecisionEvidence> {
    Ok(StoreTrackIdentityDecisionEvidence {
        track_identity_decision_evidence_id: row.get(0)?,
        track_identity_decision_id: row.get(1)?,
        track_identity_candidate_id: row.get(2)?,
        track_identity_candidate_member_id: row.get(3)?,
        track_identity_candidate_evidence_id: row.get(4)?,
        playable_media_id: row.get(5)?,
        attachment_id: row.get(6)?,
        source_file_attachment_link_id: row.get(7)?,
        source_file_id: row.get(8)?,
        source_id: row.get(9)?,
        evidence_basis_fingerprint: row.get(10)?,
        content_hash_algorithm: row.get(11)?,
        content_hash_value: row.get(12)?,
        probe_accepted_artifact_id: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn map_decision_state(raw: &str) -> StoreTrackIdentityDecisionState {
    match raw {
        "accepted" => StoreTrackIdentityDecisionState::Accepted,
        "rejected" => StoreTrackIdentityDecisionState::Rejected,
        "deferred" => StoreTrackIdentityDecisionState::Deferred,
        _ => StoreTrackIdentityDecisionState::Superseded,
    }
}

fn map_current_status(raw: &str) -> StoreTrackIdentityDecisionCurrentStatus {
    match raw {
        "current" => StoreTrackIdentityDecisionCurrentStatus::Current,
        "superseded" => StoreTrackIdentityDecisionCurrentStatus::Superseded,
        _ => StoreTrackIdentityDecisionCurrentStatus::Stale,
    }
}

fn map_candidate_status(raw: &str) -> StoreTrackIdentityCandidateStatus {
    match raw {
        "active" => StoreTrackIdentityCandidateStatus::Active,
        "superseded" => StoreTrackIdentityCandidateStatus::Superseded,
        _ => StoreTrackIdentityCandidateStatus::Stale,
    }
}
