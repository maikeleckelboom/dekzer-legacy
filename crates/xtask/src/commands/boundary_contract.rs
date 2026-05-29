use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const CONTRACT_DIR: &str = "packages/library-boundary-contract";
const PACKAGE_NAME: &str = "@dekzer/library-boundary-contract";
const GENERATOR: &str = "xtask export-boundary-contract";
const INDEX_TS: &str = "index.ts";
const SCHEMA_JSON: &str = "boundary-contract.schema.json";
const MANIFEST_JSON: &str = "manifest.json";
const PACKAGE_JSON: &str = "package.json";
const TSCONFIG_JSON: &str = "tsconfig.json";
const TSCONFIG_BUILD_JSON: &str = "tsconfig.build.json";

pub fn run_export_cli<I>(args: I) -> Result<(), XtaskError>
where
    I: IntoIterator<Item = String>,
{
    ensure_no_args(args, "export-boundary-contract")?;
    let root = workspace_root()?;
    export_boundary_contract_to(&root)?;
    println!("[xtask] exported boundary contract to {CONTRACT_DIR}");
    Ok(())
}

pub fn run_check_cli<I>(args: I) -> Result<(), XtaskError>
where
    I: IntoIterator<Item = String>,
{
    ensure_no_args(args, "check-boundary-contract")?;
    let root = workspace_root()?;
    check_boundary_contract_at(&root)?;
    println!("[xtask] boundary contract artifacts are current");
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

fn export_boundary_contract_to(workspace_root: &Path) -> Result<(), XtaskError> {
    let artifacts = generate_artifacts()?;
    let contract_dir = workspace_root.join(CONTRACT_DIR);
    fs::create_dir_all(&contract_dir).map_err(XtaskError::io)?;

    for artifact in artifacts {
        fs::write(contract_dir.join(artifact.path), artifact.contents).map_err(XtaskError::io)?;
    }

    Ok(())
}

fn check_boundary_contract_at(workspace_root: &Path) -> Result<(), XtaskError> {
    let artifacts = generate_artifacts()?;
    let contract_dir = workspace_root.join(CONTRACT_DIR);

    for artifact in artifacts {
        let path = contract_dir.join(artifact.path);
        let actual = fs::read_to_string(&path)
            .map_err(|error| XtaskError::stale(format!("{}: {error}", path.display())))?;
        if actual != artifact.contents {
            return Err(XtaskError::stale(format!(
                "{} is stale; run `cargo run -p xtask -- export-boundary-contract`",
                path.display()
            )));
        }
    }

    Ok(())
}

fn generate_artifacts() -> Result<Vec<Artifact>, XtaskError> {
    let index_ts = library_boundary_protocol::generated_contract_index_ts();
    let schema_json = library_boundary_protocol::generated_contract_schema_json()
        .map_err(|error| XtaskError::state(error.to_string()))?;
    let package_json = generated_package_json()?;
    let tsconfig_json = generated_tsconfig_json()?;
    let tsconfig_build_json = generated_tsconfig_build_json()?;
    let manifest_json = generated_manifest_json(
        &index_ts,
        &schema_json,
        &package_json,
        &tsconfig_json,
        &tsconfig_build_json,
    )?;

    Ok(vec![
        Artifact::new(INDEX_TS, index_ts),
        Artifact::new(SCHEMA_JSON, schema_json),
        Artifact::new(MANIFEST_JSON, manifest_json),
        Artifact::new(PACKAGE_JSON, package_json),
        Artifact::new(TSCONFIG_JSON, tsconfig_json),
        Artifact::new(TSCONFIG_BUILD_JSON, tsconfig_build_json),
    ])
}

fn generated_package_json() -> Result<String, XtaskError> {
    let package = serde_json::json!({
        "name": PACKAGE_NAME,
        "version": library_boundary_protocol::protocol_version(),
        "private": true,
        "type": "module",
        "main": "./dist/index.js",
        "types": "./dist/index.d.ts",
        "exports": {
            ".": {
                "types": "./dist/index.d.ts",
                "default": "./dist/index.js"
            }
        },
        "sideEffects": false,
        "generatedOnly": true,
        "scripts": {
            "build": "tsc -p tsconfig.build.json",
            "typecheck": "tsc -p tsconfig.json --noEmit"
        },
        "devDependencies": {
            "typescript": "^5.9.3"
        },
        "files": [
            "dist",
            "boundary-contract.schema.json",
            "manifest.json",
            "package.json",
            "tsconfig.json",
            "tsconfig.build.json"
        ]
    });
    to_pretty_json(&package)
}

fn generated_tsconfig_json() -> Result<String, XtaskError> {
    let config = serde_json::json!({
        "compilerOptions": {
            "forceConsistentCasingInFileNames": true,
            "isolatedModules": true,
            "module": "NodeNext",
            "moduleResolution": "NodeNext",
            "noEmit": true,
            "skipLibCheck": true,
            "strict": true,
            "target": "ES2022",
            "verbatimModuleSyntax": true
        },
        "include": [
            "index.ts"
        ]
    });
    to_pretty_json(&config)
}

fn generated_tsconfig_build_json() -> Result<String, XtaskError> {
    let config = serde_json::json!({
        "extends": "./tsconfig.json",
        "compilerOptions": {
            "declaration": true,
            "declarationMap": false,
            "noEmit": false,
            "outDir": "./dist",
            "rootDir": "."
        },
        "include": [
            "index.ts"
        ]
    });
    to_pretty_json(&config)
}

fn generated_manifest_json(
    index_ts: &str,
    schema_json: &str,
    package_json: &str,
    tsconfig_json: &str,
    tsconfig_build_json: &str,
) -> Result<String, XtaskError> {
    let artifact_inputs = [
        (INDEX_TS, index_ts),
        (SCHEMA_JSON, schema_json),
        (PACKAGE_JSON, package_json),
        (TSCONFIG_JSON, tsconfig_json),
        (TSCONFIG_BUILD_JSON, tsconfig_build_json),
    ];
    let artifacts = artifact_inputs
        .iter()
        .map(|(path, contents)| {
            serde_json::json!({
                "path": path,
                "sha256": sha256_hex(contents.as_bytes())
            })
        })
        .collect::<Vec<_>>();

    let manifest = serde_json::json!({
        "generatedOnly": true,
        "generator": GENERATOR,
        "protocolCrateName": library_boundary_protocol::protocol_crate_name(),
        "protocolVersion": library_boundary_protocol::protocol_version(),
        "packageName": PACKAGE_NAME,
        "sourceLayout": "index.ts is generated at the package root",
        "artifactHash": artifact_fingerprint(&artifact_inputs),
        "artifacts": artifacts
    });

    to_pretty_json(&manifest)
}

fn to_pretty_json(value: &serde_json::Value) -> Result<String, XtaskError> {
    let mut json = serde_json::to_string_pretty(value)
        .map_err(|error| XtaskError::state(error.to_string()))?;
    json.push('\n');
    Ok(json)
}

fn artifact_fingerprint(artifacts: &[(&str, &str)]) -> String {
    let mut hasher = Sha256::new();
    for (path, contents) in artifacts {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(contents.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

struct Artifact {
    path: &'static str,
    contents: String,
}

impl Artifact {
    fn new(path: &'static str, contents: String) -> Self {
        Self { path, contents }
    }
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
                "[xtask] usage: cargo run -p xtask -- <export-boundary-contract|check-boundary-contract>"
                    .to_string(),
                format!("[xtask] detail: {detail}"),
                "[xtask] error: invalid boundary contract invocation".to_string(),
            ],
            Self::State(detail) => vec![
                format!("[xtask] detail: {detail}"),
                "[xtask] error: boundary contract command failed".to_string(),
            ],
            Self::Stale(detail) => vec![
                format!("[xtask] detail: {detail}"),
                "[xtask] error: generated boundary contract is stale".to_string(),
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
        INDEX_TS, MANIFEST_JSON, PACKAGE_JSON, PACKAGE_NAME, SCHEMA_JSON, TSCONFIG_BUILD_JSON,
        TSCONFIG_JSON, check_boundary_contract_at, export_boundary_contract_to, generate_artifacts,
    };

    #[test]
    fn generated_artifacts_include_expected_contract_package_files() {
        let artifacts = generate_artifacts().expect("generate artifacts");
        let paths = artifacts
            .iter()
            .map(|artifact| artifact.path)
            .collect::<Vec<_>>();

        assert!(paths.contains(&INDEX_TS));
        assert!(paths.contains(&SCHEMA_JSON));
        assert!(paths.contains(&MANIFEST_JSON));
        assert!(paths.contains(&PACKAGE_JSON));
        assert!(paths.contains(&TSCONFIG_JSON));
        assert!(paths.contains(&TSCONFIG_BUILD_JSON));
    }

    #[test]
    fn generated_typescript_contains_protocol_contract_exports_and_commands() {
        let artifacts = generate_artifacts().expect("generate artifacts");
        let ts = artifact_contents(&artifacts, INDEX_TS);

        assert!(ts.contains("export type CommandRequest"));
        assert!(ts.contains("export type CommandReply"));
        assert!(ts.contains("export type CommandOutcome"));
        assert!(ts.contains("export type ProtocolError"));
        assert!(ts.contains("libraryBoundaryEvents"));
        assert!(ts.contains("registerLocalRoot"));
        assert!(ts.contains("startRootScan"));
        assert!(ts.contains("readLibraryTreeChildren"));
    }

    #[test]
    fn generated_package_metadata_uses_dekzer_boundary_contract_name() {
        let artifacts = generate_artifacts().expect("generate artifacts");
        let package_json = artifact_contents(&artifacts, PACKAGE_JSON);
        let package: serde_json::Value =
            serde_json::from_str(package_json).expect("parse package json");

        assert_eq!(
            package.pointer("/name"),
            Some(&serde_json::json!(PACKAGE_NAME))
        );
        assert_eq!(package.pointer("/private"), Some(&serde_json::json!(true)));
        assert_eq!(
            package.pointer("/sideEffects"),
            Some(&serde_json::json!(false))
        );
    }

    #[test]
    fn generated_active_artifacts_contain_no_retired_vocabulary() {
        let artifacts = generate_artifacts().expect("generate artifacts");
        let retired_vocabulary = ["sur", "face"].concat();

        for artifact in artifacts {
            let lower_contents = artifact.contents.to_ascii_lowercase();
            assert!(
                !lower_contents.contains(&retired_vocabulary),
                "{} contains forbidden retired vocabulary",
                artifact.path
            );
        }
    }

    #[test]
    fn generated_artifacts_are_identical_across_repeated_generation() {
        let first = generate_artifacts().expect("generate first artifact set");
        let second = generate_artifacts().expect("generate second artifact set");

        assert_eq!(first.len(), second.len());
        for (first, second) in first.iter().zip(second.iter()) {
            assert_eq!(first.path, second.path);
            assert_eq!(first.contents, second.contents);
        }
    }

    #[test]
    fn generated_manifest_is_deterministic_without_git_commit_hash() {
        let artifacts = generate_artifacts().expect("generate artifacts");
        let manifest_json = artifact_contents(&artifacts, MANIFEST_JSON);
        let manifest: serde_json::Value =
            serde_json::from_str(manifest_json).expect("parse manifest json");
        let artifact_entries = manifest
            .pointer("/artifacts")
            .and_then(|artifacts| artifacts.as_array())
            .expect("manifest artifacts");

        assert!(manifest.pointer("/gitCommitHash").is_none());
        assert!(manifest.pointer("/artifactHash").is_some());
        assert!(!artifact_entries.is_empty());
        for artifact in artifact_entries {
            assert!(artifact.pointer("/path").is_some());
            assert!(artifact.pointer("/sha256").is_some());
        }
    }

    #[test]
    fn check_boundary_contract_passes_immediately_after_export() {
        let tempdir = TempDir::new().expect("tempdir");

        export_boundary_contract_to(tempdir.path()).expect("export contract");

        check_boundary_contract_at(tempdir.path()).expect("fresh contract passes");
    }

    #[test]
    fn check_boundary_contract_detects_stale_artifacts() {
        let tempdir = TempDir::new().expect("tempdir");
        export_boundary_contract_to(tempdir.path()).expect("export contract");

        fs::write(
            tempdir
                .path()
                .join("packages/library-boundary-contract")
                .join(INDEX_TS),
            "stale\n",
        )
        .expect("make artifact stale");

        let error =
            check_boundary_contract_at(tempdir.path()).expect_err("stale artifact should fail");
        assert!(
            error
                .render_lines()
                .iter()
                .any(|line| line.contains("generated boundary contract is stale"))
        );
    }

    fn artifact_contents<'a>(artifacts: &'a [super::Artifact], path: &str) -> &'a str {
        artifacts
            .iter()
            .find(|artifact| artifact.path == path)
            .unwrap_or_else(|| panic!("missing artifact {path}"))
            .contents
            .as_str()
    }
}
