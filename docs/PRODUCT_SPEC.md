# Formation Lap product specification

## Purpose

Formation Lap is a lightweight Windows utility that prepares a sim-racing
session. A user chooses a Racing Profile, optionally enables VR, and starts an
ordered collection of Supporting Applications followed by one Primary Sim.
Formation Lap then monitors status and provides safe start, close, and restart
controls.

Version one prioritizes predictable lifecycle management, quiet behavior during
a race, inspectable local state, and a trustworthy release path.

## Supported platform

- 64-bit Windows 10 22H2 and Windows 11.
- Tauri 2, Rust, React, and strict TypeScript.
- English interface with localization-ready resources.
- Self-contained per-user Windows installer.

## Core workflow

1. The user selects or creates a Racing Profile.
2. The dashboard shows its Formation Rail, Supporting Applications, Primary
   Sim, and current statuses.
3. The user chooses VR on or off. The last choice is remembered by the profile.
4. Start Session launches Supporting Applications in user-defined order.
5. Formation Lap launches the Primary Sim last.
6. Closing the Primary Sim is equivalent to Close Session.
7. Close Session closes eligible Supporting Applications in reverse startup
   order, except those configured to keep running.

Only one Session and one managed instance per profile entry may be active at a
time. Starting another Racing Profile requires closing the Active Session.

## Racing Profiles

A Racing Profile contains:

- Exactly one Primary Sim.
- Zero or more ordered Supporting Applications.
- A remembered VR toggle and a preferred VR Launch Mode.
- Required/Optional classification for each Supporting Application.
- Per-entry startup timeout, optional post-start delay, arguments, working
  directory, monitored process name and optional canonical monitored executable
  path, console visibility, elevation, shutdown strategy, and keep-running
  behavior.
- Session-close behavior, including whether to stop SteamVR when eligible.

Profile structure is read-only during its Active Session. Runtime controls
remain available.

Profiles are stored locally and can be imported or exported as JSON. Existing
installed profiles remain approved. A newly imported portable profile preserves
all values but enters Needs Review and cannot start a Session until the user
reviews executable paths, arguments, working directories, elevation, monitored
executables, and custom-stop recipes. Elevated and custom-stop entries require
explicit approval; changing an approved privileged recipe invalidates that
approval. Missing or suspicious paths require file re-selection.

## First-run and discovery

First launch uses a guided profile wizard:

1. Discover installed racing sims.
2. Select a Primary Sim.
3. Suggest compatible Supporting Applications found locally.
4. Order applications and choose Required or Optional.
5. Review VR and close-session behavior.
6. Save and open the dashboard.

Discovery may inspect:

- Steam library manifests.
- Windows installed-application records.
- Currently running processes.
- Known default locations in the Curated Catalog.

Discovery must never crawl an entire drive.

The Curated Catalog is bundled with signed Formation Lap releases. It does not
update independently. Users can add unsupported sims or applications through a
Manual Entry.

### Initial racing-sim catalog

- iRacing, including standalone and Steam installations.
- Assetto Corsa.
- Assetto Corsa Competizione.
- Assetto Corsa EVO.
- Automobilista 2.
- rFactor 2.
- Le Mans Ultimate.
- RaceRoom Racing Experience.
- EA SPORTS WRC.
- DiRT Rally 2.0.

### Initial supporting-application catalog

- SimHub.
- Crew Chief.
- Trading Paints.
- Garage 61.
- RaceLab.
- iOverlay.
- Go Fast.
- SteamVR.
- LMUFFB.

The picker presents applications recommended for the selected sim first, then
allows access to the full catalog and Manual Entry flow.

## Launching applications

An executable Launch Recipe includes:

- Executable path.
- Arguments as a structured string passed directly to the executable.
- Working directory, defaulting to the executable's directory.
- Expected or overridden monitored process name and optional canonical
  executable path.
- Visible or hidden console mode.
- Normal or elevated launch.

Manual Entry and Racing Profile editing offer a native Windows file picker for
direct, monitored, and custom-stop executable paths. The picker returns only a
user-selected local path; typed paths remain available for advanced cases.

Long-running console applications are first-class Supporting Applications.
Formation Lap can hide or show the console, capture a bounded local output log,
and use a console interrupt as a graceful stop strategy.

Arbitrary pre-start and post-stop hook systems are out of scope. Applications
with arguments, including long-running terminal programs such as
`VirtualDesktopSwitcher.exe`, belong in the ordinary Startup Sequence.

## Startup sequencing

- Supporting Applications launch in saved order.
- The Primary Sim always launches last.
- A process appearing means it started; an optional post-start delay accounts
  for additional initialization.
- The default startup timeout is 30 seconds and can be overridden per entry.
- A Required Application failure blocks the Primary Sim.
- An Optional Application failure is recorded and the sequence continues.
- Start Session becomes Cancel Startup while sequencing.
- Cancel Startup stops future launches and cleans up processes started by that
  attempt while preserving Pre-existing Processes.
