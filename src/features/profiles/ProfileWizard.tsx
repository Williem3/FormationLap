import type { FormEvent } from "react";
import type { DiscoveredPrimarySim } from "../../generated/bindings";
import { CheckIcon, FlagIcon, PlusIcon, PulseIcon } from "../../ui/icons";
import {
  applicationIcon,
  displayWindowsPath,
  installationSourceLabel,
} from "../../ui/presentation";
import type {
  DiscoveryState,
  PrimarySimSource,
  RecommendationState,
} from "./profile-types";

export interface ProfileWizardProps {
  profileName: string;
  primarySimName: string;
  primarySimSource: PrimarySimSource;
  sourceValue: string;
  discoveryState: DiscoveryState;
  recommendationState: RecommendationState;
  selectedPrimarySimId: string | null;
  selectedSupportingIds: string[];
  isManualEntry: boolean;
  isSaving: boolean;
  error: string | null;
  onProfileNameChange(value: string): void;
  onPrimarySimNameChange(value: string): void;
  onPrimarySimSourceChange(value: PrimarySimSource): void;
  onSourceValueChange(value: string): void;
  onPickExecutablePath(): Promise<string |null>;
  onSelectPrimarySim(primarySim: DiscoveredPrimarySim): void;
  onEnterManual(): void;
  onToggleSupporting(applicationId: string): void;
  onCancel(): void;
  onSubmit(event: FormEvent<HTMLFormElement>): void;
}

