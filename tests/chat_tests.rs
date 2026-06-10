//! Integration tests for `specdrive chat export` / `chat import` (F-009).
//!
//! These run the built binary in a temporary git repository with the canonical
//! feature-local layout. They use subprocess working directories (not the
//! process cwd), so they are safe to run in parallel.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const BEGIN: &str = "--- SPECDRIVE:BEGIN ---";
const NOTES: &str = "--- SPECDRIVE:NOTES ---";
const PROMPT: &str = "--- SPECDRIVE:PROMPT ---";
const END: &str = "--- SPECDRIVE:END ---";

const VALID_CONTRACT: &str = "schema_version: 1\nmetadata:\n  id: F-001\n  title: Test\nrequirements:\n  high_level: []\n  low_level: []\n";

fn setup_test_repo() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("git config name");

    temp_dir
}

/// Lays out a complete feature with supporting docs and templates.
fn scaffold(repo: &Path, feature_id: &str) {
    let fdir = repo.join("docs/features").join(feature_id);
    fs::create_dir_all(&fdir).unwrap();
    fs::write(fdir.join("spec.md"), "# Spec\n\nSPEC_BODY\n").unwrap();
    fs::write(fdir.join("contract.yaml"), VALID_CONTRACT).unwrap();
    fs::create_dir_all(fdir.join("patches")).unwrap();

    let docs = repo.join("docs");
    fs::write(docs.join("constitution.md"), "# Constitution\nCONST_BODY\n").unwrap();
    fs::write(
        docs.join("system-overview.md"),
        "# Overview\nOVERVIEW_BODY\n",
    )
    .unwrap();

    let adrs = docs.join("adrs");
    fs::create_dir_all(&adrs).unwrap();
    fs::write(adrs.join("ADR-0001-x.md"), "# ADR 1\nADR_BODY\n").unwrap();

    let templates = docs.join("templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("feature.contract.minimal.yaml"),
        "minimal_template: true\n",
    )
    .unwrap();
    fs::write(
        templates.join("feature.contract.critical.yaml"),
        "critical_template: true\n",
    )
    .unwrap();
}

fn git_commit_all(repo: &Path) {
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(repo)
        .output()
        .unwrap();
}

fn run(repo: &Path, args: &[&str]) -> (i32, String, String) {
    run_with_stdin(repo, args, "")
}

fn run_with_stdin(repo: &Path, args: &[&str], stdin_data: &str) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_specdrive"))
        .args(args)
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn specdrive");

    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(stdin_data.as_bytes()).unwrap();
        // stdin dropped here -> EOF
    }

    let output = child.wait_with_output().expect("wait");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

// --- Export ---------------------------------------------------------------

/// TC-001 / TC-003: export draft prints a well-formed, ordered bundle.
#[test]
fn export_draft_prints_ordered_bundle() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let (code, stdout, _stderr) = run(repo, &["chat", "export", "draft", "F-001"]);
    assert_eq!(code, 0, "stdout: {}", stdout);

    let pb = stdout.find(BEGIN).expect("BEGIN");
    let pn = stdout.find(NOTES).expect("NOTES");
    let pf = stdout
        .find("--- SPECDRIVE:FILE ")
        .expect("at least one FILE");
    let pp = stdout.find(PROMPT).expect("PROMPT");
    let pe = stdout.find(END).expect("END");
    assert!(pb < pn && pn < pf && pf < pp && pp < pe, "ordering wrong");

    // Files are inlined.
    assert!(stdout.contains("SPEC_BODY"));
    assert!(stdout.contains("CONST_BODY"));
    assert!(stdout.contains("ADR_BODY"));
    // Draft includes templates.
    assert!(stdout.contains("minimal_template: true"));
    assert!(stdout.contains("critical_template: true"));
}

