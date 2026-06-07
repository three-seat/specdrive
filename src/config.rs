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
    /// Chat export/import configuration (F-009)
    #[serde(default)]
    pub chat: Option<ChatSection>,
}

/// Chat workflow configuration section (F-009).
#[derive(Debug, Deserialize)]
pub struct ChatSection {
    /// Import-side configuration.
    #[serde(default)]
    pub import: Option<ChatImportSection>,
}

/// Raw, unvalidated `chat.import` size-limit configuration (F-009).
///
/// Values are read as untyped YAML so that invalid or unsafe values (zero,
/// negative, or non-integer) can be detected and replaced with built-in
/// defaults plus a warning, per LLR-025, rather than causing a hard parse
/// failure of the whole config file.
#[derive(Debug, Deserialize)]
pub struct ChatImportSection {
    #[serde(default)]
    pub max_file_blocks: Option<serde_yaml::Value>,
    #[serde(default)]
    pub max_file_size_bytes: Option<serde_yaml::Value>,
    #[serde(default)]
    pub max_response_size_bytes: Option<serde_yaml::Value>,
}

/// Resolved, validated `chat.import` size limits (F-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatImportLimits {
    pub max_file_blocks: usize,
    pub max_file_size_bytes: u64,
    pub max_response_size_bytes: u64,
}

impl ChatImportLimits {
    pub const DEFAULT_MAX_FILE_BLOCKS: usize = 20;
    pub const DEFAULT_MAX_FILE_SIZE_BYTES: u64 = 1_048_576; // 1 MB
    pub const DEFAULT_MAX_RESPONSE_SIZE_BYTES: u64 = 5_242_880; // 5 MB

    /// Built-in defaults applied when config is absent or values are invalid.
    pub fn defaults() -> Self {
        Self {
            max_file_blocks: Self::DEFAULT_MAX_FILE_BLOCKS,
            max_file_size_bytes: Self::DEFAULT_MAX_FILE_SIZE_BYTES,
            max_response_size_bytes: Self::DEFAULT_MAX_RESPONSE_SIZE_BYTES,
        }
    }
}

/// Coerces a raw YAML config value into a positive integer, falling back to the
/// supplied default (with a warning) if the value is absent, not an integer,
/// zero, or negative. Per LLR-025 / E-012.
fn coerce_positive_limit(field: &Option<serde_yaml::Value>, name: &str, default: u64) -> u64 {
    match field {
        None => default,
        Some(value) => match value.as_u64() {
            Some(n) if n > 0 => n,
            _ => {
                eprintln!(
                    "warning: chat.import.{} is invalid (must be a positive integer); \
                     using built-in default {}",
                    name, default
                );
                default
            }
        },
    }
}

/// Loads and validates the `chat.import` size limits (F-009).
///
/// Built-in defaults apply if the config file is absent, the `chat.import`
/// namespace is unset, or any individual value is invalid or unsafe. Invalid
/// values produce a warning and fall back to the default for that field only
/// (LLR-024, LLR-025). A config file that cannot be parsed at all is treated as
/// "use defaults" with a warning, since size limits must never hard-fail the
/// import path.
pub fn load_chat_import_limits() -> ChatImportLimits {
    let defaults = ChatImportLimits::defaults();

    let config = match Config::load() {
        Ok(Some(c)) => c,
        Ok(None) => return defaults,
        Err(e) => {
            eprintln!("warning: {}; using built-in chat import limits", e);
            return defaults;
        }
    };

    let Some(import) = config.chat.and_then(|c| c.import) else {
        return defaults;
    };

    resolve_chat_import_limits(&import)
}

/// Resolves a raw `chat.import` section into validated limits, applying
/// built-in defaults (with a warning) for any absent or invalid value. Split
/// out from [`load_chat_import_limits`] so the validation logic is testable
/// without touching the filesystem.
fn resolve_chat_import_limits(import: &ChatImportSection) -> ChatImportLimits {
    let defaults = ChatImportLimits::defaults();
    ChatImportLimits {
        max_file_blocks: coerce_positive_limit(
            &import.max_file_blocks,
            "max_file_blocks",
            defaults.max_file_blocks as u64,
        ) as usize,
        max_file_size_bytes: coerce_positive_limit(
            &import.max_file_size_bytes,
            "max_file_size_bytes",
            defaults.max_file_size_bytes,
        ),
        max_response_size_bytes: coerce_positive_limit(
            &import.max_response_size_bytes,
            "max_response_size_bytes",
            defaults.max_response_size_bytes,
        ),
    }
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

    // These tests exercise the pure limit-resolution logic without touching the
    // filesystem or the process working directory, to avoid the cwd races that
    // affect the config-loading tests.

    fn parse_import(yaml: &str) -> ChatImportSection {
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        config.chat.unwrap().import.unwrap()
    }

    #[test]
    fn test_chat_import_limits_valid_overrides() {
        let import = parse_import(
            r#"
schema_version: 1
chat:
  import:
    max_file_blocks: 5
    max_file_size_bytes: 2048
    max_response_size_bytes: 10000
"#,
        );
        let limits = resolve_chat_import_limits(&import);
        assert_eq!(limits.max_file_blocks, 5);
        assert_eq!(limits.max_file_size_bytes, 2048);
        assert_eq!(limits.max_response_size_bytes, 10000);
    }

    #[test]
    fn test_chat_import_limits_invalid_values_fall_back() {
        // zero, negative, and non-integer values must all fall back to defaults.
        let import = parse_import(
            r#"
schema_version: 1
chat:
  import:
    max_file_blocks: 0
    max_file_size_bytes: -10
    max_response_size_bytes: "lots"
"#,
        );
        let limits = resolve_chat_import_limits(&import);
        assert_eq!(limits, ChatImportLimits::defaults());
    }

    #[test]
    fn test_chat_import_limits_partial_override() {
        let import = parse_import(
            r#"
schema_version: 1
chat:
  import:
    max_file_blocks: 7
"#,
        );
        let limits = resolve_chat_import_limits(&import);
        assert_eq!(limits.max_file_blocks, 7);
        // Unspecified fields keep defaults.
        assert_eq!(
            limits.max_file_size_bytes,
            ChatImportLimits::DEFAULT_MAX_FILE_SIZE_BYTES
        );
        assert_eq!(
            limits.max_response_size_bytes,
            ChatImportLimits::DEFAULT_MAX_RESPONSE_SIZE_BYTES
        );
    }

    #[test]
    fn test_coerce_positive_limit() {
        use serde_yaml::Value;
        assert_eq!(coerce_positive_limit(&None, "x", 99), 99);
        assert_eq!(
            coerce_positive_limit(&Some(Value::Number(42.into())), "x", 99),
            42
        );
        // zero, negative, and string all fall back.
        assert_eq!(
            coerce_positive_limit(&Some(Value::Number(0.into())), "x", 99),
            99
        );
        assert_eq!(
            coerce_positive_limit(&Some(Value::Number((-5i64).into())), "x", 99),
            99
        );
        assert_eq!(
            coerce_positive_limit(&Some(Value::String("nope".into())), "x", 99),
            99
        );
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
