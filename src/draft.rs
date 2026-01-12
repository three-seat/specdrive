use crate::Result;
use crate::fsutil;
use crate::utils;
use std::fmt;
use std::fs;
use std::path::PathBuf;

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
        return Err(DraftError::new("FEATURE_ID cannot be empty".to_string(), 1));
    }

    // 2. Preflight checks: git repo, .specify/, clean tree
    // Per F-004 refactor: use shared helper and map structured errors
    utils::ensure_repo_and_specify_ready().map_err(|e| DraftError::new(e.to_string(), 1))?;

    // 3. Resolve and validate spec and contract paths
    // Per F-004 refactor: use FeaturePaths helper
    let feature_paths = fsutil::FeaturePaths::new(feature_id);
    feature_paths.validate().map_err(|e| match e {
        fsutil::FeaturePathError::MissingSpec(_) => {
            DraftError::new(format!("{}. Feature {} does not exist.", e, feature_id), 2)
        }
        fsutil::FeaturePathError::MissingContract(_) => DraftError::new(
            format!(
                "contract skeleton file not found: {}. Feature {} does not exist.",
                feature_paths.contract.display(),
                feature_id
            ),
            2,
        ),
    })?;

    // 4. Validate required template files exist
    // Per F-004 refactor: use TemplatePaths helper
    let template_paths = fsutil::TemplatePaths::new();
    template_paths
        .validate()
        .map_err(|e| DraftError::new(format!("{}. Run 'specdrive bootstrap' first.", e), 2))?;

    // 5. Discover optional supporting docs
    // Per F-004 refactor: use fsutil helpers for optional docs discovery
    let constitution = fsutil::find_constitution();
    let system_overview = fsutil::find_system_overview();
    let adr_files = fsutil::find_adrs();

    // Validate we can read optional docs that exist
    if let Some(path) = constitution.path() {
        fs::read_to_string(path).map_err(|e| {
            DraftError::new(
                format!("failed to read constitution file {}: {}", path.display(), e),
                2,
            )
        })?;
    }

    if let Some(path) = system_overview.path() {
        fs::read_to_string(path).map_err(|e| {
            DraftError::new(
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
            DraftError::new(
                format!("failed to read ADR file {}: {}", adr_path.display(), e),
                2,
            )
        })?;
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
        &feature_paths,
        &template_paths,
        &constitution,
        &system_overview,
        &adr_files,
        header.as_deref(),
        footer.as_deref(),
    );

    println!("{}", prompt);

    Ok(())
}

fn build_draft_prompt(
    feature_id: &str,
    feature_paths: &fsutil::FeaturePaths,
    template_paths: &fsutil::TemplatePaths,
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
        "You are drafting/refining the contract {} for feature {}.\n\n",
        feature_paths.contract.display(),
        feature_id
    ));
    prompt.push_str(
        "The spec, ADRs, constitution, and system overview define the behaviour and constraints.\n",
    );
    prompt.push_str("Use the appropriate contract template (minimal vs critical).\n\n");

    // Files to read
    prompt.push_str("Files you MUST read before drafting the contract:\n");
    prompt.push_str(&format!(
        "- {} (feature spec)\n",
        feature_paths.spec.display()
    ));
    prompt.push_str(&format!(
        "- {} (current or skeleton)\n",
        feature_paths.contract.display()
    ));

    if let Some(path) = constitution.path() {
        prompt.push_str(&format!("- {}\n", path.display()));
    }

    if !adr_files.is_empty() {
        prompt.push_str("- Architecture Decision Records (ADRs):\n");
        for adr in adr_files {
            prompt.push_str(&format!("  - {}\n", adr.display()));
        }
    }

    if let Some(path) = system_overview.path() {
        prompt.push_str(&format!("- {}\n", path.display()));
    }

    prompt.push_str(&format!(
        "- {} (minimal template)\n",
        template_paths.minimal.display()
    ));
    prompt.push_str(&format!(
        "- {} (critical template)\n",
        template_paths.critical.display()
    ));

    prompt.push('\n');

    // Guidance section
    prompt.push_str("Guidance for drafting the contract:\n");
    prompt.push_str(
        "- Map the spec's behavior, context, and acceptance criteria to the contract sections:\n",
    );
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
    prompt.push_str(
        "- Follow the contract template structure (minimal or critical as appropriate)\n",
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
