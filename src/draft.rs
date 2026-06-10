use crate::Result;
use crate::config;
use crate::fsutil;
use crate::resolve::{self, FileRole};
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

struct DraftPromptContext<'a> {
    feature_id: &'a str,
    feature_paths: &'a fsutil::FeaturePaths,
    template_paths: &'a fsutil::TemplatePaths,
    system_overview: &'a fsutil::OptionalDoc,
    constitution: &'a fsutil::OptionalDoc,
    adr_files: &'a [PathBuf],
    header: Option<&'a str>,
    footer: Option<&'a str>,
}

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

    // 2. Per F-005 contract: validate FEATURE_ID against safety rules and config pattern
    config::validate_feature_id(feature_id)
        .map_err(|e| DraftError::new(e.to_string(), e.exit_code()))?;

    // 3. Preflight checks: git repo + clean tree
    // Per ADR-002 / F-007, .specify/ is no longer required.
    utils::ensure_repo_ready().map_err(|e| DraftError::new(e.to_string(), 1))?;

    // 4. Resolve and validate spec and contract paths
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

    // 5. Validate required template files exist
    // Per F-004 refactor: use TemplatePaths helper
    let template_paths = fsutil::TemplatePaths::new();
    template_paths
        .validate()
        .map_err(|e| DraftError::new(format!("{}. Run 'specdrive bootstrap' first.", e), 2))?;

    // 6. Discover optional supporting docs
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

    // 7. Read optional header and footer
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

    // 8. Build and print the prompt
    let ctx = DraftPromptContext {
        feature_id,
        feature_paths: &feature_paths,
        template_paths: &template_paths,
        system_overview: &system_overview,
        constitution: &constitution,
        adr_files: &adr_files,
        header: header.as_deref(),
        footer: footer.as_deref(),
    };

    let prompt = build_draft_prompt(&ctx);

    println!("{}", prompt);

    Ok(())
}

fn build_draft_prompt(ctx: &DraftPromptContext<'_>) -> String {
    let mut prompt = String::new();

    if let Some(h) = ctx.header {
        prompt.push_str(h);
        if !h.ends_with('\n') {
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt.push_str(&format!(
        "You are drafting/refining the contract {} for feature {}.\n\n",
        ctx.feature_paths.contract.display(),
        ctx.feature_id
    ));

    prompt.push_str(
        "The spec, ADRs, constitution, and system overview define the behaviour and constraints.\n",
    );
    prompt.push_str("Use the appropriate contract template (minimal vs critical).\n\n");

    // The set of context files is sourced from the shared draft resolver
    // (F-009, AC-13) so the existing read-validation, this prompt, and
    // `chat export` all agree on which files make up the draft context. The
    // ctx fields below are retained for read-validation in the caller.
    let _ = (
        &ctx.template_paths,
        &ctx.system_overview,
        &ctx.constitution,
        &ctx.adr_files,
    );

    prompt.push_str("Files you MUST read before drafting the contract:\n");
    let mut adr_header_written = false;
    for file in resolve::resolve_draft_files(ctx.feature_id) {
        match file.role {
            FileRole::Spec => {
                prompt.push_str(&format!("- {} (feature spec)\n", file.path.display()));
            }
            FileRole::Contract => {
                prompt.push_str(&format!(
                    "- {} (current or skeleton)\n",
                    file.path.display()
                ));
            }
            FileRole::Constitution | FileRole::SystemOverview => {
                if file.path.exists() {
                    prompt.push_str(&format!("- {}\n", file.path.display()));
                }
            }
            FileRole::Adr => {
                if !adr_header_written {
                    prompt.push_str("- Architecture Decision Records (ADRs):\n");
                    adr_header_written = true;
                }
                prompt.push_str(&format!("  - {}\n", file.path.display()));
            }
            FileRole::MinimalTemplate => {
                prompt.push_str(&format!("- {} (minimal template)\n", file.path.display()));
            }
            FileRole::CriticalTemplate => {
                prompt.push_str(&format!("- {} (critical template)\n", file.path.display()));
            }
        }
    }

    prompt.push('\n');

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

    prompt.push_str("Guardrails:\n");
    prompt.push_str("- Do NOT weaken invariants or lower safety properties\n");
    prompt.push_str("- Do NOT change feature IDs or basic layout\n");
    prompt.push_str("- Keep the contract structured and consistent with existing examples\n");
    prompt.push_str(
        "- Follow the contract template structure (minimal or critical as appropriate)\n",
    );

    if let Some(f) = ctx.footer {
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
