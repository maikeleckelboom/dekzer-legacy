use library_boundary_protocol as protocol;
use library_domain::{
    NavigationSelector, NavigationSelectorDecodeError, decode_selector, encode_selector,
};
use library_store_sqlite as store;

pub(crate) fn map_maintained_read_model_revisions(
    revisions: Vec<store::MaintainedReadModelRevision>,
) -> Vec<protocol::MaintainedSnapshotScopeRevision> {
    revisions
        .into_iter()
        .map(map_maintained_read_model_revision)
        .collect()
}

pub(crate) fn map_read_navigation_rows_reply(
    rows: Vec<store::NavigationRow>,
) -> store::LibrarySqliteResult<protocol::ReadNavigationRowsReply> {
    Ok(protocol::ReadNavigationRowsReply {
        rows: rows
            .into_iter()
            .map(map_navigation_row)
            .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
    })
}

pub(crate) fn map_load_navigation_row_reply(
    row: Option<store::NavigationRow>,
) -> store::LibrarySqliteResult<protocol::LoadNavigationRowReply> {
    Ok(protocol::LoadNavigationRowReply {
        row: row.map(map_navigation_row).transpose()?,
    })
}

pub(crate) fn map_load_navigation_row_by_stable_key_reply(
    row: Option<store::NavigationRow>,
) -> store::LibrarySqliteResult<protocol::LoadNavigationRowByStableKeyReply> {
    Ok(protocol::LoadNavigationRowByStableKeyReply {
        row: row.map(map_navigation_row).transpose()?,
    })
}

pub(crate) fn store_library_tree_entry_point(
    entry_point: protocol::LibraryTreeEntryPoint,
) -> store::StoreLiteralHierarchyEntryPoint {
    match entry_point {
        protocol::LibraryTreeEntryPoint::Source { source_id } => {
            store::StoreLiteralHierarchyEntryPoint::Source { source_id }
        }
        protocol::LibraryTreeEntryPoint::SourceLocation { source_location_id } => {
            store::StoreLiteralHierarchyEntryPoint::SourceLocation { source_location_id }
        }
    }
}

pub(crate) fn store_contents_scope(scope: protocol::ContentsScope) -> store::StoreContentsScope {
    match scope {
        protocol::ContentsScope::Source { source_id } => {
            store::StoreContentsScope::Source { source_id }
        }
        protocol::ContentsScope::SourceLocation { source_location_id } => {
            store::StoreContentsScope::SourceLocation { source_location_id }
        }
        protocol::ContentsScope::Directory {
            source_id,
            source_directory_id,
        } => store::StoreContentsScope::Directory {
            source_id,
            source_directory_id,
        },
    }
}

pub(crate) fn store_contents_policy(
    policy: protocol::ContentsReadPolicy,
) -> store::StoreContentsReadPolicy {
    store::StoreContentsReadPolicy {
        media_classes: policy
            .media_classes
            .into_iter()
            .map(store_contents_media_class)
            .collect(),
        row_profile: store_contents_row_profile(policy.row_profile),
    }
}

pub(crate) const fn store_contents_recursion(
    recursion: protocol::ContentsRecursion,
) -> store::StoreContentsRecursion {
    match recursion {
        protocol::ContentsRecursion::Immediate => store::StoreContentsRecursion::Immediate,
        protocol::ContentsRecursion::Recursive => store::StoreContentsRecursion::Recursive,
    }
}

const fn store_contents_media_class(
    media_class: protocol::ContentsMediaClass,
) -> store::StoreContentsMediaClass {
    match media_class {
        protocol::ContentsMediaClass::Audio => store::StoreContentsMediaClass::Audio,
        protocol::ContentsMediaClass::Video => store::StoreContentsMediaClass::Video,
        protocol::ContentsMediaClass::Image => store::StoreContentsMediaClass::Image,
        protocol::ContentsMediaClass::Unsupported => store::StoreContentsMediaClass::Unsupported,
    }
}

const fn store_contents_row_profile(
    row_profile: protocol::ContentsRowProfile,
) -> store::StoreContentsRowProfile {
    match row_profile {
        protocol::ContentsRowProfile::SourceFile => store::StoreContentsRowProfile::SourceFile,
        protocol::ContentsRowProfile::PrimaryMedia => store::StoreContentsRowProfile::PrimaryMedia,
    }
}

pub(crate) fn map_read_library_tree_children_reply(
    window: Option<store::StoreLiteralHierarchyWindow>,
) -> store::LibrarySqliteResult<protocol::ReadLibraryTreeChildrenReply> {
    Ok(protocol::ReadLibraryTreeChildrenReply {
        window: window.map(map_library_tree_window).transpose()?,
    })
}

pub(crate) fn map_read_source_lifecycle_reply(
    lifecycle: Option<store::StoreSourceLifecycle>,
) -> store::LibrarySqliteResult<protocol::ReadSourceLifecycleReply> {
    Ok(protocol::ReadSourceLifecycleReply {
        lifecycle: lifecycle.map(map_source_lifecycle).transpose()?,
    })
}

pub(crate) fn map_read_source_file_attachment_reply(
    attachment_link: Option<store::StoreSourceFileAttachmentLink>,
) -> store::LibrarySqliteResult<protocol::ReadSourceFileAttachmentReply> {
    Ok(protocol::ReadSourceFileAttachmentReply {
        status: if attachment_link.is_some() {
            protocol::AttachmentIdentityReadStatus::Ok
        } else {
            protocol::AttachmentIdentityReadStatus::NotFound
        },
        attachment_link: attachment_link
            .map(map_source_file_attachment_link)
            .transpose()?,
    })
}

pub(crate) fn map_read_attachment_source_files_reply(
    source_files: Option<store::StoreAttachmentSourceFiles>,
) -> store::LibrarySqliteResult<protocol::ReadAttachmentSourceFilesReply> {
    match source_files {
        Some(source_files) => Ok(protocol::ReadAttachmentSourceFilesReply {
            status: protocol::AttachmentIdentityReadStatus::Ok,
            attachment: Some(map_attachment_identity(source_files.attachment)),
            source_file_links: source_files
                .source_file_links
                .into_iter()
                .map(map_source_file_attachment_link)
                .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
            effective_limit: source_files.effective_limit,
            remaining_source_file_links: source_files.remaining_source_file_links,
        }),
        None => Ok(protocol::ReadAttachmentSourceFilesReply {
            status: protocol::AttachmentIdentityReadStatus::NotFound,
            attachment: None,
            source_file_links: Vec::new(),
            effective_limit: 0,
            remaining_source_file_links: 0,
        }),
    }
}

pub(crate) fn map_read_source_attachment_summary_reply(
    summary: Option<store::StoreSourceAttachmentSummary>,
) -> protocol::ReadSourceAttachmentSummaryReply {
    protocol::ReadSourceAttachmentSummaryReply {
        status: if summary.is_some() {
            protocol::AttachmentIdentityReadStatus::Ok
        } else {
            protocol::AttachmentIdentityReadStatus::NotFound
        },
        summary: summary.map(map_source_attachment_summary),
    }
}

pub(crate) fn store_track_identity_review_state_filter(
    filter: Option<protocol::TrackIdentityReviewStateFilter>,
) -> Option<store::StoreTrackIdentityReviewState> {
    match filter {
        None | Some(protocol::TrackIdentityReviewStateFilter::All) => None,
        Some(protocol::TrackIdentityReviewStateFilter::NeedsUserDecision) => {
            Some(store::StoreTrackIdentityReviewState::NeedsUserDecision)
        }
        Some(protocol::TrackIdentityReviewStateFilter::SystemAccepted) => {
            Some(store::StoreTrackIdentityReviewState::SystemAccepted)
        }
        Some(protocol::TrackIdentityReviewStateFilter::UserAccepted) => {
            Some(store::StoreTrackIdentityReviewState::UserAccepted)
        }
        Some(protocol::TrackIdentityReviewStateFilter::UserRejected) => {
            Some(store::StoreTrackIdentityReviewState::UserRejected)
        }
        Some(protocol::TrackIdentityReviewStateFilter::UserDeferred) => {
            Some(store::StoreTrackIdentityReviewState::UserDeferred)
        }
        Some(protocol::TrackIdentityReviewStateFilter::StaleDecision) => {
            Some(store::StoreTrackIdentityReviewState::StaleDecision)
        }
    }
}

