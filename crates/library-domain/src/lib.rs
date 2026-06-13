#![deny(unsafe_code)]

pub mod artifact;
pub mod browser;
pub mod ids;
pub mod navigation;
pub mod projection;
pub mod source;
pub mod work;

pub use artifact::{ArtifactKind, ArtifactRole, ArtifactStorageKind};
pub use browser::LibraryBrowseScope;
pub use ids::{
    ArtifactId, ProjectionSubscriberId, SourceDirectoryId, SourceFileId, SourceId,
    SourceLocationId, WorkItemId, WorkRunId,
};
pub use navigation::{
    EncodedNavigationSelector, NavigationSelector, NavigationSelectorDecodeError,
    compile_library_browse_scope, decode_selector, encode_selector,
};
pub use projection::ProjectionDomain;
pub use source::{
    SourceAccessIssueKind, SourceAccessState, SourceAvailabilityState, SourcePresenceState,
    SourceScanPhase,
};
pub use work::{
    MachineWorkKind, WorkItemState, WorkPriorityClass, WorkRunOutcome, WorkSubject, WorkSubjectKind,
};

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_round_trip {
        ($ty:ty, [$(($variant:path, $value:literal)),+ $(,)?]) => {
            $(
                assert_eq!($variant.as_str(), $value);
                assert_eq!(<$ty>::parse($value), Some($variant));
            )+
        };
    }

    fn assert_id<T: Copy + std::fmt::Debug + PartialEq>(
        new: fn(i64) -> Option<T>,
        get: fn(T) -> i64,
    ) {
        assert_eq!(new(-1), None);
        assert_eq!(new(0), None);
        let id = new(1).expect("positive id is valid");
        assert_eq!(get(id), 1);
    }

    #[test]
    fn typed_ids_reject_non_positive_values() {
        assert_id(SourceId::new, SourceId::get);
        assert_id(SourceLocationId::new, SourceLocationId::get);
        assert_id(SourceDirectoryId::new, SourceDirectoryId::get);
        assert_id(SourceFileId::new, SourceFileId::get);
        assert_id(WorkItemId::new, WorkItemId::get);
        assert_id(WorkRunId::new, WorkRunId::get);
        assert_id(ArtifactId::new, ArtifactId::get);
        assert_id(ProjectionSubscriberId::new, ProjectionSubscriberId::get);
    }

    #[test]
    fn finite_enums_round_trip_through_strings() {
        assert_round_trip!(
            SourcePresenceState,
            [
                (SourcePresenceState::Present, "present"),
                (SourcePresenceState::Missing, "missing"),
                (SourcePresenceState::Removed, "removed"),
            ]
        );
        assert_round_trip!(
            SourceAvailabilityState,
            [
                (SourceAvailabilityState::Available, "available"),
                (SourceAvailabilityState::Unavailable, "unavailable"),
                (SourceAvailabilityState::Degraded, "degraded"),
            ]
        );
        assert_round_trip!(
            SourceAccessState,
            [
                (SourceAccessState::Accessible, "accessible"),
                (SourceAccessState::Missing, "missing"),
                (SourceAccessState::Blocked, "blocked"),
                (SourceAccessState::Unknown, "unknown"),
            ]
        );
        assert_round_trip!(
            SourceAccessIssueKind,
            [
                (SourceAccessIssueKind::Missing, "missing"),
                (SourceAccessIssueKind::NotDirectory, "not_directory"),
                (SourceAccessIssueKind::PermissionDenied, "permission_denied"),
                (
                    SourceAccessIssueKind::PrivacyPermissionRequired,
                    "privacy_permission_required"
                ),
                (SourceAccessIssueKind::UnavailableMount, "unavailable_mount"),
                (SourceAccessIssueKind::ResourceBusy, "resource_busy"),
                (
                    SourceAccessIssueKind::StaleNetworkHandle,
                    "stale_network_handle"
                ),
                (SourceAccessIssueKind::SymlinkLoop, "symlink_loop"),
                (
                    SourceAccessIssueKind::SymlinkEscapeBlocked,
                    "symlink_escape_blocked"
                ),
                (SourceAccessIssueKind::UnsupportedPath, "unsupported_path"),
                (SourceAccessIssueKind::InvalidPath, "invalid_path"),
                (SourceAccessIssueKind::IoInterrupted, "io_interrupted"),
                (SourceAccessIssueKind::TimedOut, "timed_out"),
                (SourceAccessIssueKind::UnknownIo, "unknown_io"),
            ]
        );
        assert_round_trip!(
            SourceScanPhase,
            [
                (SourceScanPhase::Idle, "idle"),
                (SourceScanPhase::Scanning, "scanning"),
                (SourceScanPhase::Complete, "complete"),
                (SourceScanPhase::Partial, "partial"),
                (SourceScanPhase::Blocked, "blocked"),
                (SourceScanPhase::Failed, "failed"),
            ]
        );
        assert_round_trip!(
            WorkPriorityClass,
            [
                (WorkPriorityClass::Urgent, "urgent"),
                (WorkPriorityClass::Interactive, "interactive"),
                (WorkPriorityClass::Background, "background"),
            ]
        );
        assert_round_trip!(
            WorkSubjectKind,
            [
                (WorkSubjectKind::SourceFile, "source_file"),
                (WorkSubjectKind::ProjectionDomain, "projection_domain"),
            ]
        );
        assert_round_trip!(
            MachineWorkKind,
            [
                (MachineWorkKind::InspectSourceFile, "inspect_source_file"),
                (MachineWorkKind::RebuildProjection, "rebuild_projection"),
            ]
        );
        assert_round_trip!(
            WorkItemState,
            [
                (WorkItemState::Queued, "queued"),
                (WorkItemState::Leased, "leased"),
                (WorkItemState::Completed, "completed"),
                (WorkItemState::Blocked, "blocked"),
                (WorkItemState::Failed, "failed"),
                (WorkItemState::Canceled, "canceled"),
            ]
        );
        assert_round_trip!(
            WorkRunOutcome,
            [
                (WorkRunOutcome::Running, "running"),
                (WorkRunOutcome::Completed, "completed"),
                (WorkRunOutcome::Blocked, "blocked"),
                (WorkRunOutcome::Failed, "failed"),
                (WorkRunOutcome::Canceled, "canceled"),
            ]
        );
        assert_round_trip!(
            ArtifactKind,
            [
                (ArtifactKind::InspectionResult, "inspection_result"),
                (ArtifactKind::ProjectionSnapshot, "projection_snapshot"),
            ]
        );
        assert_round_trip!(
            ArtifactRole,
            [(ArtifactRole::PrimaryResult, "primary_result")]
        );
        assert_round_trip!(
            ArtifactStorageKind,
            [
                (ArtifactStorageKind::InlinePayload, "inline_payload"),
                (ArtifactStorageKind::FileStore, "file_store"),
            ]
        );
        assert_round_trip!(
            ProjectionDomain,
            [(ProjectionDomain::Navigation, "navigation")]
        );
    }

    #[test]
    fn work_subjects_encode_and_decode_storage_pairs() {
        let source_file = SourceFileId::new(10).unwrap();

        assert_eq!(
            WorkSubject::SourceFile(source_file).kind(),
            WorkSubjectKind::SourceFile
        );
        assert_eq!(WorkSubject::SourceFile(source_file).storage_id(), "10");
        assert_eq!(
            WorkSubject::parse(WorkSubjectKind::SourceFile, "10"),
            Some(WorkSubject::SourceFile(source_file))
        );
        assert_eq!(WorkSubject::parse(WorkSubjectKind::SourceFile, "0"), None);

        assert_eq!(
            WorkSubject::ProjectionDomain(ProjectionDomain::Navigation).kind(),
            WorkSubjectKind::ProjectionDomain
        );
        assert_eq!(
            WorkSubject::ProjectionDomain(ProjectionDomain::Navigation).storage_id(),
            "navigation"
        );
        assert_eq!(
            WorkSubject::parse(WorkSubjectKind::ProjectionDomain, "navigation"),
            Some(WorkSubject::ProjectionDomain(ProjectionDomain::Navigation))
        );
        assert_eq!(
            WorkSubject::parse(WorkSubjectKind::ProjectionDomain, "missing"),
            None
        );
    }

    fn assert_selector_round_trip(
        selector: NavigationSelector,
        expected_kind: &'static str,
        expected_payload: &str,
    ) {
        let encoded = encode_selector(&selector);
        assert_eq!(encoded.kind, expected_kind);
        assert_eq!(encoded.payload, expected_payload);
        assert_eq!(
            decode_selector(encoded.kind, &encoded.payload),
            Ok(selector)
        );
    }

    #[test]
    fn navigation_selectors_encode_and_decode_supported_variants() {
        let source_id = SourceId::new(1).unwrap();
        let source_location_id = SourceLocationId::new(2).unwrap();

        assert_selector_round_trip(NavigationSelector::AllMedia, "all_media", "");
        assert_selector_round_trip(NavigationSelector::AllAudio, "all_audio", "");
        assert_selector_round_trip(NavigationSelector::AllVideos, "all_videos", "");
        assert_selector_round_trip(NavigationSelector::RecentlyAdded, "recently_added", "");
        assert_selector_round_trip(NavigationSelector::Source(source_id), "source", "1");
        assert_selector_round_trip(
            NavigationSelector::SourceLocation(source_location_id),
            "source_location",
            "2",
        );
    }

    #[test]
    fn navigation_selectors_reject_unknown_selector_kinds() {
        assert_eq!(
            decode_selector("sleeve", "1"),
            Err(NavigationSelectorDecodeError::UnknownKind(
                "sleeve".to_string()
            ))
        );
    }

    #[test]
    fn navigation_selectors_compile_to_library_browse_scopes() {
        let source_id = SourceId::new(1).unwrap();
        let source_location_id = SourceLocationId::new(2).unwrap();

        assert_eq!(
            compile_library_browse_scope(NavigationSelector::AllMedia),
            Some(LibraryBrowseScope::AllMedia)
        );
        assert_eq!(
            compile_library_browse_scope(NavigationSelector::AllAudio),
            Some(LibraryBrowseScope::AllAudio)
        );
        assert_eq!(
            compile_library_browse_scope(NavigationSelector::AllVideos),
            Some(LibraryBrowseScope::AllVideos)
        );
        assert_eq!(
            compile_library_browse_scope(NavigationSelector::RecentlyAdded),
            Some(LibraryBrowseScope::RecentlyAdded)
        );
        assert_eq!(
            compile_library_browse_scope(NavigationSelector::Source(source_id)),
            Some(LibraryBrowseScope::Source(source_id))
        );
        assert_eq!(
            compile_library_browse_scope(NavigationSelector::SourceLocation(source_location_id)),
            Some(LibraryBrowseScope::SourceLocation(source_location_id))
        );
    }
}
