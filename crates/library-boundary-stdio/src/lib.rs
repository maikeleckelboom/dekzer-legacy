#![deny(unsafe_code)]

mod cli;
mod envelope;
mod server;

pub mod stdio_contract;

use std::io::{self, BufReader};
use std::path::Path;

use library_boundary_service::{
    LibraryBoundaryService, LibraryStorageEnvironment, LibraryStorageEnvironmentError,
    LibraryStoreContext, ProtocolError, StoreEnvironment, reset_development_library_storage,
    resolve_library_storage_environment,
};
use serde::Serialize;

use crate::cli::{CliCommand, StorageCommand, parse_cli_args};
use crate::server::run_server_loop;

pub fn run_main<I>(args: I) -> Result<(), MainError>
where
    I: IntoIterator<Item = String>,
{
    let command = parse_cli_args(args).map_err(MainError::cli)?;
    let config = match command {
        CliCommand::Serve(config) => config,
        CliCommand::Storage(command) => {
            return run_storage_command(command).map_err(MainError::storage);
        }
    };
    let service = LibraryBoundaryService::open(LibraryStoreContext {
        user_data_path: config.user_data_path,
        environment: config.environment,
    })
    .map_err(MainError::service_open)?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    run_server_loop(&service, BufReader::new(stdin.lock()), stdout.lock())
        .map_err(MainError::server_loop)
}

fn run_storage_command(command: StorageCommand) -> Result<(), StorageCommandError> {
    match command {
        StorageCommand::Status(config) => {
            let development_environment =
                storage_environment(&config.user_data_path, StoreEnvironment::Development)?;
            let production_environment =
                storage_environment(&config.user_data_path, StoreEnvironment::Production)?;
            let status = StorageStatusEnvelope {
                envelope_type: "storageStatus",
                user_data_path: path_string(development_environment.user_data_path()),
                development: StorageEnvironmentStatus::from_environment(&development_environment),
                production: StorageEnvironmentStatus::from_environment(&production_environment),
            };
            println!("{}", serde_json::to_string(&status)?);
            Ok(())
        }
        StorageCommand::Reset(config) => {
            let report =
                reset_development_library_storage(config.user_data_path, config.confirm_delete)?;
            let reset = StorageResetEnvelope {
                envelope_type: "storageReset",
                environment: report.reset_environment().environment().as_str(),
                reset_target_path: path_string(report.reset_target_path()),
                reset_target_existed: report.reset_target_existed(),
            };
            println!("{}", serde_json::to_string(&reset)?);
            Ok(())
        }
    }
}

fn storage_environment(
    user_data_path: &str,
    environment: StoreEnvironment,
) -> Result<LibraryStorageEnvironment, StorageCommandError> {
    resolve_library_storage_environment(&LibraryStoreContext {
        user_data_path: user_data_path.to_string(),
        environment,
    })
    .map_err(StorageCommandError::from)
}

#[derive(Debug)]
pub struct MainError {
    kind: MainErrorKind,
}

impl MainError {
    fn cli(error: cli::CliError) -> Self {
        Self {
            kind: MainErrorKind::Cli(error),
        }
    }

    fn server_loop(error: server::ServerLoopError) -> Self {
        Self {
            kind: MainErrorKind::ServerLoop(error),
        }
    }

    fn service_open(error: ProtocolError) -> Self {
        Self {
            kind: MainErrorKind::ServiceOpen(error),
        }
    }

    fn storage(error: StorageCommandError) -> Self {
        Self {
            kind: MainErrorKind::Storage(error),
        }
    }

    pub fn render_lines(&self) -> Vec<String> {
        match &self.kind {
            MainErrorKind::Cli(error) => error.render_lines(),
            MainErrorKind::ServerLoop(error) => error.render_lines(),
            MainErrorKind::ServiceOpen(error) => vec![
                format!(
                    "[library-boundary-stdio] detail: failed to open boundary service: {error}"
                ),
                "[library-boundary-stdio] error: serviceOpenFailure".to_string(),
            ],
            MainErrorKind::Storage(error) => vec![
                format!("[library-boundary-stdio] detail: storage command failed: {error}"),
                "[library-boundary-stdio] error: storageCommandFailure".to_string(),
            ],
        }
    }
}

#[derive(Debug)]
enum MainErrorKind {
    Cli(cli::CliError),
    ServerLoop(server::ServerLoopError),
    ServiceOpen(ProtocolError),
    Storage(StorageCommandError),
}

#[derive(Debug)]
enum StorageCommandError {
    Environment(LibraryStorageEnvironmentError),
    Json(serde_json::Error),
}

impl std::fmt::Display for StorageCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Environment(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "failed to serialize storage command output: {error}"),
        }
    }
}

impl std::error::Error for StorageCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Environment(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}

impl From<LibraryStorageEnvironmentError> for StorageCommandError {
    fn from(error: LibraryStorageEnvironmentError) -> Self {
        Self::Environment(error)
    }
}

impl From<serde_json::Error> for StorageCommandError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageStatusEnvelope {
    #[serde(rename = "type")]
    envelope_type: &'static str,
    user_data_path: String,
    development: StorageEnvironmentStatus,
    production: StorageEnvironmentStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageEnvironmentStatus {
    environment: &'static str,
    storage_root_path: String,
    storage_root_exists: bool,
    durable_store_path: String,
    durable_store_exists: bool,
    artifact_file_store_path: String,
    artifact_file_store_exists: bool,
    wal_path: String,
    wal_exists: bool,
    shm_path: String,
    shm_exists: bool,
}

impl StorageEnvironmentStatus {
    fn from_environment(environment: &LibraryStorageEnvironment) -> Self {
        Self {
            environment: environment.environment().as_str(),
            storage_root_path: path_string(environment.storage_root_path()),
            storage_root_exists: environment.storage_root_path().exists(),
            durable_store_path: path_string(environment.durable_store_path()),
            durable_store_exists: environment.durable_store_path().exists(),
            artifact_file_store_path: path_string(environment.artifact_file_store_path()),
            artifact_file_store_exists: environment.artifact_file_store_path().exists(),
            wal_path: path_string(environment.wal_path()),
            wal_exists: environment.wal_path().exists(),
            shm_path: path_string(environment.shm_path()),
            shm_exists: environment.shm_path().exists(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageResetEnvelope {
    #[serde(rename = "type")]
    envelope_type: &'static str,
    environment: &'static str,
    reset_target_path: String,
    reset_target_existed: bool,
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
