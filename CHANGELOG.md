# Changelog

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

