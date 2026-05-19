use std::fs::{self, File, Metadata};
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceMediaAccessKind {
    ReadOnlySourceAccess,
    ExplicitUserRequestedSourceWrite,
}

impl SourceMediaAccessKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnlySourceAccess => "read_only_source_access",
            Self::ExplicitUserRequestedSourceWrite => "explicit_user_requested_source_write",
        }
    }
}

impl std::fmt::Display for SourceMediaAccessKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppOwnedStorageKind {
    /// App-owned library state rooted at the durable SQLite store path.
    /// This covers the SQLite file itself plus its sibling artifact file store,
    /// and never denotes mounted source media.
    LibraryStoreSqliteState,
}

impl AppOwnedStorageKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LibraryStoreSqliteState => "library_store_sqlite_state",
        }
    }

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "library_store_sqlite_state" => Some(Self::LibraryStoreSqliteState),
            _ => None,
        }
    }
}

impl std::fmt::Display for AppOwnedStorageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum SourceMediaReferenceKind {
    SourceRootRelativeFile,
}

impl SourceMediaReferenceKind {
    #[allow(dead_code)]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceRootRelativeFile => "source_root_relative_file",
        }
    }
}

impl std::fmt::Display for SourceMediaReferenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceMediaOperation {
    RootScan,
    FilesystemDiscovery,
    Classification,
    CueSidecarDetection,
    CueSheetParsing,
    Probe,
    MediaInspection,
    MediaAssetMaterialization,
    ArtifactMaterialization,
    ArtifactCleanup,
}

impl SourceMediaOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RootScan => "root_scan",
            Self::FilesystemDiscovery => "filesystem_discovery",
            Self::Classification => "classification",
            Self::CueSidecarDetection => "cue_sidecar_detection",
            Self::CueSheetParsing => "cue_sheet_parsing",
            Self::Probe => "probe",
            Self::MediaInspection => "media_inspection",
            Self::MediaAssetMaterialization => "media_asset_materialization",
            Self::ArtifactMaterialization => "artifact_materialization",
            Self::ArtifactCleanup => "artifact_cleanup",
        }
    }
}

impl std::fmt::Display for SourceMediaOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceMediaWritePolicy {
    operation: SourceMediaOperation,
    access_kind: SourceMediaAccessKind,
}

impl SourceMediaWritePolicy {
    #[allow(dead_code)]
    pub const fn read_only(operation: SourceMediaOperation) -> Self {
        Self {
            operation,
            access_kind: SourceMediaAccessKind::ReadOnlySourceAccess,
        }
    }

    // Explicit source writes are blocked from the maintained boundary but
    // remain representable for lower-layer export/materialization paths.
    #[allow(dead_code)]
    pub const fn explicit_user_requested_source_write(operation: SourceMediaOperation) -> Self {
        Self {
            operation,
            access_kind: SourceMediaAccessKind::ExplicitUserRequestedSourceWrite,
        }
    }

    // Kept for audit/error rendering in dormant source-media write guards.
    #[allow(dead_code)]
    pub const fn operation(self) -> SourceMediaOperation {
        self.operation
    }

    pub const fn access_kind(self) -> SourceMediaAccessKind {
        self.access_kind
    }

    // Guard retained for future explicit source-write flows below this cutover.
    #[allow(dead_code)]
    pub fn reject_source_write_destination(
        self,
        root_id: i64,
        destination_path: impl Into<PathBuf>,
    ) -> Result<(), SourceMediaWriteViolation> {
        match self.access_kind {
            SourceMediaAccessKind::ReadOnlySourceAccess => {
                Err(SourceMediaWriteViolation::ForbiddenBackgroundSourceWrite {
                    operation: self.operation,
                    access_kind: self.access_kind,
                    root_id,
                    destination_path: destination_path.into(),
                })
            }
            SourceMediaAccessKind::ExplicitUserRequestedSourceWrite => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SourceMediaWriteViolation {
    #[error(
        "forbidden source-media write during {operation}: access_kind={access_kind} root_id={root_id} destination_path={destination_path:?}"
    )]
    ForbiddenBackgroundSourceWrite {
        operation: SourceMediaOperation,
        access_kind: SourceMediaAccessKind,
        root_id: i64,
        destination_path: PathBuf,
    },
}

fn debug_assert_read_only_source_access(policy: SourceMediaWritePolicy) {
    debug_assert_eq!(
        policy.access_kind(),
        SourceMediaAccessKind::ReadOnlySourceAccess,
        "source-media read helper must only be used with read-only policy"
    );
}

#[allow(dead_code)]
pub(crate) fn open_source_media_file_for_read(
    policy: SourceMediaWritePolicy,
    path: &Path,
) -> std::io::Result<File> {
    debug_assert_read_only_source_access(policy);
    File::open(path)
}

// Text reads are used by dormant source-media import/discovery helpers.
#[allow(dead_code)]
pub(crate) fn read_source_media_text(
    policy: SourceMediaWritePolicy,
    path: &Path,
) -> std::io::Result<String> {
    debug_assert_read_only_source_access(policy);
    fs::read_to_string(path)
}

#[allow(dead_code)]
pub(crate) fn source_media_metadata(
    policy: SourceMediaWritePolicy,
    path: &Path,
) -> std::io::Result<Metadata> {
    debug_assert_read_only_source_access(policy);
    fs::metadata(path)
}

#[cfg(test)]
mod tests {
    use super::{
        AppOwnedStorageKind, SourceMediaAccessKind, SourceMediaOperation, SourceMediaReferenceKind,
        SourceMediaWritePolicy, SourceMediaWriteViolation,
    };
    use std::path::PathBuf;

    #[test]
    fn read_only_policy_rejects_background_source_writes_loudly() {
        let error = SourceMediaWritePolicy::read_only(SourceMediaOperation::Probe)
            .reject_source_write_destination(7, PathBuf::from("/media/usb/.dekz/cache.json"))
            .expect_err("read-only probe policy must reject source writes");

        assert_eq!(
            error,
            SourceMediaWriteViolation::ForbiddenBackgroundSourceWrite {
                operation: SourceMediaOperation::Probe,
                access_kind: SourceMediaAccessKind::ReadOnlySourceAccess,
                root_id: 7,
                destination_path: PathBuf::from("/media/usb/.dekz/cache.json"),
            }
        );
    }

    #[test]
    fn explicit_user_requested_write_policy_allows_source_writes() {
        SourceMediaWritePolicy::explicit_user_requested_source_write(
            SourceMediaOperation::ArtifactMaterialization,
        )
        .reject_source_write_destination(11, PathBuf::from("/media/usb/export.nml"))
        .expect("explicit user-requested source writes must remain representable");
    }

    #[test]
    fn app_owned_storage_kind_is_named_explicitly() {
        assert_eq!(
            AppOwnedStorageKind::LibraryStoreSqliteState.as_str(),
            "library_store_sqlite_state"
        );
        assert_eq!(
            AppOwnedStorageKind::from_db_value("library_store_sqlite_state"),
            Some(AppOwnedStorageKind::LibraryStoreSqliteState)
        );
        assert_eq!(AppOwnedStorageKind::from_db_value("unknown"), None);
    }

    #[test]
    fn source_media_reference_kind_is_named_explicitly() {
        assert_eq!(
            SourceMediaReferenceKind::SourceRootRelativeFile.as_str(),
            "source_root_relative_file"
        );
    }
}
