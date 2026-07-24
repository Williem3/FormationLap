# Security policy

## Supported versions

Only the latest signed Stable release and the newest signed Beta prerelease are
supported. Development builds and older releases may be useful for diagnosis
but do not receive security fixes.

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

Elevated work uses a typed, signed, one-shot helper and requires explicit user
intent; there is no persistent privileged service. Official updates are
published only through `Williem3/FormationLap`, Authenticode-signed, signed
again for the Tauri updater, checksummed, and accompanied by SBOM, dependency
license, and provenance evidence.

The detailed boundaries are in
[`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md), the
[capability audit](docs/security/M1_CAPABILITY_AUDIT.md), the
[elevation threat checklist](docs/security/M7_ELEVATED_HELPER_THREAT_CHECKLIST.md),
and the [update network inventory](docs/security/M9_UPDATE_NETWORK_INVENTORY.md).
