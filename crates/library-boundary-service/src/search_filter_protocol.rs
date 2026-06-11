use library_boundary_protocol as protocol;
use library_store_sqlite as store;

pub(crate) fn store_search_filter_request(
    request: protocol::SearchFilterReadRequest,
    limit: usize,
) -> store::StoreSearchRequest {
    store::StoreSearchRequest {
        scope: store_search_scope(request.scope),
        recursion: store_search_recursion(request.recursion),
        text_query: request.text_query,
        target_kinds: request
            .target_kinds
            .into_iter()
            .map(store_search_result_kind)
            .collect(),
        filters: store_search_filters(request.filters),
        sort: store_search_sort(request.sort),
        limit,
        cursor: request.cursor,
    }
}

pub(crate) fn map_search_filter_read_reply(
    result: store::StoreSearchResult,
) -> store::LibrarySqliteResult<protocol::SearchFilterReadReply> {
    Ok(protocol::SearchFilterReadReply {
        result: protocol::SearchFilterResult {
            state: map_search_state(result.state),
            query_identity: map_search_query_identity(result.query_identity),
            index_generation: result.index_generation,
            index_state: map_search_index_state(result.index_state),
            rows: result
                .rows
                .into_iter()
                .map(map_search_result_row)
                .collect::<store::LibrarySqliteResult<Vec<_>>>()?,
            next_cursor: result.next_cursor,
            detail: result.detail,
        },
    })
}

fn store_search_scope(scope: protocol::SearchFilterScope) -> store::StoreSearchScope {
    match scope {
        protocol::SearchFilterScope::Library => store::StoreSearchScope::Library,
        protocol::SearchFilterScope::Source { source_id } => {
            store::StoreSearchScope::Source { source_id }
        }
        protocol::SearchFilterScope::SourceLocation { source_location_id } => {
            store::StoreSearchScope::SourceLocation { source_location_id }
        }
        protocol::SearchFilterScope::Directory {
            source_id,
            source_directory_id,
        } => store::StoreSearchScope::Directory {
            source_id,
            source_directory_id,
        },
    }
}

const fn store_search_recursion(
    recursion: protocol::SearchFilterRecursion,
) -> store::StoreSearchRecursion {
    match recursion {
        protocol::SearchFilterRecursion::Immediate => store::StoreSearchRecursion::Immediate,
        protocol::SearchFilterRecursion::Recursive => store::StoreSearchRecursion::Recursive,
    }
}

const fn store_search_sort(sort: protocol::SearchFilterSort) -> store::StoreSearchSort {
    match sort {
        protocol::SearchFilterSort::PathName => store::StoreSearchSort::PathName,
        protocol::SearchFilterSort::Relevance => store::StoreSearchSort::Relevance,
    }
}

const fn store_search_result_kind(
    kind: protocol::SearchFilterResultKind,
) -> store::StoreSearchResultKind {
    match kind {
        protocol::SearchFilterResultKind::Source => store::StoreSearchResultKind::Source,
        protocol::SearchFilterResultKind::SourceLocation => {
            store::StoreSearchResultKind::SourceLocation
        }
        protocol::SearchFilterResultKind::Directory => store::StoreSearchResultKind::Directory,
        protocol::SearchFilterResultKind::SourceFile => store::StoreSearchResultKind::SourceFile,
    }
}

fn store_search_filters(filters: protocol::SearchFilterSet) -> store::StoreSearchFilters {
    store::StoreSearchFilters {
        file_classes: filters
            .file_classes
            .into_iter()
            .map(store_search_file_class)
            .collect(),
        file_kinds: filters
            .file_kinds
            .into_iter()
            .map(store_search_file_kind)
            .collect(),
        media_relevance: filters
            .media_relevance
            .into_iter()
            .map(store_search_media_relevance)
            .collect(),
        presence_states: filters
            .presence_states
            .into_iter()
            .map(store_search_presence_state)
            .collect(),
        source_access_states: filters
            .source_access_states
            .into_iter()
            .map(store_search_access_state)
            .collect(),
        blake3: filters.blake3.map(store_search_evidence_availability),
        probe: filters.probe.map(store_search_evidence_availability),
        attachment_link_states: filters
            .attachment_link_states
            .into_iter()
            .map(store_search_attachment_link_state)
            .collect(),
    }
}

