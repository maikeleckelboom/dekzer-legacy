#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    InspectionResult,
    ProjectionSnapshot,
}

impl ArtifactKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InspectionResult => "inspection_result",
            Self::ProjectionSnapshot => "projection_snapshot",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "inspection_result" => Some(Self::InspectionResult),
            "projection_snapshot" => Some(Self::ProjectionSnapshot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactStorageKind {
    InlinePayload,
    FileStore,
}

impl ArtifactStorageKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InlinePayload => "inline_payload",
            Self::FileStore => "file_store",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "inline_payload" => Some(Self::InlinePayload),
            "file_store" => Some(Self::FileStore),
            _ => None,
        }
    }
}
