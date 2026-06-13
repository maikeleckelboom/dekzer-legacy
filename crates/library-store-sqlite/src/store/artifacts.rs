use crate::LibrarySqliteResult;
use crate::authority::work::{
    ArtifactFileStoreReconciliationResult, RecordFileStoreArtifactInput, RecordInlineArtifactInput,
    RecordedArtifact, WorkArtifactAuthorityTx, reconcile_artifact_file_store,
};

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn record_inline_artifact(
        &self,
        input: RecordInlineArtifactInput,
    ) -> LibrarySqliteResult<RecordedArtifact> {
        self.with_write(|write| WorkArtifactAuthorityTx::new(write).record_inline_artifact(&input))
    }

    pub fn record_file_store_artifact(
        &self,
        input: RecordFileStoreArtifactInput,
    ) -> LibrarySqliteResult<RecordedArtifact> {
        self.with_write(|write| {
            WorkArtifactAuthorityTx::new(write).record_file_store_artifact(&input)
        })
    }

    #[allow(dead_code)]
    pub(crate) fn reconcile_artifact_file_store(
        &self,
        limit: usize,
    ) -> LibrarySqliteResult<ArtifactFileStoreReconciliationResult> {
        if limit == 0 {
            return Ok(ArtifactFileStoreReconciliationResult::default());
        }

        let file_store_root = self.app_owned_state.artifact_file_store_root().clone();
        self.with_write(|write| reconcile_artifact_file_store(write, &file_store_root, limit))
    }
}
