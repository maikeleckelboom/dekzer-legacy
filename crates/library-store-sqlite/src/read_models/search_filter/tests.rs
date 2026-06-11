use rusqlite::Connection;

use super::*;
use crate::schema::install_baseline_schema_for_test;

fn open_connection() -> Connection {
    let mut connection = Connection::open_in_memory().expect("open in-memory");
    install_baseline_schema_for_test(&mut connection).expect("install schema");
    connection
}

fn seed_source(connection: &Connection) {
    connection
        .execute_batch(
            "INSERT INTO sources (
                 source_id, source_class, authority, identity_key, display_name,
                 is_user_visible, created_at, updated_at
             ) VALUES (1, 'internal', 'system', 'source:search-test', 'Search Test', 1, 1, 1);
             INSERT INTO source_state (
                 source_id, mount_status, mount_epoch, access_state, updated_at
             ) VALUES (1, 'mounted', 1, 'accessible', 1);
             INSERT INTO source_scan_state (
                 source_id, scan_phase, updated_at
             ) VALUES (1, 'complete', 1);
             INSERT INTO source_directories (
                 source_directory_id, source_id, parent_source_directory_id, name,
                 name_browse_sort_key, relative_path, presence_state, dir_scan_state,
                 dir_scan_updated_at, created_at, updated_at
             ) VALUES
                 (10, 1, NULL, 'Music', 'music', 'Music', 'present', 'complete', 1, 1, 1),
                 (11, 1, 10, 'Breaks', 'breaks', 'Music/Breaks', 'present', 'complete', 1, 1, 1);
             INSERT INTO source_locations (
                 source_location_id, source_id, authority, location_kind, relative_path,
                 display_name, is_user_visible, created_at, updated_at
             ) VALUES (100, 1, 'user', 'registered_subpath', 'Music/Breaks', 'Breaks', 1, 1, 1);
             INSERT INTO source_files (
                 source_file_id, source_id, parent_source_directory_id, name,
                 name_browse_sort_key, relative_path_browse_sort_key, relative_path,
                 size_bytes, mtime_ns, file_kind, file_class, presence_state,
                 first_discovered_at, last_observed_at, last_presence_change_at,
                 created_at, updated_at
             ) VALUES
                 (1000, 1, 11, 'Amen.wav', 'amen', 'music/breaks/amen', 'Music/Breaks/Amen.wav', 10, 100, 'audio', 'audio', 'present', 1, 1, 1, 1, 1),
                 (1001, 1, 11, 'Cover.jpg', 'cover', 'music/breaks/cover', 'Music/Breaks/Cover.jpg', 5, 100, 'image', 'image', 'present', 1, 1, 1, 1, 1),
                 (1002, 1, 10, 'Notes.txt', 'notes', 'music/notes', 'Music/Notes.txt', 3, 100, 'text_doc', 'none', 'present', 1, 1, 1, 1, 1);
             INSERT INTO WorkItems (
                 work_item_id, subject_kind, subject_id, work_kind, priority_class,
                 basis_fingerprint, state, created_at, updated_at
             ) VALUES (1, 'source_file', '1000', 'inspect_source', 'interactive', 'basis:1000', 'completed', 1, 1);
             INSERT INTO WorkRuns (
                 work_run_id, work_item_id, adapter_key, adapter_version,
                 started_at, finished_at, outcome
             ) VALUES (1, 1, 'test', '1', 1, 1, 'completed');
             INSERT INTO Artifacts (
                 artifact_id, work_run_id, subject_kind, subject_id, artifact_kind,
                 artifact_role, adapter_key, adapter_version, basis_fingerprint,
                 media_type, storage_kind, payload_hash, created_at
             ) VALUES (1, 1, 'source_file', '1000', 'inspection_result', 'primary_result', 'test', '1', 'basis:1000', 'application/json', 'inline_payload', 'hash', 1);
             INSERT INTO SourceFacts (
                 source_file_id, fact_kind, basis_fingerprint, basis_source_id,
                 basis_relative_path, basis_size_bytes, basis_mtime_ns,
                 basis_presence_state, observed_at_ms, content_hash_algorithm,
                 content_hash_value, media_kind, mime_type, duration_ms,
                 sample_rate_hz, channels, bit_depth, codec, updated_at,
                 accepted_artifact_id
             ) VALUES (1000, 'source_inspection', 'basis:1000', 1, 'Music/Breaks/Amen.wav', 10, 100, 'present', 1, 'blake3', 'abc', 'audio', 'audio/wav', 1000, 44100, 2, 16, 'pcm', 1, 1);
             INSERT INTO content_attachments (
                 attachment_id, content_hash_algorithm, content_hash_value,
                 first_observed_at, updated_at
             ) VALUES (50, 'blake3', 'abc', 1, 1);
             INSERT INTO source_file_attachment_links (
                 source_file_attachment_link_id, attachment_id, source_file_id,
                 source_id, file_kind, created_at, updated_at
             ) VALUES (60, 50, 1000, 1, 'audio', 1, 1);",
        )
        .expect("seed source");
}

