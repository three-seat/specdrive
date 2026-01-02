---
id: F-003-draft
title: 003 Draft command
type: feature
system: specdrive
status: draft
area: cli
owners:
  - three_seat
created_at: 2025-12-29
contract: docs/features/F-003-draft/contract.yaml
---

# Summary

Add a `specdrive draft <FEATURE_ID>` command that prepares and prints an AI-ready prompt for drafting or updating a feature contract. The command must sanity-check basic conditions (git repo, Spec Kit initialization, clean working tree, presence of the feature spec and contract skeleton) and then output a structured prompt that tells the AI tool (e.g., Claude) which files to read and how to produce/update `docs/features/<FEATURE_ID>/contract.yaml`.

As with `implement`, the prompt **does not embed file contents**; it references file paths instead.

# Context

- What problem does this solve?
  - Right now, writing a feature contract is manual and ad-hoc, even though we have:
    - A feature spec in `.specify/specs/<FEATURE_ID>.spec.md`
    - Contract templates in `docs/templates/feature.contract.*.yaml`
  - We want a repeatable way to have AI draft or refine `contract.yaml` using the spec, ADRs, constitution, and system overview.
- Any constraints (OS, tooling, repo layout, compatibility)?
  - Repo layout must follow:
    - `.specify/specs/<FEATURE_ID>.spec.md`
    - `docs/features/<FEATURE_ID>/contract.yaml` (initially created by `specdrive new-feature`)
  - `specdrive draft` is **read-only** in v0.1: it only reads local files and prints to stdout.
  - No network calls from `specdrive` itself.
  - Prompt structure should be deterministic for a given repo state: same inputs → same prompt (modulo header/footer files).
  - A small YAML parsing crate is allowed later for validation, but v0.1 can treat `contract.yaml` as an opaque text file.

# Behavior

## User flow

- Command / entrypoint:
  - `specdrive draft <FEATURE_ID>`
- Inputs:
  - `FEATURE_ID` that:
    - Has a spec at `.specify/specs/<FEATURE_ID>.spec.md`
    - Has a contract skeleton at `docs/features/<FEATURE_ID>/contract.yaml` created via `specdrive new-feature <FEATURE_ID> [--critical]`
- Outputs:
  - A structured AI prompt printed to stdout, something like:
    1. Optional configurable header text (if `docs/ai/draft-header.md` exists).
    2. A built-in header explaining that the goal is to **draft or refine** the contract for `<FEATURE_ID>` from the spec and supporting docs.
    3. A list of relevant file paths the AI should read, for example:
       - `.specify/specs/<FEATURE_ID>.spec.md`
       - `docs/features/<FEATURE_ID>/contract.yaml`
       - `.specify/memory/constitution.md` (if present)
       - `docs/adrs/` (for ADRs, if present)
       - `docs/system-overview.md` (if present)
       - `docs/templates/feature.contract.minimal.yaml` and/or `docs/templates/feature.contract.critical.yaml`
    4. Guidance on how to shape the contract:
       - Use the appropriate template (minimal vs critical).
       - Fill in `requirements`, `behaviour`, `logic`, `filesystem`, `git_safety`, `verification`, etc., based on the spec + ADRs.
       - Do not weaken invariants or relax safety properties.
    5. Optional configurable footer text (if `docs/ai/draft-footer.md` exists).
- Error cases:
  - Not in a git repo (`.git/` missing).
  - Spec Kit not initialized (`.specify/` missing).
  - Working tree not clean (uncommitted changes).
  - Spec file missing.
  - Contract skeleton missing.
  - Template files missing (for minimal / critical contracts, if referenced by the prompt).
  - `FEATURE_ID` malformed (empty string).
  - Files unreadable (permissions, IO errors).

## Detailed behavior

- Parse CLI args and extract `<FEATURE_ID>`.
- Validate basic usage:
  - If `<FEATURE_ID>` is empty or missing → print usage error and exit with code 1.
- Git + bootstrap checks:
  - Call `utils::ensure_repo_and_specify_ready()` (introduced in F-002) to:
    - Verify `.git/` exists.
    - Verify `.specify/` exists.
    - Verify the git working tree is clean (ignoring allowed untracked files).
  - On failure, print a clear error and exit with code 1.
- Resolve the feature paths:
  - `spec_path = .specify/specs/<FEATURE_ID>.spec.md`
  - `contract_path = docs/features/<FEATURE_ID>/contract.yaml`
- Validate feature artifacts:
  - If `spec_path` does not exist → print a clear error including the path, exit with code 2.
  - If `contract_path` does not exist → print a clear error including the path, exit with code 2.
- Determine available supporting docs:
  - Check for `.specify/memory/constitution.md`.
  - Check for `docs/adrs/` (directory exists).
  - Check for `docs/system-overview.md`.
  - Check for:
    - `docs/templates/feature.contract.minimal.yaml`
    - `docs/templates/feature.contract.critical.yaml`
  - Optionally check for:
    - `docs/ai/draft-header.md`
    - `docs/ai/draft-footer.md`
