use crate::LibrarySqliteResult;
use crate::read_models::source_file_observations::{
    StoreSourceFileObservation, read_source_file_observation,
};
use crate::store::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_source_file_observation(
        &self,
        source_file_id: i64,
    ) -> LibrarySqliteResult<Option<StoreSourceFileObservation>> {
        let connection = self.open_read_connection()?;
        read_source_file_observation(&connection, source_file_id)
    }
}
