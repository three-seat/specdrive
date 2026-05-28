---
id: F-007
title: Feature-local artifact architecture
type: feature
system: specdrive
status: draft
area: spec
owners:
  - three_seat
created_at: 2026-05-27
contract: docs/features/F-007/contract.yaml
adrs:
  - ADR-002
---

# Summary
Move SpecDrive to a feature-local artifact architecture where each feature owns its spec, contract, and patch artifacts under `docs/features/<FEATURE_ID>/`. This implements ADR-002, removes the hard dependency on Spec Kit, eliminates the split between `.specify/specs/` and `docs/features/`, and makes SpecDrive’s artifact model cleaner, more auditable, and easier to extend.

# Context
- ADR-002: Feature-local artifact architecture is the governing architectural decision for this feature.
- Specs currently live in `.specify/specs/<FEATURE_ID>.spec.md`.
- Contracts and patches live under `docs/features/<FEATURE_ID>/`.
- This split creates unnecessary cognitive overhead and weakens artifact locality.
- Spec Kit should no longer be required for SpecDrive to operate.
- Feature specs should become canonical at `docs/features/<FEATURE_ID>/spec.md`.
- Existing specs must be moved from `.specify/specs/` into their corresponding feature directories.
- `feature.spec.md` should move from `.specify/templates/` to `docs/templates/`.
- `constitution.md` should move from `.specify/memory/` to `docs/constitution.md`.
- `.claude/` should be removed from the repository.
- This is an intentional breaking change for `v0.3.0`.
- F-007 is a repository layout migration, not a runtime migration system.
- F-007 supersedes the parts of ADR-001 that require Spec Kit initialization and `.specify/` as part of normal SpecDrive operation.

# Behavior

## User flow
- Command / entrypoint:
  - Existing commands affected:
    - `specdrive bootstrap`
    - `specdrive new-feature <FEATURE_ID>`
    - `specdrive draft <FEATURE_ID>`
    - `specdrive implement <FEATURE_ID>`
    - `specdrive patch emit <FEATURE_ID>`
- Inputs:
  - Existing repo with current SpecDrive layout
  - Existing `.specify/specs/*.spec.md`
  - Existing `docs/features/<FEATURE_ID>/contract.yaml`
  - Existing templates and constitution files
  - ADR-002 as the architectural authority for the new layout
- Outputs:
  - Canonical feature specs at `docs/features/<FEATURE_ID>/spec.md`
  - Feature-local artifact directories under `docs/features/<FEATURE_ID>/`
  - Spec template at `docs/templates/feature.spec.md`
  - Constitution relocated to `docs/constitution.md`
  - Updated command path resolution
  - Removed `.claude/`
- Error cases:
  - Feature ID cannot be mapped to an existing feature directory
  - Target spec path already exists during manual migration
  - Required docs/templates directory cannot be created
  - Required feature artifact cannot be read or written

## Detailed behavior
- Treat ADR-002 as the source of truth for the feature-local artifact architecture.
- Establish `docs/features/<FEATURE_ID>/spec.md` as the canonical feature spec path.
- Update all SpecDrive path resolution to use:
  - `docs/features/<FEATURE_ID>/spec.md`
  - `docs/features/<FEATURE_ID>/contract.yaml`
  - `docs/features/<FEATURE_ID>/patches/`
- Repository files and directories are reorganized during F-007 implementation to match the new canonical layout.
- Ensure each moved spec lands in the matching feature directory.
- Move the feature spec template from:
  - `.specify/templates/feature.spec.md`
  to:
  - `docs/templates/feature.spec.md`
- Move the constitution from:
  - `.specify/memory/constitution.md`
  to:
  - `docs/constitution.md`
- Remove `.specify/` as a required runtime/preflight dependency.
- Remove `.claude/` from the repository.
- Update `bootstrap` so it initializes only SpecDrive-owned directories and templates.
- Update `new-feature` so it creates:
  - `docs/features/<FEATURE_ID>/spec.md`
  - `docs/features/<FEATURE_ID>/contract.yaml`
  - `docs/features/<FEATURE_ID>/patches/`
- Update `implement`, `draft`, and `patch emit` so they read the new feature-local spec path.
- Update docs, system overview, ADR references, README, and tests to reflect the new layout.
- Do not implement runtime migration tooling.
- Do not implement `.specify/specs/` compatibility or fallback path resolution.

# Non-Functional Requirements
- Performance:
  - Path resolution and migration should remain fast and filesystem-local.
- Portability:
  - Must work on common Unix-like environments.
  - Avoid OS-specific path assumptions where practical.
- Security:
  - No network calls.
  - No AI API calls.
  - No deletion of user source code.
  - No modification of git history.
  - No secrets written to disk.
- UX:
  - Errors should clearly identify the missing or conflicting path.
  - New layout should be easy to understand from a feature directory alone.
  - Breaking change should be documented clearly in changelog and README.

# Acceptance Criteria
- [ ] AC-1: Canonical feature specs live at `docs/features/<FEATURE_ID>/spec.md`.
- [ ] AC-2: Existing specs are moved from `.specify/specs/` into the correct `docs/features/<FEATURE_ID>/` directories.
- [ ] AC-3: `specdrive new-feature <FEATURE_ID>` creates the feature-local spec and contract under `docs/features/<FEATURE_ID>/`.
- [ ] AC-4: `specdrive implement <FEATURE_ID>` reads `docs/features/<FEATURE_ID>/spec.md`.
- [ ] AC-5: `specdrive draft <FEATURE_ID>` reads/writes feature-local artifacts.
- [ ] AC-6: `specdrive patch emit <FEATURE_ID>` continues to write patches under `docs/features/<FEATURE_ID>/patches/`.
- [ ] AC-7: SpecDrive no longer requires `.specify/` or Spec Kit initialization for normal operation.
- [ ] AC-8: `feature.spec.md` template lives under `docs/templates/`.
- [ ] AC-9: Constitution is moved from `.specify/memory/constitution.md` to `docs/constitution.md`.
- [ ] AC-10: `.claude/` is removed from the repository.
- [ ] AC-11: README, system overview, ADRs, and tests are updated to describe the new layout.
- [ ] AC-12: This breaking layout change is documented for release `v0.3.0`.
- [ ] AC-13: `specdrive new-feature <FEATURE_ID>` creates `docs/features/<FEATURE_ID>/patches/`.
- [ ] AC-14: No runtime migration command or compatibility layer is added.
- [ ] AC-15: F-007 implementation aligns with ADR-002 and updates any ADR-001 references that were superseded.

# Implementation Notes
- Expected files/modules to touch:
  - `src/fsutil.rs`
  - `src/utils.rs`
  - `src/features.rs`
  - `src/implement.rs`
  - `src/draft.rs`
  - `src/patch.rs`
  - `src/bootstrap.rs`
  - `src/config.rs`
  - tests covering path resolution and command behavior
  - README
  - system overview
  - constitution path references
  - ADRs
- Any refactors allowed/required:
  - Refactor feature path construction around `docs/features/<FEATURE_ID>/`.
  - Replace Spec Kit readiness checks with SpecDrive-owned repository readiness checks.
  - Keep migration simple and explicit.
  - Do not add runtime migration tooling.
  - Do not add compatibility/fallback resolution for `.specify/specs/`.
- Any dependencies:
  - Avoid adding crates unless required.
  - Prefer standard library filesystem operations.
- Additional notes:
  - File moves are expected to occur directly within the repository during implementation of F-007.
  - ADR-002 should be read before implementation and treated as architectural authority.

# Open Questions
- None.