pub(crate) fn map_read_track_identity_review_candidates_reply(
    candidates: Vec<store::StoreTrackIdentityReviewCandidate>,
) -> store::LibrarySqliteResult<protocol::ReadTrackIdentityReviewCandidatesReply> {
    Ok(protocol::ReadTrackIdentityReviewCandidatesReply {
        status: protocol::TrackIdentityReviewCandidatesReadStatus::Ok,
        candidates: candidates
            .into_iter()
            .map(map_track_identity_review_candidate)
            .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
    })
}

pub(crate) fn map_read_navigation_node_library_browser_window_reply(
    window: Option<store::StoreLibraryBrowserWindow>,
) -> store::LibrarySqliteResult<protocol::ReadNavigationNodeLibraryBrowserWindowReply> {
    Ok(protocol::ReadNavigationNodeLibraryBrowserWindowReply {
        window: window.map(map_library_browser_window).transpose()?,
    })
}

pub(crate) fn map_search_navigation_node_library_browser_window_reply(
    window: Option<store::StoreLibraryBrowserWindow>,
) -> store::LibrarySqliteResult<protocol::SearchNavigationNodeLibraryBrowserWindowReply> {
    Ok(protocol::SearchNavigationNodeLibraryBrowserWindowReply {
        window: window.map(map_library_browser_window).transpose()?,
    })
}

pub(crate) fn map_read_contents_reply(
    result: store::StoreContentsResult,
) -> store::LibrarySqliteResult<protocol::ContentsReadReply> {
    Ok(protocol::ContentsReadReply {
        result: map_contents_result(result)?,
    })
}

pub(crate) fn map_read_library_asset_waveform_overview_reply(
    overview: Option<store::StoreLibraryAssetWaveformOverview>,
) -> protocol::ReadLibraryAssetWaveformOverviewReply {
    protocol::ReadLibraryAssetWaveformOverviewReply {
        overview: overview.map(map_library_asset_waveform_overview),
    }
}

pub(crate) fn map_read_library_asset_preparation_detail_reply(
    detail: Option<store::StoreLibraryAssetPreparationDetail>,
) -> store::LibrarySqliteResult<protocol::ReadLibraryAssetPreparationDetailReply> {
    Ok(protocol::ReadLibraryAssetPreparationDetailReply {
        detail: detail
            .map(map_library_asset_preparation_detail)
            .transpose()?,
    })
}

fn map_maintained_read_model_revision(
    revision: store::MaintainedReadModelRevision,
) -> protocol::MaintainedSnapshotScopeRevision {
    protocol::MaintainedSnapshotScopeRevision {
        scope: map_maintained_read_model_scope(revision.scope),
        revision: protocol::MaintainedSnapshotRevision::new(revision.revision),
    }
}

const fn map_maintained_read_model_scope(
    scope: store::MaintainedReadModelScope,
) -> protocol::MaintainedSnapshotScope {
    match scope {
        store::MaintainedReadModelScope::NavigationRows => {
            protocol::MaintainedSnapshotScope::NavigationRows
        }
        store::MaintainedReadModelScope::LibraryBrowser => {
            protocol::MaintainedSnapshotScope::LibraryBrowser
        }
    }
}

fn map_navigation_row(
    row: store::NavigationRow,
) -> store::LibrarySqliteResult<protocol::NavigationRow> {
    let navigation_row_id = row.navigation_row_id;
    let family = row
        .family
        .as_deref()
        .map(|family| map_navigation_row_family(navigation_row_id, family))
        .transpose()?;
    let row_kind = map_navigation_row_kind(navigation_row_id, &row.row_kind)?;
    let selector =
        map_navigation_selector(navigation_row_id, row.selector_kind, row.selector_payload)?;
    let selector_kind = selector.as_ref().map(map_navigation_row_selector_kind);
    let selector_payload = selector
        .as_ref()
        .map(|selector| encode_selector(selector).payload);

    Ok(protocol::NavigationRow {
        navigation_row_id,
        stable_key: row.stable_key,
        parent_navigation_row_id: row.parent_navigation_row_id,
        family,
        row_kind,
        display_name: row.display_name,
        sibling_position: row.sibling_position,
        selectable: row.selectable,
        selector_kind,
        selector_payload,
        updated_at_ms: row.updated_at,
        row_version: row.row_version,
    })
}

fn map_navigation_row_family(
    navigation_row_id: i64,
    family: &str,
) -> store::LibrarySqliteResult<protocol::NavigationRowFamily> {
    match family {
        "Views" => Ok(protocol::NavigationRowFamily::Views),
        "Collections" => Ok(protocol::NavigationRowFamily::Collections),
        "Preparation" => Ok(protocol::NavigationRowFamily::Preparation),
        "Sources" => Ok(protocol::NavigationRowFamily::Sources),
        other => Err(malformed_store_state(format!(
            "navigation_rows row {navigation_row_id} has unsupported family {other:?}"
        ))),
    }
}

fn map_navigation_row_kind(
    navigation_row_id: i64,
    row_kind: &str,
) -> store::LibrarySqliteResult<protocol::NavigationRowKind> {
    match row_kind {
        "view" => Ok(protocol::NavigationRowKind::View),
        "collection-group" => Ok(protocol::NavigationRowKind::CollectionGroup),
        "playlist" => Ok(protocol::NavigationRowKind::Playlist),
        "prep-policy-group" => Ok(protocol::NavigationRowKind::PrepPolicyGroup),
        "prep-policy-scope" => Ok(protocol::NavigationRowKind::PrepPolicyScope),
        "source" => Ok(protocol::NavigationRowKind::Source),
        "location-group" => Ok(protocol::NavigationRowKind::LocationGroup),
        "location" => Ok(protocol::NavigationRowKind::Location),
        other => Err(malformed_store_state(format!(
            "navigation_rows row {navigation_row_id} has unsupported row_kind {other:?}"
        ))),
    }
}

fn map_navigation_selector(
    navigation_row_id: i64,
    selector_kind: Option<String>,
    selector_payload: Option<String>,
) -> store::LibrarySqliteResult<Option<NavigationSelector>> {
    match (selector_kind, selector_payload) {
        (None, None) => Ok(None),
        (Some(kind), Some(payload)) => decode_selector(&kind, &payload)
            .map(Some)
            .map_err(|error| invalid_navigation_selector(navigation_row_id, error)),
        (kind, payload) => Err(malformed_store_state(format!(
            "navigation_rows row {navigation_row_id} has incomplete selector pair: selector_kind={kind:?}, selector_payload={payload:?}"
        ))),
    }
}

fn invalid_navigation_selector(
    navigation_row_id: i64,
    error: NavigationSelectorDecodeError,
) -> store::LibrarySqliteError {
    malformed_store_state(format!(
        "navigation_rows row {navigation_row_id} has invalid selector: {error}"
    ))
}

const fn map_navigation_row_selector_kind(
    selector: &NavigationSelector,
) -> protocol::NavigationRowSelectorKind {
    match selector {
        NavigationSelector::AllMedia => protocol::NavigationRowSelectorKind::AllMedia,
        NavigationSelector::AllAudio => protocol::NavigationRowSelectorKind::AllAudio,
        NavigationSelector::AllVideos => protocol::NavigationRowSelectorKind::AllVideos,
        NavigationSelector::RecentlyAdded => protocol::NavigationRowSelectorKind::RecentlyAdded,
        NavigationSelector::NeedsPreparation => {
            protocol::NavigationRowSelectorKind::NeedsPreparation
        }
        NavigationSelector::PlaylistGroup => protocol::NavigationRowSelectorKind::PlaylistGroup,
        NavigationSelector::Source(_) => protocol::NavigationRowSelectorKind::Source,
        NavigationSelector::SourceLocation(_) => {
            protocol::NavigationRowSelectorKind::SourceLocation
        }
        NavigationSelector::Playlist(_) => protocol::NavigationRowSelectorKind::Playlist,
        NavigationSelector::PrepPolicyScope(_) => {
            protocol::NavigationRowSelectorKind::PrepPolicyScope
        }
    }
}

fn map_library_tree_window(
    window: store::StoreLiteralHierarchyWindow,
) -> store::LibrarySqliteResult<protocol::LibraryTreeWindow> {
    Ok(protocol::LibraryTreeWindow {
        entry_point: map_library_tree_entry_point(window.entry_point),
        parent_source_directory_id: window.parent_source_directory_id,
        offset: window.offset,
        limit: window.limit,
        total_rows: window.total_rows,
        coverage: protocol::LibraryTreeCoverage {
            state: map_library_tree_coverage_state(window.coverage.state),
            recursive_scope_complete: window.coverage.recursive_scope_complete,
            empty_result_authoritative: window.coverage.empty_result_authoritative,
            detail: window.coverage.detail,
        },
        rows: window
            .rows
            .into_iter()
            .map(map_library_tree_node)
            .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
    })
}