fn seed_second_visible_source(connection: &Connection) {
    connection
        .execute_batch(
            "INSERT INTO sources (
                 source_id, source_class, authority, identity_key, display_name,
                 is_user_visible, created_at, updated_at
             ) VALUES (2, 'internal', 'system', 'source:second-search-test', 'Second Source', 1, 1, 1);
             INSERT INTO source_state (
                 source_id, mount_status, mount_epoch, access_state, updated_at
             ) VALUES (2, 'mounted', 1, 'accessible', 1);
             INSERT INTO source_scan_state (
                 source_id, scan_phase, updated_at
             ) VALUES (2, 'complete', 1);",
        )
        .expect("seed second source");
}

fn request(query: Option<&str>) -> StoreSearchRequest {
    StoreSearchRequest {
        scope: StoreSearchScope::Library,
        recursion: StoreSearchRecursion::Recursive,
        text_query: query.map(str::to_string),
        target_kinds: Vec::new(),
        filters: StoreSearchFilters::default(),
        sort: StoreSearchSort::PathName,
        limit: 50,
        cursor: None,
    }
}

#[test]
fn missing_source_coverage_does_not_return_empty() {
    let connection = open_connection();
    seed_source(&connection);

    let mut scoped = request(Some("nothing"));
    scoped.scope = StoreSearchScope::Source { source_id: 1 };
    let result = read_search_filter(&connection, scoped).expect("search");

    assert_eq!(result.state, StoreSearchState::Partial);
    assert_eq!(result.index_state, StoreSearchIndexState::Missing);
    assert!(result.rows.is_empty());
}

#[test]
fn missing_library_coverage_does_not_return_authoritative_empty() {
    let connection = open_connection();
    seed_source(&connection);
    seed_second_visible_source(&connection);
    rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild source 1");

    let result = read_search_filter(&connection, request(Some("not-found"))).expect("search");

    assert_eq!(result.state, StoreSearchState::Partial);
    assert_eq!(result.index_state, StoreSearchIndexState::Missing);
    assert!(result.rows.is_empty());
}

#[test]
fn covered_empty_source_returns_empty() {
    let connection = open_connection();
    seed_source(&connection);
    rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");

    let mut scoped = request(Some("not-found"));
    scoped.scope = StoreSearchScope::Source { source_id: 1 };
    let result = read_search_filter(&connection, scoped).expect("search");

    assert_eq!(result.state, StoreSearchState::Empty);
    assert_eq!(result.index_state, StoreSearchIndexState::Ready);
}

#[test]
fn scoped_coverage_derives_from_owning_source() {
    let connection = open_connection();
    seed_source(&connection);

    let mut location = request(Some("not-found"));
    location.scope = StoreSearchScope::SourceLocation {
        source_location_id: 100,
    };
    let missing = read_search_filter(&connection, location.clone()).expect("location search");
    assert_eq!(missing.state, StoreSearchState::Partial);
    assert_eq!(missing.index_state, StoreSearchIndexState::Missing);

    rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");
    let covered_location = read_search_filter(&connection, location).expect("location search");
    assert_eq!(covered_location.state, StoreSearchState::Empty);

    let mut directory = request(Some("not-found"));
    directory.scope = StoreSearchScope::Directory {
        source_id: 1,
        source_directory_id: 11,
    };
    let covered_directory = read_search_filter(&connection, directory).expect("directory search");
    assert_eq!(covered_directory.state, StoreSearchState::Empty);
}

#[test]
fn rebuild_indexes_rows_updates_coverage_and_searches_path_names() {
    let connection = open_connection();
    seed_source(&connection);
    let rebuild = rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");
    assert_eq!(rebuild.rows_indexed, 7);

    let coverage = connection
        .query_row(
            "SELECT generation, state FROM search_filter_index_source_coverage WHERE source_id = 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("coverage row");
    assert_eq!(coverage, (rebuild.generation, "ready".to_string()));

    let result = read_search_filter(&connection, request(Some("amen"))).expect("search");
    assert_eq!(result.state, StoreSearchState::Ready);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result.rows[0].result_kind,
        StoreSearchResultKind::SourceFile
    );
    assert_eq!(result.rows[0].display_label, "Amen.wav");
    assert!(result.rows[0].has_current_blake3);
    assert!(result.rows[0].has_current_probe);
    assert_eq!(
        result.rows[0].attachment_link_state,
        StoreSearchAttachmentLinkState::Current
    );
}

#[test]
fn scoped_search_and_file_class_filter_use_index_rows() {
    let connection = open_connection();
    seed_source(&connection);
    rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");

    let mut scoped = request(None);
    scoped.scope = StoreSearchScope::SourceLocation {
        source_location_id: 100,
    };
    scoped.filters.file_classes = vec![StoreSearchFileClass::Image];
    let result = read_search_filter(&connection, scoped).expect("search");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].display_label, "Cover.jpg");
}

