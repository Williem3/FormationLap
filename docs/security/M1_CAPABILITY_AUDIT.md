# M1 capability audit

Date: 2026-07-23

## Result

The M1 shell exposes one local, typed application command and no generic
WebView capability.

| Boundary                    | M1 configuration                                                                                   |
| --------------------------- | -------------------------------------------------------------------------------------------------- |
| Window                      | One local window labeled `main`; native decorations enabled                                        |
| Frontend content            | Bundled `frontendDist`; loopback Vite URL only in development                                      |
| Built-in/plugin permissions | Empty list                                                                                         |
| Application commands        | `get_app_snapshot` only                                                                            |
| Navigation                  | Rust guard allows the bundled Tauri origin and loopback development origin; rejects remote origins |
| Content Security Policy     | Production `connect-src` permits only Tauri IPC; `devCsp` adds only Vite loopback HTTP/WebSocket   |
| Generic plugins             | No shell, filesystem, process, opener, upload, WebSocket, or HTTP plugin dependency                |
| Installer privilege         | NSIS `currentUser`; the main binary has no administrator manifest                                  |

The empty built-in permission list is intentional. Tauri's `core:default`
preset also enables menu, path, tray, webview, and window commands that the M1
frontend does not need.

## Verification

Run:

```powershell
pnpm.cmd security:audit
cargo test --manifest-path src-tauri/Cargo.toml
```

The JavaScript audit parses the capability, Tauri configuration, package
dependencies, and registered invoke handler. It rejects production loopback
network access and verifies the Vite exceptions are isolated to `devCsp`. The
Rust tests prove that a remote navigation URL is rejected while the bundled
Windows origin is accepted.

Future milestones must extend the command list narrowly and repeat this audit.
Adding a generic plugin or remote IPC access is not an M1-compatible change.
