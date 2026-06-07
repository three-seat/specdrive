---
id: F-009
title: Chat Export/Import Workflow
type: feature
system: specdrive
status: draft
area: cli
owners:
  - three-seat
created_at: 2026-06-02
contract: docs/features/F-009-chat/contract.yaml
adrs:
  - ADR-0003
---

# Summary

Add `specdrive chat export` and `specdrive chat import` subcommands to
support structured, auditable workflows with stateless AI chat tools.
Export assembles a self-contained context bundle from feature artifacts
and prints it to stdout. Import reads a delimited AI response from
stdin, previews changes, and writes artifacts to the feature directory
on confirmation. This closes the loop between SpecDrive's artifact model
and external AI chat tools without requiring filesystem access or API
integration on the AI side.

# Context

- SpecDrive's existing `draft` and `implement` commands generate prompts
  that reference local file paths. Stateless AI chat tools (Claude,
  ChatGPT, Copilot Chat) cannot read those paths — they require file
  contents to be provided inline.
- Currently the human must manually copy file contents into the chat,
  which is error-prone, inconsistent, and untraceable.
- There is no structured way to capture what was sent to the AI or what
  came back, breaking the artifact lineage chain defined in ADR-0003.
- F-009 addresses this by producing a self-contained export bundle and
  a structured import mechanism, both using a defined delimiter scheme.
- The `chat` subcommand namespace groups chat-related functionality and
  leaves room for future export/import concepts beyond AI chat workflows.
- Constraints:
  - No new crate dependencies
  - No clipboard API — stdout/stdin only
  - No filesystem access assumed on the AI side
  - Existing draft and implement command behavior must not change
  - Minor call-site changes for resolver extraction are permitted
  - Lifecycle state is not advanced by this feature — that is F-010's
    responsibility
  - Must work on any platform without display server or OS integration
  - Export context is determined solely by resolver functions — export
    commands must not recursively discover additional files
- Relevant constitutional principles: Constitution Sections IV
  (Safety and Reversibility), VI (SpecDrive Owns the Lifecycle),
  VIII (Bounded AI Execution), IX (Traceability as a First-Class Goal).
  See ADR-0003 for artifact ownership and lifecycle boundary decisions.

# Behavior

## User flow — Export

Command:
```
specdrive chat export draft <FEATURE_ID>
specdrive chat export implement <FEATURE_ID>
```

Inputs:
- FEATURE_ID — existing feature with spec and contract

Outputs:
- Delimited context bundle printed to stdout
- Bundle contains: export prompt, inlined file contents, delimiter
  structure instructing AI to respond in SpecDrive format

Error cases:
- FEATURE_ID not found — fail fast with clear message
- Spec or contract missing — fail fast with clear message
- Required context files missing (constitution, system overview) —
  warn, continue without missing files

## User flow — Import

Command:
```
specdrive chat import draft <FEATURE_ID>
specdrive chat import implement <FEATURE_ID>
```

Inputs:
- FEATURE_ID — existing feature
- Multiline paste from stdin terminated by `--- SPECDRIVE:END ---`

Outputs:
- Draft import: replaces contract.yaml after validation and user
  confirmation, saves notes artifact
- Implement import: saves raw output to outputs/ only, never modifies
  source code or patch artifacts

Interaction:
```
$ specdrive chat import draft F-009

Paste AI response. Waiting for --- SPECDRIVE:END ---

[user pastes]

Parsing response...

Notes from AI:
  LLR-003 may need revision once filesystem behavior is confirmed.

Files to be written:
  docs/features/F-009-chat/contract.yaml        (47 changes)
  docs/features/F-009-chat/outputs/notes-001.md (1 file)

Apply? [y/N]:
```

Error cases:
- Missing `--- SPECDRIVE:END ---` — hard failure, incomplete paste
  message
- FILE path not under `docs/features/<FEATURE_ID>/` after
  canonicalization — reject with clear message
- Draft import YAML validation failure — reject with specific error,
  nothing written
