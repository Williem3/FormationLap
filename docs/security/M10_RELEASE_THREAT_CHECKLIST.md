# M10 release threat checklist

Date: 2026-07-24

Status: implementation review complete; external signed-candidate evidence
pending

## Result

No unresolved high- or critical-severity threat was found in the repository
release path. Publication is fail-closed until the protected `release`
environment supplies the updater and Authenticode signing configuration.
Candidate completion still requires a signed Beta workflow run and the
supported Windows manual matrix.

| Threat                                         | Control                                                                                   | Repository evidence                                            | Status     |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------- | ---------- |
| Unreviewed source is released                  | Existing tag must resolve to checked-out HEAD; protected environment approval             | `.github/workflows/release.yml`                                | Controlled |
| Dependency substitution                        | Frozen pnpm/Cargo lockfiles, production audits, reviewed license policy                   | CI/release workflows, dependency audit, and `scripts/release/` | Controlled |
| Mutable workflow action                        | Every `uses:` reference is a full commit SHA and contract-tested                          | `tests/release/workflow-contract.test.mjs`                     | Controlled |
| Private updater key disclosure                 | Private key/password are environment secrets; only public key is compiled                 | Release workflow and `docs/RELEASE.md`                         | Controlled |
| Long-lived cloud credential                    | Azure authentication uses GitHub OIDC and a scoped signing profile                        | Release workflow                                               | Controlled |
| Unsigned main/helper execution                 | Both binaries are signed and immediately checked for `Valid` before bundling              | Release workflow                                               | Controlled |
| Installer contents differ from signed binaries | Bundler consumes the signed executable and signed sidecar; installer is signed afterward  | Release workflow                                               | Controlled |
| Authenticode mutates updater-signed bytes      | Tauri signing occurs only after final Authenticode signing                                | Release workflow ordering test                                 | Controlled |
| Foreign update origin                          | Metadata generator and verifier require exact `Williem3/FormationLap` release URLs        | Release artifact tests                                         | Controlled |
| Artifact tampering                             | Exact SHA-256 manifest, Tauri signature, Authenticode verification, and GitHub provenance | Release scripts/workflow                                       | Controlled |
| Missing dependency disclosure                  | Locked production graph produces JSON licenses, notices, and SPDX SBOM                    | Release scripts/workflow                                       | Controlled |
| Unsigned binary is published                   | Only the verified `release/` allowlist is passed to `gh release create`                   | Release artifact verifier/workflow                             | Controlled |
| Compromised release is silently replaced       | Release policy forbids replacing assets or reusing tags; fix-forward only                 | `docs/RELEASE.md`                                              | Controlled |
| Elevated helper bypass                         | Existing M7 typed, signed, one-shot protocol remains required                             | M7 threat checklist                                            | Controlled |

## External gates not simulated

- Microsoft Artifact Signing identity/profile and its produced certificate.
- Protected GitHub `release` environment reviewers and secret values.
- A real Tauri key pair whose public key is embedded in the Beta.
- GitHub-hosted provenance for the candidate.
- Signed Beta install/update/uninstall on Windows 10 22H2 and Windows 11.

These are release-blocking evidence, not repository test doubles. Do not mark
M10 complete or create a Stable v1 tag until they are recorded.
