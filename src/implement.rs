use crate::Result;
use crate::utils;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Custom error type for the implement command with exit codes
#[derive(Debug)]
pub struct ImplementError {
    message: String,
    exit_code: i32,
}

impl ImplementError {
    fn new(message: String, exit_code: i32) -> Self {
        Self {
            message,
            exit_code,
        }
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

    // 2. Preflight checks: git repo, .specify/, clean tree
    utils::ensure_repo_and_specify_ready().map_err(|e| ImplementError::new(e.to_string(), 1))?;

    // 3. Resolve and validate spec and contract paths
    let spec_path = PathBuf::from(".specify")
        .join("specs")
        .join(format!("{}.spec.md", feature_id));
    let contract_path = PathBuf::from("docs")
        .join("features")
        .join(feature_id)
        .join("contract.yaml");

    if !spec_path.exists() {
        return Err(ImplementError::new(
            format!(
                "spec file not found: {}. Feature {} does not exist.",
                spec_path.display(),
                feature_id
            ),
            2,
        ));
    }

    if !contract_path.exists() {
        return Err(ImplementError::new(
            format!(
                "contract file not found: {}. Feature {} does not exist.",
                contract_path.display(),
                feature_id
            ),
            2,
        ));
    }

    // 4. Read and parse contract YAML
    let contract_text = fs::read_to_string(&contract_path).map_err(|e| {
        ImplementError::new(
            format!("failed to read contract file {}: {}", contract_path.display(), e),
            2,
        )
    })?;

    let contract: serde_yaml::Value = serde_yaml::from_str(&contract_text).map_err(|e| {
        ImplementError::new(
            format!(
                "failed to parse contract YAML {}: {}",
                contract_path.display(),
                e
            ),
            2,
        )
    })?;

    // 5. Check critical feature review gate
    if let Some(metadata) = contract.get("metadata") {
        if let Some(critical) = metadata.get("critical") {
            if critical.as_bool().unwrap_or(false) {
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
        }
    }

    // 6. Validate supporting docs (constitution, system overview, ADRs)
    let constitution_path = PathBuf::from(".specify/memory/constitution.md");
    if constitution_path.exists() {
        fs::read_to_string(&constitution_path).map_err(|e| {
            ImplementError::new(
                format!(
                    "failed to read constitution file {}: {}",
                    constitution_path.display(),
                    e
                ),
                2,
            )
        })?;
    }

    let system_overview_path = PathBuf::from("docs/system-overview.md");
    if system_overview_path.exists() {
        fs::read_to_string(&system_overview_path).map_err(|e| {
            ImplementError::new(
                format!(
                    "failed to read system overview file {}: {}",
                    system_overview_path.display(),
                    e
                ),
                2,
            )
        })?;
    }

    let adrs_dir = PathBuf::from("docs/adrs");
    let mut adr_files = Vec::new();
    if adrs_dir.exists() && adrs_dir.is_dir() {
        let entries = fs::read_dir(&adrs_dir).map_err(|e| {
            ImplementError::new(
                format!("failed to read ADRs directory {}: {}", adrs_dir.display(), e),
                2,
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                ImplementError::new(
                    format!("failed to read ADR entry in {}: {}", adrs_dir.display(), e),
                    2,
                )
            })?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Validate we can read it
                fs::read_to_string(&path).map_err(|e| {
                    ImplementError::new(
                        format!("failed to read ADR file {}: {}", path.display(), e),
                        2,
                    )
                })?;
                adr_files.push(path);
            }
        }
    }

    // 7. Read optional header and footer
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

    // 8. Build and print the prompt
    let prompt = build_prompt(
        feature_id,
        &spec_path,
        &contract_path,
        &constitution_path,
        &system_overview_path,
        &adr_files,
        header.as_deref(),
        footer.as_deref(),
    );

    println!("{}", prompt);

    Ok(())
}

fn build_prompt(
    feature_id: &str,
    spec_path: &Path,
    contract_path: &Path,
    constitution_path: &Path,
    system_overview_path: &Path,
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
    prompt.push_str(&format!("- {}\n", spec_path.display()));
    prompt.push_str(&format!("- {}\n", contract_path.display()));

    if constitution_path.exists() {
        prompt.push_str(&format!("- {}\n", constitution_path.display()));
    }

    if system_overview_path.exists() {
        prompt.push_str(&format!("- {}\n", system_overview_path.display()));
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
        "- Follow all behaviour, invariants, and filesystem/git rules from the contract.\n",
    );
    prompt.push_str("- Do not modify spec, contract, ADRs, or constitution.\n");
    prompt.push_str("- Do not add dependencies beyond what the contract allows.\n");
    prompt.push_str("- Prefer small, focused functions with explicit error handling.\n");
    prompt.push_str("- This CLI command is read-only: it must not create, modify, or delete files.\n");

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
