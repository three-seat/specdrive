---
id: F-010
title: Lifecycle State Enforcement
type: feature
system: specdrive
status: draft
area: cli
owners:
  - three-seat
created_at: 2026-07-07
contract: docs/features/F-010-lifecycle/contract.yaml
adrs:
  - ADR-0003
---

# Summary

Formalize and enforce the SpecDrive feature lifecycle by introducing a
per-feature `state.yaml` sidecar that records lifecycle events as an
append-only log. Lifecycle state is partially inferred from artifact
presence (draft, contract, patch) and partially gated by explicit human
commands (review, done, blocked, deferred). This closes the gap between
SpecDrive's constitutional lifecycle model and its actual enforcement,
provides the audit trail foundation for future traceability and
verification features, and separates lifecycle records from artifact
content in alignment with DO-178C principles.

# Context

- SpecDrive's constitution defines a lifecycle:
  `draft → contract → implement → patch → review → done`
  but nothing currently enforces it. Features can exist in any state
  without the tool knowing or caring.
- The `status` field in spec and contract front matter is decorative —
  no validation, no transitions, no history.
- `reviewed_by` and `reviewed_at` fields in the contract template
  conflate artifact content with lifecycle metadata, violating the
  separation between what a feature does and what happened to it.
- DO-178C separates software artifacts from their lifecycle records.
  A per-feature `state.yaml` sidecar with an append-only event log
  provides a durable, version-controlled lifecycle record that is
  distinct from the feature's canonical artifacts.
- The hybrid model — automatic inference for early states, explicit
  human gates for review and done — balances auditability with
  workflow practicality.
- Implement state is not supported in V1. The current
  `specdrive implement` command prints a prompt to stdout and saves
  no artifact. Implement state inference requires a saved prompt
  artifact, which is F-014 territory. Features transition directly
  from contract to patch in V1 inference.
- Inferred states are never persisted. state.yaml contains only
  explicit human command events. Inference is stateless and
  read-only — the same artifacts always produce the same inferred
  state without writing anything.
- Relevant constitutional principles: Constitution Sections IV
  (Safety and Reversibility), VI (SpecDrive Owns the Lifecycle),
  IX (Traceability as a First-Class Goal). See ADR-0003 for artifact
  ownership and lifecycle boundary decisions.

# Behavior

## Lifecycle states

| State | Description | How entered |
|-------|-------------|-------------|
| `draft` | Feature directory and spec exist | inferred |
| `contract` | Contract exists and is non-empty | inferred |
| `patch` | At least one patch file exists | inferred |
| `review` | Human review in progress | `specdrive review` |
| `done` | Reviewed and marked complete | `specdrive done` |
| `blocked` | Waiting on dependency or blocker | `specdrive block` (overlay) |
| `deferred` | Intentionally postponed | `specdrive defer` (overlay) |

Note: `implement` state is reserved for F-014 when prompt saving is
implemented. It does not appear in V1 inference or commands.

## State transition table

| From | To | Via |
|------|----|-----|
| `draft` | `contract` | inferred |
| `contract` | `patch` | inferred |
| `patch` | `review` | `specdrive review` |
| `review` | `done` | `specdrive done` |
| `any base state` | `blocked` | `specdrive block` |
| `any base state` | `deferred` | `specdrive defer` |
| `blocked` | previous base state | `specdrive unblock` |
| `deferred` | previous base state | `specdrive resume` |

## State precedence rule

Lifecycle state has two independent layers:

**Base state** — the feature's progress through the lifecycle:
```
draft < contract < patch < review < done
```

**Overlay state** — an orthogonal condition that suspends progress:
```
blocked | deferred
```

The displayed state is computed as:
```
base_state      = max(last_recorded_base_state, current_inferred_state)
overlay_state   = last unresolved blocked or deferred event, if any
displayed_state = overlay_state ?? base_state
```

Unblock and resume resolve the overlay, revealing the base state
underneath. The base state is unchanged by block or defer operations.

