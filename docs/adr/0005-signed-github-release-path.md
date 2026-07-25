---
status: accepted
---

# Publish signed releases and disclosed previews through GitHub

Official Stable and Beta builds are produced from version tags and published
through GitHub Releases with Tauri update signatures, Windows Authenticode,
checksums, an SBOM, dependency licenses, and build provenance. Independent
unsigned update channels and remotely mutable catalogs were rejected because
they weaken the trust chain for software that launches and terminates local
programs.

Formation Lap's official repository is `Williem3/FormationLap`. Stable checks
use that repository's `releases/latest/download/latest.json` asset. The opt-in
Beta channel performs a bounded GitHub Releases API query and selects a
published, non-draft prerelease with exactly one `latest.json` asset. Both
channels pass the selected metadata through Tauri's signed updater; the channel
selector cannot replace the repository or signing key.

Before version one, a separate manually dispatched workflow may publish an
existing `v0.x` tag as an unsigned-Authenticode technical preview. The
application, one-shot helper, and installer are explicitly disclosed as
unsigned in the release title, notes, and artifact set. The preview installer
still carries a Tauri updater signature and is published with checksums, an
SBOM, dependency licenses, and GitHub provenance. It is always a prerelease and
never the latest Stable release.

The preview exception exists to gather early-adopter evidence without incurring
a recurring Windows publisher-signing cost before Formation Lap has meaningful
usage. It does not relax the signed version-one decision: `v1.0.0` and later
Stable releases, and their qualifying Beta candidates, remain Authenticode- and
Tauri-signed through the fail-closed official release workflow. Relabeling or
promoting preview bytes was rejected because it would make trust depend on
release metadata rather than the artifact that users execute.
