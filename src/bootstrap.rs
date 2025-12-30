//! Bootstrap command: prepares an existing repo for specdrive usage.
//!
//! Per F-001 contract, this command:
//! - Verifies .git/ and .specify/ exist (exit 1 if not)
//! - Creates standard directories if missing
//! - Installs template files from embedded assets if missing
//! - Never overwrites existing files (per LLR-007)
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
    /// Precondition failure (not a git repo, .specify/ missing) -> exit code 1
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
    /// Returns the exit code for this error per F-001 contract.
    pub fn exit_code(&self) -> i32 {
        match self {
            BootstrapError::Precondition(_) => 1,
            BootstrapError::Filesystem(_) => 2,
        }
    }
}

pub type Result<T> = std::result::Result<T, BootstrapError>;

/// Summary of what was created vs skipped during bootstrap.
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

/// Main entry point for the bootstrap command.
pub fn run() -> Result<()> {
    // Per F-001 contract: verify prerequisites first
    verify_git_repo()?;
    verify_specify_dir()?;

    let mut summary = BootstrapSummary::default();

    // Per F-001 contract: ensure standard directories exist
    ensure_directories(&mut summary)?;

    // Per F-001 contract: install template files if missing (never overwrite per LLR-007)
    install_templates(&mut summary)?;

    summary.print();
    Ok(())
}

/// Verify that .git/ exists; per F-001 contract, exit code 1 if missing.
fn verify_git_repo() -> Result<()> {
    if !Path::new(".git").exists() {
        return Err(BootstrapError::Precondition(
            "Not a git repository. Please run this command from the root of a git repo."
                .to_string(),
        ));
    }
    Ok(())
}

/// Verify that .specify/ exists; per F-001 contract, exit code 1 if missing.
fn verify_specify_dir() -> Result<()> {
    if !Path::new(".specify").exists() {
        return Err(BootstrapError::Precondition(
            ".specify/ directory not found. Please run 'specify init' first.".to_string(),
        ));
    }
    Ok(())
}

/// Ensure all required directories exist; per F-001 contract.
fn ensure_directories(summary: &mut BootstrapSummary) -> Result<()> {
    let dirs = [
        ".specify/specs",
        ".specify/templates",
        "docs",
        "docs/features",
        "docs/templates",
    ];

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
/// Per F-001 LLR-007: never overwrite existing files.
fn install_templates(summary: &mut BootstrapSummary) -> Result<()> {
    // Template: .specify/templates/feature.spec.md
    write_template_if_missing(
        ".specify/templates/feature.spec.md",
        FEATURE_SPEC_TEMPLATE,
        summary,
    )?;

    // Template: docs/templates/feature.contract.minimal.yaml
    write_template_if_missing(
        "docs/templates/feature.contract.minimal.yaml",
        FEATURE_CONTRACT_MINIMAL_TEMPLATE,
        summary,
    )?;

    // Template: docs/templates/feature.contract.critical.yaml
    write_template_if_missing(
        "docs/templates/feature.contract.critical.yaml",
        FEATURE_CONTRACT_CRITICAL_TEMPLATE,
        summary,
    )?;

    Ok(())
}

/// Write a template file if it doesn't exist; skip if it does (per LLR-007).
fn write_template_if_missing(
    path: &str,
    content: &str,
    summary: &mut BootstrapSummary,
) -> Result<()> {
    let target = Path::new(path);

    if target.exists() {
        // Per F-001 LLR-007: never overwrite existing files
        summary.skipped_files.push(path.to_string());
        return Ok(());
    }

    // Ensure parent directory exists (should already be created by ensure_directories)
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

    // Write the template file
    fs::write(target, content).map_err(|e| {
        BootstrapError::Filesystem(format!("Failed to write template to {}: {}", path, e))
    })?;

    summary.created_files.push(path.to_string());
    Ok(())
}
