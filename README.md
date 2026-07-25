# Formation Lap

Formation Lap is a local-first Windows utility that prepares, monitors, and
closes the applications used for a sim-racing Session. Choose a Primary Sim,
add Supporting Applications, set their order and close behavior, then start the
whole Startup Sequence from one place.

Version one supports 64-bit Windows 10 22H2 and Windows 11. Formation Lap keeps
profiles, settings, logs, and discovery results on the PC. It has no account,
cloud sync, analytics, or telemetry.

## Version-one behavior

- Guided profile setup with curated sim and application discovery plus Manual
  Entry.
- Ordered launches, readiness checks, optional waits, failure choices, and the
  Primary Sim launched last.
- Clear Session states, system-tray status, local diagnostics, and recovery
  after an interrupted Session.
- Safe close and force-stop controls that distinguish Session-owned from
  Pre-existing Processes.
- Optional one-shot elevation for explicit launch or close actions; the main
  application never runs as administrator.
- Light, dark, and system themes; keyboard access; scaling; reduced-motion
  support; and race-safe suppression of unsolicited UI.
- Signed Formation Lap Stable and opt-in Beta updates. Third-party updates are
  notification-only.
- Before signed version one, explicitly labeled unsigned `v0.x` technical
  previews may be offered for early evaluation with updater signatures,
  checksums, SBOM, licenses, and provenance intact.

The accepted behavior and trust boundaries are documented in
[`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md) and
[`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md).

## Install

Download the current Windows installer and `SHA256SUMS.txt` from the
[official GitHub Releases page](https://github.com/Williem3/FormationLap/releases).
First identify its release tier:

- Signed Beta/Stable installers must report Authenticode status `Valid`.
- An unsigned `v0.x` technical preview must be a GitHub prerelease whose title,
  notes, and `UNSIGNED-PREVIEW.txt` all disclose the missing Windows publisher
  signature. Expect SmartScreen and unknown-publisher warnings.

Verify the checksum, then inspect the per-user NSIS installer:

```powershell
$version = "1.0.0"
$installer = "Formation-Lap_${version}_x64-setup.exe"
$expected = (Get-Content SHA256SUMS.txt |
  Select-String ([regex]::Escape($installer))).Line.Split(" ")[0]
$actual = (Get-FileHash $installer -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $expected) { throw "Checksum mismatch" }
Get-AuthenticodeSignature $installer | Format-List Status,SignerCertificate
```

Install a signed release only when the checksum matches and Authenticode status
is `Valid`. Install an unsigned technical preview only when the checksum,
official tag, disclosure, and GitHub provenance all match and you deliberately
accept the Windows warnings.
Formation Lap installs for the current Windows user and creates a Start Menu
entry. Windows maintains the WebView2 runtime used by the interface.

If neither a signed release nor an approved technical preview is published,
clone the repository and use the development instructions below; do not
redistribute development builds.

## Development

Prerequisites:

- Microsoft C++ Build Tools with **Desktop development with C++**.
- Microsoft Edge WebView2 Runtime.
- Node.js 24 or 25.
- pnpm `10.33.0`.
- rustup; `rust-toolchain.toml` selects Rust `1.97.1` MSVC.

From a clean checkout:

```powershell
corepack enable
corepack prepare pnpm@10.33.0 --activate
pnpm.cmd install --frozen-lockfile
pnpm.cmd verify
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
pnpm.cmd tauri dev
```

PowerShell may block package-manager `.ps1` shims; use `pnpm.cmd`. Generated
TypeScript contracts are authoritative from Rust and must be changed only with
`pnpm.cmd contracts:generate`.

See [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the complete command
reference, [`CONTRIBUTING.md`](CONTRIBUTING.md) before proposing changes, and
[`docs/TROUBLESHOOTING.md`](docs/TROUBLESHOOTING.md) when setup or launch fails.

## Trust, privacy, and releases

- [`SECURITY.md`](SECURITY.md) explains supported versions and vulnerability
  reporting.
- [`PRIVACY.md`](PRIVACY.md) lists every category of local and network data.
- [`docs/RELEASE.md`](docs/RELEASE.md) defines the unsigned-preview and signed
  Stable/Beta processes, artifacts, and verification gates.
- [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) explains generated
  dependency notices.
- [`LICENSE`](LICENSE) contains the MIT license.

Formation Lap is an independent project and is not affiliated with the games,
hardware vendors, or supporting applications it can launch.
