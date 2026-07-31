# M7 elevated-helper threat checklist

Formation Lap remains an ordinary current-user process. The only privileged
surface is `formation-lap-elevated-helper.exe`, a separately built sidecar that
accepts one bounded request, returns one structured response, and exits.

## Protocol boundary

| Threat                                         | Control                                                                                                                                                                                                                                                        | Automated evidence                                                                                                                                               |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Protocol downgrade or incompatible caller      | Request, response, ownership offer, and acknowledgement carry protocol version `3`; mismatch is rejected before validation continues                                                                                                                           | `helper_rejects_a_protocol_version_mismatch_before_any_operation`                                                                                                |
| Replay                                         | A version-four nonce names the unique pipe, is repeated in request and response, and is consumed once by the validator; the server accepts one client and one request                                                                                          | `helper_accepts_one_complete_canonical_typed_batch_once`                                                                                                         |
| Wrong Windows user                             | The pipe DACL grants access only to the current user SID; the helper independently reads both its own token SID and the pipe server Process token SID, then compares the observed server SID with the request                                                  | `current_user_pipe_authenticates_both_local_process_ids`, `helper_rejects_the_wrong_user_and_wrong_parent_identity`                                              |
| Wrong or PID-reused parent                     | The helper reads the named-pipe server PID, reconstructs its stable Process identity, and requires exact PID, creation time, and canonical executable path equality                                                                                            | `helper_rejects_the_wrong_user_and_wrong_parent_identity`                                                                                                        |
| Caller from another interactive Session        | The helper derives both Windows Session IDs with `ProcessIdToSessionId` and fails closed unless they match                                                                                                                                                     | `helper_rejects_a_caller_outside_the_authenticated_release_boundary`                                                                                             |
| Renamed, copied, or non-sibling caller         | Both sides require the exact canonical sibling pair `formation-lap.exe` and `formation-lap-elevated-helper.exe` before privileged work                                                                                                                         | `application_pair_requires_exact_canonical_sibling_names`, helper boundary test                                                                                  |
| Unsigned-preview caller substitution           | The release workflow signs a bounded manifest containing the exact final main/helper SHA-256 values, version, protocol, and channel; both executables embed the public key and verify the manifest before accepting elevation                                  | `preview_manifest_binds_both_executable_hashes_and_build_metadata`, release identity/workflow contracts                                                          |
| Signed caller substitution                     | Beta/Stable binaries are opened once with write/delete sharing denied, hash-bound by the release-key-signed manifest, accepted by WinVerifyTrust, and required to expose the same signer-certificate SHA-256 approved in that manifest                         | `signed_manifest_requires_the_same_release_approved_authenticode_signer`, `verified_pair_handles_prevent_sibling_replacement_through_launch`, workflow contracts |
| Client spoofing                                | The main process compares the connected pipe client PID with the exact Process handle returned by `ShellExecuteExW`                                                                                                                                            | Windows pipe test plus production adapter review                                                                                                                 |
| PID reuse in a stop request                    | The helper independently observes the requested Process and requires the complete stable identity; parent PID, helper PID, and PID zero are protected                                                                                                          | `helper_rejects_a_reused_pid_and_protects_its_parent`                                                                                                            |
| Path traversal, links, or aliases              | Launch targets, custom-stop targets, working directories, and observed executable paths must already equal their filesystem-canonical form                                                                                                                     | `helper_rejects_noncanonical_and_shell_targets`                                                                                                                  |
| Arbitrary shell execution                      | The protocol is a deny-unknown-fields tagged enum with only launch, graceful-stop, and force-terminate operations. There is no command string. Known shell/script hosts are rejected, arguments remain direct arrays, and NUL/newline separators are forbidden | `helper_rejects_raw_shell_documents_and_out_of_scope_operations`, `helper_rejects_noncanonical_and_shell_targets`                                                |
| Privileged target replacement after approval   | Protected approval stores SHA-256 values for elevated launch and custom-stop targets; the helper opens and hashes the target with write/delete sharing denied, retains that handle through execution, and requires the typed expected hash                     | `replacing_approved_elevated_or_custom_stop_bytes_requarantines_the_profile`, `verified_target_denies_replacement_until_the_launch_guard_is_dropped`             |
| Resource exhaustion                            | Messages are limited to 64 KiB, batches to 32 operations, arguments to 32 entries and 16 KiB per operation, and startup timeouts to 1–300 seconds                                                                                                              | `helper_rejects_oversized_batches_and_line_bearing_arguments`                                                                                                    |
| Partial validation followed by privileged work | The complete request is validated before the executor sees the first operation                                                                                                                                                                                 | Validator structure and all adversarial tests                                                                                                                    |
| Long-running privileged service                | The helper has one pipe argument, reads one request, sends one response, and returns on every success or error. The parent waits for exit and terminates only the helper it launched if the bounded exit wait fails                                            | `one_shot_helper_exits_after_an_accepted_or_rejected_request`                                                                                                    |
| Main application accidentally elevated         | The embedded Windows manifest explicitly requests `asInvoker`; helper elevation occurs only through the fixed `runas` call                                                                                                                                     | `main_application_manifest_explicitly_remains_non_administrative`, capability audit                                                                              |
| Elevated starts preserve sequence order        | FormationLapCore reaches each saved position before launching it and batches only adjacent elevated entries, preserving the one-prompt-where-possible behavior without reordering normal applications                                                          | `startup_preserves_saved_order_and_batches_only_adjacent_elevated_entries`                                                                                       |
| IPC failure orphans an elevated launch         | The helper offers each stable Process identity; Core durably journals it before acknowledging ownership. A missing acknowledgement makes the helper stop the just-launched Process before it exits or proceeds                                                 | `elevated_ownership_is_journaled_before_the_helper_is_acknowledged`, `missing_ownership_acknowledgement_stops_the_just_launched_process`                         |
| Startup cancellation orphans a batched launch  | FormationLapCore retains every acknowledged Session-owned identity returned by the adjacent batch and requests reverse-order graceful cleanup for the current and not-yet-reached Processes                                                                    | `cancelling_startup_closes_every_process_from_the_elevated_launch_batch`                                                                                         |
| Elevated close/restart bypasses the helper     | Core routes graceful close, force termination, and relaunch through `PrivilegeBroker`; the development adapter records the typed batches                                                                                                                       | `elevated_manual_restart_routes_close_and_relaunch_through_the_broker`                                                                                           |