- No FILE blocks found — hard failure, malformed response message

## Detailed behavior — Export

1. Validate FEATURE_ID exists and has spec and contract
2. Call `resolve_draft_files()` or `resolve_implement_files()` to get
   file list — no additional file discovery permitted
3. Build export prompt — new prompt, files provided inline, no path
   references, delimiter guardrail embedded
4. Assemble delimited bundle:
   - `--- SPECDRIVE:BEGIN ---`
   - `--- SPECDRIVE:NOTES ---` (empty, AI fills on response)
   - `--- SPECDRIVE:FILE <path> ---` block per resolved file with
     inlined contents
   - `--- SPECDRIVE:PROMPT ---` block containing export prompt
   - `--- SPECDRIVE:END ---`
5. Print bundle to stdout
6. Print usage hint: "Copy the above and paste into your AI chat tool."

## Detailed behavior — Import

1. Print waiting message
2. Verify clean working tree — import requires a clean working tree
   before any write to keep changes isolated and git-revertible.
   Export skips this check.
3. Read stdin lines until the first `--- SPECDRIVE:END ---` is
   encountered. Input processing terminates immediately. Any content
   appearing after the first SPECDRIVE:END is ignored.
4. Parse delimited response:
   - Extract NOTES block if present
   - Extract each FILE block with path and contents
5. Dry validation pass — all validations run before any filesystem
   modification:
   - At least one FILE block present
   - All FILE paths canonicalize to within
     `docs/features/<FEATURE_ID>/` — path traversal sequences,
     symlinks, and relative tricks are rejected
   - For output paths that do not yet exist, canonicalize the parent
     directory and validate the normalized relative path rather than
     the full path
   - Draft import: validate that imported content is parseable YAML
     and contains the required top-level contract structure. Full
     schema validation is deferred to F-019.
   - Implement import: no content validation
6. If any validation fails — reject with specific error, nothing written
7. Display NOTES to user if present
8. Display file change preview with change counts
9. Prompt `Apply? [y/N]:`
10. On confirmation:
    - Draft: replace `docs/features/<FEATURE_ID>/contract.yaml` with
      validated content. Save notes to
      `docs/features/<FEATURE_ID>/outputs/notes-NNN.md` if NOTES
      block present.
    - Implement: save raw to
      `docs/features/<FEATURE_ID>/outputs/implement-NNN.raw.md`.
      Implement import must never modify source code or patch artifacts.
11. On rejection: exit cleanly, nothing written.

## Detailed behavior — File resolution

`resolve_draft_files()` returns:
- spec.md
- contract.yaml
- constitution.md
- system-overview.md
- All ADR files
- feature.contract.minimal.yaml template
- feature.contract.critical.yaml template

`resolve_implement_files()` returns:
- spec.md
- contract.yaml
- constitution.md
- system-overview.md
- All ADR files

Export context is determined solely by these resolver functions.
Export commands must not recursively discover additional files beyond
what the resolver returns.

## Detailed behavior — Output file numbering

Output files use zero-padded incrementing numbers:
- `notes-001.md`, `notes-002.md`, etc.
- `implement-001.raw.md`, `implement-002.raw.md`, etc.
- Number determined by finding the highest existing NNN matching the
  pattern in the `outputs/` directory and using NNN+1. Gaps in the
  sequence do not cause overwrites — if 001 and 003 exist, 004 is used.
- Numbering logic implemented as a reusable filesystem utility function
  for future use by other features.

# Non-Functional Requirements

- Performance: bundle assembly and import parsing must complete in under
  one second for typical feature sizes
- Portability: stdout/stdin only, no OS-specific APIs, no clipboard,
  no display server dependency — must work on Linux, macOS, and Windows
- Security: all imported FILE paths must resolve under
  `docs/features/<FEATURE_ID>/` using canonicalized paths — path
  traversal sequences, relative tricks, and symlinks are explicitly
  rejected before any write occurs. For output paths that do not yet
  exist, canonicalize the parent directory and validate the normalized
  relative path rather than the full path.
