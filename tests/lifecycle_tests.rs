//! Integration tests for the F-010 lifecycle commands
//! (`status`, `review`, `done`, `block`, `defer`, `unblock`, `resume`).
//!
//! These run the built binary inside a temporary git repository with the
//! canonical feature-local layout. Each test uses a subprocess working
//! directory (not the process cwd), so they are safe to run in parallel.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Creates a temp git repo with a deterministic actor identity.
fn setup_test_repo() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temp dir");
    let repo = temp.path();

    Command::new("git").args(["init"]).current_dir(repo).output().expect("git init");
    Command::new("git")
        .args(["config", "user.email", "actor@example.com"])
        .current_dir(repo)
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "three-seat"])
        .current_dir(repo)
        .output()
        .expect("git config name");

    temp
}

/// Level of artifacts to scaffold, driving the inferred base state.
#[derive(Clone, Copy, PartialEq)]
enum Level {
    Draft,
    Contract,
    Patch,
}

/// Lays out a feature directory with artifacts up to the requested level.
fn scaffold(repo: &Path, feature_id: &str, level: Level) -> PathBuf {
    let fdir = repo.join("docs/features").join(feature_id);
    fs::create_dir_all(&fdir).unwrap();
    fs::write(
        fdir.join("spec.md"),
        format!("---\ntitle: Test Feature\n---\n# Spec for {feature_id}\n"),
    )
    .unwrap();

    if level == Level::Contract || level == Level::Patch {
        fs::write(
            fdir.join("contract.yaml"),
            "schema_version: 1\nmetadata:\n  id: test\n  title: Test Feature\n",
        )
        .unwrap();
    }
    if level == Level::Patch {
        let patches = fdir.join("patches");
        fs::create_dir_all(&patches).unwrap();
        fs::write(patches.join(format!("{feature_id}.patch")), "diff --git\n").unwrap();
    }
    fdir
}

