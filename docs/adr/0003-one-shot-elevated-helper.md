---
status: accepted
---

# Isolate privileged work in a one-shot helper

The main application always runs without administrator privileges. Operations
that require elevation are validated, batched into a signed one-shot Rust
helper, approved through UAC, and terminated after the batch; a persistent
service was rejected because its larger privileged attack surface is not
justified by avoiding an occasional UAC prompt.
