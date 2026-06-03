use library_boundary_protocol as protocol;
use library_store_sqlite::{
    SqliteDurableStore, StoreTrackIdentityDecisionState,
    StoreTrackIdentityEffectiveDecisionCurrentStatus,
    StoreTrackIdentityEffectiveDecisionPrecedence, StoreTrackIdentityUserBlockingDecisionState,
    TrackIdentityDecisionChangeFailure, TrackIdentityDecisionChangeResult,
};

use crate::service::{map_store_error, require_positive_i64};

const MAX_TRACK_IDENTITY_DECISION_REASON_CHARS: usize = 512;

pub(crate) fn handle_track_identity_decisions_command(
    durable_store: &SqliteDurableStore,
    command: protocol::TrackIdentityDecisionCommand,
) -> protocol::ProtocolResult<protocol::TrackIdentityDecisionReply> {
    match command {
        protocol::TrackIdentityDecisionCommand::AcceptTrackIdentityCandidate(request) => {
            accept_track_identity_candidate(durable_store, request)
                .map(protocol::TrackIdentityDecisionReply::AcceptTrackIdentityCandidate)
        }
        protocol::TrackIdentityDecisionCommand::RejectTrackIdentityCandidate(request) => {
            reject_track_identity_candidate(durable_store, request)
                .map(protocol::TrackIdentityDecisionReply::RejectTrackIdentityCandidate)
        }
        protocol::TrackIdentityDecisionCommand::DeferTrackIdentityCandidate(request) => {
            defer_track_identity_candidate(durable_store, request)
                .map(protocol::TrackIdentityDecisionReply::DeferTrackIdentityCandidate)
        }
    }
}

fn accept_track_identity_candidate(
    durable_store: &SqliteDurableStore,
    request: protocol::AcceptTrackIdentityCandidateRequest,
) -> protocol::ProtocolResult<protocol::TrackIdentityDecisionCommandResult> {
    let candidate_id = require_positive_i64(request.candidate_id, "candidateId")?;
    let reason = sanitize_track_identity_decision_reason(request.reason);
    let result = durable_store
        .accept_track_identity_candidate(candidate_id, reason)
        .map_err(map_store_error)?;
    map_track_identity_decisions_result(result)
}

fn reject_track_identity_candidate(
    durable_store: &SqliteDurableStore,
    request: protocol::RejectTrackIdentityCandidateRequest,
) -> protocol::ProtocolResult<protocol::TrackIdentityDecisionCommandResult> {
    let candidate_id = require_positive_i64(request.candidate_id, "candidateId")?;
    let reason = sanitize_track_identity_decision_reason(request.reason);
    let result = durable_store
        .reject_track_identity_candidate(candidate_id, reason)
        .map_err(map_store_error)?;
    map_track_identity_decisions_result(result)
}

fn defer_track_identity_candidate(
    durable_store: &SqliteDurableStore,
    request: protocol::DeferTrackIdentityCandidateRequest,
) -> protocol::ProtocolResult<protocol::TrackIdentityDecisionCommandResult> {
    let candidate_id = require_positive_i64(request.candidate_id, "candidateId")?;
    let reason = sanitize_track_identity_decision_reason(request.reason);
    let result = durable_store
        .defer_track_identity_candidate(candidate_id, reason)
        .map_err(map_store_error)?;
    map_track_identity_decisions_result(result)
}

fn sanitize_track_identity_decision_reason(reason: Option<String>) -> Option<String> {
    let reason = reason?;
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(
        trimmed
            .chars()
            .take(MAX_TRACK_IDENTITY_DECISION_REASON_CHARS)
            .collect(),
    )
}

fn map_track_identity_decisions_result(
    result: TrackIdentityDecisionChangeResult,
) -> protocol::ProtocolResult<protocol::TrackIdentityDecisionCommandResult> {
    match result {
        TrackIdentityDecisionChangeResult::Written(success) => {
            Ok(protocol::TrackIdentityDecisionCommandResult::Written(
                protocol::TrackIdentityDecisionCommandSuccess {
                    decision_id: success.track_identity_decision_id,
                    candidate_id: success.track_identity_candidate_id,
                    decision_state: map_track_identity_decision_state(success.decision_state)?,
                    decision_source: success.decision_source,
                    evidence_snapshot_count: success.evidence_snapshot_count,
                    decision_created: success.decision_created,
                    effective_decision: protocol::TrackIdentityEffectiveDecisionSummary {
                        effective_decision_id: success.effective_decision.effective_decision_id,
                        effective_decision_state: success
                            .effective_decision
                            .effective_decision_state
                            .map(map_track_identity_decision_state)
                            .transpose()?,
                        effective_decision_source: success
                            .effective_decision
                            .effective_decision_source,
                        effective_decision_current_status: map_effective_decision_current_status(
                            success.effective_decision.effective_decision_current_status,
                        ),
                        effective_decision_precedence: map_effective_decision_precedence(
                            success.effective_decision.effective_decision_precedence,
                        ),
                        user_blocking_decision_state: map_user_blocking_decision_state(
                            success.effective_decision.user_blocking_decision_state,
                        ),
                        masked_system_decision_id: success
                            .effective_decision
                            .masked_system_decision_id,
                    },
                },
            ))
        }
        TrackIdentityDecisionChangeResult::Failed(failure) => {
            Ok(protocol::TrackIdentityDecisionCommandResult::Failed(
                map_track_identity_decisions_failure(failure),
            ))
        }
    }
}

