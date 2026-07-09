# ADR-0004: Lifecycle State Model

Status: Accepted
Date: 2026-06-08

## Context

F-010 introduced lifecycle state enforcement to SpecDrive. During its
design, several architectural decisions emerged that govern not just
F-010 but the behavior of all future lifecycle-aware features (F-012
decomposition, F-014 artifact lineage, F-017 manifest, F-019 release
artifacts, F-021 review command).

These decisions were made deliberately and after multiple design
iterations, but they were recorded only in the F-010 spec. Because
they establish platform-level behavior rather than single-feature
behavior, they warrant a standalone ADR so future features have a
stable reference and the reasoning is not lost.

The specific questions this ADR settles:

- How is lifecycle state determined — from artifacts, from records,
  or both?
- How are lifecycle records stored and why?
- How do suspending conditions (blocked, deferred) relate to lifecycle
  progress?
- What authority do lifecycle records have relative to artifact content?
- What does actor attribution mean and not mean?
- Why is one lifecycle state (implement) excluded from the initial
  implementation?

## Decision

### 1. Inferred vs Recorded State

Lifecycle state is determined from two sources:

**Inferred state** — derived from artifact presence, computed on demand,
never persisted. Draft, contract, and patch states are inferred from
the existence of spec.md, a non-empty contract.yaml, and patch files
respectively.

**Recorded state** — written explicitly to state.yaml by human commands.
Review, done, blocked, and deferred are recorded states.

Inference is stateless and deterministic — the same artifacts always
produce the same inferred state. Inferred states are never written to
state.yaml. Only explicit human command events are recorded.

The `status` command is strictly read-only and never writes state.

### 2. Append-Only Event Log

Lifecycle records are stored in a per-feature `state.yaml` sidecar as
an append-only event log.

- No existing event is ever modified or deleted by any command
- Accidental state changes are corrected by appending new events,
  never by editing history
- Current state is derived from the event log, not stored as a mutable
  field
- Append-only structure makes merge conflicts ordering questions
  rather than content questions

This is a constitutional-level invariant. It provides a complete,
tamper-evident lifecycle history and aligns with DO-178C's treatment
of lifecycle data as a durable record.

### 3. Base/Overlay State Model

Lifecycle state has two independent layers:

**Base state** — progress through the lifecycle:
```
draft < contract < patch < review < done
```

**Overlay state** — an orthogonal condition that suspends progress
without altering base state:
```
blocked | deferred
```

Displayed state is computed as:
```
base_state      = max(last_recorded_base_state, current_inferred_state)
overlay_state   = last unresolved blocked or deferred event, if any
displayed_state = overlay_state ?? base_state
```

Unblock and resume resolve the overlay, revealing the unchanged base
state underneath. This model is extensible — future overlays (e.g.
verification-failed, release-hold) fit naturally without changing
the base state ordering.

### 4. state.yaml Authority

state.yaml is the authoritative source for lifecycle records,
including review records.

The contract's former `reviews.status` block (reviewed_by, reviewed_at)
is removed from templates going forward. Where both a contract
reviews.status block and a state.yaml review event exist for a feature,
state.yaml wins. Contract review fields are ignored by all SpecDrive
tooling.

This enforces the separation established in ADR-0003 between canonical
artifact content and lifecycle records. The contract describes what a
feature does; state.yaml records what happened to it.

### 5. Actor Attribution Is Informational Only

Each recorded event captures an actor, resolved from git config
user.name, then user.email, then system username, then "unknown".

Actor attribution is informational only. It is not an authentication
mechanism, carries no security meaning, and does not constitute
independent verification in the DO-178C sense. It answers "who ran
this command" for audit context, not "who is cryptographically proven
to have approved this."

### 6. Implement State Excluded From V1

The constitution defines a six-state lifecycle including `implement`.
F-010 V1 enforces a reduced five-state lifecycle:

```
draft → contract → patch → review → done
```

Implement state is excluded because the current `specdrive implement`
command prints a prompt to stdout and saves no artifact. Without a
persisted artifact, implement state cannot be reliably inferred.

Implement state is restored when F-014 introduces prompt persistence.
This is a deliberate scoping decision to avoid inferring a state from
absent evidence.

## Relationship to Constitution and Prior ADRs

- Formalizes decisions behind Constitution Sections VI (SpecDrive Owns
  the Lifecycle) and IX (Traceability as a First-Class Goal).
- Extends ADR-0003's canonical vs derived artifact distinction —
  state.yaml is a lifecycle record distinct from canonical artifacts.
- Does not supersede ADR-0001, ADR-0002, or ADR-0003.

## Consequences

### Positive

- Lifecycle behavior is formally recorded with reasoning, giving future
  features a stable reference.
- The inferred vs recorded distinction keeps status queries cheap and
  stateless while preserving an audit trail for deliberate actions.
- The append-only event log provides tamper-evident lifecycle history
  suitable for the DO-178C research direction.
- The base/overlay model is extensible to future suspending conditions
  without disrupting the base lifecycle.
- state.yaml authority resolves ambiguity about where review records
  live and prevents drift between contract and lifecycle record.

### Negative

- Two sources of truth for state (inference and records) require a
  precedence rule that must be applied consistently.
- The append-only log grows over the life of a feature — for
  long-lived features with many block/unblock cycles the log can
  become verbose.
- Excluding implement state from V1 means the enforced lifecycle
  temporarily diverges from the constitution's stated lifecycle until
  F-014 ships.

### Neutral / accepted tradeoffs

- Merge conflicts on state.yaml are the developer's responsibility —
  SpecDrive does not attempt auto-resolution. Accepted as a team
  workflow concern outside SpecDrive's scope.
- Actor attribution provides audit context but not cryptographic
  proof — accepted as sufficient for current assurance goals.

## Affected features

- F-010: Lifecycle state enforcement — implements this model
- F-012: Feature decomposition — must extend the model to parent/
  sub-contract lifecycle tracking
- F-014: Artifact lineage — restores implement state via prompt
  persistence
- F-017: Artifact manifest — consumes lifecycle records
- F-019: Release and audit artifact generation — consumes lifecycle
  history
- F-021: Review command — builds on the review state gate

## Follow-up work

- When F-012 ships, this ADR may need a companion or amendment
  addressing how base/overlay state applies to features with multiple
  sub-contracts.
- When F-014 ships and implement state is restored, update the enforced
  lifecycle to match the constitution's full six-state model.