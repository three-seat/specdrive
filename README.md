# SpecDrive

SpecDrive is an **artifact-driven engineering workflow system** for building software with or without AI assistance.

It treats specifications, contracts, prompts, outputs, patches, and manifests as **first-class artifacts**, enforces lifecycle gates so changes remain reviewable, traceable, and auditable — and keeps humans in control of every consequential decision.

SpecDrive is **not** an agent framework.
Agents are optional workers. SpecDrive owns the lifecycle.

---

## What problem does this solve?

Modern development — especially solo or small-team work with interrupted time — suffers from:

- context loss between sessions
- oversized and untraceable diffs
- unclear or undocumented intent
- AI-generated changes that are hard to trust or review
- no canonical record of what was built, why, or how

SpecDrive addresses this by:

- forcing intent to be explicit before implementation begins (specs)
- hardening behavior as a machine-readable contract (contracts)
- keeping changes small, bounded, and intentional (patches)
- maintaining a canonical artifact record across the full lifecycle
- making AI optional, not required

---

## Core ideas

- **Lifecycle-first**: features move through explicit stages
- **Agent-optional**: works fully without AI
- **Reviewable diffs**: patches are small and intentional
- **Traceability**: every change maps back to intent
- **Human-in-the-loop by default**

---

## Current State

As of v0.3.x, SpecDrive focuses on:

- Feature-local specs and contracts
- Patch generation and emission
- Reviewable artifacts
- Spec-first workflows

Many traceability, lifecycle enforcement, and audit capabilities described in the roadmap are planned but not yet implemented.

---

## Design by contract (practical)

SpecDrive follows a practical "design by contract" approach:
- A feature spec captures intent.
- A contract (machine-readable) captures testable behavior and constraints.
- Implementation is reviewed and validated against that contract before merge.

This keeps changes small, explicit, and auditable — whether code is written by a human or generated with AI.

---

## Feature lifecycle (high level)

```
draft → contract → implement → patch → review → done
```

SpecDrive enforces what artifacts must exist at each stage. Features may also enter `blocked` or `deferred` states when dependencies are unresolved.

---

## SSDF alignment (high level)

SpecDrive is designed to align with Secure Software Development Framework (SSDF) practices by default:

- **Traceability:** intent → contract → implementation is captured in versioned artifacts.
- **Reviewability:** changes are structured as small, auditable patches.
- **Repeatability:** lifecycle stages and validation gates make the workflow consistent.
- **Human-in-the-loop:** AI is optional; approvals and merges remain explicit.
- **Change control:** implementation patches are reviewed against declared contracts before merge.

SpecDrive does not claim to make your project "SSDF compliant" by itself — compliance depends on your broader SDLC, policies, and operational controls.

---

## Research Direction

SpecDrive is exploring traceability, verification, review, and change-control workflows inspired by practices used in high-assurance software development.

One area of interest is understanding how SpecDrive's artifact model (specs, contracts, prompts, outputs, patches, reviews, and validation evidence) compares to traceability concepts found in standards such as DO-178C.

This is an exploratory effort only. SpecDrive is not a compliance tool, has not been qualified for use in regulated environments, and makes no claims of compliance with DO-178C or any other standard.

---

## Commands

### `specdrive new-feature`

Initialize a new feature and its working directory.

```bash
specdrive new-feature F-005
```

---

### `specdrive draft`

Generate a structured contract-authoring prompt for a feature.

```bash
specdrive draft F-005
```

Produces a prompt that instructs an AI to read the feature spec, 
constitution, ADRs, system overview, and contract templates, then 
draft or refine the feature contract. The prompt includes explicit 
mapping guidance and guardrails.

The resulting prompt is intended for use with an external AI chat tool. 
The contract is reviewed and committed before implementation begins.

---

### `specdrive implement`

Generate an implementation prompt for the feature.

```bash
specdrive implement F-005
```

Produces a structured prompt suitable for use with an external AI tool or manual implementation.

---

### `specdrive patch emit`

Generate a reviewable patch artifact for a feature.

```bash
specdrive patch emit F-006
```

Outputs:

```
docs/features/F-006/patches/F-006.patch
```

The patch is generated from the current git diff and can be independently reviewed and applied.

---

## Configuration

SpecDrive is configured via a file (introduced in `v0.1.0`).

Configuration schema is stabilizing pre-1.0. Refer to the repository for current configuration options.

---

## AI integration

SpecDrive is designed to work with AI systems but does not require them.

