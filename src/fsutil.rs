use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::Result;

/// Structured error types for feature path operations.
/// Per F-004 contract, these errors map to specific exit codes and error messages.
#[derive(Debug)]
pub enum FeaturePathError {
    /// Feature spec file does not exist (MISSING_SPEC)
    MissingSpec(PathBuf),
    /// Feature contract file does not exist (MISSING_CONTRACT)
    MissingContract(PathBuf),
}

impl fmt::Display for FeaturePathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeaturePathError::MissingSpec(path) => {
                write!(f, "spec file not found: {}", path.display())
            }
            FeaturePathError::MissingContract(path) => {
                write!(f, "contract file not found: {}", path.display())
            }
        }
    }
}

impl std::error::Error for FeaturePathError {}

/// Structured error types for template path operations.
/// Per F-004 contract, these errors map to specific exit codes and error messages.
#[derive(Debug)]
pub enum TemplatePathError {
    /// Required template file does not exist (MISSING_TEMPLATE)
    MissingTemplate(PathBuf),
}

impl fmt::Display for TemplatePathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplatePathError::MissingTemplate(path) => {
                write!(f, "required template not found: {}", path.display())
            }
        }
    }
}

impl std::error::Error for TemplatePathError {}

/// Represents the canonical paths for a feature's spec, contract, and
/// patches under `docs/features/<FEATURE_ID>/`.
///
/// Per ADR-002 / F-007, all feature artifacts are co-located under the
/// feature directory.
#[derive(Debug)]
pub struct FeaturePaths {
    /// Path to the feature directory: docs/features/<FEATURE_ID>/
    pub dir: PathBuf,
    /// Path to the feature spec: docs/features/<FEATURE_ID>/spec.md
    pub spec: PathBuf,
    /// Path to the feature contract: docs/features/<FEATURE_ID>/contract.yaml
    pub contract: PathBuf,
    /// Path to the feature patches directory: docs/features/<FEATURE_ID>/patches/
    pub patches: PathBuf,
}

impl FeaturePaths {
    /// Constructs canonical feature paths for the given feature ID.
    /// Per ADR-002 / F-007, paths are:
    /// - dir: docs/features/<FEATURE_ID>/
    /// - spec: docs/features/<FEATURE_ID>/spec.md
    /// - contract: docs/features/<FEATURE_ID>/contract.yaml
    /// - patches: docs/features/<FEATURE_ID>/patches/
    pub fn new(feature_id: &str) -> Self {
        let dir = PathBuf::from("docs").join("features").join(feature_id);
        let spec = dir.join("spec.md");
        let contract = dir.join("contract.yaml");
        let patches = dir.join("patches");
        Self {
            dir,
            spec,
            contract,
            patches,
        }
    }

    /// Validates that both spec and contract files exist.
    /// Returns an error indicating which file is missing if either does not exist.
    pub fn validate(&self) -> std::result::Result<(), FeaturePathError> {
        if !self.spec.exists() {
            return Err(FeaturePathError::MissingSpec(self.spec.clone()));
        }
        if !self.contract.exists() {
            return Err(FeaturePathError::MissingContract(self.contract.clone()));
        }
        Ok(())
    }
}

/// Represents the canonical paths for contract template files.
/// Per F-004 contract, provides construction and validation of template paths.
#[derive(Debug)]
pub struct TemplatePaths {
    /// Path to minimal contract template: docs/templates/feature.contract.minimal.yaml
    pub minimal: PathBuf,
    /// Path to critical contract template: docs/templates/feature.contract.critical.yaml
    pub critical: PathBuf,
}

impl TemplatePaths {
    /// Constructs canonical template paths.
    /// Per F-004 contract, paths are:
    /// - minimal: docs/templates/feature.contract.minimal.yaml
    /// - critical: docs/templates/feature.contract.critical.yaml
    pub fn new() -> Self {
        let minimal = PathBuf::from("docs/templates/feature.contract.minimal.yaml");
        let critical = PathBuf::from("docs/templates/feature.contract.critical.yaml");
        Self { minimal, critical }
    }

