use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper to create a temporary test directory with git initialized.
fn setup_test_repo() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to config git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to config git name");

    temp_dir
}

fn run_specdrive(repo_path: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_specdrive"))
        .args(args)
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute specdrive");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

/// Minimal feature-local scaffolding under `docs/features/<id>/`.
fn write_feature(repo_path: &Path, feature_id: &str, with_spec: bool, with_contract: bool) {
    let feature_dir = repo_path.join("docs/features").join(feature_id);
    fs::create_dir_all(&feature_dir).expect("Failed to create feature dir");

    if with_spec {
        fs::write(feature_dir.join("spec.md"), "# Test Spec").expect("Failed to write spec");
    }
    if with_contract {
        fs::write(
            feature_dir.join("contract.yaml"),
            "schema_version: 1\nmetadata:\n  id: F-001",
        )
        .expect("Failed to write contract");
    }
}

fn write_contract_templates(repo_path: &Path, minimal: bool, critical: bool) {
    let templates = repo_path.join("docs/templates");
    fs::create_dir_all(&templates).expect("Failed to create templates dir");
    if minimal {
        fs::write(
            templates.join("feature.contract.minimal.yaml"),
            "schema_version: 1",
        )
        .expect("Failed to write minimal template");
    }
    if critical {
        fs::write(
            templates.join("feature.contract.critical.yaml"),
            "schema_version: 1",
        )
        .expect("Failed to write critical template");
    }
}

fn git_commit_all(repo_path: &Path) {
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");
}

/// If `.git/` is missing, the command prints a clear error and exits with code 1.
#[test]
fn test_draft_without_git_repo() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.to_lowercase().contains("not a git repository") || stderr.contains(".git"),
        "Expected git repo error, got: {}",
        stderr
    );
}

/// Per F-007 LLR-005: draft must not fail solely because `.specify/` is absent.
#[test]
fn test_draft_without_specify_dir_succeeds() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // No .specify/ anywhere — only the new feature-local layout.
    write_feature(repo_path, "F-001", true, true);
    write_contract_templates(repo_path, true, true);
    git_commit_all(repo_path);

    let (exit_code, stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);
    assert!(
        stdout.contains("drafting/refining"),
        "Expected drafting intro, got: {}",
        stdout
    );
}

/// If the working tree is dirty, the command prints an error and exits with code 1.
#[test]
fn test_draft_with_dirty_tree() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-001", true, true);
    write_contract_templates(repo_path, true, true);
    git_commit_all(repo_path);

    // Now modify a tracked file to make the tree dirty.
    fs::write(
        repo_path.join("docs/features/F-001/spec.md"),
        "# Modified Spec",
    )
    .expect("Failed to modify spec");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("not clean") || stderr.contains("commit") || stderr.contains("stash"),
        "Expected dirty tree error, got: {}",
        stderr
    );
}

/// If the spec file is missing, the command prints an error including the spec path and exits with code 2.
#[test]
fn test_draft_with_missing_spec() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-001", false, true);
    write_contract_templates(repo_path, true, true);
    git_commit_all(repo_path);

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 2);
    assert!(
        stderr.contains("spec file not found") || stderr.contains("spec.md"),
        "Expected spec file not found error, got: {}",
        stderr
    );
    assert!(
        stderr.contains("F-001"),
        "Expected feature ID in error, got: {}",
        stderr
    );
}

/// If the contract is missing, the command prints an error including the contract path and exits with code 2.
#[test]
fn test_draft_with_missing_contract() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-001", true, false);
    write_contract_templates(repo_path, true, true);
    git_commit_all(repo_path);

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 2);
    assert!(
        stderr.contains("contract") && (stderr.contains("not found") || stderr.contains("missing")),
        "Expected contract not found error, got: {}",
        stderr
    );
    assert!(
        stderr.contains("contract.yaml"),
        "Expected contract.yaml in error, got: {}",
        stderr
    );
}

/// If either contract template file is missing, the command prints an error and exits with code 2.
#[test]
fn test_draft_with_missing_minimal_template() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-001", true, true);
    // Only critical template, not minimal.
    write_contract_templates(repo_path, false, true);
    git_commit_all(repo_path);

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 2);
    assert!(
        stderr.contains("template") && (stderr.contains("not found") || stderr.contains("missing")),
        "Expected template not found error, got: {}",
        stderr
    );
    assert!(
        stderr.contains("minimal"),
        "Expected 'minimal' in error, got: {}",
        stderr
    );
}

