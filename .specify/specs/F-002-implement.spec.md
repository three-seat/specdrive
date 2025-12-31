---
id: F-002-implement
title: 002 Implement command
type: feature
system: specdrive
status: draft
area: cli
owners:
  - three_seat
created_at: 2025-12-29
contract: docs/features/F-002-implement/contract.yaml
---

# Summary
Add a `specdrive implement <FEATURE_ID>` command that prepares and prints an AI-ready implementation prompt for a given feature. The command should sanity-check basic conditions (git repo, Spec Kit initialization, clean working tree, presence and validity of spec/contract and key supporting docs), and then output a structured prompt that lists the relevant file paths and instructions for the AI tool (e.g., Claude) to read those files and implement or update the code.

The prompt **does not embed the contents** of the spec or contract; it references file paths instead.

# Context
- What problem does this solve?
  - Right now, specdrive can bootstrap the project and scaffold a feature (spec + contract), but there is no standardized way to drive AI implementation from those artifacts.
  - Every time you want AI to implement a feature, you have to manually assemble context and instructions.
- Any constraints (OS, tooling, repo layout, compatibility)?
  - Repo layout must follow:
    - `.specify/specs/<FEATURE_ID>.spec.md`
    - `docs/features/<FEATURE_ID>/contract.yaml`
  - `specdrive implement` is **read-only** in v0.1: it only reads local files and prints to stdout.
  - No network calls from `specdrive` itself.
  - Prompt structure should be deterministic for a given repo state: same inputs → same prompt (modulo header/footer files and which optional docs exist).
  - A small YAML parsing crate is allowed to read contract metadata; otherwise prefer the Rust standard library.

# Behavior

## User flow
- Command / entrypoint:
  - `specdrive implement <FEATURE_ID>`
- Inputs:
  - `FEATURE_ID` that must have:
    - `.specify/specs/<FEATURE_ID>.spec.md`
    - `docs/features/<FEATURE_ID>/contract.yaml`
- Outputs:
  - A structured AI prompt printed to stdout, something like:
    1. Optional configurable header text (if `docs/ai/implement-header.md` exists).
    2. A short built-in header explaining that the feature spec + contract (and supporting docs) are the source of truth and must be read from disk.
    3. A list of relevant file paths the AI should read, for example:
       - `.specify/specs/<FEATURE_ID>.spec.md`
       - `docs/features/<FEATURE_ID>/contract.yaml`
       - `.specify/memory/constitution.md` (if present)
       - `docs/system-overview.md` (if present)
       - Individual ADR files under `docs/adrs/` (if present)
    4. Built-in implementation guardrails (small functions, respect invariants, no new deps beyond what’s allowed, etc.).
    5. Optional configurable footer text (if `docs/ai/implement-footer.md` exists).
- Error cases:
  - Not in a git repo (`.git/` missing).
  - Spec Kit not initialized (`.specify/` missing).
  - Working tree not clean (uncommitted changes) when required.
  - Spec file missing.
  - Contract file missing.
  - Contract YAML invalid.
  - Contract marked `critical: true` but not reviewed (missing `reviews.status.reviewed_by` or `reviewed_at`).
  - `FEATURE_ID` malformed (empty string).
  - Supporting docs present but unreadable (constitution, system overview, ADR files).
  - Files unreadable (permissions, IO errors).

## Detailed behavior
- Parse CLI args and extract `<FEATURE_ID>`.
- Validate basic usage:
  - If `<FEATURE_ID>` is empty or missing → print usage error and exit with code 1.
- Git + bootstrap checks:
  - Verify `.git/` exists in the current directory; if not, print a clear error and exit code 1.
  - Verify `.specify/` exists; if not, print a clear error suggesting `specify init` and exit code 1.
  - Run a git working-tree check:
    - If the tree is not clean (ignoring allowed untracked files), print a clear error and exit with code 1.
- Resolve paths:
  - `spec_path = .specify/specs/<FEATURE_ID>.spec.md`
  - `contract_path = docs/features/<FEATURE_ID>/contract.yaml`
  - `constitution_path = .specify/memory/constitution.md` (optional)
  - `system_overview_path = docs/system-overview.md` (optional)
  - `adrs_dir = docs/adrs/` (optional)
- Validate spec + contract presence:
  - If either file is missing → print a clear error including the missing path, exit with code 2.
- Read the contract file as UTF-8 text and parse the YAML:
  - Validate that it is well-formed YAML; if parsing fails, print a clear error including the path and exit with code 2.
  - Inspect `metadata.critical`:
    - If `critical: true`, require `reviews.status.reviewed_by` and `reviews.status.reviewed_at` to be non-empty.
    - If missing/empty, print a clear error that the critical feature is not yet reviewed and exit code 1.
- Read supporting docs (defensive but optional):
  - If `constitution_path` exists:
    - Read as UTF-8 text; if read fails, print an error including the path and exit with code 2.
  - If `system_overview_path` exists:
    - Read as UTF-8 text; if read fails, print an error including the path and exit with code 2.
  - If `adrs_dir` exists:
    - Enumerate ADR files (e.g., `*.md`); for each ADR file:
      - Attempt to read as UTF-8 text; if any read fails, print an error including the path and exit with code 2.
    - Collect ADR file paths for inclusion in the prompt (not their contents).
- (Optional) Read prompt customization:
  - If `docs/ai/implement-header.md` exists, read it as a header segment.
  - If `docs/ai/implement-footer.md` exists, read it as a footer segment.
