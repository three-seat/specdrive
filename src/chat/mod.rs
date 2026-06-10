//! `specdrive chat` — bridge between SpecDrive artifacts and stateless AI chat
//! tools (F-009).
//!
//! `chat export` assembles a self-contained, delimited context bundle from a
//! feature's resolved files and prints it to stdout. `chat import` reads a
//! delimited AI response from stdin, validates it fully, previews the changes,
//! and writes artifacts to the feature directory only on explicit confirmation.
//!
//! Security is the dominant concern of this module: import accepts AI-supplied
//! paths and contents and writes them to disk. FEATURE_ID and path validation,
//! line-start-only delimiter matching, and size limits are all enforced here.

mod export;
mod import;

use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::Result;

// --- SPECDRIVE delimiter scheme (LLR-002) ---------------------------------
//
// These are matched at line start only (LLR-017): a line either *is* one of the
// fixed delimiters, or *begins with* the FILE prefix and ends with the FILE
// suffix. Substring search is never used, so a delimiter embedded mid-line in
// file contents can never terminate a block.

pub const BEGIN: &str = "--- SPECDRIVE:BEGIN ---";
pub const NOTES: &str = "--- SPECDRIVE:NOTES ---";
pub const PROMPT: &str = "--- SPECDRIVE:PROMPT ---";
pub const END: &str = "--- SPECDRIVE:END ---";
pub const FILE_PREFIX: &str = "--- SPECDRIVE:FILE ";
pub const FILE_SUFFIX: &str = " ---";

/// The workflow a `chat` invocation targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workflow {
    Draft,
    Implement,
}

impl Workflow {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Workflow::Draft),
            "implement" => Some(Workflow::Implement),
            _ => None,
        }
    }
}

/// Error type for the `chat` command, carrying a CLI exit code.
///
/// Exit codes follow the contract:
/// - 1: usage or precondition failure (invalid FEATURE_ID, missing
///   spec/contract, dirty tree on import, malformed/oversized response,
///   validation failure).
/// - 2: underlying tool or IO failure.
#[derive(Debug)]
pub struct ChatError {
    message: String,
    exit_code: i32,
}

