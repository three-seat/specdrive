# ADR-003: Artifact Ownership and Traceability Model

Status: Accepted
Date: 2026-06-02

## Context

As SpecDrive evolved from a Spec Kit wrapper into an artifact-driven
engineering workflow system, several architectural principles emerged
organically through practice rather than explicit design decisions.

These principles governed decisions across F-006 through F-013 and are
referenced in the constitution (v1.0.0, Sections V through IX) but have
not been formally recorded as a cross-cutting architectural decision.

Specifically, the following questions required answers that were never
explicitly documented:

- Which artifacts are authoritative and which are derived?
- Where does SpecDrive's responsibility end and AI's responsibility begin?
- How do we ensure SpecDrive state is portable and reconstructable?
- How do we bound AI execution to prevent aggregate, untraceable changes?
- What is the canonical traceability chain for a feature?

Without a formal record, these principles risk being inconsistently
applied across future features or lost as the project scales.

## Decision

### 1. Artifact Authority

Artifacts are the durable, authoritative record of SpecDrive state.
Tools, agents, and AI systems are interchangeable. Artifacts are not.

Artifacts are classified as either canonical or derived:

**Canonical artifacts** (authoritative, versioned, committed):
- spec
- contract
- manifest
- review record

**Derived artifacts** (reproducible, produced from canonical artifacts):
- prompt
- output
- patch
- release summary
- audit report

Canonical artifacts are the source of truth. Derived artifacts can
always be reconstructed from canonical artifacts and must never be
treated as authoritative.

### 2. Lifecycle Ownership Boundary

SpecDrive owns:
- Lifecycle state and transitions
- Artifact validation
- Advancement gates
- Workflow control

AI owns:
- Artifact production only

AI never advances lifecycle state, controls workflow, or runs SpecDrive
commands. Autonomous orchestration, swarm behavior, and automatic state
advancement are explicitly rejected.

This boundary is non-negotiable and must be preserved across all future
features regardless of AI capability advances.

### 3. Repository Portability

SpecDrive state must remain repository-local and fully reconstructable
from version-controlled artifacts alone.

No external databases, cloud services, agent memory, or out-of-band
state is permitted. Cloning the repository must be sufficient to
reconstruct the full feature history and artifact record.

This principle extends ADR-002's feature-local artifact architecture
from a layout decision to a governing constraint.

### 4. Bounded AI Execution

One prompt → one output → one patch.

Complex features must be decomposed into bounded contracts, each
producing a single focused prompt, output, and patch. Aggregate prompts
producing aggregate patches are explicitly rejected because they:

- break the traceability chain
- make review impractical
- obscure the relationship between intent and implementation
- undermine the audit story

Feature decomposition (F-012) is the mechanism for managing complexity
within this constraint.

### 5. Traceability Chain

Every change must be traceable from intent to implementation via the
canonical chain:

```text
spec → contract → prompt → output → patch → review → release
```

Traceability is not a reporting feature added after the fact. It is a
governing constraint on how features are designed, implemented, and
verified. Gaps in the chain are architectural defects, not missing
reports.

## Relationship to Constitution

This ADR formalizes the decisions behind constitution v1.0.0
Sections V through IX. The constitution states the principles.
This ADR records the context, reasoning, and consequences.

## Supersedes / modifies

This ADR does not supersede ADR-001 or ADR-002. It establishes
cross-cutting principles that govern all future features and ADRs.

## Consequences

### Positive

- Architectural principles are formally recorded with context and
  reasoning, not just stated as rules.
- Future features have a clear reference for resolving ambiguity about
  artifact authority, AI boundaries, and traceability requirements.
- The canonical vs derived artifact distinction gives F-014 through
  F-022 a precise foundation to build against.
- The lifecycle ownership boundary protects the human-in-the-loop
  model as AI capabilities advance.
- Repository portability ensures SpecDrive remains auditable and
  reconstructable without external dependencies.
- The traceability chain provides the backbone for the DO-178C
  research direction noted in the README.

### Negative

- Bounded AI execution (one prompt → one output → one patch) increases
  the number of feature contracts required for complex features.
- Canonical artifact classification must be maintained as new artifact
  types are introduced — each new type requires an explicit decision
  about its classification.

### Neutral / accepted tradeoffs

- Derived artifacts are not committed by default but may be optionally
  retained for audit purposes. This is a feature-level decision, not a
  constitutional one.
- The lifecycle ownership boundary explicitly rejects future automation
  that would advance state without human approval, even if technically
  feasible and desirable for velocity.

## Affected features

- F-010: Lifecycle state enforcement — implements lifecycle ownership boundary
- F-012: Feature decomposition — implements bounded AI execution
- F-014: Prompt hashing and artifact lineage — implements traceability chain
- F-017: Artifact manifest — implements canonical artifact inventory
- F-019: Release and audit artifact generation — consumes manifest
- F-022: Review command — implements human approval gate at review boundary

## Follow-up work

- Update system overview to reference this ADR and add an
  Architectural Principles section.
- Each affected feature spec should reference ADR-003 in its context
  section.
- A future ADR may extend the canonical artifact classification as
  new artifact types are introduced (e.g., test execution records,
  verification evidence).