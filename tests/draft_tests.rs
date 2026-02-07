use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper to create a temporary test directory with git initialized
fn setup_test_repo() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git");

    // Configure git for commits
    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to config git email");

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to config git name");

    temp_dir
}

/// Helper to run specdrive command in a directory
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

/// TC-002: If `.git/` is missing, the command prints a clear error and exits with code 1
#[test]
fn test_draft_without_git_repo() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Create .specify/ but no .git/
    fs::create_dir_all(repo_path.join(".specify")).expect("Failed to create .specify");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.to_lowercase().contains("not a git repository") || stderr.contains(".git"),
        "Expected git repo error, got: {}",
        stderr
    );
}

/// TC-003: If `.specify/` is missing, the command prints a clear error suggesting `specify init` and exits with code 1
#[test]
fn test_draft_without_specify_dir() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Git exists but no .specify/
    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("Spec Kit not initialized") || stderr.contains(".specify"),
        "Expected .specify error, got: {}",
        stderr
    );
    assert!(
        stderr.contains("specify init"),
        "Expected 'specify init' suggestion, got: {}",
        stderr
    );
}

/// TC-004: If the working tree is dirty, the command prints an error and exits with code 1
#[test]
fn test_draft_with_dirty_tree() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create .specify/ and basic structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-001"))
        .expect("Failed to create feature dir");
    fs::create_dir_all(repo_path.join("docs/templates")).expect("Failed to create templates dir");

    // Create spec and contract files
    fs::write(
        repo_path.join(".specify/specs/F-001.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-001/contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-001",
    )
    .expect("Failed to write contract");

    // Create template files
    fs::write(
        repo_path.join("docs/templates/feature.contract.minimal.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write minimal template");
    fs::write(
        repo_path.join("docs/templates/feature.contract.critical.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write critical template");

    // Create a dirty file (uncommitted change)
    fs::write(repo_path.join("dirty.txt"), "dirty content").expect("Failed to write dirty file");

    // Add and commit the essential files
    Command::new("git")
        .args(&["add", ".specify", "docs"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");

    // Now modify a tracked file to make tree dirty
    fs::write(
        repo_path.join(".specify/specs/F-001.spec.md"),
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

/// TC-005: If the spec file is missing, the command prints an error including the spec path and exits with code 2
#[test]
fn test_draft_with_missing_spec() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create .specify/ but no spec file
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-001"))
        .expect("Failed to create feature dir");
    fs::create_dir_all(repo_path.join("docs/templates")).expect("Failed to create templates dir");

    // Create contract but not spec
    fs::write(
        repo_path.join("docs/features/F-001/contract.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write contract");

    // Create template files
    fs::write(
        repo_path.join("docs/templates/feature.contract.minimal.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write minimal template");
    fs::write(
        repo_path.join("docs/templates/feature.contract.critical.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write critical template");

    // Commit to have clean tree
    Command::new("git")
        .args(&["add", "-A"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 2);
    assert!(
        stderr.contains("spec file not found") || stderr.contains(".spec.md"),
        "Expected spec file not found error, got: {}",
        stderr
    );
    assert!(
        stderr.contains("F-001"),
        "Expected feature ID in error, got: {}",
        stderr
    );
}

/// TC-006: If the contract skeleton file is missing, the command prints an error including the contract path and exits with code 2
#[test]
fn test_draft_with_missing_contract() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create .specify/ and spec file
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-001"))
        .expect("Failed to create feature dir");
    fs::create_dir_all(repo_path.join("docs/templates")).expect("Failed to create templates dir");

    // Create spec but not contract
    fs::write(
        repo_path.join(".specify/specs/F-001.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");

    // Create template files
    fs::write(
        repo_path.join("docs/templates/feature.contract.minimal.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write minimal template");
    fs::write(
        repo_path.join("docs/templates/feature.contract.critical.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write critical template");

    // Commit to have clean tree
    Command::new("git")
        .args(&["add", "-A"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");

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

/// TC-007: If either contract template file is missing, the command prints an error including the template path and exits with code 2
#[test]
fn test_draft_with_missing_minimal_template() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create basic structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-001"))
        .expect("Failed to create feature dir");
    fs::create_dir_all(repo_path.join("docs/templates")).expect("Failed to create templates dir");

    // Create spec and contract
    fs::write(
        repo_path.join(".specify/specs/F-001.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-001/contract.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write contract");

    // Create only critical template, not minimal
    fs::write(
        repo_path.join("docs/templates/feature.contract.critical.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write critical template");

    // Commit to have clean tree
    Command::new("git")
        .args(&["add", "-A"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");

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

/// TC-001 & TC-009: Success case - prints a structured, path-based prompt that lists paths but does not inline contents
#[test]
fn test_draft_success_with_basic_structure() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create full structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join(".specify/memory")).expect("Failed to create memory dir");
    fs::create_dir_all(repo_path.join("docs/features/F-001"))
        .expect("Failed to create feature dir");
    fs::create_dir_all(repo_path.join("docs/templates")).expect("Failed to create templates dir");
    fs::create_dir_all(repo_path.join("docs/adrs")).expect("Failed to create adrs dir");

    // Create spec with distinctive content
    fs::write(
        repo_path.join(".specify/specs/F-001.spec.md"),
        "# Test Spec\n\nThis is DISTINCTIVE_SPEC_CONTENT that should NOT be inlined.",
    )
    .expect("Failed to write spec");

    // Create contract with distinctive content
    fs::write(
        repo_path.join("docs/features/F-001/contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-001\n# DISTINCTIVE_CONTRACT_CONTENT",
    )
    .expect("Failed to write contract");

    // Create constitution
    fs::write(
        repo_path.join(".specify/memory/constitution.md"),
        "# Constitution",
    )
    .expect("Failed to write constitution");

    // Create system overview
    fs::write(
        repo_path.join("docs/system-overview.md"),
        "# System Overview",
    )
    .expect("Failed to write system overview");

    // Create an ADR
    fs::write(repo_path.join("docs/adrs/ADR-001-test.md"), "# ADR-001")
        .expect("Failed to write ADR");

    // Create template files
    fs::write(
        repo_path.join("docs/templates/feature.contract.minimal.yaml"),
        "schema_version: 1\n# Minimal template",
    )
    .expect("Failed to write minimal template");
    fs::write(
        repo_path.join("docs/templates/feature.contract.critical.yaml"),
        "schema_version: 1\n# Critical template",
    )
    .expect("Failed to write critical template");

    // Commit to have clean tree
    Command::new("git")
        .args(&["add", "-A"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");

    let (exit_code, stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    // Verify prompt structure
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

    // Verify it lists file paths
    assert!(
        stdout.contains(".specify/specs/F-001.spec.md"),
        "Expected spec path, got: {}",
        stdout
    );
    assert!(
        stdout.contains("docs/features/F-001/contract.yaml"),
        "Expected contract path, got: {}",
        stdout
    );
    assert!(
        stdout.contains(".specify/memory/constitution.md"),
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

    // Verify it does NOT inline the distinctive content (TC-009)
    assert!(
        !stdout.contains("DISTINCTIVE_SPEC_CONTENT"),
        "Prompt should not inline spec content"
    );
    assert!(
        !stdout.contains("DISTINCTIVE_CONTRACT_CONTENT"),
        "Prompt should not inline contract content"
    );

    // Verify guidance is present
    assert!(
        stdout.contains("Guidance"),
        "Expected guidance section, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Guardrails"),
        "Expected guardrails section, got: {}",
        stdout
    );
}

/// TC-008: If header and footer files exist, their contents appear in the prompt
#[test]
fn test_draft_with_header_and_footer() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create full structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-001"))
        .expect("Failed to create feature dir");
    fs::create_dir_all(repo_path.join("docs/templates")).expect("Failed to create templates dir");
    fs::create_dir_all(repo_path.join("docs/ai")).expect("Failed to create ai dir");

    // Create spec and contract
    fs::write(
        repo_path.join(".specify/specs/F-001.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-001/contract.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write contract");

    // Create template files
    fs::write(
        repo_path.join("docs/templates/feature.contract.minimal.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write minimal template");
    fs::write(
        repo_path.join("docs/templates/feature.contract.critical.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write critical template");

    // Create header and footer
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

    // Commit to have clean tree
    Command::new("git")
        .args(&["add", "-A"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");

    let (exit_code, stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    // Verify header appears at the beginning
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("CUSTOM_HEADER_CONTENT")),
        "Expected header content in prompt"
    );

    // Verify footer appears at the end
    assert!(
        stdout.contains("CUSTOM_FOOTER_CONTENT"),
        "Expected footer content in prompt"
    );

    // Verify header comes before the main content
    let header_pos = stdout.find("CUSTOM_HEADER_CONTENT");
    let main_pos = stdout.find("drafting/refining");
    assert!(
        header_pos < main_pos,
        "Header should appear before main content"
    );

    // Verify footer comes after the main content
    let footer_pos = stdout.find("CUSTOM_FOOTER_CONTENT");
    assert!(
        footer_pos > main_pos,
        "Footer should appear after main content"
    );
}

/// TC-010: Running the command does not create, modify, or delete any files (read-only)
#[test]
fn test_draft_is_read_only() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create minimal structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-001"))
        .expect("Failed to create feature dir");
    fs::create_dir_all(repo_path.join("docs/templates")).expect("Failed to create templates dir");

    fs::write(
        repo_path.join(".specify/specs/F-001.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-001/contract.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write contract");
    fs::write(
        repo_path.join("docs/templates/feature.contract.minimal.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write minimal template");
    fs::write(
        repo_path.join("docs/templates/feature.contract.critical.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write critical template");

    // Commit to have clean tree
    Command::new("git")
        .args(&["add", "-A"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");

    // Get initial git status
    let status_before = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get git status");

    // Run draft command
    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["draft", "F-001"]);
    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    // Get git status after
    let status_after = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get git status");

    // Verify no changes
    assert_eq!(
        status_before.stdout, status_after.stdout,
        "Git status changed after draft command - command is not read-only!"
    );
}
