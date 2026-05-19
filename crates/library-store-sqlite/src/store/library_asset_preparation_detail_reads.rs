use crate::LibrarySqliteResult;
use crate::read_models::library_asset_preparation_detail::{
    StoreLibraryAssetPreparationDetail,
    read_library_asset_preparation_detail as read_library_asset_preparation_detail_query,
};

use super::{SqliteDurableStore, bootstrap::open_connection};

impl SqliteDurableStore {
    pub fn read_library_asset_preparation_detail(
        &self,
        library_asset_id: i64,
    ) -> LibrarySqliteResult<Option<StoreLibraryAssetPreparationDetail>> {
        let connection = open_connection(&self.path)?;
        read_library_asset_preparation_detail_query(&connection, library_asset_id)
    }
}
