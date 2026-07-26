import type { UpdateSnapshot } from "../generated/bindings";

export function updateStatusLabel(
  status: UpdateSnapshot["formationLap"],
): string {
  switch (status.kind) {
    case "current":
      return `Current · ${status.currentVersion}`;
    case "updateAvailable":
      return `Update available · ${status.latestVersion}`;
    case "unknown":
      return "Unknown";
  }
}