/// Success case: prints a structured, path-based prompt that lists paths but does not inline contents.
#[test]
fn test_draft_success_with_basic_structure() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    let feature_dir = repo_path.join("docs/features/F-001");
    fs::create_dir_all(&feature_dir).expect("Failed to create feature dir");

    fs::write(
        feature_dir.join("spec.md"),
        "# Test Spec\n\nThis is DISTINCTIVE_SPEC_CONTENT that should NOT be inlined.",
    )
    .expect("Failed to write spec");
    fs::write(
        feature_dir.join("contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-001\n# DISTINCTIVE_CONTRACT_CONTENT",
    )
    .expect("Failed to write contract");

    fs::create_dir_all(repo_path.join("docs/adrs")).expect("Failed to create adrs dir");
    fs::write(repo_path.join("docs/constitution.md"), "# Constitution")
        .expect("Failed to write constitution");
    fs::write(
        repo_path.join("docs/system-overview.md"),
        "# System Overview",
    )
    .expect("Failed to write system overview");
    fs::write(repo_path.join("docs/adrs/ADR-001-test.md"), "# ADR-001")
        .expect("Failed to write ADR");

    write_contract_templates(repo_path, true, true);
    git_commit_all(repo_path);

    let (exit_code, stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    assert!(
        stdout.contains("drafting/refining"),
        "Expected drafting intro, got: {}",
        stdout
    );
    assert!(
        stdout.contains("F-001"),
        "Expected feature ID, got: {}",
        stdout
    );

    assert!(
        stdout.contains("docs/features/F-001/spec.md"),
        "Expected spec path, got: {}",
        stdout
    );
    assert!(
        stdout.contains("docs/features/F-001/contract.yaml"),
        "Expected contract path, got: {}",
        stdout
    );
    assert!(
        stdout.contains("docs/constitution.md"),
        "Expected constitution path, got: {}",
        stdout
    );
    assert!(
        stdout.contains("docs/system-overview.md"),
        "Expected system overview path, got: {}",
        stdout
    );
    assert!(
        stdout.contains("docs/adrs"),
        "Expected ADRs mention, got: {}",
        stdout
    );
    assert!(
        stdout.contains("feature.contract.minimal.yaml"),
        "Expected minimal template path, got: {}",
        stdout
    );
    assert!(
        stdout.contains("feature.contract.critical.yaml"),
        "Expected critical template path, got: {}",
        stdout
    );

    // Per spec: prompt does not inline spec/contract contents.
    assert!(
        !stdout.contains("DISTINCTIVE_SPEC_CONTENT"),
        "Prompt should not inline spec content"
    );
    assert!(
        !stdout.contains("DISTINCTIVE_CONTRACT_CONTENT"),
        "Prompt should not inline contract content"
    );

    assert!(stdout.contains("Guidance"), "Expected guidance section");
    assert!(stdout.contains("Guardrails"), "Expected guardrails section");
}

/// If header and footer files exist, their contents appear in the prompt.
#[test]
fn test_draft_with_header_and_footer() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-001", true, true);
    write_contract_templates(repo_path, true, true);

    fs::create_dir_all(repo_path.join("docs/ai")).expect("Failed to create ai dir");
    fs::write(
        repo_path.join("docs/ai/draft-header.md"),
        "CUSTOM_HEADER_CONTENT\n",
    )
    .expect("Failed to write header");
    fs::write(
        repo_path.join("docs/ai/draft-footer.md"),
        "CUSTOM_FOOTER_CONTENT\n",
    )
    .expect("Failed to write footer");

    git_commit_all(repo_path);

    let (exit_code, stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    assert!(
        stdout.contains("CUSTOM_HEADER_CONTENT"),
        "Expected header content"
    );
    assert!(
        stdout.contains("CUSTOM_FOOTER_CONTENT"),
        "Expected footer content"
    );

    let header_pos = stdout.find("CUSTOM_HEADER_CONTENT").unwrap();
    let main_pos = stdout.find("drafting/refining").unwrap();
    let footer_pos = stdout.find("CUSTOM_FOOTER_CONTENT").unwrap();

    assert!(header_pos < main_pos, "Header should precede main content");
    assert!(footer_pos > main_pos, "Footer should follow main content");
}

/// Running the command does not create, modify, or delete any files (read-only).
#[test]
fn test_draft_is_read_only() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-001", true, true);
    write_contract_templates(repo_path, true, true);
    git_commit_all(repo_path);

    let status_before = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get git status");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);
    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    let status_after = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get git status");

    assert_eq!(
        status_before.stdout, status_after.stdout,
        "Git status changed after draft command — command is not read-only!"
    );
}
