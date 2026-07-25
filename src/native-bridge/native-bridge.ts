import type {
  AppSnapshot,
  ApplicationTargetPayload,
  ApproveProfilePayload,
  CreateProfilePayload,
  DuplicateProfilePayload,
  ExitApplicationPayload,
  DiagnosticExport,
  ForceStopApplicationPayload,
  GameLaunchDiagnostic,
  ImportProfilePayload,
  DiscoverySnapshot,
  PrimarySimIdPayload,
  ProfileIdPayload,
  QuitPayload,
  RestartApplicationPayload,
  SaveProfilePayload,
  SupportingApplicationRecommendation,
  UpdateSettingsPayload,
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
  approveProfile(payload: ApproveProfilePayload): Promise<AppSnapshot>;
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
  requestQuit(payload: QuitPayload): Promise<AppSnapshot>;
  listenForQuitRequest(listener: () => void): Promise<() => void>;
  updateSettings(payload: UpdateSettingsPayload): Promise<AppSnapshot>;
  checkUpdates(): Promise<AppSnapshot>;
  installFormationLapUpdate(): Promise<AppSnapshot>;
  exportDiagnostics(): Promise<DiagnosticExport>;
  acceptRecovery(): Promise<AppSnapshot>;
  dismissRecovery(): Promise<AppSnapshot>;
  discoverApplications(): Promise<DiscoverySnapshot>;
  recommendApplications(
    payload: PrimarySimIdPayload,
  ): Promise<SupportingApplicationRecommendation[]>;
}
