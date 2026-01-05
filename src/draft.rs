use crate::Result;
use crate::utils;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Custom error type for the draft command with exit codes
#[derive(Debug)]
pub struct DraftError {
    message: String,
    exit_code: i32,
}

impl DraftError {
    fn new(message: String, exit_code: i32) -> Self {
        Self { message, exit_code }
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl fmt::Display for DraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DraftError {}

/// Main entry point for the draft command
pub fn draft_feature(feature_id: &str) -> Result<()> {
    // Convert all errors to DraftError to get proper exit codes
    match draft_feature_inner(feature_id) {
        Ok(()) => Ok(()),
        Err(e) => {
            let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
            Err(err)
        }
    }
}

fn draft_feature_inner(feature_id: &str) -> std::result::Result<(), DraftError> {
    // 1. Validate feature_id is non-empty
    if feature_id.trim().is_empty() {
        return Err(DraftError::new(
            "FEATURE_ID cannot be empty".to_string(),
            1,
        ));
    }

    // 2. Preflight checks: git repo, .specify/, clean tree
    utils::ensure_repo_and_specify_ready().map_err(|e| DraftError::new(e.to_string(), 1))?;

    // 3. Resolve and validate spec and contract paths
    let spec_path = PathBuf::from(".specify")
        .join("specs")
        .join(format!("{}.spec.md", feature_id));
    let contract_path = PathBuf::from("docs")
        .join("features")
        .join(feature_id)
        .join("contract.yaml");

    if !spec_path.exists() {
        return Err(DraftError::new(
            format!(
                "spec file not found: {}. Feature {} does not exist.",
                spec_path.display(),
                feature_id
            ),
            2,
        ));
    }

    if !contract_path.exists() {
        return Err(DraftError::new(
            format!(
                "contract skeleton file not found: {}. Feature {} does not exist.",
                contract_path.display(),
                feature_id
            ),
            2,
        ));
    }

    // 4. Validate required template files exist
    let minimal_template_path = PathBuf::from("docs/templates/feature.contract.minimal.yaml");
    let critical_template_path = PathBuf::from("docs/templates/feature.contract.critical.yaml");

    if !minimal_template_path.exists() {
        return Err(DraftError::new(
            format!(
                "required template not found: {}. Run 'specdrive bootstrap' first.",
                minimal_template_path.display()
            ),
            2,
        ));
    }

    if !critical_template_path.exists() {
        return Err(DraftError::new(
            format!(
                "required template not found: {}. Run 'specdrive bootstrap' first.",
                critical_template_path.display()
            ),
            2,
        ));
    }

    // 5. Detect optional supporting docs
    let constitution_path = PathBuf::from(".specify/memory/constitution.md");
    if constitution_path.exists() {
        fs::read_to_string(&constitution_path).map_err(|e| {
            DraftError::new(
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
            DraftError::new(
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
            DraftError::new(
                format!(
                    "failed to read ADRs directory {}: {}",
                    adrs_dir.display(),
                    e
                ),
                2,
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                DraftError::new(
                    format!("failed to read ADR entry in {}: {}", adrs_dir.display(), e),
                    2,
                )
            })?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Validate we can read it
                fs::read_to_string(&path).map_err(|e| {
                    DraftError::new(
                        format!("failed to read ADR file {}: {}", path.display(), e),
                        2,
                    )
                })?;
                adr_files.push(path);
            }
        }
    }

    // 6. Read optional header and footer
    let header_path = PathBuf::from("docs/ai/draft-header.md");
    let header = if header_path.exists() {
        Some(fs::read_to_string(&header_path).map_err(|e| {
            DraftError::new(
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

    let footer_path = PathBuf::from("docs/ai/draft-footer.md");
    let footer = if footer_path.exists() {
        Some(fs::read_to_string(&footer_path).map_err(|e| {
            DraftError::new(
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

    // 7. Build and print the prompt
    let prompt = build_draft_prompt(
        feature_id,
        &spec_path,
        &contract_path,
        &constitution_path,
        &system_overview_path,
        &minimal_template_path,
        &critical_template_path,
        &adr_files,
        header.as_deref(),
        footer.as_deref(),
    );

    println!("{}", prompt);

    Ok(())
}

fn build_draft_prompt(
    feature_id: &str,
    spec_path: &Path,
    contract_path: &Path,
    constitution_path: &Path,
    system_overview_path: &Path,
    minimal_template_path: &Path,
    critical_template_path: &Path,
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
        "You are drafting/refining the contract {} for feature {}.\n\n",
        contract_path.display(),
        feature_id
    ));
    prompt.push_str(
        "The spec, ADRs, constitution, and system overview define the behaviour and constraints.\n",
    );
    prompt.push_str("Use the appropriate contract template (minimal vs critical).\n\n");

    // Files to read
    prompt.push_str("Files you MUST read before drafting the contract:\n");
    prompt.push_str(&format!("- {} (feature spec)\n", spec_path.display()));
    prompt.push_str(&format!(
        "- {} (current or skeleton)\n",
        contract_path.display()
    ));

    if constitution_path.exists() {
        prompt.push_str(&format!("- {}\n", constitution_path.display()));
    }

    if !adr_files.is_empty() {
        prompt.push_str("- Architecture Decision Records (ADRs):\n");
        for adr in adr_files {
            prompt.push_str(&format!("  - {}\n", adr.display()));
        }
    }

    if system_overview_path.exists() {
        prompt.push_str(&format!("- {}\n", system_overview_path.display()));
    }

    prompt.push_str(&format!(
        "- {} (minimal template)\n",
        minimal_template_path.display()
    ));
    prompt.push_str(&format!(
        "- {} (critical template)\n",
        critical_template_path.display()
    ));

    prompt.push('\n');

    // Guidance section
    prompt.push_str("Guidance for drafting the contract:\n");
    prompt.push_str("- Map the spec's behavior, context, and acceptance criteria to the contract sections:\n");
    prompt.push_str("  - requirements (high_level and low_level)\n");
    prompt.push_str("  - behavior (steps)\n");
    prompt.push_str("  - logic (invariants, error_conditions)\n");
    prompt.push_str("  - filesystem (creates_paths, reads_paths, must_not_modify)\n");
    prompt.push_str("  - git_safety (require_clean_tree, allow_untracked)\n");
    prompt.push_str("  - verification (test_cases)\n");
    prompt.push_str("  - ai_instructions\n");
    prompt.push_str("- For critical features:\n");
    prompt.push_str("  - Ensure metadata.critical: true\n");
    prompt.push_str("  - Add stronger invariants and git safety requirements\n");
    prompt.push_str("  - Add appropriate review expectations in reviews section\n");
    prompt.push('\n');

    // Guardrails
    prompt.push_str("Guardrails:\n");
    prompt.push_str("- Do NOT weaken invariants or lower safety properties\n");
    prompt.push_str("- Do NOT change feature IDs or basic layout\n");
    prompt.push_str("- Keep the contract structured and consistent with existing examples\n");
    prompt.push_str("- Follow the contract template structure (minimal or critical as appropriate)\n");

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