fn map_source_lifecycle(
    lifecycle: store::StoreSourceLifecycle,
) -> store::LibrarySqliteResult<protocol::SourceLifecycle> {
    Ok(protocol::SourceLifecycle {
        source_id: lifecycle.source_id,
        source_class: map_source_class(&lifecycle.source_class)?,
        is_user_visible: lifecycle.is_user_visible,
        mount_status: map_source_mount_status(&lifecycle.mount_status)?,
        access_state: map_source_access_state(&lifecycle.access_state)?,
        access_issue_kind: lifecycle
            .access_issue_kind
            .as_deref()
            .map(map_source_lifecycle_issue_kind)
            .transpose()?,
        scan_phase: map_source_scan_phase(&lifecycle.scan_phase)?,
        scan_issue_kind: lifecycle
            .scan_issue_kind
            .as_deref()
            .map(map_source_lifecycle_issue_kind)
            .transpose()?,
        last_scan_started_at_ms: lifecycle.last_scan_started_at,
        last_scan_finished_at_ms: lifecycle.last_scan_finished_at,
        last_successful_scan_at_ms: lifecycle.last_successful_scan_at,
        last_seen_at_ms: lifecycle.last_seen_at,
        updated_at_ms: lifecycle.updated_at,
    })
}

fn map_attachment_identity(
    attachment: store::StoreAttachmentIdentity,
) -> protocol::AttachmentIdentity {
    protocol::AttachmentIdentity {
        attachment_id: attachment.attachment_id,
        content_hash_algorithm: attachment.content_hash_algorithm,
        content_hash_value: attachment.content_hash_value,
    }
}

fn map_source_file_attachment_link(
    link: store::StoreSourceFileAttachmentLink,
) -> store::LibrarySqliteResult<protocol::SourceFileAttachmentLink> {
    let file_kind = protocol::ContentsFileKind::from_projection_value(&link.file_kind)
        .ok_or_else(|| invalid_attachment_identity_value("file_kind", &link.file_kind))?;

    Ok(protocol::SourceFileAttachmentLink {
        attachment_id: link.attachment_id,
        source_file_id: link.source_file_id,
        source_id: link.source_id,
        content_hash_algorithm: link.content_hash_algorithm,
        content_hash_value: link.content_hash_value,
        file_kind,
        link_status: map_source_file_attachment_link_status(link.link_status),
        created_at_ms: link.created_at,
        updated_at_ms: link.updated_at,
    })
}

const fn map_source_file_attachment_link_status(
    status: store::StoreSourceFileAttachmentLinkStatus,
) -> protocol::SourceFileAttachmentLinkStatus {
    match status {
        store::StoreSourceFileAttachmentLinkStatus::Current => {
            protocol::SourceFileAttachmentLinkStatus::Current
        }
        store::StoreSourceFileAttachmentLinkStatus::Stale => {
            protocol::SourceFileAttachmentLinkStatus::Stale
        }
    }
}

fn map_source_attachment_summary(
    summary: store::StoreSourceAttachmentSummary,
) -> protocol::SourceAttachmentSummary {
    protocol::SourceAttachmentSummary {
        source_id: summary.source_id,
        current_links_count: summary.current_links_count,
        stale_links_count: summary.stale_links_count,
        source_files_with_current_blake3_facts_count: summary
            .source_files_with_current_blake3_facts_count,
        source_files_with_attachment_links_count: summary.source_files_with_attachment_links_count,
        source_files_missing_attachment_links_count: summary
            .source_files_missing_attachment_links_count,
        unmaterialized_blake3_facts_count: summary.source_files_missing_attachment_links_count,
    }
}

fn map_track_identity_review_candidate(
    candidate: store::StoreTrackIdentityReviewCandidate,
) -> store::LibrarySqliteResult<protocol::TrackIdentityReviewCandidate> {
    Ok(protocol::TrackIdentityReviewCandidate {
        candidate_id: candidate.candidate_id,
        candidate_kind: candidate.candidate_kind,
        candidate_evidence_basis: candidate.candidate_evidence_basis,
        candidate_status: map_track_identity_review_candidate_status(candidate.candidate_status),
        evidence_key_algorithm: candidate.evidence_key_algorithm,
        evidence_key_value: candidate.evidence_key_value,
        evidence_summary: protocol::TrackIdentityReviewEvidenceSummary {
            member_count: candidate.evidence_summary.member_count,
            evidence_count: candidate.evidence_summary.evidence_count,
            current_evidence_count: candidate.evidence_summary.current_evidence_count,
        },
        source_summary: protocol::TrackIdentityReviewSourceSummary {
            source_count: candidate.source_summary.source_count,
            source_samples: candidate
                .source_summary
                .source_samples
                .into_iter()
                .map(|sample| protocol::TrackIdentityReviewSourceSample {
                    source_id: sample.source_id,
                    display_name: sample.display_name,
                })
                .collect(),
        },
        review_state: map_track_identity_review_state(candidate.review_state),
        effective_decision: candidate
            .effective_decision
            .map(map_track_identity_review_decision)
            .transpose()?,
        created_at_ms: candidate.created_at,
        updated_at_ms: candidate.updated_at,
    })
}

fn map_track_identity_review_decision(
    decision: store::StoreTrackIdentityReviewDecision,
) -> store::LibrarySqliteResult<protocol::TrackIdentityReviewDecision> {
    Ok(protocol::TrackIdentityReviewDecision {
        decision_id: decision.decision_id,
        decision_state: map_track_identity_decision_state(decision.decision_state)?,
        decision_source: decision.decision_source,
        decision_basis: decision.decision_basis,
        current_status: map_track_identity_review_decision_current_status(decision.current_status)?,
        created_at_ms: decision.created_at,
        user_blocking_decision_state: map_track_identity_user_blocking_decision_state(
            decision.user_blocking_decision_state,
        ),
        masked_system_decision_id: decision.masked_system_decision_id,
    })
}

const fn map_track_identity_review_candidate_status(
    status: store::StoreTrackIdentityCandidateStatus,
) -> protocol::TrackIdentityReviewCandidateStatus {
    match status {
        store::StoreTrackIdentityCandidateStatus::Active => {
            protocol::TrackIdentityReviewCandidateStatus::Active
        }
        store::StoreTrackIdentityCandidateStatus::Stale => {
            protocol::TrackIdentityReviewCandidateStatus::Stale
        }
        store::StoreTrackIdentityCandidateStatus::Superseded => {
            protocol::TrackIdentityReviewCandidateStatus::Superseded
        }
    }
}

const fn map_track_identity_review_state(
    state: store::StoreTrackIdentityReviewState,
) -> protocol::TrackIdentityReviewState {
    match state {
        store::StoreTrackIdentityReviewState::NeedsUserDecision => {
            protocol::TrackIdentityReviewState::NeedsUserDecision
        }
        store::StoreTrackIdentityReviewState::SystemAccepted => {
            protocol::TrackIdentityReviewState::SystemAccepted
        }
        store::StoreTrackIdentityReviewState::UserAccepted => {
            protocol::TrackIdentityReviewState::UserAccepted
        }
        store::StoreTrackIdentityReviewState::UserRejected => {
            protocol::TrackIdentityReviewState::UserRejected
        }
        store::StoreTrackIdentityReviewState::UserDeferred => {
            protocol::TrackIdentityReviewState::UserDeferred
        }
        store::StoreTrackIdentityReviewState::StaleDecision => {
            protocol::TrackIdentityReviewState::StaleDecision
        }
    }
}

fn map_track_identity_decision_state(
    state: store::StoreTrackIdentityDecisionState,
) -> store::LibrarySqliteResult<protocol::TrackIdentityDecisionState> {
    match state {
        store::StoreTrackIdentityDecisionState::Accepted => {
            Ok(protocol::TrackIdentityDecisionState::Accepted)
        }
        store::StoreTrackIdentityDecisionState::Rejected => {
            Ok(protocol::TrackIdentityDecisionState::Rejected)
        }
        store::StoreTrackIdentityDecisionState::Deferred => {
            Ok(protocol::TrackIdentityDecisionState::Deferred)
        }
        store::StoreTrackIdentityDecisionState::Superseded => Err(malformed_store_state(
            "review candidate effective decision unexpectedly resolved to superseded",
        )),
    }
}

