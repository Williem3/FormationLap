# Formation Lap development

## Supported environment

| Tool                             | Version source                                          |
| -------------------------------- | ------------------------------------------------------- |
| Rust                             | `rust-toolchain.toml` (`1.97.1`, MSVC, rustfmt, Clippy) |
| pnpm                             | `package.json` (`10.33.0`)                              |
| JavaScript and Rust dependencies | `pnpm-lock.yaml` and `src-tauri/Cargo.lock`             |

Node.js 24 and 25 satisfy the package engine range. Windows development also
requires the Microsoft C++ Build Tools with Desktop development with C++ and
the Microsoft Edge WebView2 runtime. The supported product matrix is 64-bit
Windows 10 22H2 and Windows 11.

## Clean checkout

```powershell
git clone https://github.com/Williem3/FormationLap.git
Set-Location FormationLap
corepack enable
corepack prepare pnpm@10.33.0 --activate
pnpm.cmd install --frozen-lockfile
pnpm.cmd verify
```

`node-linker=hoisted` in `.npmrc` keeps installation compatible with exFAT
workspaces. PowerShell may block `.ps1` package-manager shims; use `pnpm.cmd`.

## Commands

| Command                                                                                         | Purpose                                                         |
| ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `pnpm.cmd dev`                                                                                  | Start only the Vite frontend on loopback                        |
| `pnpm.cmd tauri dev`                                                                            | Build and open the native Formation Lap window                  |
| `pnpm.cmd format`                                                                               | Check source, test, workflow, JSON, and Markdown formatting     |
| `pnpm.cmd lint`                                                                                 | Run ESLint                                                      |
| `pnpm.cmd typecheck`                                                                            | Run strict TypeScript checks                                    |
| `pnpm.cmd test`                                                                                 | Run React and NativeBridge behavior tests                       |
| `pnpm.cmd test:release`                                                                         | Run release/version/license/workflow contract tests             |
| `pnpm.cmd release:version:check`                                                                | Check synchronized package, Cargo, lockfile, and Tauri versions |
| `pnpm.cmd contracts:generate`                                                                   | Regenerate TypeScript from Rust contracts                       |
| `pnpm.cmd contracts:check`                                                                      | Fail if committed bindings are stale                            |
| `pnpm.cmd catalog:check`                                                                        | Validate the signed Curated Catalog                             |
| `pnpm.cmd security:audit`                                                                       | Audit the WebView capability boundary                           |
| `pnpm.cmd build`                                                                                | Build bundled frontend assets                                   |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`                               | Check Rust formatting                                           |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | Lint every Rust target and feature                              |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features`                  | Run Rust seam and security tests                                |
| `pnpm.cmd tauri build --debug --no-bundle`                                                      | Build the unsigned native debug application                     |

`scripts/run-tauri.mjs` prepares the one-shot helper and applies
`src-tauri/tauri.sidecar.conf.json`. Direct Cargo checks use the empty sidecar
override in `.cargo/config.toml`, so they compile and test the helper target
without packaging a generated binary.

## Test-driven changes

The approved public seams are `FormationLapCore`, `ProcessRuntime`, typed Tauri
commands/generated contracts, and React behavior through `NativeBridge`. Write
an observable failing test through one of these seams before implementing each
behavior slice. Use temporary real storage and mock only OS or true external
seams.

Rust contracts are authoritative. Never hand-edit
`src/generated/bindings.ts`; run `pnpm.cmd contracts:generate`, inspect the
diff, and keep it in the same commit as the Rust contract.

## Native and elevated smoke tests

The ignored M7 test is the only repository test that displays UAC. Build its
fixtures, run it explicitly, and approve the launch and close prompts:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --features process-fixtures --test privileged_operations manual_uac_helper_launches_and_closes_an_elevated_window_fixture -- --ignored --exact --nocapture
```

All ordinary verification remains non-administrative and prompt-free.

## Signed updater development

Stable and Beta metadata resolves only from the official
[`Williem3/FormationLap`](https://github.com/Williem3/FormationLap) repository.
A release build must compile the Base64-encoded Tauri Minisign public-key
content:

```powershell
$env:FORMATION_LAP_UPDATE_PUBLIC_KEY = "<base64 Minisign public-key content>"
pnpm.cmd tauri build --no-bundle
```

The private key is never stored in the repository. A development build without
the public key fails closed: the self-update state is Unknown and an update
cannot be installed. See [`RELEASE.md`](RELEASE.md) for key generation,
GitHub/Azure configuration, artifact verification, and channel promotion.

## Visual and accessibility checks

The Vite server exposes development-only `?preview=` snapshots used by the
milestone evidence. Production builds remove the preview bridge and always use
`TauriNativeBridge`. Test key routes without a pointer, visible focus,
Windows scaling at 100%, 125%, 150%, and 200%, both themes, Windows reduced
motion, and the supported Windows versions before a release.

## CI

`.github/workflows/ci.yml` runs the full gate on Windows with immutable action
SHAs and no signing secrets. It verifies a frozen install, frontend and release
contracts, generated bindings, catalog and capability audits, production
dependency advisories/licenses, Rust formatting/Clippy/tests, and a native
debug build. Pull requests also run GitHub's dependency review.
