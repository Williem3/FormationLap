# Security policy

## Supported versions

Only the latest signed Stable release and newest signed Beta prerelease receive
normal support. Before version one, the newest official `v0.x` unsigned
technical preview receives best-effort security fixes and is explicitly not a
trusted Stable release. Development builds and older releases may be useful for
diagnosis but do not receive security fixes.

## Report a vulnerability

Do not disclose a vulnerability in a public issue. Use GitHub's private
**Report a vulnerability** flow on the
[Security tab](https://github.com/Williem3/FormationLap/security) and include:

- the affected Formation Lap version and Windows version;
- the preconditions and the smallest reproducible sequence;
- the observed security impact;
- relevant sanitized logs or a proof of concept; and
- whether the report can be credited publicly.

Maintainers should acknowledge a report within seven days, keep the reporter
updated while it is assessed, and coordinate disclosure after a fix is
available. Never attach secrets, private signing material, or unrelated
personal data.

## Security model

Formation Lap has no generic WebView shell, filesystem, process, or HTTP
capability. Rust validates frontend payloads and owns Session policy. A PID
alone is not trusted as process identity. Automatic cleanup is limited to
Session-owned Processes.

Elevated work uses a typed, authenticated, one-shot helper and requires
explicit user intent; there is no persistent privileged service. Signed Beta
and Stable artifacts use the same Authenticode identity for the application,
helper, and installer. A `v0.x` technical preview may omit Authenticode only
under the separate disclosure contract, so Windows identifies its helper as an
unknown publisher. Every public installer remains Tauri-signed, checksummed,
and accompanied by SBOM, dependency-license, and provenance evidence.

The detailed boundaries are in
[`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md), the
[capability audit](docs/security/M1_CAPABILITY_AUDIT.md), the
[elevation threat checklist](docs/security/M7_ELEVATED_HELPER_THREAT_CHECKLIST.md),
and the [update network inventory](docs/security/M9_UPDATE_NETWORK_INVENTORY.md).
