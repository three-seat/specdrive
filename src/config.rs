use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::Path;

/// Config file location (optional)
const CONFIG_PATH: &str = "docs/specdrive/config.yaml";

/// Structured error types for config and validation operations.
#[derive(Debug)]
pub enum ConfigError {
    /// Config file exists but contains invalid YAML syntax
    ParseError { path: String, source: String },
    /// FEATURE_ID fails basic safety checks
    SafetyViolation { feature_id: String, reason: String },
    /// FEATURE_ID does not match the configured naming pattern
    PatternMismatch {
        feature_id: String,
        pattern: String,
        example: Option<String>,
    },
}

impl ConfigError {
    /// Returns the appropriate exit code for this error.
    /// Per contract: safety violations and pattern mismatches exit with code 1,
    /// parse errors exit with code 2.
    pub fn exit_code(&self) -> i32 {
        match self {
            ConfigError::ParseError { .. } => 2,
            ConfigError::SafetyViolation { .. } => 1,
            ConfigError::PatternMismatch { .. } => 1,
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::ParseError { path, source } => {
                write!(f, "failed to parse config file {}: {}", path, source)
            }
            ConfigError::SafetyViolation { feature_id, reason } => {
                write!(f, "invalid FEATURE_ID '{}': {}", feature_id, reason)
            }
            ConfigError::PatternMismatch {
                feature_id,
                pattern,
                example,
            } => {
                if let Some(ex) = example {
                    write!(
                        f,
                        "FEATURE_ID '{}' does not match required pattern '{}'. Example: '{}'",
                        feature_id, pattern, ex
                    )
                } else {
                    write!(
                        f,
                        "FEATURE_ID '{}' does not match required pattern '{}'",
                        feature_id, pattern
                    )
                }
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Naming configuration for features
#[derive(Debug, Deserialize)]
pub struct NamingConfig {
    /// Regex pattern for valid feature IDs
    pub pattern: String,
    /// Example of a valid feature ID
    #[serde(default)]
    pub example: Option<String>,
    /// Human-readable description of the naming convention
    /// Reserved for future config introspection / UX improvements.
    #[allow(dead_code)]
    #[serde(default)]
    pub description: Option<String>,
}

/// Top-level configuration structure
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Schema version (currently unused but reserved for future use)
    #[allow(dead_code)]
    #[serde(default)]
    pub schema_version: Option<u32>,
    /// Naming conventions configuration
    #[serde(default)]
    pub naming: Option<NamingSection>,
}

/// Naming conventions section
#[derive(Debug, Deserialize)]
pub struct NamingSection {
    /// Feature naming convention
    pub feature: Option<NamingConfig>,
}

impl Config {
    /// Attempts to load the config from the default path.
    /// Returns None if the file does not exist.
    /// Returns an error if the file exists but cannot be parsed.
    pub fn load() -> Result<Option<Self>, ConfigError> {
        let path = Path::new(CONFIG_PATH);

        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(path).map_err(|e| ConfigError::ParseError {
            path: CONFIG_PATH.to_string(),
            source: e.to_string(),
        })?;

        let config: Config =
            serde_yaml::from_str(&contents).map_err(|e| ConfigError::ParseError {
                path: CONFIG_PATH.to_string(),
                source: e.to_string(),
            })?;

        Ok(Some(config))
    }
}

/// Validates a FEATURE_ID against basic safety rules and optional config patterns.
///
/// This function:
/// 1. Always performs basic safety checks (reject /, \, .., control chars, whitespace)
/// 2. If config exists and defines a naming.feature.pattern, validates against it
///
/// Returns Ok(()) if validation passes, or a ConfigError if validation fails.
pub fn validate_feature_id(feature_id: &str) -> Result<(), ConfigError> {
    // Step 1: Basic safety checks (always enforced, even without config)

    // Check for path traversal characters
    if feature_id.contains('/') || feature_id.contains('\\') || feature_id.contains("..") {
        return Err(ConfigError::SafetyViolation {
            feature_id: feature_id.to_string(),
            reason: "FEATURE_ID must not contain '/', '\\', or '..'".to_string(),
        });
    }

    // Check for control characters or whitespace
    if feature_id
        .chars()
        .any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(ConfigError::SafetyViolation {
            feature_id: feature_id.to_string(),
            reason: "FEATURE_ID must not contain control characters or whitespace".to_string(),
        });
    }

    // Step 2: Load config and check pattern if present
    let config = Config::load()?;

    if let Some(cfg) = config
        && let Some(naming) = cfg.naming
        && let Some(feature_naming) = naming.feature
    {
        // Validate against the configured pattern
        let regex =
            regex::Regex::new(&feature_naming.pattern).map_err(|e| ConfigError::ParseError {
                path: CONFIG_PATH.to_string(),
                source: format!("invalid regex pattern '{}': {}", feature_naming.pattern, e),
            })?;

        if !regex.is_match(feature_id) {
            return Err(ConfigError::PatternMismatch {
                feature_id: feature_id.to_string(),
                pattern: feature_naming.pattern,
                example: feature_naming.example,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_feature_id_no_config() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // No config file present - should accept valid IDs
        let result = validate_feature_id("F-001-bootstrap");
        assert!(result.is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_validate_feature_id_safety_slash() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Should reject paths with slashes
        let result = validate_feature_id("../../etc/passwd");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::SafetyViolation { .. } => (),
            _ => panic!("Expected SafetyViolation"),
        }

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_validate_feature_id_safety_backslash() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Should reject paths with backslashes
        let result = validate_feature_id("..\\..\\windows\\system32");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::SafetyViolation { .. } => (),
            _ => panic!("Expected SafetyViolation"),
        }

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_validate_feature_id_safety_whitespace() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Should reject IDs with whitespace
        let result = validate_feature_id("F-001 bootstrap");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::SafetyViolation { .. } => (),
            _ => panic!("Expected SafetyViolation"),
        }

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_validate_feature_id_safety_control_chars() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Should reject IDs with control characters (e.g., newline)
        let result = validate_feature_id("F-001\nbootstrap");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::SafetyViolation { .. } => (),
            _ => panic!("Expected SafetyViolation"),
        }

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_validate_feature_id_with_valid_pattern() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Create config with pattern
        fs::create_dir_all("docs/specdrive").unwrap();
        fs::write(
            "docs/specdrive/config.yaml",
            r#"
schema_version: 1
naming:
  feature:
    pattern: "^F-[0-9]{3}-[a-z0-9-]+$"
    example: "F-001-bootstrap"
    description: "ID format: F-<3 digits>-<kebab-case-label>"
"#,
        )
        .unwrap();

        // Valid ID matching pattern
        let result = validate_feature_id("F-001-bootstrap");
        assert!(result.is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_validate_feature_id_with_invalid_pattern() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Create config with pattern
        fs::create_dir_all("docs/specdrive").unwrap();
        fs::write(
            "docs/specdrive/config.yaml",
            r#"
schema_version: 1
naming:
  feature:
    pattern: "^F-[0-9]{3}-[a-z0-9-]+$"
    example: "F-001-bootstrap"
    description: "ID format: F-<3 digits>-<kebab-case-label>"
"#,
        )
        .unwrap();

        // Invalid ID not matching pattern
        let result = validate_feature_id("feature-foo");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::PatternMismatch {
                feature_id,
                pattern,
                example,
            } => {
                assert_eq!(feature_id, "feature-foo");
                assert!(pattern.contains("F-[0-9]{3}"));
                assert_eq!(example, Some("F-001-bootstrap".to_string()));
            }
            _ => panic!("Expected PatternMismatch"),
        }

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_validate_feature_id_invalid_yaml() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Create invalid YAML
        fs::create_dir_all("docs/specdrive").unwrap();
        fs::write("docs/specdrive/config.yaml", "invalid: yaml: syntax: here:").unwrap();

        // Should fail with parse error
        let result = validate_feature_id("F-001-bootstrap");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ParseError { path, .. } => {
                assert!(path.contains("config.yaml"));
            }
            _ => panic!("Expected ParseError"),
        }

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_config_error_exit_codes() {
        let safety_err = ConfigError::SafetyViolation {
            feature_id: "bad/id".to_string(),
            reason: "contains slash".to_string(),
        };
        assert_eq!(safety_err.exit_code(), 1);

        let pattern_err = ConfigError::PatternMismatch {
            feature_id: "bad-id".to_string(),
            pattern: "^F-".to_string(),
            example: None,
        };
        assert_eq!(pattern_err.exit_code(), 1);

        let parse_err = ConfigError::ParseError {
            path: "config.yaml".to_string(),
            source: "bad yaml".to_string(),
        };
        assert_eq!(parse_err.exit_code(), 2);
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::SafetyViolation {
            feature_id: "bad/id".to_string(),
            reason: "contains slash".to_string(),
        };
        assert!(err.to_string().contains("bad/id"));
        assert!(err.to_string().contains("contains slash"));

        let err = ConfigError::PatternMismatch {
            feature_id: "bad-id".to_string(),
            pattern: "^F-".to_string(),
            example: Some("F-001-test".to_string()),
        };
        assert!(err.to_string().contains("bad-id"));
        assert!(err.to_string().contains("^F-"));
        assert!(err.to_string().contains("F-001-test"));
    }
}
