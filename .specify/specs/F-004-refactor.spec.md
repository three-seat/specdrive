---
id: F-004-refactor-helpers
title: 004 Shared helper refactor
type: feature
system: specdrive
status: draft
area: core
owners:
  - three_seat
created_at: 2026-01-11
contract: docs/features/F-004-refactor-helpers/contract.yaml
---

# Summary

Refactor specdrive’s repo / Spec Kit / feature-path logic into shared utilities so commands can consistently enforce preconditions and build prompts. The goal is **no behaviour change** for existing commands (`bootstrap`, `implement`, `draft`) while improving safety, clarity, and reuse.

Key changes:

- Standardized helpers under `src/utils.rs` (and/or `src/fsutil.rs`).
- Structured error types instead of ad-hoc strings.
- Reuse of these helpers in `bootstrap`, `implement`, and `draft`.

# Context

- Today, git + `.specify` checks and path construction are duplicated across commands.
- Error handling is string-based; mapping from internal failures to contract error codes / exit codes is ad-hoc.
- As features grow (`draft`, later patch-based flows), we need:
  - One place to define “repo is ready”.
  - One place to define “feature paths/templates exist”.
  - One place to list ADRs and optional docs for prompts.

Constraints:

- No new CLI flags or visible behavioural changes for `bootstrap`, `implement`, or `draft`.
- No new external crates beyond what is already allowed (e.g., `serde_yaml` from F-002).
- For 0.1, **do not** introduce a full git library; keep using the existing approach for git checks.

# Behaviour

## What this feature does

- Introduces shared helpers (in `src/utils.rs` and optionally `src/fsutil.rs`) for:
  - Repo / Spec Kit / clean-tree checks.
  - Feature spec/contract path resolution + existence validation.
  - Template path existence checks.
  - Optional docs detection (constitution, system overview, ADRs).
- Updates `bootstrap`, `implement`, and `draft` to use these helpers.
- Introduces structured error types so commands can map failures to:
  - Contract error codes (e.g., `NOT_GIT_REPO`, `NO_SPECIFY_DIR`, `DIRTY_TREE`, `MISSING_SPEC`).
  - Appropriate exit codes.

## What this feature does NOT do

- No new subcommands.
- No changes to existing CLI arguments, flags, or exit codes.
- No new network behaviour.
- No changes to the semantics defined in F-001, F-002, or F-003 contracts (only internal refactor).

# Detailed Behaviour

## Repo / Spec Kit / clean tree helper

- Central helper (e.g., `utils::ensure_repo_and_specify_ready()`) is the **single entry point** for:
  - Ensuring `.git/` exists in the current directory.
  - Ensuring `.specify/` exists.
  - Ensuring the git working tree is clean (allowing untracked files as per existing contract behaviour).
- On failure, it returns a **structured error** with:
  - A machine-friendly code (e.g., `NotGitRepo`, `NoSpecifyDir`, `DirtyTree`).
  - A human-readable message suitable for printing to stderr.
- `bootstrap`, `implement`, and `draft` call this helper and map:
  - `NotGitRepo` → error message + exit code 1.
  - `NoSpecifyDir` → error message + exit code 1.
  - `DirtyTree` → error message + exit code 1.

## Feature paths helper

- Introduce a small struct (name is up to implementation) representing feature paths, e.g.:

  - `spec` → `.specify/specs/<FEATURE_ID>.spec.md`
  - `contract` → `docs/features/<FEATURE_ID>/contract.yaml`

- Helper responsibilities:
  - Given `FEATURE_ID`, construct canonical paths for spec and contract.
  - Provide a method to validate both exist; if not:
    - Return a structured error indicating **which** file is missing.
- `implement` and `draft`:
  - Use this helper rather than hand-building paths.
  - Treat missing spec/contract as “unknown feature”, returning:
    - Clear error including the missing path.
    - Exit code 2.

## Template paths helper

- Helper responsible for the contract templates that `bootstrap` ensures:

  - `docs/templates/feature.contract.minimal.yaml`
  - `docs/templates/feature.contract.critical.yaml`

