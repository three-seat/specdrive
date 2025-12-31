---
id: F-001-bootstrap
title: 001 Bootstrap
type: feature
system: specdrive
status: implemented
area: cli
owners:
  - three_seat
created_at: 2025-12-29
contract: docs/features/F-001-bootstrap/contract.yaml
---

# Summary
Add a `specdrive bootstrap` command that prepares an **existing** project for
spec-driven, AI-assisted development. The command verifies that the project is
a git repo and that Spec Kit has already been initialized, then creates the
standard `.specify` and `docs` scaffolding (specs + templates) using
specdrive’s embedded assets **without overwriting any existing files**.

# Context
- What problem does this solve?
  - Right now, using Spec Kit + specdrive in a new repo requires manual setup of
    `.specify/specs`, `.specify/templates`, and `docs/*` templates.
  - There is no single command that:
    - Checks that the repo is properly initialized (git + `specify init`).
    - Installs the standard feature spec and contract templates.
    - Does all of this safely without clobbering user content.
- Any constraints (OS, tooling, repo layout, compatibility)?
  - Must be run from the **project root** (where `.git/` and `.specify/` live).
  - Requires:
    - A git repository (`.git/` directory present).
    - Spec Kit already initialized (`specify init` → `.specify/` exists).
  - No network calls.
  - No external crates (std only).
  - The specdrive binary embeds the bootstrap templates at:
    - `assets/bootstrap/feature.spec.md`
    - `assets/bootstrap/feature.contract.minimal.yaml`
    - `assets/bootstrap/feature.contract.critical.yaml`

# Behavior

## User flow
- Command / entrypoint:
  - `specdrive bootstrap`
- Inputs:
  - No positional arguments or flags for v0.1.
- Outputs:
  - On success:
    - Creates missing directories and template files listed below.
    - Prints a short summary of what was created vs skipped.
  - On failure:
    - Prints a clear error message (e.g., “not a git repo”, “.specify missing”,
      “failed to write template to <path>”) and exits non-zero.
- Error cases:
  - `.git/` directory not present → not a git repo.
  - `.specify/` directory not present → Spec Kit not initialized.
  - Filesystem errors when creating directories or writing templates.

## Detailed behavior
- Parse CLI args:
  - `specdrive bootstrap` with no additional args.
- Verify prerequisites:
  - Check for `.git/` in the current working directory.
    - If missing → print error and exit with code `1` (`NOT_GIT_REPO`).
  - Check for `.specify/` in the current working directory.
    - If missing → print error suggesting `specify init` and exit with code `1` (`NO_SPECIFY_DIR`).
- Create/ensure directories:
  - Ensure `.specify/specs/` exists (create if missing).
  - Ensure `.specify/templates/` exists (create if missing).
  - Ensure `docs/`, `docs/features/`, and `docs/templates/` exist (create if missing).
- Create template files from embedded assets **if they do not already exist**:
  - `.specify/templates/feature.spec.md`
    - Contents from `assets/bootstrap/feature.spec.md`.
  - `docs/templates/feature.contract.minimal.yaml`
    - Contents from `assets/bootstrap/feature.contract.minimal.yaml`.
  - `docs/templates/feature.contract.critical.yaml`
    - Contents from `assets/bootstrap/feature.contract.critical.yaml`.
- Overwrite rules:
  - **Never** overwrite an existing file.
  - If a target file already exists, leave it untouched and optionally log that it was skipped.
- Exit behavior:
  - Exit code `0` on success.
  - Exit code `1` on missing prerequisites (git or `.specify/`).
  - Exit code `2` on filesystem write errors.

# Non-Functional Requirements
- Performance:
  - Only creates a small set of directories and template files; performance is not critical.
  - Implementation should be straightforward and avoid unnecessary filesystem calls.
- Portability:
  - Must work on macOS, Linux, and Windows where Rust binaries run.
  - Path handling should rely on `std::path` and not assume Unix-specific separators.
- Security:
  - No network calls.
  - Only reads minimal state (`.git/`, `.specify/` existence) and writes under the project root.
- UX:
  - Errors must clearly indicate:
    - What prerequisite is missing (`.git/`, `.specify/`).
    - Which path failed to be created or written on filesystem errors.
  - Success output should clearly list which paths were created and which were skipped.

# Acceptance Criteria
- [x] AC-1: In a repo with `.git/` and `.specify/` present but no `docs/` or template files,
      `specdrive bootstrap` creates:
      - `.specify/specs/`
      - `.specify/templates/feature.spec.md`
      - `docs/`
      - `docs/features/`
      - `docs/templates/feature.contract.minimal.yaml`
      - `docs/templates/feature.contract.critical.yaml`
      and exits with code `0`.
- [x] AC-2: If `.git/` is missing, `specdrive bootstrap` exits with code `1` and prints an error
      indicating that it must be run in a git repo.
- [x] AC-3: If `.specify/` is missing, `specdrive bootstrap` exits with code `1` and prints an error
      suggesting that the user run `specify init` first.
- [x] AC-4: When any of the target files already exist (e.g. a user-edited `feature.spec.md`),
      `specdrive bootstrap` leaves them unchanged, reports them as skipped, and still exits `0`
      as long as no other errors occur.
- [ ] AC-5: If a filesystem error occurs while creating a directory or writing a template file,
      the command prints an error including the path and exits with code `2`.

# Implementation Notes
- Expected files/modules to touch:
  - `src/cli.rs`
    - Add `"bootstrap"` subcommand entry and wire it to a `bootstrap::run()` or similar function.
  - `src/bootstrap.rs` (new module)
    - Implement the core bootstrap logic:
      - `ensure_git_repo_present()`
      - `ensure_specify_dir_present()`
      - `ensure_dirs_exist(...)`
      - `install_template_if_missing(...)`
  - `src/fsutil.rs`
    - Potential helpers for “write file if missing” and directory creation.
  - Possibly `src/assets.rs` or inline `include_str!` calls to embed:
    - `assets/bootstrap/feature.spec.md`
    - `assets/bootstrap/feature.contract.minimal.yaml`
    - `assets/bootstrap/feature.contract.critical.yaml`
- Any refactors allowed/required:
  - It’s acceptable to move bootstrap-related logic out of `cli.rs` into a dedicated module.
  - Keep bootstrap focused on scaffolding; do **not** introduce AI or git-cleanliness checks here.
- Any dependencies (avoid adding crates unless needed):
  - No external crates. Use only the standard library.
  - Template contents should come from `include_str!` or a similar compile-time embedding approach.

# Open Questions
- Q1: Should bootstrap optionally create a minimal `docs/system-overview.md` stub, or leave it entirely up to the user?
- Q2: Should bootstrap attempt to detect the git repo root if run from a subdirectory, or is “must run from repo root” sufficient for v0.1?
