use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

use library_store_sqlite::{
    LibraryStoreContext, SqliteDurableStoreAppOwnedState, StoreEnvironment, durable_store_path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryStorageEnvironment {
    user_data_path: PathBuf,
    environment: StoreEnvironment,
    storage_root_path: PathBuf,
    durable_store_path: PathBuf,
    artifact_file_store_path: PathBuf,
    wal_path: PathBuf,
    shm_path: PathBuf,
}

impl LibraryStorageEnvironment {
    pub fn user_data_path(&self) -> &Path {
        &self.user_data_path
    }

    pub fn environment(&self) -> StoreEnvironment {
        self.environment
    }

    pub fn storage_root_path(&self) -> &Path {
        &self.storage_root_path
    }

    pub fn durable_store_path(&self) -> &Path {
        &self.durable_store_path
    }

    pub fn artifact_file_store_path(&self) -> &Path {
        &self.artifact_file_store_path
    }

    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    pub fn shm_path(&self) -> &Path {
        &self.shm_path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryStorageResetReport {
    reset_environment: LibraryStorageEnvironment,
    reset_target_existed: bool,
}

impl LibraryStorageResetReport {
    pub fn reset_environment(&self) -> &LibraryStorageEnvironment {
        &self.reset_environment
    }

    pub fn reset_target_path(&self) -> &Path {
        self.reset_environment.storage_root_path()
    }

    pub fn reset_target_existed(&self) -> bool {
        self.reset_target_existed
    }
}

#[derive(Debug)]
pub enum LibraryStorageEnvironmentError {
    InvalidUserDataPath(String),
    MissingResetConfirmation,
    UnsafeResetTarget(PathBuf),
    Io {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for LibraryStorageEnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUserDataPath(detail) => f.write_str(detail),
            Self::MissingResetConfirmation => {
                f.write_str("storage reset requires --confirm-delete")
            }
            Self::UnsafeResetTarget(path) => {
                write!(f, "refusing to reset unsafe storage target {path:?}")
            }
            Self::Io {
                action,
                path,
                source,
            } => {
                write!(f, "storage {action} failed for {path:?}: {source}")
            }
        }
    }
}

impl std::error::Error for LibraryStorageEnvironmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn resolve_library_storage_environment(
    context: &LibraryStoreContext,
) -> Result<LibraryStorageEnvironment, LibraryStorageEnvironmentError> {
    let user_data_path = validate_user_data_path(&context.user_data_path)?;
    let normalized_user_data_path = user_data_path.to_string_lossy().into_owned();
    let durable_store_path = durable_store_path(&normalized_user_data_path, context.environment);
    let storage_root_path = durable_store_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            LibraryStorageEnvironmentError::InvalidUserDataPath(format!(
                "libraryStore userDataPath must resolve to a durable store parent: {:?}",
                context.user_data_path
            ))
        })?;
    let app_owned_state =
        SqliteDurableStoreAppOwnedState::from_durable_store_path(&durable_store_path);

    Ok(LibraryStorageEnvironment {
        user_data_path,
        environment: context.environment,
        storage_root_path,
        wal_path: append_path_suffix(&durable_store_path, "-wal"),
        shm_path: append_path_suffix(&durable_store_path, "-shm"),
        durable_store_path,
        artifact_file_store_path: app_owned_state
            .artifact_file_store_root()
            .path()
            .to_path_buf(),
    })
}

pub fn reset_development_library_storage(
    user_data_path: String,
    confirm_delete: bool,
) -> Result<LibraryStorageResetReport, LibraryStorageEnvironmentError> {
    if !confirm_delete {
        return Err(LibraryStorageEnvironmentError::MissingResetConfirmation);
    }

    let context = LibraryStoreContext {
        user_data_path,
        environment: StoreEnvironment::Development,
    };
    let reset_environment = resolve_library_storage_environment(&context)?;
    validate_development_reset_target(&reset_environment)?;

    let reset_target = reset_environment.storage_root_path();
    let reset_target_existed = reset_target.exists();
    if reset_target_existed {
        fs::remove_dir_all(reset_target).map_err(|source| LibraryStorageEnvironmentError::Io {
            action: "reset",
            path: reset_target.to_path_buf(),
            source,
        })?;
    }

    Ok(LibraryStorageResetReport {
        reset_environment,
        reset_target_existed,
    })
}

fn validate_user_data_path(
    user_data_path: &str,
) -> Result<PathBuf, LibraryStorageEnvironmentError> {
    let trimmed_path = user_data_path.trim();
    if trimmed_path.is_empty() {
        return Err(LibraryStorageEnvironmentError::InvalidUserDataPath(
            "libraryStore userDataPath must not be empty".to_string(),
        ));
    }

    let path = PathBuf::from(trimmed_path);
    if !path.is_absolute() {
        return Err(LibraryStorageEnvironmentError::InvalidUserDataPath(
            format!("libraryStore userDataPath must be absolute: {trimmed_path:?}"),
        ));
    }

    if path.parent().is_none() {
        return Err(LibraryStorageEnvironmentError::InvalidUserDataPath(
            format!("libraryStore userDataPath must not be a filesystem root: {trimmed_path:?}"),
        ));
    }

    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(LibraryStorageEnvironmentError::InvalidUserDataPath(
            format!(
                "libraryStore userDataPath must not contain parent directory segments: {trimmed_path:?}"
            ),
        ));
    }

    Ok(path)
}

