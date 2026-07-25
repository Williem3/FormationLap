# Privacy

Formation Lap is local-first. It has no account, cloud storage, analytics,
advertising, or usage telemetry.

## Data stored on this PC

Formation Lap stores profiles, settings, application overrides, the
active-Session recovery journal, bounded diagnostic logs, discovery results,
and rotating backups in the operating system's per-user local application-data
directory. These files can include user-chosen executable paths,
launch arguments, process names, versions, timestamps, and sanitized outcomes.

The bundled Curated Catalog is read-only. Diagnostic export happens only when
requested and produces a local, copyable report. Formation Lap does not upload
the report.

Uninstall removes the installed program. User configuration is intentionally
kept so reinstalling does not erase profiles without consent. To remove it,
close Formation Lap, uninstall the app, then delete its per-user application
local application-data directory as described in
[`docs/TROUBLESHOOTING.md`](docs/TROUBLESHOOTING.md).

## Network requests

Formation Lap can make direct HTTPS metadata requests for:

- signed Formation Lap Stable or opt-in Beta updates in the official GitHub
  repository; and
- notification-only version advice for curated third-party applications.

Automatic checks are off for new installations and run at most daily only
after the user enables them. Manual **Check now** is always available and
consents to that one check. Depending on configured applications, checks may
contact Formation Lap or curated application releases on GitHub, Microsoft
Winget sources, and SimHub's official site.

It does not send profile contents or an application inventory to a Formation
Lap server. Winget may access Microsoft sources when checking an exact curated
package. Session start cancels and joins active checks before becoming Active.
A signed Formation Lap installer is downloaded only after explicit install
intent. Formation Lap never downloads or installs a third-party application
update.

The exact destinations, timeouts, size limits, and redirect rules are recorded
in
[`docs/security/M9_UPDATE_NETWORK_INVENTORY.md`](docs/security/M9_UPDATE_NETWORK_INVENTORY.md).

## Windows and third parties

Launching a selected application or Steam recipe gives that program the same
network behavior it would have when launched directly. Windows, GitHub,
Microsoft Winget, Steam, and third-party applications operate under their own
privacy policies. Formation Lap is not affiliated with those providers.
