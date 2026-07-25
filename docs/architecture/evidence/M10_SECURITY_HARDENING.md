# M10 native security hardening evidence

Date: 2026-07-24

Status: in progress

The maintainer approved an additive hardening program that preserves elevated
applications, Startup Sequence behavior, existing profiles, and manual online
update checks. Publication of the local `0.9.0-preview.1` candidate remains
blocked until this program is complete and separately reviewed.

## Slice status

| Slice                                                    | Status   | Durable behavior evidence                                                                                                                                                                 |
| -------------------------------------------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Verified Process handles and monitored paths             | Complete | `launcher_style_launch_returns_the_monitored_process_identity`, `filename_only_launcher_identity_is_observed_without_session_ownership`, all 11 real Windows ProcessRuntime fixture tests |
| Profile-ID containment and legacy repair                 | Complete | `invalid_legacy_profile_identity_is_repaired_without_selecting_a_filesystem_path`; all 16 ProfileLibrary behavior tests                                                                   |
| Helper caller authentication and preview identity        | Complete | Authenticated caller boundary, three native release-identity tests, helper adversarial test, and release workflow contracts                                                               |
| Ordered adjacent elevation and ownership acknowledgement | Complete | Saved-position launch test, durable journal-before-ack test, and real helper compensation test                                                                                            |
| Imported-profile review                                  | Complete | Native review state, exact privileged-entry approval, invalidation tests, and React quarantine behavior                                                                                   |
| Local storage and startup migration                      | Complete | Three command-host migration tests, six startup policy tests, NSIS bundle contract, and a real debug NSIS build                                                                           |
| Opt-in native update coordination                        | Pending  | —                                                                                                                                                                                         |
| Signed-build equality and adversarial qualification      | Pending  | —                                                                                                                                                                                         |

## Verified Process handles and monitored paths

The public ProcessRuntime interface is unchanged. Its Windows adapter now:

- opens the PID once with every right required by the requested action;
- reads creation time and canonical executable path from that handle;
- compares the complete expected identity;
- holds the same handle through observation, graceful action, exit wait, or
  termination; and
- safely classifies a just-exited, still-open Process object without acting on
  an unverifiable replacement.

Launch Recipes accept an optional `monitoredExecutablePath`. Exact canonical
path matching rejects same-name Processes from another directory. Test Game
Launch learns the observed canonical path into the profile while keeping the
copyable diagnostic sanitized. Legacy filename-only launcher matches remain
observable but are classified as Pre-existing, so automatic cleanup cannot
target them.

### Red-green evidence

1. `launcher_style_launch_returns_the_monitored_process_identity` first failed
   because `LaunchRecipe` had no `monitoredExecutablePath`; it then passed while
   a live same-name decoy in another directory was excluded.
2. The all-feature suite exposed two red terminated-process cases where Windows
   allowed a Process handle but denied the exited image-path query. The
   verifier now checks creation time and signaled state from the same handle
   before requiring a live path. All 11 real ProcessRuntime fixture tests pass.

### Verification

