use crate::Result;
use crate::git;
use std::path::Path;

pub fn today_yyyy_mm_dd() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Shared helper that ensures the repo is ready for commands that require
/// git repo, Spec Kit initialization, and a clean working tree.
pub fn ensure_repo_and_specify_ready() -> Result<()> {
    // 1. Check .git/ exists
    git::ensure_git_repo()?;

    // 2. Check .specify/ exists
    if !Path::new(".specify").exists() {
        return Err("Spec Kit not initialized: .specify/ directory not found. Run 'specify init' first.".into());
    }

    // 3. Ensure git working tree is clean (allow untracked files per contract)
    git::ensure_git_clean(true)?;

    Ok(())
}
