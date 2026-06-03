# Specdrive System Overview

## Purpose

Specdrive is a language-agnostic **artifact-driven engineering workflow system**
for building software with or without AI assistance.

It does **not** own your build system or framework. Instead, it:

- Standardizes how features are described (`spec` + `contract`).
- Treats specifications, contracts, prompts, outputs, patches, and manifests
  as first-class artifacts.
- Wires those artifacts into AI workflows (via prompts, not APIs in v0.x).
- Enforces lifecycle gates and safety checks before AI touches code.

Goal: you can drop SpecDrive into any git repository and get a repeatable,
auditable, artifact-driven development loop. Per ADR-002 / F-007, SpecDrive
no longer requires Spec Kit or `.specify/`.

---

## Architectural Principles

These principles govern all SpecDrive design decisions. They are formally
established in the constitution (v1.0.0, Sections V–IX) and recorded in
ADR-003.

**Artifacts are authoritative. Tools and agents are interchangeable.**
Canonical artifacts (spec, contract, manifest, review record) are the durable
source of truth. Derived artifacts (prompt, output, patch, release summary)
are reproducible from canonical artifacts and must never be treated as
authoritative.

**SpecDrive owns the lifecycle. AI owns artifact production only.**
SpecDrive owns lifecycle state, transitions, validation, and advancement gates.
AI produces artifacts but never advances state, controls workflow, or runs
SpecDrive commands. Autonomous orchestration, swarm behavior, and automatic
state advancement are explicitly rejected.

**Repository portability.**
SpecDrive state must remain repository-local and fully reconstructable from
version-controlled artifacts alone. No external databases, cloud services,
or agent memory. Cloning the repository must be sufficient to reconstruct
the full feature history and artifact record.

**Bounded AI execution.**
One prompt → one output → one patch. Complex features are decomposed into
bounded contracts. Aggregate prompts producing aggregate patches are
explicitly rejected.

**Traceability as a governing constraint.**
Every change must be traceable from intent to implementation:
```
spec → contract → prompt → output → patch → review → release
```
Traceability is not a reporting feature. It is a governing constraint on
how features are designed, implemented, and verified.

---

## Core Concepts

- **Feature ID (`F-XXX-something`)**
  - Unique identifier for a feature.
  - Drives naming for specs, contracts, patches, and internal references.

- **Spec (human-facing)**
  - Location: `docs/features/<FEATURE_ID>/spec.md`
  - Markdown with front matter + prose.
  - Explains behavior, context, acceptance criteria, implementation notes.
  - Canonical artifact.

- **Contract (machine-ish)**
  - Location: `docs/features/<FEATURE_ID>/contract.yaml`
  - Structured YAML:
    - `metadata`, `requirements`, `behavior`, `logic`, `filesystem`,
      `git_safety`, `verification`, `ai_instructions`, etc.
  - Acts as the **source of truth** for implementation and tests.
  - Canonical artifact.

- **Prompt (AI execution plan)**
  - Produced by `specdrive implement` or `specdrive draft`.
  - Instructs an AI tool to read specific artifacts and act within
    defined guardrails.
  - Derived artifact — reproducible from spec, contract, and context.

- **Patches (implementation artifacts)**
  - Location: `docs/features/<FEATURE_ID>/patches/`
  - Per-feature directory holding emitted patches (`<FEATURE_ID>.patch`).
  - Derived artifact — produced from AI or human implementation against
    the contract.

- **Manifest (planned — F-017)**
  - Canonical inventory of all feature artifacts with hashes and timestamps.
  - Records relationships between specs, contracts, prompts, outputs,
    and patches.
  - Canonical artifact — single source of truth for audit and release.

- **Constitution**
  - Location: `docs/constitution.md`
  - Top-level governing principles for how SpecDrive is developed.
  - See especially Sections V–IX for artifact and lifecycle principles.

- **ADRs (Architecture Decision Records)**
  - Location: `docs/adrs/`
  - Record major decisions with context, reasoning, and consequences.
  - ADR-003 records the artifact ownership and traceability model.
  - Specs/contracts can reference ADRs; AI is told to read them.

---

## Repository Layout

Key paths SpecDrive expects (post-F-007, per ADR-002):

- `.git/`
  Git repository root; required for all SpecDrive flows.

- `docs/`
  - `features/<FEATURE_ID>/` — canonical per-feature artifact directory
    - `spec.md` — feature spec (canonical)
    - `contract.yaml` — feature contract (canonical)
    - `patches/` — emitted patches for the feature (derived)
    - `prompts/` — persisted prompts (derived, planned — F-014)
    - `outputs/` — AI outputs (derived, planned — F-014)
  - `templates/`
    - `feature.spec.md` — feature spec template (installed by `bootstrap`)
    - `feature.contract.minimal.yaml` — minimal contract template
    - `feature.contract.critical.yaml` — critical contract template
  - `adrs/ADR-XXX-*.md` — architecture decision records
  - `ai/implement-header.md` / `ai/implement-footer.md` /
    `ai/draft-header.md` / `ai/draft-footer.md` (optional prompt fragments)
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

