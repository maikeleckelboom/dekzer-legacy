use crate::ids::{SourceId, SourceLocationId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryBrowseScope {
    AllMedia,
    AllAudio,
    AllVideos,
    RecentlyAdded,
    Source(SourceId),
    SourceLocation(SourceLocationId),
}
