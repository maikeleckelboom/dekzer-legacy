use rusqlite::{Connection, Row};

use crate::LibrarySqliteResult;
use crate::read_models::track_identity_candidates::StoreTrackIdentityCandidateStatus;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreTrackIdentityDecisionEvidence {
    pub track_identity_decision_evidence_id: i64,
    pub track_identity_decision_id: i64,
    pub track_identity_candidate_id: i64,
    pub track_identity_candidate_member_id: i64,
    pub track_identity_candidate_evidence_id: i64,
    pub primary_media_candidate_id: i64,
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
    let mut decisions = connection
        .prepare(
            "SELECT DISTINCT decision.track_identity_decision_id,
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
                        WHEN candidate.status = 'active' THEN 'current'
                        ELSE 'stale'
                    END AS current_status,
                    decision.superseded_by_decision_id,
                    decision.created_at,
                    decision.updated_at
             FROM track_identity_decisions decision
             JOIN track_identity_decision_evidence evidence
               ON evidence.track_identity_decision_id = decision.track_identity_decision_id
             LEFT JOIN track_identity_candidates candidate
               ON candidate.track_identity_candidate_id = decision.track_identity_candidate_id
             WHERE evidence.source_id = ?1
             ORDER BY decision.track_identity_decision_id ASC
             LIMIT ?2",
        )?
        .query_map(rusqlite::params![source_id, limit_i64], |row| {
            Ok(StoreTrackIdentityDecision {
                track_identity_decision_id: row.get(0)?,
                track_identity_candidate_id: row.get(1)?,
                decision_state: map_decision_state(row.get::<_, String>(2)?.as_str()),
                decision_source: row.get(3)?,
                decision_basis: row.get(4)?,
                decision_reason: row.get(5)?,
                candidate_kind: row.get(6)?,
                candidate_evidence_basis: row.get(7)?,
                candidate_status_at_decision: map_candidate_status(
                    row.get::<_, String>(8)?.as_str(),
                ),
                evidence_key_algorithm: row.get(9)?,
                evidence_key_value: row.get(10)?,
                current_status: map_current_status(row.get::<_, String>(11)?.as_str()),
                proves: "a backend-owned decision accepted the exact-content track identity candidate under its recorded evidence basis".to_string(),
                does_not_prove: "canonical track identity, same-song semantic identity, metadata reconciliation, CUE association, or multi-encode equivalence".to_string(),
                superseded_by_decision_id: row.get(12)?,
                evidence: Vec::new(),
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for decision in &mut decisions {
        decision.evidence =
            read_track_identity_decision_evidence(connection, decision.track_identity_decision_id)?;
    }

    Ok(decisions)
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
        primary_media_candidate_id: row.get(5)?,
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
