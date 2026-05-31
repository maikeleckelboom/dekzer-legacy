use library_boundary_protocol as protocol;
use library_store_sqlite as store;

pub(crate) fn map_hash_source_files_blake3_reply(
    result: store::HashSourceFileBlake3BatchResult,
    source_failure: Option<protocol::HashSourceFilesBlake3SourceFailure>,
) -> protocol::HashSourceFilesBlake3Reply {
    protocol::HashSourceFilesBlake3Reply {
        effective_limit: result.effective_limit,
        outcomes: result
            .outcomes
            .into_iter()
            .map(map_hash_source_files_blake3_outcome)
            .collect(),
        hashed_count: result.hashed_count,
        skipped_count: result.skipped_count,
        failed_count: result.failed_count,
        remaining_candidates: result.remaining_candidates,
        source_failure,
    }
}

pub(crate) fn empty_hash_source_files_blake3_reply(
    effective_limit: usize,
    source_failure: protocol::HashSourceFilesBlake3SourceFailure,
) -> protocol::HashSourceFilesBlake3Reply {
    protocol::HashSourceFilesBlake3Reply {
        effective_limit,
        outcomes: Vec::new(),
        hashed_count: 0,
        skipped_count: 0,
        failed_count: 0,
        remaining_candidates: 0,
        source_failure: Some(source_failure),
    }
}

pub(crate) fn map_hash_lifecycle_source_failure(
    lifecycle: Option<&store::StoreSourceLifecycle>,
) -> store::LibrarySqliteResult<Option<protocol::HashSourceFilesBlake3SourceFailure>> {
    let Some(lifecycle) = lifecycle else {
        return Ok(Some(
            protocol::HashSourceFilesBlake3SourceFailure::SourceNotFound,
        ));
    };

    match lifecycle.access_state.as_str() {
        "missing" => Ok(Some(
            protocol::HashSourceFilesBlake3SourceFailure::SourceRootMissing(
                protocol::HashSourceFilesBlake3SourceRootMissingFailure { detail: None },
            ),
        )),
        "blocked" => Ok(Some(
            protocol::HashSourceFilesBlake3SourceFailure::SourceRootBlocked(
                protocol::HashSourceFilesBlake3SourceRootBlockedFailure {
                    access_issue_kind: lifecycle
                        .access_issue_kind
                        .as_deref()
                        .map(map_source_lifecycle_issue_kind)
                        .transpose()?,
                    detail: None,
                },
            ),
        )),
        "accessible" => {
            if lifecycle.mount_status == "unmounted" {
                return Ok(Some(
                    protocol::HashSourceFilesBlake3SourceFailure::SourceUnavailable(
                        protocol::HashSourceFilesBlake3SourceUnavailableFailure {
                            mount_status: Some(map_source_mount_status(&lifecycle.mount_status)?),
                            access_state: Some(map_source_access_state(&lifecycle.access_state)?),
                            access_issue_kind: lifecycle
                                .access_issue_kind
                                .as_deref()
                                .map(map_source_lifecycle_issue_kind)
                                .transpose()?,
                        },
                    ),
                ));
            }
            Ok(None)
        }
        "unknown" => {
            if lifecycle.mount_status == "mounted" {
                return Ok(None);
            }
            Ok(Some(
                protocol::HashSourceFilesBlake3SourceFailure::SourceUnavailable(
                    protocol::HashSourceFilesBlake3SourceUnavailableFailure {
                        mount_status: Some(map_source_mount_status(&lifecycle.mount_status)?),
                        access_state: Some(map_source_access_state(&lifecycle.access_state)?),
                        access_issue_kind: lifecycle
                            .access_issue_kind
                            .as_deref()
                            .map(map_source_lifecycle_issue_kind)
                            .transpose()?,
                    },
                ),
            ))
        }
        other => Err(malformed_store_state(format!(
            "source lifecycle access_state contains unsupported value {other:?}"
        ))),
    }
}

fn map_hash_source_files_blake3_outcome(
    outcome: store::HashSourceFileBlake3BatchOutcome,
) -> protocol::HashSourceFilesBlake3Outcome {
    protocol::HashSourceFilesBlake3Outcome {
        source_file_id: outcome.source_file_id.get(),
        source_id: outcome.source_id,
        relative_path: outcome.relative_path,
        status: map_hash_source_files_blake3_status(outcome.status),
    }
}

