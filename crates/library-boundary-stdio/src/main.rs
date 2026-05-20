#![deny(unsafe_code)]

mod cli;
mod envelope;
mod server;

use std::io::{self, BufReader};
use std::process;

use library_boundary_service::{LibraryBoundaryService, LibraryStoreContext, ProtocolError};

use crate::cli::parse_cli_args;
use crate::server::run_server_loop;

fn main() {
    install_panic_hook();

    if let Err(error) = run_main(std::env::args().skip(1)) {
        for line in error.render_lines() {
            eprintln!("{line}");
        }

        process::exit(1);
    }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("[library-boundary-stdio] panic: {panic_info}");
    }));
}

fn run_main<I>(args: I) -> Result<(), MainError>
where
    I: IntoIterator<Item = String>,
{
    let config = parse_cli_args(args).map_err(MainError::Cli)?;
    let service = LibraryBoundaryService::open(LibraryStoreContext {
        user_data_path: config.user_data_path,
        environment: config.environment,
    })
    .map_err(MainError::ServiceOpen)?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    run_server_loop(&service, BufReader::new(stdin.lock()), stdout.lock())
        .map_err(MainError::ServerLoop)
}

#[derive(Debug)]
enum MainError {
    Cli(cli::CliError),
    ServerLoop(server::ServerLoopError),
    ServiceOpen(ProtocolError),
}

impl MainError {
    fn render_lines(&self) -> Vec<String> {
        match self {
            Self::Cli(error) => error.render_lines(),
            Self::ServerLoop(error) => error.render_lines(),
            Self::ServiceOpen(error) => vec![
                format!(
                    "[library-boundary-stdio] detail: failed to open boundary service: {error}"
                ),
                "[library-boundary-stdio] error: serviceOpenFailure".to_string(),
            ],
        }
    }
}
