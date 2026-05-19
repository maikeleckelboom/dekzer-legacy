use crate::read_models::library_browser::{
    StoreLibraryBrowserWindow, read_window as read_library_browser_window_query,
    read_window_for_scope as read_library_browser_window_for_scope_query,
    read_window_for_source as read_library_browser_window_for_source_query,
    search_window as search_library_browser_window_query,
    search_window_for_scope as search_library_browser_window_for_scope_query,
};
use crate::read_models::navigation::NavigationRow;
use crate::read_models::navigation::load_row as load_navigation_row_query;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    LibraryBrowseScope, NavigationSelector, NavigationSelectorDecodeError,
    compile_library_browse_scope, decode_selector,
};

use super::{SqliteDurableStore, bootstrap::open_connection};

impl SqliteDurableStore {
    pub fn read_library_browser_window(
        &self,
        offset: usize,
        limit: usize,
    ) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
        let connection = open_connection(&self.path)?;
        read_library_browser_window_query(&connection, offset, limit)
    }

    pub fn search_library_browser_window(
        &self,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
        let connection = open_connection(&self.path)?;
        search_library_browser_window_query(&connection, query, offset, limit)
    }

    pub fn read_library_browser_window_for_source(
        &self,
        source_id: i64,
        offset: usize,
        limit: usize,
    ) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
        let connection = open_connection(&self.path)?;
        read_library_browser_window_for_source_query(&connection, source_id, offset, limit)
    }

    pub fn read_location_root_library_browser_window(
        &self,
        navigation_row_id: i64,
        offset: usize,
        limit: usize,
    ) -> LibrarySqliteResult<Option<StoreLibraryBrowserWindow>> {
        self.read_navigation_node_library_browser_window(navigation_row_id, offset, limit)
    }

    pub fn read_navigation_node_library_browser_window(
        &self,
        navigation_row_id: i64,
        offset: usize,
        limit: usize,
    ) -> LibrarySqliteResult<Option<StoreLibraryBrowserWindow>> {
        let connection = open_connection(&self.path)?;
        let Some(navigation_row) = load_navigation_row_query(&connection, navigation_row_id)?
        else {
            return Ok(None);
        };
        let Some(scope) = library_browse_scope_for_navigation_row(&navigation_row)? else {
            return Ok(None);
        };
        read_library_browser_window_for_scope_query(&connection, scope, offset, limit).map(Some)
    }

    pub fn search_navigation_node_library_browser_window(
        &self,
        navigation_row_id: i64,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> LibrarySqliteResult<Option<StoreLibraryBrowserWindow>> {
        let connection = open_connection(&self.path)?;
        let Some(navigation_row) = load_navigation_row_query(&connection, navigation_row_id)?
        else {
            return Ok(None);
        };
        let Some(scope) = library_browse_scope_for_navigation_row(&navigation_row)? else {
            return Ok(None);
        };
        search_library_browser_window_for_scope_query(&connection, scope, query, offset, limit)
            .map(Some)
    }
}

fn library_browse_scope_for_navigation_row(
    row: &NavigationRow,
) -> LibrarySqliteResult<Option<LibraryBrowseScope>> {
    if row.selector_kind.is_none() && row.selector_payload.is_none() {
        return Ok(None);
    }

    let selector = decode_required_navigation_selector(
        row.navigation_row_id,
        row.selector_kind.as_deref(),
        row.selector_payload.as_deref(),
    )?;
    compile_library_browse_scope(selector).map(Some).ok_or_else(|| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "navigation row {} has selector {selector:?}; expected a library browser scope selector",
            row.navigation_row_id
        ))
    })
}

fn decode_required_navigation_selector(
    navigation_row_id: i64,
    selector_kind: Option<&str>,
    selector_payload: Option<&str>,
) -> LibrarySqliteResult<NavigationSelector> {
    match (selector_kind, selector_payload) {
        (Some(kind), Some(payload)) => decode_selector(kind, payload)
            .map_err(|error| invalid_navigation_selector(navigation_row_id, error)),
        (kind, payload) => Err(LibrarySqliteError::MalformedSchemaState(format!(
            "navigation row {navigation_row_id} has incomplete selector pair: selector_kind={kind:?}, selector_payload={payload:?}"
        ))),
    }
}

fn invalid_navigation_selector(
    navigation_row_id: i64,
    error: NavigationSelectorDecodeError,
) -> LibrarySqliteError {
    LibrarySqliteError::MalformedSchemaState(format!(
        "navigation row {navigation_row_id} has invalid selector: {error}"
    ))
}

