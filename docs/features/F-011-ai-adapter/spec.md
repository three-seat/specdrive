---
id: F-011-ai-adapter
title: AI Adapter Interface
type: feature        # feature | refactor | bugfix | chore
system: specdrive
status: draft
area: cli
owners:
  - three-seat
created_at: 2026-10-08
contract: docs/features/F-011-ai-adapter/contract.yaml
adrs:
  - ADR-0003
  - ADR-0004
  - ADR-0005
---

# Summary

Add an optional AI adapter that lets SpecDrive invoke a user-configured
external command once, feed it the same context bundle `chat export`
produces on stdin, capture its stdout, and pass that response through the
existing `chat import` validation, preview, and confirmation pipeline.
The adapter is a process boundary, not an AI client: SpecDrive never
speaks to an AI service directly, never retries or loops, and never writes
anything without explicit human confirmation. AI remains optional;
without an adapter configured, every existing workflow is unchanged.

# Context

- F-009 introduced `chat export` / `chat import`, which bridge SpecDrive
  artifacts and stateless AI chat tools via copy/paste.
- Many AI tools now ship non-interactive CLIs that read a prompt on stdin
  and write a response to stdout. Users cannot simply pipe
  `chat export | <tool> | chat import`, because `chat import` reads both
  the response and the y/N confirmation from the same stdin. A piped
  response consumes stdin, so confirmation can never be given
  interactively.
- F-011 defines a single, explicit boundary for that invocation so it is
  repeatable, bounded, and auditable rather than ad-hoc shell plumbing.
- The adapter is an arbitrary user-chosen program. SpecDrive cannot
  sandbox it. Some AI CLIs can edit files or run commands on their own.
  F-011 therefore treats the adapter as untrusted in both directions:
  its output is validated as untrusted input, and the repository is
  checked for out-of-band changes after it exits.
- Constraints:
  - No new crate dependencies (std only: `std::process`, `std::thread`,
    `std::time`, `std::io::IsTerminal`)
  - SpecDrive itself makes no network calls; the configured adapter
    process may (ADR-0005)
  - No shell interpretation of the adapter command
  - No orchestration, retries, multi-turn conversation, or autonomous
    flow control
  - Existing `draft`, `implement`, `chat export`, and `chat import`
    behavior must not change; refactoring import/export into reusable
    stages is permitted
  - Lifecycle state is not advanced by this feature
- Relevant constitutional principles: III (AI as a Junior Dev),
  IV (Safety and Reversibility), VI (SpecDrive Owns the Lifecycle),
  VIII (Bounded AI Execution: one prompt → one output → one patch),
  IX (Traceability). ADR-0003 (artifact ownership), ADR-0004 (lifecycle
  state model), ADR-0005 (AI adapter process boundary, to be written
  before contract approval).

# Behavior

## User flow

Command:
```
specdrive ai run draft <FEATURE_ID>
specdrive ai run implement <FEATURE_ID>
```

FEATURE_ID is the full feature directory name (for example,
`F-021-some-label`), as with `chat export` / `chat import`.

Help text: "Send one context bundle to the configured AI adapter and
import its response."

Inputs:
- FEATURE_ID: existing feature with spec and contract
- Workflow: `draft` or `implement`
- Adapter configuration from `docs/specdrive/config.yaml`

Outputs (identical to `chat import` for the same workflow):
- `draft`: replaces `docs/features/<FEATURE_ID>/contract.yaml`; writes
  `outputs/notes-NNN.md` if the response contains NOTES
- `implement`: writes `outputs/implement-NNN.raw.md`; never modifies
  source code or patch artifacts
- Adapter stderr is passed through to the user's terminal unmodified

Interaction:
```
$ specdrive ai run draft F-011-ai-adapter

Adapter:  claude -p
Workflow: draft
Feature:  F-011-ai-adapter (state: contract)

The following files will be sent to the adapter process. The adapter
may transmit them to an external service:
  docs/features/F-011-ai-adapter/spec.md
  docs/features/F-011-ai-adapter/contract.yaml
  docs/constitution.md
  docs/system-overview.md
  docs/adrs/ADR-0001-specdrive-bootstrap-and-role.md
  ...
Bundle: 18,432 bytes

Send bundle to adapter? [y/N] y

Running adapter... done (exit 0, 42s)

Parsing response...

Notes from AI:
  ...

Files to be written:
  docs/features/F-011-ai-adapter/contract.yaml   (31 changes)

Write these files? [y/N]
```

