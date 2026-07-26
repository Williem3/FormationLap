---
status: accepted
---

# Persist privileged-profile approval outside editable profile JSON

Elevated and custom-stop Launch Recipes require explicit user review. Profile
JSON intentionally remains transparent and user-owned, so its `reviewStatus`
cannot be treated as authority for a privileged action after a restart.

Formation Lap stores a compact approval record separately under the local data
root. Windows DPAPI protects the record for the current Windows user without
showing a second prompt. The record contains the Racing Profile ID and a
SHA-256 fingerprint of its complete launch configuration: the Primary Sim and
every Supporting Application's ID, Launch Recipe, Required/Optional setting,
and keep-running behavior. It does not contain executable paths in plaintext.

On reload, a privileged profile remains Approved only when its editable JSON
still says Approved and its protected record decrypts, matches the profile ID,
and has the same fingerprint. Missing, unreadable, malformed, or mismatched
records fail closed to Needs Review. Saving a changed configuration removes the
prior record. Imported and duplicated privileged profiles receive new IDs, so
no record is transferred. UAC is still requested only when an approved elevated
entry is actually launched.

Keeping approval only in memory was rejected because it required a full review
on every restart. Keeping a hash or a status in profile JSON was rejected
because the file is editable. A persistent elevated service was rejected because
it would expand the privileged attack surface rather than protect a local
approval decision.
