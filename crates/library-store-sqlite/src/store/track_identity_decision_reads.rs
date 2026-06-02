use crate::LibrarySqliteResult;
use crate::read_models::track_identity_decisions::{
    StoreTrackIdentityDecision, read_track_identity_decisions_for_source,
};
use crate::store::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_track_identity_decisions_for_source(
        &self,
        source_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<Vec<StoreTrackIdentityDecision>> {
        let connection = self.open_read_connection()?;
        read_track_identity_decisions_for_source(&connection, source_id, limit)
    }
}