fn map_track_identity_decision_state(
    state: StoreTrackIdentityDecisionState,
) -> protocol::ProtocolResult<protocol::TrackIdentityDecisionState> {
    match state {
        StoreTrackIdentityDecisionState::Accepted => {
            Ok(protocol::TrackIdentityDecisionState::Accepted)
        }
        StoreTrackIdentityDecisionState::Rejected => {
            Ok(protocol::TrackIdentityDecisionState::Rejected)
        }
        StoreTrackIdentityDecisionState::Deferred => {
            Ok(protocol::TrackIdentityDecisionState::Deferred)
        }
        StoreTrackIdentityDecisionState::Superseded => {
            Err(protocol::ProtocolError::DurableStoreFailure {
                detail: "effective track identity decision unexpectedly resolved to superseded"
                    .to_string(),
            })
        }
    }
}

fn map_effective_decision_current_status(
    status: StoreTrackIdentityEffectiveDecisionCurrentStatus,
) -> protocol::TrackIdentityEffectiveDecisionCurrentStatus {
    match status {
        StoreTrackIdentityEffectiveDecisionCurrentStatus::Current => {
            protocol::TrackIdentityEffectiveDecisionCurrentStatus::Current
        }
        StoreTrackIdentityEffectiveDecisionCurrentStatus::Stale => {
            protocol::TrackIdentityEffectiveDecisionCurrentStatus::Stale
        }
        StoreTrackIdentityEffectiveDecisionCurrentStatus::NoCurrentDecision => {
            protocol::TrackIdentityEffectiveDecisionCurrentStatus::NoCurrentDecision
        }
    }
}

fn map_effective_decision_precedence(
    precedence: StoreTrackIdentityEffectiveDecisionPrecedence,
) -> protocol::TrackIdentityEffectiveDecisionPrecedence {
    match precedence {
        StoreTrackIdentityEffectiveDecisionPrecedence::User => {
            protocol::TrackIdentityEffectiveDecisionPrecedence::User
        }
        StoreTrackIdentityEffectiveDecisionPrecedence::System => {
            protocol::TrackIdentityEffectiveDecisionPrecedence::System
        }
        StoreTrackIdentityEffectiveDecisionPrecedence::None => {
            protocol::TrackIdentityEffectiveDecisionPrecedence::None
        }
    }
}

fn map_user_blocking_decision_state(
    state: StoreTrackIdentityUserBlockingDecisionState,
) -> protocol::TrackIdentityUserBlockingDecisionState {
    match state {
        StoreTrackIdentityUserBlockingDecisionState::None => {
            protocol::TrackIdentityUserBlockingDecisionState::None
        }
        StoreTrackIdentityUserBlockingDecisionState::Rejected => {
            protocol::TrackIdentityUserBlockingDecisionState::Rejected
        }
        StoreTrackIdentityUserBlockingDecisionState::Deferred => {
            protocol::TrackIdentityUserBlockingDecisionState::Deferred
        }
    }
}

fn map_track_identity_decisions_failure(
    failure: TrackIdentityDecisionChangeFailure,
) -> protocol::TrackIdentityDecisionCommandFailure {
    match failure {
        TrackIdentityDecisionChangeFailure::CandidateNotFound => {
            protocol::TrackIdentityDecisionCommandFailure::CandidateNotFound
        }
        TrackIdentityDecisionChangeFailure::CandidateStaleForAccept => {
            protocol::TrackIdentityDecisionCommandFailure::CandidateStaleForAccept
        }
        TrackIdentityDecisionChangeFailure::NoCurrentEvidenceForAccept => {
            protocol::TrackIdentityDecisionCommandFailure::NoCurrentEvidenceForAccept
        }
        TrackIdentityDecisionChangeFailure::NoSourceScopeForDecision => {
            protocol::TrackIdentityDecisionCommandFailure::NoSourceScopeForDecision
        }
    }
}
