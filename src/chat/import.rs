//! `specdrive chat import <draft|implement> <FEATURE_ID>` (F-009).
//!
//! Reads a delimited AI response from stdin, validates it completely (a dry
//! pass with no filesystem modification), previews the changes, and writes
//! artifacts only on explicit confirmation. Import is the security-critical
//! half of F-009: it accepts AI-supplied paths and contents and writes them to
//! disk, so path traversal, absolute paths, symlink escape, delimiter
//! injection, and oversized responses are all rejected before anything is
//! written (HLR-003).

use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use super::{ChatError, END, Workflow, parse_response, validate_file_path};
use crate::config::{self, ChatImportLimits};
use crate::fsutil;
use crate::git;

/// Runs the import workflow.
pub fn run(workflow: Workflow, feature_id: &str) -> std::result::Result<(), ChatError> {
    // The feature must exist and have both spec and contract (E-002).
    let feature_paths = fsutil::FeaturePaths::new(feature_id);
    feature_paths
        .validate()
        .map_err(|e| ChatError::usage(format!("{}. Feature {} not found.", e, feature_id)))?;

    // Import enforces its OWN clean-tree gate in code, before any write and
    // independent of the shared preflight (LLR-023, P-004, E-004). Untracked
    // files are allowed (git_safety.allow_untracked: true).
    ensure_clean_tree()?;

    // Size limits, with safe built-in defaults (LLR-024, LLR-025).
    let limits = config::load_chat_import_limits();

    let stdin = std::io::stdin();
    let mut reader = stdin.lock();

    println!("Paste AI response. Waiting for {}", END);
    println!();

    // Read until the first END, enforcing the total response size limit
    // (E-005 missing END; E-009 oversized).
    let raw = read_until_end(&mut reader, limits.max_response_size_bytes)?;

    println!("Parsing response...");
    println!();

    // --- Dry validation pass: nothing is written until this all succeeds and
    // the user confirms (LLR-006, AC-14). ---
    let parsed = parse_response(&raw);
    validate_blocks(&parsed, &limits)?;

    let cwd = std::env::current_dir()
        .map_err(|e| ChatError::io(format!("failed to determine working directory: {}", e)))?;

    // Validate every FILE path for containment (E-007, E-008) regardless of
    // workflow.
    let mut validated: Vec<PathBuf> = Vec::with_capacity(parsed.files.len());
    for file in &parsed.files {
        let path = validate_file_path(&feature_paths.dir, &file.path)?;
        validated.push(path);
    }

    let _ = &validated; // path containment validated above for every block.

    // Workflow-specific dry validation.
    let plan = match workflow {
        Workflow::Draft => plan_draft(&feature_paths, &parsed, &cwd)?,
        Workflow::Implement => plan_implement(&feature_paths),
    };

    // --- Preview ---
    if let Some(notes) = &parsed.notes {
        println!("Notes from AI:");
        for line in notes.lines() {
            println!("  {}", line);
        }
        println!();
    }

    println!("Files to be written:");
    for item in &plan {
        println!("  {:<48} ({})", item.display_path(), item.change_summary);
    }
    println!();

    // --- Confirmation ---
    print!("Apply? [y/N]: ");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    if !confirm(&mut reader) {
        println!();
        println!("Declined. Nothing written.");
        return Ok(()); // E-013: clean exit, nothing written.
    }
    println!();

    // --- Writes (only reached after full validation + confirmation) ---
    match workflow {
        Workflow::Draft => apply_draft(&feature_paths, &parsed, &plan)?,
        Workflow::Implement => apply_implement(&feature_paths, &raw, &plan)?,
    }

    println!("Done.");
    Ok(())
}

/// In-code clean-tree gate for import (LLR-023, E-004).
fn ensure_clean_tree() -> std::result::Result<(), ChatError> {
    if !Path::new(".git").exists() {
        return Err(ChatError::usage(
            "Not a git repository. Please run this command from the root of a git repo.",
        ));
    }
    // allow_untracked = true per git_safety.allow_untracked.
    git::ensure_git_clean(true).map_err(|_| {
        ChatError::usage(
            "git working tree is not clean: commit or stash your changes before importing",
        )
    })
}

