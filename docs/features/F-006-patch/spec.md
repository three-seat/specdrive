---
id: F-006
title: Emit implementation patch artifacts
type: feature
system: specdrive
status: draft
area: git
owners:
  - three_seat
created_at: 2026-05-26
contract: docs/features/F-006/contract.yaml
adrs:
  - ADR-001
---

# Summary
Build support for emitting a reviewable patch artifact from the current Git working tree after implementation work has been completed externally. This allows SpecDrive to capture human- or AI-produced implementation changes as a first-class artifact while preserving the existing v0.1 prompt-only `implement` model.

# Context
- This feature solves the gap between implementation prompt generation and reviewable implementation artifacts.
- `specdrive implement` currently generates implementation prompts only and should remain unchanged.
- Patch emission is now modeled as a first-class patch workflow instead of a flag on `implement`.
- Implementation work may happen manually or through external AI tools such as Copilot Chat or Claude.
- SpecDrive should inspect the resulting Git diff and write it to a feature-local patch artifact path.
- Patch generation must remain local-only and additive.
- Patch generation must not:
  - apply patches,
  - commit changes,
  - mutate git history,
  - invoke AI APIs,
  - or automatically modify implementation files.
- Patch generation captures:
  - unstaged tracked changes,
  - staged tracked changes,
  - staged newly created files.
- Untracked files are not included automatically.
- Newly created files must be explicitly staged before patch emission.
- `patch emit` should warn if untracked files exist.
- Future support for `--include-untracked <path>` is out of scope for F-006.
- The generated patch artifact must not include itself if the patch directory is inside the repository.
- The initial implementation assumes Git is available on the host system.

# Behavior

## User flow

### Existing behavior
- Command / entrypoint:
  ```bash
  specdrive implement F-006
  ```
- Inputs:
  - Feature ID
  - Existing spec + contract
- Outputs:
  - AI-ready implementation prompt
- Error cases:
  - Existing prompt-generation failures remain unchanged.
- Required preservation:
  - F-006 must not change normal `implement` behavior.

### New behavior
- Command / entrypoint:
  ```bash
  specdrive patch emit F-006
  ```
- Inputs:
  - Feature ID
  - Current Git working tree diff
  - Feature-local patch directory, defaulting to `docs/features/F-006/patches/`
- Outputs:
  - Patch file at `docs/features/F-006/patches/F-006.patch`
  - Console summary showing:
    - patch path,
    - files changed,
    - insertions,
    - deletions
  - Warning if untracked files exist
- Error cases:
  - Current directory is not inside a Git repository
  - `.specify/` is missing
  - Spec or contract files are missing
  - Git diff is empty
  - Patch directory cannot be created or written
  - Generated patch fails `git apply --check`

## Detailed behavior

### Existing `implement`
- Preserve existing `implement` behavior and clean-tree preflight logic.
- Continue using `utils::ensure_repo_and_specify_ready()` for prompt generation.
- Do not add `--emit-patch` to `implement` in F-006.

### `patch emit`
- Add a new `patch` subcommand with an `emit` action:
  ```bash
  specdrive patch emit <FEATURE_ID>
  ```
- Resolve and validate:
  - `.specify/specs/<FEATURE_ID>.spec.md`
  - `docs/features/<FEATURE_ID>/contract.yaml`
- Verify:
  - current directory is inside a Git repository,
  - `.specify/` exists.
- Do NOT require a clean working tree.
- Require a non-empty working tree diff.
- Detect untracked files and print a warning if any exist.
- Run:
  ```bash
  git diff --binary HEAD
  ```
  to capture:
  - unstaged tracked changes,
  - staged tracked changes,
  - staged newly created files.
- Ensure the feature-local patch directory exists:
  ```text
  docs/features/<FEATURE_ID>/patches/
  ```
- Ensure the generated patch artifact is excluded from the emitted diff.
- Write the diff to:
  ```text
  docs/features/<FEATURE_ID>/patches/<FEATURE_ID>.patch
  ```
- Validate the generated patch with:
  ```bash
  git apply --check <patch_file>
  ```
- Print a concise implementation summary:
  - patch path
  - changed file count
  - insertion count
  - deletion count

# Non-Functional Requirements

- Performance:
  - Patch generation should be fast for normal feature-sized diffs.
  - Avoid expensive repository-wide analysis.

- Portability:
  - Must work on common Unix-like environments with Git installed.
  - Avoid OS-specific assumptions where practical.

- Security:
  - Must not execute arbitrary user-provided shell strings.
  - Git commands should be invoked with structured process arguments.
  - Must not modify git history or apply patches automatically.
  - Must not mutate files outside the configured patch artifact output path.

- UX:
  - Errors should be direct and actionable.
  - Successful output should be short enough to read comfortably over SSH or on a phone.
  - Warnings about untracked files should clearly explain how to include newly created files.

# Acceptance Criteria

- [ ] AC-1: Running `specdrive patch emit F-006` with a non-empty Git diff writes `docs/features/F-006/patches/F-006.patch`.
- [ ] AC-2: The generated patch passes `git apply --check docs/features/F-006/patches/F-006.patch`.
- [ ] AC-3: Running `patch emit` outside a Git repository fails with a clear error.
- [ ] AC-4: Running `patch emit` with an empty diff fails with a clear error.
- [ ] AC-5: Successful patch generation prints patch path, changed file count, insertions, and deletions.
- [ ] AC-6: Existing `specdrive implement F-006` prompt-generation behavior remains unchanged.
- [ ] AC-7: `patch emit` does not require a clean working tree; it requires a non-empty diff instead.
- [ ] AC-8: The generated patch does not include the patch artifact file itself.
- [ ] AC-9: Staged newly created files are included in emitted patches.
- [ ] AC-10: Untracked files trigger a warning and are not included automatically.
- [ ] AC-11: If `docs/features/F-006/patches/` does not exist, SpecDrive creates it before writing the patch.

# Implementation Notes

- Expected files/modules to touch:
  - `cli.rs`
  - new or existing patch command module, such as `patch.rs`
  - `utils.rs`
  - Git helper utilities if present
  - Config/path resolution module

- Any refactors allowed/required:
  - Small helper functions for git diff collection and patch summary generation are allowed.
  - Avoid broad command restructuring.
  - Existing prompt-generation behavior should remain isolated from patch-emission behavior.

- Any dependencies:
  - Avoid adding crates unless required.
  - Prefer `std::process::Command` for Git invocation.

# Open Questions

- None.
