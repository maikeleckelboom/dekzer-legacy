use std::fmt;
use std::path::{Path, PathBuf};

use crate::authority::work::ArtifactFileStoreRoot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreEnvironment {
    Development,
    Production,
}

impl StoreEnvironment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Production => "production",
        }
    }
}

impl fmt::Display for StoreEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryStoreContext {
    pub user_data_path: String,
    pub environment: StoreEnvironment,
}

pub fn durable_store_path(user_data_path: &str, environment: StoreEnvironment) -> PathBuf {
    let root = PathBuf::from(user_data_path);
    match environment {
        StoreEnvironment::Development => root.join("development").join("library.sqlite3"),
        StoreEnvironment::Production => root.join("library.sqlite3"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableStoreBootstrapStatus {
    InstalledCanonicalBaseline,
    OpenedCanonicalStore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableStoreSchemaCompatibility {
    state: DurableStoreSchemaCompatibilityState,
    detail: Option<String>,
}

impl DurableStoreSchemaCompatibility {
    pub(crate) fn missing() -> Self {
        Self {
            state: DurableStoreSchemaCompatibilityState::Missing,
            detail: None,
        }
    }

    pub(crate) fn compatible() -> Self {
        Self {
            state: DurableStoreSchemaCompatibilityState::Compatible,
            detail: None,
        }
    }

    pub(crate) fn incompatible(detail: impl Into<String>) -> Self {
        Self {
            state: DurableStoreSchemaCompatibilityState::Incompatible,
            detail: Some(detail.into()),
        }
    }

    pub(crate) fn unreadable(detail: impl Into<String>) -> Self {
        Self {
            state: DurableStoreSchemaCompatibilityState::Unreadable,
            detail: Some(detail.into()),
        }
    }

    pub fn state(&self) -> DurableStoreSchemaCompatibilityState {
        self.state
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableStoreSchemaCompatibilityState {
    Missing,
    Compatible,
    Incompatible,
    Unreadable,
}

impl DurableStoreSchemaCompatibilityState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Compatible => "compatible",
            Self::Incompatible => "incompatible",
            Self::Unreadable => "unreadable",
        }
    }
}

/// App-owned library state is rooted at the durable SQLite path.
/// The sibling artifact file store is derived from that path
/// and is never derived from mounted source media.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteDurableStoreAppOwnedState {
    durable_store_path: PathBuf,
    artifact_file_store_root: ArtifactFileStoreRoot,
}

impl SqliteDurableStoreAppOwnedState {
    pub fn from_durable_store_path(path: impl AsRef<Path>) -> Self {
        let durable_store_path = path.as_ref().to_path_buf();
        Self {
            artifact_file_store_root: ArtifactFileStoreRoot::from_durable_store_path(
                &durable_store_path,
            ),
            durable_store_path,
        }
    }

    pub fn durable_store_path(&self) -> &Path {
        &self.durable_store_path
    }

    pub fn artifact_file_store_root(&self) -> &ArtifactFileStoreRoot {
        &self.artifact_file_store_root
    }
}
