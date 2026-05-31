# SpecDrive

SpecDrive is a **spec-driven development workflow** for building software with or without AI assistance.

It treats specifications, contracts, and patches as **first-class artifacts**, and enforces lightweight gates so changes remain reviewable, traceable, and safe—especially when working in small, interrupted time blocks.

SpecDrive is **not** an agent framework.  
Agents are optional workers. SpecDrive owns the lifecycle.

---

## What problem does this solve?

Modern development—especially solo or small-team work—suffers from:

- context loss
- oversized diffs
- unclear intent
- AI-generated changes that are hard to trust or review

SpecDrive addresses this by:

- forcing intent to be explicit (specs)
- hardening behavior before code (contracts)
- keeping changes small (patches)
- making AI optional, not required

---

## Core ideas

- **Lifecycle-first**: features move through explicit stages
- **Agent-optional**: works fully without AI
- **Reviewable diffs**: patches are small and intentional
- **Traceability**: every change maps back to intent
- **Human-in-the-loop by default**

---

## Design by contract (practical)

SpecDrive follows a practical “design by contract” approach:
- A feature spec captures intent.
- A contract (machine-readable) captures testable behavior and constraints.
- Implementation is reviewed and validated against that contract before merge.

This keeps changes small, explicit, and auditable—whether code is written by a human or generated with AI.

---

## Feature lifecycle (high level)

```
  draft → contract → implement → patch → review → done
```

SpecDrive enforces what artifacts must exist at each stage.

---

## SSDF alignment (high level)

SpecDrive is designed to *align with* Secure Software Development Framework (SSDF) practices by default:

- **Traceability:** intent → contract → implementation is captured in versioned artifacts.
- **Reviewability:** changes are structured as small, auditable patches.
- **Repeatability:** lifecycle stages and validation gates make the workflow consistent.
- **Human-in-the-loop:** AI is optional; approvals and merges remain explicit.
- Change control: implementation patches are reviewed against declared contracts before merge

SpecDrive does not claim to make your project “SSDF compliant” by itself—compliance depends on your broader SDLC, policies, and operational controls.


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

Work on feature specifications and contracts.

- Agent-backed mode:

```bash
specdrive draft F-005 --agent ba
```

- Agent-less (manual) mode:

```bash
specdrive draft F-005 --no-agent
```

SpecDrive validates outputs before allowing the feature to advance.

---

### `specdrive implement`

Generate or validate implementation patches.

- Agent-backed:

```bash
specdrive implement F-005 --agent impl
```

- Manual:

```bash
specdrive implement F-005 --no-agent
```

Implementation is always checked against the contract.

---

## Configuration

SpecDrive is configured via a file (introduced in `v0.1.0`).

Example (simplified):

```yaml
paths:
  specs: specs/
  contracts: contracts/
  patches: patches/
  adrs: adrs/

defaults:
  draft:
    agent: ba
  implement:
    agent: impl
```

Exact schema may evolve pre-1.0.

---

## AI agents (optional)

SpecDrive can invoke AI agents for:

- spec drafting
- contract generation
- implementation patches

However:

- agents never control flow
- agents never advance state
- agents never run SpecDrive commands

SpecDrive calls agents, validates outputs, and decides what happens next.

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
Near-term 
-  F-009: Chat export/import workflow
-  F-010: Lifecycle state enforcement
-  F-011: AI adapter interface

Medium-term
-   Prompt hashing and artifact lineage
-   Multi-prompt feature implementation
-   Structured patch metadata
-   Contract/schema validation
Long-term
-   Audit-friendly release artifacts
-   Traceability and verification workflows
-   High-assurance software development support
---

## Who is this for?

- Solo developers
- Small teams
- People using AI but wanting control
- Anyone who wants specs and intent to survive context switches

---

## License

MIT
