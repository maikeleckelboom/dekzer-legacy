#![deny(unsafe_code)]

mod cli;
mod envelope;
mod server;

pub mod stdio_contract;

use std::io::{self, BufReader};

use library_boundary_service::{LibraryBoundaryService, LibraryStoreContext, ProtocolError};

use crate::cli::parse_cli_args;
use crate::server::run_server_loop;

pub fn run_main<I>(args: I) -> Result<(), MainError>
where
    I: IntoIterator<Item = String>,
{
    let config = parse_cli_args(args).map_err(MainError::cli)?;
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
        }
    }
}

#[derive(Debug)]
enum MainErrorKind {
    Cli(cli::CliError),
    ServerLoop(server::ServerLoopError),
    ServiceOpen(ProtocolError),
}
