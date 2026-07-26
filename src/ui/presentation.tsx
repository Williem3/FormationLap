import type { ReactNode } from "react";
import type {
  AppSnapshot,
  DiscoveredInstallation,
  DiscoveredPrimarySim,
  DiscoveredSupportingApplication,
  LaunchSource,
  NewSupportingApplication,
} from "../generated/bindings";
import { FlagIcon } from "./icons";

export function launchSourceFromInstallation(
  installation: DiscoveredInstallation,
): LaunchSource {
  return installation.kind === "steam"
    ? { kind: "steam", appId: installation.appId, selector: null }
    : {
        kind: "directExecutable",
        executablePath: installation.executablePath,
      };
}

export function installationWorkingDirectory(
  installation: DiscoveredInstallation,
): string | null {
  if (installation.kind === "steam") {
    return installation.install_directory;
  }

  return directoryFromPath(installation.executablePath);
}

export function directoryFromPath(path: string): string | null {
  const lastSeparator = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return lastSeparator > 0 ? path.slice(0, lastSeparator) : null;
}

export function displayWindowsPath(path: string): string {
  const extendedUncPrefix = "\\\\?\\UNC\\";
  const extendedPathPrefix = "\\\\?\\";
  if (path.startsWith(extendedUncPrefix)) {
    return `\\\\${path.slice(extendedUncPrefix.length)}`;
  }
  return path.startsWith(extendedPathPrefix)
    ? path.slice(extendedPathPrefix.length)
    : path;
}

export function commandErrorMessage(error: unknown, fallback: string): string {
  const details =
    typeof error === "string"
      ? (() => {
          try {
            return JSON.parse(error) as unknown;
          } catch {
            return null;
          }
        })()
      : error;
  if (typeof details !== "object" || details === null) {
    return fallback;
  }
  const { message, recovery } = details as {
    message?: unknown;
    recovery?: unknown;
  };
  if (typeof message !== "string" || message.trim().length === 0) {
    return fallback;
  }
  return typeof recovery === "string" && recovery.trim().length > 0
    ? `${message} ${recovery}`
    : message;
}

export function executableNameFromPath(path: string): string | null {
  const lastSeparator = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  const fileName = path.slice(lastSeparator + 1);
  return fileName.length > 0 ? fileName : null;
}

export function discoveredSupportingApplicationToProfile(
  application: DiscoveredSupportingApplication,
): NewSupportingApplication {
  return {
    application: {
      name: application.name,
      launchRecipe: {
        source: launchSourceFromInstallation(application.installation),
        arguments: application.profileDefaults.arguments,
        workingDirectory: installationWorkingDirectory(
          application.installation,
        ),
        monitoredProcess: null,
        monitoredExecutablePath: null,
        consoleVisibility: application.profileDefaults.consoleVisibility,
        elevated: application.profileDefaults.elevated,
        startupTimeoutSeconds:
          application.profileDefaults.startupTimeoutSeconds,
        postStartDelayMilliseconds:
          application.profileDefaults.postStartDelayMilliseconds,
        shutdownStrategy: application.profileDefaults.shutdownStrategy,
      },
    },
    requirement: application.profileDefaults.requirement,
    keepRunning: application.profileDefaults.keepRunning,
  };
}

export function installationSourceLabel(
  installation: DiscoveredInstallation,
): string {
  return installation.kind === "steam" ? "Steam" : "Standalone";
}

export function applicationIcon(
  application: DiscoveredPrimarySim | DiscoveredSupportingApplication,
) {
  return application.icon.kind === "localData" ? (
    <img
      alt=""
      src={`data:${application.icon.media_type};base64,${application.icon.data_base64}`}
    />
  ) : (
    <FlagIcon />
  );
}

export function profileApplicationIcon(
  applicationId: string,
  applicationIcons: NonNullable<AppSnapshot["applicationIcons"]>,
  fallback: ReactNode,
) {
  const icon = applicationIcons.find(
    (candidate) => candidate.applicationId === applicationId,
  )?.icon;
  return icon?.kind === "localData" ? (
    <img alt="" src={`data:${icon.media_type};base64,${icon.data_base64}`} />
  ) : (
    fallback
  );
}
