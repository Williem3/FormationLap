---
status: accepted
---

# Use Tauri, React, TypeScript, and Rust

Formation Lap uses Tauri 2 with a React/TypeScript interface and a Rust
application core. This keeps the developer's preferred frontend stack, uses the
system WebView2 runtime instead of bundling a browser, and puts Windows process
and privilege work in a memory-safe native language; WPF/C# and Electron were
rejected because they would add a second backend language or a larger bundled
runtime.
