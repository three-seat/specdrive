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

/// TC-003: Running `patch emit` outside a Git repository fails with a clear error
#[test]
fn test_patch_emit_without_git_repo() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Create .specify/ but no .git/
    fs::create_dir_all(repo_path.join(".specify")).expect("Failed to create .specify");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.to_lowercase().contains("not a git repository") || stderr.contains(".git"),
        "Expected git repo error, got: {}",
        stderr
    );
}

/// Test: If `.specify/` is missing, the command prints a clear error and exits with code 1
#[test]
fn test_patch_emit_without_specify_dir() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Git exists but no .specify/
    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains(".specify")
            && (stderr.contains("not found") || stderr.contains("directory")),
        "Expected .specify error, got: {}",
        stderr
    );
    assert!(
        stderr.contains("specify init"),
        "Expected 'specify init' suggestion, got: {}",
        stderr
    );
}

/// TC-004: Running `patch emit` with an empty diff fails with a clear error
#[test]
fn test_patch_emit_with_empty_diff() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create basic structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-006"))
        .expect("Failed to create feature dir");

    // Create spec and contract files
    fs::write(
        repo_path.join(".specify/specs/F-006.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-006/contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-006",
    )
    .expect("Failed to write contract");

    // Commit everything to have a clean tree with no diff
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

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("No implementation changes") || stderr.contains("empty"),
        "Expected empty diff error, got: {}",
        stderr
    );
}

/// TC-001 & TC-002: With a non-empty Git diff, patch emit writes a valid patch file
#[test]
fn test_patch_emit_success_with_changes() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create basic structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-006"))
        .expect("Failed to create feature dir");

    // Create spec and contract files
    fs::write(
        repo_path.join(".specify/specs/F-006.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-006/contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-006",
    )
    .expect("Failed to write contract");

    // Create and commit a test file
    fs::write(repo_path.join("test.txt"), "original content").expect("Failed to write test file");

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

    // Modify the file to create a diff
    fs::write(repo_path.join("test.txt"), "modified content").expect("Failed to modify test file");

    // Run patch emit
    let (exit_code, stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    // Verify patch file was created (TC-001)
    let patch_file = repo_path.join("docs/features/F-006/patches/F-006.patch");
    assert!(
        patch_file.exists(),
        "Patch file should exist at docs/features/F-006/patches/F-006.patch"
    );

    // Verify patch file contains the diff
    let patch_content = fs::read_to_string(&patch_file).expect("Failed to read patch file");
    assert!(
        patch_content.contains("test.txt"),
        "Patch should contain test.txt"
    );
    assert!(
        patch_content.contains("original content") || patch_content.contains("modified content"),
        "Patch should contain file changes"
    );

    // Verify the patch passes git apply --check (TC-002)
    // We need to reset the working tree first since it still has the changes
    Command::new("git")
        .args(&["reset", "--hard", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to reset working tree");

    let check_output = Command::new("git")
        .args(&[
            "apply",
            "--check",
            "docs/features/F-006/patches/F-006.patch",
        ])
        .current_dir(repo_path)
        .output()
        .expect("Failed to run git apply --check");

    assert!(
        check_output.status.success(),
        "Patch should pass git apply --check. stderr: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    // Verify output summary (TC-005)
    assert!(
        stdout.contains("Patch emitted successfully"),
        "Expected success message, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Path:"),
        "Expected patch path in output, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Files changed:"),
        "Expected files changed in output, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Insertions:"),
        "Expected insertions in output, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Deletions:"),
        "Expected deletions in output, got: {}",
        stdout
    );
}

/// TC-010: Untracked files trigger a warning and are not included automatically
#[test]
fn test_patch_emit_warns_about_untracked_files() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create basic structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-006"))
        .expect("Failed to create feature dir");

    // Create spec and contract files
    fs::write(
        repo_path.join(".specify/specs/F-006.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-006/contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-006",
    )
    .expect("Failed to write contract");

    // Create and commit a test file
    fs::write(repo_path.join("test.txt"), "original content").expect("Failed to write test file");

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

    // Modify tracked file
    fs::write(repo_path.join("test.txt"), "modified content").expect("Failed to modify test file");

    // Create an untracked file
    fs::write(repo_path.join("untracked.txt"), "untracked content")
        .expect("Failed to write untracked file");

    // Run patch emit
    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success");

    // Verify warning about untracked files
    assert!(
        stderr.contains("warning: untracked files detected"),
        "Expected untracked files warning, got: {}",
        stderr
    );
    assert!(
        stderr.contains("untracked.txt"),
        "Expected untracked file name in warning, got: {}",
        stderr
    );
    assert!(
        stderr.contains("git add") || stderr.contains("stage"),
        "Expected instruction about staging, got: {}",
        stderr
    );

    // Verify patch file does not contain untracked file
    let patch_file = repo_path.join("docs/features/F-006/patches/F-006.patch");
    let patch_content = fs::read_to_string(&patch_file).expect("Failed to read patch file");
    assert!(
        !patch_content.contains("untracked.txt"),
        "Patch should not contain untracked file"
    );
}

