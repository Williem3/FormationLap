import type {
  ApplicationRequirement,
  AppSnapshot,
  CloseSessionSettings,
  CreateProfilePayload,
  DuplicateProfilePayload,
  ImportProfilePayload,
  LaunchRecipe,
  ProfileIdPayload,
  ProfileSummary,
  RacingProfile,
  SaveProfilePayload,
  VrLaunchMode,
} from "../generated/bindings";
import type { NativeBridge } from "./native-bridge";

interface PortableProfileApplication {
  name: string;
  launchRecipe: LaunchRecipe;
}

interface PortableSupportingApplication {
  application: PortableProfileApplication;
  requirement: ApplicationRequirement;
  keepRunning: boolean;
}

interface PortableRacingProfile {
  schemaVersion: number;
  name: string;
  primarySim: PortableProfileApplication;
  supportingApplications: PortableSupportingApplication[];
  vrEnabled: boolean;
  preferredVrLaunchMode: VrLaunchMode | null;
  closeSession: CloseSessionSettings;
}

export class InMemoryNativeBridge implements NativeBridge {
  #nextId = 1;
  #profilesById = new Map<string, RacingProfile>();
  #snapshot: AppSnapshot;

  constructor(snapshot: AppSnapshot) {
    this.#snapshot = structuredClone(snapshot);
    for (const summary of this.#snapshot.profiles) {
      this.#profilesById.set(summary.id, this.#profileFromSummary(summary));
    }
    if (this.#snapshot.selectedProfile) {
      this.#profilesById.set(
        this.#snapshot.selectedProfile.id,
        structuredClone(this.#snapshot.selectedProfile),
      );
    }
  }