- Construct a prompt with this structure:
  1. Optional header from `docs/ai/implement-header.md` (if present).
  2. Built-in intro line: spec + contract (and supporting docs) are the source of truth; the AI must read the referenced files from disk and not modify spec/contract unless explicitly asked.
  3. A section listing the relevant file paths to open, e.g.:

     ```text
     Files you MUST read before coding:
     - .specify/specs/<FEATURE_ID>.spec.md
     - docs/features/<FEATURE_ID>/contract.yaml
     - .specify/memory/constitution.md (if present)
     - docs/system-overview.md (if present)
     - docs/adrs/<ADR_XXX>.md (if present; list each ADR file)
     ```

  4. Built-in guardrails text (no new deps beyond those allowed, read-only behavior for this command, prefer small focused functions, respect invariants and filesystem/git rules from the contract).
  5. Optional footer from `docs/ai/implement-footer.md` (if present).
- Print the prompt to stdout only; do not write it to disk.
- Exit code:
  - `0` on success.
  - `1` on usage / precondition failures (not a git repo, `.specify` missing, dirty tree, unreviewed critical contract, etc.).
  - `2` on filesystem/read/parse errors (missing or unreadable spec/contract, invalid YAML, unreadable supporting docs).

# Non-Functional Requirements
- Performance:
  - Reading a few small files and constructing a text prompt; performance is a non-issue.
- Portability:
  - Must work on macOS, Linux, and Windows where Rust binaries run.
- Security:
  - No network operations.
  - Only reads files in the current repo; no traversal outside project root.
- UX:
  - Errors should be explicit: include the path and reason.
  - Printed prompt must be copy-paste friendly for AI tools.
  - Prompt structure should be deterministic based on repo state (spec, contract, header/footer files, presence of supporting docs).

# Acceptance Criteria
- [ ] AC-1: Running `specdrive implement F-001-bootstrap` when git is initialized, the working tree is clean, and all required files exist prints a single, well-structured prompt to stdout and exits with code 0.
- [ ] AC-2: If `.git/` is missing, `specdrive implement F-001-bootstrap` prints a clear error and exits with code 1.
- [ ] AC-3: If `.specify/` is missing, `specdrive implement F-001-bootstrap` prints a clear error suggesting `specify init` and exits with code 1.
- [ ] AC-4: If there are uncommitted changes in the working tree, `specdrive implement <FEATURE_ID>` prints an error and exits with code 1 without printing the prompt.
- [ ] AC-5: If the spec file is missing, `specdrive implement F-001-bootstrap` prints a clear error including the missing spec path and exits with code 2.
- [ ] AC-6: If the contract file is missing, `specdrive implement F-001-bootstrap` prints a clear error including the missing contract path and exits with code 2.
- [ ] AC-7: If the contract YAML is invalid, the command prints a clear error including the path and exits with code 2.
- [ ] AC-8: If the contract for a `critical: true` feature has empty `reviews.status.reviewed_by` or `reviews.status.reviewed_at`, the command prints a clear error and exits with code 1.
- [ ] AC-9: If `docs/ai/implement-header.md` and/or `docs/ai/implement-footer.md` exist, their contents appear in the prompt in the correct positions.
- [ ] AC-10: The printed prompt lists the relevant file paths and instructs the AI to read them; it does **not** inline spec/contract contents.
- [ ] AC-11: If supporting docs (constitution, system overview, ADRs) exist but cannot be read, the command prints a clear error including the path and exits with code 2.
- [ ] AC-12: The command does not create, modify, or delete any files; it is read-only.
- [ ] AC-13: If `FEATURE_ID` does not correspond to an existing feature (spec and/or contract missing), `specdrive implement <FEATURE_ID>` prints an error indicating the missing file(s) and exits with code 2.

# Implementation Notes
- Expected files/modules to touch:
  - `src/cli.rs`:
    - Add `"implement"` subcommand parsing.
  - New module `src/implement.rs` (or similar):
    - Implement `implement_feature(feature_id: &str)` or similar.
  - `src/fsutil.rs`:
    - Optional helpers for reading optional files (`docs/ai/implement-header.md`, `docs/ai/implement-footer.md`, constitution, ADRs, system overview).
  - `src/git.rs`:
    - Use existing git helper for “is repo?” and “ensure clean tree”.
- Any refactors allowed/required:
  - It’s acceptable to split feature-related logic into separate modules (`feature.rs`, `implement.rs`) if it stays small and clear.
- Any dependencies (avoid adding crates unless needed):
  - Allow a single YAML parsing crate (e.g. `serde_yaml`) strictly for reading and validating `contract.yaml` metadata.
  - No other new external crates in this feature.
- Introduce a helper, e.g. `ensure_repo_and_specify_ready()` in `utils.rs`:
  - Verifies `.git/` exists.
  - Verifies `.specify/` exists.
  - Performs git clean-tree check according to the contract.
- `specdrive implement <FEATURE_ID>` must use this helper before reading spec/contract files or supporting docs.

# Open Questions
- Q1: Should we allow a config file (e.g., `docs/ai/implement-config.yaml`) for more advanced prompt customization in a later version?
- Q2: Should failure on unreviewed critical features be configurable (e.g., `--override-critical-check`), or is it hard-required for now? (Assume **hard-required** in v0.1.)
