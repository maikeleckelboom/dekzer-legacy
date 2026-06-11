use crate::read_models::search_filter::{
    RebuildSearchFilterIndexForSourceResult, StoreSearchRequest, StoreSearchResult,
    read_search_filter, rebuild_search_filter_index_for_source,
};
use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn rebuild_search_filter_index_for_source(
        &self,
        source_id: i64,
    ) -> LibrarySqliteResult<RebuildSearchFilterIndexForSourceResult> {
        let rebuilt_at_ms = unix_time_ms()?;
        self.with_write(|write| {
            rebuild_search_filter_index_for_source(write, source_id, rebuilt_at_ms)
        })
    }

    pub fn read_search_filter(
        &self,
        request: StoreSearchRequest,
    ) -> LibrarySqliteResult<StoreSearchResult> {
        let mut connection = self.open_read_connection()?;
        let transaction = connection.transaction()?;
        let result = read_search_filter(&transaction, request)?;
        transaction.commit().map_err(LibrarySqliteError::from)?;
        Ok(result)
    }
}
