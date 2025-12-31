use crate::Result;
use std::path::Path;
use std::process::Command;

/// Ensures that .git/ exists in the current directory.
pub fn ensure_git_repo() -> Result<()> {
    if !Path::new(".git").exists() {
        return Err("not a git repository: .git/ directory not found".into());
    }
    Ok(())
}

/// Ensures git working tree is clean (no uncommitted changes).
/// If allow_untracked is true, untracked files are allowed.
pub fn ensure_git_clean(allow_untracked: bool) -> Result<()> {
    // First ensure we're in a git repo
    ensure_git_repo()?;

    let output = Command::new("git")
        .args(&["status", "--porcelain"])
        .output()
        .map_err(|e| format!("failed to run git status: {e}"))?;

    if !output.status.success() {
        return Err("git status command failed".into());
    }

    let status = String::from_utf8_lossy(&output.stdout);

    // Parse git status output
    for line in status.lines() {
        if line.is_empty() {
            continue;
        }

        // Lines starting with "??" are untracked files
        if allow_untracked && line.starts_with("??") {
            continue;
        }

        // If we get here, there are uncommitted changes
        return Err("git working tree is not clean: please commit or stash your changes".into());
    }

    Ok(())
}