const fn store_search_file_class(
    value: protocol::SearchFilterFileClass,
) -> store::StoreSearchFileClass {
    match value {
        protocol::SearchFilterFileClass::Audio => store::StoreSearchFileClass::Audio,
        protocol::SearchFilterFileClass::Video => store::StoreSearchFileClass::Video,
        protocol::SearchFilterFileClass::Image => store::StoreSearchFileClass::Image,
        protocol::SearchFilterFileClass::Unsupported => store::StoreSearchFileClass::Unsupported,
        protocol::SearchFilterFileClass::None => store::StoreSearchFileClass::None,
    }
}

const fn store_search_file_kind(value: protocol::ContentsFileKind) -> store::StoreSearchFileKind {
    match value {
        protocol::ContentsFileKind::Audio => store::StoreSearchFileKind::Audio,
        protocol::ContentsFileKind::Video => store::StoreSearchFileKind::Video,
        protocol::ContentsFileKind::Image => store::StoreSearchFileKind::Image,
        protocol::ContentsFileKind::CueSheet => store::StoreSearchFileKind::CueSheet,
        protocol::ContentsFileKind::LogDoc => store::StoreSearchFileKind::LogDoc,
        protocol::ContentsFileKind::TextDoc => store::StoreSearchFileKind::TextDoc,
        protocol::ContentsFileKind::Archive => store::StoreSearchFileKind::Archive,
        protocol::ContentsFileKind::Other => store::StoreSearchFileKind::Other,
        protocol::ContentsFileKind::Unknown => store::StoreSearchFileKind::Unknown,
    }
}

const fn store_search_media_relevance(
    value: protocol::SearchFilterMediaRelevance,
) -> store::StoreSearchMediaRelevance {
    match value {
        protocol::SearchFilterMediaRelevance::AudioWorkflow => {
            store::StoreSearchMediaRelevance::AudioWorkflow
        }
        protocol::SearchFilterMediaRelevance::PlayableMedia => {
            store::StoreSearchMediaRelevance::PlayableMedia
        }
        protocol::SearchFilterMediaRelevance::ExplicitInventory => {
            store::StoreSearchMediaRelevance::ExplicitInventory
        }
        protocol::SearchFilterMediaRelevance::CompanionFile => {
            store::StoreSearchMediaRelevance::CompanionFile
        }
        protocol::SearchFilterMediaRelevance::NotMediaRelevant => {
            store::StoreSearchMediaRelevance::NotMediaRelevant
        }
    }
}

const fn store_search_presence_state(
    value: protocol::ContentsPresenceState,
) -> store::StoreSearchPresenceState {
    match value {
        protocol::ContentsPresenceState::Present => store::StoreSearchPresenceState::Present,
        protocol::ContentsPresenceState::Missing => store::StoreSearchPresenceState::Missing,
        protocol::ContentsPresenceState::Removed => store::StoreSearchPresenceState::Removed,
    }
}

const fn store_search_access_state(
    value: protocol::SearchFilterSourceAccessState,
) -> store::StoreSearchAccessState {
    match value {
        protocol::SearchFilterSourceAccessState::Accessible => {
            store::StoreSearchAccessState::Accessible
        }
        protocol::SearchFilterSourceAccessState::Missing => store::StoreSearchAccessState::Missing,
        protocol::SearchFilterSourceAccessState::Blocked => store::StoreSearchAccessState::Blocked,
        protocol::SearchFilterSourceAccessState::Unknown => store::StoreSearchAccessState::Unknown,
    }
}

const fn store_search_evidence_availability(
    value: protocol::SearchFilterEvidenceAvailability,
) -> store::StoreSearchEvidenceAvailability {
    match value {
        protocol::SearchFilterEvidenceAvailability::HasCurrent => {
            store::StoreSearchEvidenceAvailability::HasCurrent
        }
        protocol::SearchFilterEvidenceAvailability::MissingCurrent => {
            store::StoreSearchEvidenceAvailability::MissingCurrent
        }
    }
}

