use crate::read_models::contents::{
    StoreContentsReadPolicy, StoreContentsResult, StoreContentsScope, StoreContentsScopeDepth,
    read_contents,
};
use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_contents(
        &self,
        scope: StoreContentsScope,
        policy: StoreContentsReadPolicy,
        recursion: StoreContentsScopeDepth,
        limit: usize,
        cursor: Option<&str>,
    ) -> LibrarySqliteResult<StoreContentsResult> {
        let mut connection = self.open_read_connection()?;
        let transaction = connection.transaction()?;
        let result = read_contents(&transaction, scope, policy, recursion, limit, cursor)?;
        transaction.commit().map_err(LibrarySqliteError::from)?;
        Ok(result)
    }
}
