use crate::Result;
use crate::fsutil;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Custom error type for the patch command with exit codes
#[derive(Debug)]
pub struct PatchError {
    message: String,
    exit_code: i32,
}

impl PatchError {
    fn new(message: String, exit_code: i32) -> Self {
        Self { message, exit_code }
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PatchError {}

/// Main entry point for the patch emit command
pub fn patch_emit_feature(feature_id: &str) -> Result<()> {
    // Convert all errors to PatchError to get proper exit codes
    match patch_emit_feature_inner(feature_id) {
        Ok(()) => Ok(()),
        Err(e) => {
            let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
            Err(err)
        }
    }
}

fn patch_emit_feature_inner(feature_id: &str) -> std::result::Result<(), PatchError> {
    // 1. Validate feature_id is non-empty (P-001 from contract)
    if feature_id.trim().is_empty() {
        return Err(PatchError::new("FEATURE_ID cannot be empty".to_string(), 1));
    }

    // 2. Check .git/ exists (P-002 from contract: E_NOT_GIT_REPO)
    if !Path::new(".git").exists() {
        return Err(PatchError::new(
            "Not a git repository. Please run this command from the root of a git repo."
                .to_string(),
            1,
        ));
    }

    // 3. Check .specify/ exists (P-003 from contract: E_SPECIFY_MISSING)
    if !Path::new(".specify").exists() {
        return Err(PatchError::new(
            ".specify/ directory not found. Please run 'specify init' first.".to_string(),
            1,
        ));
    }

    // 4. Resolve and validate spec and contract paths (P-004, P-005 from contract)
    let feature_paths = fsutil::FeaturePaths::new(feature_id);
    feature_paths.validate().map_err(|e| {
        PatchError::new(format!("{}. Feature {} does not exist.", e, feature_id), 1)
    })?;

    // 5. Detect untracked files and warn (LLR-007 from contract)
    let untracked = detect_untracked_files().map_err(|e| PatchError::new(e, 2))?;
    if !untracked.is_empty() {
        eprintln!("warning: untracked files detected:");
        for file in &untracked {
            eprintln!("  {}", file);
        }
        eprintln!("note: untracked files are not included in the patch.");
        eprintln!("      To include newly created files, use 'git add' to stage them first.");
        eprintln!();
    }

    // 6. Run `git diff --binary HEAD` to capture all changes (LLR-001 from contract)
    let diff_output = Command::new("git")
        .args(&["diff", "--binary", "HEAD"])
        .output()
        .map_err(|e| PatchError::new(format!("failed to run git diff: {}", e), 2))?;

    if !diff_output.status.success() {
        return Err(PatchError::new("git diff command failed".to_string(), 2));
    }

    let diff_content = String::from_utf8_lossy(&diff_output.stdout).to_string();

    // 7. Fail if diff is empty (LLR-005 from contract: E_EMPTY_DIFF)
    if diff_content.trim().is_empty() {
        return Err(PatchError::new(
            "No implementation changes to emit. The git diff is empty.".to_string(),
            1,
        ));
    }

    // 8. Ensure patch directory exists (LLR-003 from contract)
    let patch_dir = PathBuf::from("docs")
        .join("features")
        .join(feature_id)
        .join("patches");

    fs::create_dir_all(&patch_dir).map_err(|e| {
        PatchError::new(
            format!(
                "failed to create patch directory {}: {}",
                patch_dir.display(),
                e
            ),
            2,
        )
    })?;

    // 9. Write patch to file (LLR-002 from contract)
    let patch_file = patch_dir.join(format!("{}.patch", feature_id));

    // Filter out the patch file itself from the diff (LLR-008 from contract)
    let filtered_diff = filter_patch_from_diff(&diff_content, &patch_file);

    fs::write(&patch_file, &filtered_diff).map_err(|e| {
        PatchError::new(
            format!("failed to write patch file {}: {}", patch_file.display(), e),
            2,
        )
    })?;

    // 10. Validate the patch with `git apply --check` (LLR-004 from contract)
    // Note: We need to validate against HEAD, not the current working tree.
    // The working tree has the changes we just captured, so we validate by
    // temporarily stashing tracked changes (excluding the patch file itself).

    // Build the stash command with a pathspec to exclude the patch directory
    // This ensures the patch file we just wrote doesn't get stashed
    let patch_dir_str = format!(":(exclude){}", patch_dir.display());

    let stash_output = Command::new("git")
        .args(&[
            "stash",
            "push",
            "-m",
            "specdrive-patch-validation",
            "--",
            ".",
            &patch_dir_str,
        ])
        .output()
        .map_err(|e| {
            PatchError::new(format!("failed to stash changes for validation: {}", e), 2)
        })?;

    // Check if stash was successful (it might say "No local changes to save")
    let stash_stdout = String::from_utf8_lossy(&stash_output.stdout);
    let stash_stderr = String::from_utf8_lossy(&stash_output.stderr);
    let stash_applied = stash_stdout.contains("Saved working directory")
        || stash_stderr.contains("Saved working directory");

    // Now validate the patch against clean HEAD
    let check_output = Command::new("git")
        .args(&["apply", "--check", patch_file.to_str().unwrap()])
        .output();

    // Restore the stashed changes if we stashed them
    if stash_applied {
        let _ = Command::new("git").args(&["stash", "pop"]).output();
    }

    // Check validation result
    let check_output = check_output
        .map_err(|e| PatchError::new(format!("failed to run git apply --check: {}", e), 2))?;

    if !check_output.status.success() {
        let stderr = String::from_utf8_lossy(&check_output.stderr);
        return Err(PatchError::new(
            format!(
                "generated patch failed validation (git apply --check):\n{}",
                stderr
            ),
            2,
        ));
    }

    // 11. Parse diff stats and print summary (LLR-012 from contract)
    let stats = parse_diff_stats(&filtered_diff);
    println!("Patch emitted successfully:");
    println!("  Path: {}", patch_file.display());
    println!("  Files changed: {}", stats.files_changed);
    println!("  Insertions: +{}", stats.insertions);
    println!("  Deletions: -{}", stats.deletions);

    Ok(())
}

/// Detects untracked files using git status
fn detect_untracked_files() -> std::result::Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(&["status", "--porcelain"])
        .output()
        .map_err(|e| format!("failed to run git status: {}", e))?;

    if !output.status.success() {
        return Err("git status command failed".to_string());
    }

    let status = String::from_utf8_lossy(&output.stdout);
    let mut untracked = Vec::new();

    for line in status.lines() {
        if line.starts_with("??") {
            // Extract filename (skip "?? " prefix)
            if let Some(filename) = line.get(3..) {
                untracked.push(filename.to_string());
            }
        }
    }

    Ok(untracked)
}

/// Filters out references to the patch file itself from the diff
/// This ensures the patch artifact doesn't include itself (LLR-008)
fn filter_patch_from_diff(diff: &str, patch_file: &Path) -> String {
    // Convert patch_file path to string for comparison
    let patch_path_str = patch_file.to_string_lossy();

    // Also check for paths relative to git root
    let relative_patch_path = if let Ok(canonical) = patch_file.canonicalize() {
        // Try to get path relative to current directory
        if let Ok(cwd) = std::env::current_dir() {
            canonical
                .strip_prefix(&cwd)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        }
    } else {
        None
    };

    let mut result = String::new();
    let mut in_patch_file_block = false;

    for line in diff.lines() {
        // Check if this is a diff header for our patch file
        if line.starts_with("diff --git") {
            // Check if this diff block is for the patch file itself
            let is_patch_file = line.contains(&*patch_path_str)
                || relative_patch_path
                    .as_ref()
                    .map_or(false, |p| line.contains(p));

            if is_patch_file {
                in_patch_file_block = true;
                continue;
            } else {
                // Not the patch file, exit the block if we were in one
                in_patch_file_block = false;
            }
        }

        if in_patch_file_block {
            // Skip lines that are part of the patch file's diff block
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Statistics extracted from a git diff
#[derive(Debug, Default)]
struct DiffStats {
    files_changed: usize,
    insertions: usize,
    deletions: usize,
}

/// Parses a git diff to extract basic statistics
fn parse_diff_stats(diff: &str) -> DiffStats {
    let mut stats = DiffStats::default();
    let mut files = std::collections::HashSet::new();

    for line in diff.lines() {
        // Count files by tracking "diff --git" lines
        if line.starts_with("diff --git") {
            // Extract file paths from "diff --git a/path b/path"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                // Use the "b/" path as it represents the new/modified file
                files.insert(parts[3].to_string());
            }
        }
        // Count insertions (lines starting with '+' but not '+++')
        else if line.starts_with('+') && !line.starts_with("+++") {
            stats.insertions += 1;
        }
        // Count deletions (lines starting with '-' but not '---')
        else if line.starts_with('-') && !line.starts_with("---") {
            stats.deletions += 1;
        }
    }

    stats.files_changed = files.len();
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_stats_empty() {
        let diff = "";
        let stats = parse_diff_stats(diff);
        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_parse_diff_stats_basic() {
        let diff = r#"diff --git a/file1.txt b/file1.txt
index 123..456 100644
--- a/file1.txt
+++ b/file1.txt
@@ -1,3 +1,4 @@
 unchanged line
-removed line
+added line
+another added line
 unchanged line
"#;
        let stats = parse_diff_stats(diff);
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.insertions, 2);
        assert_eq!(stats.deletions, 1);
    }

    #[test]
    fn test_parse_diff_stats_multiple_files() {
        let diff = r#"diff --git a/file1.txt b/file1.txt
index 123..456 100644
--- a/file1.txt
+++ b/file1.txt
@@ -1,2 +1,3 @@
 line1
+added line
 line2
diff --git a/file2.txt b/file2.txt
index 789..abc 100644
--- a/file2.txt
+++ b/file2.txt
@@ -1,3 +1,2 @@
 line1
-removed line
 line3
"#;
        let stats = parse_diff_stats(diff);
        assert_eq!(stats.files_changed, 2);
        assert_eq!(stats.insertions, 1);
        assert_eq!(stats.deletions, 1);
    }

    #[test]
    fn test_filter_patch_from_diff() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
index 123..456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,2 +1,3 @@
 fn main() {
+    println!("hello");
 }
diff --git a/docs/features/F-006/patches/F-006.patch b/docs/features/F-006/patches/F-006.patch
new file mode 100644
index 000..111 100644
--- /dev/null
+++ b/docs/features/F-006/patches/F-006.patch
@@ -0,0 +1,2 @@
+patch content here
+more patch content
diff --git a/README.md b/README.md
index 789..abc 100644
--- a/README.md
+++ b/README.md
@@ -1,2 +1,3 @@
 # Project
+New line
"#;
        let patch_file = PathBuf::from("docs/features/F-006/patches/F-006.patch");
        let filtered = filter_patch_from_diff(diff, &patch_file);

        // The filtered diff should include main.rs and README.md but not the patch file itself
        assert!(filtered.contains("src/main.rs"));
        assert!(filtered.contains("README.md"));
        assert!(!filtered.contains("F-006.patch"));
    }

    #[test]
    fn test_patch_error_display() {
        let err = PatchError::new("test error message".to_string(), 1);
        assert_eq!(err.to_string(), "test error message");
        assert_eq!(err.exit_code(), 1);
    }
}