/// Reads stdin lines until the first line that is exactly the END delimiter,
/// returning the raw text read (including the END line). Enforces the total
/// response size limit incrementally so an unbounded stream cannot exhaust
/// memory.
fn read_until_end<R: BufRead>(
    reader: &mut R,
    max_response_size_bytes: u64,
) -> std::result::Result<String, ChatError> {
    let mut raw = String::new();
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| ChatError::io(format!("failed to read stdin: {}", e)))?;

        if n == 0 {
            // EOF before END — incomplete paste (E-005).
            return Err(ChatError::usage(format!(
                "incomplete paste: never saw the '{}' delimiter. \
                 Re-paste the full AI response including the final END line.",
                END
            )));
        }

        raw.push_str(&line);

        if raw.len() as u64 > max_response_size_bytes {
            // Reject before parsing (E-009).
            return Err(ChatError::usage(format!(
                "response exceeds the maximum allowed size of {} bytes",
                max_response_size_bytes
            )));
        }

        let logical = line.trim_end_matches(['\n', '\r']);
        if logical == END {
            break;
        }
    }
    Ok(raw)
}

/// Reads a single confirmation line. Anything other than `y`/`yes`
/// (case-insensitive) is treated as a decline.
fn confirm<R: BufRead>(reader: &mut R) -> bool {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) | Err(_) => false,
        Ok(_) => {
            let ans = line.trim().to_ascii_lowercase();
            ans == "y" || ans == "yes"
        }
    }
}

/// Validates block counts and sizes (E-006, E-010).
fn validate_blocks(
    parsed: &super::ParsedResponse,
    limits: &ChatImportLimits,
) -> std::result::Result<(), ChatError> {
    if parsed.files.is_empty() {
        return Err(ChatError::usage(
            "malformed response: no SPECDRIVE:FILE blocks found",
        ));
    }
    if parsed.files.len() > limits.max_file_blocks {
        return Err(ChatError::usage(format!(
            "response has {} FILE blocks, exceeding the maximum of {}",
            parsed.files.len(),
            limits.max_file_blocks
        )));
    }
    for file in &parsed.files {
        if file.contents.len() as u64 > limits.max_file_size_bytes {
            return Err(ChatError::usage(format!(
                "FILE block '{}' is {} bytes, exceeding the maximum of {} bytes",
                file.path,
                file.contents.len(),
                limits.max_file_size_bytes
            )));
        }
    }
    Ok(())
}

/// A planned write for the preview and apply stages.
struct PlannedWrite {
    path: PathBuf,
    change_summary: String,
    /// Content to write, when known at planning time (draft contract). For
    /// numbered artifacts the path is resolved at apply time.
    content: Option<String>,
    kind: WriteKind,
}

enum WriteKind {
    Contract,
    Notes,
    ImplementRaw,
}

impl PlannedWrite {
    fn display_path(&self) -> String {
        self.path.display().to_string()
    }
}

/// Builds and validates the draft write plan (LLR-009, LLR-019; E-011).
fn plan_draft(
    feature_paths: &fsutil::FeaturePaths,
    parsed: &super::ParsedResponse,
    cwd: &Path,
) -> std::result::Result<Vec<PlannedWrite>, ChatError> {
    // Locate the FILE block that targets this feature's contract.yaml. The
    // contract exists, so canonicalization gives a reliable identity.
    let contract_canon = feature_paths.contract.canonicalize().map_err(|e| {
        ChatError::io(format!(
            "failed to canonicalize contract {}: {}",
            feature_paths.contract.display(),
            e
        ))
    })?;

    let mut contract_idx: Option<usize> = None;
    for (idx, file) in parsed.files.iter().enumerate() {
        let on_disk = cwd.join(&file.path);
        if let Ok(canon) = on_disk.canonicalize()
            && canon == contract_canon
        {
            contract_idx = Some(idx);
            break;
        }
    }

    let Some(idx) = contract_idx else {
        return Err(ChatError::usage(format!(
            "draft response did not include a FILE block for {}",
            feature_paths.contract.display()
        )));
    };

    let new_contents = parsed.files[idx].contents.clone();

    // Validate parseable YAML with the required top-level contract structure
    // (E-011, LLR-019). Full schema validation is deferred to F-019.
    validate_contract_structure(&new_contents)?;

    // Change count for the preview.
    let old_contents = fs::read_to_string(&feature_paths.contract).unwrap_or_default();
    let changes = count_line_changes(&old_contents, &new_contents);

    let mut plan = vec![PlannedWrite {
        path: feature_paths.contract.clone(),
        change_summary: format!("{} changes", changes),
        content: Some(new_contents),
        kind: WriteKind::Contract,
    }];

    // Notes artifact, only if a NOTES block is present (LLR-009, LLR-012).
    if parsed.notes.is_some() {
        let outputs = feature_paths.dir.join("outputs");
        let name = fsutil::next_numbered_filename(&outputs, "notes-", ".md");
        plan.push(PlannedWrite {
            path: outputs.join(name),
            change_summary: "1 file".to_string(),
            content: None, // notes content written from parsed.notes at apply time
            kind: WriteKind::Notes,
        });
    }

    Ok(plan)
}