- `pnpm.cmd verify`
  - formatting, lint, type checking, 28 React tests, 3 accessibility tests, 21
    release tests, synchronized candidate version, generated contracts,
    catalog validation, and capability audit passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
  - passed; exFAT hard-link fallback warnings are environmental.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features -- --test-threads=1`
  - 119 passed, 0 failed, 1 separately evidenced manual UAC test ignored.

## Profile-ID containment and legacy repair

ProfileLibrary now retains each loaded document's trusted source path
privately. Save and delete operations resolve only that retained path and
reject non-canonical UUID identifiers instead of deriving filesystem paths from
document content.

On load, a document whose ID is not a canonical UUID or does not exactly match
its filename is repaired into a fresh UUID-backed profile. The original file is
moved to `backups/<new-id>.legacy.json`, the repaired document is atomically
activated at `profiles/<new-id>.json`, and activation failure restores the
original. Existing valid profiles keep their identity and behavior.

### Red-green evidence

`invalid_legacy_profile_identity_is_repaired_without_selecting_a_filesystem_path`
first exposed the untrusted `../outside-profile` document ID through the public
snapshot. It now proves that the loaded profile receives a fresh canonical
UUID, its filename matches that UUID, the original remains recoverable, and
subsequent save/delete operations cannot alter an outside sentinel.

### Verification

- `pnpm.cmd verify`
  - formatting, lint, type checking, 28 React tests, 3 accessibility tests, 21
    release tests, synchronized contracts/catalog/capability audit passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
  - passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --test racing_profiles --all-features -- --test-threads=1`
  - all 16 ProfileLibrary behavior tests passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features -- --test-threads=1`
  - 120 passed, 0 failed, 1 separately evidenced manual UAC test ignored.

## Helper caller authentication and preview identity

The main process now verifies the exact canonical sibling main/helper pair and
the compiled release identity before creating the pipe or requesting UAC. The
helper independently derives the named-pipe server PID, observes that Process's
token SID, stable identity, executable path, and Windows interactive Session,
and compares those facts with its own identity before the typed request can be
accepted.

Debug builds retain local development elevation only for the exact sibling
binary names. The no-UAC process fixture uses a feature-gated test-only entry
point that is absent from bundled helpers.

Unsigned previews now generate a bounded
`formation-lap-release-identity.json` after the final main and helper binaries
exist. The manifest binds their SHA-256 values, package version, helper protocol
version, and `preview` channel. The existing protected Formation Lap update key
signs the canonical identity payload, both executables embed the public key,
and the manifest is installed beside them as a native resource. Sealing fails
if either executable changes between payload creation and manifest assembly.

### Red-green evidence

1. `helper_rejects_a_caller_outside_the_authenticated_release_boundary` first
   failed because the validation context had no observed Session, application
   path, or release-identity facts. It now rejects each missing fact before any
   operation.
2. The release workflow contracts first failed because no helper authorization
   manifest was generated or bundled. Preview and signed workflows now bind the
   final executable bytes after build/signing and before bundling.
3. The native manifest fixture verifies a real Tauri/Minisign signature over
   the same canonical payload emitted by the release generator, then rejects a
   changed helper hash.

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test privileged_operations --all-features -- --test-threads=1`
  - 12 passed, 1 manual UAC test ignored.
- `cargo test --manifest-path src-tauri/Cargo.toml release_identity --all-features --lib`
  - all 3 release-identity tests passed.
- `node --test tests/release/workflow-contract.test.mjs tests/release/bundle-surface.test.mjs tests/release/release-identity.test.mjs`
  - all 9 focused release contracts passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
  - passed.
- `pnpm.cmd verify`
  - formatting, lint, type checking, 28 React tests, 3 accessibility tests, 24
    release tests, synchronized contracts/catalog/capability audit passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features -- --test-threads=1`
  - 124 passed, 0 failed, 1 separately evidenced manual UAC test ignored.

## Ordered adjacent elevation and ownership acknowledgement

FormationLapCore no longer preflights every elevated application before the
Startup Sequence begins. It reaches the application's saved position first and
passes only the contiguous elevated run to the broker. An interleaved sequence
therefore preserves `Normal A -> Elevated B -> Elevated C -> Normal D ->
Elevated E`, using one prompt for B/C and a later prompt for E.

Helper protocol version 2 adds one ownership offer and acknowledgement for
every elevated launch. The helper returns the stable Process identity, Core
writes that identity to the active Session journal, and only then does the main
process acknowledge ownership. The helper compensates for a rejected, missing,
or undeliverable acknowledgement by force-stopping the just-launched Process
through its verified stable identity before exiting. It cannot continue to the
next adjacent operation until the current launch is owned.

Elevated launch operations now carry the same optional canonical monitored
executable path as ordinary launch recipes, preserving exact launcher-process
ownership rules across the privilege seam.

### Red-green evidence

1. `startup_preserves_saved_order_and_batches_only_adjacent_elevated_entries`
   first failed because all elevated entries were launched before Normal A and
   interleaved Elevated E was included in the same batch. It now proves the
   saved order and two adjacent-only batches.
2. `missing_ownership_acknowledgement_stops_the_just_launched_process` first
   failed because the helper had no acknowledgement exchange. It now withholds
   acknowledgement from the real helper and verifies the real launched fixture
   is no longer running.
3. `elevated_ownership_is_journaled_before_the_helper_is_acknowledged` inspects
   `active-session.json` inside the broker callback and proves the offered
   creation time is durable before acknowledgement returns.

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test privileged_operations --all-features -- --test-threads=1`
  - 14 passed, 1 manual UAC test ignored.
