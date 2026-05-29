use crate::Result;
use crate::config;
use crate::fsutil;
use crate::utils;
use std::fmt;
use std::fs;
use std::path::PathBuf;

/// Custom error type for the implement command with exit codes
#[derive(Debug)]
pub struct ImplementError {
    message: String,
    exit_code: i32,
}

impl ImplementError {
    fn new(message: String, exit_code: i32) -> Self {
        Self { message, exit_code }
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl fmt::Display for ImplementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ImplementError {}

/// Main entry point for the implement command
pub fn implement_feature(feature_id: &str) -> Result<()> {
    // Convert all errors to ImplementError to get proper exit codes
    match implement_feature_inner(feature_id) {
        Ok(()) => Ok(()),
        Err(e) => {
            let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
            Err(err)
        }
    }
}

fn implement_feature_inner(feature_id: &str) -> std::result::Result<(), ImplementError> {
    // 1. Validate feature_id is non-empty
    if feature_id.trim().is_empty() {
        return Err(ImplementError::new(
            "FEATURE_ID cannot be empty".to_string(),
            1,
        ));
    }

    // 2. Per F-005 contract: validate FEATURE_ID against safety rules and config pattern
    config::validate_feature_id(feature_id)
        .map_err(|e| ImplementError::new(e.to_string(), e.exit_code()))?;

    // 3. Preflight checks: git repo + clean tree
    // Per ADR-002 / F-007, .specify/ is no longer required.
    utils::ensure_repo_ready().map_err(|e| ImplementError::new(e.to_string(), 1))?;

    // 4. Resolve and validate spec and contract paths
    // Per F-004 refactor: use FeaturePaths helper
    let feature_paths = fsutil::FeaturePaths::new(feature_id);
    feature_paths.validate().map_err(|e| {
        ImplementError::new(format!("{}. Feature {} does not exist.", e, feature_id), 2)
    })?;

    // 5. Read and parse contract YAML
    let contract_text = fs::read_to_string(&feature_paths.contract).map_err(|e| {
        ImplementError::new(
            format!(
                "failed to read contract file {}: {}",
                feature_paths.contract.display(),
                e
            ),
            2,
        )
    })?;

    let contract: serde_yaml::Value = serde_yaml::from_str(&contract_text).map_err(|e| {
        ImplementError::new(
            format!(
                "failed to parse contract YAML {}: {}",
                feature_paths.contract.display(),
                e
            ),
            2,
        )
    })?;

    // 6. Check critical feature review gate
    if let Some(metadata) = contract.get("metadata")
        && let Some(critical) = metadata.get("critical")
        && critical.as_bool().unwrap_or(false)
    {
        // This is a critical feature - check review status
        let reviewed_by = contract
            .get("reviews")
            .and_then(|r| r.get("status"))
            .and_then(|s| s.get("reviewed_by"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let reviewed_at = contract
            .get("reviews")
            .and_then(|r| r.get("status"))
            .and_then(|s| s.get("reviewed_at"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if reviewed_by.is_empty() || reviewed_at.is_empty() {
            return Err(ImplementError::new(
                format!(
                    "critical feature {} has not been reviewed: reviews.status.reviewed_by and reviews.status.reviewed_at must be populated",
                    feature_id
                ),
                1,
            ));
        }
    }

    // 7. Discover optional supporting docs
    // Per F-004 refactor: use fsutil helpers for optional docs discovery
    let constitution = fsutil::find_constitution();
    let system_overview = fsutil::find_system_overview();
    let adr_files = fsutil::find_adrs();

    // Validate we can read optional docs that exist
    if let Some(path) = constitution.path() {
        fs::read_to_string(path).map_err(|e| {
            ImplementError::new(
                format!("failed to read constitution file {}: {}", path.display(), e),
                2,
            )
        })?;
    }

    if let Some(path) = system_overview.path() {
        fs::read_to_string(path).map_err(|e| {
            ImplementError::new(
                format!(
                    "failed to read system overview file {}: {}",
                    path.display(),
                    e
                ),
                2,
            )
        })?;
    }

    for adr_path in &adr_files {
        fs::read_to_string(adr_path).map_err(|e| {
            ImplementError::new(
                format!("failed to read ADR file {}: {}", adr_path.display(), e),
                2,
            )
        })?;
    }

    // 8. Read optional header and footer
    let header_path = PathBuf::from("docs/ai/implement-header.md");
    let header = if header_path.exists() {
        Some(fs::read_to_string(&header_path).map_err(|e| {
            ImplementError::new(
                format!(
                    "failed to read header file {}: {}",
                    header_path.display(),
                    e
                ),
                2,
            )
        })?)
    } else {
        None
    };

    let footer_path = PathBuf::from("docs/ai/implement-footer.md");
    let footer = if footer_path.exists() {
        Some(fs::read_to_string(&footer_path).map_err(|e| {
            ImplementError::new(
                format!(
                    "failed to read footer file {}: {}",
                    footer_path.display(),
                    e
                ),
                2,
            )
        })?)
    } else {
        None
    };

    // 9. Build and print the prompt
    let prompt = build_prompt(
        feature_id,
        &feature_paths,
        &constitution,
        &system_overview,
        &adr_files,
        header.as_deref(),
        footer.as_deref(),
    );

    println!("{}", prompt);

    Ok(())
}

fn build_prompt(
    feature_id: &str,
    feature_paths: &fsutil::FeaturePaths,
    constitution: &fsutil::OptionalDoc,
    system_overview: &fsutil::OptionalDoc,
    adr_files: &[PathBuf],
    header: Option<&str>,
    footer: Option<&str>,
) -> String {
    let mut prompt = String::new();

    // Optional header
    if let Some(h) = header {
        prompt.push_str(h);
        if !h.ends_with('\n') {
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    // Built-in intro
    prompt.push_str(&format!(
        "You are implementing feature {} in this repository.\n\n",
        feature_id
    ));
    prompt.push_str(
        "The spec and contract are the source of truth. Do NOT modify them unless explicitly instructed.\n\n",
    );

    // Files to read
    prompt.push_str("Files you MUST read before coding:\n");
    prompt.push_str(&format!("- {}\n", feature_paths.spec.display()));
    prompt.push_str(&format!("- {}\n", feature_paths.contract.display()));

    if let Some(path) = constitution.path() {
        prompt.push_str(&format!("- {}\n", path.display()));
    }

    if let Some(path) = system_overview.path() {
        prompt.push_str(&format!("- {}\n", path.display()));
    }

    if !adr_files.is_empty() {
        prompt.push_str("- Architecture Decision Records (ADRs):\n");
        for adr in adr_files {
            prompt.push_str(&format!("  - {}\n", adr.display()));
        }
    }

    prompt.push('\n');

    // Built-in guardrails
    prompt.push_str("Guardrails:\n");
    prompt.push_str(
        "- Follow all behavior, invariants, and filesystem/git rules from the contract.\n",
    );
    prompt.push_str("- Do not modify spec, contract, ADRs, or constitution.\n");
    prompt.push_str("- Do not add dependencies beyond what the contract allows.\n");
    prompt.push_str("- Prefer small, focused functions with explicit error handling.\n");
    prompt.push_str(
        "- This CLI command is read-only: it must not create, modify, or delete files.\n",
    );

    // Optional footer
    if let Some(f) = footer {
        prompt.push('\n');
        if !prompt.ends_with('\n') {
            prompt.push('\n');
        }
        prompt.push_str(f);
        if !f.ends_with('\n') {
            prompt.push('\n');
        }
    }

    prompt
}
