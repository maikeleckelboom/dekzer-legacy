use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::store::{
    SOURCE_FILE_BLAKE3_ALGORITHM, TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
    TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
};
use crate::track_identity_evidence_predicates::current_track_identity_candidate_evidence_predicate;
use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::track_identity_candidates::StoreTrackIdentityCandidateStatus;
use super::track_identity_decisions::{
    StoreTrackIdentityDecisionState, StoreTrackIdentityEffectiveDecisionCurrentStatus,
    StoreTrackIdentityUserBlockingDecisionState,
    read_effective_track_identity_decision_for_candidate,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCandidate {
    pub candidate_id: i64,
    pub candidate_kind: String,
    pub candidate_evidence_basis: String,
    pub candidate_status: StoreTrackIdentityCandidateStatus,
    pub evidence_key_algorithm: String,
    pub evidence_key_value: String,
    pub evidence_summary: ReviewEvidenceSummary,
    pub source_summary: ReviewSourceSummary,
    pub review_state: ReviewState,
    pub effective_decision: Option<ReviewDecision>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewDecision {
    pub decision_id: i64,
    pub decision_state: StoreTrackIdentityDecisionState,
    pub decision_source: String,
    pub decision_basis: String,
    pub current_status: StoreTrackIdentityEffectiveDecisionCurrentStatus,
    pub created_at: i64,
    pub user_blocking_decision_state: StoreTrackIdentityUserBlockingDecisionState,
    pub masked_system_decision_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewEvidenceSummary {
    pub member_count: usize,
    pub evidence_count: usize,
    pub current_evidence_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSourceSummary {
    pub source_count: usize,
    pub source_samples: Vec<ReviewSourceSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSourceSample {
    pub source_id: i64,
    pub display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    NeedsUserDecision,
    SystemAccepted,
    UserAccepted,
    UserRejected,
    UserDeferred,
    StaleDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FactualCandidate {
    candidate_id: i64,
    candidate_kind: String,
    candidate_evidence_basis: String,
    candidate_status: StoreTrackIdentityCandidateStatus,
    evidence_key_algorithm: String,
    evidence_key_value: String,
    evidence_summary: ReviewEvidenceSummary,
    source_count: usize,
    created_at: i64,
    updated_at: i64,
}

struct EffectiveDecisionDetails {
    decision_basis: String,
    created_at: i64,
}

pub fn read_track_identity_review_candidates(
    connection: &Connection,
    source_id: Option<i64>,
    review_state_filter: Option<ReviewState>,
    limit: usize,
) -> LibrarySqliteResult<Vec<ReviewCandidate>> {
    // TODO(track-identity-review-cursor): add a V1 cursor with version,
    // scope/filter identity, and the last candidate_id position.
    let mut candidates = read_factual_candidates(connection, source_id)?;
    let mut review_candidates = Vec::new();

    for candidate in candidates.drain(..) {
        let candidate = hydrate_review_candidate(connection, candidate)?;
        if review_state_filter.is_none_or(|filter| candidate.review_state == filter) {
            review_candidates.push(candidate);
        }

        if review_candidates.len() == limit {
            break;
        }
    }

    Ok(review_candidates)
}

fn read_factual_candidates(
    connection: &Connection,
    source_id: Option<i64>,
) -> LibrarySqliteResult<Vec<FactualCandidate>> {
    let current_evidence_predicate = current_track_identity_candidate_evidence_predicate("?2");
    connection
        .prepare(&format!(
            "SELECT candidate.track_identity_candidate_id,
                    candidate.candidate_kind,
                    candidate.evidence_basis,
                    candidate.status,
                    candidate.evidence_key_algorithm,
                    candidate.evidence_key_value,
                    (
                        SELECT COUNT(*)
                        FROM track_identity_candidate_members member
                        WHERE member.track_identity_candidate_id =
                              candidate.track_identity_candidate_id
                    ) AS member_count,
                    (
                        SELECT COUNT(*)
                        FROM track_identity_candidate_evidence evidence
                        WHERE evidence.track_identity_candidate_id =
                              candidate.track_identity_candidate_id
                    ) AS evidence_count,
                    (
                        SELECT COUNT(DISTINCT evidence.source_id)
                        FROM track_identity_candidate_evidence evidence
                        WHERE evidence.track_identity_candidate_id =
                              candidate.track_identity_candidate_id
                    ) AS source_count,
                    (
                        SELECT COUNT(*)
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
                        WHERE evidence.track_identity_candidate_id =
                              candidate.track_identity_candidate_id
                          AND {current_evidence_predicate}
                    ) AS current_evidence_count,
                    candidate.created_at,
                    candidate.updated_at
             FROM track_identity_candidates candidate
             WHERE (?1 IS NULL
                OR EXISTS (
                    SELECT 1
                    FROM track_identity_candidate_evidence source_filter
                    WHERE source_filter.track_identity_candidate_id =
                          candidate.track_identity_candidate_id
                      AND source_filter.source_id = ?1
                ))
             ORDER BY candidate.track_identity_candidate_id ASC",
            current_evidence_predicate = current_evidence_predicate,
        ))?
        .query_map(params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM], |row| {
            map_factual_candidate_row(row)
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn hydrate_review_candidate(
    connection: &Connection,
    candidate: FactualCandidate,
) -> LibrarySqliteResult<ReviewCandidate> {
    let effective =
        read_effective_track_identity_decision_for_candidate(connection, candidate.candidate_id)?;
    let effective_decision = match effective.effective_decision_id {
        Some(decision_id) => {
            let decision_state = effective.effective_decision_state.ok_or_else(|| {
                malformed_review_state(format!(
                    "candidate {} effective decision {decision_id} has no state",
                    candidate.candidate_id
                ))
            })?;
            let decision_source = effective.effective_decision_source.ok_or_else(|| {
                malformed_review_state(format!(
                    "candidate {} effective decision {decision_id} has no source",
                    candidate.candidate_id
                ))
            })?;
            let details = read_effective_decision_details(connection, decision_id)?;
            Some(ReviewDecision {
                decision_id,
                decision_state,
                decision_source,
                decision_basis: details.decision_basis,
                current_status: effective.effective_decision_current_status,
                created_at: details.created_at,
                user_blocking_decision_state: effective.user_blocking_decision_state,
                masked_system_decision_id: effective.masked_system_decision_id,
            })
        }
        None => None,
    };

    let review_state = derive_review_state(candidate.candidate_id, effective_decision.as_ref())?;
    let source_summary = ReviewSourceSummary {
        source_count: candidate.source_count,
        source_samples: read_source_samples(connection, candidate.candidate_id)?,
    };

    Ok(ReviewCandidate {
        candidate_id: candidate.candidate_id,
        candidate_kind: candidate.candidate_kind,
        candidate_evidence_basis: candidate.candidate_evidence_basis,
        candidate_status: candidate.candidate_status,
        evidence_key_algorithm: candidate.evidence_key_algorithm,
        evidence_key_value: candidate.evidence_key_value,
        evidence_summary: candidate.evidence_summary,
        source_summary,
        review_state,
        effective_decision,
        created_at: candidate.created_at,
        updated_at: candidate.updated_at,
    })
}

fn derive_review_state(
    candidate_id: i64,
    effective_decision: Option<&ReviewDecision>,
) -> LibrarySqliteResult<ReviewState> {
    let Some(decision) = effective_decision else {
        return Ok(ReviewState::NeedsUserDecision);
    };

    if decision.current_status == StoreTrackIdentityEffectiveDecisionCurrentStatus::Stale {
        return Ok(ReviewState::StaleDecision);
    }

    if decision.current_status != StoreTrackIdentityEffectiveDecisionCurrentStatus::Current {
        return Err(malformed_review_state(format!(
            "candidate {candidate_id} effective decision {} has unsupported current status {:?}",
            decision.decision_id, decision.current_status
        )));
    }

    match (decision.decision_source.as_str(), decision.decision_state) {
        (
            TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
            StoreTrackIdentityDecisionState::Accepted,
        ) => Ok(ReviewState::SystemAccepted),
        (
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
            StoreTrackIdentityDecisionState::Accepted,
        ) => Ok(ReviewState::UserAccepted),
        (
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
            StoreTrackIdentityDecisionState::Rejected,
        ) => Ok(ReviewState::UserRejected),
        (
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
            StoreTrackIdentityDecisionState::Deferred,
        ) => Ok(ReviewState::UserDeferred),
        _ => Err(malformed_review_state(format!(
            "candidate {candidate_id} effective decision {} has unsupported source/state pair {:?}/{:?}",
            decision.decision_id, decision.decision_source, decision.decision_state
        ))),
    }
}

fn read_effective_decision_details(
    connection: &Connection,
    decision_id: i64,
) -> LibrarySqliteResult<EffectiveDecisionDetails> {
    connection
        .query_row(
            "SELECT decision_basis,
                    created_at
             FROM track_identity_decisions
             WHERE track_identity_decision_id = ?1",
            [decision_id],
            |row| {
                Ok(EffectiveDecisionDetails {
                    decision_basis: row.get(0)?,
                    created_at: row.get(1)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            malformed_review_state(format!(
                "effective decision {decision_id} was not found while hydrating review candidate"
            ))
        })
}

fn read_source_samples(
    connection: &Connection,
    candidate_id: i64,
) -> LibrarySqliteResult<Vec<ReviewSourceSample>> {
    connection
        .prepare(
            "SELECT evidence.source_id,
                    COALESCE(source.display_name, '')
             FROM track_identity_candidate_evidence evidence
             LEFT JOIN sources source
               ON source.source_id = evidence.source_id
             WHERE evidence.track_identity_candidate_id = ?1
             GROUP BY evidence.source_id, source.display_name
             ORDER BY evidence.source_id ASC
             LIMIT 3",
        )?
        .query_map([candidate_id], |row| {
            Ok(ReviewSourceSample {
                source_id: row.get(0)?,
                display_name: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn map_factual_candidate_row(row: &Row<'_>) -> rusqlite::Result<FactualCandidate> {
    Ok(FactualCandidate {
        candidate_id: row.get(0)?,
        candidate_kind: row.get(1)?,
        candidate_evidence_basis: row.get(2)?,
        candidate_status: map_candidate_status(row.get::<_, String>(3)?.as_str()),
        evidence_key_algorithm: row.get(4)?,
        evidence_key_value: row.get(5)?,
        evidence_summary: ReviewEvidenceSummary {
            member_count: read_usize(row, 6)?,
            evidence_count: read_usize(row, 7)?,
            current_evidence_count: read_usize(row, 9)?,
        },
        source_count: read_usize(row, 8)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn map_candidate_status(raw: &str) -> StoreTrackIdentityCandidateStatus {
    match raw {
        "active" => StoreTrackIdentityCandidateStatus::Active,
        "superseded" => StoreTrackIdentityCandidateStatus::Superseded,
        _ => StoreTrackIdentityCandidateStatus::Stale,
    }
}

fn read_usize(row: &Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let value: i64 = row.get(index)?;
    usize::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

fn malformed_review_state(detail: impl Into<String>) -> LibrarySqliteError {
    LibrarySqliteError::MalformedSchemaState(detail.into())
}
