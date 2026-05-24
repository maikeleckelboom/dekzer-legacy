use crate::read_models::selected_contents::{
    StoreSelectedContentsResult, StoreSelectedContentsScope, read_selected_contents,
};
use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_selected_contents(
        &self,
        scope: StoreSelectedContentsScope,
        limit: usize,
        cursor: Option<&str>,
    ) -> LibrarySqliteResult<StoreSelectedContentsResult> {
        let mut connection = self.open_read_connection()?;
        let transaction = connection.transaction()?;
        let result = read_selected_contents(&transaction, scope, limit, cursor)?;
        transaction.commit().map_err(LibrarySqliteError::from)?;
        Ok(result)
    }
}
