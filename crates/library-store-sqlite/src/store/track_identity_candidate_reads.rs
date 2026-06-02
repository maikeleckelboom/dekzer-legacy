use crate::LibrarySqliteResult;
use crate::read_models::track_identity_candidates::{
    StoreTrackIdentityCandidate, read_track_identity_candidates_for_source,
};
use crate::store::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_track_identity_candidates_for_source(
        &self,
        source_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<Vec<StoreTrackIdentityCandidate>> {
        let connection = self.open_read_connection()?;
        read_track_identity_candidates_for_source(&connection, source_id, limit)
    }
}
