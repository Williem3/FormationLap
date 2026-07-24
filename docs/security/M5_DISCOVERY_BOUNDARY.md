# M5 targeted discovery boundary

Date: 2026-07-23

## Result

Local discovery is limited to explicit Windows sources and signed Curated
Catalog matchers. It does not enumerate arbitrary directories or crawl a
drive.

| Source                 | Read boundary                                                                                                                                           | Match boundary                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Steam                  | The current user's Valve Steam registry key, `libraryfolders.vdf`, curated `appmanifest_<id>.acf` files, and each manifest's declared install directory | A Steam App ID in the bundled sim catalog                                  |
| Windows installed apps | `DisplayName` and `InstallLocation` under the current-user and local-machine 32-bit and 64-bit uninstall keys                                           | An exact catalog display name plus a safe catalog-relative executable path |
| Running Processes      | The Tool Help process inventory and each accessible process image path                                                                                  | An exact catalog executable filename                                       |
| Known locations        | `ProgramFiles`, `ProgramFiles(x86)`, `LOCALAPPDATA`, `ProgramData`, and `USERPROFILE`                                                                   | A catalog-selected root plus a safe catalog-relative executable path       |

All candidates must resolve to an existing local directory or file before
discovery returns them. Catalog-relative paths containing absolute roots,
parent traversal, or non-normal components are rejected. Observations remain
local and are not transmitted.

## Dependency capability

M5 enables the `Win32_System_Registry` feature on the already-pinned
`windows-sys` dependency. This permits read-only registry inspection through
the APIs above. It adds no Tauri permission, generic WebView process or
filesystem API, remote source, shell command, service, or new dependency.

## Verification

FormationLapCore tests use real temporary files and explicit Windows
observations to prove:

- separate Steam and standalone iRacing installations remain distinguishable;
- only exact curated running-process names match;
- only signed known-location paths are checked; and
- missing declared installations are omitted.

The production constructor collects those same typed observations from the
bounded Windows APIs. `pnpm.cmd security:audit` continues to verify that the
WebView has zero generic permissions.
