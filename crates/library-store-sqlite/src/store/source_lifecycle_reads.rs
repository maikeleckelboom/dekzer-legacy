use crate::LibrarySqliteResult;
use crate::read_models::source_lifecycle::{
    StoreSourceLifecycle, read_source_lifecycle as read_source_lifecycle_query,
};

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_source_lifecycle(
        &self,
        source_id: i64,
    ) -> LibrarySqliteResult<Option<StoreSourceLifecycle>> {
        let connection = self.open_read_connection()?;
        read_source_lifecycle_query(&connection, source_id)
    }
}
