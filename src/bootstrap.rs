//! Bootstrap command: prepares a git repo for specdrive usage.
//!
//! Per ADR-002 / F-007, bootstrap initializes only SpecDrive-owned directories
//! and templates under `docs/`. SpecDrive no longer depends on Spec Kit or
//! `.specify/`.
//!
//! Behavior:
//! - Verifies `.git/` exists (exit 1 if not)
//! - Creates standard SpecDrive directories if missing
//! - Installs template files from embedded assets if missing
//! - Never overwrites existing files
//! - Exits with code 0 (success), 1 (precondition failure), or 2 (filesystem error)

use std::fmt;
use std::fs;
use std::path::Path;

/// Embedded template: feature spec template
const FEATURE_SPEC_TEMPLATE: &str = include_str!("../assets/bootstrap/feature.spec.md");

/// Embedded template: minimal contract template
const FEATURE_CONTRACT_MINIMAL_TEMPLATE: &str =
    include_str!("../assets/bootstrap/feature.contract.minimal.yaml");

/// Embedded template: critical contract template
const FEATURE_CONTRACT_CRITICAL_TEMPLATE: &str =
    include_str!("../assets/bootstrap/feature.contract.critical.yaml");

/// Bootstrap-specific errors with exit code semantics.
#[derive(Debug)]
pub enum BootstrapError {
    /// Precondition failure (not a git repo) -> exit code 1
    Precondition(String),
    /// Filesystem error (failed to create dir or write file) -> exit code 2
    Filesystem(String),
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootstrapError::Precondition(msg) => write!(f, "{}", msg),
            BootstrapError::Filesystem(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for BootstrapError {}

impl BootstrapError {
    pub fn exit_code(&self) -> i32 {
        match self {
            BootstrapError::Precondition(_) => 1,
            BootstrapError::Filesystem(_) => 2,
        }
    }
}

pub type Result<T> = std::result::Result<T, BootstrapError>;

#[derive(Default)]
struct BootstrapSummary {
    created_dirs: Vec<String>,
    created_files: Vec<String>,
    skipped_files: Vec<String>,
}

impl BootstrapSummary {
    fn print(&self) {
        println!("Bootstrap complete!");
        if !self.created_dirs.is_empty() {
            println!("\nCreated directories:");
            for dir in &self.created_dirs {
                println!("  {}", dir);
            }
        }
        if !self.created_files.is_empty() {
            println!("\nCreated files:");
            for file in &self.created_files {
                println!("  {}", file);
            }
        }
        if !self.skipped_files.is_empty() {
            println!("\nSkipped (already exist):");
            for file in &self.skipped_files {
                println!("  {}", file);
            }
        }
    }
}

pub fn run() -> Result<()> {
    verify_git_repo()?;

    let mut summary = BootstrapSummary::default();

    ensure_directories(&mut summary)?;
    install_templates(&mut summary)?;

    summary.print();
    Ok(())
}

/// Per ADR-002 / F-007, bootstrap only requires that we are inside a git repo.
/// `.specify/` is no longer required for normal SpecDrive operation.
fn verify_git_repo() -> Result<()> {
    if !Path::new(".git").exists() {
        return Err(BootstrapError::Precondition(
            "Not a git repository. Please run this command from the root of a git repo."
                .to_string(),
        ));
    }
    Ok(())
}

/// Ensure all required SpecDrive-owned directories exist.
///
/// Per ADR-002 / F-007, bootstrap creates only `docs/`-rooted directories;
/// it does not create feature-local `prompts/` or `outputs/` directories.
fn ensure_directories(summary: &mut BootstrapSummary) -> Result<()> {
    let dirs = ["docs", "docs/features", "docs/templates"];

    for dir in &dirs {
        let path = Path::new(dir);
        if !path.exists() {
            fs::create_dir_all(path).map_err(|e| {
                BootstrapError::Filesystem(format!("Failed to create directory {}: {}", dir, e))
            })?;
            summary.created_dirs.push(dir.to_string());
        }
    }

    Ok(())
}

/// Install template files from embedded assets if they don't exist.
/// Never overwrites existing files.
fn install_templates(summary: &mut BootstrapSummary) -> Result<()> {
    write_template_if_missing(
        "docs/templates/feature.spec.md",
        FEATURE_SPEC_TEMPLATE,
        summary,
    )?;

    write_template_if_missing(
        "docs/templates/feature.contract.minimal.yaml",
        FEATURE_CONTRACT_MINIMAL_TEMPLATE,
        summary,
    )?;

    write_template_if_missing(
        "docs/templates/feature.contract.critical.yaml",
        FEATURE_CONTRACT_CRITICAL_TEMPLATE,
        summary,
    )?;

    Ok(())
}

fn write_template_if_missing(
    path: &str,
    content: &str,
    summary: &mut BootstrapSummary,
) -> Result<()> {
    let target = Path::new(path);

    if target.exists() {
        summary.skipped_files.push(path.to_string());
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                BootstrapError::Filesystem(format!(
                    "Failed to create parent directory for {}: {}",
                    path, e
                ))
            })?;
        }
    }

    fs::write(target, content).map_err(|e| {
        BootstrapError::Filesystem(format!("Failed to write template to {}: {}", path, e))
    })?;

    summary.created_files.push(path.to_string());
    Ok(())
}
