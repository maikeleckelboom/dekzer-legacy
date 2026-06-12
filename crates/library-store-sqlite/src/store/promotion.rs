use crate::LibrarySqliteResult;
use crate::authority::promotion::{
    InspectSourcePromotionInput, InspectSourcePromotionResult, InspectSourcePromotionTx,
};
use crate::publication;
use library_domain::ProjectionDomain;

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn inspect_source(
        &self,
        input: InspectSourcePromotionInput,
    ) -> LibrarySqliteResult<InspectSourcePromotionResult> {
        let file_store_root = self.app_owned_state.artifact_file_store_root().clone();
        self.with_write(|write| {
            let result =
                InspectSourcePromotionTx::new(write, file_store_root).inspect_source(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(result)
        })
    }
}