#[test]
fn deterministic_pagination_and_cursor_identity_are_enforced() {
    let connection = open_connection();
    seed_source(&connection);
    let rebuild = rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");

    let mut first = request(None);
    first.limit = 2;
    let page1 = read_search_filter(&connection, first.clone()).expect("page1");
    assert_eq!(page1.rows.len(), 2);
    let cursor = page1.next_cursor.clone().expect("cursor");

    first.cursor = Some(cursor.clone());
    let page2 = read_search_filter(&connection, first.clone()).expect("page2");
    assert!(!page2.rows.is_empty());
    assert_ne!(page1.rows[0].stable_key, page2.rows[0].stable_key);

    let invalid_cases = [
        {
            let mut changed = first.clone();
            changed.scope = StoreSearchScope::Source { source_id: 1 };
            changed
        },
        {
            let mut changed = first.clone();
            changed.recursion = StoreSearchRecursion::Immediate;
            changed
        },
        {
            let mut changed = first.clone();
            changed.text_query = Some("amen".to_string());
            changed
        },
        {
            let mut changed = first.clone();
            changed.filters.file_classes = vec![StoreSearchFileClass::Audio];
            changed
        },
        {
            let mut changed = first.clone();
            changed.sort = StoreSearchSort::Relevance;
            changed.text_query = Some("amen".to_string());
            changed
        },
        {
            let mut changed = first.clone();
            changed.limit = 3;
            changed
        },
    ];

    for changed in invalid_cases {
        let invalid = read_search_filter(&connection, changed).expect("invalid cursor");
        assert_eq!(invalid.state, StoreSearchState::CursorInvalid);
    }

    rebuild_search_filter_index_for_source(&connection, 1, 20).expect("rebuild again");
    let mut stale_generation = request(None);
    stale_generation.limit = 2;
    stale_generation.cursor = Some(cursor);
    let invalid_generation =
        read_search_filter(&connection, stale_generation).expect("invalid generation");
    assert_eq!(invalid_generation.state, StoreSearchState::CursorInvalid);
    assert!(invalid_generation.index_generation > rebuild.generation);
}

#[test]
fn relevance_sort_orders_match_tiers_before_stable_ties() {
    let connection = open_connection();
    seed_source(&connection);
    connection
        .execute_batch(
            "INSERT INTO source_directories (
                 source_directory_id, source_id, parent_source_directory_id, name,
                 name_browse_sort_key, relative_path, presence_state, dir_scan_state,
                 dir_scan_updated_at, created_at, updated_at
             ) VALUES (12, 1, 10, 'Amen Path', 'amen path', 'Music/Amen Path', 'present', 'complete', 1, 1, 1);
             INSERT INTO source_files (
                 source_file_id, source_id, parent_source_directory_id, name,
                 name_browse_sort_key, relative_path_browse_sort_key, relative_path,
                 size_bytes, mtime_ns, file_kind, file_class, presence_state,
                 first_discovered_at, last_observed_at, last_presence_change_at,
                 created_at, updated_at
             ) VALUES
                 (1003, 1, 11, 'amen', 'amen exact', 'z/exact', 'Z/Exact/amen', 1, 1, 'audio', 'audio', 'present', 1, 1, 1, 1, 1),
                 (1004, 1, 11, 'Amen Intro.wav', 'amen intro', 'a/prefix', 'A/Prefix/Amen Intro.wav', 1, 1, 'audio', 'audio', 'present', 1, 1, 1, 1, 1),
                 (1005, 1, 11, 'Great Amen.wav', 'great amen', 'b/contains', 'B/Contains/Great Amen.wav', 1, 1, 'audio', 'audio', 'present', 1, 1, 1, 1, 1),
                 (1006, 1, 12, 'Other.wav', 'other', 'c/path', 'Music/Amen Path/Other.wav', 1, 1, 'audio', 'audio', 'present', 1, 1, 1, 1, 1);",
        )
        .expect("seed relevance rows");
    rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");

    let mut search = request(Some("amen"));
    search.scope = StoreSearchScope::Source { source_id: 1 };
    search.target_kinds = vec![StoreSearchResultKind::SourceFile];
    search.sort = StoreSearchSort::Relevance;
    let result = read_search_filter(&connection, search).expect("search");
    let labels = result
        .rows
        .iter()
        .map(|row| (row.display_label.as_str(), row.match_reason))
        .collect::<Vec<_>>();

    assert_eq!(
        labels[..4],
        [
            ("amen", StoreSearchMatchReason::ExactLabel),
            ("Amen Intro.wav", StoreSearchMatchReason::LabelPrefix),
            ("Amen.wav", StoreSearchMatchReason::LabelPrefix),
            ("Great Amen.wav", StoreSearchMatchReason::Label),
        ]
    );
    assert!(
        labels
            .iter()
            .any(|(label, reason)| *label == "Other.wav" && *reason == StoreSearchMatchReason::Path)
    );
}
