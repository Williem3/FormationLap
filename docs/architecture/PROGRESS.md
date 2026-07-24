# Formation Lap progress ledger

This is the only source of truth for milestone status. The milestone definitions
and exit criteria live in [`BUILD_PLAN.md`](BUILD_PLAN.md).

Last updated: 2026-07-24

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
| M2 | `complete` | Codex `/root` | 2026-07-23 | 2026-07-23 | [M2 evidence](evidence/M2.md), [wizard screenshot](evidence/m2-wizard.jpg), [editor screenshot](evidence/m2-editor.jpg) | Begin M3 |
| M3 | `complete` | Codex `/root` | 2026-07-23 | 2026-07-24 | [M3 evidence](evidence/M3.md), [Dashboard screenshot](evidence/m3-dashboard.jpg) | Begin M4 |
| M4 | `not_started` | — | — | — | — | Ready after M3 |
| M5 | `complete` | Codex `/root` | 2026-07-23 | 2026-07-24 | [M5 evidence](evidence/M5.md), [recommended path](evidence/m5-recommended.jpg), [Manual Entry](evidence/m5-manual-entry.jpg) | Continue M4 |
| M6 | `not_started` | — | — | — | — | Wait for M4 |
| M7 | `not_started` | — | — | — | — | Ready after M3 |
| M8 | `not_started` | — | — | — | — | Wait for M4 and M7 |
| M9 | `not_started` | — | — | — | — | Wait for M5 and M8 |
| M10 | `not_started` | — | — | — | — | Wait for M6 and M9 |

## Current work

No milestone implementation is active. M3 and M5 are complete; M4 is the next
dependency-ready milestone and should begin with the FormationLapCore Session
state red test.

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