impl ChatError {
    pub fn new(message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            message: message.into(),
            exit_code,
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(message, 1)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(message, 2)
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl fmt::Display for ChatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChatError {}

/// CLI entry point for `specdrive chat <export|import> <draft|implement> <FEATURE_ID>`.
pub fn run(action: &str, workflow: &str, feature_id: &str) -> Result<()> {
    match run_inner(action, workflow, feature_id) {
        Ok(()) => Ok(()),
        Err(e) => {
            let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
            Err(err)
        }
    }
}

fn run_inner(action: &str, workflow: &str, feature_id: &str) -> std::result::Result<(), ChatError> {
    let workflow = Workflow::parse(workflow).ok_or_else(|| {
        ChatError::usage(format!(
            "unknown chat workflow '{}': expected 'draft' or 'implement'",
            workflow
        ))
    })?;

    // FEATURE_ID is validated as a safe single-directory component before any
    // path construction or filesystem operation (LLR-013, P-002, E-001).
    validate_feature_id(feature_id)?;

    match action {
        "export" => export::run(workflow, feature_id),
        "import" => import::run(workflow, feature_id),
        other => Err(ChatError::usage(format!(
            "unknown chat action '{}': expected 'export' or 'import'",
            other
        ))),
    }
}

/// Validates FEATURE_ID as a safe single-directory component (LLR-013).
///
/// Only ASCII alphanumerics, hyphens, and underscores are permitted. This
/// rejects path separators, traversal sequences (`..` contains `.`), null
/// bytes, control characters, whitespace, and any other metacharacter before a
/// path is ever constructed.
pub fn validate_feature_id(feature_id: &str) -> std::result::Result<(), ChatError> {
    if feature_id.is_empty() {
        return Err(ChatError::usage("FEATURE_ID cannot be empty"));
    }

    let ok = feature_id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');

    if !ok {
        return Err(ChatError::usage(format!(
            "invalid FEATURE_ID '{}': only ASCII letters, digits, '-', and '_' are allowed \
             (no path separators, traversal sequences, or control characters)",
            feature_id
        )));
    }

    Ok(())
}

/// Returns true if `line` is a SPECDRIVE delimiter, matched at line start only
/// (LLR-017). The line must either equal one of the fixed delimiters exactly,
/// or begin with the FILE prefix and end with the FILE suffix. Leading
/// whitespace or surrounding text means it is ordinary content, not a
/// delimiter.
///
/// This is the canonical line-start delimiter predicate for the SPECDRIVE
/// scheme. `parse_response` open-codes the equivalent checks for its state
/// machine; this function documents and tests the rule in one place.
#[allow(dead_code)]
pub fn is_delimiter_line(line: &str) -> bool {
    line == BEGIN
        || line == NOTES
        || line == PROMPT
        || line == END
        || parse_file_path(line).is_some()
}

/// If `line` is a well-formed `--- SPECDRIVE:FILE <path> ---` delimiter,
/// returns the trimmed `<path>`. Matching is line-start only.
fn parse_file_path(line: &str) -> Option<&str> {
    let inner = line.strip_prefix(FILE_PREFIX)?;
    let path = inner.strip_suffix(FILE_SUFFIX)?;
    let path = path.trim();
    if path.is_empty() { None } else { Some(path) }
}

/// A FILE block parsed from an AI response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFile {
    /// The raw path string as supplied by the AI (repository-root relative).
    pub path: String,
    /// The inlined contents of the block.
    pub contents: String,
}

/// The structured result of parsing a delimited AI response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedResponse {
    /// NOTES block content, only `Some` when a NOTES block is present and has
    /// non-whitespace content (LLR-012 — empty notes never produce an artifact).
    pub notes: Option<String>,
    /// FILE blocks in the order they appeared.
    pub files: Vec<ParsedFile>,
}

/// Parses a delimited AI response (LLR-007, LLR-017).
///
/// Sections are delimited by line-start-only markers. Content lines accumulate
/// into the currently open NOTES or FILE block. BEGIN, PROMPT, and END markers
/// open no content block (any stray content outside a NOTES/FILE block is
/// ignored). Parsing stops at the first END marker; callers are responsible for
/// having already truncated input at END so that trailing content is ignored
/// (LLR-005).
pub fn parse_response(raw: &str) -> ParsedResponse {
    enum Section {
        None,
        Notes,
        File(usize),
    }

    let mut notes: Option<String> = None;
    let mut files: Vec<ParsedFile> = Vec::new();
    let mut section = Section::None;

    for line in raw.split_inclusive('\n') {
        // Work with the line without its trailing newline for delimiter
        // comparison, but preserve the original (with newline) for content.
        let logical = line.strip_suffix('\n').unwrap_or(line);
        let logical = logical.strip_suffix('\r').unwrap_or(logical);

        if logical == END {
            break;
        }

        if logical == BEGIN || logical == PROMPT {
            section = Section::None;
            continue;
        }

        if logical == NOTES {
            if notes.is_none() {
                notes = Some(String::new());
            }
            section = Section::Notes;
            continue;
        }

        if let Some(path) = parse_file_path(logical) {
            files.push(ParsedFile {
                path: path.to_string(),
                contents: String::new(),
            });
            section = Section::File(files.len() - 1);
            continue;
        }

        // Ordinary content line — append to the open block (with newline).
        match section {
            Section::Notes => {
                if let Some(buf) = notes.as_mut() {
                    buf.push_str(line);
                }
            }
            Section::File(idx) => {
                files[idx].contents.push_str(line);
            }
            Section::None => {}
        }
    }

    // Empty / whitespace-only NOTES must not yield an artifact (LLR-012).
    let notes = notes.filter(|n| !n.trim().is_empty());

    ParsedResponse { notes, files }
}

