use crate::LibrarySqliteResult;
use crate::authority::promotion::{
    RebuildProjectionPromotionInput, RebuildProjectionPromotionResult, RebuildProjectionPromotionTx,
};
use crate::publication;
use crate::time::unix_time_ms;
use library_domain::ProjectionDomain;

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn rebuild_projection(
        &self,
        input: RebuildProjectionPromotionInput,
    ) -> LibrarySqliteResult<RebuildProjectionPromotionResult> {
        self.with_write(|write| RebuildProjectionPromotionTx::new(write).rebuild_projection(&input))
    }

    #[allow(dead_code)]
    fn advance_substrate_maintenance(&self) -> LibrarySqliteResult<()> {
        let now_ms = unix_time_ms()?;
        self.with_write(|write| {
            publication::sweep_projection_retention(write, now_ms)?;
            Ok(())
        })
    }

    pub(super) fn sync_root_projection_state(&self, root_ids: &[i64]) -> LibrarySqliteResult<()> {
        if root_ids.is_empty() {
            return Ok(());
        }

        self.reseed_navigation_and_library_browser_projections()
    }

    #[allow(dead_code)]
    pub(super) fn sync_root_and_source_projection_state(
        &self,
        _root_id: i64,
        file_ids: &[i64],
    ) -> LibrarySqliteResult<usize> {
        if file_ids.is_empty() {
            return Ok(0);
        }
        self.reseed_navigation_and_library_browser_projections()?;
        Ok(0)
    }

    pub(super) fn reseed_navigation_and_library_browser_projections(
        &self,
    ) -> LibrarySqliteResult<()> {
        self.with_write(|write| {
            publication::reseed_projection_domains(
                write,
                &[
                    ProjectionDomain::Navigation,
                    ProjectionDomain::LibraryBrowser,
                ],
            )
        })
    }

    pub fn reseed_current_projection_state(&self) -> LibrarySqliteResult<()> {
        self.with_write(|write| publication::reseed_current_projection_state(write))
    }
}
