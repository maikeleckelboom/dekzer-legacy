use crate::{LibraryAssetId, SourceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepAssignmentScopeKind {
    Library,
    Source,
    LibraryAsset,
}

impl PrepAssignmentScopeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Library => "library",
            Self::Source => "source",
            Self::LibraryAsset => "library_asset",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "library" => Some(Self::Library),
            "source" => Some(Self::Source),
            "library_asset" => Some(Self::LibraryAsset),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrepScope {
    Library,
    Source(SourceId),
    LibraryAsset(LibraryAssetId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepTargetStabilityClass {
    Provisional,
    Stable,
}

impl PrepTargetStabilityClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provisional => "provisional",
            Self::Stable => "stable",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "provisional" => Some(Self::Provisional),
            "stable" => Some(Self::Stable),
            _ => None,
        }
    }
}
