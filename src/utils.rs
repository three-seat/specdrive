use crate::git;
use std::fmt;
use std::path::Path;

pub fn today_yyyy_mm_dd() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Structured error types for repo readiness checks.
/// Per F-004 contract, these errors map to specific exit codes and error messages.
#[derive(Debug)]
pub enum RepoReadinessError {
    /// .git/ directory not found (NOT_GIT_REPO)
    NotGitRepo,
    /// .specify/ directory not found (NO_SPECIFY_DIR)
    NoSpecifyDir,
    /// Git working tree has uncommitted changes (DIRTY_TREE)
    DirtyTree,
}

impl fmt::Display for RepoReadinessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoReadinessError::NotGitRepo => {
                write!(f, "Not a git repository. Please run this command from the root of a git repo.")
            }
            RepoReadinessError::NoSpecifyDir => {
                write!(f, ".specify/ directory not found. Please run 'specify init' first.")
            }
            RepoReadinessError::DirtyTree => {
                write!(f, "git working tree is not clean: please commit or stash your changes")
            }
        }
    }
}

impl std::error::Error for RepoReadinessError {}

/// Shared helper that ensures the repo is ready for commands that require
/// git repo, Spec Kit initialization, and a clean working tree.
///
/// Per F-004 contract, this is the canonical preflight check that:
/// - Verifies .git/ exists
/// - Verifies .specify/ exists
/// - Verifies the working tree is clean (allowing untracked files)
///
/// Returns structured errors that commands can map to exit codes.
pub fn ensure_repo_and_specify_ready() -> Result<(), RepoReadinessError> {
    // 1. Check .git/ exists (must be checked first per contract invariants)
    if !Path::new(".git").exists() {
        return Err(RepoReadinessError::NotGitRepo);
    }

    // 2. Check .specify/ exists (must be checked second per contract invariants)
    if !Path::new(".specify").exists() {
        return Err(RepoReadinessError::NoSpecifyDir);
    }

    // 3. Ensure git working tree is clean (allow untracked files per contract)
    // Map git errors to DirtyTree error
    git::ensure_git_clean(true).map_err(|_| RepoReadinessError::DirtyTree)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_repo_and_specify_ready_success() {
        // Create a temporary directory with .git and .specify
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create .git directory
        fs::create_dir(temp_path.join(".git")).unwrap();

        // Create .specify directory
        fs::create_dir(temp_path.join(".specify")).unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Initialize a git repo (required for git status check)
        std::process::Command::new("git")
            .args(&["init"])
            .output()
            .unwrap();

        // Should succeed
        let result = ensure_repo_and_specify_ready();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_repo_and_specify_ready_no_git() {
        // Create a temporary directory without .git
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create .specify directory but not .git
        fs::create_dir(temp_path.join(".specify")).unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Should fail with NotGitRepo
        let result = ensure_repo_and_specify_ready();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            RepoReadinessError::NotGitRepo => (),
            _ => panic!("Expected NotGitRepo error"),
        }
    }

    #[test]
    fn test_ensure_repo_and_specify_ready_no_specify() {
        // Create a temporary directory with .git but not .specify
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create .git directory but not .specify
        fs::create_dir(temp_path.join(".git")).unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Should fail with NoSpecifyDir
        let result = ensure_repo_and_specify_ready();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            RepoReadinessError::NoSpecifyDir => (),
            _ => panic!("Expected NoSpecifyDir error"),
        }
    }

    #[test]
    fn test_repo_readiness_error_display() {
        let err = RepoReadinessError::NotGitRepo;
        assert!(err.to_string().contains("git repository"));

        let err = RepoReadinessError::NoSpecifyDir;
        assert!(err.to_string().contains(".specify/"));

        let err = RepoReadinessError::DirtyTree;
        assert!(err.to_string().contains("not clean"));
    }
}
