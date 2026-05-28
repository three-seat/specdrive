# Specdrive System Overview

## Purpose

Specdrive is a language-agnostic **spec/contract + AI helper CLI**.

It does **not** own your build system or framework. Instead, it:

- Standardizes how features are described (`spec` + `contract`).
- Wires those artifacts into AI workflows (via prompts, not APIs in v0.x).
- Enforces basic safety (git, clean tree) before you let AI touch code.

Goal: you can drop specdrive into any git repository and get a repeatable,
auditable AI dev loop. Per ADR-002 / F-007, SpecDrive no longer requires
Spec Kit or `.specify/`.

---

## Core Concepts

- **Feature ID (`F-XXX-something`)**
  - Unique identifier for a feature.
  - Drives naming for specs, contracts, patches, and internal references.

- **Spec (human-facing)**
  - Location: `docs/features/<FEATURE_ID>/spec.md`
  - Markdown with front matter + prose.
  - Explains behavior, context, acceptance criteria, implementation notes.

- **Contract (machine-ish)**
  - Location: `docs/features/<FEATURE_ID>/contract.yaml`
  - Structured YAML:
    - `metadata`, `requirements`, `behavior`, `logic`, `filesystem`, `git_safety`, `verification`, `ai_instructions`, etc.
  - Acts as the **source of truth** for implementation and tests.

- **Patches (implementation artifacts)**
  - Location: `docs/features/<FEATURE_ID>/patches/`
  - Per-feature directory holding emitted patches (`<FEATURE_ID>.patch`).

- **Constitution**
  - Location: `docs/constitution.md`
  - Top-level rules for how specdrive is developed (principles, workflow, safety rules).

- **ADRs (Architecture Decision Records)**
  - Location: `docs/adrs/`
  - Record major decisions (e.g., bootstrap assumptions, AI usage strategy,
    feature-local artifact architecture).
  - Specs/contracts can reference ADRs; AI is told to read them.

---

## Repository Layout

Key paths specdrive expects (post-F-007):

- `.git/`
  Git repository root; required for all specdrive flows.

- `docs/`
  - `features/<FEATURE_ID>/` — canonical per-feature artifact directory
    - `spec.md` — feature spec
    - `contract.yaml` — feature contract
    - `patches/` — emitted patches for the feature
  - `templates/`
    - `feature.spec.md` — feature spec template (installed by `bootstrap`)
    - `feature.contract.minimal.yaml` — minimal contract template
    - `feature.contract.critical.yaml` — critical contract template
  - `adrs/ADR-XXX-*.md` — architecture decision records
  - `ai/implement-header.md` / `ai/implement-footer.md` / `ai/draft-header.md`
    / `ai/draft-footer.md` (optional prompt fragments)
  - `constitution.md` — project constitution
  - `system-overview.md` — this document

- `src/`
  - Rust implementation of the CLI:
    - `cli.rs` — argument parsing / subcommand dispatch
    - `bootstrap.rs` — bootstrap logic
    - `feature.rs` — `new-feature` scaffolding
    - `feature_spec.rs` — feature spec creation from template
    - `implement.rs` — `implement` command
    - `draft.rs` — `draft` command
    - `patch.rs` — `patch emit` command
    - `utils.rs` — helpers like `ensure_repo_ready()`
    - `git.rs`, `fsutil.rs`, `config.rs`

---

## Commands and Flows

### `specdrive bootstrap`

**Purpose:** prepare an existing git repo for specdrive.

- Preconditions:
  - `.git/` exists.
- Behavior:
  - Ensures `docs/`, `docs/features/`, `docs/templates/` exist.
  - Installs:
    - `docs/templates/feature.spec.md` (if missing).
    - `docs/templates/feature.contract.minimal.yaml` (if missing).
    - `docs/templates/feature.contract.critical.yaml` (if missing).
  - Never overwrites or deletes existing files.
- Role: make any git repo "specdrive-ready" without touching user code.
- Per ADR-002 / F-007, bootstrap no longer requires `.specify/` and no longer
  creates feature-local `prompts/` or `outputs/` directories.

### `specdrive new-feature <FEATURE_ID> [--critical]`

**Purpose:** scaffold a new feature's spec + contract.

- Uses the templates installed by `bootstrap`.
- Creates:
  - `docs/features/<FEATURE_ID>/spec.md`
  - `docs/features/<FEATURE_ID>/contract.yaml`
  - `docs/features/<FEATURE_ID>/patches/`
- `--critical` chooses the maximal (critical) contract template.
- Refuses to overwrite existing files.

### `specdrive implement <FEATURE_ID>`

**Purpose:** generate a deterministic, AI-ready prompt to implement `<FEATURE_ID>`.

- Preconditions (via `utils::ensure_repo_ready()`):
  - In a git repo (`.git/` present).
  - Working tree is clean (uncommitted changes cause a fail-fast).

- Behavior:
  - Resolves and validates:
    - `docs/features/<FEATURE_ID>/spec.md`
    - `docs/features/<FEATURE_ID>/contract.yaml`
  - Parses `contract.yaml` (YAML) to:
    - Confirm validity.
    - Enforce critical review gate for `metadata.critical: true`.
  - Optionally reads prompt fragments:
    - `docs/ai/implement-header.md`
    - `docs/ai/implement-footer.md`
  - Prints a prompt that:
    - Lists files the AI must read (spec, contract, constitution, ADRs, system overview).
    - States guardrails (no new deps, respect invariants, read-only nature, etc.).
    - **Does not inline** the contents of spec/contract.

- Role: a **bridge** from "we have a spec + contract" to "AI writes code" using local files.

### `specdrive draft <FEATURE_ID>`

**Purpose:** generate an AI prompt to draft or refine `contract.yaml` from a
spec and/or human notes.

- Reuses `utils::ensure_repo_ready()` for safety.
- Reads `docs/features/<FEATURE_ID>/spec.md` and the current/skeleton
  `contract.yaml`, plus the contract templates under `docs/templates/`.
- Prints a structured prompt; does not modify any files.

### `specdrive patch emit <FEATURE_ID>`

**Purpose:** capture the current working-tree diff against `HEAD` as a
reviewable patch.

- Writes `docs/features/<FEATURE_ID>/patches/<FEATURE_ID>.patch`.
- Validates the patch using `git apply --check`.
- Warns about untracked files (which are not included).
- Excludes the patch file itself from its own diff.

---

## Safety Model

- **Shared defensive helper**
  - `utils::ensure_repo_ready()` is the standard gate for spec-aware commands
    (`implement`, `draft`). It verifies `.git/` and a clean tree.
- **Read-only AI flows**
  - `implement` and `draft` print prompts only; no code changes.
- **Git-backed reversibility**
  - All mutations are expected to be reversible via git.
- **Local-only**
  - No network calls; specdrive works entirely on local files and git state.

---

## How AI Uses This

Specdrive assumes AI tools can:

- Read files from disk (specs, contracts, ADRs, constitution, system overview).
- Follow path-based instructions from prompts.
- Treat contracts as the non-negotiable source of truth.

Specdrive's job is to make those instructions **consistent, safe, and repeatable**
across features and repos.