/// Builds the implement write plan (LLR-010). The raw response is saved to a
/// single numbered artifact; no source or patch files are touched.
fn plan_implement(feature_paths: &fsutil::FeaturePaths) -> Vec<PlannedWrite> {
    let outputs = feature_paths.dir.join("outputs");
    let name = fsutil::next_numbered_filename(&outputs, "implement-", ".raw.md");
    vec![PlannedWrite {
        path: outputs.join(name),
        change_summary: "1 file".to_string(),
        content: None,
        kind: WriteKind::ImplementRaw,
    }]
}

/// Applies the draft plan: replace contract.yaml, then write notes if present.
fn apply_draft(
    feature_paths: &fsutil::FeaturePaths,
    parsed: &super::ParsedResponse,
    plan: &[PlannedWrite],
) -> std::result::Result<(), ChatError> {
    for item in plan {
        match item.kind {
            WriteKind::Contract => {
                let content = item.content.as_deref().unwrap_or_default();
                fs::write(&item.path, content).map_err(|e| {
                    ChatError::io(format!(
                        "failed to write contract {}: {}",
                        item.path.display(),
                        e
                    ))
                })?;
                println!("Wrote {}", item.path.display());
            }
            WriteKind::Notes => {
                ensure_parent_dir(&item.path)?;
                let notes = parsed.notes.as_deref().unwrap_or_default();
                fs::write(&item.path, notes).map_err(|e| {
                    ChatError::io(format!(
                        "failed to write notes {}: {}",
                        item.path.display(),
                        e
                    ))
                })?;
                println!("Wrote {}", item.path.display());
            }
            WriteKind::ImplementRaw => {}
        }
    }
    let _ = feature_paths;
    Ok(())
}