fn run(repo: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_specdrive"))
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run specdrive")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}
fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn state_path(repo: &Path, feature_id: &str) -> PathBuf {
    repo.join("docs/features").join(feature_id).join("state.yaml")
}

// --- status (read-only) --------------------------------------------------

#[test]
fn status_infers_patch_and_writes_nothing() {
    // TC-003, TC-020, LLR-006, LLR-023: inferred patch, no state.yaml created.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Patch);

    let out = run(repo.path(), &["status", "F-1"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("Status:   patch"), "{s}");
    assert!(s.contains("Source:   inferred"), "{s}");
    assert!(s.contains("Since:    --"), "{s}");
    assert!(!state_path(repo.path(), "F-1").exists(), "status must not create state.yaml");
}

#[test]
fn status_infers_contract_and_draft() {
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-c", Level::Contract);
    scaffold(repo.path(), "F-d", Level::Draft);

    assert!(stdout(&run(repo.path(), &["status", "F-c"])).contains("Status:   contract"));
    assert!(stdout(&run(repo.path(), &["status", "F-d"])).contains("Status:   draft"));
}

#[test]
fn status_all_lists_every_feature() {
    // TC-002.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Patch);
    scaffold(repo.path(), "F-2", Level::Contract);

    let out = run(repo.path(), &["status", "--all"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("F-1"), "{s}");
    assert!(s.contains("F-2"), "{s}");
    assert!(s.contains("patch"), "{s}");
    assert!(s.contains("contract"), "{s}");
}

#[test]
fn status_missing_feature_rejects_exit_1() {
    // E-001.
    let repo = setup_test_repo();
    let out = run(repo.path(), &["status", "F-nope"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn status_reflects_contract_handoff_without_writing() {
    // TC-008, LLR-008: adding contract.yaml (F-009 handoff) is reflected by
    // inference on the next status call, writing nothing.
    let repo = setup_test_repo();
    let fdir = scaffold(repo.path(), "F-1", Level::Draft);
    assert!(stdout(&run(repo.path(), &["status", "F-1"])).contains("Status:   draft"));

    fs::write(fdir.join("contract.yaml"), "schema_version: 1\n").unwrap();
    assert!(stdout(&run(repo.path(), &["status", "F-1"])).contains("Status:   contract"));
    assert!(!state_path(repo.path(), "F-1").exists());
}

// --- review / done -------------------------------------------------------

#[test]
fn review_succeeds_and_creates_state_file() {
    // TC-009, TC-018: review from patch creates state.yaml with one event.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Patch);

    let out = run(repo.path(), &["review", "F-1"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("State advanced to: review"));

    let state = fs::read_to_string(state_path(repo.path(), "F-1")).unwrap();
    assert!(state.contains("feature_id: F-1"), "{state}");
    assert!(state.contains("status: review"), "{state}");
    assert!(state.contains("by: three-seat"), "actor from git config: {state}");
    // Exactly one event.
    assert_eq!(state.matches("- status:").count(), 1, "{state}");

    // And status now reports it as recorded.
    let st = stdout(&run(repo.path(), &["status", "F-1"]));
    assert!(st.contains("Status:   review"), "{st}");
    assert!(st.contains("Source:   recorded"), "{st}");
}

#[test]
fn review_rejects_when_not_patch() {
    // TC-010.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Contract);
    let out = run(repo.path(), &["review", "F-1"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("patch"), "{}", stderr(&out));
    assert!(!state_path(repo.path(), "F-1").exists());
}

#[test]
fn done_full_flow_and_rejections() {
    // TC-011: done requires base=review, a review event, and a patch artifact.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Patch);

    // done before review is rejected.
    let out = run(repo.path(), &["done", "F-1"]);
    assert_eq!(out.status.code(), Some(1));

    assert!(run(repo.path(), &["review", "F-1"]).status.success());
    let out = run(repo.path(), &["done", "F-1"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("State advanced to: done"));

    let state = fs::read_to_string(state_path(repo.path(), "F-1")).unwrap();
    assert!(state.contains("status: done"), "{state}");
    assert_eq!(state.matches("- status:").count(), 2, "{state}");
}

// --- block / defer / unblock / resume ------------------------------------

#[test]
fn block_records_overlay_and_status_shows_it() {
    // TC-012, TC-001: block --reason appends blocked event with previous_status.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Contract);

    let out = run(repo.path(), &["block", "F-1", "--reason", "waiting on F-012"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let state = fs::read_to_string(state_path(repo.path(), "F-1")).unwrap();
    assert!(state.contains("status: blocked"), "{state}");
    assert!(state.contains("reason: waiting on F-012"), "{state}");
    assert!(state.contains("previous_status: contract"), "{state}");

    let st = stdout(&run(repo.path(), &["status", "F-1"]));
    assert!(st.contains("Status:   blocked"), "{st}");
    assert!(st.contains("Base:     contract"), "{st}");
    assert!(st.contains("Reason:   waiting on F-012"), "{st}");
}

#[test]
fn block_without_reason_rejects() {
    // TC-012, AC-10.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Contract);
    let out = run(repo.path(), &["block", "F-1"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("--reason"), "{}", stderr(&out));
    assert!(!state_path(repo.path(), "F-1").exists());
}

#[test]
fn block_rejects_when_overlay_already_active() {
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Contract);
    assert!(run(repo.path(), &["block", "F-1", "--reason", "r1"]).status.success());
    let out = run(repo.path(), &["defer", "F-1", "--reason", "r2"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn block_rejects_when_done() {
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Patch);
    assert!(run(repo.path(), &["review", "F-1"]).status.success());
    assert!(run(repo.path(), &["done", "F-1"]).status.success());
    let out = run(repo.path(), &["block", "F-1", "--reason", "too late"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn unblock_returns_to_previous_base_state() {
    // TC-014, LLR-013.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Contract);
    assert!(run(repo.path(), &["block", "F-1", "--reason", "waiting"]).status.success());

    let out = run(repo.path(), &["unblock", "F-1"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("returned to: contract"), "{}", stdout(&out));

    let state = fs::read_to_string(state_path(repo.path(), "F-1")).unwrap();
    assert!(state.contains("via: unblock"), "{state}");

    let st = stdout(&run(repo.path(), &["status", "F-1"]));
    assert!(st.contains("Status:   contract"), "{st}");
    // Overlay resolved: no Base line, no Reason line.
    assert!(!st.contains("Base:"), "{st}");
}

#[test]
fn unblock_on_non_blocked_rejects() {
    // TC-014.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Contract);
    let out = run(repo.path(), &["unblock", "F-1"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn defer_and_resume_flow() {
    // TC-013, TC-015.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Patch);
    assert!(run(repo.path(), &["defer", "F-1", "--reason", "postponed"]).status.success());
    assert!(stdout(&run(repo.path(), &["status", "F-1"])).contains("Status:   deferred"));

    // Cannot unblock a deferred feature.
    assert_eq!(run(repo.path(), &["unblock", "F-1"]).status.code(), Some(1));

    let out = run(repo.path(), &["resume", "F-1"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let state = fs::read_to_string(state_path(repo.path(), "F-1")).unwrap();
    assert!(state.contains("via: resume"), "{state}");
    assert!(stdout(&run(repo.path(), &["status", "F-1"])).contains("Status:   patch"));
}

// --- append-only invariant -----------------------------------------------

#[test]
fn events_are_append_only() {
    // TC-016, LLR-015: every prior event is preserved unchanged in order.
    let repo = setup_test_repo();
    scaffold(repo.path(), "F-1", Level::Patch);

    run(repo.path(), &["block", "F-1", "--reason", "first blocker"]);
    run(repo.path(), &["unblock", "F-1"]);
    run(repo.path(), &["review", "F-1"]);
    run(repo.path(), &["done", "F-1"]);

    let state = fs::read_to_string(state_path(repo.path(), "F-1")).unwrap();
    // All four events are present, in order.
    let blocked = state.find("status: blocked").unwrap();
    let unblock = state.find("via: unblock").unwrap();
    let review = state.find("status: review").unwrap();
    let done = state.find("status: done").unwrap();
    assert!(blocked < unblock && unblock < review && review < done, "{state}");
    assert_eq!(state.matches("- status:").count(), 4, "{state}");
    // The original blocker reason is untouched.
    assert!(state.contains("reason: first blocker"), "{state}");
}
