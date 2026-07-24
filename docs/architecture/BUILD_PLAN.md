# Formation Lap build plan

This is the stable delivery plan. Milestone status lives only in
[`PROGRESS.md`](PROGRESS.md); do not add completion checkboxes here.

Each milestone is a vertical slice with user-observable behavior, tests at an
approved seam, and explicit exit evidence. Agents may split a milestone into
smaller red-green cycles, but must not mark it complete until every exit
criterion is satisfied.

## Delivery order

| ID | Milestone | Depends on |
| --- | --- | --- |
| M0 | Product, architecture, and design baseline | — |
| M1 | Secure project foundation | M0 |
| M2 | Racing Profiles and transparent persistence | M1 |
| M3 | Local application lifecycle | M2 |
| M4 | Session orchestration | M3 |
| M5 | Curated catalog and local discovery | M2 |
| M6 | Steam, non-Steam, and VR launch recipes | M4, M5 |
| M7 | Privileged operations | M3 |
| M8 | Desktop integration and recovery | M4, M7 |
| M9 | Update advice and signed self-update | M5, M8 |
| M10 | Release hardening and version-one distribution | M6, M9 |

M5 and M7 may proceed independently after their dependencies are complete.
Only one milestone should normally be marked `in_progress`; parallel work is
allowed only when agents own non-overlapping files and the ledger names both
owners.

## M0 — Product, architecture, and design baseline

### Outcome

Future contributors share one vocabulary, product definition, module map,
visual system, test seams, and completion process.

### Deliverables

- Domain glossary.
- Version-one product specification.
- Architecture and security model.
- ADRs for hard-to-reverse decisions.
- UI system plus Dashboard, Profiles, and Settings concepts.
- Milestone plan and progress ledger.
- Root agent instructions.

### Exit criteria

- The product specification contains every decision from the grilling session.
- Public test seams are recorded and approved.
- Architecture assigns lifecycle truth to FormationLapCore.
- UI tokens and screen contracts are preserved in the repository.
- Future agents have an unambiguous startup and handoff procedure.

### Required evidence

- `CONTEXT.md`
- `docs/PRODUCT_SPEC.md`
- `docs/architecture/ARCHITECTURE.md`
- `docs/design/UI_SYSTEM.md`
- `docs/adr/`
- `AGENTS.md`

## M1 — Secure project foundation

### Outcome

A developer can clone, install, test, run, and package a minimal Formation Lap
window with the approved visual shell and a typed Rust/TypeScript seam.

### Deliverables

- Pinned stable Rust toolchain.
- Tauri 2 workspace with React, strict TypeScript, and Vite.
- Pinned pnpm version and committed lockfiles.
- Native Windows title bar and single main window.
- Base UI tokens, typography, sidebar shell, theme primitives, focus styles,
  reduced-motion support, and generic icon fallback.
- Minimal Tauri capability file that loads bundled content only.
- NativeBridge interface with production and in-memory test adapters.
- Rust-to-TypeScript contract generation with a CI staleness check.
- Rust and React test harnesses.
- Formatting, linting, type-checking, and test commands.
- Initial Windows CI workflow without signing secrets.

### Exit criteria

- A clean checkout has documented setup commands.
- `pnpm` install uses the frozen lockfile.
- Rust formatting, linting, and tests pass.
- TypeScript linting, type checking, and tests pass.
- Generated bindings are unchanged after regeneration.
- `pnpm tauri dev` opens the designed shell on Windows.
- A debug installer or bundle builds successfully.
- The WebView cannot navigate to a remote origin or invoke generic shell,
  filesystem, process, or HTTP capabilities.

### Required evidence

- Tool versions in `rust-toolchain.toml`, `package.json`, and lockfiles.
- CI run or local command transcript summarized in `PROGRESS.md`.
- Screenshot of the running shell compared with the design references.
- Capability audit linked from the milestone evidence.

## M2 — Racing Profiles and transparent persistence

### Outcome

Users can create, edit, duplicate, delete, import, export, and select Racing
Profiles whose state survives restart and is recoverable from storage damage.

### Deliverables

