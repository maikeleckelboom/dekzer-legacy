use crate::ids::{PlaylistId, PrepPolicyId, SourceId, SourceLocationId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryBrowseScope {
    AllMedia,
    AllAudio,
    AllVideos,
    RecentlyAdded,
    NeedsPreparation,
    PlaylistGroup,
    Source(SourceId),
    SourceLocation(SourceLocationId),
    Playlist(PlaylistId),
    PrepPolicyScope(PrepPolicyId),
}