- Construct a prompt with this structure:
  1. Optional header from `docs/ai/draft-header.md` (if present).
  2. Built-in intro line:
     - You are drafting/refining `docs/features/<FEATURE_ID>/contract.yaml` for `<FEATURE_ID>`.
     - Spec + ADRs + constitution + system overview define the behaviour & constraints.
     - Use the appropriate contract template (minimal vs critical).
  3. A section listing files to read, e.g.:

     ```text
     Files you MUST read before drafting the contract:
     - .specify/specs/<FEATURE_ID>.spec.md
     - docs/features/<FEATURE_ID>/contract.yaml (current or skeleton)
     - .specify/memory/constitution.md (if present)
     - docs/adrs/ (scan for relevant ADRs)
     - docs/system-overview.md (if present)
     - docs/templates/feature.contract.minimal.yaml
     - docs/templates/feature.contract.critical.yaml
     ```

  4. Guidance section:
     - How to map spec → `requirements`, `behaviour`, `logic`, `filesystem`, `git_safety`, `verification`, `ai_instructions`.
     - For critical features:
       - Ensure `critical: true` and stronger invariants / git safety.
       - Add appropriate review expectations.
  5. Guardrails:
     - Do **not** weaken invariants or lower safety.
     - Do **not** change feature IDs or basic layout.
     - Keep the contract structured and consistent with existing examples.
  6. Optional footer from `docs/ai/draft-footer.md` (if present).
- Print the prompt to stdout only; do not write it to disk.
- Exit code:
  - `0` on success.
  - `1` on usage / precondition failures (not a git repo, `.specify/` missing, dirty tree, etc.).
  - `2` on filesystem errors (missing/unreadable spec/contract or required templates).

# Non-Functional Requirements

- Performance:
  - Reads a handful of small files and prints a text prompt; performance is a non-issue.
- Portability:
  - Must work on macOS, Linux, and Windows where Rust binaries run.
- Security:
  - No network operations.
  - Only reads files in the current repo; no traversal outside project root.
- UX:
  - Errors should be explicit and actionable (mention the failing path and reason).
  - Prompt must be copy-paste friendly for AI tools.
  - Prompt structure deterministic for a given repo state.

# Acceptance Criteria

- [ ] AC-1: With git initialized, clean tree, and existing spec/contract skeleton for `F-001-bootstrap`, `specdrive draft F-001-bootstrap` prints a single, well-structured prompt to stdout and exits with code 0.
- [ ] AC-2: If `.git/` is missing, `specdrive draft <FEATURE_ID>` prints a clear error and exits with code 1.
- [ ] AC-3: If `.specify/` is missing, `specdrive draft <FEATURE_ID>` prints a clear error suggesting `specify init` and exits with code 1.
- [ ] AC-4: If the working tree is dirty, `specdrive draft <FEATURE_ID>` prints an error and exits with code 1 without printing the prompt.
- [ ] AC-5: If the spec file is missing, `specdrive draft <FEATURE_ID>` prints a clear error including the missing spec path and exits with code 2.
- [ ] AC-6: If the contract skeleton file is missing, `specdrive draft <FEATURE_ID>` prints a clear error including the missing contract path and exits with code 2.
- [ ] AC-7: If `docs/templates/feature.contract.minimal.yaml` or `feature.contract.critical.yaml` are missing, the command prints an error and exits with a non-zero status.
- [ ] AC-8: If `docs/ai/draft-header.md` and/or `docs/ai/draft-footer.md` exist, their contents appear in the prompt in the correct positions.
- [ ] AC-9: The printed prompt lists relevant file paths and instructs the AI to read them; it does **not** inline spec/contract contents.
- [ ] AC-10: The command does not create, modify, or delete any files; it is read-only.

# Implementation Notes

- Expected files/modules to touch:
  - `src/cli.rs`:
    - Add `"draft"` subcommand parsing.
  - New module `src/draft.rs` (or similar):
    - Implement `draft_feature(feature_id: &str)` or similar.
  - `src/utils.rs`:
    - Reuse `ensure_repo_and_specify_ready()` from F-002 (git + `.specify` + clean tree).
  - `src/fsutil.rs`:
    - Optional helpers for checking existence & reading optional files (header/footer).
- Any refactors allowed/required:
  - Share as much defensive setup logic as possible with `implement` (F-002).
  - Keep prompt construction contained and testable (e.g., pure function returning `String`).
- Dependencies:
  - No new external crates required for v0.1.
  - Future work may allow `serde_yaml` for validating `contract.yaml`, but that’s not required for this first draft command.

# Open Questions

- Q1: Should `draft` treat a missing `contract.yaml` as an error (current assumption) or generate a brand new file from the template in a future version?
- Q2: Should we add a `--template=minimal|critical` flag for `draft` to explicitly tell the AI which contract template to follow?
- Q3: Do we eventually want `draft` to have a “apply changes” mode that writes AI-generated contract updates back to disk (post-0.1)?