Example:
- Feature has patch artifact — base state inferred as patch
- Developer runs `specdrive block` — overlay is blocked, base remains patch
- Developer runs `specdrive unblock` — overlay resolved, displayed state
  returns to patch

## User flow — Status

Command:
```
specdrive status <FEATURE_ID>
specdrive status --all
```

Inputs:
- FEATURE_ID — existing feature, or --all for all features

Outputs:
- Displayed state (overlay if present, otherwise base)
- Base state if overlay is active
- Whether base state is inferred or explicitly recorded
- Last explicit event timestamp and actor if available
- Blocked or deferred reason if applicable

Status is strictly read-only. It never writes to state.yaml under
any circumstances. Inference is stateless — no inferred state is
ever persisted.

Example single feature:
```
$ specdrive status F-009

Feature:  F-009 (Chat Export/Import Workflow)
Status:   patch
Source:   inferred
Since:    --
Actor:    --
```

Example feature with explicit events:
```
$ specdrive status F-009

Feature:  F-009 (Chat Export/Import Workflow)
Status:   review
Source:   recorded
Since:    2026-06-08T10:00:00Z
Actor:    three-seat
```

Example blocked feature:
```
$ specdrive status F-010

Feature:  F-010 (Lifecycle State Enforcement)
Status:   blocked
Base:     contract
Source:   recorded
Since:    2026-06-07T14:00:00Z
Actor:    three-seat
Reason:   Waiting on F-012 feature decomposition
```

Example all features:
```
$ specdrive status --all

F-006  done      recorded  2026-05-28  three-seat
F-007  done      recorded  2026-05-28  three-seat
F-008  done      recorded  2026-05-28  three-seat
F-009  patch     inferred  --          --
F-010  blocked   recorded  2026-06-07  three-seat
F-011  draft     inferred  --          --
```

## User flow — Review

Command:
```
specdrive review <FEATURE_ID>
```

Inputs:
- FEATURE_ID — feature with base state of patch

Outputs:
- Stamps review event to state.yaml
- Advances base state to review

Error cases:
- Feature base state not patch — reject with clear message
- No patch artifact exists — reject with clear message
- Feature has unresolved overlay — reject with clear message
  instructing user to unblock or resume first

Example:
```
$ specdrive review F-009

Feature:  F-009 (Chat Export/Import Workflow)
Base:     patch

Review recorded:
  by:  three-seat
  at:  2026-06-08T10:00:00Z

State advanced to: review
```

## User flow — Done

Command:
```
specdrive done <FEATURE_ID>
```

Inputs:
- FEATURE_ID — feature with base state of review

Outputs:
- Stamps done event to state.yaml
- Advances base state to done

Preconditions:
- Feature base state must be review
- Review event must exist in state.yaml
- At least one patch artifact must exist
- No unresolved overlay

Error cases:
- Feature base state not review — reject with clear message
- No review event in state.yaml — reject with clear message
- No patch artifact exists — reject with clear message
- Feature has unresolved overlay — reject with clear message

Example:
```
$ specdrive done F-009

Feature:  F-009 (Chat Export/Import Workflow)
Base:     review

Done recorded:
  by:  three-seat
  at:  2026-06-08T11:00:00Z

State advanced to: done
```

## User flow — Block and Defer

Commands:
```
specdrive block <FEATURE_ID> --reason "<reason>"
specdrive defer <FEATURE_ID> --reason "<reason>"
```

Inputs:
- FEATURE_ID — any feature with base state not done
- --reason — required string, recorded in state.yaml

Outputs:
- Stamps blocked or deferred event to state.yaml
- Records current base state as previous_status for unblock/resume
- Displays what was recorded

Error cases:
- --reason not provided — reject with clear message
- Feature base state is done — reject with clear message
- Feature already has unresolved overlay — reject with clear message