- Elevated entries execute at their saved positions. Only adjacent elevated
  entries may share a UAC transaction, so batching never reorders the Startup
  Sequence.

## Status model

The dashboard can show:

- Starting.
- Running.
- Running (pre-existing).
- Not Responding.
- Stopping.
- Stopped.
- Failed.

Update availability is a separate indicator.

Not Responding applies to windowed applications that fail two consecutive
Windows responsiveness checks, approximately six seconds total. Background
applications without a suitable window remain Running or Stopped.

## Ownership and process identity

If a matching process is already running when startup reaches an entry,
Formation Lap:

- Shows Running (pre-existing).
- Does not launch a duplicate.
- Does not automatically close it during Close Session.

Session-owned identity includes process ID, creation time, and expected
executable identity so a reused process ID cannot be mistaken for an old
process.

Every ProcessRuntime action opens the PID once, verifies creation time and the
canonical executable path from that handle, and keeps the same handle through
the action. A filename-only monitored match may be shown as observed, but it
cannot become Session-owned or be stopped automatically.

Explicit Exit or Restart actions may target a Pre-existing Process only after a
confirmation that names the ownership risk.

A Keep-running Application started by the Session becomes unmanaged after Close
Session. A later Session therefore sees it as Pre-existing.

## Closing and restarting

Exit and Restart use this order:

1. Request the configured graceful shutdown.
2. Wait up to five seconds.
3. If the process remains, ask before force termination.
4. Restart only after the old process has exited.

Graceful shutdown strategies include:

- Closing the application's windows.
- Sending a console interrupt.
- Running an explicitly configured stop executable and arguments.

Background tools without any graceful strategy require confirmation before
force termination.

Close Session:

1. Closes the Primary Sim when it is still running.
2. Closes Session-owned Supporting Applications in reverse startup order.
3. Leaves Pre-existing and Keep-running Applications untouched.
4. Leaves Steam and SteamVR running unless the relevant profile option applies.

The Stop SteamVR option closes SteamVR only when the Session started it.

## Race-safe behavior

While the Primary Sim is running, Formation Lap must not:

- Automatically restart a crashed application.
- Display unsolicited dialogs, toasts, update messages, or disruptive alerts.
- Install updates.

Status changes are recorded quietly. After the Session ends, a summary may show
launch failures, crashes, and deferred update notifications.

## Tray, startup, and recovery

- Closing the main window during an Active Session hides it to the system tray.
- An explicit Quit action asks whether to close the Active Session or leave its
  applications running.
- Start with Windows is opt-in, defaults off, and opens Formation Lap minimized
  to the tray.
- Racing Profiles never auto-start with Windows.

Formation Lap records an active-session journal. After an unexpected launcher
exit, it may offer to resume monitoring verified processes. It never resumes,
launches, closes, or restarts anything without the user's acceptance of the
Recovery Offer.

## Steam, non-Steam, and VR

Version one supports:

- Installed Steam sims.
- Direct executable sims.
- Standalone iRacing.

Dedicated Epic Games Store and EA App protocol integration is out of scope.

For Steam games, the ordinary and VR paths use no-dialog Steam launch recipes
where supported. The dashboard VR toggle remains binary, while the profile
editor can choose a game-specific preferred mode such as OpenXR, OpenVR/SteamVR,
or Oculus.

Every curated game recipe can be overridden in one Racing Profile. Test Game
Launch:

- Launches only the Primary Sim.
- Records the exact URI or arguments used.
- Observes the process that appears.
- Learns the canonical monitored executable path for a launcher-based profile
  and asks the user to confirm it before that path can establish ownership.
- Produces a copyable local diagnostic report.
- Never mutates the signed Curated Catalog.

## Elevated applications

The main Formation Lap process never runs as administrator.

Elevated operations use an authenticated, one-shot helper:

- Startup operations execute at their saved position and batch only adjacent
  elevated entries behind one UAC prompt where possible.
- Closing elevated applications may require one additional UAC prompt.
- The helper derives the named-pipe server PID from Windows, then verifies the
  same user and interactive Session, exact canonical sibling main executable,
  release identity, protocol version, and one-time nonce before accepting typed
  work.
- Each elevated launch is acknowledged only after FormationLapCore journals its
  stable Process identity. If acknowledgement is lost, the helper compensates
  by stopping the untracked Process.
- The helper exits after the requested batch.
- No persistent privileged service is installed.

Signed Beta and Stable builds require successful WinVerifyTrust validation and
the same approved signer certificate for the main executable and helper.
Unsigned `v0.x` technical previews instead verify a release-generated,
release-identity-key-signed manifest containing both executable SHA-256 values,
version, protocol version, and release channel. Windows still identifies the
preview helper as an unknown publisher. The exception never applies to a signed
Beta or Stable release.

