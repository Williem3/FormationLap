# M10 release threat checklist

Date: 2026-07-24

Status: signed-release implementation review complete; disclosed technical
preview path added; external signed-candidate evidence pending

## Result

No unresolved high- or critical-severity threat was found in the repository
release path. Publication is fail-closed until the protected `release`
environment supplies the updater and Authenticode signing configuration.
Candidate completion still requires a signed Beta workflow run and the
supported Windows manual matrix.

| Threat                                              | Control                                                                                                                                                 | Repository evidence                                            | Status     |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- | ---------- |
| Unreviewed source is released                       | Existing tag must resolve to checked-out HEAD; protected environment approval                                                                           | `.github/workflows/release.yml`                                | Controlled |
| Dependency substitution                             | Frozen pnpm/Cargo lockfiles, production audits, reviewed license policy                                                                                 | CI/release workflows, dependency audit, and `scripts/release/` | Controlled |
| Mutable workflow action                             | Every `uses:` reference is a full commit SHA and contract-tested                                                                                        | `tests/release/workflow-contract.test.mjs`                     | Controlled |
| Private updater key disclosure                      | Private key/password are environment secrets; only public key is compiled                                                                               | Release workflow and `docs/RELEASE.md`                         | Controlled |
| Long-lived cloud credential                         | Azure authentication uses GitHub OIDC and a scoped signing profile                                                                                      | Release workflow                                               | Controlled |
| Unsigned main/helper execution                      | Both binaries are signed and immediately checked for `Valid` before bundling                                                                            | Release workflow                                               | Controlled |
| Installer contents differ from signed binaries      | Bundler consumes the signed executable and signed sidecar; installer is signed afterward                                                                | Release workflow                                               | Controlled |
| Authenticode mutates updater-signed bytes           | Tauri signing occurs only after final Authenticode signing                                                                                              | Release workflow ordering test                                 | Controlled |
| Foreign update origin                               | Metadata generator and verifier require exact `Williem3/FormationLap` release URLs                                                                      | Release artifact tests                                         | Controlled |
| Artifact tampering                                  | Exact SHA-256 manifest, Tauri signature, Authenticode verification, and GitHub provenance                                                               | Release scripts/workflow                                       | Controlled |
| Missing dependency disclosure                       | Locked production graph produces JSON licenses, notices, and SPDX SBOM                                                                                  | Release scripts/workflow                                       | Controlled |
| Unsigned binary is published                        | Only the verified `release/` allowlist is passed to `gh release create`                                                                                 | Release artifact verifier/workflow                             | Controlled |
| Compromised release is silently replaced            | Release policy forbids replacing assets or reusing tags; fix-forward only                                                                               | `docs/RELEASE.md`                                              | Controlled |
| Elevated helper bypass                              | Existing M7 typed, signed, one-shot protocol remains required                                                                                           | M7 threat checklist                                            | Controlled |
| Foreign process impersonates Formation Lap          | Helper observes the pipe server's SID, Session, stable identity, exact sibling path, and release identity before validating typed work                  | M7 threat checklist, helper boundary tests                     | Controlled |
| Preview main or helper is substituted               | A bounded Minisign manifest binds final main/helper SHA-256 values, version, protocol, and preview channel; both sides verify it before UAC work        | Release identity and workflow contract tests                   | Controlled |
| PID verification races with an action               | Windows adapter opens once, verifies creation/path from that handle, and holds it through the complete action                                           | `M10_SECURITY_HARDENING.md`, ProcessRuntime fixtures           | Controlled |
| Same-name launcher Process is mis-owned             | Canonical monitored path is required for Session ownership; filename-only matches remain Pre-existing                                                   | ProcessRuntime and FormationLapCore behavior tests             | Controlled |
| Profile document ID escapes its storage directory   | ProfileLibrary retains the trusted source path; canonical UUID IDs must match filenames and invalid legacy files are repaired with recoverable backups  | `M10_SECURITY_HARDENING.md`, ProfileLibrary tests              | Controlled |
| Elevated startup reorders saved applications        | Core reaches the saved sequence position and batches only adjacent elevated entries                                                                     | Startup ordering and privileged-operation tests                | Controlled |
| Elevated launch loses ownership over IPC            | Core journals each stable identity before acknowledgement; missing acknowledgement makes the helper stop the just-launched Process                      | M7 checklist and real helper compensation test                 | Controlled |
| Imported executable configuration runs unreviewed   | Only portable imports enter Needs Review; Session start requires native approval of all configuration and every elevated/custom-stop entry              | ProfileLibrary, command, Core, and React review tests          | Controlled |
| Roaming migration follows a linked or corrupt entry | Copy rejects links/reparse points and non-files; every JSON/JSONL document plus live Core schema is validated before atomic local activation            | Storage migration tests and `M10_SECURITY_HARDENING.md`        | Controlled |
| Local and roaming stores are silently merged        | Migration runs only when local storage is empty, activates one validated temporary copy atomically, and leaves roaming unchanged                        | Storage migration conflict test                                | Controlled |
| Startup cleanup removes an unrelated Run value      | Runtime and NSIS cleanup compare the complete current-executable command; foreign namespaced values fail closed and foreign legacy values are preserved | Startup policy tests and NSIS bundle contract                  | Controlled |
| Preview is mistaken for a signed or Stable release  | Preview accepts only `v0.x`, is manual-only, always uses `--prerelease`, and requires title/notes/artifact disclosure                                   | Preview workflow and contract tests                            | Controlled |
| Preview loses updater integrity                     | The Authenticode exception does not apply to the Tauri updater signature or embedded public key                                                         | Preview workflow and artifact verifier                         | Controlled |
| Preview omits supply-chain evidence                 | Exact allowlist still requires checksums, SPDX SBOM, dependency licenses/notices, and GitHub provenance                                                 | Preview artifact verifier and attestation                      | Controlled |
| Preview bytes are promoted to version one           | Signed workflow rebuilds from the approved tag, requires Authenticode, and rejects any missing signature                                                | Separate fail-closed release workflow                          | Controlled |
| Unsigned elevated helper surprises the user         | `UNSIGNED-PREVIEW.txt`, release notes, title, README, and troubleshooting guidance name the unknown-publisher UAC behavior                              | Preview disclosure contract                                    | Controlled |

## Approved pre-version-one exception

On 2026-07-24 the maintainer approved delaying paid Authenticode until
Formation Lap demonstrates a user base that justifies the recurring expense.
Only `v0.x` technical previews receive this exception. The separate workflow:

- has no tag-push trigger and requires an existing immutable tag;
- cannot publish Stable or use a `v1` version;
- cannot access or imply an Authenticode identity;
- retains Tauri updater signing and all other release evidence; and
- publishes an exact disclosure file inside the checksummed, attested asset
  set.

This is not evidence for the signed M10 completion gate.

## External gates not simulated

- Microsoft Artifact Signing identity/profile and its produced certificate.
- Protected GitHub `release` environment reviewers and secret values.
- A real Tauri key pair whose public key is embedded in the Beta.
- GitHub-hosted provenance for the candidate.
- Signed Beta install/update/uninstall on Windows 10 22H2 and Windows 11.

These are release-blocking evidence, not repository test doubles. Do not mark
M10 complete or create a Stable v1 tag until they are recorded.
