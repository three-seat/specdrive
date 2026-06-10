//! `specdrive chat export <draft|implement> <FEATURE_ID>` (F-009).
//!
//! Assembles a self-contained, delimited context bundle from the files resolved
//! for the workflow and prints it to stdout. Export is strictly read-only: it
//! performs no clean-tree check and must succeed even on a dirty working tree
//! (LLR-022, AC-18).

use std::fs;

use super::{BEGIN, ChatError, END, FILE_PREFIX, FILE_SUFFIX, NOTES, PROMPT, Workflow};
use crate::fsutil;
use crate::resolve;

/// Runs the export workflow.
pub fn run(workflow: Workflow, feature_id: &str) -> std::result::Result<(), ChatError> {
    // The feature must exist and have both spec and contract (E-002).
    let feature_paths = fsutil::FeaturePaths::new(feature_id);
    feature_paths
        .validate()
        .map_err(|e| ChatError::usage(format!("{}. Feature {} not found.", e, feature_id)))?;

    // Export context is determined solely by the resolver (LLR-001, AC-16).
    let files = match workflow {
        Workflow::Draft => resolve::resolve_draft_files(feature_id),
        Workflow::Implement => resolve::resolve_implement_files(feature_id),
    };

    let mut bundle = String::new();

    bundle.push_str(BEGIN);
    bundle.push('\n');

    // Empty NOTES block — the AI fills this in on response.
    bundle.push_str(NOTES);
    bundle.push('\n');
    bundle.push('\n');

    // One FILE block per resolved file, with inlined contents.
    for file in &files {
        match fs::read_to_string(&file.path) {
            Ok(contents) => {
                bundle.push_str(FILE_PREFIX);
                bundle.push_str(&file.path.display().to_string());
                bundle.push_str(FILE_SUFFIX);
                bundle.push('\n');
                bundle.push_str(&contents);
                // Guarantee the next delimiter starts on its own line.
                if !contents.ends_with('\n') {
                    bundle.push('\n');
                }
            }
            Err(e) => {
                if file.required {
                    // Spec or contract unreadable — fail fast (E-002).
                    return Err(ChatError::io(format!(
                        "failed to read required file {}: {}",
                        file.path.display(),
                        e
                    )));
                }
                // Optional context missing — warn and continue (E-003, LLR-004).
                eprintln!(
                    "warning: skipping missing context file {}",
                    file.path.display()
                );
            }
        }
    }

    // PROMPT block with the inline-files export prompt.
    bundle.push_str(PROMPT);
    bundle.push('\n');
    bundle.push_str(&build_export_prompt(workflow, feature_id));
    bundle.push('\n');

    bundle.push_str(END);
    bundle.push('\n');

    print!("{}", bundle);

    // Usage hint goes to stderr so the bundle on stdout stays clean and
    // copy-pasteable.
    eprintln!();
    eprintln!("Copy the above and paste into your AI chat tool.");

    Ok(())
}

/// Builds the export prompt embedded in the PROMPT block (LLR-003).
///
/// The prompt provides files inline (no path references the AI must read) and
/// embeds the delimiter guardrail instructing the AI to respond in SpecDrive
/// delimited format.
fn build_export_prompt(workflow: Workflow, feature_id: &str) -> String {
    let mut p = String::new();

    match workflow {
        Workflow::Draft => {
            p.push_str(&format!(
                "You are drafting/refining the contract for feature {}.\n\n",
                feature_id
            ));
            p.push_str(
                "All required context (the feature spec, current/skeleton contract, \
                 constitution, ADRs, system overview, and contract templates) is provided \
                 inline above in SPECDRIVE:FILE blocks. Do not ask for file paths — every \
                 file you need is already inlined.\n\n",
            );
            p.push_str("Produce the complete, updated contract for this feature.\n\n");
        }
        Workflow::Implement => {
            p.push_str(&format!("You are implementing feature {}.\n\n", feature_id));
            p.push_str(
                "All required context (the feature spec, contract, constitution, ADRs, and \
                 system overview) is provided inline above in SPECDRIVE:FILE blocks. Do not ask \
                 for file paths — every file you need is already inlined.\n\n",
            );
            p.push_str(
                "Treat the spec and contract as the source of truth. Do not weaken invariants, \
                 lower safety properties, or add dependencies the contract does not allow.\n\n",
            );
        }
    }

    p.push_str("Respond ONLY in SpecDrive delimited format:\n");
    p.push_str(&format!("  {}\n", BEGIN));
    p.push_str(&format!(
        "  {} (put any notes, caveats, or open questions here)\n",
        NOTES
    ));
    p.push_str(&format!(
        "  {}<repo-relative path under docs/features/{}/>{}\n",
        FILE_PREFIX, feature_id, FILE_SUFFIX
    ));
    p.push_str("  ...file contents inline...\n");
    p.push_str("  (repeat a FILE block for each file you are returning)\n");
    p.push_str(&format!("  {}\n\n", END));

    match workflow {
        Workflow::Draft => {
            p.push_str(&format!(
                "Return the full updated contract as a single FILE block whose path is \
                 docs/features/{}/contract.yaml. It must be valid YAML.\n",
                feature_id
            ));
        }
        Workflow::Implement => {
            p.push_str(
                "Use FILE blocks for the files you change. Paths must be repository-relative \
                 and remain within the feature directory.\n",
            );
        }
    }

    p.push_str(&format!(
        "Every path must be relative to the repository root and resolve within \
         docs/features/{}/. End your response with the {} line.\n",
        feature_id, END
    ));

    p
}