Example:
```
$ specdrive block F-010 --reason "Waiting on F-012 feature decomposition"

Feature:  F-010 (Lifecycle State Enforcement)
Base:     contract

Blocked:
  by:      three-seat
  at:      2026-06-07T14:00:00Z
  reason:  Waiting on F-012 feature decomposition
  returns: contract

State overlaid as: blocked
```

## User flow — Unblock and Resume

Commands:
```
specdrive unblock <FEATURE_ID>
specdrive resume <FEATURE_ID>
```

Inputs:
- FEATURE_ID — feature with active blocked or deferred overlay

Outputs:
- Stamps unblocked or resumed event to state.yaml
- Resolves overlay returning feature to previous base state
- Displays what was recorded

Error cases:
- Feature not blocked (unblock) — reject with clear message
- Feature not deferred (resume) — reject with clear message
- No previous base state recorded — reject with clear message

Example:
```
$ specdrive unblock F-010

Feature:  F-010 (Lifecycle State Enforcement)
Was:      blocked (Waiting on F-012 feature decomposition)

Unblocked:
  by:  three-seat
  at:  2026-06-07T16:00:00Z

Overlay resolved. Base state returned to: contract
```

## Detailed behavior — State inference

Inference rules:

- `draft` — feature directory exists and spec.md is present
- `contract` — contract.yaml exists and is non-empty
- `patch` — at least one file exists under patches/

Inference uses artifact presence only — no content validation.
Validation belongs to F-013 and F-015, not to lifecycle inference.

Blocked and deferred are never inferred — explicit commands only.
Review and done are never inferred — explicit commands only.
Implement state is not supported in V1 — reserved for F-014.

Inference is deterministic and stateless. The same artifacts always
produce the same inferred state. No inferred state is ever written
to state.yaml. Status is strictly read-only.

Automatic inference applies to F-009 handoff: when
`specdrive chat import draft` replaces contract.yaml, the next
`specdrive status` call automatically reflects contract state without
manual advancement required. No event is written to state.yaml by
this transition.

## Detailed behavior — state.yaml sidecar

Location: `docs/features/<FEATURE_ID>/state.yaml`

state.yaml contains only explicit human command events. Inferred
states are never written. The log is a record of what a human
deliberately did, not of what SpecDrive observed.

Schema:
```yaml
feature_id: F-010
events:
  - status: blocked
    at: 2026-06-07T14:00:00Z
    by: three-seat
    reason: "Waiting on F-012 feature decomposition"
    previous_status: contract
  - status: contract
    at: 2026-06-07T16:00:00Z
    by: three-seat
    via: unblock
  - status: review
    at: 2026-06-08T10:00:00Z
    by: three-seat
  - status: done
    at: 2026-06-08T11:00:00Z
    by: three-seat
```

Rules:
- Events are append-only — no existing event is ever modified
  or deleted by any SpecDrive command or process
- Append-only is a constitutional-level invariant — correction
  of accidental state changes is achieved by appending a new
  event, never by editing history
- Only explicit command events are written — inferred states
  are never persisted
- Base state is derived from the last non-overlay event combined
  with the precedence rule
- Overlay state is the last unresolved blocked or deferred event
- Blocked and deferred events record `previous_status`
- Unblock and resume events record `via: unblock` or `via: resume`
- Actor is informational only — not an authentication mechanism
  and carries no security meaning
- Merge conflicts are the developer's responsibility — SpecDrive
  does not attempt to auto-resolve event log conflicts
- state.yaml is the authoritative source for review records —
  contract review fields are ignored by tooling going forward

## Detailed behavior — Actor identity

Actor is resolved in priority order:
1. git config user.name
2. git config user.email
3. System username
4. "unknown" if none available

Actor is informational only. It is not an authentication mechanism,
carries no security meaning, and cannot be relied upon as proof of
identity. It does not constitute independent verification in the
DO-178C sense.

## Detailed behavior — Contract template update

The `reviews.status` block is removed from the critical contract
template. The checklist remains as review policy. The record of
who reviewed it and when moves to state.yaml.