## Detailed behavior — Run

1. Require stdin to be an interactive terminal; otherwise fail (E-011).
2. Validate FEATURE_ID as a safe single-directory component (reuse
   F-009 validation) before any filesystem operation (E-001).
3. Validate that the feature exists and has spec and contract (E-002).
4. Compute lifecycle state via the F-010 state model
   (`StateLog::compute` over the inferred base state). Refuse if the
   displayed state is `blocked`, `deferred`, or `done`, or if the
   workflow is `draft` and the state is `review` (E-012). `implement` is
   allowed in `review`. Nothing is written to `state.yaml`.
5. Run the import clean-tree gate (same check as `chat import`, untracked
   files allowed) (E-004).
6. Load and validate adapter configuration (see Configuration).
7. Build the export bundle in memory using the same resolver and builder
   as `chat export <workflow>`. Content is byte-identical to the stdout
   of `chat export`. Missing optional context files produce the same
   warnings as export.
8. Show adapter argv, workflow, feature state, the list of files in the
   bundle, the bundle size, and a disclosure that the adapter may
   transmit the content externally. Ask for confirmation to invoke.
   Declining exits 0 with nothing invoked or written.
9. Invoke the adapter (see Adapter invocation).
10. Re-run the clean-tree check. If the adapter changed tracked files,
    fail (E-010), discard the response, and write nothing. SpecDrive
    does not attempt to revert the adapter's changes; the user reverts
    with git.
11. Hand the captured stdout to the `chat import` stages: read until END
    → parse → validate → preview → confirm → apply. Every F-009 import
    rule and error applies unchanged (missing END, malformed response,
    path containment, delimiter injection, block limits, contract
    structure). Stray text outside NOTES/FILE blocks is ignored as in
    F-009. The confirmation is read from the terminal, not from adapter
    output.
12. Exactly one adapter invocation per command run. No retries.

## Detailed behavior — Adapter invocation

- Spawn `command[0]` with `command[1..]` as arguments directly, with no
  shell. Shell metacharacters in config are passed literally. No values
  (FEATURE_ID, workflow, paths) are interpolated into argv.
- Working directory: repository root. Environment: inherited unchanged
  from SpecDrive (the adapter may need API keys from it).
- stdin: the bundle, written from a dedicated thread, then closed. This
  avoids deadlock when the bundle and response both exceed pipe buffers.
  If the adapter closes stdin early (broken pipe), stop writing and
  continue to wait for exit; the exit status and output decide the
  outcome.
- stdout: captured from a dedicated thread, bounded by
  `chat.import.max_response_size_bytes`. On exceeding the limit, kill
  the adapter and fail (E-009).
- stderr: inherited (passed through to the terminal).
- Timeout: measured from spawn. On expiry, kill the adapter and fail
  (E-008). Kill applies to the direct child process only; descendants
  the adapter spawned are not tracked (documented limitation).
- Exit status: non-zero exit fails (E-007), and the exit code is shown.
  Termination by signal is treated the same as non-zero exit.
- Captured stdout must be valid UTF-8; otherwise fail (E-013).
- Interrupt (Ctrl-C): the adapter and SpecDrive both receive the signal
  as members of the terminal's foreground process group. No write is
  possible before the second confirmation, so an interrupt at any
  earlier point leaves the repository unchanged.

## Detailed behavior — Configuration

Adapter settings are read from the SpecDrive config file under the
`ai.adapter` namespace:

```yaml
ai:
  adapter:
    command: ["claude", "-p"]   # required for ai run; argv list, no shell
    timeout_seconds: 600         # optional; default 600
```

- `ai.adapter` absent: `ai run` fails with a message showing the config
  to add (E-005). No other command reads this section.
- `command` missing, empty list, empty first element, or not a list of
  strings: fail (E-006). This is a hard error, not a fallback, because
  there is no safe default command.
