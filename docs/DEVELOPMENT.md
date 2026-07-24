# Formation Lap development

## Supported environment

M1 is pinned by three repository files:

| Tool                             | Version source                                          |
| -------------------------------- | ------------------------------------------------------- |
| Rust                             | `rust-toolchain.toml` (`1.97.1`, MSVC, rustfmt, Clippy) |
| pnpm                             | `package.json` (`10.33.0`)                              |
| JavaScript and Rust dependencies | `pnpm-lock.yaml` and `src-tauri/Cargo.lock`             |

Node.js 24 and 25 satisfy the package engine range. Windows development also
requires the Microsoft C++ Build Tools and WebView2 runtime.

## Install

```powershell
corepack enable
corepack prepare pnpm@10.33.0 --activate
pnpm.cmd install --frozen-lockfile
```

`node-linker=hoisted` in `.npmrc` keeps installation compatible with exFAT
workspaces. Dependency versions remain exact and the lockfile remains
authoritative.

## Commands

| Command                                                                                         | Purpose                                               |
| ----------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `pnpm.cmd dev`                                                                                  | Start only the Vite frontend on loopback              |
| `pnpm.cmd tauri dev`                                                                            | Build and open the native Formation Lap window        |
| `pnpm.cmd format`                                                                               | Check frontend, script, workflow, and JSON formatting |
| `pnpm.cmd lint`                                                                                 | Run ESLint                                            |
| `pnpm.cmd typecheck`                                                                            | Run strict TypeScript project checks                  |
| `pnpm.cmd test`                                                                                 | Run React and NativeBridge behavior tests             |
| `pnpm.cmd contracts:generate`                                                                   | Regenerate TypeScript from Rust contracts             |
| `pnpm.cmd contracts:check`                                                                      | Fail if committed bindings are stale                  |
| `pnpm.cmd security:audit`                                                                       | Audit the M1 WebView capability boundary              |
| `pnpm.cmd build`                                                                                | Build bundled frontend assets                         |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`                               | Check Rust formatting                                 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | Lint Rust                                             |
| `cargo test --manifest-path src-tauri/Cargo.toml`                                               | Run Rust seam/security tests                          |
| `pnpm.cmd tauri build --debug --bundles nsis`                                                   | Build an unsigned per-user debug installer            |

`pnpm.cmd tauri dev` and `pnpm.cmd tauri build` run through
`scripts/run-tauri.mjs`. The wrapper builds the one-shot helper first and then
adds `src-tauri/tauri.sidecar.conf.json`, so the helper is copied beside the
main executable and included in bundles. Direct Cargo checks deliberately use
the empty sidecar override in `.cargo/config.toml`; they compile and test the
helper target without trying to package a generated binary.

The ignored M7 smoke test is the only test that displays UAC. Build fixtures,
run it explicitly, and approve the launch and close prompts:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --features process-fixtures --test privileged_operations manual_uac_helper_launches_and_closes_an_elevated_window_fixture -- --ignored --exact --nocapture
```

All ordinary verification remains non-administrative and prompt-free.

## Signed updater development

Formation Lap's Stable and Beta metadata is resolved only from the official
[`Williem3/FormationLap`](https://github.com/Williem3/FormationLap) GitHub
repository. The release build must compile the Tauri Minisign public-key
content as Base64:

```powershell
$env:FORMATION_LAP_UPDATE_PUBLIC_KEY = "<base64 Minisign public-key content>"
pnpm.cmd tauri build
```

The private signing key is never stored in the repository. Release automation
provides it through `TAURI_SIGNING_PRIVATE_KEY` and, when needed,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. A development build without the public
key fails closed: Formation Lap reports its own update state as Unknown and
cannot install an update.

Stable resolves
`https://github.com/Williem3/FormationLap/releases/latest/download/latest.json`.
Beta discovers a published non-draft prerelease through the bounded GitHub API
and accepts only one `latest.json` asset under the same repository.

## Native boundary

Rust owns `AppSnapshot`. The generator in
`src-tauri/src/bin/generate-bindings.rs` produces both the matching TypeScript
type and its narrow invoke wrapper. React calls it only through `NativeBridge`;
tests use `InMemoryNativeBridge`.

Do not hand-edit `src/generated/bindings.ts`. Do not import Tauri APIs outside
the production NativeBridge adapter.

## M2 visual evidence

The Vite development server exposes two development-only NativeBridge previews
for repeatable visual checks. Production builds remove this branch and always
use `TauriNativeBridge`.

| URL                                        | State                                               |
| ------------------------------------------ | --------------------------------------------------- |
| `http://127.0.0.1:1420/?preview=m2-wizard` | Empty library ready for the first-profile wizard    |
| `http://127.0.0.1:1420/?preview=m2-editor` | Complete sample profile ready to open in the editor |

Start `pnpm.cmd dev`, then use these previews for keyboard navigation,
125–200% scaling checks, and the required M2 screenshots.

## CI

`.github/workflows/ci.yml` runs on Windows with immutable action SHAs and no
signing secrets. It verifies the frozen pnpm install, frontend checks, generated
contracts, capability audit, Rust formatting/Clippy/tests, and a native debug
build.
