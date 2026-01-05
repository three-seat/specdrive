# Specdrive System Overview

## Purpose

Specdrive is a language-agnostic **spec/contract + AI helper CLI**.

It does **not** own your build system or framework. Instead, it:

- Standardizes how features are described (`spec` + `contract`).
- Wires those artifacts into AI workflows (via prompts, not APIs in v0.1).
- Enforces basic safety (git, Spec Kit, clean tree) before you let AI touch code.

Goal: you can drop specdrive into any git + Spec Kit repo and get a repeatable, auditable AI dev loop.

---

## Core Concepts

- **Feature ID (`F-XXX-something`)**
  - Unique identifier for a feature.
  - Drives naming for specs, contracts, and internal references.

- **Spec (human-facing)**
  - Location: `.specify/specs/<FEATURE_ID>.spec.md`
  - Markdown with front matter + prose.
  - Explains behavior, context, acceptance criteria, implementation notes.

- **Contract (machine-ish)**
  - Location: `docs/features/<FEATURE_ID>/contract.yaml`
  - Structured YAML:
    - `metadata`, `requirements`, `behavior`, `logic`, `filesystem`, `git_safety`, `verification`, `ai_instructions`, etc.
  - Acts as the **source of truth** for implementation and tests.

- **Constitution**
  - Location: `.specify/memory/constitution.md`
  - Top-level rules for how specdrive is developed (principles, workflow, safety rules).

- **ADRs (Architecture Decision Records)**
  - Location: `docs/adrs/`
  - Record major decisions (e.g., bootstrap assumptions, AI usage strategy).
  - Specs/contracts can reference ADRs; AI is told to read them.

---

## Repository Layout (v0.1)

Key paths specdrive expects:

- `.git/`  
  Git repository root; required for all specdrive flows.

- `.specify/`
  - `specs/` — feature specs (`<FEATURE_ID>.spec.md`)
  - `templates/` — `feature.spec.md` template (installed by `bootstrap`)
  - `memory/constitution.md` — project constitution

- `docs/`
  - `features/<FEATURE_ID>/contract.yaml` — feature contracts
  - `templates/feature.contract.minimal.yaml` — minimal feature template
  - `templates/feature.contract.critical.yaml` — critical feature template
  - `adrs/ADR-XXX-*.md` — architecture decision records
  - `ai/implement-header.md` / `ai/implement-footer.md` (optional prompt fragments)
  - `system-overview.md` — this document

- `src/`
  - Rust implementation of the CLI:
    - `cli.rs` — argument parsing / subcommand dispatch
    - `bootstrap.rs` — bootstrap logic (F-001)
    - `features.rs` — `new-feature` scaffolding (F-000-style)
    - `implement.rs` — `implement` command (F-002)
    - `utils.rs` — helpers like `ensure_repo_and_specify_ready()`
    - `git.rs`, `fsutil.rs`, etc.

---

## Commands and Flows

### `specdrive bootstrap` (F-001)

**Purpose:** prepare an existing repo (already git + Spec Kit) for specdrive.

- Preconditions:
  - `.git/` exists.
  - `.specify/` exists (from `specify init`).
- Behavior:
  - Ensures `.specify/specs/` exists.
  - Installs:
    - `.specify/templates/feature.spec.md` (if missing).
    - `docs/templates/feature.contract.minimal.yaml` (if missing).
    - `docs/templates/feature.contract.critical.yaml` (if missing).
  - Never overwrites or deletes existing files.
- Role: make any repo “specdrive-ready” without touching user code.

### `specdrive new-feature <FEATURE_ID> [--critical]`

**Purpose:** scaffold a new feature’s spec + contract.

- Uses the templates installed by `bootstrap`.
- Creates:
  - `.specify/specs/<FEATURE_ID>.spec.md`
  - `docs/features/<FEATURE_ID>/contract.yaml`
- `--critical` chooses the maximal (critical) contract template.
- Idempotent for existing files (respects `fsutil`’s no-overwrite rules).

### `specdrive implement <FEATURE_ID>` (F-002)

**Purpose:** generate a deterministic, AI-ready prompt to implement `<FEATURE_ID>`.

- Preconditions (via `utils::ensure_repo_and_specify_ready()`):
  - In a git repo (`.git/` present).
  - `.specify/` exists.
  - Working tree is clean (uncommitted changes cause a fail-fast).

- Behavior:
  - Resolves and validates:
    - `.specify/specs/<FEATURE_ID>.spec.md`
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

- Role: a **bridge** from “we have a spec + contract” to “AI writes code” using local files.

### Future (not in v0.1, but planned)

- `specdrive draft <FEATURE_ID>` (F-003)
  - Generate an AI prompt to draft or refine `contract.yaml` from a spec and/or human notes.
  - Reuse `utils::ensure_repo_and_specify_ready()` for safety.

- `specdrive bootstrap` refactor (F-004)
  - Make `bootstrap` use the shared helper for repo/spec checks.

- Configurable naming conventions and AI accept/reject flows (F-005, F-006, etc.).

---

## Safety Model

- **Shared defensive helper**
  - `utils::ensure_repo_and_specify_ready()` is the standard gate:
    - Enforced for `implement` and future `draft` and refactored `bootstrap`.
- **Read-only for v0.1 AI flows**
  - `implement` prints prompts only, no code changes.
- **Git-backed reversibility**
  - All mutations (when introduced) are expected to be reversible via git.
- **Local-only**
  - No network calls in v0.1; specdrive works entirely on local files and git state.

---

## How AI Uses This

Specdrive assumes AI tools can:

- Read files from disk (specs, contracts, ADRs, constitution, system overview).
- Follow path-based instructions from prompts.
- Treat contracts as the non-negotiable source of truth.

Specdrive’s job is to make those instructions **consistent, safe, and repeatable** across features and repos.
