use std::fs;
use std::path::Path;
use std::process::Command;

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

/// Write the canonical feature-local spec and contract under `docs/features/<id>/`.
fn write_feature(repo_path: &Path, feature_id: &str) {
    let feature_dir = repo_path.join("docs/features").join(feature_id);
    fs::create_dir_all(&feature_dir).expect("Failed to create feature dir");
    fs::write(feature_dir.join("spec.md"), "# Test Spec").expect("Failed to write spec");
    fs::write(
        feature_dir.join("contract.yaml"),
        format!("schema_version: 1\nmetadata:\n  id: {}", feature_id),
    )
    .expect("Failed to write contract");
}

/// TC-003: Running `patch emit` outside a Git repository fails with a clear error.
#[test]
fn test_patch_emit_without_git_repo() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.to_lowercase().contains("not a git repository") || stderr.contains(".git"),
        "Expected git repo error, got: {}",
        stderr
    );
}

/// Per F-007 LLR-005: `patch emit` must not fail solely because `.specify/` is absent.
#[test]
fn test_patch_emit_without_specify_dir_succeeds() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-006");
    fs::write(repo_path.join("test.txt"), "original content").expect("Failed to write test file");

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

    fs::write(repo_path.join("test.txt"), "modified content").expect("Failed to modify test file");

    let (exit_code, stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);
    assert!(
        stdout.contains("Patch emitted successfully"),
        "Expected success output, got: {}",
        stdout
    );
}

/// TC-004: Running `patch emit` with an empty diff fails with a clear error.
#[test]
fn test_patch_emit_with_empty_diff() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-006");

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

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("No implementation changes") || stderr.contains("empty"),
        "Expected empty diff error, got: {}",
        stderr
    );
}

/// TC-001 & TC-002: With a non-empty Git diff, patch emit writes a valid patch file.
#[test]
fn test_patch_emit_success_with_changes() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-006");
    fs::write(repo_path.join("test.txt"), "original content").expect("Failed to write test file");

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

    fs::write(repo_path.join("test.txt"), "modified content").expect("Failed to modify test file");

    let (exit_code, stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    let patch_file = repo_path.join("docs/features/F-006/patches/F-006.patch");
    assert!(
        patch_file.exists(),
        "Patch file should exist at docs/features/F-006/patches/F-006.patch"
    );

    let patch_content = fs::read_to_string(&patch_file).expect("Failed to read patch file");
    assert!(
        patch_content.contains("test.txt"),
        "Patch should contain test.txt"
    );
    assert!(
        patch_content.contains("original content") || patch_content.contains("modified content"),
        "Patch should contain file changes"
    );

    Command::new("git")
        .args(["reset", "--hard", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to reset working tree");

    let check_output = Command::new("git")
        .args([
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

    assert!(stdout.contains("Patch emitted successfully"));
    assert!(stdout.contains("Path:"));
    assert!(stdout.contains("Files changed:"));
    assert!(stdout.contains("Insertions:"));
    assert!(stdout.contains("Deletions:"));
}

/// TC-010: Untracked files trigger a warning and are not included automatically.
#[test]
fn test_patch_emit_warns_about_untracked_files() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-006");
    fs::write(repo_path.join("test.txt"), "original content").expect("Failed to write test file");

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

    fs::write(repo_path.join("test.txt"), "modified content").expect("Failed to modify test file");
    fs::write(repo_path.join("untracked.txt"), "untracked content")
        .expect("Failed to write untracked file");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success");

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

    let patch_file = repo_path.join("docs/features/F-006/patches/F-006.patch");
    let patch_content = fs::read_to_string(&patch_file).expect("Failed to read patch file");
    assert!(
        !patch_content.contains("untracked.txt"),
        "Patch should not contain untracked file"
    );
}

/// TC-009: Staged newly created files are included in emitted patches.
#[test]
fn test_patch_emit_includes_staged_new_files() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-006");

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

    fs::write(repo_path.join("new_file.txt"), "new file content")
        .expect("Failed to write new file");

    Command::new("git")
        .args(["add", "new_file.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add new file");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

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

/// TC-008: The generated patch does not include the patch artifact file itself.
#[test]
fn test_patch_emit_excludes_itself() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-006");

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

    fs::write(repo_path.join("test.txt"), "initial test content")
        .expect("Failed to write test file");
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add test file");
    Command::new("git")
        .args(["commit", "-m", "Add test file"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit test file");

    fs::write(repo_path.join("test.txt"), "modified test content")
        .expect("Failed to modify test file");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);
    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    Command::new("git")
        .args(["add", "docs/features/F-006/patches/F-006.patch"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add patch");

    fs::write(repo_path.join("test.txt"), "further modified test content")
        .expect("Failed to modify test file");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);
    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    let patch_file = repo_path.join("docs/features/F-006/patches/F-006.patch");
    let patch_content = fs::read_to_string(&patch_file).expect("Failed to read patch file");

    assert!(
        patch_content.contains("test.txt"),
        "Patch should contain test.txt changes"
    );

    for line in patch_content.lines() {
        if line.starts_with("diff --git") {
            assert!(
                !line.contains("F-006.patch"),
                "Patch file should not include itself in diff headers"
            );
        }
    }
}

/// TC-011: If patches/ doesn't exist, SpecDrive creates it before writing the patch.
#[test]
fn test_patch_emit_creates_directory() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    write_feature(repo_path, "F-006");

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

    fs::write(repo_path.join("test.txt"), "initial test content")
        .expect("Failed to write test file");
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add test file");
    Command::new("git")
        .args(["commit", "-m", "Add test file"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit test file");

    fs::write(repo_path.join("test.txt"), "modified test content")
        .expect("Failed to modify test file");

    let patches_dir = repo_path.join("docs/features/F-006/patches");
    assert!(
        !patches_dir.exists(),
        "Patches directory should not exist yet"
    );

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 0, "Expected success, stderr: {}", stderr);

    assert!(
        patches_dir.exists(),
        "Patches directory should have been created"
    );
    assert!(patches_dir.is_dir(), "Patches path should be a directory");

    let patch_file = patches_dir.join("F-006.patch");
    assert!(patch_file.exists(), "Patch file should exist");
}

/// Running `patch emit` with missing spec file fails.
#[test]
fn test_patch_emit_with_missing_spec() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    let feature_dir = repo_path.join("docs/features/F-006");
    fs::create_dir_all(&feature_dir).expect("Failed to create feature dir");
    fs::write(feature_dir.join("contract.yaml"), "schema_version: 1")
        .expect("Failed to write contract");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("spec file not found") || stderr.contains("spec.md"),
        "Expected spec file not found error, got: {}",
        stderr
    );
}

/// Running `patch emit` with missing contract file fails.
#[test]
fn test_patch_emit_with_missing_contract() {
    let temp_dir = setup_test_repo();
    let repo_path = temp_dir.path();

    let feature_dir = repo_path.join("docs/features/F-006");
    fs::create_dir_all(&feature_dir).expect("Failed to create feature dir");
    fs::write(feature_dir.join("spec.md"), "# Test Spec").expect("Failed to write spec");

    let (exit_code, _stdout, stderr) = run_specdrive(repo_path, &["patch", "emit", "F-006"]);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("contract") && (stderr.contains("not found") || stderr.contains("missing")),
        "Expected contract not found error, got: {}",
        stderr
    );
}
