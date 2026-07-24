import type {
  AppSnapshot,
  CreateProfilePayload,
  DuplicateProfilePayload,
  ImportProfilePayload,
  ProfileIdPayload,
  RacingProfile,
  SaveProfilePayload,
} from "../generated/bindings";
import type { NativeBridge } from "./native-bridge";

export class InMemoryNativeBridge implements NativeBridge {
  #nextId = 1;
  #snapshot: AppSnapshot;

  constructor(snapshot: AppSnapshot) {
    this.#snapshot = structuredClone(snapshot);
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
    this.#snapshot.selectedProfile = structuredClone(payload.profile);
    return this.getAppSnapshot();
  }

  selectProfile(payload: ProfileIdPayload): Promise<AppSnapshot> {
    if (this.#snapshot.selectedProfile?.id !== payload.profileId) {
      return Promise.reject(new Error("Profile detail is unavailable"));
    }
    return this.getAppSnapshot();
  }

  duplicateProfile(payload: DuplicateProfilePayload): Promise<AppSnapshot> {
    const source = this.#snapshot.selectedProfile;
    if (!source || source.id !== payload.sourceProfileId) {
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
    return this.getAppSnapshot();
  }

  deleteProfile(payload: ProfileIdPayload): Promise<AppSnapshot> {
    this.#snapshot.profiles = this.#snapshot.profiles.filter(
      (profile) => profile.id !== payload.profileId,
    );
    if (this.#snapshot.selectedProfile?.id === payload.profileId) {
      this.#snapshot.selectedProfile = null;
    }
    return this.getAppSnapshot();
  }

  exportProfile(payload: ProfileIdPayload): Promise<string> {
    const profile = this.#snapshot.selectedProfile;
    if (!profile || profile.id !== payload.profileId) {
      return Promise.reject(new Error("Racing Profile was not found"));
    }
    return Promise.resolve(JSON.stringify(profile));
  }

  importProfile(payload: ImportProfilePayload): Promise<AppSnapshot> {
    const portable = JSON.parse(payload.document) as {
      name: string;
      primarySim: { name: string };
    };
    return this.createProfile({
      name: portable.name,
      primarySimName: portable.primarySim.name,
    });
  }

  #id(prefix: string): string {
    const id = `${prefix}-${this.#nextId}`;
    this.#nextId += 1;
    return id;
  }
}