/// TC-002: export implement inlines implement files; templates excluded.
#[test]
fn export_implement_excludes_templates() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let (code, stdout, _stderr) = run(repo, &["chat", "export", "implement", "F-001"]);
    assert_eq!(code, 0);

    assert!(stdout.contains("SPEC_BODY"));
    assert!(stdout.contains("OVERVIEW_BODY"));
    // Implement must NOT include templates.
    assert!(!stdout.contains("minimal_template: true"));
    assert!(!stdout.contains("critical_template: true"));
}

/// TC-017 / AC-18: export succeeds on a dirty tree (no clean-tree check).
#[test]
fn export_succeeds_on_dirty_tree() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    // Dirty a tracked file.
    fs::write(repo.join("docs/features/F-001/spec.md"), "# Spec\nDIRTY\n").unwrap();

    let (code, stdout, _stderr) = run(repo, &["chat", "export", "draft", "F-001"]);
    assert_eq!(
        code, 0,
        "export must succeed on dirty tree; stdout: {}",
        stdout
    );
    assert!(stdout.contains(BEGIN));
}

/// E-003 / LLR-004: missing optional context warns but export still succeeds.
#[test]
fn export_warns_on_missing_context() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    // Remove an optional context file.
    fs::remove_file(repo.join("docs/system-overview.md")).unwrap();
    git_commit_all(repo);

    let (code, stdout, stderr) = run(repo, &["chat", "export", "draft", "F-001"]);
    assert_eq!(code, 0);
    assert!(stdout.contains(BEGIN));
    assert!(
        stderr.contains("system-overview.md"),
        "expected warning, stderr: {}",
        stderr
    );
}

/// E-002: export on a missing feature fails fast.
#[test]
fn export_missing_feature_fails() {
    let temp = setup_test_repo();
    let repo = temp.path();
    fs::create_dir_all(repo.join("docs")).unwrap();
    git_commit_all(repo);

    let (code, _stdout, stderr) = run(repo, &["chat", "export", "draft", "F-404"]);
    assert_eq!(code, 1);
    assert!(stderr.to_lowercase().contains("not found") || stderr.contains("spec"));
}

/// TC-009 / E-001 / AC-19: invalid FEATURE_ID rejected before filesystem work.
#[test]
fn export_rejects_invalid_feature_id() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let (code, _stdout, stderr) = run(repo, &["chat", "export", "draft", "../etc"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("invalid FEATURE_ID"));
}

// --- Import ---------------------------------------------------------------

fn draft_response(feature_id: &str, contract_body: &str, notes: Option<&str>) -> String {
    let mut s = String::new();
    s.push_str(BEGIN);
    s.push('\n');
    s.push_str(NOTES);
    s.push('\n');
    if let Some(n) = notes {
        s.push_str(n);
        s.push('\n');
    }
    s.push_str(&format!(
        "--- SPECDRIVE:FILE docs/features/{}/contract.yaml ---\n",
        feature_id
    ));
    s.push_str(contract_body);
    if !contract_body.ends_with('\n') {
        s.push('\n');
    }
    s.push_str(END);
    s.push('\n');
    s
}

/// TC-004 / AC-4: draft import replaces contract.yaml on confirmation; notes saved.
#[test]
fn import_draft_replaces_contract_and_saves_notes() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let new_contract = "schema_version: 1\nmetadata:\n  id: F-001\n  title: Updated\nrequirements:\n  high_level: []\n  low_level: []\n";
    let response = draft_response("F-001", new_contract, Some("LLR-003 may need revision."));
    let stdin = format!("{}y\n", response);

    let (code, stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "draft", "F-001"], &stdin);
    assert_eq!(code, 0, "stdout: {} stderr: {}", stdout, stderr);
    assert!(stdout.contains("Notes from AI:"));
    assert!(stdout.contains("Files to be written:"));

    let written = fs::read_to_string(repo.join("docs/features/F-001/contract.yaml")).unwrap();
    assert!(written.contains("title: Updated"));

    // Notes artifact exists.
    let notes = fs::read_to_string(repo.join("docs/features/F-001/outputs/notes-001.md")).unwrap();
    assert!(notes.contains("LLR-003 may need revision"));
}

