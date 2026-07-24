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
| M2 | `blocked` | Codex `/root` | 2026-07-23 | — | [M2 evidence](evidence/M2.md) | Confirm one native Tab traversal through the visible M2 preview |
| M3 | `not_started` | — | — | — | — | Wait for M2 |
| M4 | `not_started` | — | — | — | — | Wait for M3 |
| M5 | `not_started` | — | — | — | — | Wait for M2 |
| M6 | `not_started` | — | — | — | — | Wait for M4 and M5 |
| M7 | `not_started` | — | — | — | — | Wait for M3 |
| M8 | `not_started` | — | — | — | — | Wait for M4 and M7 |
| M9 | `not_started` | — | — | — | — | Wait for M5 and M8 |
| M10 | `not_started` | — | — | — | — | Wait for M6 and M9 |

## Current work

M2 is blocked after the profile-management slice:

- Delivered: `CreateProfile` assigns a stable identifier, atomically persists
  one versioned Racing Profile with exactly one Primary Sim; blank or
  whitespace-only Racing Profile and Primary Sim names are rejected before
  state or storage changes and when persisted documents are opened. Existing
  profiles can be edited while retaining their identifier and prior backup, or
  duplicated under a new stable identifier, or deleted while retaining a
  recoverable backup. Complete profile behavior settings persist and are
  exposed in authoritative snapshots and generated TypeScript contracts. The
  selected Racing Profile survives restart in versioned local settings, and an
  interrupted profile replacement recovers its last valid backup. Schema-one
  profiles migrate to the complete schema-two contract with backups. Portable
  export omits local identity and diagnostics; import assigns fresh stable IDs,
  preserves missing paths, and flags them for repair. Eight narrow typed Tauri
  commands and both NativeBridge adapters expose profile behavior. An
  empty-library user can create the first Racing Profile through the React
  wizard and see the authoritative result in the sidebar and dashboard. The
  editor saves profile, Primary Sim, ordered Supporting Application, VR, and
  Close Session settings through NativeBridge. Another Racing Profile can be
  selected in the sidebar and its authoritative detail replaces the dashboard.
  React also exposes duplication, explicit-target deletion confirmation,
  portable JSON export, and portable JSON import through NativeBridge. Profile
  dialogs use native modal behavior, support Escape, and restore keyboard focus
  to their triggering action or the surviving New profile action. Successful
  profile edits and migrations use Windows atomic replacement while retaining
  the previous document as the bounded backup. FormationLapCore also retains
  native-owned application identities and recomputes missing-path diagnostics
  instead of trusting those frontend fields. Profile selection settings use
  the same atomic replacement and recover their last valid backup when a
  legacy interrupted write leaves a temporary marker. If a live Racing Profile
  replacement is invalid JSON or contains invalid names, ProfileLibrary
  validates and restores its bounded last-valid backup before exposing state.
  The wizard and editor now stack before their minimum-width columns exceed the
  workspace; manual checks at effective 125% and 200% show no horizontal
  overflow. Durable wizard and editor screenshots are captured.
- Unblock action: with the M2 wizard preview visible, press Tab from Profile
  name through Primary Sim name, source, executable path, and Create Racing
  Profile. Confirm that focus advances in that order and remains visibly
  outlined.
- Test seams: FormationLapCore, Tauri commands and generated contracts, and
  React behavior through NativeBridge.
- Next file scope: `docs/architecture/evidence/M2.md`, UI evidence, and this
  ledger.

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

- M2 screenshots and scaling evidence are complete. A native Tab traversal
  remains pending because the in-app Browser's synthetic keyboard dispatch does
  not perform the browser's default focus movement. The visible preview is
  ready for the user to perform the five-control Tab pass described above.

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
