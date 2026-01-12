---
id: F-005-naming-config
title: 005 Configurable naming conventions
type: feature
system: specdrive
status: draft
area: cli
owners:
  - three_seat
created_at: 2026-01-12
contract: docs/features/F-005-naming-config/contract.yaml
---

# Summary

Add support for **configurable feature naming conventions** so teams can codify how feature IDs look (e.g. `F-001-bootstrap`) and have `specdrive` enforce that convention. The configuration should be optional, live in a simple YAML file, and be respected by `new-feature`, `implement`, and `draft` without changing path layout or doing any wildcard file searches.

# Context

- Today:
  - We implicitly use IDs like `F-001-bootstrap`, but nothing enforces that shape.
  - `specdrive` assumes paths:
    - `.specify/specs/<FEATURE_ID>.spec.md`
    - `docs/features/<FEATURE_ID>/contract.yaml`
  - Security choices:
    - We do **not** scan for files via globs or prefixes.
    - We derive exact paths from `FEATURE_ID`.

- We want:
  - A way to encode “this is what a valid feature id looks like” in config.
  - Validation in all relevant commands.
  - No change to how paths are constructed (still `<FEATURE_ID>` plugged into fixed templates).
  - No increased LFI risk (still no directory traversal, no globbing).

This is about **validation and policy**, not changing the filesystem layout.

# Behavior

## Config file

- Config location (v0.1):
  - `docs/specdrive/config.yaml` (optional).

- If the config file is **missing**:
  - `specdrive` falls back to built-in defaults and behaves exactly as it does today.

- If the config file is **present**, we support at least a `naming.feature` section, for example:

  **CONFIG SNIPPET GOES HERE**  
  (e.g. `naming.feature.pattern`, `example`, `description`)

- Future fields (out of scope for v0.1 but can be mentioned as comments in the template):
  - `naming.feature.pattern_flags`
  - Separate patterns for different “namespaces” (e.g. infra vs app).

## Safety checks (before regex)

For any `FEATURE_ID` we:

- Reject IDs that contain:
  - `/`, `\`, or `..`
  - Control characters or whitespace

These checks apply **even if no config file exists**.

If a `FEATURE_ID` fails basic safety:

- Print a clear error (e.g. “FEATURE_ID must not contain `/` or `..`”) and exit with code 1.

## Where naming rules apply

The naming convention and safety checks are enforced in:

- `specdrive new-feature <FEATURE_ID> [--critical]`
- `specdrive implement <FEATURE_ID>`
- `specdrive draft <FEATURE_ID>`

For v0.1:

- All three commands:
  - Run the **basic safety checks**.
  - If config exists, validate `FEATURE_ID` against `naming.feature.pattern`.
  - On mismatch:
    - Print an error that includes:
      - The invalid `FEATURE_ID`
      - The pattern and example from config (if available)
    - Exit with code 1 (usage / precondition failure).

## Detailed behavior per command

### new-feature

Before doing any filesystem work:

1. Run safety checks on `FEATURE_ID`.
2. Load `docs/specdrive/config.yaml` if it exists.
3. If `naming.feature.pattern` is present, validate `FEATURE_ID` against it.
4. On validation failure:
   - Do not create any spec/contract files.
   - Exit code 1 with a clear error.
5. On success:
   - Continue as today (create spec + contract skeleton) using the unchanged path scheme.

### implement

Before resolving paths:

1. Run safety checks on `FEATURE_ID`.
2. Load `docs/specdrive/config.yaml` if it exists.
3. Validate against `naming.feature.pattern` if present.
4. On failure:
   - Print a clear error and exit code 1.
5. On success:
   - Continue existing F-002 flow (repo/specify checks, read contract, etc.).

### draft

Same pattern as `implement`:

1. Run safety checks on `FEATURE_ID`.
2. Load config if present.
3. Validate against `naming.feature.pattern` if present.
4. On failure:
   - Print error, exit code 1.
5. On success:
   - Continue existing F-003 flow.

# Non-Functional Requirements

- **Backward compatible**:
  - Repos without `docs/specdrive/config.yaml` should behave exactly as before.

- **Deterministic**:
  - For a given repo state + config, the pass/fail outcome for a `FEATURE_ID` is deterministic.

- **Security**:
  - No globs or directory traversal based on `FEATURE_ID`.
  - Paths remain of the form:
    - `.specify/specs/<FEATURE_ID>.spec.md`
    - `docs/features/<FEATURE_ID>/contract.yaml`

- **Simplicity**:
  - Config schema is small and stable.
  - Only one regex-based rule for v0.1 (`naming.feature.pattern`).

# Acceptance Criteria

- [ ] AC-1: With no `docs/specdrive/config.yaml` present, `specdrive new-feature`, `implement`, and `draft` accept existing IDs like `F-001-bootstrap` and behave as today.
- [ ] AC-2: If `docs/specdrive/config.yaml` defines a `naming.feature.pattern`, then:
  - `specdrive new-feature` with a matching ID (e.g. `F-001-bootstrap`) succeeds.
  - `specdrive new-feature` with an invalid ID (e.g. `feature-foo`) fails with a clear pattern-based error and exits 1.
- [ ] AC-3: Passing a `FEATURE_ID` containing `/` or `..` (e.g. `../../etc/passwd`) causes all three commands (`new-feature`, `implement`, `draft`) to fail with a clear safety error and exit code 1, even if no config file exists.
- [ ] AC-4: If config is present but syntactically invalid YAML, the command prints a clear error including the config path and exits 2 (treat as parse error, not as “no config”).
- [ ] AC-5: All commands remain **read-only** with respect to `docs/specdrive/config.yaml` (never modify it).
- [ ] AC-6: Error messages for invalid IDs include:
  - The actual `FEATURE_ID`
  - The regex pattern (if configured)
  - The example (if configured).

# Implementation Notes

- Config:
  - Add a small `config` module to handle:
    - Loading `docs/specdrive/config.yaml` if it exists.
    - Parsing into a simple struct (e.g. via `serde_yaml`) in v0.1, or minimal manual parsing if serde is deferred.
  - Expose a helper: `validate_feature_id(feature_id: &str) -> Result<()>` that:
    - Performs safety checks.
    - Optionally enforces the configured pattern.

- Call sites:
  - `new-feature`, `implement`, and `draft` call `validate_feature_id` early before any other work.

- Dependencies:
  - If `serde_yaml` is already in use (e.g. F-002), reuse it here.
  - No other new crates.

# Open Questions

- Q1: Should we support multiple naming schemes (e.g. “infra features” vs “product features”) keyed off a prefix or folder in a later version?
- Q2: Do we want a `specdrive config init` helper in a future feature to scaffold `docs/specdrive/config.yaml`?
- Q3: In the long term, should path templates (where spec/contract live) also be configurable, or are IDs the only thing we want to customize?