fn map_track_identity_review_decision_current_status(
    status: store::StoreTrackIdentityEffectiveDecisionCurrentStatus,
) -> store::LibrarySqliteResult<protocol::TrackIdentityEffectiveDecisionCurrentStatus> {
    match status {
        store::StoreTrackIdentityEffectiveDecisionCurrentStatus::Current => {
            Ok(protocol::TrackIdentityEffectiveDecisionCurrentStatus::Current)
        }
        store::StoreTrackIdentityEffectiveDecisionCurrentStatus::Stale => {
            Ok(protocol::TrackIdentityEffectiveDecisionCurrentStatus::Stale)
        }
        store::StoreTrackIdentityEffectiveDecisionCurrentStatus::NoCurrentDecision => {
            Err(malformed_store_state(
                "review candidate decision unexpectedly had no-current-decision status",
            ))
        }
    }
}

const fn map_track_identity_user_blocking_decision_state(
    state: store::StoreTrackIdentityUserBlockingDecisionState,
) -> protocol::TrackIdentityUserBlockingDecisionState {
    match state {
        store::StoreTrackIdentityUserBlockingDecisionState::None => {
            protocol::TrackIdentityUserBlockingDecisionState::None
        }
        store::StoreTrackIdentityUserBlockingDecisionState::Rejected => {
            protocol::TrackIdentityUserBlockingDecisionState::Rejected
        }
        store::StoreTrackIdentityUserBlockingDecisionState::Deferred => {
            protocol::TrackIdentityUserBlockingDecisionState::Deferred
        }
    }
}

fn invalid_attachment_identity_value(field_name: &str, value: &str) -> store::LibrarySqliteError {
    malformed_store_state(format!(
        "attachment identity field {field_name} contains unsupported value {value:?}"
    ))
}

fn map_source_class(value: &str) -> store::LibrarySqliteResult<protocol::SourceClass> {
    match value {
        "internal" => Ok(protocol::SourceClass::Internal),
        "external_mounted" => Ok(protocol::SourceClass::ExternalMounted),
        "removable_mounted" => Ok(protocol::SourceClass::RemovableMounted),
        other => Err(invalid_source_lifecycle_value("source_class", other)),
    }
}

fn map_source_mount_status(value: &str) -> store::LibrarySqliteResult<protocol::SourceMountStatus> {
    match value {
        "unknown" => Ok(protocol::SourceMountStatus::Unknown),
        "mounted" => Ok(protocol::SourceMountStatus::Mounted),
        "unmounted" => Ok(protocol::SourceMountStatus::Unmounted),
        "eject_requested" => Ok(protocol::SourceMountStatus::EjectRequested),
        "eject_pending" => Ok(protocol::SourceMountStatus::EjectPending),
        other => Err(invalid_source_lifecycle_value("mount_status", other)),
    }
}

fn map_source_access_state(value: &str) -> store::LibrarySqliteResult<protocol::SourceAccessState> {
    match value {
        "accessible" => Ok(protocol::SourceAccessState::Accessible),
        "missing" => Ok(protocol::SourceAccessState::Missing),
        "blocked" => Ok(protocol::SourceAccessState::Blocked),
        "unknown" => Ok(protocol::SourceAccessState::Unknown),
        other => Err(invalid_source_lifecycle_value("access_state", other)),
    }
}

fn map_source_scan_phase(value: &str) -> store::LibrarySqliteResult<protocol::SourceScanPhase> {
    match value {
        "idle" => Ok(protocol::SourceScanPhase::Idle),
        "scanning" => Ok(protocol::SourceScanPhase::Scanning),
        "complete" => Ok(protocol::SourceScanPhase::Complete),
        "partial" => Ok(protocol::SourceScanPhase::Partial),
        "blocked" => Ok(protocol::SourceScanPhase::Blocked),
        "failed" => Ok(protocol::SourceScanPhase::Failed),
        other => Err(invalid_source_lifecycle_value("scan_phase", other)),
    }
}

fn map_source_lifecycle_issue_kind(
    value: &str,
) -> store::LibrarySqliteResult<protocol::SourceLifecycleIssueKind> {
    match value {
        "missing" => Ok(protocol::SourceLifecycleIssueKind::Missing),
        "not_directory" => Ok(protocol::SourceLifecycleIssueKind::NotDirectory),
        "permission_denied" => Ok(protocol::SourceLifecycleIssueKind::PermissionDenied),
        "privacy_permission_required" => {
            Ok(protocol::SourceLifecycleIssueKind::PrivacyPermissionRequired)
        }
        "unavailable_mount" => Ok(protocol::SourceLifecycleIssueKind::UnavailableMount),
        "resource_busy" => Ok(protocol::SourceLifecycleIssueKind::ResourceBusy),
        "stale_network_handle" => Ok(protocol::SourceLifecycleIssueKind::StaleNetworkHandle),
        "symlink_loop" => Ok(protocol::SourceLifecycleIssueKind::SymlinkLoop),
        "symlink_escape_blocked" => Ok(protocol::SourceLifecycleIssueKind::SymlinkEscapeBlocked),
        "unsupported_path" => Ok(protocol::SourceLifecycleIssueKind::UnsupportedPath),
        "invalid_path" => Ok(protocol::SourceLifecycleIssueKind::InvalidPath),
        "io_interrupted" => Ok(protocol::SourceLifecycleIssueKind::IoInterrupted),
        "timed_out" => Ok(protocol::SourceLifecycleIssueKind::TimedOut),
        "unknown_io" => Ok(protocol::SourceLifecycleIssueKind::UnknownIo),
        other => Err(invalid_source_lifecycle_value("issue_kind", other)),
    }
}

fn invalid_source_lifecycle_value(field_name: &str, value: &str) -> store::LibrarySqliteError {
    malformed_store_state(format!(
        "source lifecycle field {field_name} contains unsupported value {value:?}"
    ))
}

const fn map_library_tree_coverage_state(
    state: store::StoreLiteralHierarchyCoverageState,
) -> protocol::LibraryTreeCoverageState {
    match state {
        store::StoreLiteralHierarchyCoverageState::Complete => {
            protocol::LibraryTreeCoverageState::Complete
        }
        store::StoreLiteralHierarchyCoverageState::Pending => {
            protocol::LibraryTreeCoverageState::Pending
        }
        store::StoreLiteralHierarchyCoverageState::Scanning => {
            protocol::LibraryTreeCoverageState::Scanning
        }
        store::StoreLiteralHierarchyCoverageState::Blocked => {
            protocol::LibraryTreeCoverageState::Blocked
        }
        store::StoreLiteralHierarchyCoverageState::Failed => {
            protocol::LibraryTreeCoverageState::Failed
        }
        store::StoreLiteralHierarchyCoverageState::SourceUnavailable => {
            protocol::LibraryTreeCoverageState::SourceUnavailable
        }
        store::StoreLiteralHierarchyCoverageState::LocationMissing => {
            protocol::LibraryTreeCoverageState::LocationMissing
        }
    }
}

const fn map_library_tree_entry_point(
    entry_point: store::StoreLiteralHierarchyEntryPoint,
) -> protocol::LibraryTreeEntryPoint {
    match entry_point {
        store::StoreLiteralHierarchyEntryPoint::Source { source_id } => {
            protocol::LibraryTreeEntryPoint::Source { source_id }
        }
        store::StoreLiteralHierarchyEntryPoint::SourceLocation { source_location_id } => {
            protocol::LibraryTreeEntryPoint::SourceLocation { source_location_id }
        }
    }
}

fn map_library_tree_node(
    node: store::StoreLiteralHierarchyNode,
) -> store::LibrarySqliteResult<protocol::LibraryTreeNode> {
    let has_child_directories = map_directory_only_field(
        &node.node_kind,
        node.has_child_directories,
        "has_child_directories",
    )?;
    let directory_primary_media_state = map_directory_primary_media_state(&node)?;
    let directory_image_media_state = map_directory_image_media_state(&node)?;
    let directory_scan_state = map_directory_scan_state_for_node(&node)?;
    let media_class = map_library_tree_file_media_class(&node)?;
    let child_row_state = match node.node_kind.as_str() {
        "directory" => {
            match (
                directory_primary_media_state.as_ref(),
                directory_scan_state.as_ref(),
            ) {
                (Some(protocol::DirectoryPrimaryMediaState::HasPrimaryMediaDescendants), _) => {
                    Some(protocol::ChildRowState::HasChildRows)
                }
                (
                    Some(protocol::DirectoryPrimaryMediaState::NoPrimaryMediaDescendants),
                    Some(protocol::DirectoryScanState::Complete),
                ) => Some(protocol::ChildRowState::NoChildRows),
                _ => Some(protocol::ChildRowState::Unknown),
            }
        }
        "file" => None,
        _ => None,
    };

    Ok(protocol::LibraryTreeNode {
        node_kind: map_library_tree_node_kind(&node.node_kind)?,
        source_id: node.source_id,
        source_directory_id: node.source_directory_id,
        source_file_id: node.source_file_id,
        parent_source_directory_id: node.parent_source_directory_id,
        relative_path: node.relative_path,
        display_name: node.display_name,
        media_class,
        presence_state: map_library_tree_presence_state(&node.presence_state)?,
        size_bytes: node.size_bytes,
        modified_at_ns: node.modified_at_ns,
        updated_at_ms: node.updated_at,
        has_child_directories,
        directory_primary_media_state,
        directory_image_media_state,
        directory_scan_state,
        child_row_state,
    })
}

