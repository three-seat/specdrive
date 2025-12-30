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
Add a `specdrive implement <FEATURE_ID>` command that prepares and prints an AI-ready implementation prompt for a given feature. The command should read the feature’s spec and contract, sanity-check basic conditions, and output a structured prompt that can be pasted into an AI tool (e.g., Claude) to implement or update the code.

# Context
- What problem does this solve?
  - Right now, specdrive can bootstrap the project and scaffold a feature (spec + contract), but there is no standardized way to drive AI implementation from those artifacts.
  - Every time you want AI to implement a feature, you have to manually assemble context and instructions.
- Any constraints (OS, tooling, repo layout, compatibility)?
  - Repo layout must follow:
    - `.specify/specs/<FEATURE_ID>.spec.md`
    - `docs/features/<FEATURE_ID>/contract.yaml`
  - No network calls from `specdrive` itself; it only reads local files and prints to stdout.
  - No external crates (for now) beyond the Rust standard library.
  - This is v0.1: we are **not** invoking AI via API yet, just generating a prompt.

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
    - A short header
    - The spec contents
    - The contract contents
    - Implementation instructions and guardrails
- Error cases:
  - Spec file missing
  - Contract file missing
  - `FEATURE_ID` malformed (empty string)
  - Files unreadable (permissions, IO errors)

## Detailed behavior
- Parse CLI args and extract `<FEATURE_ID>`.
- Resolve:
  - `spec_path = .specify/specs/<FEATURE_ID>.spec.md`
  - `contract_path = docs/features/<FEATURE_ID>/contract.yaml`
- Validate:
  - If either file is missing → print a clear error and exit with non-zero status.
- Read both files as UTF-8 text.
- Construct a prompt with this structure (high level):
  1. Short intro line explaining that spec + contract are the source of truth.
  2. Spec content, clearly delimited (e.g., `--- SPEC START/END ---`).
  3. Contract YAML, clearly delimited (e.g., `--- CONTRACT START/END ---`).
  4. Implementation guardrails:
     - Don’t modify spec/contract unless asked.
     - Prefer small, focused functions.
     - Respect invariants and filesystem/git rules in the contract.
- Print the prompt to stdout only; do not write it to disk.
- Exit code:
  - `0` on success.
  - Non-zero on validation or IO errors.

# Non-Functional Requirements
- Performance:
  - Reading and printing two files; performance is a non-issue, but implementation should be straightforward and not do unnecessary work.
- Portability:
  - Must work on macOS, Linux, and Windows where Rust binaries run.
- Security:
  - No network operations.
  - Only reads files in the current repo; no traversal outside project root.
- UX:
  - Errors should be explicit: include the missing path and reason.
  - Printed prompt must be copy-paste friendly for AI tools.

# Acceptance Criteria
- [ ] AC-1: Running `specdrive implement F-001-bootstrap` when both spec and contract exist prints a single, well-structured prompt to stdout and exits with code 0.
- [ ] AC-2: If the spec file is missing, `specdrive implement F-001-bootstrap` prints a clear error including the missing spec path and exits with a non-zero status.
- [ ] AC-3: If the contract file is missing, `specdrive implement F-001-bootstrap` prints a clear error including the missing contract path and exits with a non-zero status.
- [ ] AC-4: The output prompt clearly delimits spec and contract sections so an AI can easily parse/use them.
- [ ] AC-5: The command does not create, modify, or delete any files; it is read-only.

# Implementation Notes
- Expected files/modules to touch:
  - `src/cli.rs`:
    - Add `"implement"` subcommand parsing.
  - `src/feature.rs` (or a new module, e.g. `src/implement.rs`):
    - Implement `implement_feature(feature_id: &str)` or similar.
  - Optionally a small helper in `fsutil` for “read spec + contract as strings”.
- Any refactors allowed/required:
  - It’s acceptable to split feature-related logic into separate modules (`feature.rs`, `implement.rs`) if it stays small and clear.
- Any dependencies (avoid adding crates unless needed):
  - No new external crates. Use `std::fs` and `std::path` for reading files.

# Open Questions
- Q1: Should `implement` enforce a clean git working tree for future “auto-apply AI patch” workflows, or remain purely read-only? (For now, assume **no git check** since it only prints to stdout.)
- Q2: Should we support an optional flag like `--short` or `--no-spec` / `--no-contract` to control how much context is printed for the AI? (Out of scope for v0.1 unless it’s obviously needed.)