fn map_hash_source_files_blake3_status(
    status: store::HashSourceFileBlake3BatchOutcomeStatus,
) -> protocol::HashSourceFilesBlake3OutcomeStatus {
    match status {
        store::HashSourceFileBlake3BatchOutcomeStatus::Hashed {
            content_hash_value,
            accepted_artifact_id,
            work_item_id,
        } => protocol::HashSourceFilesBlake3OutcomeStatus::Hashed(
            protocol::HashSourceFilesBlake3HashedOutcome {
                content_hash_algorithm: store::SOURCE_FILE_BLAKE3_ALGORITHM.to_string(),
                content_hash_value,
                accepted_artifact_id,
                work_item_id,
            },
        ),
        store::HashSourceFileBlake3BatchOutcomeStatus::Skipped { reason } => {
            protocol::HashSourceFilesBlake3OutcomeStatus::Skipped(
                protocol::HashSourceFilesBlake3SkippedOutcome {
                    reason: map_hash_source_files_blake3_skip_reason(reason),
                },
            )
        }
        store::HashSourceFileBlake3BatchOutcomeStatus::Failed { failure } => {
            protocol::HashSourceFilesBlake3OutcomeStatus::Failed(
                protocol::HashSourceFilesBlake3FailedOutcome {
                    failure: map_hash_source_files_blake3_file_failure(failure),
                },
            )
        }
    }
}

const fn map_hash_source_files_blake3_skip_reason(
    reason: store::SourceFileBlake3HashSkipReason,
) -> protocol::HashSourceFilesBlake3SkipReason {
    match reason {
        store::SourceFileBlake3HashSkipReason::WorkAlreadyActive => {
            protocol::HashSourceFilesBlake3SkipReason::WorkAlreadyActive
        }
    }
}

fn map_hash_source_files_blake3_file_failure(
    failure: store::SourceFileBlake3HashFailure,
) -> protocol::HashSourceFilesBlake3FileFailure {
    match failure {
        store::SourceFileBlake3HashFailure::SourceFileNotFound => {
            protocol::HashSourceFilesBlake3FileFailure::SourceFileNotFound
        }
        store::SourceFileBlake3HashFailure::SourceFileUnavailable { presence_state } => {
            protocol::HashSourceFilesBlake3FileFailure::SourceFileUnavailable(
                protocol::HashSourceFilesBlake3SourceFileUnavailableFailure { presence_state },
            )
        }
        store::SourceFileBlake3HashFailure::SourceRootUnavailable {
            mount_status,
            access_state,
            access_issue_kind,
        } => protocol::HashSourceFilesBlake3FileFailure::SourceRootUnavailable(
            protocol::HashSourceFilesBlake3SourceUnavailableFailure {
                mount_status: mount_status
                    .as_deref()
                    .and_then(source_mount_status_from_store_value),
                access_state: access_state
                    .as_deref()
                    .and_then(source_access_state_from_store_value),
                access_issue_kind: access_issue_kind
                    .as_deref()
                    .and_then(source_lifecycle_issue_kind_from_store_value),
            },
        ),
        store::SourceFileBlake3HashFailure::SourceRootMissing { detail } => {
            protocol::HashSourceFilesBlake3FileFailure::SourceRootMissing(
                protocol::HashSourceFilesBlake3SourceRootMissingFailure { detail },
            )
        }
        store::SourceFileBlake3HashFailure::SourceRootBlocked {
            access_issue_kind,
            detail,
        } => protocol::HashSourceFilesBlake3FileFailure::SourceRootBlocked(
            protocol::HashSourceFilesBlake3SourceRootBlockedFailure {
                access_issue_kind: access_issue_kind
                    .as_deref()
                    .and_then(source_lifecycle_issue_kind_from_store_value),
                detail,
            },
        ),
        store::SourceFileBlake3HashFailure::InvalidRelativePath { reason } => {
            protocol::HashSourceFilesBlake3FileFailure::InvalidRelativePath(
                protocol::HashSourceFilesBlake3InvalidRelativePathFailure { reason },
            )
        }
        store::SourceFileBlake3HashFailure::SourceFilePathEscapesRoot => {
            protocol::HashSourceFilesBlake3FileFailure::SourceFilePathEscapesRoot
        }
        store::SourceFileBlake3HashFailure::PhysicalFileMissing { .. } => {
            protocol::HashSourceFilesBlake3FileFailure::PhysicalFileMissing
        }
        store::SourceFileBlake3HashFailure::PhysicalFileBlocked { detail, .. } => {
            protocol::HashSourceFilesBlake3FileFailure::PhysicalFileBlocked(
                protocol::HashSourceFilesBlake3IoFailure { detail },
            )
        }
        store::SourceFileBlake3HashFailure::FileOpen { detail, .. } => {
            protocol::HashSourceFilesBlake3FileFailure::FileOpen(
                protocol::HashSourceFilesBlake3IoFailure { detail },
            )
        }
        store::SourceFileBlake3HashFailure::FileRead { detail, .. } => {
            protocol::HashSourceFilesBlake3FileFailure::FileRead(
                protocol::HashSourceFilesBlake3IoFailure { detail },
            )
        }
        store::SourceFileBlake3HashFailure::BasisChanged => {
            protocol::HashSourceFilesBlake3FileFailure::BasisChanged
        }
        store::SourceFileBlake3HashFailure::Store { .. } => {
            protocol::HashSourceFilesBlake3FileFailure::StoreFailure
        }
    }
}