#[cfg(test)]
mod tests {
    use super::library_browse_scope_for_navigation_row;
    use crate::LibrarySqliteError;
    use crate::read_models::navigation::NavigationRow;
    use library_domain::{
        LibraryBrowseScope, PlaylistId, PrepPolicyId, SourceId, SourceLocationId,
    };

    fn navigation_row(
        row_kind: &str,
        selector_kind: Option<&str>,
        selector_payload: Option<&str>,
    ) -> NavigationRow {
        NavigationRow {
            navigation_row_id: 9,
            stable_key: "source:9".to_string(),
            parent_navigation_row_id: None,
            family: Some("Sources".to_string()),
            row_kind: row_kind.to_string(),
            display_name: "Library".to_string(),
            sibling_position: 0,
            selectable: true,
            selector_kind: selector_kind.map(str::to_string),
            selector_payload: selector_payload.map(str::to_string),
            updated_at: 100,
            row_version: 1,
        }
    }

    fn assert_malformed_schema_state_contains(error: LibrarySqliteError, expected: &str) {
        match error {
            LibrarySqliteError::MalformedSchemaState(detail) => {
                assert!(
                    detail.contains(expected),
                    "expected {detail:?} to contain {expected:?}"
                );
            }
            other => panic!("expected MalformedSchemaState, got {other:?}"),
        }
    }

    #[test]
    fn navigation_node_browser_boundary_decodes_source_selector() {
        let scope = library_browse_scope_for_navigation_row(&navigation_row(
            "source",
            Some("source"),
            Some("9"),
        ))
        .expect("selector decodes");

        assert_eq!(
            scope,
            Some(LibraryBrowseScope::Source(
                SourceId::new(9).expect("positive id")
            ))
        );
    }

    #[test]
    fn navigation_node_browser_boundary_decodes_source_location_selector() {
        let scope = library_browse_scope_for_navigation_row(&navigation_row(
            "location",
            Some("source_location"),
            Some("11"),
        ))
        .expect("selector decodes");

        assert_eq!(
            scope,
            Some(LibraryBrowseScope::SourceLocation(
                SourceLocationId::new(11).expect("positive id")
            ))
        );
    }

    #[test]
    fn navigation_node_browser_boundary_decodes_playlist_selector() {
        let scope = library_browse_scope_for_navigation_row(&navigation_row(
            "playlist",
            Some("playlist"),
            Some("12"),
        ))
        .expect("selector decodes");

        assert_eq!(
            scope,
            Some(LibraryBrowseScope::Playlist(
                PlaylistId::new(12).expect("positive id")
            ))
        );
    }

    #[test]
    fn navigation_node_browser_boundary_decodes_prep_policy_scope_selector() {
        let scope = library_browse_scope_for_navigation_row(&navigation_row(
            "prep-policy-scope",
            Some("prep_policy_scope"),
            Some("13"),
        ))
        .expect("selector decodes");

        assert_eq!(
            scope,
            Some(LibraryBrowseScope::PrepPolicyScope(
                PrepPolicyId::new(13).expect("positive id")
            ))
        );
    }

    #[test]
    fn navigation_node_browser_boundary_decodes_all_media_selector() {
        let scope = library_browse_scope_for_navigation_row(&navigation_row(
            "view",
            Some("all_media"),
            Some(""),
        ))
        .expect("selector decodes");

        assert_eq!(scope, Some(LibraryBrowseScope::AllMedia));
    }

    #[test]
    fn navigation_node_browser_boundary_decodes_needs_preparation_selector() {
        let scope = library_browse_scope_for_navigation_row(&navigation_row(
            "view",
            Some("needs_preparation"),
            Some(""),
        ))
        .expect("selector decodes");

        assert_eq!(scope, Some(LibraryBrowseScope::NeedsPreparation));
    }

    #[test]
    fn navigation_node_browser_boundary_rejects_unknown_selector_kind() {
        let error = library_browse_scope_for_navigation_row(&navigation_row(
            "source",
            Some("unknown"),
            Some("9"),
        ))
        .expect_err("unknown selector kind is malformed");

        assert_malformed_schema_state_contains(error, "unknown navigation selector kind");
    }

    #[test]
    fn navigation_node_browser_boundary_rejects_malformed_selector_payload() {
        let error = library_browse_scope_for_navigation_row(&navigation_row(
            "source",
            Some("source"),
            Some("not-an-id"),
        ))
        .expect_err("invalid selector payload is malformed");

        assert_malformed_schema_state_contains(error, "invalid navigation selector payload");
    }

    #[test]
    fn navigation_node_browser_boundary_ignores_structural_rows() {
        let scope =
            library_browse_scope_for_navigation_row(&navigation_row("location-group", None, None))
                .expect("structural rows do not compile to a browser scope");

        assert_eq!(scope, None);
    }
}
