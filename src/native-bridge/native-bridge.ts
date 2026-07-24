import type {
  AppSnapshot,
  ApplicationTargetPayload,
  CreateProfilePayload,
  DuplicateProfilePayload,
  ExitApplicationPayload,
  ForceStopApplicationPayload,
  GameLaunchDiagnostic,
  ImportProfilePayload,
  DiscoverySnapshot,
  PrimarySimIdPayload,
  ProfileIdPayload,
  RestartApplicationPayload,
  SaveProfilePayload,
  SupportingApplicationRecommendation,
} from "../generated/bindings";

export interface NativeBridge {
  getAppSnapshot(): Promise<AppSnapshot>;
  createProfile(payload: CreateProfilePayload): Promise<AppSnapshot>;
  saveProfile(payload: SaveProfilePayload): Promise<AppSnapshot>;
  selectProfile(payload: ProfileIdPayload): Promise<AppSnapshot>;
  duplicateProfile(payload: DuplicateProfilePayload): Promise<AppSnapshot>;
  deleteProfile(payload: ProfileIdPayload): Promise<AppSnapshot>;
  exportProfile(payload: ProfileIdPayload): Promise<string>;
  importProfile(payload: ImportProfilePayload): Promise<AppSnapshot>;
  startApplication(payload: ApplicationTargetPayload): Promise<AppSnapshot>;
  refreshProcesses(): Promise<AppSnapshot>;
  exitApplication(payload: ExitApplicationPayload): Promise<AppSnapshot>;
  forceStopApplication(
    payload: ForceStopApplicationPayload,
  ): Promise<AppSnapshot>;
  restartApplication(payload: RestartApplicationPayload): Promise<AppSnapshot>;
  startSession(payload: ProfileIdPayload): Promise<AppSnapshot>;
  testGameLaunch(payload: ProfileIdPayload): Promise<GameLaunchDiagnostic>;
  cancelStartup(): Promise<AppSnapshot>;
  closeSession(): Promise<AppSnapshot>;
  acceptRecovery(): Promise<AppSnapshot>;
  dismissRecovery(): Promise<AppSnapshot>;
  discoverApplications(): Promise<DiscoverySnapshot>;
  recommendApplications(
    payload: PrimarySimIdPayload,
  ): Promise<SupportingApplicationRecommendation[]>;
}