- `command[0]` not found or not executable: fail at spawn (E-007).
- `timeout_seconds` absent: default 600. Zero, negative, or non-integer:
  warning, fall back to 600 (same pattern as `chat.import` limits).
- Response size uses the existing `chat.import.max_response_size_bytes`
  setting and default. No separate adapter size setting.

## Error cases

| ID    | Condition                                         | Exit |
|-------|---------------------------------------------------|------|
| E-001 | FEATURE_ID invalid                                | 1    |
| E-002 | Feature, spec, or contract missing                | 1    |
| E-003 | Workflow not `draft` or `implement`               | 1    |
| E-004 | Working tree dirty before invocation              | 1    |
| E-005 | No `ai.adapter` configuration                     | 1    |
| E-006 | `ai.adapter.command` invalid                      | 1    |
| E-007 | Adapter failed to start, or exited non-zero       | 2    |
| E-008 | Adapter exceeded timeout                          | 2    |
| E-009 | Adapter stdout exceeded max response size         | 1    |
| E-010 | Adapter modified tracked files                    | 1    |
| E-011 | stdin is not an interactive terminal              | 1    |
| E-012 | Feature blocked, deferred, done, or draft-in-review | 1  |
| E-013 | Adapter stdout is not valid UTF-8                 | 1    |

Response validation failures after a successful invocation use the
F-009 import errors and exit codes unchanged. Declining either
confirmation exits 0. In every error case, nothing is written.

# Non-Functional Requirements

- Performance: adds no overhead beyond the adapter process itself;
  stdout capture is bounded by the existing response size limit.
- Portability: Linux, macOS, Windows via `std::process`; no shell or
  display-server dependency. Terminal detection uses
  `std::io::IsTerminal`.
- Security:
  - No shell; argv only from config; no interpolation.
  - Adapter output is untrusted and receives the full F-009 import
    defenses (path traversal, absolute paths, symlink escape, delimiter
    injection, size limits).
  - Adapter side effects on tracked files are detected after exit and
    abort the run (E-010). Untracked-file side effects are not detected
    (documented limitation; consistent with `allow_untracked`).
  - Data egress is disclosed: the user sees every file in the bundle and
    an explicit notice before anything is sent.
  - The adapter inherits the full environment, so it can read any
    secret SpecDrive's environment holds. This is intended (credentials
    are the adapter's concern) and documented.
  - Nothing is written before the second confirmation.
- Git safety: clean tree required before invocation and re-verified after
  the adapter exits. SpecDrive never runs git commands that modify the
  repository.
- UX: the user always sees what will be sent and to which command before
  it runs, and what will be written before it is written. Adapter stderr
  stays visible for diagnostics. Every failure says what went wrong and
  what to do next.

# Acceptance Criteria

- [ ] AC-1: With no adapter configured, `ai run` fails with E-005 and a
      config example; all other commands behave exactly as before
- [ ] AC-2: The bundle sent to the adapter is byte-identical to
      `chat export <workflow> <FEATURE_ID>` stdout
- [ ] AC-3: The adapter command is executed without a shell; shell
      metacharacters in config are passed literally
- [ ] AC-4: Declining the invoke prompt runs nothing, writes nothing,
      exits 0
- [ ] AC-5: Non-zero exit (E-007), timeout (E-008), oversized stdout
      (E-009), and non-UTF-8 stdout (E-013) each fail with a specific
      error and write nothing
- [ ] AC-6: Adapter output is subject to every `chat import` validation;
      a response rejected by `chat import` is rejected identically here
- [ ] AC-7: Files are written only after the second, terminal-read
      confirmation
- [ ] AC-8: The adapter is invoked exactly once per run
- [ ] AC-9: `chat import` and `chat export` behavior and existing tests
      are unchanged after they are refactored into reusable stages
- [ ] AC-10: `ai run` does not change lifecycle state or write
      `state.yaml`
- [ ] AC-11: `ai run` refuses blocked, deferred, and done features, and
      `draft` on features in `review` (E-012), before invoking the
      adapter; `implement` in `review` is allowed
- [ ] AC-12: An adapter that modifies a tracked file causes E-010 and
      nothing is written
