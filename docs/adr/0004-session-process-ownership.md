---
status: accepted
---

# Automatic cleanup requires session ownership

Formation Lap automatically closes only processes started by the Active
Session, identified by more than a reusable process ID. Matching processes that
predate the Session remain untouched unless the user invokes an explicit
per-application action, preventing session cleanup from disrupting unrelated
work.

The Windows ProcessRuntime adapter opens a PID once with all rights required by
the requested action, reads creation time and canonical executable path from
that handle, compares the complete expected identity, and holds the same handle
through observation, waiting, graceful action, or termination. This private
`VerifiedProcessHandle` deepens the adapter without changing its public
interface.

Direct recipes use their canonical source path. Launcher and Steam recipes may
store an optional canonical monitored executable path. Filename-only matches
remain observable for compatibility but cannot become Session-owned or be
stopped automatically. Test Game Launch learns a candidate path for explicit
user confirmation.
