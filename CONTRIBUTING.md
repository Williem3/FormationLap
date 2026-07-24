# Contributing

Thanks for helping Formation Lap. Start by reading [`AGENTS.md`](AGENTS.md);
its domain language, architecture guardrails, test-driven workflow, and
milestone ledger are mandatory for every change.

## Before changing code

1. Read `CONTEXT.md`, the product spec, architecture, build plan, progress
   ledger, UI system, and relevant ADRs in the order listed by `AGENTS.md`.
2. Discuss product-scope changes before implementation. Do not silently revise
   accepted behavior or security boundaries.
3. Keep Rust authoritative for contracts and Session truth. React renders
   snapshots and sends intent through `NativeBridge`.
4. Add dependencies only when their capability and maintenance trade-off are
   justified. Do not hand-edit generated bindings or the Curated Catalog
   outside the signed release process.

## Development loop

Write a failing observable test through an approved public seam, confirm why it
fails, implement the smallest behavior, and repeat. Before committing:

```powershell
pnpm.cmd install --frozen-lockfile
pnpm.cmd verify
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
pnpm.cmd tauri build --debug --no-bundle
```

Update behavior documentation and `docs/architecture/PROGRESS.md` in the same
slice. Keep commits focused and preserve unrelated worktree changes.

## Pull requests

Describe the user-visible behavior, security or privacy impact, tests run, and
durable evidence. Include screenshots for interface changes and call out any
manual Windows or UAC verification. CI must be green. Generated bindings,
lockfiles, and relevant documentation belong in the same pull request as their
source change.

Use public issues for ordinary bugs and proposals. Follow
[`SECURITY.md`](SECURITY.md) for vulnerabilities.