fn map_library_tree_node_kind(
    value: &str,
) -> store::LibrarySqliteResult<protocol::LibraryTreeNodeKind> {
    match value {
        "directory" => Ok(protocol::LibraryTreeNodeKind::Directory),
        "file" => Ok(protocol::LibraryTreeNodeKind::File),
        other => Err(malformed_store_state(format!(
            "literal hierarchy node has unsupported kind {other:?}"
        ))),
    }
}

fn map_library_tree_file_media_class(
    node: &store::StoreLiteralHierarchyNode,
) -> store::LibrarySqliteResult<Option<protocol::LibraryTreeFileMediaClass>> {
    match (node.node_kind.as_str(), node.media_class.as_deref()) {
        ("file", Some(value)) => Ok(protocol::LibraryTreeFileMediaClass::from_projection_value(
            value,
        )),
        ("file", None) => Ok(None),
        ("directory", None) => Ok(None),
        ("directory", Some(_)) => Ok(None),
        _ => Ok(None),
    }
}

fn map_library_tree_presence_state(
    value: &str,
) -> store::LibrarySqliteResult<protocol::LibraryTreePresenceState> {
    match value {
        "present" => Ok(protocol::LibraryTreePresenceState::Present),
        "missing" => Ok(protocol::LibraryTreePresenceState::Missing),
        "removed" => Ok(protocol::LibraryTreePresenceState::Removed),
        other => Err(malformed_store_state(format!(
            "literal hierarchy node has unsupported presence_state {other:?}"
        ))),
    }
}

fn map_directory_only_field<T>(
    node_kind: &str,
    value: Option<T>,
    _field_name: &str,
) -> store::LibrarySqliteResult<Option<T>> {
    match (node_kind, value) {
        ("directory", Some(value)) => Ok(Some(value)),
        ("directory", None) => Ok(None),
        ("file", _) => Ok(None),
        _ => Ok(None),
    }
}

fn map_directory_primary_media_state(
    node: &store::StoreLiteralHierarchyNode,
) -> store::LibrarySqliteResult<Option<protocol::DirectoryPrimaryMediaState>> {
    if node.node_kind != "directory" {
        return Ok(None);
    }

    let has_primary_media_descendant = node.has_primary_media_descendant.unwrap_or(false);
    let dir_scan_state = node.dir_scan_state.as_deref().unwrap_or("pending");
    let directory_scan_state = map_directory_scan_state(dir_scan_state)?;

    Ok(Some(if has_primary_media_descendant {
        protocol::DirectoryPrimaryMediaState::HasPrimaryMediaDescendants
    } else if directory_scan_state == protocol::DirectoryScanState::Complete {
        protocol::DirectoryPrimaryMediaState::NoPrimaryMediaDescendants
    } else {
        protocol::DirectoryPrimaryMediaState::Unknown
    }))
}

fn map_directory_image_media_state(
    node: &store::StoreLiteralHierarchyNode,
) -> store::LibrarySqliteResult<Option<protocol::DirectoryImageMediaState>> {
    if node.node_kind != "directory" {
        return Ok(None);
    }

    let has_image_media_descendant = node.has_image_media_descendant.unwrap_or(false);
    let dir_scan_state = node.dir_scan_state.as_deref().unwrap_or("pending");
    let directory_scan_state = map_directory_scan_state(dir_scan_state)?;

    Ok(Some(if has_image_media_descendant {
        protocol::DirectoryImageMediaState::HasImageMediaDescendants
    } else if directory_scan_state == protocol::DirectoryScanState::Complete {
        protocol::DirectoryImageMediaState::NoImageMediaDescendants
    } else {
        protocol::DirectoryImageMediaState::Unknown
    }))
}

fn map_directory_scan_state_for_node(
    node: &store::StoreLiteralHierarchyNode,
) -> store::LibrarySqliteResult<Option<protocol::DirectoryScanState>> {
    if node.node_kind != "directory" {
        return Ok(None);
    }

    let dir_scan_state = node.dir_scan_state.as_deref().unwrap_or("pending");
    Ok(Some(map_directory_scan_state(dir_scan_state)?))
}

fn map_directory_scan_state(
    value: &str,
) -> store::LibrarySqliteResult<protocol::DirectoryScanState> {
    match value {
        "pending" => Ok(protocol::DirectoryScanState::Pending),
        "scanning" => Ok(protocol::DirectoryScanState::Scanning),
        "complete" => Ok(protocol::DirectoryScanState::Complete),
        "failed" => Ok(protocol::DirectoryScanState::Failed),
        "blocked" => Ok(protocol::DirectoryScanState::Blocked),
        other => Err(malformed_store_state(format!(
            "literal hierarchy node has unsupported dir_scan_state {other:?}"
        ))),
    }
}

fn map_library_browser_window(
    window: store::StoreLibraryBrowserWindow,
) -> store::LibrarySqliteResult<protocol::LibraryBrowserWindow> {
    Ok(protocol::LibraryBrowserWindow {
        offset: window.offset,
        limit: window.limit,
        total_rows: window.total_rows,
        rows: window
            .rows
            .into_iter()
            .map(map_library_asset_browser_row)
            .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
    })
}

fn map_library_asset_browser_row(
    row: store::ScopedLibraryAssetBrowserRow,
) -> store::LibrarySqliteResult<protocol::LibraryAssetBrowserRow> {
    let availability_state =
        protocol::LibraryAssetAvailabilityState::from_projection_value(&row.availability_state)
            .ok_or_else(|| {
                invalid_library_browser_projection_value(
                    "availability_state",
                    &row.availability_state,
                )
            })?;
    let stems_state_summary = row
        .stems_state_summary
        .as_deref()
        .map(|value| {
            protocol::LibraryAssetStemsStateSummary::from_projection_value(value).ok_or_else(|| {
                invalid_library_browser_projection_value("stems_state_summary", value)
            })
        })
        .transpose()?;
    let prep_readiness_summary = protocol::LibraryAssetPrepReadinessSummary::from_projection_value(
        &row.prep_readiness_summary,
    )
    .ok_or_else(|| {
        invalid_library_browser_projection_value(
            "prep_readiness_summary",
            &row.prep_readiness_summary,
        )
    })?;

    Ok(protocol::LibraryAssetBrowserRow {
        library_asset_id: row.library_asset_id,
        row_version: row.row_version,
        primary_source_file_id: row.primary_source_file_id,
        scoped_source_file_id: row.scoped_source_file_id,
        source_id: row.source_id,
        relative_path: row.relative_path,
        file_name: row.file_name,
        availability_state,
        title: row.title,
        artist: row.artist,
        album: row.album,
        duration_ms: row.duration_ms,
        musical_key: row.musical_key,
        tempo_bpm: row.tempo_bpm,
        waveform_quality_current: row.waveform_quality_current,
        waveform_quality_target: row.waveform_quality_target,
        stems_state_summary,
        prep_readiness_summary,
        updated_at_ms: row.updated_at,
    })
}

fn map_contents_result(
    result: store::StoreContentsResult,
) -> store::LibrarySqliteResult<protocol::ContentsResult> {
    Ok(protocol::ContentsResult {
        state: map_contents_state(result.state),
        scope: map_contents_scope(result.scope),
        policy: map_contents_policy(result.policy),
        recursion: map_contents_recursion(result.recursion),
        rows: result
            .rows
            .into_iter()
            .map(map_contents_row)
            .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
        coverage: protocol::ContentsCoverage {
            state: map_contents_coverage_state(result.coverage.state),
            recursive_scope_complete: result.coverage.recursive_scope_complete,
            empty_result_authoritative: result.coverage.empty_result_authoritative,
            detail: result.coverage.detail,
        },
        next_cursor: result.next_cursor,
        detail: result.detail,
    })
}

fn map_contents_policy(policy: store::StoreContentsReadPolicy) -> protocol::ContentsReadPolicy {
    protocol::ContentsReadPolicy {
        media_classes: policy
            .media_classes
            .into_iter()
            .map(map_contents_media_class)
            .collect(),
        row_profile: map_contents_row_profile(policy.row_profile),
    }
}