- Responsibilities:
  - Construct canonical paths.
  - Validate existence when needed (e.g., for `draft` prompts).
  - Return a structured error if any required template is missing.

## Optional docs helpers

- Helpers to discover optional documentation used in prompts:

  - Constitution:
    - `.specify/memory/constitution.md`
    - Helper returns “present + path” or “absent”.
  - System overview:
    - `docs/system-overview.md`
    - Helper returns “present + path” or “absent”.
  - ADRs:
    - Scan `docs/adrs/` for markdown files.
    - Return a deterministic, sorted list of paths (may be empty).

- `implement` and `draft`:
  - Use these helpers to build their prompt file-path lists.
  - Do not inline file contents; they reference paths only.

## Error handling

- All helpers should return structured error types (enums/structs), not bare strings.
- Command entrypoints remain responsible for:
  - Mapping internal errors to:
    - Human readable messages.
    - Exit codes.
    - Contract-level error codes (implied via message text and code mapping).

# Non-Functional Requirements

- **No behaviour change**:
  - All existing acceptance criteria for F-001, F-002, and F-003 remain valid.
  - Any test added by earlier features must still pass.
- **Deterministic behaviour**:
  - Helper functions must produce deterministic results for a given repo state (e.g., ADR listing order).
- **Maintainability**:
  - New helpers must be small, focused, and covered by unit tests.
  - Commands (`bootstrap`, `implement`, `draft`) should stay thin and delegate checks to helpers.

# Acceptance Criteria

- [ ] AC-1: `specdrive bootstrap` behaviour (inputs, outputs, exit codes, error conditions) is unchanged, but now delegates repo/Spec Kit checks to the shared helper.
- [ ] AC-2: `specdrive implement <FEATURE_ID>` behaviour is unchanged, but now:
  - Uses the shared repo/Spec Kit/clean-tree helper.
  - Uses a shared helper to resolve and validate feature spec/contract paths.
  - Uses shared helpers to discover constitution, system overview, and ADR files for prompt construction.
- [ ] AC-3: `specdrive draft <FEATURE_ID>` behaviour is unchanged, but now:
  - Uses the shared repo/Spec Kit/clean-tree helper.
  - Uses the same feature-path and docs helpers as `implement`.
- [ ] AC-4: Structured error types exist for:
  - “Not a git repo” (`.git/` missing).
  - “Spec Kit not initialized” (`.specify/` missing).
  - “Dirty working tree”.
  - “Missing spec”.
  - “Missing contract”.
  - “Missing required template”.
- [ ] AC-5: Unit tests cover each helper’s success and error paths (at least: repo ready, `.git/` missing, `.specify/` missing, dirty tree, missing spec, missing contract, missing templates, no ADRs, some ADRs).
- [ ] AC-6: Integration tests (or existing ones) confirm that exit codes and user-facing error messages for `bootstrap`, `implement`, and `draft` match their respective contracts before and after this refactor.
- [ ] AC-7: No new external crates are introduced by this feature.

# Implementation Notes

- Place generic cross-cutting helpers in `src/utils.rs`:
  - `ensure_repo_and_specify_ready()` and related error types.
- Place filesystem-specific helpers in `src/fsutil.rs` or similar:
  - Feature path resolver.
  - Template path checks.
  - Optional-docs discovery (constitution, system overview, ADR listing).
- Keep helpers small and composable so they can be reused by future features (e.g., F-005 naming conventions, F-006 patch flows).
- Avoid embedding any git library at this stage; continue to rely on the existing approach in the `git` module, but centralize the logic behind the new helpers.

# Open Questions

- Q1: Should the structured error codes be surfaced directly to users (e.g., printed alongside messages) or remain internal to contracts and tests?
- Q2: Do we want a single unified error enum for all helpers, or separate enums per helper group (repo readiness vs feature paths vs templates)?
- Q3: Should we add a small “diagnostic” command in a future feature (e.g., `specdrive doctor`) that reuses these helpers to report readiness status?
