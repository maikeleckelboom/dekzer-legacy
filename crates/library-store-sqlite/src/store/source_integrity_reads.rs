use crate::read_models::source_integrity::{
    StoreSourceIntegrity, read_source_integrity as read_source_integrity_query,
};
use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_source_integrity(
        &self,
        source_id: i64,
    ) -> LibrarySqliteResult<StoreSourceIntegrity> {
        let mut connection = self.open_read_connection()?;
        let transaction = connection.transaction()?;
        let result = read_source_integrity_query(&transaction, source_id)?;
        transaction.commit().map_err(LibrarySqliteError::from)?;
        Ok(result)
    }
}
