---
id: F-001-bootstrap
title: 001 Bootstrap
type: feature
system: specdrive
status: draft
area: cli
owners:
  - three_seat
created_at: 2025-12-29
contract: docs/features/F-001-bootstrap/contract.yaml
---

# Summary

Add a `specdrive bootstrap` command that prepares the current repo for specdrive/spec-kit usage.  
It should create the expected `.specify/` and `docs/` structure and template files **if they are missing**, without overwriting anything that already exists. The command is idempotent and safe to run multiple times.

# Context

- **Problem**: Right now, setting up a repo for specdrive requires manually creating `.specify/templates/feature.spec.md`, `docs/features/`, and contract templates. That’s boring, error-prone, and hard to repeat.
- **Goal**: One command that:
  - Verifies we’re in a git repo / project root.
  - Creates the minimum directory and template structure for specdrive.
  - Does not destroy or overwrite existing files.
- **Constraints**:
  - Local CLI only (no network calls).
  - No new crates; use `std` only for now.
  - Must work from the **repo root** of the project we’re bootstrapping.

# Behaviour

## User flow

- **Command / entrypoint**:  
  `specdrive bootstrap`

- **Inputs**:  
  - No required args.  
  - (Future) Optional `--force` flag (not implemented in this feature).

- **Outputs**:
  - Creates directories if missing:
    - `.specify/templates/`
    - `.specify/specs/` (directory only; no specs created here)
    - `docs/features/`
    - `docs/templates/`
    - `docs/adrs/`
    - `docs/milestones/`
  - Creates template files if missing:
    - `.specify/templates/feature.spec.md`
    - `docs/templates/feature.contract.minimal.yaml`
    - `docs/templates/feature.contract.critical.yaml`
  - Prints a short summary of what was created vs already present.

- **Error cases**:
  - Not run from a git repo root (or `.git` not found upwards): fail with a clear error.
  - Filesystem write errors: bubble up with helpful messages.
  - If a target **file** exists, it must **not** be overwritten.

## Detailed behaviour

1. Check that the current directory is inside a git repo; ideally at the repo root (look for `.git` directory in `.`).
2. Ensure directories exist:
   - `.specify/`
   - `.specify/templates/`
   - `.specify/specs/`
   - `docs/`
   - `docs/features/`
   - `docs/templates/`
   - `docs/adrs/`
   - `docs/milestones/`
3. For each template file:
   - `.specify/templates/feature.spec.md`
   - `docs/templates/feature.contract.minimal.yaml`
   - `docs/templates/feature.contract.critical.yaml`
   - If the file **does not exist**, copy from specdrive’s own embedded/template version into the target path.
   - If the file **does exist**, leave it alone and report “exists (skipped)”.
4. Print a concise summary:
   - “Created: …”
   - “Existing (left untouched): …”
5. Exit with:
   - Code `0` on success.
   - Non-zero on failure (git not found, IO errors, etc.).

# Non-Functional Requirements

- **Performance**:  
  - Single-run, local filesystem operations only; should complete in well under a second for typical projects.

- **Portability**:  
  - Must work on macOS, Linux, and Windows where git and Rust binaries are available.
  - Avoid hard-coding path separators; use `std::path`.

- **Security**:  
  - No network calls.
  - No reading/writing outside the repo root tree.
  - No secrets or tokens involved.

- **UX**:  
  - Output should be brief but clear:
    - what was created,
    - what already existed,
    - any warnings.

# Acceptance Criteria

- [ ] **AC-1**: Running `specdrive bootstrap` in a fresh Cargo project (with git initialized) creates:
      - `.specify/templates/feature.spec.md`
      - `docs/templates/feature.contract.minimal.yaml`
      - `docs/templates/feature.contract.critical.yaml`
      - plus required directories (`.specify/specs/`, `docs/features/`, `docs/adrs/`, `docs/milestones/`).

- [ ] **AC-2**: Running `specdrive bootstrap` a second time is **idempotent**:
      - No files are overwritten.
      - Output clearly indicates that templates already exist and were skipped.
      - Exit code remains `0`.

- [ ] **AC-3**: If run in a directory that is **not** a git repo (no `.git` in `.`), the command:
      - Fails with a non-zero exit code.
      - Prints a clear message such as: “specdrive bootstrap must be run from a git repo root”.

# Implementation Notes

- **Expected files/modules to touch**:
  - `src/cli.rs` – add a `bootstrap` subcommand.
  - `src/bootstrap.rs` (or similar new module) – implement the behaviour described here.
  - Possibly `src/fsutil.rs` – reuse helpers for creating directories/copying templates.

- **Behavioural constraints**:
  - Use the existing template files that ship with specdrive as the source for copies.
  - Never overwrite existing templates; treat them as user-owned once created.

- **Dependencies**:
  - Use only `std` for filesystem and process operations.
  - No external crates added in this feature.

# Open Questions

- Q1: Should `bootstrap` also create a **starter** feature spec/contract (e.g. `F-000-example`) or remain purely structural?
- Q2: Should we enforce “repo root” strictly (require `.git` in `.`), or is “anywhere inside a git work tree” acceptable for now?
