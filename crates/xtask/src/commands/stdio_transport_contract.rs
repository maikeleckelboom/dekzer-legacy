use std::fs;
use std::path::{Path, PathBuf};

const STDIO_CONTRACT_ARTIFACT: &str =
    "packages/library-boundary-stdio-transport/src/generated/stdioEnvelope.ts";

pub fn run_export_cli<I>(args: I) -> Result<(), XtaskError>
where
    I: IntoIterator<Item = String>,
{
    ensure_no_args(args, "export-stdio-transport-contract")?;
    let root = workspace_root()?;
    export_stdio_transport_contract_to(&root)?;
    println!("[xtask] exported stdio transport contract to {STDIO_CONTRACT_ARTIFACT}");
    Ok(())
}

pub fn run_check_cli<I>(args: I) -> Result<(), XtaskError>
where
    I: IntoIterator<Item = String>,
{
    ensure_no_args(args, "check-stdio-transport-contract")?;
    let root = workspace_root()?;
    check_stdio_transport_contract_at(&root)?;
    println!("[xtask] stdio transport contract artifact is current");
    Ok(())
}

fn ensure_no_args<I>(args: I, command: &str) -> Result<(), XtaskError>
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args.len() == 1 && args[0] == command {
        Ok(())
    } else {
        Err(XtaskError::usage(format!(
            "{command} does not accept additional arguments"
        )))
    }
}

fn workspace_root() -> Result<PathBuf, XtaskError> {
    std::env::current_dir().map_err(|error| XtaskError::state(error.to_string()))
}

fn export_stdio_transport_contract_to(workspace_root: &Path) -> Result<(), XtaskError> {
    let artifact = generate_artifact();
    let path = workspace_root.join(artifact.path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(XtaskError::io)?;
    }
    fs::write(path, artifact.contents).map_err(XtaskError::io)
}

fn check_stdio_transport_contract_at(workspace_root: &Path) -> Result<(), XtaskError> {
    let artifact = generate_artifact();
    let path = workspace_root.join(artifact.path);
    let actual = fs::read_to_string(&path)
        .map_err(|error| XtaskError::stale(format!("{}: {error}", path.display())))?;
    if actual != artifact.contents {
        return Err(XtaskError::stale(format!(
            "{} is stale; run `cargo run -p xtask -- export-stdio-transport-contract`",
            path.display()
        )));
    }

    Ok(())
}

fn generate_artifact() -> Artifact {
    Artifact {
        path: STDIO_CONTRACT_ARTIFACT,
        contents: library_boundary_stdio::stdio_contract::generated_stdio_envelope_ts(),
    }
}

struct Artifact {
    path: &'static str,
    contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XtaskError {
    Usage(String),
    State(String),
    Stale(String),
}

impl XtaskError {
    fn usage(detail: impl Into<String>) -> Self {
        Self::Usage(detail.into())
    }

    fn state(detail: impl Into<String>) -> Self {
        Self::State(detail.into())
    }

    fn stale(detail: impl Into<String>) -> Self {
        Self::Stale(detail.into())
    }

    fn io(error: std::io::Error) -> Self {
        Self::State(error.to_string())
    }

    pub fn render_lines(&self) -> Vec<String> {
        match self {
            Self::Usage(detail) => vec![
                "[xtask] usage: cargo run -p xtask -- <export-boundary-contract|check-boundary-contract|export-stdio-transport-contract|check-stdio-transport-contract>"
                    .to_string(),
                format!("[xtask] detail: {detail}"),
                "[xtask] error: invalid stdio transport contract invocation".to_string(),
            ],
            Self::State(detail) => vec![
                format!("[xtask] detail: {detail}"),
                "[xtask] error: stdio transport contract command failed".to_string(),
            ],
            Self::Stale(detail) => vec![
                format!("[xtask] detail: {detail}"),
                "[xtask] error: generated stdio transport contract is stale".to_string(),
            ],
        }
    }
}

impl std::fmt::Display for XtaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render_lines().join("\n"))
    }
}

impl std::error::Error for XtaskError {}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        STDIO_CONTRACT_ARTIFACT, check_stdio_transport_contract_at,
        export_stdio_transport_contract_to, generate_artifact,
    };

    #[test]
    fn generated_artifact_has_expected_path_and_remote_code_exports() {
        let artifact = generate_artifact();

        assert_eq!(artifact.path, STDIO_CONTRACT_ARTIFACT);
        assert!(
            artifact
                .contents
                .contains("export const libraryBoundaryStdioRemoteErrorCodes")
        );
        assert!(artifact.contents.contains("\"invalidFrame\""));
        assert!(!artifact.contents.contains("serviceOpenFailure"));
    }

    #[test]
    fn generated_stdio_transport_contract_is_deterministic() {
        assert_eq!(generate_artifact().contents, generate_artifact().contents);
    }

    #[test]
    fn check_stdio_transport_contract_passes_immediately_after_export() {
        let tempdir = TempDir::new().expect("tempdir");

        export_stdio_transport_contract_to(tempdir.path()).expect("export contract");

        check_stdio_transport_contract_at(tempdir.path()).expect("fresh contract passes");
    }

    #[test]
    fn check_stdio_transport_contract_detects_stale_artifact() {
        let tempdir = TempDir::new().expect("tempdir");
        export_stdio_transport_contract_to(tempdir.path()).expect("export contract");

        fs::write(tempdir.path().join(STDIO_CONTRACT_ARTIFACT), "stale\n")
            .expect("make artifact stale");

        let error = check_stdio_transport_contract_at(tempdir.path())
            .expect_err("stale artifact should fail");
        assert!(
            error
                .render_lines()
                .iter()
                .any(|line| line.contains("generated stdio transport contract is stale"))
        );
    }
}
