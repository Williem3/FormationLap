---
status: accepted
---

# Automatic cleanup requires session ownership

Formation Lap automatically closes only processes started by the Active
Session, identified by more than a reusable process ID. Matching processes that
predate the Session remain untouched unless the user invokes an explicit
per-application action, preventing session cleanup from disrupting unrelated
work.
