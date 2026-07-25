# Troubleshooting

## Formation Lap does not start

1. Confirm 64-bit Windows 10 22H2 or Windows 11 and install current Windows
   updates.
2. Install or repair Microsoft Edge WebView2 Runtime.
3. Identify the release tier. A signed Beta/Stable installer must pass
   `Get-AuthenticodeSignature`. An official `v0.x` technical preview is
   intentionally unsigned and must instead match its checksum, GitHub
   provenance, prerelease tag, and `UNSIGNED-PREVIEW.txt` disclosure.
4. Launch Formation Lap from the Start Menu. If it is already running, use its
   system-tray icon to reopen the window.
5. Export diagnostics from Settings if the interface opens.

## A selected application is not found

Open the profile editor and rescan. If discovery still does not find it, use
Manual Entry and select the executable. Formation Lap does not search arbitrary
drives or download catalog changes. Moved or uninstalled executables must be
selected again.

For Steam titles, verify Steam is installed and the game appears in the local
Steam library. The ordinary and VR recipes intentionally avoid Steam's launch
choice dialog.

## Startup Sequence stops or times out

Formation Lap reports which Supporting Application failed and offers the
profile's allowed retry, skip, or cancel choices. Check the configured wait,
readiness method, executable path, and launch arguments. Use **Test Game
Launch** in the profile editor for a sanitized local report.

Do not force-stop a Pre-existing Process unless you recognize it and explicitly
accept the confirmation. Automatic cleanup applies only to Processes started
by the current Session.

## Elevation fails

The main application never runs as administrator. An explicit elevated launch
or close request starts the one-shot helper and displays Windows UAC. Signed
Beta/Stable builds show the verified publisher. An unsigned `v0.x` technical
preview shows **Unknown publisher** by design; confirm that the installed build
matches the official preview evidence before accepting. Canceling UAC cancels
the operation. Any invalid signature on a release claiming to be signed must
be reported through [`../SECURITY.md`](../SECURITY.md).

## Updates are Unknown

Check the network connection, system clock, and GitHub availability. Corporate
TLS interception or blocked GitHub endpoints can make the check fail closed.
Beta is opt-in and considers only published prereleases from the official
repository. Formation Lap never substitutes a download that lacks the required
Tauri updater signature.

Third-party update advice is notification-only. Provider throttling, changed
pages, unavailable versions, or Winget errors intentionally produce Unknown.

## Reset local state

First export any profiles you want to keep. Close Formation Lap, including the
system-tray process. The per-user application configuration directory is:

```text
%APPDATA%\com.formationlap.desktop
```

Rename that directory to keep a reversible backup, then start Formation Lap.
Delete the backup only after confirming the new state is correct.

## Uninstall completely

Use Windows **Settings → Apps → Installed apps → Formation Lap → Uninstall**.
The uninstaller removes application binaries and shortcuts. It preserves the
local configuration directory to avoid silently deleting profiles. Remove the
directory above manually only if a full data deletion is intended.

## Development setup failures

- Use `pnpm.cmd`, not a PowerShell `.ps1` shim.
- Confirm `node --version`, `pnpm.cmd --version`, `rustc --version`, and
  `cargo --version` match [`DEVELOPMENT.md`](DEVELOPMENT.md).
- Install the MSVC Desktop development with C++ workload.
- Run `pnpm.cmd install --frozen-lockfile`; do not repair lockfile drift by
  deleting committed lockfiles.
- exFAT hard-link fallback warnings from Cargo are harmless.

For a reproducible report, include the Formation Lap version, Windows version,
the failing command or interaction, and a sanitized diagnostic export.