- `cargo test --manifest-path src-tauri/Cargo.toml --test session_orchestration --all-features -- --test-threads=1`
  - all 16 Session orchestration tests passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
  - passed.
- The complete `pnpm.cmd verify` gate passed across the resumed commands after
  reclaiming generated Cargo cache space: formatting, lint, type checking, 28
  React tests, 3 accessibility tests, 24 release tests, version, generated
  contracts, catalog, and capability audit.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features -- --test-threads=1`
  - 126 passed, 0 failed, 1 separately evidenced manual UAC test ignored.

## Imported-profile review

Only portable imports now enter native-owned `NeedsReview`; existing stored
profiles deserialize as `Approved` and retain their behavior. Profile summaries
expose the review state, while the editable RacingProfile payload cannot grant
itself approval.

FormationLapCore rejects Start Session while a profile needs review. The narrow
`approve_profile` command requires one complete configuration confirmation and
the exact set of application IDs whose recipes launch elevated or use a custom
stop executable. Unknown, duplicate, or missing approvals fail closed.

ProfileLibrary recomputes path diagnostics for direct executables, working
directories, monitored executables, and custom-stop executables. Missing,
relative, non-executable, or known shell-host paths remain quarantined.
Changing an approved elevated source, arguments, working directory, elevation,
or custom-stop recipe returns the complete profile to `NeedsReview`.

The Dashboard labels quarantined profiles, disables Start Session, and offers a
focused review action. The Profile editor preserves every imported value,
displays every reviewed field, requires the overall confirmation and one
checkbox per privileged entry, and submits approval through NativeBridge only
after saving the reviewed values.

### Red-green evidence

1. `newly_imported_profile_cannot_start_until_its_configuration_is_approved`
   first failed at compile time because no review state, Core error, or approval
   command existed. It now proves import quarantine, Session rejection, and
   native approval.
2. `imported_profile_quarantines_missing_secondary_executable_paths` first
   failed because only the source executable participated in path repair. It
   now proves working-directory, monitored-executable, and custom-stop paths
   prevent approval.
3. `keeps an imported profile quarantined until executable settings are
reviewed` first failed because the Dashboard had no review heading or
   disabled action. It now drives import, selection, editor confirmation, native
   approval, and the re-enabled Start Session action through NativeBridge.
4. Focused native tests prove exact elevated/custom-stop approvals and
   save-time invalidation after approved elevated arguments or custom-stop
   arguments change.

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test racing_profiles --all-features -- --test-threads=1`
  - all 20 ProfileLibrary behavior tests passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --test profile_commands --all-features -- --test-threads=1`
  - all 7 native command tests passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --test privileged_operations --all-features -- --test-threads=1`
  - 14 passed, 1 manual UAC test ignored; privileged fixtures now perform
    explicit approval.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
  - passed.
- Frontend verification passed formatting, lint, type checking, 29 React tests,
  3 accessibility tests, 24 release tests, generated contracts, catalog, and
  the twenty-seven-command capability audit.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features -- --test-threads=1`
  - 131 passed, 0 failed, 1 separately evidenced manual UAC test ignored.

## Local storage and startup migration

The production host now opens Tauri's per-user local-data path. When that path
is empty and the former roaming path has data, NativeCommandHost copies the
roaming tree to a uniquely named temporary local sibling. The copier rejects
links, Windows reparse points, and non-file entries, flushes copied files,
parses every JSON document and every JSONL record, and opens FormationLapCore
against the temporary copy for authoritative live-schema validation. Only then
does one directory rename activate the local store. Validation failure leaves
the empty local destination untouched, while a populated local store always
wins without merging; roaming data is never removed.

Start with Windows now uses the namespaced
`com.formationlap.desktop.StartWithWindows.v1` HKCU Run value. The native
adapter reads both namespaced and legacy values before acting. It refuses to
overwrite or delete a foreign namespaced value, accepts the exact current
command plus the prior canonical Windows path spelling as owned, and deletes
the legacy `Formation Lap` value only when it is an exact owned command.

The Tauri NSIS hooks apply the same comparison during uninstall. They remove
the namespaced value only for the installed executable and preserve a foreign
legacy product-name value across Tauri's built-in product-name cleanup.
Updater-driven uninstall leaves both registrations in place.

### Red-green evidence

1. `first_local_open_atomically_copies_and_validates_the_roaming_store` first
   failed because NativeCommandHost had no migration entry point. It now proves
   the local copy activates and the roaming profile remains.
2. `invalid_roaming_documents_never_activate_local_storage` places malformed
   JSON in a backup that FormationLapCore would not otherwise open and proves
   every copied document is validated before activation.
3. `populated_local_and_roaming_stores_are_never_merged` proves that a local
   profile remains authoritative and the conflicting roaming profile remains
   untouched.
4. Startup policy tests prove exact legacy migration, foreign-value
   preservation, exact disable cleanup, and compatibility with the previous
   canonical-path command spelling. The release contract requires the
   pre/post-uninstall hooks and namespaced value in every NSIS bundle.

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test profile_commands --all-features -- --test-threads=1`
  - all 10 command-host tests passed, including the three migration cases.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib desktop_host::tests --all-features -- --test-threads=1`
  - all 6 desktop-host policy and single-instance tests passed.
- `pnpm.cmd verify`
  - formatting, lint, type checking, 29 React tests, 3 accessibility tests, 25
    release tests, synchronized contracts/catalog, and capability audit passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
  - passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features -- --test-threads=1`
  - 138 passed, 0 failed, 1 separately evidenced manual UAC test ignored.
