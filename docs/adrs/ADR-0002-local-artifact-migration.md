# ADR-002: Feature-local artifact architecture

Status: Accepted  
Date: 2026-05-27

## Context

SpecDrive began as a lightweight wrapper around Spec Kit conventions. In ADR-001, SpecDrive assumed that:

- a git repository already existed;
- Spec Kit had initialized `.specify/`;
- feature specs lived under `.specify/specs/`;
- SpecDrive owned feature scaffolding and AI prompt structure around that layout.

That split was useful for bootstrap, but it creates problems as SpecDrive evolves:

- Feature artifacts are spread across multiple roots:
  - specs in `.specify/specs/`;
  - contracts in `docs/features/<FEATURE_ID>/`;
  - patches in `docs/features/<FEATURE_ID>/patches/`.
- SpecDrive is becoming its own artifact-driven lifecycle tool rather than only a Spec Kit wrapper.
- Future features will need clean artifact lineage:
  - spec;
  - contract;
  - prompt;
  - AI output;
  - patch;
  - validation result.
- Keeping specs outside `docs/features/<FEATURE_ID>/` weakens traceability and makes review harder.
- Requiring `.specify/` also makes Spec Kit a runtime dependency even when SpecDrive can operate independently.

The project is still pre-1.0, so this is the right time to make a breaking layout correction before users or future features depend on the old structure.

## Decision

SpecDrive will adopt a feature-local artifact architecture.

The canonical feature directory is:

```text
docs/features/<FEATURE_ID>/
```

Each feature directory owns its core artifacts:

```text
docs/features/<FEATURE_ID>/
  spec.md
  contract.yaml
  patches/
```

SpecDrive will no longer require Spec Kit or `.specify/` for normal operation.

SpecDrive-owned templates move to:

```text
docs/templates/
```

SpecDrive-owned constitution moves to:

```text
docs/constitution.md
```

SpecDrive will remove `.claude/` from the repository.

F-007 is a repository layout migration, not a runtime migration system. The implementation will move files directly in the repository and update the codebase to use the new canonical paths.

SpecDrive will not add:

- a migration subcommand;
- a compatibility layer for `.specify/specs/`;
- fallback spec resolution;
- long-term dual-layout support.

This is an intentional breaking change for `v0.3.0`.

## Supersedes / modifies

This ADR supersedes the parts of ADR-001 that required Spec Kit initialization and `.specify/` as part of normal SpecDrive operation.

ADR-001 remains valid for these principles:

- SpecDrive is local-first.
- SpecDrive should be additive and safe where practical.
- SpecDrive should avoid network calls unless explicitly introduced by a later feature and ADR.
- AI API calls are not required for the core workflow.

## Consequences

### Positive

- Feature artifacts become easier to inspect, review, archive, and reason about.
- Specs, contracts, and patches are co-located under one feature directory.
- Future prompt/output/patch lineage can be added naturally under the same feature directory.
- SpecDrive becomes less coupled to Spec Kit and can operate as an independent workflow tool.
- The repository layout becomes more audit-friendly and easier to explain.
- The architecture better supports SSDF-style traceability and future high-assurance workflow features.

### Negative

- This breaks the previous `.specify/specs/` layout.
- Existing internal feature specs must be moved.
- Code, tests, documentation, and examples must be updated together.
- Users of `v0.1.x` or `v0.2.x` layouts will need to migrate manually if they adopt `v0.3.0`.
- ADR-001 and the system overview must be updated to reflect the new ownership boundary.

### Neutral / accepted tradeoffs

- No compatibility layer will be added because the project is still pre-1.0.
- No migration command will be added because the immediate need is to clean up SpecDrive’s own internal layout.
- Spec Kit may still be used externally by users, but SpecDrive no longer depends on it structurally.

## Implementation notes

F-007 should update:

- feature path resolution;
- `bootstrap`;
- `new-feature`;
- `draft`;
- `implement`;
- `patch emit`;
- README;
- system overview;
- constitution references;
- tests.

`new-feature` should create:

```text
docs/features/<FEATURE_ID>/spec.md
docs/features/<FEATURE_ID>/contract.yaml
docs/features/<FEATURE_ID>/patches/
```

`bootstrap` should create only SpecDrive-owned shared directories and templates. It should not require `.specify/` and should not create feature-local `prompts/` or `outputs/` directories.

Future prompt/output lineage can introduce:

```text
docs/features/<FEATURE_ID>/prompts/
docs/features/<FEATURE_ID>/outputs/
```

but those are out of scope for F-007.

## Follow-up work

- Update ADR-001 or mark the affected assumptions as superseded by ADR-002.
- Update the system overview to describe the feature-local architecture.
- Update README examples.
- Update F-007 spec and contract to reference this ADR.
- Release the breaking layout change as `v0.3.0`.

