use crate::LibrarySqliteResult;
use crate::read_models::observed_file_facts::{
    StoreObservedFileFacts, read_observed_file_facts_for_source_file,
};
use crate::store::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_observed_file_facts_for_source_file(
        &self,
        source_file_id: i64,
    ) -> LibrarySqliteResult<Option<StoreObservedFileFacts>> {
        let connection = self.open_read_connection()?;
        read_observed_file_facts_for_source_file(&connection, source_file_id)
    }
}
