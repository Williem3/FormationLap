# M9 update network inventory

Date: 2026-07-24

## Result

Update checks originate only in the native Rust host or the exact Winget
process adapter. The WebView retains zero generic HTTP, process, filesystem,
shell, or updater permissions. Formation Lap has no centralized application
inventory service, and third-party providers expose no download or install
operation.

Automatic checks are off on a new installation and occur at most daily only
after explicit opt-in. Manual **Check now** consents to one check. All
automatic and explicit update work is suppressed while race-safe behavior
applies; Session start cancels and joins any active native provider tasks before
FormationLapCore may transition the Session.

## Destinations

| Purpose                       | Destination                                                                                                   | Boundary                                                                                                                                                            |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Stable Formation Lap metadata | `https://github.com/Williem3/FormationLap/releases/latest/download/latest.json`                               | Fixed official repository; signed metadata only                                                                                                                     |
| Beta discovery                | `https://api.github.com/repos/Williem3/FormationLap/releases?per_page=20`                                     | 30-second timeout, 256 KiB maximum, published non-draft prereleases only                                                                                            |
| Beta Formation Lap metadata   | `https://github.com/Williem3/FormationLap/releases/download/<tag>/latest.json`                                | Exactly one `latest.json` asset from the official API response                                                                                                      |
| Formation Lap bundle          | `https://github.com/Williem3/FormationLap/releases/download/v<version>/Formation-Lap_<version>_x64-setup.exe` | Explicit install intent while Idle; exact repository/tag/version/x64 filename, controlled HTTPS redirects, 128 MiB maximum, and Minisign verification before launch |
| LMUFFB advice                 | `https://api.github.com/repos/coasting-nc/LMUFFB/releases/latest`                                             | Metadata only; no release asset requests                                                                                                                            |
| Trading Paints advice         | `winget.exe list --id Rhinode.TradingPaints --exact --source winget --disable-interactivity`                  | Exact package query without a shell; Winget owns its Microsoft source traffic                                                                                       |
| SimHub advice                 | `https://www.simhubdash.com/download-2/`                                                                      | Exact official page; redirects remain on the curated HTTPS host                                                                                                     |

Direct provider HTTP uses HTTPS-only clients, no automatic redirects, a
15-second timeout, at most three manually validated redirects, and a 1 MiB
response limit. GitHub requests send the documented media type, API version,
and a Formation Lap user agent. Provider errors, throttling, localization,
ambiguous versions, changed markup, and unavailable local file versions all
produce Unknown.

## Dependency capability

The pinned `reqwest` blocking HTTPS client performs only the allowlisted
provider, metadata, and installer requests above. The native updater applies
HTTPS-only policy, three controlled redirects, 256 KiB metadata limits, and a
128 MiB installer limit before retaining a candidate. There is no generic
updater plugin or WebView updater permission.

The pinned `base64`, `minisign-verify`, and `semver` crates decode the embedded
release trust root, verify the complete installer bytes, and compare only valid
release versions. The verified installer is staged under a unique local
temporary directory while a read-share-only file handle prevents replacement
through UAC launch.

## Verification

- The capability audit asserts the complete narrow command allowlist and zero
  WebView permissions.
- Provider contract tests prove exact curated identities and fail-closed
  Current, Update Available, and Unknown parsing.
- Beta tests reject drafts, full releases, duplicate metadata assets, and
  foreign repositories.
- Signed-update tests reject missing metadata and tampered bundles before the
  installer stage.
- FormationLapCore and UpdateCoordinator tests prove opt-in persistence,
  once-daily scheduling, cancellation/join before Session start, and exclusive
  Session/install leases.
- Native updater tests reject foreign repository/tag/version/architecture/file
  combinations, uncontrolled redirects, invalid signatures, and ambiguous
  Beta metadata.
