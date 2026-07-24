# Signed release procedure

Official Formation Lap releases are built from a clean Git tag by
`.github/workflows/release.yml`. The workflow is the only supported public
packaging path. It Authenticode-signs every shipped executable before signing
the final installer for Tauri updates, verifies the artifact contract, attests
provenance, and publishes to the official GitHub repository.

## One-time protected environment setup

Create a GitHub environment named `release` with required reviewers and restrict
deployment branches/tags to the release policy. Configure:

| Kind     | Name                                 | Purpose                                  |
| -------- | ------------------------------------ | ---------------------------------------- |
| Variable | `FORMATION_LAP_UPDATE_PUBLIC_KEY`    | Base64 Tauri Minisign public-key content |
| Variable | `AZURE_ARTIFACT_SIGNING_ENDPOINT`    | Microsoft Artifact Signing endpoint      |
| Variable | `AZURE_ARTIFACT_SIGNING_ACCOUNT`     | Artifact Signing account name            |
| Variable | `AZURE_ARTIFACT_SIGNING_PROFILE`     | Certificate profile name                 |
| Secret   | `TAURI_SIGNING_PRIVATE_KEY`          | Tauri Minisign private-key content       |
| Secret   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Private-key password, when configured    |
| Secret   | `AZURE_CLIENT_ID`                    | OIDC-enabled Azure application/client ID |
| Secret   | `AZURE_TENANT_ID`                    | Azure tenant ID                          |
| Secret   | `AZURE_SUBSCRIPTION_ID`              | Azure subscription ID                    |

Grant the GitHub OIDC identity access only to the required Artifact Signing
profile. Do not store private keys, certificates, passwords, or Azure
credentials in Git, artifacts, logs, local diagnostics, or release notes.

Generate the Tauri updater key with the pinned Tauri CLI in an isolated,
backed-up operator environment. Record the public key as the environment
variable above and keep the encrypted private key offline plus in the GitHub
environment secret. A changed updater public key requires an explicit
key-rotation release plan; old clients cannot silently trust it.

## Candidate preparation

1. Start from a clean `master` synchronized with `origin/master`.
2. Complete every prior milestone and resolve all high/critical security
   findings.
3. Choose a SemVer. Use `1.0.0-beta.N` for Beta and `1.0.0` for Stable.
4. Synchronize `package.json`, `src-tauri/Cargo.toml`,
   `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
5. Add `docs/releases/<version>.md`.
6. Run:

```powershell
pnpm.cmd install --frozen-lockfile
pnpm.cmd verify
pnpm.cmd audit --audit-level high --prod
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
cargo audit --file src-tauri/Cargo.lock
pnpm.cmd tauri build --debug --no-bundle
```

7. Review the complete diff and obtain the required release approval.
8. Create and push a lightweight version tag that identifies the approved
   commit. A prerelease suffix publishes as Beta; a release version publishes
   as latest Stable.

Do not create a Stable tag until the same source behavior has passed the signed
Beta workflow and the Windows 10/11 candidate matrix.

## Automated signing and publication order

The workflow:

1. verifies the tag and synchronized version;
2. runs the complete CI, dependency advisory, and license gates;
3. compiles the public updater key into the application;
4. builds and Authenticode-signs the main executable and one-shot helper;
5. bundles those signed binaries in a per-user branded NSIS installer;
6. Authenticode-signs and verifies the installer;
7. Tauri-signs the final Authenticode installer bytes;
8. generates dependency notices, SPDX SBOM, update metadata, and checksums;
9. verifies the exact official artifact set and URL;
10. creates a GitHub build-provenance attestation; and
11. publishes the release only after every preceding gate passes.

The workflow fails if a signing input is missing or any signature is not
`Valid`. No unsigned binary is uploaded as a public artifact.

## Required release assets

For `<version>` and tag `v<version>`:

- `Formation-Lap_<version>_x64-setup.exe`
- `Formation-Lap_<version>_x64-setup.exe.sig`
- `latest.json`
- `SHA256SUMS.txt`
- `AUTHENTICODE.txt`
- `Formation-Lap_<version>.spdx.json`
- `THIRD-PARTY-LICENSES.json`
- `THIRD-PARTY-NOTICES.txt`
- GitHub build-provenance attestation

Verify a downloaded candidate:

```powershell
Get-AuthenticodeSignature ".\Formation-Lap_<version>_x64-setup.exe" |
  Format-List Status,SignerCertificate,TimeStamperCertificate
Get-FileHash ".\Formation-Lap_<version>_x64-setup.exe" -Algorithm SHA256
gh attestation verify ".\Formation-Lap_<version>_x64-setup.exe" `
  --repo Williem3/FormationLap
```

Compare the hash to `SHA256SUMS.txt`, inspect `latest.json` for the exact
official release URL, and run `verify-release-artifacts.mjs` against the full
downloaded set.

## Beta qualification and Stable promotion

Install the signed Beta on fresh or reset 64-bit Windows 10 22H2 and Windows
11 machines. Complete
[`architecture/evidence/M10_WINDOWS_RELEASE_CHECKLIST.md`](architecture/evidence/M10_WINDOWS_RELEASE_CHECKLIST.md),
including install, launch, core Session behavior, signed update, uninstall,
keyboard-only access, contrast, 100–200% scaling, themes, and reduced motion.

Promote to Stable only when:

- the Beta workflow and all release assets are green;
- Authenticode, updater signature, checksum, SBOM, licenses, and provenance
  have been independently verified;
- the Windows matrix is complete with durable evidence;
- no unresolved high/critical threat remains; and
- product/spec/UI review has no version-one mismatch.

Stable promotion uses a new approved Stable commit/tag and reruns the entire
workflow. Never repoint or replace an existing release asset.

## Rollback

If a published release is unsafe, mark it non-latest and document the issue.
Do not mutate its assets or reuse its tag. Fix forward through a new version
that passes the complete Beta qualification. Clients continue to verify every
new installer with the embedded updater public key.
