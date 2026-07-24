# Formation Lap agent guide

This file is the mandatory starting point for every agent working in this
repository.

## Start every task here

Read these files in order before editing:

1. [`CONTEXT.md`](CONTEXT.md) — canonical domain language.
2. [`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md) — accepted version-one
   behavior and scope.
3. [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md) —
   module interfaces, seams, security, and persistence.
4. [`docs/architecture/BUILD_PLAN.md`](docs/architecture/BUILD_PLAN.md) —
   milestone deliverables and exit criteria.
5. [`docs/architecture/PROGRESS.md`](docs/architecture/PROGRESS.md) — current
   owner, status, evidence, blockers, and next action.
6. [`docs/design/UI_SYSTEM.md`](docs/design/UI_SYSTEM.md) — interface tokens and
   screen contracts.
7. Relevant files in [`docs/adr/`](docs/adr/).

Then:

1. Inspect the current worktree and preserve unrelated user changes.
2. Confirm the target milestone is ready according to its dependencies.
3. Mark it `in_progress` in `PROGRESS.md` before implementation.
4. Record the exact behavior slice and file scope under Current Work.

Do not infer progress from code alone. `PROGRESS.md` is the milestone status
source of truth.

## Shared technical language

Use the architecture vocabulary consistently:

- **Module** — behavior hidden behind one interface.
- **Interface** — everything callers and tests must know.
- **Seam** — where behavior can vary without editing the caller.
- **Adapter** — a concrete implementation at a seam.

Prefer deep modules with small interfaces. Do not add a seam unless at least two
adapters are justified. Do not expose an internal seam merely to make a test
easier.

Use the domain terms from `CONTEXT.md` in code, tests, UI copy, and docs. If new
behavior makes a term ambiguous, resolve and update the glossary before naming
types.

## Architecture guardrails

These are non-negotiable unless the user approves a scope change and the
relevant documents are updated:

1. React renders snapshots and sends user intent; it does not own process or
   Session truth.
2. FormationLapCore owns lifecycle policy and serializes state transitions.
3. Tauri commands are narrow typed adapters with no policy.
4. ProcessRuntime observes Windows behavior but does not decide Session policy.
5. Automatic cleanup applies only to Session-owned Processes.
6. A PID alone is never a stable process identity.
7. Elevated work uses a typed, signed, one-shot helper; no persistent service.
8. The WebView receives no generic shell, filesystem, process, or HTTP access.
9. Third-party updates are notification-only.
10. The Curated Catalog changes only through signed Formation Lap releases.
11. User state, logs, and discovery results remain local.
12. Race-safe Behavior suppresses unsolicited disruptions while the Primary Sim
    runs.

## Test-driven implementation

The public test seams are already approved:

- FormationLapCore.
- ProcessRuntime.
- Tauri commands and generated contracts.
- React behavior through NativeBridge.

For each behavior slice:

1. Write one failing test through the relevant public interface.
2. Confirm the failure represents missing behavior.
3. Implement only enough to pass.
4. Repeat with the next behavior.

Tests describe observable outcomes, not private methods, internal call counts,
or incidental structure. Mock only OS and true external seams. Use real
temporary storage instead of mocking the filesystem.

Do not mark a milestone complete because its happy path works. Every exit
criterion and required evidence in `BUILD_PLAN.md` must be satisfied.

## Frontend implementation

- Follow `docs/design/UI_SYSTEM.md`; concept PNGs guide composition but are not
  literal assets.
- Keep native Windows window chrome.
- Use the Formation Rail only where it communicates actual Startup Sequence
  state.
- Use local application icons or the generic fallback; do not bundle game art.
- Status always has icon and text.
- Preserve keyboard access, visible focus, scaling, themes, and reduced motion.
- Do not replace approved tokens with a generic UI-library theme.
- Do not overwrite generated concept images unless the user explicitly asks for
  new concepts.

## Dependencies and generated code

- Use pinned pnpm and Rust versions.
- Commit `pnpm-lock.yaml` and `Cargo.lock`.
- Prefer the standard library and focused, actively maintained dependencies.
- Explain any dependency that gains process, filesystem, network, update, or
  privilege capability.
- Rust contracts are authoritative; generated TypeScript files must never be
  hand-edited.
- CI must fail when generated bindings are stale.

PowerShell script execution is restricted in the current environment. Use
`pnpm.cmd` and `npm.cmd` when PowerShell blocks their `.ps1` shims.

## Security and destructive actions

- Pass executable arguments directly; never build a shell command string.
- Canonicalize paths before launch or privilege escalation.
- Validate every frontend payload in Rust.
- Require explicit confirmation before force termination and before controlling
  a Pre-existing Process.
- Never publish, push, sign, or create a release unless the user has authorized
  that external action.
- Preserve user-owned worktree changes and avoid destructive git commands.

## Documentation and decisions

Update documentation in the same slice when behavior, an interface, or a
milestone changes.

`CONTEXT.md` is a glossary only—no implementation details.

Create or revise an ADR only for a decision that is:

1. Costly to reverse.
2. Surprising without context.
3. The result of a real trade-off.

Do not silently revise the product specification. Raise the conflict and obtain
user agreement first.

## Handoff and completion

Before ending a workspace-changing turn:

1. Run the verification appropriate to the slice.
2. Update the milestone row and Current Work in `PROGRESS.md`.
3. Append one concise Handoff log entry with:
   - behavior delivered;
   - commands/tests run;
   - durable evidence links;
   - exact blocker or next smallest action.
4. Leave a milestone `in_progress` if required work remains.
5. Mark `complete` only after every exit criterion has evidence.

The next agent should be able to continue from `PROGRESS.md` without reconstructing
intent from chat history.