const fn map_contents_recursion(
    recursion: store::StoreContentsRecursion,
) -> protocol::ContentsRecursion {
    match recursion {
        store::StoreContentsRecursion::Immediate => protocol::ContentsRecursion::Immediate,
        store::StoreContentsRecursion::Recursive => protocol::ContentsRecursion::Recursive,
    }
}

const fn map_contents_row_profile(
    row_profile: store::StoreContentsRowProfile,
) -> protocol::ContentsRowProfile {
    match row_profile {
        store::StoreContentsRowProfile::SourceFile => protocol::ContentsRowProfile::SourceFile,
        store::StoreContentsRowProfile::PrimaryMedia => protocol::ContentsRowProfile::PrimaryMedia,
    }
}

const fn map_contents_media_class(
    media_class: store::StoreContentsMediaClass,
) -> protocol::ContentsMediaClass {
    match media_class {
        store::StoreContentsMediaClass::Audio => protocol::ContentsMediaClass::Audio,
        store::StoreContentsMediaClass::Video => protocol::ContentsMediaClass::Video,
        store::StoreContentsMediaClass::Image => protocol::ContentsMediaClass::Image,
        store::StoreContentsMediaClass::Unsupported => protocol::ContentsMediaClass::Unsupported,
    }
}

const fn map_contents_scope(scope: store::StoreContentsScope) -> protocol::ContentsScope {
    match scope {
        store::StoreContentsScope::Source { source_id } => {
            protocol::ContentsScope::Source { source_id }
        }
        store::StoreContentsScope::SourceLocation { source_location_id } => {
            protocol::ContentsScope::SourceLocation { source_location_id }
        }
        store::StoreContentsScope::Directory {
            source_id,
            source_directory_id,
        } => protocol::ContentsScope::Directory {
            source_id,
            source_directory_id,
        },
    }
}

const fn map_contents_state(state: store::StoreContentsState) -> protocol::ContentsState {
    match state {
        store::StoreContentsState::Ready => protocol::ContentsState::Ready,
        store::StoreContentsState::Empty => protocol::ContentsState::Empty,
        store::StoreContentsState::Partial => protocol::ContentsState::Partial,
        store::StoreContentsState::SourceUnavailable => protocol::ContentsState::SourceUnavailable,
        store::StoreContentsState::LocationMissing => protocol::ContentsState::LocationMissing,
        store::StoreContentsState::Blocked => protocol::ContentsState::Blocked,
        store::StoreContentsState::Failed => protocol::ContentsState::Failed,
        store::StoreContentsState::PolicyConflict => protocol::ContentsState::PolicyConflict,
        store::StoreContentsState::CursorInvalid => protocol::ContentsState::CursorInvalid,
    }
}

const fn map_contents_coverage_state(
    state: store::StoreContentsCoverageState,
) -> protocol::ContentsCoverageState {
    match state {
        store::StoreContentsCoverageState::Complete => protocol::ContentsCoverageState::Complete,
        store::StoreContentsCoverageState::Pending => protocol::ContentsCoverageState::Pending,
        store::StoreContentsCoverageState::Scanning => protocol::ContentsCoverageState::Scanning,
        store::StoreContentsCoverageState::Blocked => protocol::ContentsCoverageState::Blocked,
        store::StoreContentsCoverageState::Failed => protocol::ContentsCoverageState::Failed,
        store::StoreContentsCoverageState::SourceUnavailable => {
            protocol::ContentsCoverageState::SourceUnavailable
        }
        store::StoreContentsCoverageState::LocationMissing => {
            protocol::ContentsCoverageState::LocationMissing
        }
        store::StoreContentsCoverageState::Incomplete => {
            protocol::ContentsCoverageState::Incomplete
        }
    }
}

fn map_contents_row(
    row: store::StoreContentsFileRow,
) -> store::LibrarySqliteResult<protocol::ContentsFileRow> {
    let media_class = protocol::ContentsMediaClass::from_projection_value(&row.media_class)
        .ok_or_else(|| invalid_contents_value("media_class", &row.media_class))?;
    let file_kind = protocol::ContentsFileKind::from_projection_value(&row.file_kind)
        .ok_or_else(|| invalid_contents_value("file_kind", &row.file_kind))?;
    let presence = protocol::ContentsPresenceState::from_projection_value(&row.presence)
        .ok_or_else(|| invalid_contents_value("presence", &row.presence))?;
    let availability_state = row
        .availability_state
        .as_deref()
        .map(|value| {
            protocol::LibraryAssetAvailabilityState::from_projection_value(value)
                .ok_or_else(|| invalid_contents_value("availability_state", value))
        })
        .transpose()?;
    let primary_media = row
        .primary_media
        .map(map_primary_media_summary)
        .transpose()?;

    Ok(protocol::ContentsFileRow {
        id: row.id,
        source_id: row.source_id,
        source_file_id: row.source_file_id,
        parent_directory_id: row.parent_directory_id,
        label: row.label,
        relative_path: Some(row.relative_path),
        file_name: row.file_name,
        media_class,
        file_kind,
        presence,
        availability_state,
        primary_media,
        updated_at_ms: Some(row.updated_at),
    })
}

fn map_primary_media_summary(
    summary: store::StorePrimaryMediaSummary,
) -> store::LibrarySqliteResult<protocol::PrimaryMediaSummary> {
    let stems_state_summary = summary
        .stems_state_summary
        .as_deref()
        .map(|value| {
            protocol::LibraryAssetStemsStateSummary::from_projection_value(value)
                .ok_or_else(|| invalid_contents_value("stems_state_summary", value))
        })
        .transpose()?;
    let prep_readiness_summary = protocol::LibraryAssetPrepReadinessSummary::from_projection_value(
        &summary.prep_readiness_summary,
    )
    .ok_or_else(|| {
        invalid_contents_value("prep_readiness_summary", &summary.prep_readiness_summary)
    })?;

    Ok(protocol::PrimaryMediaSummary {
        origin: map_contents_row_origin(&summary.origin),
        primary_media_candidate_id: summary.primary_media_candidate_id,
        attachment_id: summary.attachment_id,
        content_hash_algorithm: summary.content_hash_algorithm,
        content_hash_value: summary.content_hash_value,
        evidence_source_file_id: summary.evidence_source_file_id,
        media_kind: summary.media_kind,
        mime_type: summary.mime_type,
        library_asset_id: summary.library_asset_id,
        row_version: summary.row_version,
        primary_source_file_id: summary.primary_source_file_id,
        title: summary.title,
        artist: summary.artist,
        album: summary.album,
        duration_ms: summary.duration_ms,
        sample_rate_hz: summary.sample_rate_hz,
        channels: summary.channels,
        bit_depth: summary.bit_depth,
        codec: summary.codec,
        musical_key: summary.musical_key,
        tempo_bpm: summary.tempo_bpm,
        waveform_quality_current: summary.waveform_quality_current,
        waveform_quality_target: summary.waveform_quality_target,
        stems_state_summary,
        prep_readiness_summary: Some(prep_readiness_summary),
    })
}

const fn map_contents_row_origin(
    origin: &store::StoreContentsRowOrigin,
) -> protocol::ContentsRowOrigin {
    match origin {
        store::StoreContentsRowOrigin::LibraryAsset => protocol::ContentsRowOrigin::LibraryAsset,
        store::StoreContentsRowOrigin::SourceFile => protocol::ContentsRowOrigin::SourceFile,
        store::StoreContentsRowOrigin::PrimaryMediaCandidate => {
            protocol::ContentsRowOrigin::PrimaryMediaCandidate
        }
    }
}

fn invalid_library_browser_projection_value(
    field_name: &str,
    value: &str,
) -> store::LibrarySqliteError {
    malformed_store_state(format!(
        "library browser projection field {field_name} contains unsupported value {value:?}"
    ))
}

fn invalid_contents_value(field_name: &str, value: &str) -> store::LibrarySqliteError {
    malformed_store_state(format!(
        "contents field {field_name} contains unsupported value {value:?}"
    ))
}

