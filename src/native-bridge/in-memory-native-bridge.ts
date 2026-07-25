import type {
  ApplicationRequirement,
  ApplicationTargetPayload,
  AppSnapshot,
  ApproveProfilePayload,
  CloseSessionSettings,
  CreateProfilePayload,
  DuplicateProfilePayload,
  DiagnosticExport,
  DiscoverySnapshot,
  ExitApplicationPayload,
  ForceStopApplicationPayload,
  GameLaunchDiagnostic,
  ImportProfilePayload,
  LaunchRecipe,
  ProfileIdPayload,
  ProfileSummary,
  PrimarySimIdPayload,
  QuitPayload,
  RacingProfile,
  RestartApplicationPayload,
  SaveProfilePayload,
  SupportingApplicationRecommendation,
  UpdateSettingsPayload,
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
  #nextProcessId = 10_000;
  #profilesById = new Map<string, RacingProfile>();
  #snapshot: AppSnapshot;
  #discovery: DiscoverySnapshot;
  #recommendationsBySim: Record<string, SupportingApplicationRecommendation[]>;

  constructor(
    snapshot: AppSnapshot,
    discovery: DiscoverySnapshot = {
      primarySims: [],
      supportingApplications: [],
      installedPrimarySims: [],
      installedSupportingApplications: [],
    },
    recommendationsBySim: Record<
      string,
      SupportingApplicationRecommendation[]
    > = {},
  ) {
    this.#snapshot = structuredClone(snapshot);
    this.#discovery = structuredClone(discovery);
    this.#recommendationsBySim = structuredClone(recommendationsBySim);
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

  pickExecutablePath(): Promise<string | null> {
    return Promise.resolve(null);
  }

  discoverApplications(): Promise<DiscoverySnapshot> {
    return Promise.resolve(structuredClone(this.#discovery));
  }

  recommendApplications(
    payload: PrimarySimIdPayload,
  ): Promise<SupportingApplicationRecommendation[]> {
    return Promise.resolve(
      structuredClone(this.#recommendationsBySim[payload.primarySimId] ?? []),
    );
  }

  startApplication(payload: ApplicationTargetPayload): Promise<AppSnapshot> {
    const profile = this.#profilesById.get(payload.profileId);
    const application = profile
      ? [
          profile.primarySim,
          ...profile.supportingApplications.map(
            (supporting) => supporting.application,
          ),
        ].find((candidate) => candidate.id === payload.applicationId)
      : undefined;
    if (!application) {
      return Promise.reject(new Error("Configured application was not found"));
    }
    const existing = this.#snapshot.applicationProcesses.find(
      (process) => process.applicationId === payload.applicationId,
    );
    if (existing?.identity) {
      return this.getAppSnapshot();
    }
    this.#snapshot.applicationProcesses = [
      ...this.#snapshot.applicationProcesses.filter(
        (process) => process.applicationId !== payload.applicationId,
      ),
      {
        applicationId: payload.applicationId,
        status: "starting",
        ownership: "sessionOwned",
        identity: {
          pid: this.#nextProcessId++,
          creationTime: String(Date.now()),
          canonicalExecutablePath:
            application.launchRecipe.source.kind === "directExecutable"
              ? application.launchRecipe.source.executablePath
              : `steam://${application.launchRecipe.source.appId}`,
        },
        output: null,
      },
    ];
    return this.getAppSnapshot();
  }

  async refreshProcesses(): Promise<AppSnapshot> {
    for (const process of this.#snapshot.applicationProcesses) {
      if (process.status === "starting") {
        process.status = "running";
      }
    }
    if (this.#snapshot.session.state === "starting") {
      for (const application of this.#snapshot.session.applications) {
        const process = this.#snapshot.applicationProcesses.find(
          (candidate) => candidate.applicationId === application.applicationId,
        );
        if (process?.status === "running") {
          application.state = "running";
        }
      }
      const next = this.#snapshot.session.applications.find(
        (application) => application.state === "pending",
      );
      if (next && this.#snapshot.session.activeProfileId) {
        await this.startApplication({
          profileId: this.#snapshot.session.activeProfileId,
          applicationId: next.applicationId,
        });
        next.state = "starting";
      } else if (
        this.#snapshot.session.applications.every(
          (application) =>
            application.state === "running" ||
            application.state === "runningPreExisting" ||
            application.state === "failed",
        )
      ) {
        this.#snapshot.session.state = "active";
      }
    } else if (
      this.#snapshot.session.state === "cancelling" ||
      this.#snapshot.session.state === "closing"
    ) {
      for (const application of this.#snapshot.session.applications) {
        const process = this.#snapshot.applicationProcesses.find(
          (candidate) => candidate.applicationId === application.applicationId,
        );
        if (process?.ownership === "sessionOwned") {
          process.status = "stopped";
          process.ownership = null;
          process.identity = null;
          application.state = "stopped";
        } else if (process?.ownership === "preExisting") {
          application.state = "detached";
        }
      }
      this.#snapshot.session.state = "idle";
      this.#snapshot.session.activeProfileId = null;
    }
    return this.getAppSnapshot();
  }

  exitApplication(payload: ExitApplicationPayload): Promise<AppSnapshot> {
    const process = this.#snapshot.applicationProcesses.find(
      (candidate) => candidate.applicationId === payload.applicationId,
    );
    if (
      !process ||
      (process.ownership === "preExisting" && !payload.preExistingConfirmed)
    ) {
      return this.getAppSnapshot();
    }
    process.status = "stopped";
    process.ownership = null;
    process.identity = null;
    return this.getAppSnapshot();
  }

  forceStopApplication(
    payload: ForceStopApplicationPayload,
  ): Promise<AppSnapshot> {
    const process = this.#snapshot.applicationProcesses.find(
      (candidate) => candidate.applicationId === payload.applicationId,
    );
    if (
      !process ||
      !payload.forceConfirmed ||
      (process.ownership === "preExisting" && !payload.preExistingConfirmed)
    ) {
      return this.getAppSnapshot();
    }
    process.status = "stopped";
    process.ownership = null;
    process.identity = null;
    return this.getAppSnapshot();
  }

  async restartApplication(
    payload: RestartApplicationPayload,
  ): Promise<AppSnapshot> {
    const process = this.#snapshot.applicationProcesses.find(
      (candidate) => candidate.applicationId === payload.applicationId,
    );
    if (process?.ownership === "preExisting" && !payload.preExistingConfirmed) {
      return this.getAppSnapshot();
    }
    if (process) {
      process.status = "stopped";
      process.ownership = null;
      process.identity = null;
    }
    return this.startApplication(payload);
  }

  async startSession(payload: ProfileIdPayload): Promise<AppSnapshot> {
    if (this.#snapshot.session.state !== "idle") {
      return Promise.reject(
        new Error("a Session action is already in progress"),
      );
    }
    const profile = this.#profilesById.get(payload.profileId);
    if (!profile) {
      return Promise.reject(new Error("Racing Profile was not found"));
    }
    const summary = this.#snapshot.profiles.find(
      (candidate) => candidate.id === payload.profileId,
    );
    if (summary?.reviewStatus === "needsReview") {
      return Promise.reject(new Error("Racing Profile needs review"));
    }
    this.#snapshot.session = {
      state: "starting",
      activeProfileId: profile.id,
      applications: [
        ...profile.supportingApplications.map((supporting) => ({
          applicationId: supporting.application.id,
          name: supporting.application.name,
          role: "supporting" as const,
          requirement: supporting.requirement,
          state: "pending" as const,
        })),
        {
          applicationId: profile.primarySim.id,
          name: profile.primarySim.name,
          role: "primarySim",
          requirement: null,
          state: "pending",
        },
      ],
      summary: null,
    };
    const first = this.#snapshot.session.applications[0];
    if (first) {
      await this.startApplication({
        profileId: profile.id,
        applicationId: first.applicationId,
      });
      first.state = "starting";
    }
    return this.getAppSnapshot();
  }

  testGameLaunch(payload: ProfileIdPayload): Promise<GameLaunchDiagnostic> {
    const profile = this.#profilesById.get(payload.profileId);
    if (!profile) {
      return Promise.reject(new Error("Racing Profile was not found"));
    }
    const source = profile.primarySim.launchRecipe.source;
    const executableName =
      source.kind === "directExecutable"
        ? source.executablePath.split(/[\\/]/).at(-1) || "unknown.exe"
        : profile.primarySim.launchRecipe.monitoredProcess || "unknown.exe";
    const target =
      source.kind === "steam"
        ? ({
            kind: "steam",
            uri:
              profile.primarySim.launchRecipe.arguments.length > 0
                ? `steam://run/${source.appId}//${profile.primarySim.launchRecipe.arguments
                    .map(encodeURIComponent)
                    .join("%20")}/`
                : `steam://launch/${source.appId}/${
                    source.selector?.kind === "openVr"
                      ? "VR"
                      : source.selector?.kind === "oculus"
                        ? "OTHERVR"
                        : source.selector?.kind === "option"
                          ? `option${source.selector.index}`
                          : "option0"
                  }`,
          } as const)
        : ({ kind: "directExecutable", executableName } as const);
    return Promise.resolve({
      schemaVersion: 1,
      profileName: profile.name,
      vrEnabled: profile.vrEnabled,
      vrLaunchMode: profile.vrEnabled ? profile.preferredVrLaunchMode : null,
      target,
      arguments: [...profile.primarySim.launchRecipe.arguments],
      monitoredProcess: profile.primarySim.launchRecipe.monitoredProcess,
      observedProcess: executableName,
    });
  }

  cancelStartup(): Promise<AppSnapshot> {
    if (this.#snapshot.session.state !== "starting") {
      return Promise.reject(new Error("Startup is not active"));
    }
    this.#snapshot.session.state = "cancelling";
    return this.getAppSnapshot();
  }

  closeSession(): Promise<AppSnapshot> {
    if (this.#snapshot.session.state !== "active") {
      return Promise.reject(new Error("Session is not active"));
    }
    this.#snapshot.session.state = "closing";
    const primary = this.#snapshot.session.applications.find(
      (application) => application.role === "primarySim",
    );
    if (primary?.state === "running") {
      primary.state = "stopping";
    }
    return this.getAppSnapshot();
  }

  requestQuit(payload: QuitPayload): Promise<AppSnapshot> {
    if (payload.disposition === "closeSession") {
      if (this.#snapshot.session.state === "starting") {
        this.#snapshot.session.state = "cancelling";
      } else if (
        this.#snapshot.session.state === "active" ||
        this.#snapshot.session.state === "recoveryAvailable"
      ) {
        this.#snapshot.session.state = "closing";
      }
    } else {
      for (const process of this.#snapshot.applicationProcesses) {
        if (process.identity) {
          process.ownership = "preExisting";
          process.status = "runningPreExisting";
        }
      }
      for (const application of this.#snapshot.session.applications) {
        application.state = "detached";
      }
      this.#snapshot.session.state = "idle";
      this.#snapshot.session.activeProfileId = null;
    }
    return this.getAppSnapshot();
  }

  listenForQuitRequest(listener: () => void): Promise<() => void> {
    void listener;
    return Promise.resolve(() => undefined);
  }

  updateSettings(payload: UpdateSettingsPayload): Promise<AppSnapshot> {
    this.#snapshot.settings = structuredClone(payload.settings);
    return this.getAppSnapshot();
  }

  checkUpdates(): Promise<AppSnapshot> {
    return this.getAppSnapshot();
  }

  installFormationLapUpdate(): Promise<AppSnapshot> {
    return this.getAppSnapshot();
  }

  exportDiagnostics(): Promise<DiagnosticExport> {
    return Promise.resolve({
      schemaVersion: 1,
      applicationVersion: "0.1.0-preview",
      platform: "browser-preview",
      settings: structuredClone(this.#snapshot.settings),
      sessionState: this.#snapshot.session.state,
      profileCount: this.#snapshot.profiles.length,
      configuredApplicationCount: this.#snapshot.profiles.length,
      recentEvents: [],
      telemetryUpload: false,
    });
  }

  acceptRecovery(): Promise<AppSnapshot> {
    if (this.#snapshot.session.state !== "recoveryAvailable") {
      return Promise.reject(new Error("Recovery is not available"));
    }
    this.#snapshot.session.state = "active";
    return this.getAppSnapshot();
  }

  dismissRecovery(): Promise<AppSnapshot> {
    if (this.#snapshot.session.state !== "recoveryAvailable") {
      return Promise.reject(new Error("Recovery is not available"));
    }
    this.#snapshot.session.state = "idle";
    this.#snapshot.session.activeProfileId = null;
    for (const application of this.#snapshot.session.applications) {
      application.state = "detached";
    }
    return this.getAppSnapshot();
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
          monitoredExecutablePath: null,
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
      reviewStatus: "approved",
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
    const stored = this.#profilesById.get(payload.profile.id);
    if (
      summary.reviewStatus !== "needsReview" &&
      stored &&
      this.#privilegedRecipeChanged(stored, payload.profile)
    ) {
      summary.reviewStatus = "needsReview";
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
      reviewStatus:
        this.#snapshot.profiles.find(
          (profile) => profile.id === payload.sourceProfileId,
        )?.reviewStatus ?? "approved",
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
      reviewStatus: "needsReview",
    });
    this.#snapshot.profiles.sort((left, right) =>
      left.name.localeCompare(right.name),
    );
    this.#snapshot.selectedProfile ??= profile;
    return this.getAppSnapshot();
  }

  approveProfile(payload: ApproveProfilePayload): Promise<AppSnapshot> {
    const profile = this.#profilesById.get(payload.profileId);
    const summary = this.#snapshot.profiles.find(
      (candidate) => candidate.id === payload.profileId,
    );
    if (!profile || !summary || !payload.configurationReviewed) {
      return Promise.reject(new Error("Racing Profile approval is invalid"));
    }
    const required = [
      profile.primarySim,
      ...profile.supportingApplications.map(
        (supporting) => supporting.application,
      ),
    ]
      .filter(
        (application) =>
          application.launchRecipe.elevated ||
          application.launchRecipe.shutdownStrategy.kind === "customStop",
      )
      .map((application) => application.id)
      .sort();
    const approved = [
      ...new Set(payload.approvedPrivilegedApplicationIds),
    ].sort();
    if (
      approved.length !== payload.approvedPrivilegedApplicationIds.length ||
      JSON.stringify(approved) !== JSON.stringify(required) ||
      [
        profile.primarySim,
        ...profile.supportingApplications.map(
          (supporting) => supporting.application,
        ),
      ].some((application) => application.pathNeedsRepair)
    ) {
      return Promise.reject(new Error("Racing Profile approval is incomplete"));
    }
    summary.reviewStatus = "approved";
    return this.getAppSnapshot();
  }

  #id(prefix: string): string {
    const id = `${prefix}-${this.#nextId}`;
    this.#nextId += 1;
    return id;
  }

  #privilegedRecipeChanged(
    stored: RacingProfile,
    updated: RacingProfile,
  ): boolean {
    const storedApplications = new Map(
      [
        stored.primarySim,
        ...stored.supportingApplications.map(
          (supporting) => supporting.application,
        ),
      ].map((application) => [application.id, application]),
    );
    return [
      updated.primarySim,
      ...updated.supportingApplications.map(
        (supporting) => supporting.application,
      ),
    ].some((application) => {
      const previous = storedApplications.get(application.id);
      const currentApproval = this.#privilegedRecipeApprovalValue(application);
      return previous
        ? this.#privilegedRecipeApprovalValue(previous) !== currentApproval
        : currentApproval !== null;
    });
  }

  #privilegedRecipeApprovalValue(
    application: RacingProfile["primarySim"],
  ): string | null {
    const recipe = application.launchRecipe;
    const customStop =
      recipe.shutdownStrategy.kind === "customStop"
        ? recipe.shutdownStrategy
        : null;
    if (!recipe.elevated && !customStop) {
      return null;
    }
    return JSON.stringify({
      elevated: recipe.elevated,
      source: recipe.elevated ? recipe.source : null,
      arguments: recipe.elevated ? recipe.arguments : null,
      workingDirectory: recipe.elevated ? recipe.workingDirectory : null,
      customStop,
    });
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
          monitoredExecutablePath: null,
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
