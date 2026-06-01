use crate::LibrarySqliteResult;
use crate::read_models::attachment_identity::{
    StoreAttachmentSourceFiles, StoreSourceAttachmentSummary, StoreSourceFileAttachmentLink,
    get_attachment_for_source_file, get_source_attachment_summary,
    get_source_files_for_attachment_limited,
};
use crate::store::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_attachment_for_source_file(
        &self,
        source_file_id: i64,
    ) -> LibrarySqliteResult<Option<StoreSourceFileAttachmentLink>> {
        let connection = self.open_read_connection()?;
        get_attachment_for_source_file(&connection, source_file_id)
    }

    pub fn read_source_files_for_attachment(
        &self,
        attachment_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<Option<StoreAttachmentSourceFiles>> {
        let connection = self.open_read_connection()?;
        get_source_files_for_attachment_limited(&connection, attachment_id, limit)
    }

    pub fn read_source_attachment_summary(
        &self,
        source_id: i64,
    ) -> LibrarySqliteResult<Option<StoreSourceAttachmentSummary>> {
        let connection = self.open_read_connection()?;
        get_source_attachment_summary(&connection, source_id)
    }
}
