use crate::LibrarySqliteResult;
use crate::read_models::literal_hierarchy::{
    StoreLiteralHierarchyEntryPoint, StoreLiteralHierarchyWindow,
    read_children as read_literal_hierarchy_children_query,
};

use super::{SqliteDurableStore, bootstrap::open_connection};

impl SqliteDurableStore {
    pub fn read_literal_hierarchy_children(
        &self,
        entry_point: StoreLiteralHierarchyEntryPoint,
        parent_source_directory_id: Option<i64>,
        offset: usize,
        limit: usize,
    ) -> LibrarySqliteResult<Option<StoreLiteralHierarchyWindow>> {
        let connection = open_connection(&self.path)?;
        read_literal_hierarchy_children_query(
            &connection,
            entry_point,
            parent_source_directory_id,
            offset,
            limit,
        )
    }
}
