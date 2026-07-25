---
status: accepted
---

# Isolate privileged work in a one-shot helper

The main application always runs without administrator privileges. Operations
that require elevation are validated, batched into a signed one-shot Rust
helper, approved through UAC, and terminated after the batch; a persistent
service was rejected because its larger privileged attack surface is not
justified by avoiding an occasional UAC prompt.

The helper authenticates the actual Formation Lap caller before validating
operations. It derives the named-pipe server PID from Windows and verifies the
same user and interactive Session, exact canonical sibling main executable,
release identity, protocol version, and single-use nonce. Signed Beta/Stable
builds require WinVerifyTrust and approved-signer equality; unsigned previews
use a release-identity-key-signed manifest binding main/helper hashes, version,
protocol, and channel.

Elevated entries execute at their saved sequence position. Only adjacent
elevated entries share a UAC transaction; interleaved normal entries are not
reordered. Each launched identity must be journaled and acknowledged before the
helper proceeds or exits. Lost acknowledgement causes compensating termination
of the untracked Process. A longer-lived helper released by typed barriers was
rejected for now because its protocol and privileged lifetime are materially
more complex.
