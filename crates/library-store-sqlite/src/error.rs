use crate::source_media::SourceMediaWriteViolation;
use rusqlite::{Error as SqliteError, ErrorCode, ffi};
use thiserror::Error;

pub type LibrarySqliteResult<T> = Result<T, LibrarySqliteError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanonicalErrorCode {
    NotFound,
    LineageExpired,
}

impl CanonicalErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "NOT_FOUND",
            Self::LineageExpired => "LINEAGE_EXPIRED",
        }
    }

    pub fn from_wire_value(value: &str) -> Result<Self, CanonicalErrorCodeParseError> {
        match value {
            "NOT_FOUND" => Ok(Self::NotFound),
            "LINEAGE_EXPIRED" => Ok(Self::LineageExpired),
            other => Err(CanonicalErrorCodeParseError(other.to_string())),
        }
    }
}

impl std::fmt::Display for CanonicalErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown canonical error code: {0}")]
pub struct CanonicalErrorCodeParseError(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code}: {message}")]
pub struct CanonicalError {
    pub code: CanonicalErrorCode,
    pub message: String,
}

impl CanonicalError {
    pub fn new(code: CanonicalErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DurableStoreOpenFailureKind {
    NonCanonicalSchema,
    CorruptOrUnreadableStore,
    IoFailure,
    PermissionFailure,
    UnsupportedJournalMode,
    UnexpectedFailure,
}

impl DurableStoreOpenFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NonCanonicalSchema => "NON_CANONICAL_SCHEMA",
            Self::CorruptOrUnreadableStore => "CORRUPT_OR_UNREADABLE_STORE",
            Self::IoFailure => "IO_FAILURE",
            Self::PermissionFailure => "PERMISSION_FAILURE",
            Self::UnsupportedJournalMode => "UNSUPPORTED_JOURNAL_MODE",
            Self::UnexpectedFailure => "UNEXPECTED_FAILURE",
        }
    }

    pub fn from_library_sqlite_error(error: &LibrarySqliteError) -> Self {
        match error {
            LibrarySqliteError::MalformedSchemaState(_) => Self::NonCanonicalSchema,
            LibrarySqliteError::JournalMode(_) => Self::UnsupportedJournalMode,
            LibrarySqliteError::RootPathCanonicalization { source, .. } => {
                if source.kind() == std::io::ErrorKind::PermissionDenied {
                    Self::PermissionFailure
                } else {
                    Self::IoFailure
                }
            }
            LibrarySqliteError::Sqlite(error) => classify_sqlite_open_error(error),
            _ => Self::UnexpectedFailure,
        }
    }
}

impl std::fmt::Display for DurableStoreOpenFailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error)]
#[error("{kind}: {detail}")]
pub struct DurableStoreOpenFailure {
    pub kind: DurableStoreOpenFailureKind,
    pub detail: String,
    #[source]
    pub source: LibrarySqliteError,
}

impl DurableStoreOpenFailure {
    pub fn from_library_sqlite_error(source: LibrarySqliteError) -> Self {
        let kind = DurableStoreOpenFailureKind::from_library_sqlite_error(&source);
        let detail = source.to_string();
        Self {
            kind,
            detail,
            source,
        }
    }
}

impl From<LibrarySqliteError> for DurableStoreOpenFailure {
    fn from(source: LibrarySqliteError) -> Self {
        Self::from_library_sqlite_error(source)
    }
}

#[derive(Debug, Error)]
pub enum LibrarySqliteError {
    #[error(transparent)]
    Canonical(#[from] CanonicalError),
    #[error(transparent)]
    SourceMediaWriteViolation(#[from] SourceMediaWriteViolation),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("failed to canonicalize root path {path:?}: {source}")]
    RootPathCanonicalization {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("canonical root path is not valid UTF-8: {0:?}")]
    NonUtf8CanonicalRootPath(std::path::PathBuf),
    #[error("system clock drifted before unix epoch: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
    #[error("discovery batch must contain at least one file")]
    EmptyDiscoveryBatch,
    #[error("discovery batch contains duplicate relative_path: {0}")]
    DuplicateDiscoveryRelativePath(String),
    #[error("root {0} does not exist")]
    MissingRoot(i64),
    #[error("file {0} does not exist")]
    MissingFile(i64),
    #[error(
        "root-bound work for root {root_id} was denied admission because the root is frozen or cancelled"
    )]
    RootWorkAdmissionDenied { root_id: i64 },
    #[error(
        "root-bound work for root {root_id} was cancelled after admission and before completion"
    )]
    RootWorkCancelled { root_id: i64 },
    #[error(
        "root-bound work for root {root_id} was admitted under mount_epoch {admitted_mount_epoch} but current mount_epoch is {current_mount_epoch}"
    )]
    StaleMountEpochStamp {
        root_id: i64,
        admitted_mount_epoch: i64,
        current_mount_epoch: i64,
    },
    #[error("artifact file-store {action} failed for {path:?}: {source}")]
    ArtifactFileStoreIo {
        action: &'static str,
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("database must be in WAL mode, got journal_mode={0}")]
    JournalMode(String),
    #[error("database schema state is malformed: {0}")]
    MalformedSchemaState(String),
    #[error("canonical write invariant failed: {0}")]
    WriteInvariant(String),
}

