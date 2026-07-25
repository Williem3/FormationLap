---
status: accepted
---

# Keep user state in transparent local JSON

Profiles, settings, catalog overrides, and session recovery state are stored as
versioned, human-readable JSON with atomic replacement and recoverable backups.
An embedded database or cloud account would obscure simple user-owned state and
add migration, privacy, and recovery costs that this local utility does not
need.

New state belongs under `%LOCALAPPDATA%`, not roaming `%APPDATA%`, because
executable paths, process recipes, logs, and discovery results are
machine-specific. On first upgraded launch, an empty local store may be
populated only by copying the roaming store through a temporary directory,
validating every document, and atomically activating the copy. The roaming store
remains a recoverable backup, and conflicting stores are never merged silently.

ProfileLibrary retains each trusted source path separately from document
content. Save and delete require a UUID ID matching the source filename.
Invalid legacy documents are backed up and repaired into new UUID-backed
profiles, preventing untrusted IDs from selecting filesystem paths.