    /// Validates that both template files exist.
    /// Returns an error indicating which template is missing if either does not exist.
    pub fn validate(&self) -> std::result::Result<(), TemplatePathError> {
        if !self.minimal.exists() {
            return Err(TemplatePathError::MissingTemplate(self.minimal.clone()));
        }
        if !self.critical.exists() {
            return Err(TemplatePathError::MissingTemplate(self.critical.clone()));
        }
        Ok(())
    }
}

impl Default for TemplatePaths {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of discovering an optional documentation file.
/// Per F-004 contract, helpers return presence + path or absence.
#[derive(Debug)]
pub enum OptionalDoc {
    /// Document exists at the given path
    Present(PathBuf),
    /// Document does not exist
    Absent,
}

impl OptionalDoc {
    /// Returns true if the document is present.
    #[allow(dead_code)]
    pub fn is_present(&self) -> bool {
        matches!(self, OptionalDoc::Present(_))
    }

    /// Returns the path if the document is present, None otherwise.
    pub fn path(&self) -> Option<&Path> {
        match self {
            OptionalDoc::Present(p) => Some(p),
            OptionalDoc::Absent => None,
        }
    }
}

/// Discovers the constitution file (docs/constitution.md).
/// Per ADR-002 / F-007, the constitution lives under docs/.
/// This helper never fails; it returns Absent if the file doesn't exist.
pub fn find_constitution() -> OptionalDoc {
    let path = PathBuf::from("docs/constitution.md");
    if path.exists() {
        OptionalDoc::Present(path)
    } else {
        OptionalDoc::Absent
    }
}

/// Discovers the system overview file (docs/system-overview.md).
/// Per F-004 contract, returns presence + path or absence.
/// This helper never fails; it returns Absent if the file doesn't exist.
pub fn find_system_overview() -> OptionalDoc {
    let path = PathBuf::from("docs/system-overview.md");
    if path.exists() {
        OptionalDoc::Present(path)
    } else {
        OptionalDoc::Absent
    }
}

/// Discovers ADR markdown files in docs/adrs/.
/// Per F-004 contract, returns a deterministic, sorted list of paths (may be empty).
/// This helper never fails; it returns an empty vec if the directory doesn't exist or has no markdown files.
pub fn find_adrs() -> Vec<PathBuf> {
    let adrs_dir = PathBuf::from("docs/adrs");

    if !adrs_dir.exists() || !adrs_dir.is_dir() {
        return Vec::new();
    }

    let entries = match fs::read_dir(&adrs_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut adr_files = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            adr_files.push(path);
        }
    }

    // Per F-004 contract, return deterministic sorted list
    adr_files.sort();
    adr_files
}