- Versioned profile and settings contracts.
- Profile validation and stable identifiers.
- ProfileLibrary with atomic JSON replacement, bounded backups, and migrations.
- Import/export format with machine-specific path-repair diagnostics.
- First-run profile wizard.
- Profiles sidebar and editor matching the approved concepts.
- Source choice for Steam or direct executable.
- Startup ordering editor with locked game-last row.
- Required/Optional, keep-running, VR default, and close-session settings.

### Exit criteria

- Core behavior tests prove create/list/edit/delete through the public interface.
- Persistence tests use a real temporary directory and prove restart survival.
- A deliberately interrupted or invalid write does not destroy the last valid
  profile.
- Import does not restore transient Session or process identities.
- Invalid executable paths are preserved but clearly marked for repair.
- React tests cover first profile creation and editing through NativeBridge.
- Keyboard navigation and 125–200% scaling are manually checked.

### Required evidence

- Red-green test names and results.
- Example exported profile fixture.
- Screenshots of wizard and profile editor.
- Storage migration and backup test results.

## M3 — Local application lifecycle

### Outcome

One configured local application can be launched, observed, closed, and
restarted safely from the Dashboard.

### Deliverables

- ProcessRuntime seam and Windows adapter.
- Stable process identity using PID, creation time, and executable identity.
- Direct executable launch with arguments and working directory.
- Monitored-process override for launcher-style applications.
- Pre-existing Process detection and single-instance behavior.
- Starting, Running, Running (pre-existing), Not Responding, Stopping, Stopped,
  and Failed statuses.
- Normal window close, console interrupt, custom stop command, five-second
  graceful wait, and confirmed force termination.
- Visible/hidden console modes and bounded output capture.
- Dashboard application rows and explicit Start, Exit, and Restart actions.
- Fixture programs for healthy, slow, failing, launcher-style, console, and Not
  Responding behavior.

### Exit criteria

- FormationLapCore behavior tests use the scripted ProcessRuntime adapter.
- Windows integration tests launch and control real fixture executables.
- Arguments containing spaces and quotes reach the executable without shell
  interpretation.
- A reused PID cannot be mistaken for an earlier process identity.
- Automatic actions never close a Pre-existing Process.
- Explicit pre-existing Exit/Restart requires confirmation.
- Restart cannot produce a duplicate old/new pair.
- Not Responding requires two consecutive failed checks.
- Console output remains bounded and shutdown attempts graceful interruption
  before force.
- The Dashboard reports only state returned by Rust.

### Required evidence

- Fixture matrix and Windows integration results.
- Dashboard screenshot for each major status family.
- Process-identity test report.
- Manual VirtualDesktopSwitcher-compatible recipe demonstration.

## M4 — Session orchestration

### Outcome

A complete Racing Profile starts and closes as one predictable Session.

### Deliverables

- Idle, Starting, Cancelling, Active, Closing, and RecoveryAvailable states.
- Serialized async command loop.
- Ordered Supporting Application startup and Primary Sim last.
- Per-entry timeout and post-start delay.
- Required failure block and Optional failure continuation.
- Cancel Startup with partial-attempt cleanup.
- Reverse-order Close Session.
- Primary Sim exit triggers Close Session.
- Keep-running detachment.
- Single Active Session and active-profile edit lock.
- Quiet race-safe event recording and post-session summary.
- Dynamic Formation Rail connected to authoritative snapshots.

### Exit criteria

- Core tests cover every legal state transition and reject illegal transitions.
- Required and Optional failure scenarios produce the agreed outcomes.
- Cancellation never launches the next entry and cleans only attempt-owned
  processes.
- Primary Sim is never launched before eligible Supporting Applications finish
  startup.
- Close Session preserves Pre-existing and Keep-running Applications.
- Unexpected Primary Sim exit begins cleanup exactly once.
- Competing Start/Close/Restart requests cannot create two Sessions.
- No unsolicited frontend notification appears while the Primary Sim is
  running.
- Formation Rail order and state match the core snapshot.

### Required evidence

- State-transition test matrix.
- End-to-end fixture Session run.
- Dashboard captures for pre-start, starting, active, failed, and closing.

