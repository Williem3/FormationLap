# Signed release procedure

Formation Lap has two public packaging paths:

- `.github/workflows/preview.yml` manually publishes an existing `v0.x` tag as
  an explicitly unsigned-Authenticode technical prerelease. It retains Tauri
  updater signing and the complete non-Authenticode supply-chain evidence.
- `.github/workflows/release.yml` publishes signed Beta and Stable candidates.
  It Authenticode-signs every shipped executable before signing the final
  installer for Tauri updates.

Both workflows build from an immutable tag, verify an exact artifact contract,
attest provenance, and publish only to the official GitHub repository. Neither
workflow creates a tag.

## No-cost technical previews

Technical previews gather early-adopter evidence without paying for a Windows
publisher certificate before version one. They are not Stable releases.

The repository must be public before configuring the protected preview
environment. Create a GitHub environment named `preview`, add a required
reviewer, and allow only `v0.*` deployment tags. Configure:

| Kind     | Name                                 | Purpose                                  |
| -------- | ------------------------------------ | ---------------------------------------- |
| Variable | `FORMATION_LAP_UPDATE_PUBLIC_KEY`    | Base64 Tauri Minisign public-key content |
| Secret   | `TAURI_SIGNING_PRIVATE_KEY`          | Tauri Minisign private-key content       |
| Secret   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Private-key password, when configured    |

Generate the updater key in an isolated, backed-up operator environment. Keep
the encrypted private key offline and in the environment secret; do not place
it in the repository or release artifacts. The same trust root should be used
for later signed releases unless an explicit rotation plan is shipped first.

Prepare and publish a preview:

1. Choose a `0.x` SemVer such as `0.9.0-preview.1`.
2. Synchronize `package.json`, `src-tauri/Cargo.toml`,
   `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
3. Add `docs/releases/<version>.md`. Its heading and first paragraph must state
   **unsigned technical preview** and explain the SmartScreen and
   unknown-publisher warnings.
4. Run the complete candidate verification commands below.
5. Review and approve the exact commit, then create and push its lightweight
   `v0.x` tag. Pushing the tag does not publish anything.
6. Manually dispatch **Unsigned Windows technical preview** with that existing
   tag and approve the `preview` environment deployment.
7. Verify the checksum, Tauri signature, SBOM, licenses, disclosure, and GitHub
   provenance. Confirm that the GitHub release is a prerelease and is not
   marked latest.

The preview artifact set is:

- `Formation-Lap_<version>_x64-setup.exe`
- `Formation-Lap_<version>_x64-setup.exe.sig`
- `latest.json`
- `SHA256SUMS.txt`
- `UNSIGNED-PREVIEW.txt`
- `Formation-Lap_<version>.spdx.json`
- `THIRD-PARTY-LICENSES.json`
- `THIRD-PARTY-NOTICES.txt`
- GitHub build-provenance attestation

The workflow rejects a `v1` tag, Stable publication, automatic tag-triggered
publication, missing updater signature, missing disclosure, or unexpected
asset. Do not upload `AUTHENTICODE.txt` or imply that the preview has a Windows
publisher signature.

## One-time signed-release environment setup

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

## Signed candidate preparation

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

## Signed publication order

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

## Required signed-release assets

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