/// Copy a UTF-8 text template to `to`, doing simple string replacements.
/// Refuses to overwrite existing destination.
pub fn copy_template_with_replacements(
    from: &Path,
    to: &Path,
    replacements: &[(&str, &str)],
) -> Result<()> {
    if !from.exists() {
        return Err(format!("template missing: {}", from.display()).into());
    }
    if to.exists() {
        return Err(format!("refusing to overwrite: {}", to.display()).into());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut contents = std::fs::read_to_string(from)
        .map_err(|e| format!("failed to read template {}: {e}", from.display()))?;

    for (needle, value) in replacements {
        contents = contents.replace(needle, value);
    }

    std::fs::write(to, contents).map_err(|e| format!("failed to write {}: {e}", to.display()))?;

    Ok(())
}

/// Computes the next zero-padded numbered filename in `dir` matching the
/// pattern `<prefix><NNN><suffix>` (for example `notes-` + `001` + `.md`).
///
/// Per F-009 LLR-011, the next number is the highest existing NNN matching the
/// pattern in the directory, plus one. Gaps never cause overwrites — if `001`
/// and `003` exist, `004` is returned. If the directory does not exist or holds
/// no matching files, the sequence starts at `001`.
///
/// This is a reusable utility intended for any feature that writes numbered
/// output artifacts. It never creates the directory and never fails: an
/// unreadable directory simply yields the first number.
pub fn next_numbered_filename(dir: &Path, prefix: &str, suffix: &str) -> String {
    let mut highest: u32 = 0;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };

            // Match `<prefix><digits><suffix>` exactly, where the middle is a
            // run of ASCII digits. Anything else is ignored.
            let Some(rest) = name.strip_prefix(prefix) else {
                continue;
            };
            let Some(digits) = rest.strip_suffix(suffix) else {
                continue;
            };
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            if let Ok(n) = digits.parse::<u32>()
                && n > highest
            {
                highest = n;
            }
        }
    }

    let next = highest.saturating_add(1);
    format!("{}{:03}{}", prefix, next, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_feature_paths_construction() {
        let paths = FeaturePaths::new("F-001-test");
        assert_eq!(paths.dir, PathBuf::from("docs/features/F-001-test"));
        assert_eq!(
            paths.spec,
            PathBuf::from("docs/features/F-001-test/spec.md")
        );
        assert_eq!(
            paths.contract,
            PathBuf::from("docs/features/F-001-test/contract.yaml")
        );
        assert_eq!(
            paths.patches,
            PathBuf::from("docs/features/F-001-test/patches")
        );
    }

    #[test]
    fn test_feature_paths_validate_success() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        fs::create_dir_all("docs/features/F-001-test").unwrap();
        fs::write("docs/features/F-001-test/spec.md", "# Test").unwrap();
        fs::write("docs/features/F-001-test/contract.yaml", "test: true").unwrap();

        let paths = FeaturePaths::new("F-001-test");
        let result = paths.validate();

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn test_feature_paths_validate_missing_spec() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Create contract but not spec
        fs::create_dir_all("docs/features/F-001-test").unwrap();
        fs::write("docs/features/F-001-test/contract.yaml", "test: true").unwrap();

        let paths = FeaturePaths::new("F-001-test");
        let result = paths.validate();

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            FeaturePathError::MissingSpec(_) => (),
            _ => panic!("Expected MissingSpec error"),
        }
    }

    #[test]
    fn test_feature_paths_validate_missing_contract() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Create spec but not contract
        fs::create_dir_all("docs/features/F-001-test").unwrap();
        fs::write("docs/features/F-001-test/spec.md", "# Test").unwrap();

        let paths = FeaturePaths::new("F-001-test");
        let result = paths.validate();

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            FeaturePathError::MissingContract(_) => (),
            _ => panic!("Expected MissingContract error"),
        }
    }

    #[test]
    fn test_template_paths_construction() {
        let paths = TemplatePaths::new();
        assert_eq!(
            paths.minimal,
            PathBuf::from("docs/templates/feature.contract.minimal.yaml")
        );
        assert_eq!(
            paths.critical,
            PathBuf::from("docs/templates/feature.contract.critical.yaml")
        );
    }

    #[test]
    fn test_template_paths_validate_success() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Create template files
        fs::create_dir_all("docs/templates").unwrap();
        fs::write(
            "docs/templates/feature.contract.minimal.yaml",
            "minimal: true",
        )
        .unwrap();
        fs::write(
            "docs/templates/feature.contract.critical.yaml",
            "critical: true",
        )
        .unwrap();

        let paths = TemplatePaths::new();
        let result = paths.validate();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn test_template_paths_validate_missing_template() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Create only minimal template
        fs::create_dir_all("docs/templates").unwrap();
        fs::write(
            "docs/templates/feature.contract.minimal.yaml",
            "minimal: true",
        )
        .unwrap();

        let paths = TemplatePaths::new();
        let result = paths.validate();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            TemplatePathError::MissingTemplate(_) => (),
        }
    }

    #[test]
    fn test_find_constitution_present() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        fs::create_dir_all("docs").unwrap();
        fs::write("docs/constitution.md", "# Constitution").unwrap();

        let result = find_constitution();

        std::env::set_current_dir(original_dir).unwrap();

        assert!(matches!(result, OptionalDoc::Present(_)));
        assert_eq!(
            result.path(),
            Some(PathBuf::from("docs/constitution.md").as_path())
        );
    }

    #[test]
    fn test_find_constitution_absent() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = find_constitution();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(matches!(result, OptionalDoc::Absent));
        assert_eq!(result.path(), None);
    }

    #[test]
    fn test_find_system_overview_present() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Create system overview
        fs::create_dir_all("docs").unwrap();
        fs::write("docs/system-overview.md", "# Overview").unwrap();

        let result = find_system_overview();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(matches!(result, OptionalDoc::Present(_)));
    }

    #[test]
    fn test_find_system_overview_absent() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = find_system_overview();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(matches!(result, OptionalDoc::Absent));
    }

    #[test]
    fn test_find_adrs_empty() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = find_adrs();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_find_adrs_with_files() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Create ADR files
        fs::create_dir_all("docs/adrs").unwrap();
        fs::write("docs/adrs/ADR-001-test.md", "# ADR 001").unwrap();
        fs::write("docs/adrs/ADR-002-another.md", "# ADR 002").unwrap();
        // Non-markdown file should be ignored
        fs::write("docs/adrs/README.txt", "Not an ADR").unwrap();

        let result = find_adrs();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(result.len(), 2);
        // Results should be sorted
        assert!(result[0].to_string_lossy().contains("ADR-001"));
        assert!(result[1].to_string_lossy().contains("ADR-002"));
    }

    #[test]
    fn test_optional_doc_methods() {
        let present = OptionalDoc::Present(PathBuf::from("test.md"));
        assert!(present.is_present());
        assert!(present.path().is_some());

        let absent = OptionalDoc::Absent;
        assert!(!absent.is_present());
        assert!(absent.path().is_none());
    }

    #[test]
    fn test_next_numbered_filename_empty_dir() {
        let temp = TempDir::new().unwrap();
        let name = next_numbered_filename(temp.path(), "notes-", ".md");
        assert_eq!(name, "notes-001.md");
    }

    #[test]
    fn test_next_numbered_filename_missing_dir() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("does-not-exist");
        let name = next_numbered_filename(&missing, "implement-", ".raw.md");
        assert_eq!(name, "implement-001.raw.md");
    }

    #[test]
    fn test_next_numbered_filename_gaps_do_not_overwrite() {
        let temp = TempDir::new().unwrap();
        // 001 and 003 present, 002 is a gap. Next must be 004, not 002.
        fs::write(temp.path().join("notes-001.md"), "a").unwrap();
        fs::write(temp.path().join("notes-003.md"), "b").unwrap();
        // Files that don't match the pattern must be ignored.
        fs::write(temp.path().join("notes-xyz.md"), "c").unwrap();
        fs::write(temp.path().join("implement-002.raw.md"), "d").unwrap();

        let name = next_numbered_filename(temp.path(), "notes-", ".md");
        assert_eq!(name, "notes-004.md");
    }

    #[test]
    fn test_next_numbered_filename_distinct_patterns() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("implement-001.raw.md"), "a").unwrap();
        fs::write(temp.path().join("implement-002.raw.md"), "b").unwrap();

        let name = next_numbered_filename(temp.path(), "implement-", ".raw.md");
        assert_eq!(name, "implement-003.raw.md");
        // The notes pattern is independent and starts fresh.
        let notes = next_numbered_filename(temp.path(), "notes-", ".md");
        assert_eq!(notes, "notes-001.md");
    }

    #[test]
    fn test_error_display() {
        let err = FeaturePathError::MissingSpec(PathBuf::from("test.spec.md"));
        assert!(err.to_string().contains("spec file not found"));

        let err = FeaturePathError::MissingContract(PathBuf::from("test.yaml"));
        assert!(err.to_string().contains("contract file not found"));

        let err = TemplatePathError::MissingTemplate(PathBuf::from("template.yaml"));
        assert!(err.to_string().contains("required template not found"));
    }
}
