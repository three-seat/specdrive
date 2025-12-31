## Goal

You are helping implement feature <FEATURE_ID> in the `specdrive` project.

1. Read the following local files in full:
   - .specify/specs/<FEATURE_ID>.spec.md
   - docs/features/<FEATURE_ID>/contract.yaml
   - .specify/memory/constitution.md (if present)
   - docs/adrs/ (scan for relevant ADRs, if present)
   - docs/system-overview.md (if present)

2. Treat the spec + contract as the source of truth.
3. Do not modify the spec, contract, constitution, or ADRs unless explicitly instructed.

## Constraints

- Language: Rust.
- No new dependencies beyond what the contract allows (at most `serde_yaml` for this feature).
- Follow git safety and filesystem rules in the contract.