export function ProfileWizard({
  profileName,
  primarySimName,
  primarySimSource,
  sourceValue,
  discoveryState,
  recommendationState,
  selectedPrimarySimId,
  selectedSupportingIds,
  isManualEntry,
  isSaving,
  error,
  onProfileNameChange,
  onPrimarySimNameChange,
  onPrimarySimSourceChange,
  onSourceValueChange,
  onPickExecutablePath,
  onSelectPrimarySim,
  onEnterManual,
  onToggleSupporting,
  onCancel,
  onSubmit,
}: ProfileWizardProps) {
  const selectedSupportingApplications =
    discoveryState.kind === "ready"
      ? discoveryState.snapshot.installedSupportingApplications.filter(
          (application) => selectedSupportingIds.includes(application.id),
        )
      : [];
  const selectedPrimarySim =
    discoveryState.kind === "ready"
      ? discoveryState.snapshot.installedPrimarySims.find(
          (primarySim) => primarySim.id === selectedPrimarySimId,
        )
      : undefined;
  const recommendedSupportingIds =
    recommendationState.kind === "ready"
      ? new Set(
          recommendationState.recommendations.map(
            (recommendation) => recommendation.id,
          ),
        )
      : new Set<string>();
  const otherInstalledSupportingApplications =
    discoveryState.kind === "ready"
      ? discoveryState.snapshot.installedSupportingApplications.filter(
          (application) => !recommendedSupportingIds.has(application.id),
        )
      : [];

  return (
    <>
      <header className="workspace-header wizard-header">
        <div>
          <p className="eyebrow">First profile</p>
          <h1>Create your first Racing Profile</h1>
          <p className="workspace-summary">
            Name the setup, choose its Primary Sim, and confirm the locked
            game-last order.
          </p>
        </div>
        <button type="button" className="secondary-button" onClick={onCancel}>
          Cancel
        </button>
      </header>

      <form className="profile-wizard" onSubmit={onSubmit}>
        <div className="wizard-form">
          <section className="wizard-step" aria-labelledby="profile-step-title">
            <span className="step-index">01</span>
            <div>
              <h2 id="profile-step-title">Profile identity</h2>
              <p>Use a name you will recognize from the sidebar.</p>
              <label className="field">
                <span>Profile name</span>
                <input
                  autoFocus
                  required
                  value={profileName}
                  onChange={(event) =>
                    onProfileNameChange(event.currentTarget.value)
                  }
                  placeholder="Le Mans evening"
                />
              </label>
            </div>
          </section>

          <section className="wizard-step" aria-labelledby="sim-step-title">
            <span className="step-index">02</span>
            <div>
              <h2 id="sim-step-title">Choose a Primary Sim</h2>
              <p>
                Formation Lap found these Curated Catalog sims on this PC. The
                Primary Sim always launches last.
              </p>

              {discoveryState.kind === "loading" && (
                <div className="discovery-status" role="status">
                  <PulseIcon />
                  <span>
                    <strong>Checking known locations</strong>
                    <small>
                      Only targeted Steam and Windows locations are inspected.
                    </small>
                  </span>
                </div>
              )}

              {discoveryState.kind === "error" && (
                <div className="discovery-status discovery-status-warning">
                  <FlagIcon />
                  <span>
                    <strong>Automatic discovery is unavailable</strong>
                    <small>
                      You can still configure the Primary Sim manually.
                    </small>
                  </span>
                </div>
              )}

              {discoveryState.kind === "ready" &&
                discoveryState.snapshot.installedPrimarySims.length > 0 && (
                  <div className="sim-picker">
                    {discoveryState.snapshot.installedPrimarySims.map(
                      (primarySim) => {
                        const isSelected =
                          primarySim.id === selectedPrimarySimId;
                        const sourceLabel = installationSourceLabel(
                          primarySim.installation,
                        );
                        return (
                          <button
                            type="button"
                            className="sim-choice"
                            aria-label={`Use ${primarySim.name} (${sourceLabel})`}
                            aria-pressed={isSelected}
                            key={`${primarySim.id}:${sourceLabel}`}
                            onClick={() => onSelectPrimarySim(primarySim)}
                          >
                            <span className="sim-choice-icon">
                              {applicationIcon(primarySim)}
                            </span>
                            <span className="sim-choice-copy">
                              <strong>{primarySim.name}</strong>
                              <small>{sourceLabel}</small>
                            </span>
                            <span className="sim-choice-status">
                              <CheckIcon />
                              Installed
                            </span>
                          </button>
                        );
                      },
                    )}
                  </div>
                )}

              {discoveryState.kind === "ready" &&
                discoveryState.snapshot.installedPrimarySims.length === 0 && (
                  <div className="discovery-status">
                    <FlagIcon />
                    <span>
                      <strong>No Curated Catalog sim was found</strong>
                      <small>
                        Manual Entry keeps uncommon and custom installs
                        available.
                      </small>
                    </span>
                  </div>
                )}

              <div className="manual-entry-action">
                <span>Not listed or using a custom executable?</span>
                <button
                  type="button"
                  className="text-button"
                  onClick={onEnterManual}
                >
                  Enter a sim manually
                </button>
              </div>

              {isManualEntry && (
                <div className="manual-entry-panel">
                  <p className="eyebrow">Manual Entry</p>
                  <div className="field-grid">
                    <label className="field">
                      <span>Primary Sim name</span>
                      <input
                        required
                        value={primarySimName}
                        onChange={(event) =>
                          onPrimarySimNameChange(event.currentTarget.value)
                        }
                        placeholder="Le Mans Ultimate"
                      />
                    </label>
                    <label className="field">
                      <span>Primary Sim source</span>
                      <select
                        value={primarySimSource}
                        onChange={(event) =>
                          onPrimarySimSourceChange(
                            event.currentTarget.value as PrimarySimSource,
                          )
                        }
                      >
                        <option value="direct">Direct executable</option>
                        <option value="steam">Steam</option>
                      </select>
                    </label>
                  </div>
                  <label className="field">
                    <span>
                      {primarySimSource === "steam"
                        ? "Steam App ID"
                        : "Executable path"}
                    </span>
                    <div className="path-input">
                      <input
                        aria-label={
                          primarySimSource === "steam"
                            ? "Steam App ID"
                            : "Executable path"
                        }
                        inputMode={
                          primarySimSource === "steam" ? "numeric" : "text"
                        }
                        value={
                          primarySimSource === "direct"
                            ? displayWindowsPath(sourceValue)
                            : sourceValue
                        }
                        onChange={(event) =>
                          onSourceValueChange(event.currentTarget.value)
                        }
                        placeholder={
                          primarySimSource === "steam"
                            ? "2399420"
                            : String.raw`C:\Games\Le Mans Ultimate\LMU.exe`
                        }
                      />
                      {primarySimSource === "direct" && (
                        <button
                          type="button"
                          className="secondary-button path-browse-button"
                          aria-label="Browse for Primary Sim executable"
                          onClick={() =>
                            void onPickExecutablePath().then(
                              (path) => {
                                if (path) {
                                  onSourceValueChange(path);
                                }
                              },
                            )
                          }
                        >
                          Browseâ€¦
                        </button>
                      )}
                    </div>
                    <small>
                      You can leave this blank and repair the path in the
                      profile editor.
                    </small>
                  </label>
                </div>
              )}

              {!isManualEntry && selectedPrimarySimId && (
                <div className="selected-sim-summary" role="status">
                  <CheckIcon />
                  <span>
                    <strong>{primarySimName} selected</strong>
                    <small>
                      {primarySimSource === "steam"
                        ? `Steam App ID ${sourceValue}`
                        : displayWindowsPath(sourceValue)}
                    </small>
                  </span>
                </div>
              )}
            </div>
          </section>

          {!isManualEntry && selectedPrimarySimId && (
            <section
              className="wizard-step"
              aria-labelledby="recommendations-step-title"
            >
              <span className="step-index">03</span>
              <div>
                <h2 id="recommendations-step-title">
                  Add Supporting Applications
                </h2>
                <p>
                  Detected applications are optional and launch before the
                  Primary Sim.
                </p>
                <div
                  className="recommendation-region"
                  role="region"
                  aria-label={`Recommended for ${primarySimName}`}
                >
                  {recommendationState.kind === "loading" && (
                    <div className="recommendation-empty" role="status">
                      <PulseIcon />
                      <span>Ranking installed applications…</span>
                    </div>
                  )}
                  {recommendationState.kind === "error" && (
                    <div className="recommendation-empty">
                      <FlagIcon />
                      <span>Recommendations could not be loaded.</span>
                    </div>
                  )}
                  {recommendationState.kind === "ready" &&
                    recommendationState.recommendations.length === 0 && (
                      <div className="recommendation-empty">
                        <CheckIcon />
                        <span>No installed recommendations to add.</span>
                      </div>
                    )}
                  {recommendationState.kind === "ready" &&
                    recommendationState.recommendations.length > 0 && (
                      <div className="recommendation-list">
                        {recommendationState.recommendations.map(
                          (recommendation) => {
                            const installedApplication =
                              discoveryState.kind === "ready"
                                ? discoveryState.snapshot.installedSupportingApplications.find(
                                    (application) =>
                                      application.id === recommendation.id,
                                  )
                                : undefined;
                            const rankLabel =
                              recommendation.rank === "recommended"
                                ? "Recommended"
                                : "Compatible";
                            return (
                              <label
                                className="recommendation-row"
                                key={recommendation.id}
                              >
                                <input
                                  type="checkbox"
                                  aria-label={`Add ${recommendation.name}`}
                                  checked={selectedSupportingIds.includes(
                                    recommendation.id,
                                  )}
                                  onChange={() =>
                                    onToggleSupporting(recommendation.id)
                                  }
                                />
                                <span className="recommendation-icon">
                                  {installedApplication ? (
                                    applicationIcon(installedApplication)
                                  ) : (
                                    <FlagIcon />
                                  )}
                                </span>
                                <span className="recommendation-copy">
                                  <strong>{recommendation.name}</strong>
                                  <small>
                                    {recommendation.updateProvider?.kind ===
                                    "githubReleases"
                                      ? "Update notifications via GitHub Releases"
                                      : "Detected on this PC"}
                                  </small>
                                </span>
                                <span
                                  className={`recommendation-rank recommendation-rank-${recommendation.rank}`}
                                >
                                  {rankLabel}
                                </span>
                              </label>
                            );
                          },
                        )}
                      </div>
                    )}
                  {otherInstalledSupportingApplications.length > 0 && (
                    <div className="additional-discovered-applications">
                      <p className="additional-discovered-heading">
                        Other detected applications
                      </p>
                      <div className="recommendation-list">
                        {otherInstalledSupportingApplications.map(
                          (application) => (
                            <label
                              className="recommendation-row"
                              key={application.id}
                            >
                              <input
                                type="checkbox"
                                aria-label={`Add ${application.name}`}
                                checked={selectedSupportingIds.includes(
                                  application.id,
                                )}
                                onChange={() =>
                                  onToggleSupporting(application.id)
                                }
                              />
                              <span className="recommendation-icon">
                                {applicationIcon(application)}
                              </span>
                              <span className="recommendation-copy">
                                <strong>{application.name}</strong>
                                <small>Detected on this PC</small>
                              </span>
                              <span className="recommendation-rank">
                                Available
                              </span>
                            </label>
                          ),
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </section>
          )}

          {error && (
            <p className="form-error" role="alert">
              {error}
            </p>
          )}

          <div className="wizard-actions">
            <button
              type="submit"
              className="primary-button"
              disabled={isSaving || primarySimName.trim().length === 0}
            >
              {isSaving ? "Creating profile…" : "Create Racing Profile"}
            </button>
          </div>
        </div>

        <aside className="wizard-preview" aria-labelledby="preview-title">
          <div className="preview-heading">
            <span className="step-index">04</span>
            <div>
              <p className="eyebrow">Review</p>
              <h2 id="preview-title">Startup order</h2>
            </div>
          </div>
          <div className="order-preview">
            {selectedSupportingApplications.length > 0 ? (
              selectedSupportingApplications.map((application) => (
                <div className="order-application-row" key={application.id}>
                  <span className="application-icon">
                    {applicationIcon(application)}
                  </span>
                  <span>
                    <strong>{application.name}</strong>
                    <small>Supporting Application</small>
                  </span>
                </div>
              ))
            ) : (
              <div className="order-empty">
                <PlusIcon />
                <span>
                  <strong>Supporting Applications</strong>
                  <small>Select installed recommendations to add them.</small>
                </span>
              </div>
            )}
            <div className="order-divider" aria-hidden="true">
              <span />
              <small>Launches last</small>
            </div>
            <div className="game-order-row">
              <span className="game-order-icon">
                {selectedPrimarySim ? (
                  applicationIcon(selectedPrimarySim)
                ) : (
                  <FlagIcon />
                )}
              </span>
              <span>
                <strong>{primarySimName || "Primary Sim"}</strong>
                <small>
                  {primarySimSource === "steam"
                    ? "Steam"
                    : "Standalone / Direct executable"}
                </small>
              </span>
              <span className="locked-label">Locked</span>
            </div>
          </div>
          <p className="preview-note">
            The game-last rule keeps preparation predictable and cannot be
            reordered.
          </p>
        </aside>
      </form>
    </>
  );
}