- `pnpm.cmd tauri build --debug --bundles nsis --no-sign --ci`
  - built the native application and the branded NSIS installer with the
    configured hooks.

## Opt-in, race-safe online updates

UpdateCoordinator now owns the one active provider check, its cancellation
token, the Session-start barrier, and the exclusive checked-version install
lease. The Tauri Session command cancels and joins provider work before asking
FormationLapCore to start; new check work cannot cross that barrier. Installation
acquires the native lease before the core-owned activity lease, so concurrent
Session and install commands choose one activity without an overlap window.
Failure releases both leases, while a successfully launched installer keeps the
core activity blocked until application exit.

Automatic checks now default off for a new settings document or a legacy
document without the field. A persisted explicit `true` remains true, manual
**Check now** remains available, and the UI names GitHub Releases, Winget, and
SimHub's official site. Dashboard status now reports
`Local data · Online checks on/off`.

The native updater replaced the generic updater runtime with a smaller bounded
module. It accepts only HTTPS metadata from the exact Formation Lap GitHub
repository, one exact `windows-x86_64` platform, a channel-appropriate SemVer,
and an installer URL whose tag, version, architecture, and filename all agree.
Metadata is capped at 256 KiB, installers at 128 MiB, and redirects at three
across an exact host allowlist. The complete installer bytes must pass the
embedded Minisign trust root before staging. A read-share-only file handle then
remains open from the verified write through UAC-aware ShellExecute process
creation, closing the replacement race.

### Red-green evidence

1. `automatic_update_checks_are_opt_in_and_an_explicit_true_persists` first
   failed because the default and missing-field fallback were `true`; it now
   proves new-user opt-out and explicit persisted opt-in.
2. `session_start_cancels_and_joins_the_active_check` first failed because no
   native activity owner existed; it now proves cancellation, join, and the
   barrier against new checks.
3. `install_and_session_start_are_mutually_exclusive` plus the core install
   test prove installation blocks Session start and releases safely on failure.
4. Native updater tests reject mismatched repository, tag, version,
   architecture, filename, redirect scheme/host, and signature inputs.
5. React tests first failed on the old `LOCAL ONLY` footer and unnamed provider
   copy; they now prove the persisted on/off status and explicit provider
   disclosure.

### Verification

- `pnpm.cmd verify`
  - formatting, lint, type checking, 29 React tests, 3 accessibility tests, 25
    release contracts, version/contracts/catalog checks, and the twenty-seven
    command zero-permission capability audit passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
  - passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features -- --test-threads=1`
  - 143 passed, 0 failed, 1 separately evidenced manual UAC test ignored.
- `pnpm.cmd tauri build --debug --bundles nsis --no-sign --ci`
  - built the native application and branded NSIS installer after exercising
    the build-only updater configuration with no updater runtime plugin.

## Publication boundary

No candidate commit was pushed, no tag was created, no workflow was dispatched,
and no GitHub release was published.
