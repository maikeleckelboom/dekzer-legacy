use library_boundary_service::StoreEnvironment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerConfig {
    pub(crate) user_data_path: String,
    pub(crate) environment: StoreEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliCommand {
    Serve(ServerConfig),
    Storage(StorageCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StorageCommand {
    Status(StorageConfig),
    Reset(StorageResetConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StorageConfig {
    pub(crate) user_data_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StorageResetConfig {
    pub(crate) user_data_path: String,
    pub(crate) confirm_delete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliError {
    DuplicateArgument(&'static str),
    InvalidEnvironment(String),
    MissingRequired(&'static str),
    MissingStorageSubcommand,
    MissingValue(&'static str),
    UnknownArgument(String),
    UnknownStorageSubcommand(String),
}

impl CliError {
    pub(crate) fn render_lines(&self) -> Vec<String> {
        vec![
            format!("[library-boundary-stdio] detail: {self}"),
            "[library-boundary-stdio] error: invalid stdio server invocation".to_string(),
            "[library-boundary-stdio] usage: library-boundary-stdio --user-data-path <path> --env <development|production>".to_string(),
            "[library-boundary-stdio] usage: library-boundary-stdio storage status --user-data <path>".to_string(),
            "[library-boundary-stdio] usage: library-boundary-stdio storage reset --user-data <path> --confirm-delete".to_string(),
        ]
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateArgument(flag) => {
                write!(f, "duplicate {flag} argument")
            }
            Self::InvalidEnvironment(value) => {
                write!(
                    f,
                    "invalid --env value {value:?}; expected development or production"
                )
            }
            Self::MissingRequired(flag) => {
                write!(f, "missing required {flag} argument")
            }
            Self::MissingStorageSubcommand => {
                write!(f, "missing storage subcommand; expected status or reset")
            }
            Self::MissingValue(flag) => {
                write!(f, "missing value for {flag}")
            }
            Self::UnknownArgument(argument) => {
                write!(f, "unknown argument {argument:?}")
            }
            Self::UnknownStorageSubcommand(argument) => {
                write!(f, "unknown storage subcommand {argument:?}")
            }
        }
    }
}

pub(crate) fn parse_cli_args<I>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("storage") {
        return parse_storage_command(args.into_iter().skip(1));
    }

    parse_server_command(args).map(CliCommand::Serve)
}

fn parse_server_command<I>(args: I) -> Result<ServerConfig, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut user_data_path = None;
    let mut environment = None;
    let mut iter = args.into_iter();

    while let Some(argument) = iter.next() {
        if let Some(value) = argument.strip_prefix("--user-data-path=") {
            set_once(&mut user_data_path, "--user-data-path", value.to_string())?;
            continue;
        }

        if argument == "--user-data-path" {
            let value = iter
                .next()
                .ok_or(CliError::MissingValue("--user-data-path"))?;
            set_once(&mut user_data_path, "--user-data-path", value)?;
            continue;
        }

        if let Some(value) = argument.strip_prefix("--env=") {
            set_once(&mut environment, "--env", parse_environment(value)?)?;
            continue;
        }

        if argument == "--env" {
            let value = iter.next().ok_or(CliError::MissingValue("--env"))?;
            set_once(&mut environment, "--env", parse_environment(&value)?)?;
            continue;
        }

        return Err(CliError::UnknownArgument(argument));
    }

    Ok(ServerConfig {
        user_data_path: user_data_path.ok_or(CliError::MissingRequired("--user-data-path"))?,
        environment: environment.ok_or(CliError::MissingRequired("--env"))?,
    })
}

fn parse_storage_command<I>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let subcommand = iter.next().ok_or(CliError::MissingStorageSubcommand)?;

    match subcommand.as_str() {
        "status" => parse_storage_status(iter)
            .map(|config| CliCommand::Storage(StorageCommand::Status(config))),
        "reset" => parse_storage_reset(iter)
            .map(|config| CliCommand::Storage(StorageCommand::Reset(config))),
        _ => Err(CliError::UnknownStorageSubcommand(subcommand)),
    }
}

fn parse_storage_status<I>(args: I) -> Result<StorageConfig, CliError>
where
    I: IntoIterator<Item = String>,
{
    let (user_data_path, _) = parse_storage_options(args)?;
    Ok(StorageConfig {
        user_data_path: user_data_path.ok_or(CliError::MissingRequired("--user-data"))?,
    })
}

fn parse_storage_reset<I>(args: I) -> Result<StorageResetConfig, CliError>
where
    I: IntoIterator<Item = String>,
{
    let (user_data_path, confirm_delete) = parse_storage_options(args)?;
    Ok(StorageResetConfig {
        user_data_path: user_data_path.ok_or(CliError::MissingRequired("--user-data"))?,
        confirm_delete,
    })
}

fn parse_storage_options<I>(args: I) -> Result<(Option<String>, bool), CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut user_data_path = None;
    let mut confirm_delete = false;
    let mut iter = args.into_iter();

    while let Some(argument) = iter.next() {
        if let Some(value) = argument.strip_prefix("--user-data=") {
            set_once(&mut user_data_path, "--user-data", value.to_string())?;
            continue;
        }

        if argument == "--user-data" {
            let value = iter.next().ok_or(CliError::MissingValue("--user-data"))?;
            set_once(&mut user_data_path, "--user-data", value)?;
            continue;
        }

        if let Some(value) = argument.strip_prefix("--user-data-path=") {
            set_once(&mut user_data_path, "--user-data", value.to_string())?;
            continue;
        }

        if argument == "--user-data-path" {
            let value = iter
                .next()
                .ok_or(CliError::MissingValue("--user-data-path"))?;
            set_once(&mut user_data_path, "--user-data", value)?;
            continue;
        }

        if argument == "--confirm-delete" {
            if confirm_delete {
                return Err(CliError::DuplicateArgument("--confirm-delete"));
            }
            confirm_delete = true;
            continue;
        }

        return Err(CliError::UnknownArgument(argument));
    }

    Ok((user_data_path, confirm_delete))
}

fn set_once<T>(slot: &mut Option<T>, flag: &'static str, value: T) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::DuplicateArgument(flag));
    }

    *slot = Some(value);
    Ok(())
}

fn parse_environment(value: &str) -> Result<StoreEnvironment, CliError> {
    match value {
        "development" => Ok(StoreEnvironment::Development),
        "production" => Ok(StoreEnvironment::Production),
        _ => Err(CliError::InvalidEnvironment(value.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{CliCommand, CliError, StorageCommand, parse_cli_args};
    use library_boundary_service::StoreEnvironment;

    #[test]
    fn parses_space_separated_required_args() {
        let config = parse_cli_args([
            "--user-data-path".to_string(),
            "C:/Dekzer".to_string(),
            "--env".to_string(),
            "development".to_string(),
        ])
        .expect("parse args");

        assert_eq!(
            config,
            CliCommand::Serve(super::ServerConfig {
                user_data_path: "C:/Dekzer".to_string(),
                environment: StoreEnvironment::Development,
            })
        );
    }

    #[test]
    fn parses_equals_separated_required_args() {
        let config = parse_cli_args([
            "--env=production".to_string(),
            "--user-data-path=C:/Dekzer".to_string(),
        ])
        .expect("parse args");

        assert_eq!(
            config,
            CliCommand::Serve(super::ServerConfig {
                user_data_path: "C:/Dekzer".to_string(),
                environment: StoreEnvironment::Production,
            })
        );
    }

    #[test]
    fn rejects_missing_user_data_path() {
        let error = parse_cli_args(["--env=development".to_string()])
            .expect_err("missing user data path is rejected");

        assert_eq!(error, CliError::MissingRequired("--user-data-path"));
    }

    #[test]
    fn rejects_missing_env() {
        let error = parse_cli_args(["--user-data-path=C:/Dekzer".to_string()])
            .expect_err("missing env is rejected");

        assert_eq!(error, CliError::MissingRequired("--env"));
    }

    #[test]
    fn rejects_invalid_env() {
        let error = parse_cli_args([
            "--user-data-path=C:/Dekzer".to_string(),
            "--env=test".to_string(),
        ])
        .expect_err("invalid env is rejected");

        assert_eq!(error, CliError::InvalidEnvironment("test".to_string()));
    }

    #[test]
    fn parses_storage_status_user_data_root() {
        let command = parse_cli_args([
            "storage".to_string(),
            "status".to_string(),
            "--user-data".to_string(),
            "C:/Dekzer".to_string(),
        ])
        .expect("parse storage status");

        assert_eq!(
            command,
            CliCommand::Storage(StorageCommand::Status(super::StorageConfig {
                user_data_path: "C:/Dekzer".to_string(),
            }))
        );
    }

    #[test]
    fn parses_storage_reset_confirmation() {
        let command = parse_cli_args([
            "storage".to_string(),
            "reset".to_string(),
            "--user-data=C:/Dekzer".to_string(),
            "--confirm-delete".to_string(),
        ])
        .expect("parse storage reset");

        assert_eq!(
            command,
            CliCommand::Storage(StorageCommand::Reset(super::StorageResetConfig {
                user_data_path: "C:/Dekzer".to_string(),
                confirm_delete: true,
            }))
        );
    }

    #[test]
    fn rejects_unknown_storage_subcommand() {
        let error = parse_cli_args(["storage".to_string(), "inspect".to_string()])
            .expect_err("unknown storage subcommand is rejected");

        assert_eq!(
            error,
            CliError::UnknownStorageSubcommand("inspect".to_string())
        );
    }
}