const fn store_search_attachment_link_state(
    value: protocol::SearchFilterAttachmentLinkState,
) -> store::StoreSearchAttachmentLinkState {
    match value {
        protocol::SearchFilterAttachmentLinkState::Current => {
            store::StoreSearchAttachmentLinkState::Current
        }
        protocol::SearchFilterAttachmentLinkState::Stale => {
            store::StoreSearchAttachmentLinkState::Stale
        }
        protocol::SearchFilterAttachmentLinkState::Missing => {
            store::StoreSearchAttachmentLinkState::Missing
        }
        protocol::SearchFilterAttachmentLinkState::NotApplicable => {
            store::StoreSearchAttachmentLinkState::NotApplicable
        }
    }
}

fn map_search_query_identity(
    identity: store::StoreSearchQueryIdentity,
) -> protocol::SearchFilterQueryIdentity {
    protocol::SearchFilterQueryIdentity {
        scope: map_search_scope(identity.scope),
        recursion: map_search_recursion(identity.recursion),
        text_query: identity.text_query,
        target_kinds: identity
            .target_kinds
            .into_iter()
            .map(map_search_result_kind)
            .collect(),
        filters: map_search_filters(identity.filters),
        sort: map_search_sort(identity.sort),
        page_size: identity.page_size,
        index_generation: identity.index_generation,
    }
}

fn map_search_result_row(
    row: store::StoreSearchResultRow,
) -> store::LibrarySqliteResult<protocol::SearchFilterResultRow> {
    Ok(protocol::SearchFilterResultRow {
        result_kind: map_search_result_kind(row.result_kind),
        authority_layer: map_search_authority_layer(row.authority_layer),
        stable_key: row.stable_key,
        source_id: row.source_id,
        source_location_id: row.source_location_id,
        source_directory_id: row.source_directory_id,
        parent_source_directory_id: row.parent_source_directory_id,
        source_file_id: row.source_file_id,
        display_label: row.display_label,
        display_path: row.display_path,
        relative_path: row.relative_path,
        file_class: row
            .file_class
            .as_deref()
            .map(map_search_file_class_value)
            .transpose()?,
        file_kind: row
            .file_kind
            .as_deref()
            .map(map_contents_file_kind_value)
            .transpose()?,
        media_relevance: row
            .media_relevance
            .as_deref()
            .map(map_search_media_relevance_value)
            .transpose()?,
        presence_state: row
            .presence_state
            .as_deref()
            .map(map_contents_presence_state_value)
            .transpose()?,
        source_access_state: row
            .source_access_state
            .as_deref()
            .map(map_search_access_state_value)
            .transpose()?,
        source_scan_phase: row
            .source_scan_phase
            .as_deref()
            .map(map_source_scan_phase)
            .transpose()?,
        has_current_blake3: row.has_current_blake3,
        has_current_probe: row.has_current_probe,
        attachment_link_state: map_search_attachment_link_state(row.attachment_link_state),
        attachment_id: row.attachment_id,
        content_hash_algorithm: row.content_hash_algorithm,
        content_hash_value: row.content_hash_value,
        evidence_coverage_state: map_search_evidence_coverage_state(row.evidence_coverage_state),
        match_reason: map_search_match_reason(row.match_reason),
        updated_at_ms: row.updated_at,
    })
}

const fn map_search_state(state: store::StoreSearchState) -> protocol::SearchFilterState {
    match state {
        store::StoreSearchState::Ready => protocol::SearchFilterState::Ready,
        store::StoreSearchState::Empty => protocol::SearchFilterState::Empty,
        store::StoreSearchState::Partial => protocol::SearchFilterState::Partial,
        store::StoreSearchState::CursorInvalid => protocol::SearchFilterState::CursorInvalid,
        store::StoreSearchState::Unsupported => protocol::SearchFilterState::Unsupported,
    }
}

const fn map_search_index_state(
    state: store::StoreSearchIndexState,
) -> protocol::SearchFilterIndexState {
    match state {
        store::StoreSearchIndexState::Ready => protocol::SearchFilterIndexState::Ready,
        store::StoreSearchIndexState::Rebuilding => protocol::SearchFilterIndexState::Rebuilding,
        store::StoreSearchIndexState::Partial => protocol::SearchFilterIndexState::Partial,
        store::StoreSearchIndexState::Failed => protocol::SearchFilterIndexState::Failed,
        store::StoreSearchIndexState::Missing => protocol::SearchFilterIndexState::Missing,
    }
}

