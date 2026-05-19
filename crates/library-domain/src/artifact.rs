#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    InspectionResult,
    SegmentationResult,
    CapabilityResult,
    ProjectionSnapshot,
    DiagnosticResult,
}

impl ArtifactKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InspectionResult => "inspection_result",
            Self::SegmentationResult => "segmentation_result",
            Self::CapabilityResult => "capability_result",
            Self::ProjectionSnapshot => "projection_snapshot",
            Self::DiagnosticResult => "diagnostic_result",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "inspection_result" => Some(Self::InspectionResult),
            "segmentation_result" => Some(Self::SegmentationResult),
            "capability_result" => Some(Self::CapabilityResult),
            "projection_snapshot" => Some(Self::ProjectionSnapshot),
            "diagnostic_result" => Some(Self::DiagnosticResult),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactRole {
    PrimaryResult,
    PreviewSummary,
    Manifest,
    DiagnosticPayload,
    IntermediateOutput,
}

impl ArtifactRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrimaryResult => "primary_result",
            Self::PreviewSummary => "preview_summary",
            Self::Manifest => "manifest",
            Self::DiagnosticPayload => "diagnostic_payload",
            Self::IntermediateOutput => "intermediate_output",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "primary_result" => Some(Self::PrimaryResult),
            "preview_summary" => Some(Self::PreviewSummary),
            "manifest" => Some(Self::Manifest),
            "diagnostic_payload" => Some(Self::DiagnosticPayload),
            "intermediate_output" => Some(Self::IntermediateOutput),
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
