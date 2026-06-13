use crate::ProjectionDomain;
use crate::ids::SourceFileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkPriorityClass {
    Urgent,
    Interactive,
    Background,
}

impl WorkPriorityClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Urgent => "urgent",
            Self::Interactive => "interactive",
            Self::Background => "background",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "urgent" => Some(Self::Urgent),
            "interactive" => Some(Self::Interactive),
            "background" => Some(Self::Background),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkSubjectKind {
    SourceFile,
    ProjectionDomain,
}

impl WorkSubjectKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceFile => "source_file",
            Self::ProjectionDomain => "projection_domain",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "source_file" => Some(Self::SourceFile),
            "projection_domain" => Some(Self::ProjectionDomain),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkSubject {
    SourceFile(SourceFileId),
    ProjectionDomain(ProjectionDomain),
}

impl WorkSubject {
    pub const fn kind(self) -> WorkSubjectKind {
        match self {
            Self::SourceFile(_) => WorkSubjectKind::SourceFile,
            Self::ProjectionDomain(_) => WorkSubjectKind::ProjectionDomain,
        }
    }

    pub fn parse(kind: WorkSubjectKind, id: &str) -> Option<Self> {
        match kind {
            WorkSubjectKind::SourceFile => id
                .parse::<i64>()
                .ok()
                .and_then(SourceFileId::new)
                .map(Self::SourceFile),
            WorkSubjectKind::ProjectionDomain => {
                ProjectionDomain::parse(id).map(Self::ProjectionDomain)
            }
        }
    }

    pub fn storage_id(self) -> String {
        match self {
            Self::SourceFile(id) => id.get().to_string(),
            Self::ProjectionDomain(domain) => domain.as_str().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineWorkKind {
    InspectSource,
    RebuildProjection,
}

impl MachineWorkKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InspectSource => "inspect_source",
            Self::RebuildProjection => "rebuild_projection",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "inspect_source" => Some(Self::InspectSource),
            "rebuild_projection" => Some(Self::RebuildProjection),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkItemState {
    Queued,
    Leased,
    Completed,
    Blocked,
    Failed,
    Canceled,
}

impl WorkItemState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Leased => "leased",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "leased" => Some(Self::Leased),
            "completed" => Some(Self::Completed),
            "blocked" => Some(Self::Blocked),
            "failed" => Some(Self::Failed),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkRunOutcome {
    Running,
    Completed,
    Blocked,
    Failed,
    Canceled,
}

impl WorkRunOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "blocked" => Some(Self::Blocked),
            "failed" => Some(Self::Failed),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}