fn map_library_asset_waveform_overview(
    overview: store::StoreLibraryAssetWaveformOverview,
) -> protocol::LibraryAssetWaveformOverview {
    protocol::LibraryAssetWaveformOverview {
        source_profile_key: overview.source_profile_key,
        source_quality_current: overview.source_quality_current,
        capability_state: map_library_asset_waveform_capability_state(overview.capability_state),
        amplitude_scale: map_library_asset_waveform_amplitude_scale(overview.amplitude_scale),
        bucket_count: overview.bucket_count,
        duration_ms: overview.duration_ms,
        source_sample_count: overview.source_sample_count,
        samples_per_bucket: overview.samples_per_bucket,
        buckets: overview
            .buckets
            .into_iter()
            .map(|bucket| protocol::LibraryAssetWaveformOverviewBucket {
                min_amplitude_i16: bucket.min_amplitude_i16,
                max_amplitude_i16: bucket.max_amplitude_i16,
            })
            .collect(),
        accepted_artifact_id: overview.accepted_artifact_id,
        basis_fingerprint: overview.basis_fingerprint,
        capability_updated_at_ms: overview.capability_updated_at,
        artifact_created_at_ms: overview.artifact_created_at,
    }
}

const fn map_library_asset_waveform_capability_state(
    state: store::StoreLibraryAssetWaveformOverviewCapabilityState,
) -> protocol::LibraryAssetWaveformOverviewCapabilityState {
    match state {
        store::StoreLibraryAssetWaveformOverviewCapabilityState::Ready => {
            protocol::LibraryAssetWaveformOverviewCapabilityState::Ready
        }
        store::StoreLibraryAssetWaveformOverviewCapabilityState::Stale => {
            protocol::LibraryAssetWaveformOverviewCapabilityState::Stale
        }
    }
}

const fn map_library_asset_waveform_amplitude_scale(
    amplitude_scale: store::StoreLibraryAssetWaveformOverviewAmplitudeScale,
) -> protocol::LibraryAssetWaveformOverviewAmplitudeScale {
    match amplitude_scale {
        store::StoreLibraryAssetWaveformOverviewAmplitudeScale::SignedI16 => {
            protocol::LibraryAssetWaveformOverviewAmplitudeScale::SignedI16
        }
    }
}

fn map_library_asset_preparation_detail(
    detail: store::StoreLibraryAssetPreparationDetail,
) -> store::LibrarySqliteResult<protocol::LibraryAssetPreparationDetail> {
    let aggregate_readiness_summary =
        protocol::LibraryAssetPrepReadinessSummary::from_projection_value(
            &detail.aggregate_readiness_summary,
        )
        .ok_or_else(|| {
            invalid_preparation_detail_value(
                "aggregate_readiness_summary",
                &detail.aggregate_readiness_summary,
            )
        })?;

    Ok(protocol::LibraryAssetPreparationDetail {
        library_asset_id: detail.library_asset_id,
        aggregate_readiness_summary,
        groups: detail
            .groups
            .into_iter()
            .map(map_preparation_detail_group)
            .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
    })
}

fn map_preparation_detail_group(
    group: store::StoreLibraryAssetPreparationDetailGroup,
) -> store::LibrarySqliteResult<protocol::LibraryAssetPreparationDetailGroup> {
    let group_key = map_preparation_detail_group_key(&group.group_key)
        .ok_or_else(|| invalid_preparation_detail_value("group_key", &group.group_key))?;

    Ok(protocol::LibraryAssetPreparationDetailGroup {
        group_key,
        rows: group
            .rows
            .into_iter()
            .map(map_preparation_detail_row)
            .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
    })
}

fn map_preparation_detail_row(
    row: store::StoreLibraryAssetPreparationDetailRow,
) -> store::LibrarySqliteResult<protocol::LibraryAssetPreparationDetailRow> {
    let capability_key = map_preparation_capability_key(&row.capability_kind)
        .ok_or_else(|| invalid_preparation_detail_value("capability_kind", &row.capability_kind))?;
    let requirement_class =
        map_preparation_requirement_class(&row.requirement_class).ok_or_else(|| {
            invalid_preparation_detail_value("requirement_class", &row.requirement_class)
        })?;
    let outcome_kind = map_preparation_outcome_kind(&row.outcome_kind)
        .ok_or_else(|| invalid_preparation_detail_value("outcome_kind", &row.outcome_kind))?;
    let work_state = map_preparation_work_state(&row.work_state)
        .ok_or_else(|| invalid_preparation_detail_value("work_state", &row.work_state))?;
    let outcome_state = map_preparation_outcome_state(&row.outcome_state)
        .ok_or_else(|| invalid_preparation_detail_value("outcome_state", &row.outcome_state))?;
    let satisfaction_state = map_preparation_satisfaction_state(&row.satisfaction_state)
        .ok_or_else(|| {
            invalid_preparation_detail_value("satisfaction_state", &row.satisfaction_state)
        })?;
    let artifact_coverage_state = row
        .artifact_coverage_state
        .as_deref()
        .map(|value| {
            map_preparation_artifact_coverage_state(value)
                .ok_or_else(|| invalid_preparation_detail_value("artifact_coverage_state", value))
        })
        .transpose()?;

    Ok(protocol::LibraryAssetPreparationDetailRow {
        capability_key,
        label: row.label,
        requirement_class,
        outcome_kind,
        work_state,
        outcome_state,
        satisfaction_state,
        display_value_summary: row.display_value_summary,
        target_summary: row.target_summary,
        explanation_summary: row.explanation_summary,
        artifact_coverage_state,
        progress: row.progress.map(map_preparation_progress),
    })
}

const fn map_preparation_detail_group_key(
    value: &str,
) -> Option<protocol::LibraryAssetPreparationDetailGroupKey> {
    match value.as_bytes() {
        b"required_facts" => Some(protocol::LibraryAssetPreparationDetailGroupKey::RequiredFacts),
        b"required_structures" => {
            Some(protocol::LibraryAssetPreparationDetailGroupKey::RequiredStructures)
        }
        b"required_artifacts" => {
            Some(protocol::LibraryAssetPreparationDetailGroupKey::RequiredArtifacts)
        }
        b"on_demand_capabilities" => {
            Some(protocol::LibraryAssetPreparationDetailGroupKey::OnDemandCapabilities)
        }
        _ => None,
    }
}

const fn map_preparation_capability_key(
    value: &str,
) -> Option<protocol::LibraryAssetPreparationCapabilityKey> {
    match value.as_bytes() {
        b"tempo" => Some(protocol::LibraryAssetPreparationCapabilityKey::Bpm),
        b"musical_key" => Some(protocol::LibraryAssetPreparationCapabilityKey::MusicalKey),
        b"beatgrid" => Some(protocol::LibraryAssetPreparationCapabilityKey::Beatgrid),
        b"waveform" => Some(protocol::LibraryAssetPreparationCapabilityKey::Waveform),
        b"stems" => Some(protocol::LibraryAssetPreparationCapabilityKey::Stems),
        _ => None,
    }
}

const fn map_preparation_requirement_class(
    value: &str,
) -> Option<protocol::LibraryAssetPreparationRequirementClass> {
    match value.as_bytes() {
        b"required" => Some(protocol::LibraryAssetPreparationRequirementClass::Required),
        b"on_demand" => Some(protocol::LibraryAssetPreparationRequirementClass::OnDemand),
        _ => None,
    }
}

const fn map_preparation_outcome_kind(
    value: &str,
) -> Option<protocol::LibraryAssetPreparationOutcomeKind> {
    match value.as_bytes() {
        b"fact" => Some(protocol::LibraryAssetPreparationOutcomeKind::Fact),
        b"structure" => Some(protocol::LibraryAssetPreparationOutcomeKind::Structure),
        b"artifact" => Some(protocol::LibraryAssetPreparationOutcomeKind::Artifact),
        _ => None,
    }
}

const fn map_preparation_work_state(
    value: &str,
) -> Option<protocol::LibraryAssetPreparationWorkState> {
    match value.as_bytes() {
        b"none" => Some(protocol::LibraryAssetPreparationWorkState::None),
        b"not_requested" => Some(protocol::LibraryAssetPreparationWorkState::NotRequested),
        b"queued" => Some(protocol::LibraryAssetPreparationWorkState::Queued),
        b"active" => Some(protocol::LibraryAssetPreparationWorkState::Active),
        b"blocked" => Some(protocol::LibraryAssetPreparationWorkState::Blocked),
        b"failed" => Some(protocol::LibraryAssetPreparationWorkState::Failed),
        _ => None,
    }
}

const fn map_preparation_outcome_state(
    value: &str,
) -> Option<protocol::LibraryAssetPreparationOutcomeState> {
    match value.as_bytes() {
        b"missing" => Some(protocol::LibraryAssetPreparationOutcomeState::Missing),
        b"provisional" => Some(protocol::LibraryAssetPreparationOutcomeState::Provisional),
        b"ready" => Some(protocol::LibraryAssetPreparationOutcomeState::Ready),
        b"stale" => Some(protocol::LibraryAssetPreparationOutcomeState::Stale),
        _ => None,
    }
}

