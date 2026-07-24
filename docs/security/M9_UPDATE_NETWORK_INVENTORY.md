# M9 update network inventory

Date: 2026-07-24

## Result

Update checks originate only in the native Rust host or the exact Winget
process adapter. The WebView retains zero generic HTTP, process, filesystem,
shell, or updater permissions. Formation Lap has no centralized application
inventory service, and third-party providers expose no download or install
operation.

All automatic and explicit update work is suppressed while race-safe behavior
applies. A result already in flight is held until the Session returns to Idle.

## Destinations

| Purpose                       | Destination                                                                                  | Boundary                                                                      |
| ----------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Stable Formation Lap metadata | `https://github.com/Williem3/FormationLap/releases/latest/download/latest.json`              | Fixed official repository; signed metadata only                               |
| Beta discovery                | `https://api.github.com/repos/Williem3/FormationLap/releases?per_page=20`                    | 30-second timeout, 256 KiB maximum, published non-draft prereleases only      |
| Beta Formation Lap metadata   | `https://github.com/Williem3/FormationLap/releases/download/<tag>/latest.json`               | Exactly one `latest.json` asset from the official API response                |
| Formation Lap bundle          | URL authenticated by the signed updater metadata                                             | Explicit install intent while Idle; Tauri verifies bytes before install       |
| LMUFFB advice                 | `https://api.github.com/repos/coasting-nc/LMUFFB/releases/latest`                            | Metadata only; no release asset requests                                      |
| Trading Paints advice         | `winget.exe list --id Rhinode.TradingPaints --exact --source winget --disable-interactivity` | Exact package query without a shell; Winget owns its Microsoft source traffic |
| SimHub advice                 | `https://www.simhubdash.com/download-2/`                                                     | Exact official page; redirects remain on the curated HTTPS host               |

Direct provider HTTP uses HTTPS-only clients, no automatic redirects, a
15-second timeout, at most three manually validated redirects, and a 1 MiB
response limit. GitHub requests send the documented media type, API version,
and a Formation Lap user agent. Provider errors, throttling, localization,
ambiguous versions, changed markup, and unavailable local file versions all
produce Unknown.

## Dependency capability

M9 adds the pinned `reqwest` blocking HTTPS client and
`tauri-plugin-updater`. `reqwest` performs only the allowlisted metadata reads
above. The updater plugin is initialized in the native host and receives only
the official Formation Lap endpoints and the compile-time public key; its
WebView permission is deliberately absent.

The pinned `base64` and `minisign-verify` crates validate the configured public
key and signature-metadata shape and provide deterministic rejection fixtures.
The official Tauri updater remains responsible for cryptographic bundle
verification before installation.

## Verification

- The capability audit asserts the complete narrow command allowlist and zero
  WebView permissions.
- Provider contract tests prove exact curated identities and fail-closed
  Current, Update Available, and Unknown parsing.
- Beta tests reject drafts, full releases, duplicate metadata assets, and
  foreign repositories.
- Signed-update tests reject missing metadata and tampered bundles before the
  installer stage.
- FormationLapCore tests prove once-daily scheduling, opt-out, race-safe
  suppression, deferred visibility, and an Idle-only installation gate.
