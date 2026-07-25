# M10 Windows release checklist

Candidate: `v1.0.0-beta.__` · Commit: pending · GitHub workflow run: pending

Tester/date: pending

This matrix must be completed against the published signed Beta before the
Stable `v1.0.0` tag. Record machine/VM identifiers and link screenshots or logs
without including personal data.

An unsigned `v0.x` technical preview does not satisfy or replace any row in
this checklist.

## Local unsigned preflight

Completed on the development Windows 11 machine on 2026-07-24. This validates
bundle shape and installer mechanics but does not replace the signed Beta
matrix.

- [x] Branded per-user NSIS installer built at version `1.0.0`.
- [x] Installed file set contained only the main app, one-shot helper, and
      uninstaller.
- [x] Installed app produced a responsive native window and clean idle exit.
- [x] Uninstall removed binaries, Start Menu shortcut, and registry entry.
- [x] Uninstall preserved the per-user configuration directory.
- [x] No local debug artifact was uploaded or represented as signed.

## Artifact verification

- [ ] Release workflow completed after protected-environment approval.
- [ ] Main executable Authenticode status is `Valid`.
- [ ] One-shot helper Authenticode status is `Valid`.
- [ ] Installer Authenticode status is `Valid` and timestamped.
- [ ] Installer SHA-256 matches `SHA256SUMS.txt`.
- [ ] Tauri updater accepts the candidate signature and rejects altered bytes.
- [ ] `latest.json` names only the official GitHub asset.
- [ ] SPDX SBOM, dependency-license report, notices, and provenance are present
      and independently readable/verifiable.
- [ ] Public release assets contain no unsigned `.exe`, `.msi`, `.dll`, or
      archive with executable content.

## Operating-system matrix

Repeat every row on a clean/reset 64-bit Windows 10 22H2 machine and a current
Windows 11 machine.

| Check                                                                  | Windows 10 22H2 | Windows 11 | Evidence |
| ---------------------------------------------------------------------- | --------------- | ---------- | -------- |
| Per-user install completes without administrator rights                | Pending         | Pending    | —        |
| Start Menu shortcut and branded installer/uninstaller are correct      | Pending         | Pending    | —        |
| First launch opens the onboarding flow                                 | Pending         | Pending    | —        |
| Curated discovery and Manual Entry work                                | Pending         | Pending    | —        |
| Profile save/export/import round trip works                            | Pending         | Pending    | —        |
| Empty roaming storage migrates to LocalAppData without deleting backup | Pending         | Pending    | —        |
| Conflicting local and roaming stores are not merged                    | Pending         | Pending    | —        |
| Startup Sequence launches Supporting Applications, then Primary Sim    | Pending         | Pending    | —        |
| Failure retry/skip/cancel paths remain usable                          | Pending         | Pending    | —        |
| Session-owned close and Pre-existing confirmation behave correctly     | Pending         | Pending    | —        |
| Explicit elevated launch/close shows one UAC prompt and succeeds       | Pending         | Pending    | —        |
| Tray hide/show/exit and interrupted-Session recovery work              | Pending         | Pending    | —        |
| Stable/Beta update check stays race-safe while Primary Sim runs        | Pending         | Pending    | —        |
| Signed Beta-to-newer-Beta update installs only after explicit intent   | Pending         | Pending    | —        |
| Third-party update advice never downloads an installer                 | Pending         | Pending    | —        |
| Uninstall removes binaries, shortcuts, and uninstaller registry entry  | Pending         | Pending    | —        |
| Uninstall removes only exact-owned Formation Lap startup values        | Pending         | Pending    | —        |
| User profiles remain until explicitly deleted                          | Pending         | Pending    | —        |

## Accessibility and visual matrix

| Check                                                                   | Windows 10 22H2 | Windows 11 | Evidence |
| ----------------------------------------------------------------------- | --------------- | ---------- | -------- |
| All version-one flows complete with keyboard only                       | Pending         | Pending    | —        |
| Focus is visible and follows a logical order                            | Pending         | Pending    | —        |
| Status uses icon and text, not color alone                              | Pending         | Pending    | —        |
| Light theme contrast has no known WCAG AA failure                       | Pending         | Pending    | —        |
| Dark theme contrast has no known WCAG AA failure                        | Pending         | Pending    | —        |
| 100%, 125%, 150%, and 200% scaling preserve content/actions             | Pending         | Pending    | —        |
| System, Light, and Dark theme selection persists                        | Pending         | Pending    | —        |
| Windows reduced-motion preference suppresses nonessential motion        | Pending         | Pending    | —        |
| Native Windows chrome remains present                                   | Pending         | Pending    | —        |
| Final Dashboard, profile editor, onboarding, and Settings match UI spec | Pending         | Pending    | —        |

## Completion

- [ ] No unresolved high/critical threat.
- [ ] No accepted product-spec or UI-system mismatch.
- [ ] Final screenshot set linked from `M10.md`.
- [ ] Stable promotion approved only after all checks above are complete.
