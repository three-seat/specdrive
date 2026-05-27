# Changelog

## v0.1.0 - 2026-02-06
### Added
- File-based configuration for SpecDrive (initial config format).
- Configurable paths and defaults to support different repo layouts.
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