Current workflows are human-driven and artifact-driven. The spec and contract authoring phases work naturally with external AI chat tools — the contract acts as an execution plan that can be handed directly to an AI for implementation.

Future releases will add optional adapter interfaces for invoking external AI systems directly. See F-011 in the roadmap.

---

## What SpecDrive is *not*

- ❌ Not an agent orchestration framework
- ❌ Not a task tracker
- ❌ Not a replacement for Git, CI, or code review
- ❌ Not a chat-based coding tool

---

## Status

- **Pre-1.0**
- CLI and config format may change
- Lifecycle model is stabilizing
- Designed for real-world use with interrupted time and fatigue

---

## Roadmap

### Near-term
- F-009: Chat export/import workflow
  - Structured prompt export for stateless AI chat tools
  - Clipboard-based import of AI responses back into SpecDrive
  - Supports Claude, ChatGPT, Copilot Chat, and similar interfaces

- F-010: Lifecycle state enforcement
  - Formalize draft → contract → implement → patch → review → done
  - Include blocked and deferred states for dependency-driven workflows
  - Enforce required artifacts before advancing lifecycle state

- F-011: AI adapter interface
  - Define clean boundary for optional AI execution via stdin/stdout
  - Maintain human-in-the-loop and explicit execution model
  - No orchestration, swarm behavior, or autonomous flow control

- F-012: Feature decomposition
  - Support multiple sub-contracts and patch sets per feature
  - Parent spec remains singular; sub-contracts decompose implementation
  - Support dependencies between sub-contracts with lifecycle enforcement

- F-013: Spec and contract versioning
  - Support multiple versions of specs and contracts per feature
  - Versioning model informed by decomposition object model
  - Maintain canonical active version alongside version history

### Medium-term
- F-014: Prompt hashing and artifact lineage
  - Persist prompts as first-class artifacts on disk
  - Hash prompts and link to outputs and patches for full chain of custody
  - Build traceable chain: spec → contract → prompt → output → patch

- F-015: Repository baseline tracking
  - Record git commit, branch, and dirty-tree state for generated artifacts
  - Associate repository state with prompts, outputs, and patches
  - Repository state is part of lineage — not an afterthought

- F-016: Structured patch metadata
  - Attach originating feature, prompt hash, and timestamp to each patch
  - Record validation status at time of emission
  - Foundation for audit and review workflows

- F-017: Artifact manifest
  - Maintain canonical inventory of all feature artifacts with hashes and timestamps
  - Record relationships between specs, contracts, prompts, outputs, and patches
  - Single source of truth for release generation, audit reporting, and verification

- F-018: Validation gates
  - Patch, invariant, schema, and lifecycle validation layers
  - Fail fast on violations before patch is emitted
  - Write validation results to artifact manifest

- F-019: Contract and schema validation engine
  - Validate HLR/LLR structure and requirement references
  - Verify test case linkage and invariant formatting
  - Consume artifact manifest for results storage

- F-020: Release and audit artifact generation
  - Generate feature summaries, patch manifests, and validation summaries
  - Consume artifact manifest rather than walking filesystem
  - Produce reviewable release artifacts across all features

- F-021: Feature status command
  - Show all features and current lifecycle state at a glance
  - Surface missing artifacts, blocked transitions, and unresolved dependencies
  - Driven by artifact manifest for consistency

- F-022: Review command and review artifact stamping
  - Explicit specdrive review command for human approval gate
  - Stamp reviewed_by and reviewed_at into contract on execution
  - Required for critical features before lifecycle advances to done

### Long-term
- Test execution recording and verification evidence
  - Record test outcomes against contract-defined test cases
  - Bridge the gap between verification intent and verification evidence

- Verification command (specdrive verify)
  - Execute contract-defined test cases against implementation
  - Record pass/fail results as verification evidence
  - Feed outcomes into artifact manifest and release artifacts

- Traceability and verification workflows
  - Bidirectional traceability from requirements through tests
  - Surface uncovered requirements and untested claims automatically

- Requirement coverage analysis
  - Detect requirements with no verification evidence
  - Detect tests not linked to requirements
  - Generate coverage summaries for releases

- High-assurance software development support
  - Deeper alignment with safety-critical software lifecycle standards
  - Audit package generation suitable for regulated environment review

- Multi-agent coordination controls
  - Human approval gates at agent handoff boundaries
  - Explicit execution boundaries between planning and implementation agents

---

## Who is this for?

- Solo developers
- Small teams
- People using AI but wanting control
- Anyone who wants specs and intent to survive context switches
- Engineers working in safety-critical or high-assurance contexts

---

## License

MIT