fn map_source_mount_status(value: &str) -> store::LibrarySqliteResult<protocol::SourceMountStatus> {
    source_mount_status_from_store_value(value).ok_or_else(|| {
        malformed_store_state(format!(
            "source lifecycle mount_status contains unsupported value {value:?}"
        ))
    })
}

fn source_mount_status_from_store_value(value: &str) -> Option<protocol::SourceMountStatus> {
    match value {
        "unknown" => Some(protocol::SourceMountStatus::Unknown),
        "mounted" => Some(protocol::SourceMountStatus::Mounted),
        "unmounted" => Some(protocol::SourceMountStatus::Unmounted),
        "eject_requested" => Some(protocol::SourceMountStatus::EjectRequested),
        "eject_pending" => Some(protocol::SourceMountStatus::EjectPending),
        _ => None,
    }
}

fn map_source_access_state(value: &str) -> store::LibrarySqliteResult<protocol::SourceAccessState> {
    source_access_state_from_store_value(value).ok_or_else(|| {
        malformed_store_state(format!(
            "source lifecycle access_state contains unsupported value {value:?}"
        ))
    })
}

fn source_access_state_from_store_value(value: &str) -> Option<protocol::SourceAccessState> {
    match value {
        "accessible" => Some(protocol::SourceAccessState::Accessible),
        "missing" => Some(protocol::SourceAccessState::Missing),
        "blocked" => Some(protocol::SourceAccessState::Blocked),
        "unknown" => Some(protocol::SourceAccessState::Unknown),
        _ => None,
    }
}

fn map_source_lifecycle_issue_kind(
    value: &str,
) -> store::LibrarySqliteResult<protocol::SourceLifecycleIssueKind> {
    source_lifecycle_issue_kind_from_store_value(value).ok_or_else(|| {
        malformed_store_state(format!(
            "source lifecycle issue_kind contains unsupported value {value:?}"
        ))
    })
}

fn source_lifecycle_issue_kind_from_store_value(
    value: &str,
) -> Option<protocol::SourceLifecycleIssueKind> {
    match value {
        "missing" => Some(protocol::SourceLifecycleIssueKind::Missing),
        "not_directory" => Some(protocol::SourceLifecycleIssueKind::NotDirectory),
        "permission_denied" => Some(protocol::SourceLifecycleIssueKind::PermissionDenied),
        "privacy_permission_required" => {
            Some(protocol::SourceLifecycleIssueKind::PrivacyPermissionRequired)
        }
        "unavailable_mount" => Some(protocol::SourceLifecycleIssueKind::UnavailableMount),
        "resource_busy" => Some(protocol::SourceLifecycleIssueKind::ResourceBusy),
        "stale_network_handle" => Some(protocol::SourceLifecycleIssueKind::StaleNetworkHandle),
        "symlink_loop" => Some(protocol::SourceLifecycleIssueKind::SymlinkLoop),
        "symlink_escape_blocked" => Some(protocol::SourceLifecycleIssueKind::SymlinkEscapeBlocked),
        "unsupported_path" => Some(protocol::SourceLifecycleIssueKind::UnsupportedPath),
        "invalid_path" => Some(protocol::SourceLifecycleIssueKind::InvalidPath),
        "io_interrupted" => Some(protocol::SourceLifecycleIssueKind::IoInterrupted),
        "timed_out" => Some(protocol::SourceLifecycleIssueKind::TimedOut),
        "unknown_io" => Some(protocol::SourceLifecycleIssueKind::UnknownIo),
        _ => None,
    }
}

fn malformed_store_state(detail: String) -> store::LibrarySqliteError {
    store::LibrarySqliteError::MalformedSchemaState(detail)
}
