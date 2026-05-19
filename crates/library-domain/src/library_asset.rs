#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryAssetRetentionPolicy {
    KeepMetadata,
    Purge,
}

impl LibraryAssetRetentionPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeepMetadata => "keep_metadata",
            Self::Purge => "purge",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "keep_metadata" => Some(Self::KeepMetadata),
            "purge" => Some(Self::Purge),
            _ => None,
        }
    }
}
