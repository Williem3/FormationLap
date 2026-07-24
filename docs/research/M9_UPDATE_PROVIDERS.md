# M9 Update Providers Research

**Research date:** 2026-07-24
**Scope:** Formation Lap self-update and notification-only update advice for
recognized Supporting Applications.

## Executive recommendation

Use the official Tauri v2 updater only for Formation Lap itself, behind narrow
Rust commands owned by `FormationLapCore`. Do not grant the WebView the
`updater:default` capability: that permission includes check, download, install,
and combined download-and-install operations
([Tauri updater permissions](https://v2.tauri.app/plugin/updater/#permissions)).

Ship M9 with three curated third-party provider records:

| Supporting Application | Provider | Curated identity |
| --- | --- | --- |
| LMUFFB | GitHub Releases | `coasting-nc/LMUFFB`, stable releases only |
| Trading Paints | Winget | `Rhinode.TradingPaints`, exact match in the `winget` source |
| SimHub | Official page | `https://www.simhubdash.com/download-2/` |

Leave every other recognized Supporting Application at `Unknown` until an
equally defensible provider identity and extraction contract is added in a
signed Formation Lap release. Third-party providers report only; they never
download or install software.

The official Formation Lap repository is `Williem3/FormationLap`. M9 uses that
coordinate for both channels and accepts the bounded GitHub prerelease
discovery shape described below.

## 1. Tauri v2 updater facts

### Required configuration and release output

Tauri v2 updater signing is mandatory and cannot be disabled. The application
contains the public key, while the private key signs update artifacts; losing
the private key prevents existing installations from accepting future updates
([Tauri updater signing](https://v2.tauri.app/plugin/updater/#signing-updates)).
The `plugins.updater.pubkey` value is the public-key **content**, not a path
([Tauri updater configuration](https://v2.tauri.app/plugin/updater/#configuration)).

Set `bundle.createUpdaterArtifacts` to `true`. On Windows, Tauri produces the
NSIS setup executable and its `.sig` file (and, when MSI is enabled, the MSI and
its `.sig`)
([Tauri updater artifacts](https://v2.tauri.app/plugin/updater/#update-artifacts)).
The `v1Compatible` mode exists to migrate Tauri v1 installations and is not
needed for a new Tauri v2 application
([Tauri v1-compatible artifacts](https://v2.tauri.app/plugin/updater/#updater-artifacts-for-older-tauri-versions)).

Release builds receive `TAURI_SIGNING_PRIVATE_KEY` and, when applicable,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; Tauri explicitly notes that a `.env` file
does not provide these build variables
([Tauri updater build variables](https://v2.tauri.app/plugin/updater/#signing-updates)).
These secrets belong in the release workflow, never in the repository.

The official `tauri-apps/tauri-action` can create a GitHub Release, upload the
bundles and signatures, and upload updater JSON when an updater is configured.
Its `uploadUpdaterJson` input defaults to `true` when updater artifacts are
enabled
([tauri-action inputs](https://github.com/tauri-apps/tauri-action#inputs)).
When the action is not given an existing release ID or an explicit tag, its
generated JSON uses GitHub's `releases/latest/download` asset URLs; the action
also warns that every release which can become “latest” must contain updater
artifacts
([tauri-action updater JSON behavior](https://github.com/tauri-apps/tauri-action#updater-json)).

### Metadata contract and verification point

A static updater response requires a top-level `version` and a matching
`platforms.<target>.url` and `platforms.<target>.signature`. The signature value
is the **contents** of the `.sig` file; a signature URL or path is not accepted
([Tauri static JSON format](https://v2.tauri.app/plugin/updater/#static-json-file)).
A dynamic server returns `204 No Content` for no update or `200 OK` with
`version`, `url`, and `signature`
([Tauri dynamic server format](https://v2.tauri.app/plugin/updater/#dynamic-update-server)).

The updater's `check()` request substitutes configured endpoint variables,
performs a JSON request, treats 204 as no update, deserializes the metadata, and
by default compares whether the offered version is greater than the current
version
([official updater implementation](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs)).
Consequently:

- absent or structurally invalid signature metadata fails during `check()`;
- a well-formed but cryptographically invalid signature is rejected after the
  bundle is downloaded, before the installer is returned or run.

The second behavior follows from the official implementation: `download()`
buffers the bundle and calls signature verification before returning it, while
`download_and_install()` downloads first and installs only the verified result
([official updater implementation](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs)).
M9 tests should cover those two failure phases separately and prove that neither
can invoke installation.

### Host-side API and Session safety

The Rust plugin exposes `app.updater()?.check().await?`, followed by
`download_and_install(...)`
([Tauri Rust updater API](https://v2.tauri.app/plugin/updater/#checking-for-updates)).
Tauri also documents a Rust-only design that stores the pending `Update` in
host state and exposes narrow commands for a later install
([Tauri Rust-only updater example](https://v2.tauri.app/plugin/updater/#rust-only)).
That is the appropriate Formation Lap shape.

Endpoint selection may be changed at runtime through the updater builder, so
Stable and Beta channels do not require WebView network access
([Tauri runtime configuration](https://v2.tauri.app/plugin/updater/#runtime-configuration)).
On Windows, running the updater installer causes the application to exit
automatically
([Tauri Windows behavior](https://v2.tauri.app/plugin/updater/#windows)).
Formation Lap must therefore re-check that no Active Session exists immediately
before `download_and_install`, not only when the update was discovered. A
Session can begin between those two user actions.

## 2. Formation Lap self-update channels

### Stable

The signed Stable metadata endpoint is:

```text
GET https://github.com/Williem3/FormationLap/releases/latest/download/latest.json
```

GitHub documents the `releases/latest/download/<asset-name>` link form
([GitHub link to the latest release asset](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases#linking-to-the-latest-release)).
Its “latest release” API deliberately excludes drafts and prereleases
([GitHub latest release API](https://docs.github.com/en/rest/releases/releases#get-the-latest-release)).
That makes this endpoint suitable for Stable only.

### Beta

The Stable URL cannot discover prereleases. To keep GitHub Releases as the
official distribution path established by ADR-0005, use bounded host-side
discovery:

```text
GET https://api.github.com/repos/<OWNER>/<REPO>/releases?per_page=<BOUNDED>
GET https://github.com/<OWNER>/<REPO>/releases/download/<TAG>/latest.json
```

Select the newest published, non-draft release whose `prerelease` field is
`true`, and require exactly one asset named `latest.json`. GitHub's release
schema exposes the draft/prerelease flags and each asset's
`browser_download_url`
([GitHub Releases REST API](https://docs.github.com/en/rest/releases/releases#list-releases)).
Pass the selected metadata URL to the runtime Tauri updater builder; Tauri
remains responsible for version selection and signature verification.

This Beta discovery is the accepted M9 interface. A separately hosted static
Beta feed was rejected because it adds another origin and release pipeline.

For GitHub API requests, send `Accept: application/vnd.github+json`, a current
`X-GitHub-Api-Version`, and a meaningful `User-Agent`; GitHub requires a valid
User-Agent
([GitHub REST getting started](https://docs.github.com/en/rest/using-the-rest-api/getting-started-with-the-rest-api)).
Unauthenticated public-repository requests are limited to 60 per hour, which is
compatible with a once-daily check over this small catalog but still requires
timeouts and graceful `Unknown` results
([GitHub REST rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api#primary-rate-limit-for-unauthenticated-users)).

## 3. Direct network destinations

All requests originate in Rust or in the Winget process adapter. The WebView
gets no generic HTTP capability. Redirects must remain HTTPS, have a bounded
count, and never cause third-party installer downloads.

| Purpose | Direct request | When allowed | Notes |
| --- | --- | --- | --- |
| Stable self metadata | `GET https://github.com/Williem3/FormationLap/releases/latest/download/latest.json` | Daily/explicit check; never during race-safe suppression | Official Stable feed. |
| Beta discovery | `GET https://api.github.com/repos/Williem3/FormationLap/releases?per_page=20` | Opt-in Beta check; never during race-safe suppression | Host selects a published prerelease and its exact `latest.json` asset. |
| Beta metadata | `GET https://github.com/Williem3/FormationLap/releases/download/<TAG>/latest.json` | After Beta discovery | Metadata still passes through the Tauri updater parser. |
| Formation Lap bundle | The exact HTTPS `url` in signed updater metadata, expected to be `https://github.com/Williem3/FormationLap/releases/download/<TAG>/<BUNDLE>` | Only after explicit install intent and a fresh no-Active-Session check | Tauri verifies the downloaded bytes before install. GitHub asset delivery can return `200` or redirect; clients must handle both ([GitHub release assets API](https://docs.github.com/en/rest/releases/assets#get-a-release-asset)). The final CDN host is not a stable documented contract. |
| LMUFFB advice | `GET https://api.github.com/repos/coasting-nc/LMUFFB/releases/latest` | Scheduled/explicit advice check; never during race-safe suppression | Metadata only; no asset request. |
| Winget source refresh | `GET/HEAD https://cdn.winget.microsoft.com/cache/source2.msix`; fallback `https://cdn.winget.microsoft.com/cache/source.msix` | Only as required by the `winget` adapter | The default source is `Microsoft.PreIndexed.Package`, not a simple per-package REST service ([Winget sources](https://learn.microsoft.com/en-us/windows/package-manager/winget/source)). The filenames are defined by the official client ([Winget pre-indexed source implementation](https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerRepositoryCore/Microsoft/PreIndexedPackageSourceFactory.cpp)). |
| Winget package metadata | A provider path under `https://cdn.winget.microsoft.com/cache/`, selected by Winget's downloaded index | Only through `winget.exe`; metadata only | The per-package manifest path is index data, not a stable public API. Do not reproduce the pre-indexed protocol in Formation Lap. |
| SimHub advice | `GET https://www.simhubdash.com/download-2/` | Scheduled/explicit advice check; never during race-safe suppression | Exact page only; never follow a redirect to an unallowlisted host. |

GitHub's documented release-asset response can be direct or a redirect, so a
hard-coded CDN hostname would be brittle
([GitHub release assets API](https://docs.github.com/en/rest/releases/assets#get-a-release-asset)).
The defensible network contract is the configured `github.com`/`api.github.com`
origin plus constrained HTTPS redirects for Formation Lap's own signed bundle.
There is no Formation Lap inventory server.

## 4. Curated third-party provider records

### LMUFFB — GitHub Releases

```text
provider: github-releases
owner: coasting-nc
repository: LMUFFB
channel: stable
endpoint: https://api.github.com/repos/coasting-nc/LMUFFB/releases/latest
```

The catalog identity matches LMUFFB's first-party repository
([LMUFFB repository](https://github.com/coasting-nc/LMUFFB)).
The repository describes Standard Releases as the recommended distribution
([LMUFFB README](https://github.com/coasting-nc/LMUFFB#readme)).
GitHub's latest-release endpoint returns the most recent published full release,
excluding drafts and prereleases
([GitHub latest release API](https://docs.github.com/en/rest/releases/releases#get-the-latest-release)).

Read only `tag_name` and `html_url`. Strip at most one leading `v`, then compare
the normalized value with the locally discovered version. If the local version
is absent, either value cannot be parsed, the response is ambiguous, or GitHub
fails or throttles the request, report `Unknown`. Never enumerate or download
LMUFFB assets.

### Trading Paints — Winget

```text
provider: winget
packageId: Rhinode.TradingPaints
source: winget
match: exact
```

Microsoft's official community manifest identifies
`Rhinode.TradingPaints` version `2.0.37`
([version manifest](https://github.com/microsoft/winget-pkgs/blob/master/manifests/r/Rhinode/TradingPaints/2.0.37/Rhinode.TradingPaints.yaml)).
Its locale manifest names the publisher `Rhinode LLC`, the product
`Trading Paints`, and the first-party site `https://www.tradingpaints.com`
([locale manifest](https://github.com/microsoft/winget-pkgs/blob/master/manifests/r/Rhinode/TradingPaints/2.0.37/Rhinode.TradingPaints.locale.en-US.yaml)).
The corresponding installer manifest points to a versioned installer on that
same first-party host
([installer manifest](https://github.com/microsoft/winget-pkgs/blob/master/manifests/r/Rhinode/TradingPaints/2.0.37/Rhinode.TradingPaints.installer.yaml)).
Formation Lap must not fetch that installer.

Invoke Winget without a shell command string:

```text
winget.exe list --id Rhinode.TradingPaints --exact --source winget --disable-interactivity
```

`winget list` reports installed packages and their available upgrades and
supports exact ID and source filters
([winget list documentation](https://learn.microsoft.com/en-us/windows/package-manager/winget/list)).
Package versions are authored by each package publisher; the manifest schema
does not promise Semantic Versioning
([Winget manifest documentation](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest)).
Prefer Winget's own installed/available result instead of imposing a SemVer
ordering.

Do not silently pass `--accept-source-agreements`; Microsoft documents that it
accepts source agreements on the user's behalf
([Winget source agreements](https://learn.microsoft.com/en-us/windows/package-manager/winget/source)).
If Winget is absent, the source needs agreement or repair, output is localized
or ambiguous, or the installed/available fields cannot be established safely,
return `Unknown`.

### SimHub — official update page

```text
provider: official-page
url: https://www.simhubdash.com/download-2/
allowedRedirectHosts: [www.simhubdash.com]
```

The first-party download page currently exposes a heading in the form
`Download SimHub v<version>` and a matching newest changelog version
([SimHub download page](https://www.simhubdash.com/download-2/)).
Fetch only that page. Require one visible download-heading version and the same
version at the start of the current changelog; compare the numeric dotted value
with the locally discovered SimHub version. Any redirect outside the allowlist,
missing or multiple match, disagreement, markup change, timeout, or absent
local version yields `Unknown`.

This adapter is intentionally conservative because an HTML page is not a
version API. Its extraction rule is curated code shipped with Formation Lap;
the page cannot remotely supply selectors, commands, or additional URLs.

## 5. Implementation and verification consequences

1. Keep self-update metadata/check/install in a deep host module. Expose only
   typed intents and snapshots through narrow Tauri commands.
2. Do not grant `updater:default` to the WebView. That permission includes
   installation operations
   ([Tauri updater permissions](https://v2.tauri.app/plugin/updater/#permissions)).
3. Store a checked Formation Lap `Update` only in host memory. Revalidate the
   selected channel and absence of an Active Session immediately before
   download/install.
4. Test missing signature metadata at check time and an invalid cryptographic
   signature at download time. Assert both paths leave installation untouched.
5. Test that Stable cannot select a prerelease and that Beta cannot select a
   draft or a full release.
6. Give every network operation a timeout and map provider failures,
   throttling, parsing changes, and missing local versions to `Unknown`.
7. Ensure third-party provider types expose no download/install method. Their
   output is limited to current version, available version, status, and a
   curated information/release link.
8. Persist only local check timestamps and results. Provider requests go
   directly to the origins listed above; no centralized inventory is sent.
9. Defer scheduled work while race-safe behavior suppresses network activity,
   then run it after the Primary Sim exits. An explicit check does not override
   the no-network-during-Active-Session policy.

## Remaining uncertainties

- **GitHub download redirects:** GitHub documents redirect behavior but not a
  stable final CDN hostname. The installer path must tolerate constrained HTTPS
  redirects for Formation Lap's signed artifact.
- **Winget output stability:** `winget list` is the supported user surface, but
  its human-readable output and source readiness vary by client and locale.
  Parsing must fail closed to `Unknown`; a future structured Winget interface
  should replace it if Microsoft exposes one suitable for this use.
- **Official-page fragility:** SimHub's HTML can change without notice.
  Extraction failures are normal `Unknown` outcomes, not permission to broaden
  scraping or download links.