const fn map_search_scope(scope: store::StoreSearchScope) -> protocol::SearchFilterScope {
    match scope {
        store::StoreSearchScope::Library => protocol::SearchFilterScope::Library,
        store::StoreSearchScope::Source { source_id } => {
            protocol::SearchFilterScope::Source { source_id }
        }
        store::StoreSearchScope::SourceLocation { source_location_id } => {
            protocol::SearchFilterScope::SourceLocation { source_location_id }
        }
        store::StoreSearchScope::Directory {
            source_id,
            source_directory_id,
        } => protocol::SearchFilterScope::Directory {
            source_id,
            source_directory_id,
        },
    }
}

const fn map_search_recursion(
    recursion: store::StoreSearchRecursion,
) -> protocol::SearchFilterRecursion {
    match recursion {
        store::StoreSearchRecursion::Immediate => protocol::SearchFilterRecursion::Immediate,
        store::StoreSearchRecursion::Recursive => protocol::SearchFilterRecursion::Recursive,
    }
}

const fn map_search_sort(sort: store::StoreSearchSort) -> protocol::SearchFilterSort {
    match sort {
        store::StoreSearchSort::PathName => protocol::SearchFilterSort::PathName,
        store::StoreSearchSort::Relevance => protocol::SearchFilterSort::Relevance,
    }
}

const fn map_search_result_kind(
    kind: store::StoreSearchResultKind,
) -> protocol::SearchFilterResultKind {
    match kind {
        store::StoreSearchResultKind::Source => protocol::SearchFilterResultKind::Source,
        store::StoreSearchResultKind::SourceLocation => {
            protocol::SearchFilterResultKind::SourceLocation
        }
        store::StoreSearchResultKind::Directory => protocol::SearchFilterResultKind::Directory,
        store::StoreSearchResultKind::SourceFile => protocol::SearchFilterResultKind::SourceFile,
    }
}

fn map_search_filters(filters: store::StoreSearchFilters) -> protocol::SearchFilterSet {
    protocol::SearchFilterSet {
        file_classes: filters
            .file_classes
            .into_iter()
            .map(map_search_file_class)
            .collect(),
        file_kinds: filters
            .file_kinds
            .into_iter()
            .map(map_search_file_kind)
            .collect(),
        media_relevance: filters
            .media_relevance
            .into_iter()
            .map(map_search_media_relevance)
            .collect(),
        presence_states: filters
            .presence_states
            .into_iter()
            .map(map_search_presence_state)
            .collect(),
        source_access_states: filters
            .source_access_states
            .into_iter()
            .map(map_search_access_state)
            .collect(),
        blake3: filters.blake3.map(map_search_evidence_availability),
        probe: filters.probe.map(map_search_evidence_availability),
        attachment_link_states: filters
            .attachment_link_states
            .into_iter()
            .map(map_search_attachment_link_state)
            .collect(),
    }
}

const fn map_search_file_class(
    value: store::StoreSearchFileClass,
) -> protocol::SearchFilterFileClass {
    match value {
        store::StoreSearchFileClass::Audio => protocol::SearchFilterFileClass::Audio,
        store::StoreSearchFileClass::Video => protocol::SearchFilterFileClass::Video,
        store::StoreSearchFileClass::Image => protocol::SearchFilterFileClass::Image,
        store::StoreSearchFileClass::Unsupported => protocol::SearchFilterFileClass::Unsupported,
        store::StoreSearchFileClass::None => protocol::SearchFilterFileClass::None,
    }
}

const fn map_search_file_kind(value: store::StoreSearchFileKind) -> protocol::ContentsFileKind {
    match value {
        store::StoreSearchFileKind::Audio => protocol::ContentsFileKind::Audio,
        store::StoreSearchFileKind::Video => protocol::ContentsFileKind::Video,
        store::StoreSearchFileKind::Image => protocol::ContentsFileKind::Image,
        store::StoreSearchFileKind::CueSheet => protocol::ContentsFileKind::CueSheet,
        store::StoreSearchFileKind::LogDoc => protocol::ContentsFileKind::LogDoc,
        store::StoreSearchFileKind::TextDoc => protocol::ContentsFileKind::TextDoc,
        store::StoreSearchFileKind::Archive => protocol::ContentsFileKind::Archive,
        store::StoreSearchFileKind::Other => protocol::ContentsFileKind::Other,
        store::StoreSearchFileKind::Unknown => protocol::ContentsFileKind::Unknown,
    }
}