**Purpose:** prepare an existing git repo for SpecDrive.

- Preconditions:
  - `.git/` exists.
- Behavior:
  - Ensures `docs/`, `docs/features/`, `docs/templates/` exist.
  - Installs:
    - `docs/templates/feature.spec.md` (if missing).
    - `docs/templates/feature.contract.minimal.yaml` (if missing).
    - `docs/templates/feature.contract.critical.yaml` (if missing).
  - Never overwrites or deletes existing files.
- Role: make any git repo "SpecDrive-ready" without touching user code.
- Per ADR-002 / F-007, bootstrap no longer requires `.specify/` and no
  longer creates feature-local `prompts/` or `outputs/` directories.
  Those directories are planned for F-014.

### `specdrive new-feature <FEATURE_ID> [--critical]`

**Purpose:** scaffold a new feature's spec + contract.

- Uses the templates installed by `bootstrap`.
- Creates:
  - `docs/features/<FEATURE_ID>/spec.md`
  - `docs/features/<FEATURE_ID>/contract.yaml`
  - `docs/features/<FEATURE_ID>/patches/`
- `--critical` chooses the maximal (critical) contract template.
- Refuses to overwrite existing files.

### `specdrive draft <FEATURE_ID>`

**Purpose:** generate a structured contract-authoring prompt for a feature.

- Reuses `utils::ensure_repo_ready()` for safety.
- Reads:
  - `docs/features/<FEATURE_ID>/spec.md`
  - Current or skeleton `contract.yaml`
  - Contract templates under `docs/templates/`
  - Constitution, ADRs, system overview
- Produces a prompt that instructs an AI to draft or refine the contract,
  with explicit mapping guidance from spec sections to contract sections
  and embedded guardrails (do not weaken invariants, do not change feature
  IDs, follow template structure).
- Prints prompt only; does not modify any files.
- The resulting prompt is intended for use with an external AI chat tool.
  The contract is reviewed and committed before implementation begins.

### `specdrive implement <FEATURE_ID>`

**Purpose:** generate a deterministic, AI-ready prompt to implement
`<FEATURE_ID>`.

- Preconditions (via `utils::ensure_repo_ready()`):
  - In a git repo (`.git/` present).
  - Working tree is clean (uncommitted changes cause a fail-fast).
- Behavior:
  - Resolves and validates:
    - `docs/features/<FEATURE_ID>/spec.md`
    - `docs/features/<FEATURE_ID>/contract.yaml`
  - Parses `contract.yaml` to:
    - Confirm validity.
    - Enforce critical review gate for `metadata.critical: true`.
  - Optionally reads prompt fragments:
    - `docs/ai/implement-header.md`
    - `docs/ai/implement-footer.md`
  - Prints a prompt that:
    - Lists files the AI must read (spec, contract, constitution, ADRs,
      system overview).
    - States guardrails (no new deps, respect invariants, do not modify
      specs or contracts, read-only nature of the command).
    - Does not inline the contents of spec/contract.
- Role: a **bridge** from "we have a spec + contract" to "AI writes code"
  using local files. The contract is the execution plan. The prompt
  delivers it to the AI.

### `specdrive patch emit <FEATURE_ID>`

**Purpose:** capture the current working-tree diff against `HEAD` as a
reviewable patch artifact.

- Writes `docs/features/<FEATURE_ID>/patches/<FEATURE_ID>.patch`.
- Validates the patch using `git apply --check`.
- Warns about untracked files (which are not included).
- Excludes the patch file itself from its own diff.

---

## Safety Model

- **Shared defensive helper**
  - `utils::ensure_repo_ready()` is the standard gate for spec-aware
    commands (`implement`, `draft`). It verifies `.git/` and a clean tree.
- **Read-only AI flows**
  - `implement` and `draft` print prompts only; no code changes.
- **Git-backed reversibility**
  - All mutations are expected to be reversible via git.
- **Local-only**
  - No network calls; SpecDrive works entirely on local files and git state.
- **Guardrails embedded in prompts**
  - Generated prompts include explicit guardrails instructing AI not to
    weaken invariants, modify canonical artifacts, or exceed contract scope.
  - These are runtime constraints, not just documented principles.

---

## How AI Uses This

SpecDrive assumes AI tools can:

- Read files from disk (specs, contracts, ADRs, constitution, system overview).
- Follow path-based instructions from prompts.
- Treat contracts as the non-negotiable source of truth.

SpecDrive's job is to make those instructions **consistent, safe, and
repeatable** across features and repos.

AI is a producer of derived artifacts within boundaries set by canonical
artifacts. It is never a decision-maker about lifecycle state or workflow
advancement. See ADR-003 for the formal record of this boundary.