/// TC-021 / E-013 / AC: declining writes nothing and exits cleanly.
#[test]
fn import_decline_writes_nothing() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let new_contract = "schema_version: 1\nmetadata:\n  id: F-001\n  title: Updated\nrequirements:\n  high_level: []\n  low_level: []\n";
    let response = draft_response("F-001", new_contract, None);
    let stdin = format!("{}n\n", response);

    let (code, _stdout, _stderr) =
        run_with_stdin(repo, &["chat", "import", "draft", "F-001"], &stdin);
    assert_eq!(code, 0);

    let written = fs::read_to_string(repo.join("docs/features/F-001/contract.yaml")).unwrap();
    assert!(
        written.contains("title: Test"),
        "contract must be unchanged"
    );
    assert!(!repo.join("docs/features/F-001/outputs").exists());
}

/// TC-013 / AC-9: no NOTES block produces no notes artifact.
#[test]
fn import_draft_no_notes_no_artifact() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    // Response without any NOTES block at all.
    let mut response = String::new();
    response.push_str(BEGIN);
    response.push('\n');
    response.push_str("--- SPECDRIVE:FILE docs/features/F-001/contract.yaml ---\n");
    response.push_str(VALID_CONTRACT);
    response.push_str(END);
    response.push('\n');
    let stdin = format!("{}y\n", response);

    let (code, _stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "draft", "F-001"], &stdin);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(
        !repo
            .join("docs/features/F-001/outputs/notes-001.md")
            .exists()
    );
}

/// TC-005 / AC-5 / AC-15: implement import saves raw to outputs/ only.
#[test]
fn import_implement_saves_raw_only() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let mut response = String::new();
    response.push_str(BEGIN);
    response.push('\n');
    response.push_str("--- SPECDRIVE:FILE docs/features/F-001/spec.md ---\n");
    response.push_str("UNIQUE_IMPLEMENT_OUTPUT\n");
    response.push_str(END);
    response.push('\n');
    let stdin = format!("{}y\n", response);

    let (code, stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "implement", "F-001"], &stdin);
    assert_eq!(code, 0, "stdout: {} stderr: {}", stdout, stderr);

    let raw =
        fs::read_to_string(repo.join("docs/features/F-001/outputs/implement-001.raw.md")).unwrap();
    assert!(raw.contains("UNIQUE_IMPLEMENT_OUTPUT"));

    // Spec must be untouched (implement never modifies source/spec).
    let spec = fs::read_to_string(repo.join("docs/features/F-001/spec.md")).unwrap();
    assert!(spec.contains("SPEC_BODY"));
    assert!(!spec.contains("UNIQUE_IMPLEMENT_OUTPUT"));
}

/// TC-006 / E-005 / AC-6: missing END delimiter is a hard failure.
#[test]
fn import_missing_end_fails() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let stdin = format!(
        "{}\n--- SPECDRIVE:FILE docs/features/F-001/contract.yaml ---\n{}",
        BEGIN, VALID_CONTRACT
    ); // no END

    let (code, _stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "draft", "F-001"], &stdin);
    assert_eq!(code, 1);
    assert!(stderr.contains("incomplete paste"), "stderr: {}", stderr);
}

/// E-006: no FILE blocks is a malformed response.
#[test]
fn import_no_file_blocks_fails() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let stdin = format!("{}\n{}\nsome notes\n{}\n", BEGIN, NOTES, END);
    let (code, _stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "draft", "F-001"], &stdin);
    assert_eq!(code, 1);
    assert!(stderr.contains("no SPECDRIVE:FILE"), "stderr: {}", stderr);
}