const fn map_search_media_relevance(
    value: store::StoreSearchMediaRelevance,
) -> protocol::SearchFilterMediaRelevance {
    match value {
        store::StoreSearchMediaRelevance::AudioWorkflow => {
            protocol::SearchFilterMediaRelevance::AudioWorkflow
        }
        store::StoreSearchMediaRelevance::PlayableMedia => {
            protocol::SearchFilterMediaRelevance::PlayableMedia
        }
        store::StoreSearchMediaRelevance::ExplicitInventory => {
            protocol::SearchFilterMediaRelevance::ExplicitInventory
        }
        store::StoreSearchMediaRelevance::CompanionFile => {
            protocol::SearchFilterMediaRelevance::CompanionFile
        }
        store::StoreSearchMediaRelevance::NotMediaRelevant => {
            protocol::SearchFilterMediaRelevance::NotMediaRelevant
        }
    }
}

const fn map_search_presence_state(
    value: store::StoreSearchPresenceState,
) -> protocol::ContentsPresenceState {
    match value {
        store::StoreSearchPresenceState::Present => protocol::ContentsPresenceState::Present,
        store::StoreSearchPresenceState::Missing => protocol::ContentsPresenceState::Missing,
        store::StoreSearchPresenceState::Removed => protocol::ContentsPresenceState::Removed,
    }
}

const fn map_search_access_state(
    value: store::StoreSearchAccessState,
) -> protocol::SearchFilterSourceAccessState {
    match value {
        store::StoreSearchAccessState::Accessible => {
            protocol::SearchFilterSourceAccessState::Accessible
        }
        store::StoreSearchAccessState::Missing => protocol::SearchFilterSourceAccessState::Missing,
        store::StoreSearchAccessState::Blocked => protocol::SearchFilterSourceAccessState::Blocked,
        store::StoreSearchAccessState::Unknown => protocol::SearchFilterSourceAccessState::Unknown,
    }
}

const fn map_search_evidence_availability(
    value: store::StoreSearchEvidenceAvailability,
) -> protocol::SearchFilterEvidenceAvailability {
    match value {
        store::StoreSearchEvidenceAvailability::HasCurrent => {
            protocol::SearchFilterEvidenceAvailability::HasCurrent
        }
        store::StoreSearchEvidenceAvailability::MissingCurrent => {
            protocol::SearchFilterEvidenceAvailability::MissingCurrent
        }
    }
}

const fn map_search_attachment_link_state(
    value: store::StoreSearchAttachmentLinkState,
) -> protocol::SearchFilterAttachmentLinkState {
    match value {
        store::StoreSearchAttachmentLinkState::Current => {
            protocol::SearchFilterAttachmentLinkState::Current
        }
        store::StoreSearchAttachmentLinkState::Stale => {
            protocol::SearchFilterAttachmentLinkState::Stale
        }
        store::StoreSearchAttachmentLinkState::Missing => {
            protocol::SearchFilterAttachmentLinkState::Missing
        }
        store::StoreSearchAttachmentLinkState::NotApplicable => {
            protocol::SearchFilterAttachmentLinkState::NotApplicable
        }
    }
}

const fn map_search_authority_layer(
    authority_layer: store::StoreSearchAuthorityLayer,
) -> protocol::SearchFilterAuthorityLayer {
    match authority_layer {
        store::StoreSearchAuthorityLayer::Source => protocol::SearchFilterAuthorityLayer::Source,
        store::StoreSearchAuthorityLayer::SourceLocation => {
            protocol::SearchFilterAuthorityLayer::SourceLocation
        }
        store::StoreSearchAuthorityLayer::SourceHierarchy => {
            protocol::SearchFilterAuthorityLayer::SourceHierarchy
        }
        store::StoreSearchAuthorityLayer::SourceFileInventory => {
            protocol::SearchFilterAuthorityLayer::SourceFileInventory
        }
    }
}

