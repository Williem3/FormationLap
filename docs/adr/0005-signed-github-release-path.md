---
status: accepted
---

# Publish signed releases through GitHub

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
