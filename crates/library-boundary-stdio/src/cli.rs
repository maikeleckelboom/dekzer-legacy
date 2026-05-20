use library_boundary_service::StoreEnvironment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerConfig {
    pub(crate) user_data_path: String,
    pub(crate) environment: StoreEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliError {
    DuplicateArgument(&'static str),
    InvalidEnvironment(String),
    MissingRequired(&'static str),
    MissingValue(&'static str),
    UnknownArgument(String),
}

impl CliError {
    pub(crate) fn render_lines(&self) -> Vec<String> {
        vec![
            format!("[library-boundary-stdio] detail: {self}"),
            "[library-boundary-stdio] error: invalid stdio server invocation".to_string(),
            "[library-boundary-stdio] usage: library-boundary-stdio --user-data-path <path> --env <development|production>".to_string(),
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
            Self::MissingValue(flag) => {
                write!(f, "missing value for {flag}")
            }
            Self::UnknownArgument(argument) => {
                write!(f, "unknown argument {argument:?}")
            }
        }
    }
}

pub(crate) fn parse_cli_args<I>(args: I) -> Result<ServerConfig, CliError>
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
    use super::{CliError, parse_cli_args};
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

        assert_eq!(config.user_data_path, "C:/Dekzer");
        assert_eq!(config.environment, StoreEnvironment::Development);
    }

    #[test]
    fn parses_equals_separated_required_args() {
        let config = parse_cli_args([
            "--env=production".to_string(),
            "--user-data-path=C:/Dekzer".to_string(),
        ])
        .expect("parse args");

        assert_eq!(config.user_data_path, "C:/Dekzer");
        assert_eq!(config.environment, StoreEnvironment::Production);
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
}