const fn map_search_evidence_coverage_state(
    state: store::StoreSearchEvidenceCoverageState,
) -> protocol::SearchFilterEvidenceCoverageState {
    match state {
        store::StoreSearchEvidenceCoverageState::Indexed => {
            protocol::SearchFilterEvidenceCoverageState::Indexed
        }
        store::StoreSearchEvidenceCoverageState::NotApplicable => {
            protocol::SearchFilterEvidenceCoverageState::NotApplicable
        }
    }
}

const fn map_search_match_reason(
    reason: store::StoreSearchMatchReason,
) -> protocol::SearchFilterMatchReason {
    match reason {
        store::StoreSearchMatchReason::Filter => protocol::SearchFilterMatchReason::Filter,
        store::StoreSearchMatchReason::ExactLabel => protocol::SearchFilterMatchReason::ExactLabel,
        store::StoreSearchMatchReason::LabelPrefix => {
            protocol::SearchFilterMatchReason::LabelPrefix
        }
        store::StoreSearchMatchReason::Label => protocol::SearchFilterMatchReason::Label,
        store::StoreSearchMatchReason::Path => protocol::SearchFilterMatchReason::Path,
        store::StoreSearchMatchReason::Text => protocol::SearchFilterMatchReason::Text,
    }
}

fn map_search_file_class_value(
    value: &str,
) -> store::LibrarySqliteResult<protocol::SearchFilterFileClass> {
    match value {
        "audio" => Ok(protocol::SearchFilterFileClass::Audio),
        "video" => Ok(protocol::SearchFilterFileClass::Video),
        "image" => Ok(protocol::SearchFilterFileClass::Image),
        "unsupported" => Ok(protocol::SearchFilterFileClass::Unsupported),
        "none" => Ok(protocol::SearchFilterFileClass::None),
        other => Err(invalid_search_filter_value("file_class", other)),
    }
}

fn map_contents_file_kind_value(
    value: &str,
) -> store::LibrarySqliteResult<protocol::ContentsFileKind> {
    protocol::ContentsFileKind::from_projection_value(value)
        .ok_or_else(|| invalid_search_filter_value("file_kind", value))
}

fn map_contents_presence_state_value(
    value: &str,
) -> store::LibrarySqliteResult<protocol::ContentsPresenceState> {
    protocol::ContentsPresenceState::from_projection_value(value)
        .ok_or_else(|| invalid_search_filter_value("presence_state", value))
}

fn map_search_media_relevance_value(
    value: &str,
) -> store::LibrarySqliteResult<protocol::SearchFilterMediaRelevance> {
    match value {
        "audio_workflow" => Ok(protocol::SearchFilterMediaRelevance::AudioWorkflow),
        "playable_media" => Ok(protocol::SearchFilterMediaRelevance::PlayableMedia),
        "explicit_inventory" => Ok(protocol::SearchFilterMediaRelevance::ExplicitInventory),
        "companion_file" => Ok(protocol::SearchFilterMediaRelevance::CompanionFile),
        "not_media_relevant" => Ok(protocol::SearchFilterMediaRelevance::NotMediaRelevant),
        other => Err(invalid_search_filter_value("media_relevance", other)),
    }
}

fn map_search_access_state_value(
    value: &str,
) -> store::LibrarySqliteResult<protocol::SearchFilterSourceAccessState> {
    match value {
        "accessible" => Ok(protocol::SearchFilterSourceAccessState::Accessible),
        "missing" => Ok(protocol::SearchFilterSourceAccessState::Missing),
        "blocked" => Ok(protocol::SearchFilterSourceAccessState::Blocked),
        "unknown" => Ok(protocol::SearchFilterSourceAccessState::Unknown),
        other => Err(invalid_search_filter_value("source_access_state", other)),
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
        other => Err(invalid_search_filter_value("source_scan_phase", other)),
    }
}

fn invalid_search_filter_value(field_name: &str, value: &str) -> store::LibrarySqliteError {
    store::LibrarySqliteError::MalformedSchemaState(format!(
        "search/filter index field {field_name} contains unsupported value {value:?}"
    ))
}
