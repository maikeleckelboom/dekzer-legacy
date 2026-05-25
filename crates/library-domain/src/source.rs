#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAvailabilityState {
    Available,
    Unavailable,
    Degraded,
}

impl SourceAvailabilityState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::Degraded => "degraded",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "available" => Some(Self::Available),
            "unavailable" => Some(Self::Unavailable),
            "degraded" => Some(Self::Degraded),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAccessState {
    Accessible,
    Missing,
    Blocked,
    Unknown,
}

impl SourceAccessState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accessible => "accessible",
            Self::Missing => "missing",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "accessible" => Some(Self::Accessible),
            "missing" => Some(Self::Missing),
            "blocked" => Some(Self::Blocked),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAccessIssueKind {
    Missing,
    NotDirectory,
    PermissionDenied,
    PrivacyPermissionRequired,
    UnavailableMount,
    ResourceBusy,
    StaleNetworkHandle,
    SymlinkLoop,
    SymlinkEscapeBlocked,
    UnsupportedPath,
    InvalidPath,
    IoInterrupted,
    TimedOut,
    UnknownIo,
}

impl SourceAccessIssueKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::NotDirectory => "not_directory",
            Self::PermissionDenied => "permission_denied",
            Self::PrivacyPermissionRequired => "privacy_permission_required",
            Self::UnavailableMount => "unavailable_mount",
            Self::ResourceBusy => "resource_busy",
            Self::StaleNetworkHandle => "stale_network_handle",
            Self::SymlinkLoop => "symlink_loop",
            Self::SymlinkEscapeBlocked => "symlink_escape_blocked",
            Self::UnsupportedPath => "unsupported_path",
            Self::InvalidPath => "invalid_path",
            Self::IoInterrupted => "io_interrupted",
            Self::TimedOut => "timed_out",
            Self::UnknownIo => "unknown_io",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "missing" => Some(Self::Missing),
            "not_directory" => Some(Self::NotDirectory),
            "permission_denied" => Some(Self::PermissionDenied),
            "privacy_permission_required" => Some(Self::PrivacyPermissionRequired),
            "unavailable_mount" => Some(Self::UnavailableMount),
            "resource_busy" => Some(Self::ResourceBusy),
            "stale_network_handle" => Some(Self::StaleNetworkHandle),
            "symlink_loop" => Some(Self::SymlinkLoop),
            "symlink_escape_blocked" => Some(Self::SymlinkEscapeBlocked),
            "unsupported_path" => Some(Self::UnsupportedPath),
            "invalid_path" => Some(Self::InvalidPath),
            "io_interrupted" => Some(Self::IoInterrupted),
            "timed_out" => Some(Self::TimedOut),
            "unknown_io" => Some(Self::UnknownIo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceScanPhase {
    Idle,
    Scanning,
    Complete,
    Partial,
    Blocked,
    Failed,
}

impl SourceScanPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Scanning => "scanning",
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "idle" => Some(Self::Idle),
            "scanning" => Some(Self::Scanning),
            "complete" => Some(Self::Complete),
            "partial" => Some(Self::Partial),
            "blocked" => Some(Self::Blocked),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePresenceState {
    Present,
    Missing,
    Removed,
}

impl SourcePresenceState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Missing => "missing",
            Self::Removed => "removed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "present" => Some(Self::Present),
            "missing" => Some(Self::Missing),
            "removed" => Some(Self::Removed),
            _ => None,
        }
    }
}
