use std::path::{Path, PathBuf};

use library_domain::SourceAccessIssueKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceAccessProbeResult {
    Accessible {
        effective_root: PathBuf,
        checked_at_ms: i64,
    },
    Missing {
        issue_kind: SourceAccessIssueKind,
        diagnostic_detail: Option<String>,
        checked_at_ms: i64,
    },
    Blocked {
        issue_kind: SourceAccessIssueKind,
        diagnostic_detail: Option<String>,
        checked_at_ms: i64,
    },
}

impl SourceAccessProbeResult {
    pub fn issue_kind(&self) -> Option<SourceAccessIssueKind> {
        match self {
            Self::Accessible { .. } => None,
            Self::Missing { issue_kind, .. } | Self::Blocked { issue_kind, .. } => {
                Some(*issue_kind)
            }
        }
    }

    pub fn diagnostic_detail(&self) -> Option<&str> {
        match self {
            Self::Accessible { .. } => None,
            Self::Missing {
                diagnostic_detail, ..
            }
            | Self::Blocked {
                diagnostic_detail, ..
            } => diagnostic_detail.as_deref(),
        }
    }

    pub fn checked_at_ms(&self) -> i64 {
        match self {
            Self::Accessible { checked_at_ms, .. }
            | Self::Missing { checked_at_ms, .. }
            | Self::Blocked { checked_at_ms, .. } => *checked_at_ms,
        }
    }

    pub fn is_accessible(&self) -> bool {
        matches!(self, Self::Accessible { .. })
    }
}

pub fn probe_source_access(root_path: &Path, checked_at_ms: i64) -> SourceAccessProbeResult {
    if root_path.as_os_str().is_empty() {
        return SourceAccessProbeResult::Blocked {
            issue_kind: SourceAccessIssueKind::InvalidPath,
            diagnostic_detail: Some("source root path is empty".to_string()),
            checked_at_ms,
        };
    }

    let metadata = match std::fs::symlink_metadata(root_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return SourceAccessProbeResult::Missing {
                issue_kind: SourceAccessIssueKind::Missing,
                diagnostic_detail: Some(error.to_string()),
                checked_at_ms,
            };
        }
        Err(error) => {
            return SourceAccessProbeResult::Blocked {
                issue_kind: source_access_issue_kind_from_io_error(&error),
                diagnostic_detail: Some(error.to_string()),
                checked_at_ms,
            };
        }
    };

    if metadata.file_type().is_symlink() {
        let target = match std::fs::canonicalize(root_path) {
            Ok(target) => target,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return SourceAccessProbeResult::Missing {
                    issue_kind: SourceAccessIssueKind::Missing,
                    diagnostic_detail: Some(error.to_string()),
                    checked_at_ms,
                };
            }
            Err(error) => {
                return SourceAccessProbeResult::Blocked {
                    issue_kind: source_access_issue_kind_from_io_error(&error),
                    diagnostic_detail: Some(error.to_string()),
                    checked_at_ms,
                };
            }
        };
        return probe_resolved_directory(root_path, &target, checked_at_ms);
    }

    if !metadata.is_dir() {
        return SourceAccessProbeResult::Blocked {
            issue_kind: SourceAccessIssueKind::NotDirectory,
            diagnostic_detail: Some(format!("source root {:?} is not a directory", root_path)),
            checked_at_ms,
        };
    }

    probe_resolved_directory(root_path, root_path, checked_at_ms)
}

pub fn source_access_issue_kind_from_io_error(error: &std::io::Error) -> SourceAccessIssueKind {
    match error.kind() {
        std::io::ErrorKind::NotFound => SourceAccessIssueKind::Missing,
        std::io::ErrorKind::PermissionDenied => SourceAccessIssueKind::PermissionDenied,
        std::io::ErrorKind::Interrupted => SourceAccessIssueKind::IoInterrupted,
        std::io::ErrorKind::TimedOut => SourceAccessIssueKind::TimedOut,
        std::io::ErrorKind::InvalidData | std::io::ErrorKind::InvalidInput => {
            SourceAccessIssueKind::InvalidPath
        }
        std::io::ErrorKind::WouldBlock => SourceAccessIssueKind::ResourceBusy,
        _ => SourceAccessIssueKind::UnknownIo,
    }
}

fn probe_resolved_directory(
    requested_root: &Path,
    effective_root: &Path,
    checked_at_ms: i64,
) -> SourceAccessProbeResult {
    let metadata = match std::fs::metadata(effective_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return SourceAccessProbeResult::Missing {
                issue_kind: SourceAccessIssueKind::Missing,
                diagnostic_detail: Some(error.to_string()),
                checked_at_ms,
            };
        }
        Err(error) => {
            return SourceAccessProbeResult::Blocked {
                issue_kind: source_access_issue_kind_from_io_error(&error),
                diagnostic_detail: Some(error.to_string()),
                checked_at_ms,
            };
        }
    };
    if !metadata.is_dir() {
        return SourceAccessProbeResult::Blocked {
            issue_kind: SourceAccessIssueKind::NotDirectory,
            diagnostic_detail: Some(format!(
                "source root {:?} resolved from {:?} is not a directory",
                effective_root, requested_root
            )),
            checked_at_ms,
        };
    }

    match std::fs::read_dir(effective_root) {
        Ok(_) => SourceAccessProbeResult::Accessible {
            effective_root: effective_root.to_path_buf(),
            checked_at_ms,
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            SourceAccessProbeResult::Missing {
                issue_kind: SourceAccessIssueKind::Missing,
                diagnostic_detail: Some(error.to_string()),
                checked_at_ms,
            }
        }
        Err(error) => SourceAccessProbeResult::Blocked {
            issue_kind: source_access_issue_kind_from_io_error(&error),
            diagnostic_detail: Some(error.to_string()),
            checked_at_ms,
        },
    }
}