## Updates

### Formation Lap

- New installations default automatic checks off; an explicitly saved existing
  `true` remains enabled.
- When enabled, check at most once per day.
- Manual Check Now is always available and consents to that check.
- Show a non-blocking notification.
- Cancel and await any in-flight check before a Session becomes Active.
- Never install during a Session or start a Session during installation.
- Support Stable and opt-in Beta channels.
- Verify Tauri update signatures before installation.

The updater signature is required independently of Windows Authenticode.
Unsigned `v0.x` technical previews may therefore update only to another
Tauri-signed official prerelease and must retain their unsigned-preview
disclosure.

Formation Lap updater downloads are constrained before fetch to HTTPS, the
exact `Williem3/FormationLap` repository, expected tag/version/architecture/file
name, controlled redirect hosts, and bounded metadata and installer sizes.
First-run and Settings copy names every contacted provider, and privacy status
states `Local data · Online checks on/off`.

### Other applications

Formation Lap only detects and reports updates. It never installs a third-party
update.

Update Providers may use:

- Curated GitHub Releases.
- A configured Winget package ID.
- An official update page.

An application with no reliable provider shows Unknown rather than a guessed
result. Update requests go directly to their providers; Formation Lap has no
central inventory service.

## Storage, diagnostics, and privacy

- All profiles, settings, overrides, journals, logs, and backups remain local.
- State uses versioned, human-readable JSON with atomic replacement.
- Recoverable backups protect against corruption and failed migrations.
- New storage lives under `%LOCALAPPDATA%`. On the first upgraded launch, a
  roaming `%APPDATA%` store is copied through a validated temporary local
  directory only when local storage is empty, atomically activated, and left in
  place as a recoverable backup. Conflicting stores are never merged silently.
- Profile filesystem operations use the trusted source path retained by
  ProfileLibrary. IDs must be UUIDs matching their filenames before save or
  delete; invalid legacy files are backed up and repaired into new UUID-backed
  profiles.
- Start-with-Windows uses a namespaced HKCU Run value. Migration and uninstall
  touch an older value only when it exactly identifies the current executable.
- Diagnostic logs are bounded and export only when requested.
- Formation Lap has no account, analytics, cloud storage, or usage telemetry.

## Interface

The approved design is defined by
[`design/UI_SYSTEM.md`](design/UI_SYSTEM.md) and its concept images.

Key interface rules:

- Apple-inspired utility content inside native Windows window chrome.
- Profile sidebar, focused dashboard, separate profile editor, and grouped
  settings.
- Formation Rail is the single expressive racing-specific device.
- System, Light, and Dark themes.
- Status uses icon and text, never color alone.
- Complete keyboard access, visible focus, reduced motion, and Windows scaling
  support.
- Local executable and Steam metadata supply icons; Formation Lap does not
  redistribute game artwork.

## Distribution and trust

- Open source under the MIT license.
- Official source and releases on GitHub.
- Per-user Windows installer for version one.
- Authenticode-signed application, helper, and installer before a version-one
  Beta or Stable public release.
- Tauri-signed update bundles.
- SHA-256 checksums, SBOM, dependency-license report, and build provenance for
  every tagged release.
- SignPath Foundation is the preferred open-source signing path; Azure Artifact
  Signing is the fallback.

### Pre-version-one technical previews

Formation Lap may publish an unsigned-Authenticode Windows installer only as a
`v0.x` GitHub prerelease for technical evaluation. This is an explicit,
temporary distribution tier rather than a Stable release:

- Publication uses a separate manually dispatched workflow; pushing a tag does
  not publish a preview.
- The release title, notes, and `UNSIGNED-PREVIEW.txt` state that the
  application, one-shot helper, and installer have no Windows publisher
  signature and may trigger SmartScreen or unknown-publisher warnings.
- The installer still has a Tauri updater signature. Checksums, SPDX SBOM,
  dependency licenses/notices, and GitHub build provenance remain mandatory.
- Only matching `v0.x` tags are accepted, and previews are always GitHub
  prereleases rather than the latest Stable release.
- The signed release workflow remains fail-closed. No unsigned preview may be
  relabeled, promoted, or reused as version one.

This exception does not change the version-one requirement: the application,
helper, installer, and update bundle must be signed and qualified before the
Stable `v1.0.0` tag.

## Explicitly out of scope for version one

- Multiple simultaneous Sessions.
- Multiple managed instances of the same profile entry.
- Automatic application crash recovery.
- Automatic third-party update installation.
- Arbitrary lifecycle hook or general automation scripting.
- Full-drive application scans.
- Independently downloaded Curated Catalog mutations.
- Cloud sync, accounts, analytics, or telemetry.
- Dedicated non-Steam storefront protocol integrations beyond direct
  executables.
- Portable ZIP distribution.
