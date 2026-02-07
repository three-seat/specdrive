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

## Requirements Structure (Contracts)

All `contract.yaml` files must structure requirements as:

- `requirements.high_level`: array of high-level requirements.
- `requirements.low_level`: array of low-level requirements that refine HLRs.

### High-Level Requirements (HLR)

- Each HLR MUST have an `id` of the form: `HLR-###`
  - Example: `HLR-001`, `HLR-002`, ...
- Each HLR MUST have a `text` field describing the requirement in clear language.
- HLRs describe *what* the feature must achieve from a product/behaviour standpoint.

### Low-Level Requirements (LLR)

- Each LLR MUST have an `id` of the form: `LLR-###`
  - Example: `LLR-001`, `LLR-002`, ...
- Each LLR MUST have:
  - `parent`: the `id` of an existing HLR (e.g., `HLR-001`).
  - `text`: a concrete, implementable refinement of that HLR.
- Every `LLR.parent` MUST refer to a valid HLR in the same contract.
- LLRs describe *how* the HLRs are satisfied in terms of behaviour, checks, or constraints.

### General Rules

- IDs MUST NOT be reused for different meanings within the same contract.
- New requirements SHOULD use new IDs; do not renumber existing HLR/LLR entries.
- AI-generated contracts MUST follow this structure and naming scheme by default.

### Assumptions

All critical contracts must define an `assumptions` section.

Rules:

- Each assumption must have:
  - An `id` of the form `A-###` (for example: `A-001`, `A-002`, etc.).
  - A `text` field that describes an environmental or contextual condition that is **taken as given**, not something the command enforces at runtime.
- Assumptions are things like: “we are in the repo root,” “Spec Kit has already been initialized,” “templates live in a given directory,” and so on.
- IDs must not be reused with different meanings within the same contract.
- When adding new assumptions, use new IDs; do not renumber existing ones.

---

### Preconditions

All critical contracts must define a `preconditions` section.

Rules:

- Each precondition must have:
  - An `id` of the form `P-###` (for example: `P-001`, `P-002`, etc.).
  - A `text` field that describes a condition that the command **actively checks and enforces**, failing fast if it is not met.
- Preconditions should correspond to checks that result in usage or precondition errors, such as:
  - “FEATURE_ID is a non-empty string.”
  - “The command is run from the repository root.”
- IDs must remain stable over time; do not renumber once a precondition is in use.
- Preconditions should be enforced explicitly in code, not just implied.

---

### Test Cases (Verification)

All contracts must define test cases under the `verification` section.

Rules:

- Each test case must have:
  - An `id` of the form `TC-###` (for example: `TC-001`, `TC-002`, etc.).
  - A `requirement` field that references the `id` of the HLR or LLR it verifies (for example: `HLR-001`, `LLR-003`).
  - A `type` that classifies the test, such as `unit`, `integration`, or another clearly named category.
  - A `description` that clearly states the scenario and the expected outcome (for example: what inputs, what state, what exit code or behavior).
- Every high-level requirement should have at least one associated test case.
- Low-level requirements that encode important edge cases or error handling should also be covered by test cases.
- Test case IDs must be stable over time; do not renumber existing test cases.
- AI-generated contracts must follow the `TC-###` naming convention and always link tests back to specific requirements via the `requirement` field.

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