Before:
```yaml
reviews:
  required: true
  checklist:
    - "..."
  status:
    reviewed_by: ""
    reviewed_at: ""
```

After:
```yaml
reviews:
  required: true
  checklist:
    - "..."
  # Review records are stored in state.yaml not in this contract.
  # See F-010. state.yaml is authoritative for all review records.
```

state.yaml is authoritative for review records. If both a contract
reviews.status block and a state.yaml review event exist for a
feature, state.yaml wins. Contract review fields are ignored by
all SpecDrive tooling going forward.

## Detailed behavior — Existing features

Features F-001 through F-008 have no state.yaml. When F-010 ships:
- `specdrive status <FEATURE_ID>` infers state from artifact presence
- No state.yaml is created automatically by status
- state.yaml is created on first explicit command
- Inference without a sidecar is sufficient for pre-F-010 features
- No migration command is provided in V1

## Detailed behavior — Future decomposition compatibility

The state model must support future parent/sub-contract lifecycle
tracking introduced by F-012. state.yaml schema and inference rules
must not assume a single contract per feature. Specific implementation
is deferred to F-012.

# Non-Functional Requirements

- Performance: status inference must complete in under one second
  per feature, under three seconds for --all across a typical project
- Portability: state.yaml is plain YAML committed to git — no
  external dependencies
- Auditability: append-only event log preserves full transition
  history — no event is ever modified or deleted. Only explicit
  human command events are recorded.
- Merge safety: append-only structure makes merge conflicts ordering
  questions not content questions — no special merge driver required
- DO-178C alignment: lifecycle records are separate from artifact
  content, consistent with DO-178C's separation of software artifacts
  from lifecycle data. Actor identity is informational only and does
  not constitute independent verification in the DO-178C sense.
  V1 done state represents human review acknowledgment not
  independent verification.

# Acceptance Criteria

- [ ] AC-1: `specdrive status <FEATURE_ID>` displays displayed state
      (overlay if active, otherwise base), source (inferred or
      recorded), last explicit event metadata, and blocked/deferred
      reason if applicable
- [ ] AC-2: `specdrive status --all` displays state for all features
      in the project
- [ ] AC-3: Status is strictly read-only — it never writes to
      state.yaml under any circumstances
- [ ] AC-4: `specdrive review <FEATURE_ID>` stamps a review event
      to state.yaml — only valid when base state is patch, patch
      artifact exists, and no unresolved overlay
- [ ] AC-5: `specdrive done <FEATURE_ID>` stamps a done event —
      only valid when base state is review, review event exists,
      patch artifact exists, and no unresolved overlay
- [ ] AC-6: `specdrive block <FEATURE_ID> --reason` stamps a blocked
      event with reason and previous_status — rejected if base state
      is done or overlay already active
- [ ] AC-7: `specdrive defer <FEATURE_ID> --reason` stamps a deferred
      event with reason and previous_status — rejected if base state
      is done or overlay already active
- [ ] AC-8: `specdrive unblock <FEATURE_ID>` stamps an unblocked
      event and resolves overlay returning to previous base state
- [ ] AC-9: `specdrive resume <FEATURE_ID>` stamps a resumed event
      and resolves overlay returning to previous base state
- [ ] AC-10: --reason is required for block and defer — reject
      without it with a clear message
- [ ] AC-11: State inference correctly identifies draft, contract,
      and patch states from artifact presence only — no content
      validation at inference time
- [ ] AC-12: Base state precedence rule enforced:
      base = max(last_recorded_base, inferred)
- [ ] AC-13: Overlay state computed independently of base state —
      displayed state is overlay if active, otherwise base
- [ ] AC-14: Blocked and deferred are never inferred — explicit
      commands only
- [ ] AC-15: Review and done are never inferred — explicit commands
      only
- [ ] AC-16: Implement state does not appear in V1 — reserved for
      F-014
- [ ] AC-17: Events in state.yaml are append-only — no existing
      event is modified or deleted by any command
- [ ] AC-18: Inferred states are never written to state.yaml —
      only explicit human command events are persisted
