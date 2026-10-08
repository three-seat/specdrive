# ADR-0005: AI Adapter Process Boundary

Status: Proposed
Date: 2026-10-08

## Context

F-011 introduces an optional AI adapter: SpecDrive invokes a
user-configured external command, sends it a context bundle on stdin,
and imports the response from its stdout. This is the first time
SpecDrive launches a process that is expected to reach the network and
whose behavior SpecDrive does not control.

Until now, SpecDrive's AI integration has been human-mediated. `draft`
and `implement` print prompts; F-009 `chat export` / `chat import` move
content through a human copy/paste step. The human was the boundary.

The constitution states "No network calls from specdrive" (Section IV)
and "No network traffic from the CLI in v0.1" (Additional Constraints).
Those rules were written for a tool that only touched local files. They
do not say whether SpecDrive may launch a program that makes network
calls. F-011 needs an answer, and the answer will govern every future
AI-facing feature, so it belongs in an ADR rather than a single spec.

The specific questions this ADR settles:

- Does the no-network rule permit launching a networked adapter?
- What is SpecDrive's trust model for an adapter process?
- How is an adapter invoked, and what is SpecDrive responsible for?
- Where must a human decide, and can that be bypassed?
- How does the adapter relate to Bounded AI Execution and traceability?

## Decision

### 1. SpecDrive Makes No Network Calls; Adapters May

The no-network rule applies to the SpecDrive process. SpecDrive does
not open sockets, embed HTTP clients, or link AI provider SDKs.

A user-configured adapter process may make network calls. Choosing an
adapter, and accepting what it transmits, is the user's decision,
made explicitly in configuration and confirmed at each run.

This keeps SpecDrive provider-neutral, dependency-free, and
offline-capable. Every AI provider, local model, or internal gateway
integrates the same way: as a command that reads stdin and writes
stdout.

### 2. Adapters Are Untrusted in Both Directions

SpecDrive treats an adapter as an arbitrary program it cannot sandbox.

- **Output is untrusted input.** Adapter stdout passes through the
  same validation as a human-pasted `chat import` response: path
  containment, delimiter rules, size limits, and structure checks. No
  validation is relaxed because the content came from a configured
  tool.
- **Side effects are detected, not prevented.** Some AI tools edit
  files or run commands on their own. SpecDrive verifies the working
  tree is clean before invocation and re-verifies after the adapter
  exits. Changes to tracked files abort the run with nothing written.
  SpecDrive does not revert them; git is the recovery mechanism
  (Constitution IV).
- **Resources are bounded.** Every invocation has a timeout and an
  output size limit. Exceeding either kills the adapter and aborts.

### 3. Invocation Contract

An adapter is invoked as a direct process, never through a shell.

- argv comes only from configuration, as a list. Nothing (feature IDs,
  paths, workflow names) is interpolated into it.
- stdin receives exactly one context bundle in the SpecDrive delimited
  format and is then closed.
- stdout is the response, in the SpecDrive delimited format.
- stderr belongs to the user and is passed through unmodified.
- The adapter inherits SpecDrive's environment. Credentials are the
  adapter's concern; SpecDrive neither reads nor manages them.
- Exit code 0 means a response is available. Any other outcome is a
  failure.

stdin/stdout and the existing delimiter format are the entire
interface. There is no adapter plugin API, no protocol negotiation,
and no provider-specific code in SpecDrive.

### 4. Humans Gate Both Sides of the Boundary

Every adapter run has two explicit confirmations:

1. **Before sending.** The user sees the command, the files in the
   bundle, and a notice that the adapter may transmit them externally.
2. **Before writing.** The user sees the files to be written and a
   change summary, exactly as in `chat import`.

Both confirmations are read from an interactive terminal. Adapter runs
refuse non-interactive stdin, and there is no flag to skip either
confirmation. Relaxing this for CI or automation requires a new ADR.

### 5. One Invocation, No Flow Control

Each command run invokes the adapter at most once. SpecDrive does not
retry, chain, loop, or hold multi-turn conversations with an adapter,
and does not let adapter output decide what SpecDrive does next.

This is Bounded AI Execution (Constitution VIII, ADR-0003 §4) applied
at the process boundary: one bundle in, one response out, one human
decision on what to keep. It is also what keeps SpecDrive outside the
"agent orchestration framework" category the README rules out.

### 6. Lifecycle and Traceability

Adapter runs do not record or advance lifecycle state (ADR-0004).
Their artifacts are the same derived artifacts `chat import` writes,
and any lifecycle effect is inferred from those artifacts exactly as
for a manual import.

Adapters close the gap between prompt and output mechanically, but
SpecDrive does not yet persist prompts or hash them. Full
spec → contract → prompt → output lineage for adapter runs is F-014's
responsibility. No parallel persistence scheme is introduced here.

## Relationship to Constitution and Prior ADRs

- Clarifies Constitution Section IV and Additional Constraints: the
  no-network rule governs the SpecDrive process. A constitution
  amendment stating this explicitly accompanies this ADR.
- Applies Constitution III (AI as a Junior Dev), IV (Safety and
  Reversibility), VIII (Bounded AI Execution), and IX (Traceability)
  to external process invocation.
- Builds on ADR-0003 (artifact ownership; AI produces artifacts only)
  and ADR-0004 (lifecycle state is owned by SpecDrive and recorded
  only by human commands).
- Does not supersede ADR-0001 through ADR-0004.

## Consequences

### Positive

- SpecDrive stays dependency-free and provider-neutral; any CLI that
  speaks stdin/stdout can be an adapter.
- The security model is uniform: pasted and adapter responses take
  the same validation path.
- Human control is preserved at both points where content crosses the
  boundary.
- The boundary is narrow enough to reason about and test with a fake
  adapter.

### Negative

- SpecDrive cannot prevent an adapter from reading files, making
  network calls, or changing untracked files; it can only detect
  tracked-file changes after the fact.
- The timeout kills only the direct child process; descendants the
  adapter spawned may outlive it.
- Requiring an interactive terminal rules out CI use for now.
- Users must configure and trust a third-party CLI, including its data
  handling.

### Neutral / accepted tradeoffs

- The adapter inherits the full environment, so it can see any secret
  SpecDrive's environment holds. Accepted: filtering would break
  credential lookup and gives a false sense of isolation.
- Adapter output must already be in the SpecDrive delimited format.
  Adapters that cannot produce it need a wrapper script; SpecDrive
  does not translate formats.

## Affected features

- F-011: AI adapter interface. Implements this boundary.
- F-014: Prompt hashing and artifact lineage. Must cover adapter-run
  prompts and outputs.
- F-015: Repository baseline tracking. Should record repository state
  around adapter runs.
- F-018: Validation gates. Adapter responses must pass the same gates
  as any import.

## Follow-up work

- Amend the constitution: state that the no-network rule (Section IV,
  Additional Constraints) applies to the SpecDrive process, and that
  configured adapter processes are governed by this ADR. Version bump
  per Governance.
- Revisit Decision 4 if non-interactive use is needed. Any relaxation
  requires a new ADR.
- Revisit Decision 2 if a portable, dependency-free way to contain
  adapter side effects becomes available.