/// TC-009: Staged newly created files are included in emitted patches
#[test]
fn test_patch_emit_includes_staged_new_files() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create basic structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-006"))
        .expect("Failed to create feature dir");

    // Create spec and contract files
    fs::write(
        repo_path.join(".specify/specs/F-006.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-006/contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-006",
    )
    .expect("Failed to write contract");

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

    // Create and stage a new file
    fs::write(repo_path.join("new_file.txt"), "new file content")
        .expect("Failed to write new file");

    Command::new("git")
        .args(&["add", "new_file.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add new file");

    // Run patch emit
    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    // Verify patch file contains the new file
    let patch_file = repo_path.join("docs/features/F-006/patches/F-006.patch");
    let patch_content = fs::read_to_string(&patch_file).expect("Failed to read patch file");
    assert!(
        patch_content.contains("new_file.txt"),
        "Patch should contain staged new file"
    );
    assert!(
        patch_content.contains("new file content"),
        "Patch should contain new file content"
    );
}

/// TC-008: The generated patch does not include the patch artifact file itself
#[test]
fn test_patch_emit_excludes_itself() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create basic structure
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-006"))
        .expect("Failed to create feature dir");

    // Create spec and contract files
    fs::write(
        repo_path.join(".specify/specs/F-006.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-006/contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-006",
    )
    .expect("Failed to write contract");

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

    // Create and commit a test file first
    fs::write(repo_path.join("test.txt"), "initial test content")
        .expect("Failed to write test file");
    Command::new("git")
        .args(&["add", "test.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add test file");
    Command::new("git")
        .args(&["commit", "-m", "Add test file"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit test file");

    // Now modify it to create a diff
    fs::write(repo_path.join("test.txt"), "modified test content")
        .expect("Failed to modify test file");

    // Run patch emit once
    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);
    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    // Stage the patch file (simulating it being in the working tree)
    Command::new("git")
        .args(&["add", "docs/features/F-006/patches/F-006.patch"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add patch");

    // Modify test file again (make it different from the first modification)
    fs::write(repo_path.join("test.txt"), "further modified test content")
        .expect("Failed to modify test file");

    // Run patch emit again
    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);
    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    // Read the patch file
    let patch_file = repo_path.join("docs/features/F-006/patches/F-006.patch");
    let patch_content = fs::read_to_string(&patch_file).expect("Failed to read patch file");

    // The patch should contain test.txt but NOT reference itself
    assert!(
        patch_content.contains("test.txt"),
        "Patch should contain test.txt changes"
    );

    // Count how many times "F-006.patch" appears in the diff content
    // It should not appear in diff headers (we filter those out)
    let patch_references: Vec<&str> = patch_content.matches("F-006.patch").collect();
    // If it appears, it should only be in comments or metadata, not in actual diff blocks
    // The safest check is to ensure "diff --git" lines don't reference the patch file
    for line in patch_content.lines() {
        if line.starts_with("diff --git") {
            assert!(
                !line.contains("F-006.patch"),
                "Patch file should not include itself in diff headers"
            );
        }
    }
}

/// TC-011: If patches directory doesn't exist, SpecDrive creates it before writing the patch
#[test]
fn test_patch_emit_creates_directory() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create basic structure WITHOUT patches directory
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-006"))
        .expect("Failed to create feature dir");

    // Create spec and contract files
    fs::write(
        repo_path.join(".specify/specs/F-006.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");
    fs::write(
        repo_path.join("docs/features/F-006/contract.yaml"),
        "schema_version: 1\nmetadata:\n  id: F-006",
    )
    .expect("Failed to write contract");

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

    // Create and commit a test file, then modify it
    fs::write(repo_path.join("test.txt"), "initial test content")
        .expect("Failed to write test file");
    Command::new("git")
        .args(&["add", "test.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add test file");
    Command::new("git")
        .args(&["commit", "-m", "Add test file"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit test file");

    // Now modify it
    fs::write(repo_path.join("test.txt"), "modified test content")
        .expect("Failed to modify test file");

    // Verify patches directory doesn't exist
    let patches_dir = repo_path.join("docs/features/F-006/patches");
    assert!(
        !patches_dir.exists(),
        "Patches directory should not exist yet"
    );

    // Run patch emit
    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    // Verify patches directory was created
    assert!(
        patches_dir.exists(),
        "Patches directory should have been created"
    );
    assert!(patches_dir.is_dir(), "Patches path should be a directory");

    // Verify patch file exists
    let patch_file = patches_dir.join("F-006.patch");
    assert!(patch_file.exists(), "Patch file should exist");
}

/// Test: Running `patch emit` with missing spec file fails
#[test]
fn test_patch_emit_with_missing_spec() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create .specify/ but no spec file
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-006"))
        .expect("Failed to create feature dir");

    // Create contract but not spec
    fs::write(
        repo_path.join("docs/features/F-006/contract.yaml"),
        "schema_version: 1",
    )
    .expect("Failed to write contract");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("spec file not found") || stderr.contains(".spec.md"),
        "Expected spec file not found error, got: {}",
        stderr
    );
}

/// Test: Running `patch emit` with missing contract file fails
#[test]
fn test_patch_emit_with_missing_contract() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    // Create .specify/ and spec file
    fs::create_dir_all(repo_path.join(".specify/specs")).expect("Failed to create specs dir");
    fs::create_dir_all(repo_path.join("docs/features/F-006"))
        .expect("Failed to create feature dir");

    // Create spec but not contract
    fs::write(
        repo_path.join(".specify/specs/F-006.spec.md"),
        "# Test Spec",
    )
    .expect("Failed to write spec");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("contract") && (stderr.contains("not found") || stderr.contains("missing")),
        "Expected contract not found error, got: {}",
        stderr
    );
}
