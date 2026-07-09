//! `specdrive` lifecycle state enforcement (F-010).
//!
//! Formalizes the SpecDrive feature lifecycle via a per-feature, append-only
//! `state.yaml` sidecar. Early base states (draft, contract, patch) are
//! inferred read-only from artifact presence; review, done, block, defer,
//! unblock, and resume are explicit human-gated commands that append events.
//!
//! Safety properties owned by this module (Constitution VI, ADR-0003):
//! - `state.yaml` is append-only: no existing event is ever modified,
//!   reordered, or deleted (LLR-015).
//! - Inferred states are never persisted; only explicit human command events
//!   are written (LLR-016).
//! - Actor identity is informational only and carries no security meaning
//!   (LLR-020).

pub mod commands;
pub mod infer;
pub mod state;

use std::fmt;
use std::path::Path;
use std::process::Command;

use crate::fsutil::FeaturePaths;

/// Error type for lifecycle commands, carrying a CLI exit code.
///
/// Exit codes follow the contract:
/// - 1: usage or precondition failure (invalid FEATURE_ID, missing feature
///   directory, missing --reason, wrong base state, unresolved overlay,
///   missing required artifact or review event, feature not blocked/deferred).
/// - 2: underlying tool or IO failure (git, filesystem, YAML parse/serialize).
#[derive(Debug)]
pub struct LifecycleError {
    message: String,
    exit_code: i32,
}

impl LifecycleError {
    pub fn new(message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            message: message.into(),
            exit_code,
        }
    }

    /// Usage or precondition failure (exit 1).
    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(message, 1)
    }

    /// Underlying tool or IO failure (exit 2).
    pub fn io(message: impl Into<String>) -> Self {
        Self::new(message, 2)
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LifecycleError {}

/// Validates FEATURE_ID as a safe single-directory component (P-002, E-001).
///
/// Only ASCII alphanumerics, hyphens, and underscores are permitted. This
/// rejects path separators, traversal sequences, control characters, and any
/// other metacharacter before a path is ever constructed.
pub fn validate_feature_id(feature_id: &str) -> Result<(), LifecycleError> {
    if feature_id.trim().is_empty() {
        return Err(LifecycleError::usage("FEATURE_ID cannot be empty"));
    }

    let ok = feature_id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');

    if !ok {
        return Err(LifecycleError::usage(format!(
            "invalid FEATURE_ID '{}': only ASCII letters, digits, '-', and '_' are allowed \
             (no path separators, traversal sequences, or control characters)",
            feature_id
        )));
    }

    Ok(())
}

/// Ensures the command is running inside a git repository (P-001).
///
/// Lifecycle commands set `git_safety.require_clean_tree: false` — the mutating
/// commands only append to the feature's own append-only sidecar and must be
/// usable mid-workflow (for example `block`). So this only checks that `.git/`
/// is present and never requires a clean tree.
pub fn ensure_git_repo() -> Result<(), LifecycleError> {
    if !Path::new(".git").exists() {
        return Err(LifecycleError::usage(
            "Not a git repository. Please run this command from the root of a git repo.",
        ));
    }
    Ok(())
}

/// Everything the lifecycle commands need after validating preconditions:
/// the resolved feature paths and the (possibly empty) loaded event log.
pub struct FeatureContext {
    pub paths: FeaturePaths,
    pub log: state::StateLog,
}

/// Validates FEATURE_ID, repository readiness, and feature-directory existence,
/// then loads the feature's event log (empty when no `state.yaml` exists).
///
/// This never creates any file — reading is strictly read-only (LLR-003,
/// LLR-023).
pub fn load_feature(feature_id: &str) -> Result<FeatureContext, LifecycleError> {
    validate_feature_id(feature_id)?;
    ensure_git_repo()?;

    let paths = FeaturePaths::new(feature_id);
    if !paths.dir.is_dir() {
        return Err(LifecycleError::usage(format!(
            "feature '{}' not found: {} does not exist",
            feature_id,
            paths.dir.display()
        )));
    }

    let log = state::load_or_empty(feature_id, &paths)?;
    Ok(FeatureContext { paths, log })
}

/// Reads a single git config value, returning `None` when unset or empty.
fn git_config(key: &str) -> Option<String> {
    let output = Command::new("git").args(["config", key]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// Resolves an informational actor identity for a recorded event (LLR-020).
///
/// Priority order: git config user.name, then user.email, then the system
/// username ($USER / $USERNAME), then "unknown". Actor is informational only —
/// it is not authentication and carries no security meaning.
pub fn resolve_actor() -> String {
    git_config("user.name")
        .or_else(|| git_config("user.email"))
        .or_else(|| std::env::var("USER").ok().filter(|s| !s.is_empty()))
        .or_else(|| std::env::var("USERNAME").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Current UTC timestamp in ISO 8601 format (LLR-019), e.g.
/// `2026-06-07T14:00:00Z`.
pub fn now_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
