---
status: accepted
---

# Keep user state in transparent local JSON

Profiles, settings, catalog overrides, and session recovery state are stored as
versioned, human-readable JSON with atomic replacement and recoverable backups.
An embedded database or cloud account would obscure simple user-owned state and
add migration, privacy, and recovery costs that this local utility does not
need.