## Binary and distribution boundary

- The helper is a separate Cargo binary and Tauri sidecar. It is copied beside
  the main executable and receives no WebView, network, generic filesystem, or
  shell capability.
- The broker canonicalizes the installed helper path and accepts only the
  exact `formation-lap-elevated-helper.exe` file name before requesting UAC.
- Debug helpers are unsigned and accepted only by debug builds with the exact
  sibling names; the process-fixture bypass is feature-gated and is not present
  in a bundled helper.
- An approved `v0.x` technical preview is a separate public exception: its
  helper remains protected by the complete typed protocol boundary plus a
  Minisign release-identity manifest over both final executable hashes, but
  Windows displays an unknown publisher. The preview workflow and release
  assets must disclose that fact and cannot publish or promote a Stable tag.

Public version-one artifacts must complete this signature gate:

1. Build the main executable, helper, update bundle, and installer from the
   same immutable source revision.
2. Sign the main executable, helper, and installer with the same approved
   Authenticode identity (SignPath Foundation preferred; Azure Artifact
   Signing fallback).
3. Verify every PE signature with Windows trust policy before packaging,
   require identical main/helper signer-certificate SHA-256 values, and seal
   that approved value into the release-key-signed identity manifest.
4. At runtime, open the main/helper pair once with replacement denied, repeat
   WinVerifyTrust, and require both signer certificates to equal the manifest.
   Fail on an unsigned, invalid, expired-without-valid-timestamp, or unexpected
   signer.
5. Sign the Tauri update bundle with the release update key.
6. Publish SHA-256 checksums, an SBOM, dependency-license report, and build
   provenance that name the helper as its own artifact.
7. Keep all signing credentials outside the repository and unavailable to
   pull-request builds.

M10 owns execution and durable evidence for that public-artifact gate. M7
establishes the separable binary, fixed bundle path, and verification plan.
