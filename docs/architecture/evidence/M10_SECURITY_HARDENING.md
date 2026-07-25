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
| Profile-ID containment and legacy repair                 | In progress | Next slice                                                                                                                                                                                |
| Helper caller authentication and preview identity        | Pending     | —                                                                                                                                                                                         |
| Ordered adjacent elevation and ownership acknowledgement | Pending     | —                                                                                                                                                                                         |
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

## Publication boundary

No candidate commit was pushed, no tag was created, no workflow was dispatched,
and no GitHub release was published.
