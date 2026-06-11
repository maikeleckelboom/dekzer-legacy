mod coverage;
mod cursor;
mod filters;
mod fts;
mod read;
mod rebuild;
mod row;
mod scope;
mod types;

#[cfg(test)]
mod tests;

const SEARCH_CURSOR_VERSION: u8 = 1;
const SEARCH_INDEXER_VERSION: &str = "search_filter_v0";

pub(crate) use read::read_search_filter;
pub(crate) use rebuild::rebuild_search_filter_index_for_source;
pub use types::{
    RebuildSearchFilterIndexForSourceResult, StoreSearchAccessState,
    StoreSearchAttachmentLinkState, StoreSearchAuthorityLayer, StoreSearchEvidenceAvailability,
    StoreSearchEvidenceCoverageState, StoreSearchFileClass, StoreSearchFileKind,
    StoreSearchFilters, StoreSearchIndexState, StoreSearchMatchReason, StoreSearchMediaRelevance,
    StoreSearchPresenceState, StoreSearchQueryIdentity, StoreSearchRecursion, StoreSearchRequest,
    StoreSearchResult, StoreSearchResultKind, StoreSearchResultRow, StoreSearchScope,
    StoreSearchSort, StoreSearchState,
};
