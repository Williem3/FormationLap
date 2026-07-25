# Third-party notices

Formation Lap is distributed under the MIT license and includes open-source
dependencies under their own licenses. The release workflow resolves the
locked production dependency graph for both Rust and pnpm, enforces the
reviewed SPDX expression allowlist in
[`scripts/release/allowed-licenses.json`](scripts/release/allowed-licenses.json),
and publishes:

- `THIRD-PARTY-LICENSES.json`, a machine-readable dependency and license
  inventory; and
- `THIRD-PARTY-NOTICES.txt`, a readable inventory included with each release.

Every official release also includes a Cyclone-free SPDX 2.3 SBOM named
`Formation-Lap_<version>.spdx.json`. The generated files for a particular
version are authoritative because they reflect that tag's lockfiles.

Formation Lap uses operating-system-supplied system fonts and does not bundle
third-party font packages, game artwork, third-party application installers, or
vendor-owned branding.