- None.

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
| 2026-07-23 | Codex `/root` | M2 | Added the first FormationLapCore profile command, stable IDs, versioned atomic JSON creation, restart loading, and generated snapshot contracts | [`M2 evidence`](evidence/M2.md): red compile failure, green restart test, frontend verification, Clippy, and four Rust tests passed | No blocker; reject blank profile and Primary Sim names before storage |
| 2026-07-23 | Codex `/root` | M2 | Finalized the first M2 slice and added the standing agent requirement to create focused local commits after verified workspace changes | `pnpm.cmd verify`; Rust fmt, Clippy, and tests; final diff and Git status reviewed | No blocker; reject blank profile and Primary Sim names before storage |
| 2026-07-23 | Codex `/root` | M2 | Completed the automated M2 behavior surface, including atomic profile/settings replacement and recovery of interrupted or invalid writes | [`M2 evidence`](evidence/M2.md): frontend verification, Rust fmt, Clippy, generated-contract check, capability audit, and 24 Rust tests passed | Open the documented M2 preview in a fresh in-app Browser tab; capture wizard/editor screenshots and manually verify keyboard access and 125–200% scaling |
| 2026-07-23 | Codex `/root` | M2 | Confirmed the automated milestone surface remains complete and the development preview is healthy | Clean worktree; M2 wizard endpoint returned HTTP 200; no fresh in-app Browser tab was available on the third consecutive goal turn | Blocked on the documented manual UI evidence; open the M2 wizard preview in a fresh in-app Browser tab |
| 2026-07-23 | Codex `/root` | M2 | Captured wizard/editor screenshots and fixed the 125% overflow by stacking profile layouts before their minimum columns exceed the workspace | [`M2 evidence`](evidence/M2.md): 125% red and green dimensions, 200% no-overflow dimensions, screenshots, formatting, lint, typecheck, ten React tests, production build, contracts, and capability audit passed | Confirm the five-control native Tab traversal in the visible wizard; then complete M2 and begin M3 |
| 2026-07-23 | Codex `/root` | M2 | Completed every profile, persistence, recovery, UI, keyboard, and scaling exit criterion | [`M2 evidence`](evidence/M2.md); user confirmed native five-control Tab traversal; worktree and evidence links reviewed | No blocker; begin M3 with a red FormationLapCore launch test through ProcessRuntime |
| 2026-07-23 | Codex `/root` | M3 | Delivered stable process identity, the complete local lifecycle policy, 11 real Windows fixture cases, five typed commands, Dashboard controls/status/output, and the VirtualDesktopSwitcher-compatible demonstration | [`M3 evidence`](evidence/M3.md): 45 Rust tests, 14 React tests, Rust fmt/Clippy, frontend verify/build, generated contracts, and zero-permission capability audit passed | Reload the already-open `?preview=m3-dashboard` tab so screenshot/scaling review can complete M3 |
| 2026-07-23 | Codex `/root` | M5 | Added limited Steam discovery through FormationLapCore, following declared libraries and curated App IDs while omitting stale installations | [`M5 evidence`](evidence/M5.md): 50 Rust tests and all-target/all-feature Clippy passed; the real temporary fixture covers two libraries and a missing installation | No blocker; add targeted Windows installed-app, running-process, and known-location discovery |
| 2026-07-23 | Codex `/root` | M5 | Added bounded installed-app, running-Process, and known-location discovery with separate Steam and standalone iRacing results | [`M5 evidence`](evidence/M5.md) and the [discovery boundary](../security/M5_DISCOVERY_BOUNDARY.md): 53 Rust tests and all-target/all-feature Clippy passed | No blocker; rank compatibility recommendations and add LMUFFB's GitHub Update Provider |
| 2026-07-23 | Codex `/root` | M5 | Added compatibility-ranked recommendations, LMUFFB's typed GitHub Releases provider, and referential catalog validation | [`M5 evidence`](evidence/M5.md): 55 Rust tests and all-target/all-feature Clippy passed; official LMUFFB and SimHub metadata links recorded | No blocker; resolve local icons, generic fallback, and missing paths |
| 2026-07-23 | Codex `/root` | M5 | Added local Steam and executable icon extraction, generic fallback, and documented the existing missing-path repair workflow | [`M5 evidence`](evidence/M5.md) and [discovery boundary](../security/M5_DISCOVERY_BOUNDARY.md): 56 Rust tests and all-target/all-feature Clippy passed | No blocker; expose discovery and recommendations through NativeBridge, then build Manual Entry |
| 2026-07-23 | Codex `/root` | M5 | Exposed discovery and ranked recommendations through two narrow Tauri commands, generated contracts, and matching NativeBridge adapters | [`M5 evidence`](evidence/M5.md): 57 Rust tests, 15 React tests, contracts, Clippy, and fifteen-command capability audit passed | No blocker; build and capture the recommended and Manual Entry wizard paths |
| 2026-07-23 | Codex `/root` | M5 | Added the installed-sim picker, ranked installed recommendations, saved discovered launch sources, Manual Entry, and a deterministic M5 preview | [`M5 evidence`](evidence/M5.md): 17 React tests, lint, typecheck, and production build passed | Reload `?preview=m5-wizard`; capture and review the required recommended and Manual Entry screenshots |
| 2026-07-24 | Codex `/root` | M3 | Captured all Dashboard status families and fixed lifecycle-row overflow at a 200% effective viewport | [`M3 evidence`](evidence/M3.md), [Dashboard screenshot](evidence/m3-dashboard.jpg), `pnpm.cmd verify`, and browser measurements at 125%/200% | M3 complete; M4 and M7 are ready |
| 2026-07-24 | Codex `/root` | M5 | Captured and reviewed the recommended and Manual Entry wizard paths, including selected Supporting Application persistence and scaling | [`M5 evidence`](evidence/M5.md), [recommended path](evidence/m5-recommended.jpg), [Manual Entry](evidence/m5-manual-entry.jpg), and `pnpm.cmd verify` | M5 complete; begin M4 with a Session-state red test |

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
