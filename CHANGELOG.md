# Changelog

## [0.5.0] - 2026-07-08

### Added

- Lifecycle state enforcement (F-010)
- Append-only `state.yaml` lifecycle log
- Lifecycle validation and transition rules
- Foundation for future review, traceability, and workflow automation

### Changed

- Feature lifecycle is now enforced rather than purely documented.


## v0.4.0 — Chat Export/Import Workflow

- Added `specdrive chat export` and `specdrive chat import` subcommands bridging SpecDrive's artifact model with stateless AI chat tools
- Export assembles a self-contained, delimited context bundle from resolved feature artifacts and prints to stdout — no clipboard, no API, works with any AI chat tool
- Import reads a delimited AI response from stdin, validates it, previews changes, and writes artifacts only on explicit confirmation
- Import defends against path traversal, LFI, symlink escape, delimiter injection, and oversized responses — all rejections occur before any write
- Introduced configurable size limits under `chat.import` config namespace with safe built-in defaults
- Extracted shared file resolver and output numbering utilities as foundation for future artifact lineage features
- Updated critical contract template — review records now belong in `state.yaml` (F-010), not in `contract.yaml`
## v0.3.1 - 2026-05-28

### Fixed

- Updated tests to reflect the feature-local artifact architecture introduced in v0.3.0.
- Removed stale `.specify/` assumptions from test fixtures and validation logic.
- Fixed path-resolution and fixture issues uncovered during the F-007 migration.

### Changed

- Refactored draft prompt generation to use a structured context object.
- Simplified several internal code paths based on clippy recommendations.
- Removed obsolete helper code and minor implementation debt.

### Quality

- `cargo build` passes.
- `cargo clippy -- -D warnings` passes.
- Full test suite passes.

### Notes

This is a stabilization release following the v0.3.0 architecture migration. No user-facing functionality was added.

## v0.3.0 - 2026-05-28
### Breaking changes
- Feature-local artifact architecture (F-007, ADR-002): every feature now owns its spec, contract, and patches under a single directory:
  ```text
  docs/features/<FEATURE_ID>/
    spec.md
    contract.yaml
    patches/
  ```
- Feature specs moved from `.specify/specs/<FEATURE_ID>.spec.md` to `docs/features/<FEATURE_ID>/spec.md`.
- Feature spec template moved from `.specify/templates/feature.spec.md` to `docs/templates/feature.spec.md`.
- Constitution moved from `.specify/memory/constitution.md` to `docs/constitution.md`.
- `.claude/` removed from the repository.
- Spec Kit / `.specify/` is no longer required for normal SpecDrive operation:
  - `bootstrap` no longer requires `.specify/` and no longer creates feature-local `prompts/` or `outputs/` directories.
  - `new-feature`, `draft`, `implement`, and `patch emit` now read and write feature-local artifacts under `docs/features/<FEATURE_ID>/`.
  - `utils::ensure_repo_ready()` validates only `.git/` + a clean working tree.

### Migration
- This is an intentional breaking layout change with no runtime migration command, no `.specify/specs/` fallback resolution, and no long-term compatibility layer.
- Users on `v0.1.x` / `v0.2.x` must move existing artifacts manually:
  - `.specify/specs/<FEATURE_ID>.spec.md` → `docs/features/<FEATURE_ID>/spec.md`
  - `.specify/templates/feature.spec.md` → `docs/templates/feature.spec.md`
  - `.specify/memory/constitution.md` → `docs/constitution.md`

### Safety
- No network calls, AI API calls, auto-commit, patch application, or git history mutation were added.
- No new dependencies were added.

### Docs
- `docs/system-overview.md`, `docs/constitution.md`, and tests updated to describe the new layout.
- ADR-002 records the architectural decision; ADR-001 assumptions about Spec Kit / `.specify/` are superseded by ADR-002.

## v0.2.0 - 2026-05-27
### Added
- Added first-class patch workflow:
  ```bash
  specdrive patch emit <FEATURE_ID>
  ```
- Added feature-local patch artifact generation:
  ```text
  docs/features/<FEATURE_ID>/patches/<FEATURE_ID>.patch
  ```
- Added patch generation using:
  ```bash
  git diff --binary HEAD
  ```
- Added support for staged newly created files in emitted patches.
- Added warnings for untracked files that are not automatically included.
- Added automatic creation of feature-local patch directories.
- Added patch validation workflow using clean git worktrees.
- Added patch summary output including:
  - patch path
  - changed file count
  - insertions
  - deletions

### Changed
- Promoted patch emission from a proposed `implement --emit-patch` flag to a dedicated:
  ```bash
  specdrive patch emit
  ```
  workflow.
- Preserved existing prompt-only `implement` behavior.

### Safety
- Patch workflows remain:
  - local-only
  - additive
  - human-reviewed
- No AI APIs, network calls, auto-commit, patch application, or git history mutation were added.

### Notes
- Repository-wide clippy/test stabilization remains future work and was intentionally kept out of F-006 scope.

## v0.1.0 - 2026-02-06
### Added
- File-based configuration for SpecDrive (initial config format).
- Configurable paths and defaults to support different repo layouts.

