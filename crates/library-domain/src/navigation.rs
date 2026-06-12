use std::fmt;

use crate::browser::LibraryBrowseScope;
use crate::ids::{SourceId, SourceLocationId};

const ALL_MEDIA_KIND: &str = "all_media";
const ALL_AUDIO_KIND: &str = "all_audio";
const ALL_VIDEOS_KIND: &str = "all_videos";
const RECENTLY_ADDED_KIND: &str = "recently_added";
const SOURCE_KIND: &str = "source";
const SOURCE_LOCATION_KIND: &str = "source_location";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigationSelector {
    AllMedia,
    AllAudio,
    AllVideos,
    RecentlyAdded,
    Source(SourceId),
    SourceLocation(SourceLocationId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedNavigationSelector {
    pub kind: &'static str,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationSelectorDecodeError {
    UnknownKind(String),
    InvalidPayload { kind: String, payload: String },
}

impl fmt::Display for NavigationSelectorDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKind(kind) => {
                write!(formatter, "unknown navigation selector kind {kind:?}")
            }
            Self::InvalidPayload { kind, payload } => write!(
                formatter,
                "invalid navigation selector payload {payload:?} for kind {kind:?}"
            ),
        }
    }
}

impl std::error::Error for NavigationSelectorDecodeError {}

pub fn encode_selector(selector: &NavigationSelector) -> EncodedNavigationSelector {
    match selector {
        NavigationSelector::AllMedia => EncodedNavigationSelector {
            kind: ALL_MEDIA_KIND,
            payload: String::new(),
        },
        NavigationSelector::AllAudio => EncodedNavigationSelector {
            kind: ALL_AUDIO_KIND,
            payload: String::new(),
        },
        NavigationSelector::AllVideos => EncodedNavigationSelector {
            kind: ALL_VIDEOS_KIND,
            payload: String::new(),
        },
        NavigationSelector::RecentlyAdded => EncodedNavigationSelector {
            kind: RECENTLY_ADDED_KIND,
            payload: String::new(),
        },
        NavigationSelector::Source(id) => EncodedNavigationSelector {
            kind: SOURCE_KIND,
            payload: id.get().to_string(),
        },
        NavigationSelector::SourceLocation(id) => EncodedNavigationSelector {
            kind: SOURCE_LOCATION_KIND,
            payload: id.get().to_string(),
        },
    }
}

pub fn decode_selector(
    kind: &str,
    payload: &str,
) -> Result<NavigationSelector, NavigationSelectorDecodeError> {
    match kind {
        ALL_MEDIA_KIND => {
            if payload.is_empty() {
                Ok(NavigationSelector::AllMedia)
            } else {
                Err(invalid_payload(kind, payload))
            }
        }
        ALL_AUDIO_KIND => decode_empty_payload(kind, payload, NavigationSelector::AllAudio),
        ALL_VIDEOS_KIND => decode_empty_payload(kind, payload, NavigationSelector::AllVideos),
        RECENTLY_ADDED_KIND => {
            decode_empty_payload(kind, payload, NavigationSelector::RecentlyAdded)
        }
        SOURCE_KIND => parse_id(kind, payload)
            .map(SourceId::new)
            .and_then(|id| id)
            .map(NavigationSelector::Source)
            .ok_or_else(|| invalid_payload(kind, payload)),
        SOURCE_LOCATION_KIND => parse_id(kind, payload)
            .map(SourceLocationId::new)
            .and_then(|id| id)
            .map(NavigationSelector::SourceLocation)
            .ok_or_else(|| invalid_payload(kind, payload)),
        _ => Err(NavigationSelectorDecodeError::UnknownKind(kind.to_string())),
    }
}

pub const fn compile_library_browse_scope(
    selector: NavigationSelector,
) -> Option<LibraryBrowseScope> {
    match selector {
        NavigationSelector::AllMedia => Some(LibraryBrowseScope::AllMedia),
        NavigationSelector::AllAudio => Some(LibraryBrowseScope::AllAudio),
        NavigationSelector::AllVideos => Some(LibraryBrowseScope::AllVideos),
        NavigationSelector::RecentlyAdded => Some(LibraryBrowseScope::RecentlyAdded),
        NavigationSelector::Source(id) => Some(LibraryBrowseScope::Source(id)),
        NavigationSelector::SourceLocation(id) => Some(LibraryBrowseScope::SourceLocation(id)),
    }
}

fn decode_empty_payload(
    kind: &str,
    payload: &str,
    selector: NavigationSelector,
) -> Result<NavigationSelector, NavigationSelectorDecodeError> {
    if payload.is_empty() {
        Ok(selector)
    } else {
        Err(invalid_payload(kind, payload))
    }
}

fn parse_id(_kind: &str, payload: &str) -> Option<i64> {
    payload.parse::<i64>().ok()
}

fn invalid_payload(kind: &str, payload: &str) -> NavigationSelectorDecodeError {
    NavigationSelectorDecodeError::InvalidPayload {
        kind: kind.to_string(),
        payload: payload.to_string(),
    }
}
