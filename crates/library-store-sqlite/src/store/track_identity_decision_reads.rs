use crate::LibrarySqliteResult;
use crate::read_models::track_identity_decisions::{
    StoreTrackIdentityDecision, StoreTrackIdentityEffectiveDecisionSummary,
    read_effective_track_identity_decision_for_candidate,
    read_track_identity_decisions_for_candidate, read_track_identity_decisions_for_source,
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

    pub fn read_track_identity_decisions_for_candidate(
        &self,
        track_identity_candidate_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<Vec<StoreTrackIdentityDecision>> {
        let connection = self.open_read_connection()?;
        read_track_identity_decisions_for_candidate(&connection, track_identity_candidate_id, limit)
    }

    pub fn read_effective_track_identity_decision_for_candidate(
        &self,
        track_identity_candidate_id: i64,
    ) -> LibrarySqliteResult<StoreTrackIdentityEffectiveDecisionSummary> {
        let connection = self.open_read_connection()?;
        read_effective_track_identity_decision_for_candidate(
            &connection,
            track_identity_candidate_id,
        )
    }
}
