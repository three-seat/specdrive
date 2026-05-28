use crate::git;
use std::fmt;
use std::path::Path;

pub fn today_yyyy_mm_dd() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Structured error types for repo readiness checks.
///
/// Per ADR-002 / F-007, SpecDrive validates SpecDrive-owned paths (`.git/`,
/// clean tree) and no longer requires `.specify/` to exist.
#[derive(Debug)]
pub enum RepoReadinessError {
    /// .git/ directory not found (NOT_GIT_REPO)
    NotGitRepo,
    /// Git working tree has uncommitted changes (DIRTY_TREE)
    DirtyTree,
}

impl fmt::Display for RepoReadinessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoReadinessError::NotGitRepo => {
                write!(
                    f,
                    "Not a git repository. Please run this command from the root of a git repo."
                )
            }
            RepoReadinessError::DirtyTree => {
                write!(
                    f,
                    "git working tree is not clean: please commit or stash your changes"
                )
            }
        }
    }
}

impl std::error::Error for RepoReadinessError {}

/// Shared helper that ensures the repo is ready for spec-aware commands.
///
/// Per ADR-002 / F-007 this verifies:
/// - `.git/` exists (we are inside a git repository)
/// - the working tree is clean (untracked files are allowed)
///
/// SpecDrive no longer requires `.specify/` or Spec Kit initialization for
/// normal operation.
pub fn ensure_repo_ready() -> Result<(), RepoReadinessError> {
    if !Path::new(".git").exists() {
        return Err(RepoReadinessError::NotGitRepo);
    }

    git::ensure_git_clean(true).map_err(|_| RepoReadinessError::DirtyTree)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_repo_ready_success() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        fs::create_dir(temp_path.join(".git")).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        std::process::Command::new("git")
            .args(["init"])
            .output()
            .unwrap();

        let result = ensure_repo_ready();

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_repo_ready_no_git() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = ensure_repo_ready();

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            RepoReadinessError::NotGitRepo => (),
            _ => panic!("Expected NotGitRepo error"),
        }
    }

    #[test]
    fn test_repo_readiness_error_display() {
        let err = RepoReadinessError::NotGitRepo;
        assert!(err.to_string().contains("git repository"));

        let err = RepoReadinessError::DirtyTree;
        assert!(err.to_string().contains("not clean"));
    }
}
