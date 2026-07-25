# M10 native security hardening evidence

Date: 2026-07-24

Status: in progress

The maintainer approved an additive hardening program that preserves elevated
applications, Startup Sequence behavior, existing profiles, and manual online
update checks. Publication of the local `0.9.0-preview.1` candidate remains
blocked until this program is complete and separately reviewed.

## Slice status

| Slice                                                    | Status      | Durable behavior evidence                                                                                                                                                                 |
| -------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Verified Process handles and monitored paths             | Complete    | `launcher_style_launch_returns_the_monitored_process_identity`, `filename_only_launcher_identity_is_observed_without_session_ownership`, all 11 real Windows ProcessRuntime fixture tests |
| Profile-ID containment and legacy repair                 | Complete    | `invalid_legacy_profile_identity_is_repaired_without_selecting_a_filesystem_path`; all 16 ProfileLibrary behavior tests                                                                   |
| Helper caller authentication and preview identity        | Complete    | Authenticated caller boundary, three native release-identity tests, helper adversarial test, and release workflow contracts                                                               |
| Ordered adjacent elevation and ownership acknowledgement | In progress | Next slice                                                                                                                                                                                |
| Imported-profile review                                  | Pending     | —                                                                                                                                                                                         |
| Local storage and startup migration                      | Pending     | —                                                                                                                                                                                         |
| Opt-in native update coordination                        | Pending     | —                                                                                                                                                                                         |
| Signed-build equality and adversarial qualification      | Pending     | —                                                                                                                                                                                         |

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

## Publication boundary

No candidate commit was pushed, no tag was created, no workflow was dispatched,
and no GitHub release was published.
