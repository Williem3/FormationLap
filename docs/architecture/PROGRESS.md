# Formation Lap progress ledger

This is the only source of truth for milestone status. The milestone definitions
and exit criteria live in [`BUILD_PLAN.md`](BUILD_PLAN.md).

Last updated: 2026-07-23

## Status values

- `not_started` — no implementation work has begun.
- `in_progress` — an agent owns the milestone and is actively working it.
- `blocked` — progress requires a named external decision, permission, or state
  change.
- `complete` — every exit criterion is satisfied and linked evidence exists.

Normally only one milestone is `in_progress`. If parallel milestones are active,
their owners and non-overlapping file scopes must be recorded in Current Work.

## Milestones

| ID | Status | Owner | Started | Completed | Evidence | Next action |
| --- | --- | --- | --- | --- | --- | --- |
| M0 | `complete` | Codex | 2026-07-23 | 2026-07-23 | Product spec, architecture, ADRs, design system, concept images, test seams | Begin M1 |
| M1 | `complete` | Codex `/root` | 2026-07-23 | 2026-07-23 | [M1 evidence](evidence/M1.md), [shell screenshot](evidence/m1-shell.png), [capability audit](../security/M1_CAPABILITY_AUDIT.md) | Begin M2 |
| M2 | `not_started` | — | — | — | — | Begin Racing Profiles and transparent persistence |
| M3 | `not_started` | — | — | — | — | Wait for M2 |
| M4 | `not_started` | — | — | — | — | Wait for M3 |
| M5 | `not_started` | — | — | — | — | Wait for M2 |
| M6 | `not_started` | — | — | — | — | Wait for M4 and M5 |
| M7 | `not_started` | — | — | — | — | Wait for M3 |
| M8 | `not_started` | — | — | — | — | Wait for M4 and M7 |
| M9 | `not_started` | — | — | — | — | Wait for M5 and M8 |
| M10 | `not_started` | — | — | — | — | Wait for M6 and M9 |

## Current work

No milestone is currently in progress.

The next ready milestone is **M2 — Racing Profiles and transparent
persistence**. Do not begin M3, M4, M5, or another later milestone until its
recorded dependencies are complete.

Known environment facts:

- Node.js `25.8.0` is installed.
- pnpm `10.33.0` is installed.
- Visual Studio Build Tools 2026 with MSVC `14.50.35717` is installed.
- Microsoft Edge WebView2 Runtime `150.0.4078.83` is installed.
- Rust and Cargo `1.97.1` are installed and selected by
  `rust-toolchain.toml`.
- PowerShell script execution is restricted; use `.cmd` shims for npm/pnpm
  commands when necessary.
- The workspace drive is exFAT. pnpm uses the hoisted linker; Cargo emits
  harmless hard-link fallback warnings and copies incremental-cache files.
- Local Git history is present. No remote is configured, so hosted CI metadata
  is not available in this workspace.

## Blockers

None.

## M1 evidence

- [`evidence/M1.md`](evidence/M1.md)
- [`evidence/m1-shell.png`](evidence/m1-shell.png)
- [`../security/M1_CAPABILITY_AUDIT.md`](../security/M1_CAPABILITY_AUDIT.md)
- [`../../README.md`](../../README.md)
- [`../DEVELOPMENT.md`](../DEVELOPMENT.md)
- [`../../.github/workflows/ci.yml`](../../.github/workflows/ci.yml)

## M0 evidence

- [`../../CONTEXT.md`](../../CONTEXT.md)
- [`../PRODUCT_SPEC.md`](../PRODUCT_SPEC.md)
- [`ARCHITECTURE.md`](ARCHITECTURE.md)
- [`BUILD_PLAN.md`](BUILD_PLAN.md)
- [`../design/UI_SYSTEM.md`](../design/UI_SYSTEM.md)
- [`../design/IMAGEGEN_PROMPTS.md`](../design/IMAGEGEN_PROMPTS.md)
- [`../design/concepts/ui-kit.png`](../design/concepts/ui-kit.png)
- [`../design/concepts/dashboard.png`](../design/concepts/dashboard.png)
- [`../design/concepts/profiles.png`](../design/concepts/profiles.png)
- [`../design/concepts/settings.png`](../design/concepts/settings.png)
- [`../adr/0001-tauri-react-rust.md`](../adr/0001-tauri-react-rust.md)
- [`../adr/0002-transparent-local-json-storage.md`](../adr/0002-transparent-local-json-storage.md)
- [`../adr/0003-one-shot-elevated-helper.md`](../adr/0003-one-shot-elevated-helper.md)
- [`../adr/0004-session-process-ownership.md`](../adr/0004-session-process-ownership.md)
- [`../adr/0005-signed-github-release-path.md`](../adr/0005-signed-github-release-path.md)

## Handoff log

Append one entry at the end of every implementation turn that changes the
workspace. Keep entries concise and link detailed evidence instead of pasting
logs.

| Date | Agent | Milestone | Work completed | Verification | Blockers / next |
| --- | --- | --- | --- | --- | --- |
| 2026-07-23 | Codex | M0 | Captured product, domain, architecture, ADRs, milestone plan, UI contract, and four generated concepts | Documentation links validated; PNG dimensions verified | No blocker; start M1 by installing Rust and scaffolding the secure shell |
| 2026-07-23 | Codex `/root` | M1 | Delivered the pinned Tauri/React foundation, native visual shell, generated Rust/TypeScript NativeBridge seam, least-privilege capability, test harnesses, CI, and setup/security docs | [`M1 evidence`](evidence/M1.md): frozen install, frontend/Rust checks, contract and capability audits, native screenshot, and debug NSIS bundle all passed | No blocker; begin only M2 with a profile-contract red test and real temporary storage |
| 2026-07-23 | Codex `/root` | M1 | Reviewed the full M0/M1 baseline, isolated Vite loopback access to the development CSP, aligned the dark border token, and organized the initial local history | Frozen install; `pnpm.cmd verify`; Rust fmt, Clippy, and tests; native debug build; Markdown-link and PNG-integrity checks passed | No blocker; begin M2 with a profile-contract red test and real temporary storage |

## Handoff entry template

```text
| YYYY-MM-DD | agent/task name | Mx | concise behavior delivered | commands/tests/evidence links | exact blocker or next smallest action |
```

## How to update this ledger

When starting a milestone:

1. Change its Status to `in_progress`.
2. Set Owner and Started.
3. Replace Current Work with the exact slice and files being touched.
4. Append a Handoff log entry before yielding.

When completing a milestone:

1. Verify every exit criterion in `BUILD_PLAN.md`.
2. Link durable evidence in the milestone row or a dedicated evidence section.
3. Set Status to `complete` and add the completion date.
4. Clear Current Work and identify the next ready milestone.

When blocked:

1. Set Status to `blocked` only when the work truly cannot continue.
2. Record the exact missing permission, decision, dependency, or external state.
3. Name the smallest action that would unblock progress.
