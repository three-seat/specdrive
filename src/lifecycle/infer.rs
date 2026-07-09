//! Stateless base-state inference from artifact presence only (F-010, LLR-006).
//!
//! Inference performs no content validation and never writes anything. The same
//! artifacts always produce the same inferred state. Only draft, contract, and
//! patch are ever inferred — review, done, blocked, deferred, and implement are
//! never inferred (LLR-007).

use std::fs;

use crate::fsutil::FeaturePaths;

use super::state::BaseState;

/// Returns true when at least one file exists under the feature's `patches/`
/// directory. Presence only — the file contents are never inspected.
pub fn patch_artifact_exists(paths: &FeaturePaths) -> bool {
    match fs::read_dir(&paths.patches) {
        Ok(entries) => entries.flatten().any(|e| e.path().is_file()),
        Err(_) => false,
    }
}

/// Returns true when `contract.yaml` exists and is non-empty (non-whitespace).
/// No YAML parsing or content validation is performed (LLR-006).
fn contract_present_nonempty(paths: &FeaturePaths) -> bool {
    match fs::read_to_string(&paths.contract) {
        Ok(contents) => !contents.trim().is_empty(),
        Err(_) => false,
    }
}

/// Infers the current base state from artifact presence only (LLR-006):
/// - `patch` when at least one file exists under `patches/`
/// - else `contract` when `contract.yaml` exists and is non-empty
/// - else `draft` (the feature directory exists as a precondition; a present
///   `spec.md` is the canonical draft marker)
///
/// Implement state never appears in V1 inference (LLR-007).
pub fn infer_base_state(paths: &FeaturePaths) -> BaseState {
    if patch_artifact_exists(paths) {
        return BaseState::Patch;
    }
    if contract_present_nonempty(paths) {
        return BaseState::Contract;
    }
    BaseState::Draft
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Builds FeaturePaths rooted at a temp dir and creates the feature dir.
    fn scaffold(temp: &TempDir, feature_id: &str) -> FeaturePaths {
        let dir = temp.path().join("docs/features").join(feature_id);
        fs::create_dir_all(&dir).unwrap();
        FeaturePaths {
            spec: dir.join("spec.md"),
            contract: dir.join("contract.yaml"),
            patches: dir.join("patches"),
            dir,
        }
    }

    #[test]
    fn infers_draft_with_spec_only() {
        let temp = TempDir::new().unwrap();
        let paths = scaffold(&temp, "F-1");
        fs::write(&paths.spec, "# Spec").unwrap();
        assert_eq!(infer_base_state(&paths), BaseState::Draft);
    }

    #[test]
    fn infers_contract_when_nonempty() {
        let temp = TempDir::new().unwrap();
        let paths = scaffold(&temp, "F-1");
        fs::write(&paths.spec, "# Spec").unwrap();
        fs::write(&paths.contract, "schema_version: 1").unwrap();
        assert_eq!(infer_base_state(&paths), BaseState::Contract);
    }

    #[test]
    fn empty_contract_is_not_contract_state() {
        let temp = TempDir::new().unwrap();
        let paths = scaffold(&temp, "F-1");
        fs::write(&paths.spec, "# Spec").unwrap();
        fs::write(&paths.contract, "   \n\t\n").unwrap();
        assert_eq!(infer_base_state(&paths), BaseState::Draft);
    }

    #[test]
    fn infers_patch_when_patch_file_present() {
        let temp = TempDir::new().unwrap();
        let paths = scaffold(&temp, "F-1");
        fs::write(&paths.spec, "# Spec").unwrap();
        fs::write(&paths.contract, "schema_version: 1").unwrap();
        fs::create_dir_all(&paths.patches).unwrap();
        fs::write(paths.patches.join("F-1.patch"), "diff").unwrap();
        assert_eq!(infer_base_state(&paths), BaseState::Patch);
    }

    #[test]
    fn empty_patches_dir_is_not_patch_state() {
        let temp = TempDir::new().unwrap();
        let paths = scaffold(&temp, "F-1");
        fs::write(&paths.contract, "schema_version: 1").unwrap();
        fs::create_dir_all(&paths.patches).unwrap();
        assert!(!patch_artifact_exists(&paths));
        assert_eq!(infer_base_state(&paths), BaseState::Contract);
    }

    #[test]
    fn inference_never_returns_review_done_or_overlay() {
        // TC-007: inference only ever yields draft/contract/patch.
        let temp = TempDir::new().unwrap();
        let paths = scaffold(&temp, "F-1");
        fs::write(&paths.spec, "# Spec").unwrap();
        fs::write(&paths.contract, "x: 1").unwrap();
        fs::create_dir_all(&paths.patches).unwrap();
        fs::write(paths.patches.join("p.patch"), "d").unwrap();
        let s = infer_base_state(&paths);
        assert!(matches!(
            s,
            BaseState::Draft | BaseState::Contract | BaseState::Patch
        ));
    }
}
