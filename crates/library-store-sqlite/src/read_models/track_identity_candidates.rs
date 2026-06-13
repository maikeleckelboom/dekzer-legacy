use rusqlite::{Connection, Row, params};

use crate::LibrarySqliteResult;
use crate::store::SOURCE_FILE_BLAKE3_ALGORITHM;
use crate::track_identity_evidence_predicates::current_track_identity_candidate_evidence_predicate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreTrackIdentityCandidate {
    pub track_identity_candidate_id: i64,
    pub candidate_kind: String,
    pub evidence_basis: String,
    pub evidence_key_algorithm: String,
    pub evidence_key_value: String,
    pub status: StoreTrackIdentityCandidateStatus,
    pub exists_because: String,
    pub does_not_prove: String,
    pub members: Vec<StoreTrackIdentityCandidateMember>,
    pub evidence: Vec<StoreTrackIdentityCandidateEvidence>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreTrackIdentityCandidateStatus {
    Active,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreTrackIdentityCandidateMember {
    pub track_identity_candidate_member_id: i64,
    pub track_identity_candidate_id: i64,
    pub playable_media_id: i64,
    pub attachment_id: i64,
    pub evidence_source_file_id: i64,
    pub evidence_basis_fingerprint: String,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreTrackIdentityCandidateEvidence {
    pub track_identity_candidate_evidence_id: i64,
    pub track_identity_candidate_id: i64,
    pub playable_media_id: i64,
    pub attachment_id: i64,
    pub source_file_attachment_link_id: i64,
    pub source_file_id: i64,
    pub source_id: i64,
    pub evidence_basis_fingerprint: String,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    pub probe_accepted_artifact_id: i64,
    pub evidence_status: StoreTrackIdentityCandidateEvidenceStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreTrackIdentityCandidateEvidenceStatus {
    Current,
    Stale,
}

pub fn read_track_identity_candidates_for_source(
    connection: &Connection,
    source_id: i64,
    limit: usize,
) -> LibrarySqliteResult<Vec<StoreTrackIdentityCandidate>> {
    let limit_i64 =
        i64::try_from(limit).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?;
    let mut candidates = connection
        .prepare(
            "SELECT DISTINCT candidate.track_identity_candidate_id,
                    candidate.candidate_kind,
                    candidate.evidence_basis,
                    candidate.evidence_key_algorithm,
                    candidate.evidence_key_value,
                    candidate.status,
                    candidate.created_at,
                    candidate.updated_at
             FROM track_identity_candidates candidate
             JOIN track_identity_candidate_evidence evidence
               ON evidence.track_identity_candidate_id = candidate.track_identity_candidate_id
             WHERE evidence.source_id = ?1
             ORDER BY candidate.track_identity_candidate_id ASC
             LIMIT ?2",
        )?
        .query_map(rusqlite::params![source_id, limit_i64], |row| {
            Ok(StoreTrackIdentityCandidate {
                track_identity_candidate_id: row.get(0)?,
                candidate_kind: row.get(1)?,
                evidence_basis: row.get(2)?,
                evidence_key_algorithm: row.get(3)?,
                evidence_key_value: row.get(4)?,
                status: map_candidate_status(row.get::<_, String>(5)?.as_str()),
                exists_because:
                    "current playableMedia candidates share exact BLAKE3 content evidence"
                        .to_string(),
                does_not_prove:
                    "canonical track identity, user decision, CUE association, or semantic multi-encode equivalence"
                        .to_string(),
                members: Vec::new(),
                evidence: Vec::new(),
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for candidate in &mut candidates {
        candidate.members = read_track_identity_candidate_members(
            connection,
            candidate.track_identity_candidate_id,
        )?;
        candidate.evidence = read_track_identity_candidate_evidence(
            connection,
            candidate.track_identity_candidate_id,
        )?;
    }

    Ok(candidates)
}

fn read_track_identity_candidate_members(
    connection: &Connection,
    track_identity_candidate_id: i64,
) -> LibrarySqliteResult<Vec<StoreTrackIdentityCandidateMember>> {
    connection
        .prepare(
            "SELECT track_identity_candidate_member_id,
                    track_identity_candidate_id,
                    playable_media_id,
                    attachment_id,
                    evidence_source_file_id,
                    evidence_basis_fingerprint,
                    content_hash_algorithm,
                    content_hash_value,
                    created_at,
                    updated_at
             FROM track_identity_candidate_members
             WHERE track_identity_candidate_id = ?1
             ORDER BY playable_media_id ASC",
        )?
        .query_map([track_identity_candidate_id], |row| {
            Ok(StoreTrackIdentityCandidateMember {
                track_identity_candidate_member_id: row.get(0)?,
                track_identity_candidate_id: row.get(1)?,
                playable_media_id: row.get(2)?,
                attachment_id: row.get(3)?,
                evidence_source_file_id: row.get(4)?,
                evidence_basis_fingerprint: row.get(5)?,
                content_hash_algorithm: row.get(6)?,
                content_hash_value: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn read_track_identity_candidate_evidence(
    connection: &Connection,
    track_identity_candidate_id: i64,
) -> LibrarySqliteResult<Vec<StoreTrackIdentityCandidateEvidence>> {
    let current_evidence_predicate = current_track_identity_candidate_evidence_predicate("?2");
    connection
        .prepare(&format!(
            "SELECT evidence.track_identity_candidate_evidence_id,
                   evidence.track_identity_candidate_id,
                   evidence.playable_media_id,
                   evidence.attachment_id,
                   evidence.source_file_attachment_link_id,
                   evidence.source_file_id,
                   evidence.source_id,
                   evidence.evidence_basis_fingerprint,
                   evidence.content_hash_algorithm,
                   evidence.content_hash_value,
                   evidence.probe_accepted_artifact_id,
                   CASE
                       WHEN {current_evidence_predicate}
                       THEN 'current'
                       ELSE 'stale'
                   END AS evidence_status,
                   evidence.created_at,
                   evidence.updated_at
            FROM track_identity_candidate_evidence evidence
            JOIN source_files file
              ON file.source_file_id = evidence.source_file_id
            LEFT JOIN source_file_observations observations
              ON observations.source_file_id = evidence.source_file_id
            LEFT JOIN source_file_attachment_links link
              ON link.source_file_attachment_link_id = evidence.source_file_attachment_link_id
            LEFT JOIN content_attachments attachment
              ON attachment.attachment_id = evidence.attachment_id
            WHERE evidence.track_identity_candidate_id = ?1
            ORDER BY evidence.source_id ASC,
                     evidence.source_file_id ASC",
        ))?
        .query_map(
            params![track_identity_candidate_id, SOURCE_FILE_BLAKE3_ALGORITHM],
            map_evidence_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn map_candidate_status(raw: &str) -> StoreTrackIdentityCandidateStatus {
    match raw {
        "active" => StoreTrackIdentityCandidateStatus::Active,
        "superseded" => StoreTrackIdentityCandidateStatus::Superseded,
        _ => StoreTrackIdentityCandidateStatus::Stale,
    }
}

fn map_evidence_row(row: &Row<'_>) -> rusqlite::Result<StoreTrackIdentityCandidateEvidence> {
    Ok(StoreTrackIdentityCandidateEvidence {
        track_identity_candidate_evidence_id: row.get(0)?,
        track_identity_candidate_id: row.get(1)?,
        playable_media_id: row.get(2)?,
        attachment_id: row.get(3)?,
        source_file_attachment_link_id: row.get(4)?,
        source_file_id: row.get(5)?,
        source_id: row.get(6)?,
        evidence_basis_fingerprint: row.get(7)?,
        content_hash_algorithm: row.get(8)?,
        content_hash_value: row.get(9)?,
        probe_accepted_artifact_id: row.get(10)?,
        evidence_status: match row.get::<_, String>(11)?.as_str() {
            "current" => StoreTrackIdentityCandidateEvidenceStatus::Current,
            _ => StoreTrackIdentityCandidateEvidenceStatus::Stale,
        },
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}