- [ ] AC-13: `ai run` refuses when stdin is not a terminal (E-011)
- [ ] AC-14: A bundle and a response each larger than the OS pipe buffer
      complete without deadlock
- [ ] AC-15: The invoke prompt lists every file in the bundle and the
      external-transmission notice
- [ ] AC-16: Invalid `timeout_seconds` warns and uses 600; invalid
      `command` fails with E-006

# Implementation Notes

- Expected files/modules to touch:
  - `src/cli.rs`: `ai run` subcommand and usage text
  - `src/ai/` (new): adapter config, invocation, run flow, errors
  - `src/chat/import.rs`: split into stages (read-until-END, parse,
    validate, plan, confirm, apply) that accept a response source and a
    separate confirmation reader; `chat import` composes them unchanged
  - `src/chat/export.rs`: expose bundle building (returning content and
    the resolved file list); `chat export` still prints the bundle
  - `src/config.rs`: `ai.adapter` section and validation
  - `src/lifecycle/`: read-only use of existing state computation; no
    changes expected
  - README, `docs/system-overview.md`
- Refactors allowed: import/export staging as above, with no behavior
  change to existing commands.
- Dependencies: none.
- Lifecycle: does not advance lifecycle state. A `draft` import may
  change the inferred base state on the next `status` check, exactly as
  F-009 `chat import` does today. Prompt/output persistence remains F-014.
- Testing:
  - Tests need a fake adapter that works on Linux, macOS, and Windows,
    so no shell scripts. Use a small test-only binary (for example,
    `tests/bin/fake_adapter.rs` registered as a `[[bin]]` or built via
    `CARGO_BIN_EXE_*` in integration tests) that can be driven by
    arguments to: echo a canned response, exit non-zero, sleep past a
    timeout, emit oversized output, emit invalid UTF-8, close stdin
    early, and modify a tracked file.
  - Interactive-terminal checks require the run flow to accept injected
    "is terminal" and confirmation sources so tests can exercise the
    full path without a TTY.
  - New tests must not use `set_current_dir`, which is the cause of the
    existing flaky parallel tests; pass the repo root explicitly.

# Follow-up Work

- ADR-0005: AI adapter process boundary. Required before contract
  approval. Covers: SpecDrive makes no network calls itself; adapters are
  untrusted external processes; no shell; one invocation per run; human
  confirmation on both sides of the boundary.
- Constitution clarification: the no-network rule (Section IV and
  Additional Constraints) applies to the SpecDrive process; configured
  adapter processes are governed by ADR-0005. Version bump per
  Governance.
- README: `ai run` command docs; correct short feature ID examples
  (`F-005`) to full IDs where commands require them.
- `docs/system-overview.md`: add the adapter boundary.
- Possible later feature: `specdrive ai check` to validate adapter config
  and executability without sending anything.
- Lifecycle follow-up: define a general policy for which commands may
  act in which lifecycle states (including `chat import` and `ai run`
  during `review`). F-011's Q8 rule is interim. Candidate home: F-012
  or an ADR-0004 companion.
- Separate fix (not F-011): existing tests that call `set_current_dir`
  fail intermittently under parallel `cargo test`.

# Open Questions

- Q1 (closed): Command name is `specdrive ai run <workflow> <FEATURE_ID>`.
- Q2 (closed): SpecDrive makes no network calls itself; a configured
  adapter process may. ADR-0005 (AI adapter process boundary) and a
  constitution clarification that the no-network rule applies to the
  SpecDrive process are required before contract approval.
- Q3 (closed): No argv placeholders in F-011. Fixed argv from config only.
- Q4 (closed): `ai run` refuses when stdin is not a terminal. No `--yes`
  flag in F-011.
- Q5 (closed): No persistence of the bundle or raw adapter output beyond
  what `chat import` already writes. Lineage persistence is F-014.
- Q6 (closed): The adapter inherits SpecDrive's full environment.
  SpecDrive does not manage credentials.
- Q7 (closed): `ai run` refuses blocked, deferred, and done features.
- Q8 (closed, interim): `ai run draft` is refused in `review`;
  `ai run implement` is allowed. Which commands may act in which
  lifecycle states should be revisited in depth as lifecycle work (see
  Follow-up Work).
