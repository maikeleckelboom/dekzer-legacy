#![deny(unsafe_code)]

mod commands;

use std::process;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.first().map(String::as_str) {
        Some("export-boundary-contract") => commands::boundary_contract::run_export_cli(args)
            .map_err(XtaskCliError::BoundaryContract),
        Some("check-boundary-contract") => commands::boundary_contract::run_check_cli(args)
            .map_err(XtaskCliError::BoundaryContract),
        Some("export-stdio-transport-contract") => {
            commands::stdio_transport_contract::run_export_cli(args)
                .map_err(XtaskCliError::StdioTransportContract)
        }
        Some("check-stdio-transport-contract") => {
            commands::stdio_transport_contract::run_check_cli(args)
                .map_err(XtaskCliError::StdioTransportContract)
        }
        _ => Err(XtaskCliError::Usage(
            "usage: cargo run -p xtask -- <export-boundary-contract|check-boundary-contract|export-stdio-transport-contract|check-stdio-transport-contract>"
                .to_string(),
        )),
    };

    if let Err(error) = result {
        for line in error.render_lines() {
            eprintln!("{line}");
        }

        process::exit(1);
    }
}

enum XtaskCliError {
    BoundaryContract(commands::boundary_contract::XtaskError),
    StdioTransportContract(commands::stdio_transport_contract::XtaskError),
    Usage(String),
}

impl XtaskCliError {
    fn render_lines(&self) -> Vec<String> {
        match self {
            Self::BoundaryContract(error) => error.render_lines(),
            Self::StdioTransportContract(error) => error.render_lines(),
            Self::Usage(message) => vec![
                format!("[xtask] detail: {message}"),
                "[xtask] error: invalid xtask invocation".to_string(),
            ],
        }
    }
}
