use crate::LibrarySqliteResult;
use crate::authority::promotion::{
    InspectSourceFilePromotionInput, InspectSourceFilePromotionResult, InspectSourceFilePromotionTx,
};

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn inspect_source_file(
        &self,
        input: InspectSourceFilePromotionInput,
    ) -> LibrarySqliteResult<InspectSourceFilePromotionResult> {
        let file_store_root = self.app_owned_state.artifact_file_store_root().clone();
        self.with_write(|write| {
            let result = InspectSourceFilePromotionTx::new(write, file_store_root)
                .inspect_source_file(&input)?;
            Ok(result)
        })
    }
}