/// Applies the implement plan: save the raw response to outputs/ only. Never
/// modifies source code or patch artifacts (LLR-010, AC-15).
fn apply_implement(
    _feature_paths: &fsutil::FeaturePaths,
    raw: &str,
    plan: &[PlannedWrite],
) -> std::result::Result<(), ChatError> {
    for item in plan {
        if let WriteKind::ImplementRaw = item.kind {
            ensure_parent_dir(&item.path)?;
            fs::write(&item.path, raw).map_err(|e| {
                ChatError::io(format!(
                    "failed to write output {}: {}",
                    item.path.display(),
                    e
                ))
            })?;
            println!("Wrote {}", item.path.display());
        }
    }
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> std::result::Result<(), ChatError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            ChatError::io(format!(
                "failed to create directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }
    Ok(())
}

/// Validates that draft content is parseable YAML with the required top-level
/// contract structure (LLR-019, E-011). Full schema validation is F-019.
fn validate_contract_structure(content: &str) -> std::result::Result<(), ChatError> {
    let value: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|e| ChatError::usage(format!("draft contract is not valid YAML: {}", e)))?;

    let mapping = value.as_mapping().ok_or_else(|| {
        ChatError::usage("draft contract must be a YAML mapping at the top level")
    })?;

    for required in ["metadata", "requirements"] {
        if !mapping.contains_key(serde_yaml::Value::String(required.to_string())) {
            return Err(ChatError::usage(format!(
                "draft contract is missing the required top-level '{}' section",
                required
            )));
        }
    }

    Ok(())
}

/// Counts changed lines between two texts as a line-multiset symmetric
/// difference. Used only for the human-facing change-count preview (Q1: V1
/// shows change counts only).
fn count_line_changes(old: &str, new: &str) -> usize {
    let mut counts: HashMap<&str, i64> = HashMap::new();
    for line in old.lines() {
        *counts.entry(line).or_insert(0) += 1;
    }
    for line in new.lines() {
        *counts.entry(line).or_insert(0) -= 1;
    }
    counts.values().map(|v| v.unsigned_abs() as usize).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn limits() -> ChatImportLimits {
        ChatImportLimits::defaults()
    }

    #[test]
    fn test_read_until_end_ok() {
        let input = format!("line1\nline2\n{}\nignored after\n", END);
        let mut cur = Cursor::new(input);
        let raw = read_until_end(&mut cur, 1_000_000).unwrap();
        assert!(raw.contains("line1"));
        assert!(raw.trim_end().ends_with(END));
        assert!(!raw.contains("ignored after"));
    }

    #[test]
    fn test_read_until_end_missing_end() {
        let input = "line1\nline2\n".to_string();
        let mut cur = Cursor::new(input);
        let err = read_until_end(&mut cur, 1_000_000).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.to_string().contains("incomplete paste"));
    }

    #[test]
    fn test_read_until_end_oversized() {
        let big = "x".repeat(100);
        let input = format!("{}\n{}\n", big, END);
        let mut cur = Cursor::new(input);
        let err = read_until_end(&mut cur, 10).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.to_string().contains("maximum"));
    }

    #[test]
    fn test_confirm_variants() {
        for yes in ["y\n", "Y\n", "yes\n", "YES\n", " y \n"] {
            let mut cur = Cursor::new(yes.to_string());
            assert!(confirm(&mut cur), "expected yes for {:?}", yes);
        }
        for no in ["n\n", "no\n", "\n", "anything\n", ""] {
            let mut cur = Cursor::new(no.to_string());
            assert!(!confirm(&mut cur), "expected no for {:?}", no);
        }
    }

    #[test]
    fn test_validate_blocks_no_files() {
        let parsed = super::super::ParsedResponse {
            notes: None,
            files: vec![],
        };
        let err = validate_blocks(&parsed, &limits()).unwrap_err();
        assert!(err.to_string().contains("no SPECDRIVE:FILE blocks"));
    }

    #[test]
    fn test_validate_blocks_too_many() {
        let files = (0..30)
            .map(|i| super::super::ParsedFile {
                path: format!("docs/features/F-1/f{}.md", i),
                contents: "x".to_string(),
            })
            .collect();
        let parsed = super::super::ParsedResponse { notes: None, files };
        let err = validate_blocks(&parsed, &limits()).unwrap_err();
        assert!(err.to_string().contains("exceeding the maximum"));
    }

    #[test]
    fn test_validate_blocks_file_too_big() {
        let mut lim = limits();
        lim.max_file_size_bytes = 5;
        let parsed = super::super::ParsedResponse {
            notes: None,
            files: vec![super::super::ParsedFile {
                path: "docs/features/F-1/f.md".to_string(),
                contents: "way too long".to_string(),
            }],
        };
        let err = validate_blocks(&parsed, &lim).unwrap_err();
        assert!(err.to_string().contains("exceeding the maximum"));
    }

    #[test]
    fn test_validate_contract_structure_ok() {
        let yaml = "metadata:\n  id: F-1\nrequirements:\n  high_level: []\n";
        assert!(validate_contract_structure(yaml).is_ok());
    }

    #[test]
    fn test_validate_contract_structure_bad_yaml() {
        let yaml = "metadata: : :\n  bad";
        assert!(validate_contract_structure(yaml).is_err());
    }

    #[test]
    fn test_validate_contract_structure_missing_section() {
        let yaml = "metadata:\n  id: F-1\n";
        let err = validate_contract_structure(yaml).unwrap_err();
        assert!(err.to_string().contains("requirements"));
    }

    #[test]
    fn test_count_line_changes() {
        assert_eq!(count_line_changes("a\nb\nc\n", "a\nb\nc\n"), 0);
        // one line removed, one added => 2 changes.
        assert_eq!(count_line_changes("a\nb\n", "a\nc\n"), 2);
    }
}