/// TC-008 / E-007 / AC-20: absolute FILE paths rejected; nothing written.
#[test]
fn import_rejects_absolute_path() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let mut response = String::new();
    response.push_str(BEGIN);
    response.push('\n');
    response.push_str("--- SPECDRIVE:FILE /etc/passwd ---\n");
    response.push_str("malicious\n");
    response.push_str(END);
    response.push('\n');
    let stdin = format!("{}y\n", response);

    let (code, _stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "implement", "F-001"], &stdin);
    assert_eq!(code, 1);
    assert!(stderr.contains("absolute"), "stderr: {}", stderr);
}

/// TC-007 / E-008 / AC-7: traversal outside the feature directory rejected.
#[test]
fn import_rejects_path_traversal() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let mut response = String::new();
    response.push_str(BEGIN);
    response.push('\n');
    response.push_str("--- SPECDRIVE:FILE docs/features/F-001/../../../etc/passwd ---\n");
    response.push_str("malicious\n");
    response.push_str(END);
    response.push('\n');
    let stdin = format!("{}y\n", response);

    let (code, _stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "implement", "F-001"], &stdin);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("outside") || stderr.contains("traversal"),
        "stderr: {}",
        stderr
    );
}

/// TC-010 / E-011 / AC-8: draft import rejects YAML lacking required structure.
#[test]
fn import_draft_rejects_bad_structure() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    // Valid YAML but missing required top-level sections.
    let response = draft_response("F-001", "just_a_key: true\n", None);
    let stdin = format!("{}y\n", response);

    let (code, _stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "draft", "F-001"], &stdin);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("metadata") || stderr.contains("requirements"),
        "stderr: {}",
        stderr
    );

    // Contract must be unchanged.
    let written = fs::read_to_string(repo.join("docs/features/F-001/contract.yaml")).unwrap();
    assert!(written.contains("title: Test"));
}

/// TC-018 / E-004 / AC-18: import fails fast on a dirty tree before any write.
#[test]
fn import_fails_on_dirty_tree() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    // Dirty a tracked file.
    fs::write(repo.join("docs/features/F-001/spec.md"), "# Spec\nDIRTY\n").unwrap();

    let new_contract = VALID_CONTRACT;
    let response = draft_response("F-001", new_contract, None);
    let stdin = format!("{}y\n", response);

    let (code, _stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "draft", "F-001"], &stdin);
    assert_eq!(code, 1);
    assert!(stderr.contains("not clean"), "stderr: {}", stderr);
}

/// TC-011 / AC-23: embedded delimiters in contents do not terminate blocks.
#[test]
fn import_embedded_delimiter_not_terminating() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    // Contract content contains an INDENTED END line that must be preserved.
    let body = "schema_version: 1\nmetadata:\n  id: F-001\n  note: \"  --- SPECDRIVE:END --- embedded\"\nrequirements:\n  high_level: []\n";
    let response = draft_response("F-001", body, None);
    let stdin = format!("{}y\n", response);

    let (code, stdout, stderr) =
        run_with_stdin(repo, &["chat", "import", "draft", "F-001"], &stdin);
    assert_eq!(code, 0, "stdout: {} stderr: {}", stdout, stderr);

    let written = fs::read_to_string(repo.join("docs/features/F-001/contract.yaml")).unwrap();
    assert!(
        written.contains("embedded"),
        "embedded delimiter content must be preserved: {}",
        written
    );
}

/// AC-11 / TC-020: existing draft/implement commands still behave correctly
/// after resolver extraction (paths listed, not inlined).
#[test]
fn existing_implement_unchanged() {
    let temp = setup_test_repo();
    let repo = temp.path();
    scaffold(repo, "F-001");
    git_commit_all(repo);

    let (code, stdout, stderr) = run(repo, &["implement", "F-001"]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(stdout.contains("docs/features/F-001/spec.md"));
    assert!(stdout.contains("docs/features/F-001/contract.yaml"));
    assert!(stdout.contains("docs/constitution.md"));
    // implement must NOT inline file contents.
    assert!(!stdout.contains("SPEC_BODY"));
}
