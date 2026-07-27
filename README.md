# Formation Lap

Formation Lap is a local-first Windows utility for starting, monitoring, and
closing everything used in a sim-racing Session. Build a Racing Profile with
one Primary Sim and an ordered set of Supporting Applications, then run the
whole Startup Sequence from one dashboard.

Formation Lap supports 64-bit Windows 10 22H2 and Windows 11. Profiles,
settings, discovery results, backups, and logs stay on this PC. There is no
account, cloud sync, analytics, or telemetry.

## Technical preview

Before version one, Formation Lap releases are technical previews for early
evaluation. An official `v0.x` preview:

- is published only as a prerelease on the
  [Formation Lap GitHub Releases page](https://github.com/Williem3/FormationLap/releases);
- does not have a Windows Authenticode publisher signature, so SmartScreen and
  **Unknown publisher** warnings are expected;
- still includes a Tauri updater signature, SHA-256 checksums, an SBOM,
  dependency licenses, and GitHub build provenance; and
- is not a signed Beta or Stable release.

Only continue past a Windows warning when the installer came from the official
repository and its checksum matches the release. Preview builds can display a
second **Unknown publisher** UAC prompt when a Racing Profile explicitly
launches or closes an elevated application.

## Install Formation Lap

### 1. Download the release

Open [GitHub Releases](https://github.com/Williem3/FormationLap/releases) and
select the newest `v0.x` release marked **Pre-release**. In its **Assets**
section, download:

- `Formation-Lap_<version>_x64-setup.exe`
- `SHA256SUMS.txt`
- `UNSIGNED-PREVIEW.txt`

Confirm that the release title and notes also call it an **unsigned technical
preview**. If no preview is published, there is currently no supported
installer to download.

### 2. Verify the installer

Put the installer and `SHA256SUMS.txt` in the same folder. Open PowerShell in
that folder, confirm the version below matches the release you selected, and
run:

```powershell
$version = "0.9.0-preview.8"
$installer = Get-Item ".\Formation-Lap_${version}_x64-setup.exe" `
  -ErrorAction SilentlyContinue

if (-not $installer) {
  throw "Formation Lap installer not found."
}

$checksumLine = Get-Content ".\SHA256SUMS.txt" |
  Where-Object { $_ -match [regex]::Escape($installer.Name) } |
  Select-Object -First 1

if (-not $checksumLine) {
  throw "Matching checksum not found."
}

$expected = $checksumLine.Split(" ", [System.StringSplitOptions]::RemoveEmptyEntries)[0]
$actual = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA256).Hash

if ($actual -ne $expected) {
  throw "Checksum mismatch. Do not run this installer."
}

"Checksum verified: $($installer.Name)"
Get-AuthenticodeSignature -LiteralPath $installer.FullName |
  Format-List Status, StatusMessage
```

For an official `v0.x` preview, the checksum must match and the Authenticode
status is expected to be `NotSigned`. A future release that claims to be a
signed Beta or Stable build must instead report `Valid`.

### 3. Run the installer

1. Double-click the verified installer.
2. If Microsoft Defender SmartScreen appears, confirm the filename and source,
   choose **More info**, then **Run anyway** only if you accept the preview
   warning.
3. Complete the per-user installation. Administrator access is not required
   for the main application.
4. Open **Formation Lap** from the Start Menu.

Formation Lap uses the Microsoft Edge WebView2 Runtime supplied and maintained
by Windows. If the window does not open, install current Windows updates or
repair WebView2, then see [Troubleshooting](docs/TROUBLESHOOTING.md).

## Create your first Racing Profile

On first launch, choose **Create Racing Profile** to open the profile wizard:

1. Give the Racing Profile a recognizable name, such as `iRacing road` or
   `LMU VR`.
2. Choose a detected Primary Sim. Formation Lap checks targeted locations such
   as Steam libraries, installed applications, running Processes, and Start
   Menu shortcuts; it never scans an entire drive.
3. If the sim is not detected, choose **Manual Entry**, select its executable
   or enter its Steam App ID, and name it.
4. Add any detected Supporting Applications. They appear before the Primary
   Sim in the Startup Sequence.
5. Review the order and choose **Create Racing Profile**.

Open **Edit profile** afterward to fine-tune the setup:

- move Supporting Applications into startup order;
- mark each one **Required** or **Optional**;
- choose whether it should keep running after Close Session;
- configure startup timeouts, delays, console visibility, elevation, and
  shutdown behavior;
- set the preferred VR launch mode and SteamVR close behavior; and
- repair or override executable and monitored-process paths.

A Required Application failure prevents the Primary Sim from starting. An
Optional Application failure is recorded while the Startup Sequence continues.
Use elevation only for an application that actually needs it; Formation Lap
itself continues to run without administrator privileges.

## Start and close a Session

For a new or changed profile, select **Test game launch** first. It starts only
the Primary Sim, reports the recipe and observed Process, and can learn the
monitored executable used by a launcher-based sim. Review and confirm any
learned path in the profile editor before relying on automatic ownership.

To run a normal Session:

1. Select the Racing Profile in the sidebar.
2. Set the **VR** toggle before startup.
3. Choose **Start session**.
4. Watch the Formation Rail as Supporting Applications start in saved order and
   the Primary Sim starts last.
5. Use an application's row to view captured output or perform an explicit
   Start, Exit, Restart, or Force stop action when available.
6. Choose **Close session** when finished. Closing the Primary Sim also begins
   Session cleanup.

Close Session requests a graceful stop, closes Session-owned Supporting
Applications in reverse startup order, and honors keep-running choices.
Formation Lap does not automatically close a matching Process that was already
running when the Session began. Explicitly controlling a Pre-existing Process
requires confirmation, and force termination always requires confirmation.

Only one Session can be active at a time, and its Racing Profile is read-only
until the Session ends. Closing the Formation Lap window during an Active
Session hides it to the system tray so monitoring continues. Use **Quit…** when
you intend to stop Formation Lap; during an Active Session it asks whether to
close the Session or leave its applications running.

## Profiles, recovery, and settings

- **New profile** creates another independent setup.
- **Duplicate profile** copies the selected setup for editing.
- **Export profile** produces portable JSON. **Import profile** accepts that
  JSON on another installation.
- An imported profile enters **Native Launch Quarantine**. Review its paths,
  arguments, elevation, monitored executable, and stop recipes before approving
  it and starting a Session.
- After an interrupted launcher exit, a **Recovery Offer** can resume monitoring
  only the Processes whose identity Formation Lap verifies. Dismissing it
  launches, closes, and adopts nothing.

In **Settings** you can choose the theme, reduce motion, opt into Start with
Windows, select the Stable or Beta update channel, and export diagnostics.
Start with Windows opens Formation Lap in the tray; it never starts a Racing
Profile automatically.

Automatic online checks are off for a new installation. **Check now** consents
to one check of Formation Lap and configured application providers. Formation
Lap installs only verified first-party updates and never installs a third-party
application update. Update checks and notifications stay quiet while the
Primary Sim is running.

## Data, uninstall, and help

User data is stored in:

```text
%LOCALAPPDATA%\com.formationlap.desktop
```

Uninstall Formation Lap from **Windows Settings → Apps → Installed apps**. The
uninstaller keeps the data directory so reinstalling does not silently erase
Racing Profiles. Export any profiles you need and delete that directory
manually only when you want a complete data reset.

For common launch, discovery, Startup Sequence, elevation, update, reset, and
uninstall problems, see [Troubleshooting](docs/TROUBLESHOOTING.md). Export a
sanitized local diagnostic from **Settings → Export diagnostics** when filing a
bug. Report security vulnerabilities privately as described in
[SECURITY.md](SECURITY.md).

## Build from source

Contributors need Microsoft C++ Build Tools with **Desktop development with
C++**, Microsoft Edge WebView2 Runtime, Node.js 24 or 25, pnpm `10.33.0`, and
rustup. `rust-toolchain.toml` selects Rust `1.97.1` MSVC.

```powershell
git clone https://github.com/Williem3/FormationLap.git
Set-Location FormationLap
corepack enable
corepack prepare pnpm@10.33.0 --activate
pnpm.cmd install --frozen-lockfile
pnpm.cmd verify
pnpm.cmd tauri dev
```

PowerShell may block package-manager `.ps1` shims; use `pnpm.cmd`. See the
[development guide](docs/DEVELOPMENT.md) for Rust checks and the complete
command reference, and read [CONTRIBUTING.md](CONTRIBUTING.md) before proposing
changes. Development builds are unsigned and must not be redistributed as
official Formation Lap releases.

## Trust and project documentation

- [Product behavior](docs/PRODUCT_SPEC.md)
- [Architecture and security boundaries](docs/architecture/ARCHITECTURE.md)
- [Privacy](PRIVACY.md)
- [Security policy](SECURITY.md)
- [Release process and artifact contract](docs/RELEASE.md)
- [MIT license](LICENSE)
- [Third-party notices](THIRD-PARTY-NOTICES.md)

Formation Lap is an independent project and is not affiliated with the games,
hardware vendors, storefronts, or Supporting Applications it can launch.