- Git safety: export is read-only and does not require a clean working
  tree. Import requires a clean working tree before any write to keep
  changes isolated and git-revertible per Constitution Section IV.
- UX: import interaction must be clear at each step — waiting, parsing,
  preview, confirm. Failure messages must state exactly what went wrong
  and what the user should do next.

# Acceptance Criteria

- [ ] AC-1: `specdrive chat export draft <FEATURE_ID>` prints a valid
      delimited bundle to stdout containing all resolved draft files
      inlined with correct delimiters
- [ ] AC-2: `specdrive chat export implement <FEATURE_ID>` prints a
      valid delimited bundle to stdout containing all resolved implement
      files inlined with correct delimiters
- [ ] AC-3: Export bundle contains SPECDRIVE:BEGIN, SPECDRIVE:NOTES,
      at least one SPECDRIVE:FILE block, SPECDRIVE:PROMPT, and
      SPECDRIVE:END in correct order
- [ ] AC-4: `specdrive chat import draft <FEATURE_ID>` reads stdin
      until SPECDRIVE:END, displays notes and file preview, writes on
      confirmation
- [ ] AC-5: `specdrive chat import implement <FEATURE_ID>` reads stdin
      until SPECDRIVE:END, displays notes and file preview, saves raw
      output only on confirmation
- [ ] AC-6: Import rejects response missing SPECDRIVE:END with clear
      error message
- [ ] AC-7: All imported FILE paths are canonicalized and validated to
      resolve under `docs/features/<FEATURE_ID>/` — path traversal,
      symlinks, and relative tricks are rejected with clear error. For
      non-existent output paths, parent directory is canonicalized and
      normalized relative path is validated.
- [ ] AC-8: Draft import rejects content that is not parseable YAML or
      lacks required top-level contract structure. Full schema
      validation is deferred to F-019.
- [ ] AC-9: Missing NOTES block does not produce an empty notes
      artifact
- [ ] AC-10: Output file numbering finds the highest existing NNN and
      uses NNN+1 — gaps in the sequence do not cause overwrites
- [ ] AC-11: Existing draft and implement command behavior is unchanged
- [ ] AC-12: No new crate dependencies introduced
- [ ] AC-13: `resolve_draft_files()` and `resolve_implement_files()`
      are extracted as shared functions callable by both existing
      commands and export commands
- [ ] AC-14: Import performs a complete dry validation pass before any
      filesystem modifications occur — all-or-nothing behavior
- [ ] AC-15: Implement import never modifies source code or patch
      artifacts — raw output saved to outputs/ only
- [ ] AC-16: Export context is determined solely by resolver functions
      — no recursive file discovery
- [ ] AC-17: Import ignores all content appearing after the first
      SPECDRIVE:END delimiter
- [ ] AC-18: Export does not require a clean working tree. Import
      verifies a clean working tree before any write.

# Implementation Notes

- Expected files/modules to touch:
  - `src/chat/export.rs` — export command implementation
  - `src/chat/import.rs` — import command implementation
  - `src/chat/mod.rs` — chat subcommand namespace
  - `src/resolve.rs` — shared file resolution functions
  - `src/fsutil.rs` — output file numbering utility function
  - `src/cli.rs` — add chat subcommand with export and import children

- Existing commands:
  - Existing draft and implement command behavior must not change
  - Minor call-site changes for resolver extraction are permitted

- Refactors:
  - Extract `resolve_draft_files()` and `resolve_implement_files()`
    into `src/resolve.rs`
  - Extract output numbering logic into `src/fsutil.rs` as a reusable
    utility

- Dependencies:
  - No new crates
  - Standard library string handling for delimiter parsing
  - `serde_yaml` for draft import YAML validation (already present)

# Open Questions

- Q1 (closed): V1 displays change counts only. Detailed diff display
  deferred to a future artifact review feature.