  getAppSnapshot(): Promise<AppSnapshot> {
    return Promise.resolve(structuredClone(this.#snapshot));
  }

  createProfile(payload: CreateProfilePayload): Promise<AppSnapshot> {
    const profile: RacingProfile = {
      id: this.#id("profile"),
      name: payload.name,
      primarySim: {
        id: this.#id("application"),
        name: payload.primarySimName,
        launchRecipe: {
          source: { kind: "directExecutable", executablePath: "" },
          arguments: [],
          workingDirectory: null,
          monitoredProcess: null,
          consoleVisibility: "hidden",
          elevated: false,
          startupTimeoutSeconds: 30,
          postStartDelayMilliseconds: 0,
          shutdownStrategy: { kind: "closeWindows" },
        },
        pathNeedsRepair: true,
      },
      supportingApplications: [],
      vrEnabled: false,
      preferredVrLaunchMode: null,
      closeSession: { stopSteamVr: false },
    };
    this.#snapshot.profiles.push({
      id: profile.id,
      name: profile.name,
      primarySimName: profile.primarySim.name,
    });
    this.#snapshot.profiles.sort((left, right) =>
      left.name.localeCompare(right.name),
    );
    this.#profilesById.set(profile.id, structuredClone(profile));
    this.#snapshot.selectedProfile ??= profile;
    return this.getAppSnapshot();
  }

  saveProfile(payload: SaveProfilePayload): Promise<AppSnapshot> {
    const summary = this.#snapshot.profiles.find(
      (profile) => profile.id === payload.profile.id,
    );
    if (!summary) {
      return Promise.reject(new Error("Racing Profile was not found"));
    }
    summary.name = payload.profile.name;
    summary.primarySimName = payload.profile.primarySim.name;
    this.#profilesById.set(
      payload.profile.id,
      structuredClone(payload.profile),
    );
    this.#snapshot.selectedProfile = structuredClone(payload.profile);
    return this.getAppSnapshot();
  }

  selectProfile(payload: ProfileIdPayload): Promise<AppSnapshot> {
    const profile = this.#profilesById.get(payload.profileId);
    if (!profile) {
      return Promise.reject(new Error("Profile detail is unavailable"));
    }
    this.#snapshot.selectedProfile = structuredClone(profile);
    return this.getAppSnapshot();
  }

  duplicateProfile(payload: DuplicateProfilePayload): Promise<AppSnapshot> {
    const source = this.#profilesById.get(payload.sourceProfileId);
    if (!source) {
      return Promise.reject(new Error("Racing Profile was not found"));
    }
    const duplicate = structuredClone(source);
    duplicate.id = this.#id("profile");
    duplicate.name = payload.name;
    duplicate.primarySim.id = this.#id("application");
    duplicate.supportingApplications.forEach(({ application }) => {
      application.id = this.#id("application");
    });
    this.#snapshot.profiles.push({
      id: duplicate.id,
      name: duplicate.name,
      primarySimName: duplicate.primarySim.name,
    });
    this.#profilesById.set(duplicate.id, structuredClone(duplicate));
    return this.getAppSnapshot();
  }

  deleteProfile(payload: ProfileIdPayload): Promise<AppSnapshot> {
    this.#snapshot.profiles = this.#snapshot.profiles.filter(
      (profile) => profile.id !== payload.profileId,
    );
    this.#profilesById.delete(payload.profileId);
    if (this.#snapshot.selectedProfile?.id === payload.profileId) {
      const fallback = this.#snapshot.profiles[0];
      this.#snapshot.selectedProfile = fallback
        ? structuredClone(this.#profilesById.get(fallback.id) ?? null)
        : null;
    }
    return this.getAppSnapshot();
  }

  exportProfile(payload: ProfileIdPayload): Promise<string> {
    const profile = this.#profilesById.get(payload.profileId);
    if (!profile) {
      return Promise.reject(new Error("Racing Profile was not found"));
    }
    const portable: PortableRacingProfile = {
      schemaVersion: 1,
      name: profile.name,
      primarySim: {
        name: profile.primarySim.name,
        launchRecipe: structuredClone(profile.primarySim.launchRecipe),
      },
      supportingApplications: profile.supportingApplications.map(
        ({ application, requirement, keepRunning }) => ({
          application: {
            name: application.name,
            launchRecipe: structuredClone(application.launchRecipe),
          },
          requirement,
          keepRunning,
        }),
      ),
      vrEnabled: profile.vrEnabled,
      preferredVrLaunchMode: profile.preferredVrLaunchMode,
      closeSession: structuredClone(profile.closeSession),
    };
    return Promise.resolve(JSON.stringify(portable, null, 2));
  }

  importProfile(payload: ImportProfilePayload): Promise<AppSnapshot> {
    const portable = JSON.parse(payload.document) as PortableRacingProfile;
    if (portable.schemaVersion !== 1) {
      return Promise.reject(
        new Error("Portable profile schema is unsupported"),
      );
    }
    const profile: RacingProfile = {
      id: this.#id("profile"),
      name: portable.name,
      primarySim: {
        id: this.#id("application"),
        name: portable.primarySim.name,
        launchRecipe: structuredClone(portable.primarySim.launchRecipe),
        pathNeedsRepair:
          portable.primarySim.launchRecipe.source.kind === "directExecutable",
      },
      supportingApplications: portable.supportingApplications.map(
        ({ application, requirement, keepRunning }) => ({
          application: {
            id: this.#id("application"),
            name: application.name,
            launchRecipe: structuredClone(application.launchRecipe),
            pathNeedsRepair:
              application.launchRecipe.source.kind === "directExecutable",
          },
          requirement,
          keepRunning,
        }),
      ),
      vrEnabled: portable.vrEnabled,
      preferredVrLaunchMode: portable.preferredVrLaunchMode,
      closeSession: structuredClone(portable.closeSession),
    };
    this.#profilesById.set(profile.id, structuredClone(profile));
    this.#snapshot.profiles.push({
      id: profile.id,
      name: profile.name,
      primarySimName: profile.primarySim.name,
    });
    this.#snapshot.profiles.sort((left, right) =>
      left.name.localeCompare(right.name),
    );
    this.#snapshot.selectedProfile ??= profile;
    return this.getAppSnapshot();
  }

  #id(prefix: string): string {
    const id = `${prefix}-${this.#nextId}`;
    this.#nextId += 1;
    return id;
  }

  #profileFromSummary(summary: ProfileSummary): RacingProfile {
    return {
      id: summary.id,
      name: summary.name,
      primarySim: {
        id: `${summary.id}-primary`,
        name: summary.primarySimName,
        launchRecipe: {
          source: { kind: "directExecutable", executablePath: "" },
          arguments: [],
          workingDirectory: null,
          monitoredProcess: null,
          consoleVisibility: "hidden",
          elevated: false,
          startupTimeoutSeconds: 30,
          postStartDelayMilliseconds: 0,
          shutdownStrategy: { kind: "closeWindows" },
        },
        pathNeedsRepair: true,
      },
      supportingApplications: [],
      vrEnabled: false,
      preferredVrLaunchMode: null,
      closeSession: { stopSteamVr: false },
    };
  }
}
