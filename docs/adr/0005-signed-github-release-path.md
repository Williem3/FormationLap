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
