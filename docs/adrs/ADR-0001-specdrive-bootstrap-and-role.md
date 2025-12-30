# ADR-001: Specdrive bootstrap assumptions and responsibilities

Status: Accepted  
Date: 2025-12-29

## Context

- The goal is to build a reusable AI-helper CLI (specdrive) that can be dropped into **any project**, regardless of language or stack.
- Specdrive’s job is to standardize spec/contract structure and AI prompts, not to own repo initialization or external tooling lifecycles.

## Decision

- Specdrive **assumes**:
  - A git repository already exists in the project root (`.git/` present).
  - Spec Kit has been initialized via `specify init`, creating `.specify/`.

- `specdrive bootstrap` is **additive-only**:
  - It creates missing directories and template files.
  - It never overwrites existing files.
  - It never deletes files.
  - It makes no network calls.

- Specdrive ships with **embedded templates** for:
  - `.specify/templates/feature.spec.md`
  - `docs/templates/feature.contract.minimal.yaml`
  - `docs/templates/feature.contract.critical.yaml`

- For v0.1, specdrive **does not call AI APIs**:
  - AI is invoked manually by the user using prompts generated from specs + contracts.
  - Future versions may add API calls without changing the basic bootstrap/structure concepts.

## Consequences

- Specdrive is easy to drop into any existing repo with git + Spec Kit already configured.
- There is a clear contract/boundary between Spec Kit and specdrive:
  - Spec Kit owns `.specify` initialization.
  - Specdrive owns feature scaffolding + AI prompt structure.
- Because the tool is additive and local-only, it is safe to introduce into critical repos.
- Adding API-based AI flows later is straightforward:
  - The same specs/contracts and bootstrap assumptions still apply.
