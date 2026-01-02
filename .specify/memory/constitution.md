# Specdrive Constitution

## Core Principles

### I. Spec + Contract First (NON-NEGOTIABLE)
Every non-trivial change must be defined by:
- A feature spec in `.specify/specs/<FEATURE_ID>.spec.md`
- A feature contract in `docs/features/<FEATURE_ID>/contract.yaml`

Code changes **must follow** the spec + contract, not the other way around.
Specs and contracts are versioned in git and reviewed like code.

### II. CLI-First, Simple, and Boring
Specdrive is a **single binary CLI** with:
- Simple subcommands (`bootstrap`, `new-feature`, `implement`, etc.)
- No hidden side effects
- No external crates unless explicitly approved in a feature contract

Commands should:
- Read/write plain files
- Print clear, copy-pasteable output
- Fail fast with explicit error messages

### III. AI as a Junior Dev, Not the Architect
AI is used to:
- Draft specs and contracts
- Implement code **inside the boundaries** set by specs + contracts

AI is **not** allowed to:
- Change specs or contracts without explicit instruction
- Add dependencies not permitted by the contract
- Bypass invariants (e.g., git safety, read-only commands)

Human responsibilities:
- Decide which features are `--critical`
- Review diffs
- Keep specs and contracts aligned with reality

### IV. Safety and Reversibility
- Prefer read-only commands for early versions (e.g., `implement` prints prompts only).
- Any command that mutates code or config must:
  - Be clearly documented in its contract (`git_safety`, `filesystem` sections).
  - Be easy to revert via git.
- No network calls from specdrive in v0.1; it operates on **local files only**.

#### Defensive helpers (NON-NEGOTIABLE FOR CLI FLOWS)

- Shared defensive helpers must be used instead of ad-hoc checks.
- `utils::ensure_repo_and_specify_ready()` is the canonical preflight for “spec-aware” commands:
  - Verifies we are in a git repo (`.git/` exists).
  - Verifies Spec Kit is initialized (`.specify/` exists).
  - Verifies the working tree is clean according to `git_safety`.
- Commands that depend on repo/spec state **must** call this helper (or its future equivalent), including:
  - `bootstrap` (once refactored in F-004),
  - `implement`,
  - `draft`,
  - and any future commands that read or write specs/contracts.

## Development Workflow

### Feature Flow
1. Create a feature:
   - `specdrive new-feature <FEATURE_ID> [--critical]`
   - Edit the generated spec + contract until they match the intended behavior.
2. Commit spec + contract **before** AI-implemented code.
3. Use AI (via Claude / Spec Kit) to implement or update code:
   - AI must read:
     - `.specify/specs/<FEATURE_ID>.spec.md`
     - `docs/features/<FEATURE_ID>/contract.yaml`
4. Review and run tests; then commit code changes.

### Command Design
- Each new command must:
  - Have a feature id (F-XXX)
  - Have a spec + contract
  - Describe CLI shape, behavior, invariants, and filesystem effects
- `--critical` features use the maximal contract template and tighter rules.

## Architecture Decision Records (ADRs)

- Long-lived technical and process decisions are captured as ADRs under `docs/adrs/ADR-xxx-*.md`.
- ADRs record:
  - Context (why this decision was needed)
  - Decision
  - Consequences
  - Status (proposed / accepted / superseded)
- Features that implement or depend on an ADR should:
  - Reference the ADR in their spec front-matter or context section (e.g. “Implements ADR-001”).
  - Optionally be referenced back from the ADR (“Affected features: F-001, F-002…”).

Rules of thumb:
- If it changes how specdrive is used across projects, or affects multiple future features, it probably deserves an ADR.
- Minor internal refactors that don’t change behavior or external contracts generally do **not** need an ADR.

ADRs and specs/contracts:
- ADRs set **direction and constraints**.
- Feature specs + contracts describe **specific changes** within that direction.
- If an ADR is superseded, affected future features should reference the newer ADR instead of editing old ones.

## Additional Constraints

- Technology:
  - Rust, standard library first.
  - No new crates without an explicit rationale in a feature contract (and ADR if it’s a cross-cutting choice).
- Security:
  - No hard-coded secrets.
  - No network traffic from the CLI in v0.1.
- Repo layout is stable:
  - `.specify/` for specs/templates/memory
  - `docs/features/` for contracts
  - `docs/adrs/` for architecture decision records
  - `src/` for Rust code

## Governance

- This constitution is the top-level guidance for specdrive’s SDLC.
- Any change that breaks these principles (e.g., adding network calls, changing the feature flow) must:
  - Be introduced via a feature spec + contract;
  - Be backed by an ADR when it has cross-cutting or long-term impact;
  - Update this constitution with a version bump and rationale.
- ADRs are append-only:
  - Old ADRs are not rewritten; they are marked as superseded by newer ADRs when direction changes.

**Version**: 0.1.0 | **Ratified**: 2025-12-29 | **Last Amended**: 2025-12-29
