# Formation Lap

Formation Lap is a Windows utility that prepares, monitors, and closes the
applications needed for a sim-racing Session. The repository currently contains
the secure M1 project foundation: a native Tauri window, React shell, typed
Rust/TypeScript boundary, and test/CI harnesses.

The accepted product behavior is in
[`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md). Contributors must begin with
[`AGENTS.md`](AGENTS.md).

## Clean-checkout setup

Formation Lap targets 64-bit Windows 10 22H2 and Windows 11. Install:

- Microsoft C++ Build Tools with **Desktop development with C++**.
- Microsoft Edge WebView2 Runtime.
- Node.js 24 or 25.
- pnpm `10.33.0`.
- rustup. The repository automatically selects Rust `1.97.1` MSVC using
  `rust-toolchain.toml`.

From a new checkout:

```powershell
corepack enable
corepack prepare pnpm@10.33.0 --activate
pnpm.cmd install --frozen-lockfile
```

PowerShell may block package-manager `.ps1` shims. Use `pnpm.cmd` in that case.
The project uses pnpm's hoisted linker because the development drive may be
exFAT and unable to create package symlinks.

## Verify

```powershell
pnpm.cmd verify
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

`pnpm.cmd contracts:generate` is the only supported way to update
`src/generated/bindings.ts`. CI fails when the generated file is stale.

## Run and package

```powershell
pnpm.cmd tauri dev
pnpm.cmd tauri build --debug --bundles nsis
```

The second command creates a per-user debug NSIS installer. Development builds
are unsigned; public signing is a later release milestone.

See [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for command details and
[`docs/security/M1_CAPABILITY_AUDIT.md`](docs/security/M1_CAPABILITY_AUDIT.md)
for the WebView capability boundary.