const fn map_preparation_satisfaction_state(
    value: &str,
) -> Option<protocol::LibraryAssetPreparationSatisfactionState> {
    match value.as_bytes() {
        b"satisfied" => Some(protocol::LibraryAssetPreparationSatisfactionState::Satisfied),
        b"unsatisfied" => Some(protocol::LibraryAssetPreparationSatisfactionState::Unsatisfied),
        _ => None,
    }
}

const fn map_preparation_artifact_coverage_state(
    value: &str,
) -> Option<protocol::LibraryAssetPreparationArtifactCoverageState> {
    match value.as_bytes() {
        b"none" => Some(protocol::LibraryAssetPreparationArtifactCoverageState::None),
        b"preview_ready" => {
            Some(protocol::LibraryAssetPreparationArtifactCoverageState::PreviewReady)
        }
        b"overview_ready" => {
            Some(protocol::LibraryAssetPreparationArtifactCoverageState::OverviewReady)
        }
        b"refinement_available" => {
            Some(protocol::LibraryAssetPreparationArtifactCoverageState::RefinementAvailable)
        }
        b"stale" => Some(protocol::LibraryAssetPreparationArtifactCoverageState::Stale),
        _ => None,
    }
}

fn map_preparation_progress(
    progress: store::StoreLibraryAssetPreparationProgress,
) -> protocol::LibraryAssetPreparationProgress {
    match progress {
        store::StoreLibraryAssetPreparationProgress::Indeterminate { active_stage } => {
            protocol::LibraryAssetPreparationProgress::Indeterminate { active_stage }
        }
        store::StoreLibraryAssetPreparationProgress::BoundedStage {
            active_stage,
            completed_units,
            total_units,
        } => protocol::LibraryAssetPreparationProgress::BoundedStage {
            active_stage,
            completed_units,
            total_units,
            fraction: if total_units > 0 {
                completed_units as f64 / total_units as f64
            } else {
                0.0
            },
        },
    }
}

fn invalid_preparation_detail_value(field_name: &str, value: &str) -> store::LibrarySqliteError {
    malformed_store_state(format!(
        "library asset preparation detail field {field_name} contains unsupported value {value:?}"
    ))
}

fn malformed_store_state(detail: impl Into<String>) -> store::LibrarySqliteError {
    store::LibrarySqliteError::MalformedSchemaState(detail.into())
}

#[cfg(test)]
mod tests {
    use super::{map_library_tree_node, map_read_library_tree_children_reply};
    use library_boundary_protocol as protocol;
    use library_store_sqlite as store;

    fn directory_node(
        has_primary_media_descendant: bool,
        has_image_media_descendant: bool,
        dir_scan_state: &str,
    ) -> store::StoreLiteralHierarchyNode {
        store::StoreLiteralHierarchyNode {
            node_kind: "directory".to_string(),
            source_id: 7,
            source_directory_id: Some(11),
            source_file_id: None,
            parent_source_directory_id: None,
            relative_path: "Albums".to_string(),
            display_name: "Albums".to_string(),
            media_class: None,
            presence_state: "present".to_string(),
            size_bytes: None,
            modified_at_ns: None,
            updated_at: 100,
            has_child_directories: Some(true),
            has_primary_media_descendant: Some(has_primary_media_descendant),
            has_image_media_descendant: Some(has_image_media_descendant),
            dir_scan_state: Some(dir_scan_state.to_string()),
        }
    }

    fn file_node() -> store::StoreLiteralHierarchyNode {
        store::StoreLiteralHierarchyNode {
            node_kind: "file".to_string(),
            source_id: 7,
            source_directory_id: None,
            source_file_id: Some(31),
            parent_source_directory_id: Some(11),
            relative_path: "Albums/track.flac".to_string(),
            display_name: "track.flac".to_string(),
            media_class: Some("audio".to_string()),
            presence_state: "present".to_string(),
            size_bytes: Some(10),
            modified_at_ns: Some(20),
            updated_at: 100,
            has_child_directories: None,
            has_primary_media_descendant: None,
            has_image_media_descendant: None,
            dir_scan_state: None,
        }
    }

    #[test]
    fn library_tree_mapping_returns_directory_coverage_facts_and_omits_them_for_files() {
        let reply =
            map_read_library_tree_children_reply(Some(store::StoreLiteralHierarchyWindow {
                entry_point: store::StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
                parent_source_directory_id: None,
                offset: 0,
                limit: 25,
                total_rows: 2,
                coverage: store::StoreLiteralHierarchyCoverage {
                    state: store::StoreLiteralHierarchyCoverageState::Scanning,
                    recursive_scope_complete: false,
                    empty_result_authoritative: false,
                    detail: Some("Still indexing.".to_string()),
                },
                rows: vec![directory_node(true, false, "scanning"), file_node()],
            }))
            .expect("map library tree reply");
        let window = reply.window.expect("window");
        assert_eq!(
            window.coverage.state,
            protocol::LibraryTreeCoverageState::Scanning
        );

        let directory = &window.rows[0];
        assert_eq!(directory.has_child_directories, Some(true));
        assert_eq!(
            directory.directory_primary_media_state,
            Some(protocol::DirectoryPrimaryMediaState::HasPrimaryMediaDescendants)
        );
        assert_eq!(
            directory.directory_image_media_state,
            Some(protocol::DirectoryImageMediaState::Unknown)
        );
        assert_eq!(
            directory.directory_scan_state,
            Some(protocol::DirectoryScanState::Scanning)
        );
        assert_eq!(
            directory.child_row_state,
            Some(protocol::ChildRowState::HasChildRows)
        );

        let file = &window.rows[1];
        assert_eq!(
            file.media_class,
            Some(protocol::LibraryTreeFileMediaClass::Audio)
        );
        assert_eq!(file.has_child_directories, None);
        assert_eq!(file.directory_primary_media_state, None);
        assert_eq!(file.directory_image_media_state, None);
        assert_eq!(file.directory_scan_state, None);
        assert_eq!(file.child_row_state, None);
    }

    #[test]
    fn library_tree_mapping_preserves_file_media_class_values() {
        for (stored, expected) in [
            ("audio", protocol::LibraryTreeFileMediaClass::Audio),
            ("video", protocol::LibraryTreeFileMediaClass::Video),
            ("image", protocol::LibraryTreeFileMediaClass::Image),
            (
                "unsupported",
                protocol::LibraryTreeFileMediaClass::Unsupported,
            ),
            ("none", protocol::LibraryTreeFileMediaClass::None),
        ] {
            let mut node = file_node();
            node.display_name = format!("fixture-{stored}");
            node.media_class = Some(stored.to_string());

            let mapped = map_library_tree_node(node).expect("map file node");

            assert_eq!(mapped.media_class, Some(expected), "{stored}");
        }
    }

    #[test]
    fn directory_media_states_require_complete_coverage_for_negative_knowledge() {
        let complete = map_library_tree_node(directory_node(false, false, "complete"))
            .expect("map complete directory");
        assert_eq!(
            complete.directory_primary_media_state,
            Some(protocol::DirectoryPrimaryMediaState::NoPrimaryMediaDescendants)
        );
        assert_eq!(
            complete.directory_image_media_state,
            Some(protocol::DirectoryImageMediaState::NoImageMediaDescendants)
        );
        assert_eq!(
            complete.child_row_state,
            Some(protocol::ChildRowState::NoChildRows)
        );

        for scan_state in ["pending", "scanning", "blocked", "failed"] {
            let mapped = map_library_tree_node(directory_node(false, false, scan_state))
                .expect("map incomplete directory");
            assert_eq!(
                mapped.directory_primary_media_state,
                Some(protocol::DirectoryPrimaryMediaState::Unknown),
                "{scan_state} must not map to confirmed no-primary-media"
            );
            assert_eq!(
                mapped.directory_image_media_state,
                Some(protocol::DirectoryImageMediaState::Unknown),
                "{scan_state} must not map to confirmed no-media"
            );
            assert_eq!(
                mapped.child_row_state,
                Some(protocol::ChildRowState::Unknown),
                "{scan_state} must not map to confirmed no-child-rows"
            );
        }
    }

    #[test]
    fn malformed_directory_scan_state_is_rejected() {
        let error = map_library_tree_node(directory_node(false, false, "unsupported"))
            .expect_err("unsupported scan state must fail");
        let detail = match error {
            store::LibrarySqliteError::MalformedSchemaState(detail) => detail,
            other => panic!("expected malformed store state, found {other:?}"),
        };
        assert!(detail.contains("unsupported dir_scan_state"));
    }
}