## M5 — Curated catalog and local discovery

### Outcome

Formation Lap discovers recognized installed sims and applications without a
full-drive scan, while preserving a Manual Entry escape hatch.

### Deliverables

- Versioned bundled catalog schemas for sims and Supporting Applications.
- Initial catalog entries listed in the product specification.
- Steam manifest, Windows installed-app, running-process, and known-location
  discovery.
- Compatibility-ranked recommendations per sim.
- Manual Entry flow.
- Local executable and Steam icon extraction with generic fallback.
- Missing-path repair workflow.
- Catalog validation and duplicate App ID checks in CI.

### Exit criteria

- Discovery is limited to documented targeted sources.
- Only curated racing sims appear in the default sim picker.
- Manual unsupported sims and applications remain possible.
- LMUFFB is recommended for Le Mans Ultimate and has its GitHub Update Provider.
- Standalone and Steam iRacing can be distinguished.
- No copyrighted game artwork is bundled or downloaded.
- Invalid catalog data fails CI with actionable errors.

### Required evidence

- Catalog validation report.
- Discovery fixtures for multiple Steam libraries and missing installations.
- Wizard screenshots showing recommended and manual paths.

## M6 — Steam, non-Steam, and VR launch recipes

### Outcome

The Primary Sim launches through Steam or a direct executable, with a remembered
VR choice and a diagnosable no-dialog game-specific recipe.

### Deliverables

- Steam and direct-executable Primary Sim recipes.
- Remembered dashboard VR toggle.
- Preferred VR Launch Mode per Racing Profile.
- Curated ordinary and VR recipes for supported sims.
- Per-profile recipe override.
- Test Game Launch flow and local diagnostic report.
- Monitored process learning/override after Steam launch.
- SteamVR pre-existing detection and ownership-aware stop option.

### Exit criteria

- VR toggle is editable before startup and locked during an Active Session.
- Steam launch recipes do not intentionally open a mode-choice dialog.
- Direct executable iRacing works without Steam.
- Test Game Launch starts no Supporting Applications and mutates no signed
  catalog data.
- Diagnostics include the chosen recipe and observed process without exposing
  unnecessary local data.
- Stop SteamVR affects it only when the Session started it.
- Curated recipe tests and documented manual checks cover every supported sim
  that is available to the maintainers.

### Required evidence

- Recipe test table per sim and VR mode.
- Sanitized Test Game Launch diagnostic example.
- Manual verification notes for Steam and standalone paths.

## M7 — Privileged operations

### Outcome

Formation Lap can safely launch, close, and restart elevated applications while
its main process remains non-administrative.

### Deliverables

- Versioned helper request/response protocol.
- Signed-build-compatible one-shot helper binary.
- Current-user-only authenticated IPC and single-use nonce.
- Canonical target and bounded operation validation.
- Batched elevated startup.
- Elevated close/restart path.
- Development test adapter and adversarial protocol tests.

### Exit criteria

- Main application manifest does not request administrator privileges.
- Helper rejects version mismatch, replay, wrong user, wrong parent identity,
  noncanonical paths, arbitrary shell text, and out-of-scope operations.
- Helper exits after every success or failure.
- One Session startup causes at most one UAC prompt for its elevated launch
  batch.
- No long-running privileged process or service remains.
- Elevated launch and close pass a documented manual Windows test.

### Required evidence

- Protocol threat checklist.
- Automated validation test results.
- Manual UAC test notes.
- Signature verification plan for public artifacts.

## M8 — Desktop integration and recovery

### Outcome

Formation Lap behaves like a dependable Windows utility across window close,
tray use, restart, theme changes, and interrupted Sessions.

### Deliverables

- System tray menu and status.
- Close-to-tray during Active Session.
- Explicit Quit flow.
- Opt-in Start with Windows, minimized.
- Single Formation Lap instance.
- Active-session journal written as ownership changes.
- Verified Recovery Offer that never acts automatically.
- Settings screen matching the approved concept.
- System, Light, and Dark themes.
- Local bounded logs, diagnostics export, and settings backups.