- [ ] AC-19: Blocked and deferred events record previous_status
      for use by unblock and resume
- [ ] AC-20: state.yaml is created on first explicit command if
      not already present
- [ ] AC-21: Actor is resolved from git config with system username
      fallback — documented as informational only with no security
      meaning
- [ ] AC-22: Critical contract template no longer contains
      reviews.status block — comment references state.yaml
- [ ] AC-23: Existing features without state.yaml have state
      inferred from artifact presence without error
- [ ] AC-24: state.yaml is authoritative for review records —
      contract reviews.status fields are ignored by tooling
- [ ] AC-25: State model leaves room for future parent/sub-contract
      lifecycle tracking per F-012
- [ ] AC-26: Unblock and resume record via field in emitted event
- [ ] AC-27: Review and done commands reject features with active
      overlay — instruct user to unblock or resume first

# Implementation Notes

- Expected files/modules to touch:
  - `src/lifecycle/mod.rs` — lifecycle module
  - `src/lifecycle/state.rs` — state.yaml read/write and event log
  - `src/lifecycle/infer.rs` — inference rules
  - `src/lifecycle/commands.rs` — review, done, block, defer,
    unblock, resume
  - `src/status.rs` — status command implementation
  - `src/cli.rs` — add status, review, done, block, defer,
    unblock, resume subcommands
  - `docs/templates/feature.contract.critical.yaml` — remove
    reviews.status block

- State model implementation notes:
  - Base state and overlay state are computed separately
  - Base state: max(last non-overlay event status, inferred)
  - Overlay state: last unresolved blocked or deferred event
  - Displayed state: overlay if present, otherwise base
  - Implement state explicitly excluded from V1 inference
  - Inference reads artifact presence only — no content validation
  - Status never writes state.yaml under any circumstances

- Event log implementation notes:
  - Append-only — read then append, never overwrite
  - Only explicit command events are written — never inferred states
  - Timestamp in UTC ISO 8601 format from system clock
  - Actor resolved from git config user.name, then user.email,
    then system username, then "unknown"
  - state.yaml created with first explicit event if not present
  - blocked and deferred events must record previous_status
  - unblock and resume events must record via field

- Dependencies:
  - No new crates
  - `serde_yaml` for state.yaml read/write (already present)
  - `chrono` for timestamps (already present)

# Follow-up Work

- ADR-0004: Lifecycle State Model — after F-010 ships, capture the
  architectural decisions established here as a permanent ADR:
  inferred vs recorded distinction, append-only event log,
  base/overlay state model, state.yaml authority over contract
  review fields, and actor identity as informational only. These
  are platform-level decisions that will govern future features.

# Open Questions

- Q5 (open): Multi-contract features — F-012 feature decomposition
  is not yet shipped. When it ships, does lifecycle state live at
  the parent feature level, sub-contract level, or both? Deferred
  to F-012 spec but state model must not preclude it.

- Q1 (closed): Inference uses artifact presence only — no content
  validation at inference time. Validation belongs to F-013 and
  F-015.

- Q2 (closed): Status is strictly read-only — never writes
  state.yaml under any circumstances. Inferred states are never
  persisted.

- Q3 (closed): No migration command in V1. Inference without a
  sidecar is sufficient for pre-F-010 features.

- Q4 (closed): Done requires review event in state.yaml and at
  least one patch artifact. No additional validation in V1.

- Q6 (closed): Implement state is not supported in V1. The current
  implement command prints to stdout and saves no artifact.
  Implement state is reserved for F-014 when prompt saving exists.

- Q7 (closed): Blocked means waiting on dependency or external
  blocker. Deferred means intentionally postponed with no current
  blocker. Both require --reason. Both record previous_status.
  Unblock and resume resolve the overlay returning to previous
  base state.

- Q8 (closed): Automatic inference. F-009 import triggering
  contract state is detected automatically on next status check.
  No manual advancement required. No event written to state.yaml.