fn validate_development_reset_target(
    environment: &LibraryStorageEnvironment,
) -> Result<(), LibraryStorageEnvironmentError> {
    if environment.environment != StoreEnvironment::Development {
        return Err(LibraryStorageEnvironmentError::UnsafeResetTarget(
            environment.storage_root_path.clone(),
        ));
    }

    let expected_target = environment.user_data_path().join("development");
    if environment.storage_root_path() != expected_target {
        return Err(LibraryStorageEnvironmentError::UnsafeResetTarget(
            environment.storage_root_path.clone(),
        ));
    }

    Ok(())
}

fn append_path_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = OsString::from(path.as_os_str());
    value.push(suffix);
    PathBuf::from(value)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{
        LibraryStorageEnvironmentError, reset_development_library_storage,
        resolve_library_storage_environment,
    };
    use library_store_sqlite::{LibraryStoreContext, StoreEnvironment};

    #[test]
    fn resolves_development_storage_under_explicit_user_data_root() {
        let tempdir = TempDir::new().expect("create tempdir");
        let user_data_path = tempdir.path().join("user-data");
        let context = LibraryStoreContext {
            user_data_path: user_data_path.to_string_lossy().into_owned(),
            environment: StoreEnvironment::Development,
        };

        let environment =
            resolve_library_storage_environment(&context).expect("resolve storage environment");

        assert_eq!(environment.user_data_path(), user_data_path);
        assert_eq!(environment.environment(), StoreEnvironment::Development);
        assert_eq!(
            environment.storage_root_path(),
            user_data_path.join("development")
        );
        assert_eq!(
            environment.durable_store_path(),
            user_data_path.join("development").join("library.sqlite3")
        );
        assert_eq!(
            environment.artifact_file_store_path(),
            user_data_path
                .join("development")
                .join("library.sqlite3.artifact-file-store")
        );
        assert_eq!(
            environment.wal_path(),
            user_data_path
                .join("development")
                .join("library.sqlite3-wal")
        );
        assert_eq!(
            environment.shm_path(),
            user_data_path
                .join("development")
                .join("library.sqlite3-shm")
        );
    }

    #[test]
    fn resolves_production_storage_under_explicit_user_data_root() {
        let tempdir = TempDir::new().expect("create tempdir");
        let user_data_path = tempdir.path().join("user-data");
        let context = LibraryStoreContext {
            user_data_path: user_data_path.to_string_lossy().into_owned(),
            environment: StoreEnvironment::Production,
        };

        let environment =
            resolve_library_storage_environment(&context).expect("resolve storage environment");

        assert_eq!(environment.storage_root_path(), user_data_path);
        assert_eq!(
            environment.durable_store_path(),
            user_data_path.join("library.sqlite3")
        );
    }

    #[test]
    fn rejects_relative_user_data_roots() {
        let error = resolve_library_storage_environment(&LibraryStoreContext {
            user_data_path: "relative-user-data".to_string(),
            environment: StoreEnvironment::Development,
        })
        .expect_err("relative user data root is invalid");

        assert!(matches!(
            error,
            LibraryStorageEnvironmentError::InvalidUserDataPath(_)
        ));
    }

    #[test]
    fn rejects_parent_directory_segments_in_user_data_roots() {
        let tempdir = TempDir::new().expect("create tempdir");
        let user_data_path = tempdir.path().join("user-data").join("..").join("other");
        let error = resolve_library_storage_environment(&LibraryStoreContext {
            user_data_path: user_data_path.to_string_lossy().into_owned(),
            environment: StoreEnvironment::Development,
        })
        .expect_err("parent directory segments are rejected");

        assert!(matches!(
            error,
            LibraryStorageEnvironmentError::InvalidUserDataPath(_)
        ));
    }

    #[test]
    fn development_reset_requires_confirmation() {
        let tempdir = TempDir::new().expect("create tempdir");
        let error = reset_development_library_storage(
            tempdir
                .path()
                .join("user-data")
                .to_string_lossy()
                .into_owned(),
            false,
        )
        .expect_err("reset without confirmation is rejected");

        assert!(matches!(
            error,
            LibraryStorageEnvironmentError::MissingResetConfirmation
        ));
    }

    #[test]
    fn development_reset_removes_only_development_storage_root() {
        let tempdir = TempDir::new().expect("create tempdir");
        let user_data_path = tempdir.path().join("user-data");
        let development_storage = user_data_path.join("development");
        let production_marker = user_data_path.join("production-marker.txt");
        std::fs::create_dir_all(&development_storage).expect("create development storage");
        std::fs::write(development_storage.join("library.sqlite3"), b"not sqlite")
            .expect("write development database");
        std::fs::write(&production_marker, b"keep").expect("write production marker");

        let report =
            reset_development_library_storage(user_data_path.to_string_lossy().into_owned(), true)
                .expect("reset development storage");

        assert_eq!(report.reset_target_path(), development_storage);
        assert!(report.reset_target_existed());
        assert!(!development_storage.exists());
        assert!(production_marker.exists());
        assert!(user_data_path.exists());
    }
}