### Exit criteria

- Closing the window during an Active Session preserves monitoring.
- Quit clearly offers close-session or leave-applications-running choices.
- Profiles never auto-start with Windows.
- Recovery verifies PID, creation time, and executable identity before offering
  monitoring.
- Dismissing recovery launches, closes, and adopts nothing.
- Theme, scaling, keyboard, focus, and reduced-motion checks pass.
- Diagnostic export contains useful local evidence and no telemetry upload.

### Required evidence

- Tray and restart manual test matrix.
- Recovery fixture tests including PID reuse.
- Light and dark Settings screenshots.
- Diagnostic export sample.

## M9 — Update advice and signed self-update

### Outcome

Users receive trustworthy, race-safe update information for Formation Lap and
notification-only advice for configured applications.

### Deliverables

- Stable and Beta channel selection.
- Tauri signed self-update check and install flow.
- Daily maximum check schedule and manual Check Now.
- Active-Session update suppression.
- GitHub Releases, Winget, and official-page Update Provider adapters.
- Current, Update Available, and Unknown states.
- Curated providers for recognized applications where reliable.
- No third-party install capability.

### Exit criteria

- Invalid or missing Tauri update signatures are rejected.
- Update installation cannot start during an Active Session.
- Automatic checks happen at most once per day and can be disabled.
- Third-party providers send no centralized inventory.
- Unknown is shown instead of guessing when current/latest versions cannot be
  compared reliably.
- No interface or Rust command can install a third-party application update.
- Deferred update results appear only after race-safe behavior ends.

### Required evidence

- Signed-update happy-path and rejection tests.
- Provider adapter contract tests.
- Network destination inventory.
- Settings and Dashboard update-state screenshots.

## M10 — Release hardening and version-one distribution

### Outcome

Formation Lap version one is documented, accessible, signed, auditable, and
installable through the official GitHub release path.

### Deliverables

- Complete README, setup, security, privacy, troubleshooting, and contribution
  documentation.
- MIT license and third-party notices.
- Full Windows CI and release workflows with pinned actions.
- Dependency audit and license policy.
- Installer branding and clean uninstall.
- Authenticode for application, helper, and installer.
- Tauri updater signatures.
- SHA-256 checksums, SBOM, dependency-license report, and build provenance.
- Stable and Beta release procedures.
- Release candidate test pass on Windows 10 22H2 and Windows 11.

### Exit criteria

- Every earlier milestone is complete with evidence.
- All automated release gates pass from a clean checkout.
- Installer, launch, update, and uninstall are manually verified.
- No unsigned binary ships in a public artifact.
- Accessibility checks cover keyboard, contrast, screen scaling, and reduced
  motion.
- Threat checklist has no unresolved critical or high-severity issue.
- Product specification and UI system match shipped behavior.
- A signed Beta candidate is tested before the Stable version-one tag.

### Required evidence

- GitHub release workflow run.
- Signature and checksum verification output.
- SBOM, license report, and provenance links.
- Windows 10 and 11 release checklist.
- Final screenshot set.

## Universal milestone rules

Every implementation milestone follows these rules:

1. Read `AGENTS.md`, `CONTEXT.md`, the architecture, the product specification,
   the current progress ledger, relevant ADRs, and the UI system before editing.
2. Mark the milestone `in_progress` in `PROGRESS.md` before implementation.
3. Work one red-green behavior slice at a time.
4. Test through approved interfaces; mock only true external or OS seams.
5. Preserve unrelated user changes.
6. Update documentation when behavior or an interface changes.
7. Record commands, results, screenshots, and unresolved risks as evidence.
8. Mark `complete` only when all exit criteria and evidence are present.
9. If blocked, record the exact blocker and the smallest next action; do not
   mark the milestone complete.
10. Append a handoff entry before ending work.

## Scope-change rule

An agent may clarify implementation details within an accepted milestone. It may
not silently change version-one behavior, security guardrails, storage,
ownership semantics, technology stack, or distribution trust. Such a change
requires user agreement, an updated product specification, and an ADR when the
decision meets the ADR threshold.