impl LibrarySqliteError {
    pub fn canonical_code(&self) -> Option<CanonicalErrorCode> {
        match self {
            Self::Canonical(error) => Some(error.code),
            _ => None,
        }
    }
}

fn classify_sqlite_open_error(error: &SqliteError) -> DurableStoreOpenFailureKind {
    match error {
        SqliteError::SqliteFailure(sqlite_error, detail) => match sqlite_error.code {
            ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase => {
                DurableStoreOpenFailureKind::CorruptOrUnreadableStore
            }
            ErrorCode::PermissionDenied | ErrorCode::ReadOnly => {
                DurableStoreOpenFailureKind::PermissionFailure
            }
            ErrorCode::SystemIoFailure => {
                if sqlite_extended_code_implies_unreadable_store(sqlite_error.extended_code) {
                    DurableStoreOpenFailureKind::CorruptOrUnreadableStore
                } else {
                    DurableStoreOpenFailureKind::IoFailure
                }
            }
            ErrorCode::CannotOpen => {
                if sqlite_detail_implies_permission_denied(detail.as_deref()) {
                    DurableStoreOpenFailureKind::PermissionFailure
                } else {
                    DurableStoreOpenFailureKind::IoFailure
                }
            }
            _ => DurableStoreOpenFailureKind::UnexpectedFailure,
        },
        SqliteError::InvalidPath(_) => DurableStoreOpenFailureKind::IoFailure,
        _ => DurableStoreOpenFailureKind::UnexpectedFailure,
    }
}

fn sqlite_extended_code_implies_unreadable_store(extended_code: i32) -> bool {
    matches!(
        extended_code,
        ffi::SQLITE_IOERR_SHORT_READ
            | ffi::SQLITE_IOERR_DATA
            | ffi::SQLITE_IOERR_CORRUPTFS
            | ffi::SQLITE_IOERR_SEEK
            | ffi::SQLITE_IOERR_IN_PAGE
    )
}

fn sqlite_detail_implies_permission_denied(detail: Option<&str>) -> bool {
    detail
        .map(|detail| {
            let detail = detail.to_ascii_lowercase();
            detail.contains("permission denied")
                || detail.contains("access is denied")
                || detail.contains("readonly")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{
        CanonicalError, CanonicalErrorCode, DurableStoreOpenFailure, DurableStoreOpenFailureKind,
        LibrarySqliteError,
    };
    use rusqlite::ffi;

    #[test]
    fn lineage_expired_round_trips_through_canonical_error_code_boundary() {
        let encoded = CanonicalErrorCode::LineageExpired.as_str();
        let decoded =
            CanonicalErrorCode::from_wire_value(encoded).expect("lineage-expired code decodes");

        assert_eq!(encoded, "LINEAGE_EXPIRED");
        assert_eq!(decoded, CanonicalErrorCode::LineageExpired);
    }

    #[test]
    fn lineage_expired_remains_distinct_from_not_found() {
        let not_found = CanonicalErrorCode::from_wire_value("NOT_FOUND")
            .expect("not-found code should decode canonically");
        let lineage_expired = CanonicalErrorCode::from_wire_value("LINEAGE_EXPIRED")
            .expect("lineage-expired code should decode canonically");

        assert_ne!(not_found, lineage_expired);

        let backend_error = LibrarySqliteError::from(CanonicalError::new(
            lineage_expired,
            "publication anchor expired",
        ));
        assert_eq!(
            backend_error.canonical_code(),
            Some(CanonicalErrorCode::LineageExpired)
        );
    }

    #[test]
    fn durable_store_open_failure_classifies_permission_denied_sqlite_errors() {
        let failure = DurableStoreOpenFailure::from(LibrarySqliteError::Sqlite(
            rusqlite::Error::SqliteFailure(
                ffi::Error::new(ffi::SQLITE_PERM),
                Some("permission denied".to_string()),
            ),
        ));

        assert_eq!(failure.kind, DurableStoreOpenFailureKind::PermissionFailure);
    }

    #[test]
    fn durable_store_open_failure_classifies_unsupported_journal_mode() {
        let failure =
            DurableStoreOpenFailure::from(LibrarySqliteError::JournalMode("delete".to_string()));

        assert_eq!(
            failure.kind,
            DurableStoreOpenFailureKind::UnsupportedJournalMode
        );
    }

    #[test]
    fn durable_store_open_failure_classifies_non_canonical_schema() {
        let failure = DurableStoreOpenFailure::from(LibrarySqliteError::MalformedSchemaState(
            "unexpected prep_tracks table".to_string(),
        ));

        assert_eq!(
            failure.kind,
            DurableStoreOpenFailureKind::NonCanonicalSchema
        );
    }
}