/// Returns true if `rel` looks like an absolute path, checked *before* any
/// canonicalization (LLR-014, E-007). Covers POSIX roots, Windows drive
/// prefixes, and UNC paths so the same check is safe cross-platform.
pub fn is_absolute_pattern(rel: &str) -> bool {
    if rel.starts_with('/') || rel.starts_with('\\') {
        return true;
    }
    if Path::new(rel).is_absolute() {
        return true;
    }
    let bytes = rel.as_bytes();
    // Windows drive prefix: e.g. "C:" or "C:\".
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return true;
    }
    false
}

/// Lexically normalizes `base.join(rel)`, resolving `.` and `..` without
/// touching the filesystem. Returns `None` if the path would escape above the
/// root of `base` (i.e. a `..` pops past the start).
fn normalize_lexical(base: &Path, rel: &str) -> Option<PathBuf> {
    let joined = base.join(rel);
    let mut out = PathBuf::new();
    for comp in joined.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    Some(out)
}

/// Finds the nearest existing ancestor of `path` (including `path` itself).
fn nearest_existing(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(p) = current {
        if p.exists() {
            return Some(p.to_path_buf());
        }
        current = p.parent();
    }
    None
}

/// Validates that an AI-supplied FILE path resolves strictly within
/// `feature_dir` (LLR-014, LLR-015, LLR-016; E-007, E-008).
///
/// The path is interpreted relative to the repository root (the current working
/// directory), matching the repository-root-relative paths emitted by export.
///
/// Validation order:
/// 1. Absolute-path patterns are rejected before any canonicalization.
/// 2. The path is normalized lexically; a `..` that escapes the feature
///    directory is rejected even when the target does not exist.
/// 3. The nearest existing ancestor is canonicalized and re-checked for
///    containment, so a symlink pointing outside the feature directory is
///    caught after resolution.
///
/// On success, returns the lexically-normalized absolute path (useful for
/// identity comparisons, e.g. locating the contract block).
pub fn validate_file_path(
    feature_dir: &Path,
    rel: &str,
) -> std::result::Result<PathBuf, ChatError> {
    if is_absolute_pattern(rel) {
        return Err(ChatError::usage(format!(
            "rejected absolute FILE path '{}': FILE paths must be relative and within {}",
            rel,
            feature_dir.display()
        )));
    }

    let cwd = std::env::current_dir()
        .map_err(|e| ChatError::io(format!("failed to determine working directory: {}", e)))?;

    let feature_abs = normalize_lexical(&cwd, &feature_dir.to_string_lossy()).ok_or_else(|| {
        ChatError::io(format!(
            "failed to normalize feature directory {}",
            feature_dir.display()
        ))
    })?;

    // 2. Lexical containment (catches `..` traversal, even for paths that do
    //    not yet exist — LLR-016).
    let candidate_abs = normalize_lexical(&cwd, rel).ok_or_else(|| {
        ChatError::usage(format!(
            "rejected FILE path '{}': path traversal escapes the repository root",
            rel
        ))
    })?;

    if !candidate_abs.starts_with(&feature_abs) {
        return Err(ChatError::usage(format!(
            "rejected FILE path '{}': resolves outside the feature directory {}",
            rel,
            feature_dir.display()
        )));
    }

    // 3. Symlink containment re-verification after resolution (LLR-015).
    let feature_canon = feature_dir.canonicalize().map_err(|e| {
        ChatError::io(format!(
            "failed to canonicalize feature directory {}: {}",
            feature_dir.display(),
            e
        ))
    })?;

    let on_disk = cwd.join(rel);
    if let Some(ancestor) = nearest_existing(&on_disk) {
        let ancestor_canon = ancestor.canonicalize().map_err(|e| {
            ChatError::io(format!(
                "failed to canonicalize path {}: {}",
                ancestor.display(),
                e
            ))
        })?;
        if !ancestor_canon.starts_with(&feature_canon) {
            return Err(ChatError::usage(format!(
                "rejected FILE path '{}': resolves (via symlink) outside the feature directory {}",
                rel,
                feature_dir.display()
            )));
        }
    }

    Ok(candidate_abs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_feature_id_accepts_safe() {
        assert!(validate_feature_id("F-009-chat").is_ok());
        assert!(validate_feature_id("F_009_chat").is_ok());
        assert!(validate_feature_id("abc123").is_ok());
    }

    #[test]
    fn test_validate_feature_id_rejects_unsafe() {
        for bad in [
            "",
            "../etc",
            "a/b",
            "a\\b",
            "..",
            "F-009 chat",
            "F-009\nchat",
            "F-009\0chat",
            "F.009",
            "F-009-chat/",
        ] {
            assert!(
                validate_feature_id(bad).is_err(),
                "expected rejection of {:?}",
                bad
            );
        }
    }

    #[test]
    fn test_is_delimiter_line_line_start_only() {
        assert!(is_delimiter_line(BEGIN));
        assert!(is_delimiter_line(END));
        assert!(is_delimiter_line(NOTES));
        assert!(is_delimiter_line("--- SPECDRIVE:FILE docs/x.md ---"));

        // Embedded / indented delimiters are NOT delimiters (injection guard).
        assert!(!is_delimiter_line("  --- SPECDRIVE:END ---"));
        assert!(!is_delimiter_line("text --- SPECDRIVE:END ---"));
        assert!(!is_delimiter_line("--- SPECDRIVE:FILE  ---")); // empty path
    }

    #[test]
    fn test_parse_response_basic() {
        let raw = format!(
            "{begin}\n{notes}\nHello note\n{file} docs/features/F-1/contract.yaml {suf}\nkey: value\n{end}\n",
            begin = BEGIN,
            notes = NOTES,
            file = "--- SPECDRIVE:FILE",
            suf = "---",
            end = END,
        );
        let parsed = parse_response(&raw);
        assert_eq!(parsed.notes.as_deref(), Some("Hello note\n"));
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].path, "docs/features/F-1/contract.yaml");
        assert_eq!(parsed.files[0].contents, "key: value\n");
    }

    #[test]
    fn test_parse_response_ignores_embedded_delimiter() {
        // A FILE block whose contents contain an indented END must not terminate.
        let raw = format!(
            "{begin}\n--- SPECDRIVE:FILE docs/features/F-1/contract.yaml ---\nline1\n  --- SPECDRIVE:END ---\nline2\n{end}\n",
            begin = BEGIN,
            end = END,
        );
        let parsed = parse_response(&raw);
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(
            parsed.files[0].contents,
            "line1\n  --- SPECDRIVE:END ---\nline2\n"
        );
    }

    #[test]
    fn test_parse_response_stops_at_first_end() {
        let raw = format!(
            "{begin}\n--- SPECDRIVE:FILE docs/features/F-1/a.md ---\nkeep\n{end}\n--- SPECDRIVE:FILE docs/features/F-1/b.md ---\nignored\n",
            begin = BEGIN,
            end = END,
        );
        let parsed = parse_response(&raw);
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].path, "docs/features/F-1/a.md");
    }

    #[test]
    fn test_parse_response_empty_notes_is_none() {
        let raw = format!(
            "{begin}\n{notes}\n\n--- SPECDRIVE:FILE docs/features/F-1/a.md ---\nx\n{end}\n",
            begin = BEGIN,
            notes = NOTES,
            end = END,
        );
        let parsed = parse_response(&raw);
        assert_eq!(parsed.notes, None);
        assert_eq!(parsed.files.len(), 1);
    }

    #[test]
    fn test_is_absolute_pattern() {
        assert!(is_absolute_pattern("/etc/passwd"));
        assert!(is_absolute_pattern("\\\\server\\share"));
        assert!(is_absolute_pattern("C:\\Windows"));
        assert!(is_absolute_pattern("C:/Windows"));
        assert!(!is_absolute_pattern("docs/features/F-1/contract.yaml"));
        assert!(!is_absolute_pattern("contract.yaml"));
    }
}
