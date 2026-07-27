import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type FormEvent,
} from "react";
import { createPortal } from "react-dom";
import type {
  AppSnapshot,
  ProfileApplication,
  RacingProfile,
  SupportingApplication,
} from "../../generated/bindings";
import { FlagIcon, PlusIcon } from "../../ui/icons";
import {
  directoryFromPath,
  displayWindowsPath,
  executableNameFromPath,
  profileApplicationIcon,
} from "../../ui/presentation";
import type { ProfileApproval } from "./profile-types";

export interface ProfileEditorProps {
  profile: RacingProfile;
  applicationIcons: NonNullable<AppSnapshot["applicationIcons"]>;
  needsReview: boolean;
  isSaving: boolean;
  error: string | null;
  onPickExecutablePath(initialPath?: string | null): Promise<string | null>;
  onChange(profile: RacingProfile): void;
  onCancel(): void;
  onSubmit(event: FormEvent<HTMLFormElement>, approval?: ProfileApproval): void;
}

interface SupportingApplicationDragPreview {
  applicationId: string;
  x: number;
  y: number;
  width: number;
  grabOffsetX: number;
  grabOffsetY: number;
}

export function ProfileEditor({
  profile,
  applicationIcons,
  needsReview,
  isSaving,
  error,
  onPickExecutablePath,
  onChange,
  onCancel,
  onSubmit,
}: ProfileEditorProps) {
  const nextSupportingApplicationKey = useRef(0);
  const activeReorderPointerId = useRef<number | null>(null);
  const lastReorderTarget = useRef<string | null>(null);
  const [configurationReviewed, setConfigurationReviewed] = useState(false);
  const [openSupportingApplicationId, setOpenSupportingApplicationId] =
    useState<string | null>(
      profile.supportingApplications[0]?.application.id ?? null,
    );
  const [draggedSupportingApplicationId, setDraggedSupportingApplicationId] =
    useState<string | null>(null);
  const [dragPreview, setDragPreview] =
    useState<SupportingApplicationDragPreview | null>(null);
  const [dropTarget, setDropTarget] = useState<{
    applicationId: string;
    position: "before" | "after";
  } | null>(null);
  const [
    approvedPrivilegedApplicationIds,
    setApprovedPrivilegedApplicationIds,
  ] = useState<string[]>([]);
  const clearSupportingApplicationReorder = useCallback(
    (pointerId?: number) => {
      if (
        pointerId !== undefined &&
        activeReorderPointerId.current !== pointerId
      ) {
        return;
      }
      activeReorderPointerId.current = null;
      lastReorderTarget.current = null;
      setDraggedSupportingApplicationId(null);
      setDragPreview(null);
      setDropTarget(null);
    },
    [],
  );
  useEffect(() => {
    const clearOnPointerEnd = (event: PointerEvent) => {
      clearSupportingApplicationReorder(event.pointerId);
    };
    const clearOnWindowBlur = () => {
      clearSupportingApplicationReorder();
    };
    window.addEventListener("pointerup", clearOnPointerEnd);
    window.addEventListener("pointercancel", clearOnPointerEnd);
    window.addEventListener("blur", clearOnWindowBlur);
    return () => {
      window.removeEventListener("pointerup", clearOnPointerEnd);
      window.removeEventListener("pointercancel", clearOnPointerEnd);
      window.removeEventListener("blur", clearOnWindowBlur);
    };
  }, [clearSupportingApplicationReorder]);
  const profileApplications = [
    profile.primarySim,
    ...profile.supportingApplications.map(
      (supporting) => supporting.application,
    ),
  ];
  const privilegedApplications = profileApplications.filter(
    (application) =>
      application.launchRecipe.elevated ||
      application.launchRecipe.shutdownStrategy.kind === "customStop",
  );
  const pathsNeedRepair = profileApplications.some(
    (application) => application.pathNeedsRepair,
  );
  const applicationsNeedingRepair = profileApplications.filter(
    (application) => application.pathNeedsRepair,
  );
  const approvalComplete =
    configurationReviewed &&
    privilegedApplications.every((application) =>
      approvedPrivilegedApplicationIds.includes(application.id),
    );
  const previewSupportingApplication = dragPreview
    ? profile.supportingApplications.find(
        (supporting) => supporting.application.id === dragPreview.applicationId,
      )
    : null;

  const update = (change: (next: RacingProfile) => void) => {
    const next = structuredClone(profile);
    change(next);
    onChange(next);
  };

  const updateSupportingApplication = (
    index: number,
    change: (supporting: SupportingApplication) => void,
  ) => {
    update((next) => {
      const supportingApplication = next.supportingApplications[index];
      if (supportingApplication) {
        change(supportingApplication);
      }
    });
  };

  const addSupportingApplication = () => {
    nextSupportingApplicationKey.current += 1;
    const application: ProfileApplication = {
      id: `new-supporting-application-${nextSupportingApplicationKey.current}`,
      name: "New Supporting Application",
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
    };
    update((next) => {
      next.supportingApplications.push({
        application,
        requirement: "optional",
        keepRunning: false,
      });
    });
    setOpenSupportingApplicationId(application.id);
  };

  const removeSupportingApplication = (index: number) => {
    const removed = profile.supportingApplications[index];
    if (removed?.application.id === openSupportingApplicationId) {
      setOpenSupportingApplicationId(
        profile.supportingApplications[index + 1]?.application.id ??
          profile.supportingApplications[index - 1]?.application.id ??
          null,
      );
    }
    update((next) => {
      next.supportingApplications.splice(index, 1);
    });
  };

  const moveSupportingApplication = (index: number, direction: -1 | 1) => {
    update((next) => {
      const destination = index + direction;
      if (
        destination < 0 ||
        destination >= next.supportingApplications.length
      ) {
        return;
      }
      const [application] = next.supportingApplications.splice(index, 1);
      if (application) {
        next.supportingApplications.splice(destination, 0, application);
      }
    });
  };

  const reorderSupportingApplications = (
    sourceApplicationId: string,
    destinationApplicationId: string,
    position: "before" | "after",
  ) => {
    if (sourceApplicationId === destinationApplicationId) {
      return;
    }
    update((next) => {
      const sourceIndex = next.supportingApplications.findIndex(
        (supporting) => supporting.application.id === sourceApplicationId,
      );
      const destinationIndex = next.supportingApplications.findIndex(
        (supporting) => supporting.application.id === destinationApplicationId,
      );
      if (sourceIndex < 0 || destinationIndex < 0) {
        return;
      }
      const [application] = next.supportingApplications.splice(sourceIndex, 1);
      if (!application) {
        return;
      }
      const destinationAfterRemoval = next.supportingApplications.findIndex(
        (supporting) => supporting.application.id === destinationApplicationId,
      );
      next.supportingApplications.splice(
        destinationAfterRemoval + (position === "after" ? 1 : 0),
        0,
        application,
      );
    });
  };

  const dropPositionFor = (
    element: HTMLElement,
    clientY: number,
  ): "before" | "after" => {
    const bounds = element.getBoundingClientRect();
    return clientY > bounds.top + bounds.height / 2 ? "after" : "before";
  };

  const dropTargetAt = (clientX: number, clientY: number) => {
    const row = document
      .elementFromPoint(clientX, clientY)
      ?.closest<HTMLElement>("[data-supporting-application-id]");
    const applicationId = row?.dataset.supportingApplicationId;
    if (!row || !applicationId) {
      return null;
    }
    return {
      applicationId,
      position: dropPositionFor(row, clientY),
    };
  };

  return (
    <form
      className="profile-editor"
      onSubmit={(event) =>
        onSubmit(
          event,
          needsReview
            ? {
                configurationReviewed,
                approvedPrivilegedApplicationIds,
              }
            : undefined,
        )
      }
    >
      <header className="workspace-header editor-header">
        <div>
          <p className="eyebrow">Profile editor</p>
          <h1>{profile.name}</h1>
          <p className="workspace-summary">
            Configure launch behavior on the left and keep the Primary Sim
            locked last on the right.
          </p>
        </div>
        <div className="editor-header-actions">
          <div className="header-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={onCancel}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="primary-button"
              disabled={isSaving || (needsReview && !approvalComplete)}
            >
              {isSaving
                ? "Saving…"
                : needsReview
                  ? "Save and approve profile"
                  : "Save changes"}
            </button>
          </div>
          {error && (
            <p className="form-error editor-header-error" role="alert">
              {error}
            </p>
          )}
        </div>
      </header>

      <div className="profile-editor-grid">
        <div className="editor-column">
          {needsReview && (
            <section
              className="editor-panel profile-review-panel"
              aria-labelledby="profile-review-editor-title"
            >
              <div className="editor-panel-heading">
                <p className="eyebrow">Native launch quarantine</p>
                <h2 id="profile-review-editor-title">
                  Approve reviewed configuration
                </h2>
              </div>
              <p>
                Confirm the preserved portable values below. Missing or
                suspicious paths must be repaired before approval.
              </p>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={configurationReviewed}
                  onChange={(event) =>
                    setConfigurationReviewed(event.currentTarget.checked)
                  }
                />
                <span>
                  <strong>
                    I reviewed executable paths, arguments, working directories,
                    elevation, monitored executables, and stop settings.
                  </strong>
                </span>
              </label>
              {privilegedApplications.map((application) => (
                <label className="check-row" key={application.id}>
                  <input
                    type="checkbox"
                    checked={approvedPrivilegedApplicationIds.includes(
                      application.id,
                    )}
                    onChange={(event) => {
                      const isApproved = event.currentTarget.checked;
                      setApprovedPrivilegedApplicationIds((current) =>
                        isApproved
                          ? [...current, application.id]
                          : current.filter(
                              (applicationId) =>
                                applicationId !== application.id,
                            ),
                      );
                    }}
                  />
                  <span>
                    <strong>
                      Approve privileged launch settings for {application.name}
                    </strong>
                    <small>
                      {application.launchRecipe.elevated
                        ? "Elevated launch"
                        : "Ordinary launch"}
                      {application.launchRecipe.shutdownStrategy.kind ===
                      "customStop"
                        ? " · custom stop executable"
                        : ""}
                    </small>
                  </span>
                </label>
              ))}
              {pathsNeedRepair && (
                <div className="profile-repair-list" role="alert">
                  <strong>Select an executable for:</strong>
                  <ul>
                    {applicationsNeedingRepair.map((application) => (
                      <li key={application.id}>
                        {application.name} needs an executable path.
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </section>
          )}
          <section className="editor-panel" aria-labelledby="identity-title">
            <div className="editor-panel-heading">
              <p className="eyebrow">Profile</p>
              <h2 id="identity-title">Launch and Session behavior</h2>
            </div>
            <label className="field">
              <span>Profile name</span>
              <input
                required
                value={profile.name}
                onChange={(event) =>
                  update((next) => {
                    next.name = event.currentTarget.value;
                  })
                }
              />
            </label>
            <div className="settings-checks">
              <label className="check-row">
                <input
                  type="checkbox"
                  aria-label="VR enabled by default"
                  checked={profile.vrEnabled}
                  onChange={(event) =>
                    update((next) => {
                      next.vrEnabled = event.currentTarget.checked;
                    })
                  }
                />
                <span>
                  <strong>VR enabled by default</strong>
                  <small>Remember this choice on the Dashboard.</small>
                </span>
              </label>
              <label className="field compact-field">
                <span>Preferred VR Launch Mode</span>
                <select
                  value={profile.preferredVrLaunchMode ?? ""}
                  onChange={(event) =>
                    update((next) => {
                      const value = event.currentTarget.value;
                      next.preferredVrLaunchMode =
                        value === ""
                          ? null
                          : (value as NonNullable<
                              RacingProfile["preferredVrLaunchMode"]
                            >);
                    })
                  }
                >
                  <option value="openVr">OpenVR / SteamVR</option>
                  <option value="oculus">Oculus</option>
                </select>
              </label>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={profile.closeSession.stopSteamVr}
                  onChange={(event) =>
                    update((next) => {
                      next.closeSession.stopSteamVr =
                        event.currentTarget.checked;
                    })
                  }
                />
                <span>
                  <strong>Stop SteamVR on Close Session</strong>
                  <small>Only when this Session started SteamVR.</small>
                </span>
              </label>
            </div>
          </section>

          <section className="editor-panel" aria-labelledby="primary-sim-title">
            <div className="editor-panel-heading">
              <p className="eyebrow">Primary Sim</p>
              <h2 id="primary-sim-title">Game Launch Settings</h2>
            </div>
            <label className="field">
              <span>Primary Sim name</span>
              <input
                required
                value={profile.primarySim.name}
                onChange={(event) =>
                  update((next) => {
                    next.primarySim.name = event.currentTarget.value;
                  })
                }
              />
            </label>
            <ApplicationRecipeFields
              application={profile.primarySim}
              label="Primary Sim"
              onPickExecutablePath={onPickExecutablePath}
              onChange={(application) =>
                update((next) => {
                  next.primarySim = application;
                })
              }
            />
          </section>
        </div>

        <section
          className="editor-panel startup-editor"
          aria-labelledby="order-title"
        >
          <div className="editor-panel-heading order-heading">
            <div>
              <p className="eyebrow">Startup order</p>
              <h2 id="order-title">Supporting Applications</h2>
            </div>
            <button
              type="button"
              className="secondary-button"
              onClick={addSupportingApplication}
            >
              <PlusIcon />
              Add application
            </button>
          </div>

          <div className="supporting-editor-list">
            {profile.supportingApplications.length === 0 && (
              <div className="application-empty">
                <PlusIcon />
                <span>
                  <strong>No Supporting Applications</strong>
                  <small>Add them in the order they should start.</small>
                </span>
              </div>
            )}
            {profile.supportingApplications.map(
              (supportingApplication, index) => {
                const isOpen =
                  supportingApplication.application.id ===
                  openSupportingApplicationId;
                return (
                  <article
                    className={`supporting-editor-row${isOpen ? " is-open" : ""}${
                      dropTarget?.applicationId ===
                      supportingApplication.application.id
                        ? ` is-drop-target-${dropTarget.position}`
                        : ""
                    }${
                      draggedSupportingApplicationId ===
                      supportingApplication.application.id
                        ? " is-reordering"
                        : ""
                    }`}
                    key={supportingApplication.application.id}
                    data-supporting-application-id={
                      supportingApplication.application.id
                    }
                  >
                    <div className="supporting-row-heading">
                      <button
                        type="button"
                        className="supporting-drag-handle"
                        aria-label={`Reorder ${supportingApplication.application.name}. Use Up and Down arrow keys to move it.`}
                        onPointerDown={(event) => {
                          const row = event.currentTarget.closest<HTMLElement>(
                            ".supporting-editor-row",
                          );
                          if (!row) {
                            return;
                          }
                          const bounds = row.getBoundingClientRect();
                          activeReorderPointerId.current = event.pointerId;
                          lastReorderTarget.current = null;
                          event.currentTarget.setPointerCapture?.(
                            event.pointerId,
                          );
                          setDragPreview({
                            applicationId: supportingApplication.application.id,
                            x: bounds.left,
                            y: bounds.top,
                            width: bounds.width,
                            grabOffsetX: event.clientX - bounds.left,
                            grabOffsetY: event.clientY - bounds.top,
                          });
                          setDraggedSupportingApplicationId(
                            supportingApplication.application.id,
                          );
                        }}
                        onPointerMove={(event) => {
                          if (
                            activeReorderPointerId.current !== event.pointerId
                          ) {
                            return;
                          }
                          const nextDropTarget = dropTargetAt(
                            event.clientX,
                            event.clientY,
                          );
                          setDropTarget(nextDropTarget);
                          const reorderTargetKey = nextDropTarget
                            ? `${nextDropTarget.applicationId}:${nextDropTarget.position}`
                            : null;
                          if (
                            nextDropTarget &&
                            nextDropTarget.applicationId !==
                              supportingApplication.application.id &&
                            reorderTargetKey !== lastReorderTarget.current
                          ) {
                            reorderSupportingApplications(
                              supportingApplication.application.id,
                              nextDropTarget.applicationId,
                              nextDropTarget.position,
                            );
                            lastReorderTarget.current = reorderTargetKey;
                          }
                          setDragPreview((current) =>
                            current?.applicationId ===
                            supportingApplication.application.id
                              ? {
                                  ...current,
                                  x: event.clientX - current.grabOffsetX,
                                  y: event.clientY - current.grabOffsetY,
                                }
                              : current,
                          );
                        }}
                        onPointerUp={(event) => {
                          event.currentTarget.releasePointerCapture?.(
                            event.pointerId,
                          );
                          clearSupportingApplicationReorder(event.pointerId);
                        }}
                        onPointerCancel={(event) => {
                          clearSupportingApplicationReorder(event.pointerId);
                        }}
                        onLostPointerCapture={(event) => {
                          clearSupportingApplicationReorder(event.pointerId);
                        }}
                        onKeyDown={(event) => {
                          if (event.key === "ArrowUp" && index > 0) {
                            event.preventDefault();
                            moveSupportingApplication(index, -1);
                          }
                          if (
                            event.key === "ArrowDown" &&
                            index < profile.supportingApplications.length - 1
                          ) {
                            event.preventDefault();
                            moveSupportingApplication(index, 1);
                          }
                        }}
                      >
                        <span aria-hidden="true" className="drag-dots">
                          <i />
                          <i />
                          <i />
                          <i />
                          <i />
                          <i />
                        </span>
                      </button>
                      <button
                        type="button"
                        className="supporting-editor-toggle"
                        aria-label={`Edit ${supportingApplication.application.name}`}
                        onClick={() =>
                          setOpenSupportingApplicationId((current) =>
                            current === supportingApplication.application.id
                              ? null
                              : supportingApplication.application.id,
                          )
                        }
                      >
                        <span
                          className="supporting-application-icon"
                          aria-hidden="true"
                        >
                          {profileApplicationIcon(
                            supportingApplication.application.id,
                            applicationIcons,
                            <FlagIcon />,
                          )}
                        </span>
                        <span className="supporting-editor-toggle-copy">
                          <strong>
                            {supportingApplication.application.name}
                          </strong>
                        </span>
                      </button>
                      <span className="requirement-chip">
                        {supportingApplication.requirement === "required"
                          ? "Required"
                          : "Optional"}
                      </span>
                      <button
                        type="button"
                        className="supporting-overflow-button"
                        aria-expanded={isOpen}
                        aria-label={`${supportingApplication.application.name} Supporting Application editor`}
                        onClick={() =>
                          setOpenSupportingApplicationId((current) =>
                            current === supportingApplication.application.id
                              ? null
                              : supportingApplication.application.id,
                          )
                        }
                      >
                        <span aria-hidden="true">…</span>
                      </button>
                      <button
                        type="button"
                        className="supporting-remove-button"
                        aria-label={`Remove ${supportingApplication.application.name}`}
                        onClick={() => removeSupportingApplication(index)}
                      >
                        <span aria-hidden="true">×</span>
                      </button>
                    </div>

                    {isOpen && (
                      <div className="supporting-editor-content">
                        <label className="field inline-name-field">
                          <span>Application Name</span>
                          <input
                            required
                            value={supportingApplication.application.name}
                            onChange={(event) =>
                              updateSupportingApplication(
                                index,
                                (supporting) => {
                                  supporting.application.name =
                                    event.currentTarget.value;
                                },
                              )
                            }
                          />
                        </label>
                        <div className="supporting-policy">
                          <label className="field compact-field">
                            <span>Launch Requirement</span>
                            <select
                              value={supportingApplication.requirement}
                              onChange={(event) =>
                                updateSupportingApplication(
                                  index,
                                  (supporting) => {
                                    supporting.requirement = event.currentTarget
                                      .value as SupportingApplication["requirement"];
                                  },
                                )
                              }
                            >
                              <option value="required">Required</option>
                              <option value="optional">Optional</option>
                            </select>
                          </label>
                          <label className="field compact-field">
                            <span>Shutdown Method</span>
                            <select
                              value={
                                supportingApplication.application.launchRecipe
                                  .shutdownStrategy.kind
                              }
                              onChange={(event) =>
                                updateSupportingApplication(
                                  index,
                                  (supporting) => {
                                    switch (event.currentTarget.value) {
                                      case "consoleInterrupt":
                                        supporting.application.launchRecipe.shutdownStrategy =
                                          {
                                            kind: "consoleInterrupt",
                                          };
                                        break;
                                      case "customStop":
                                        supporting.application.launchRecipe.shutdownStrategy =
                                          {
                                            kind: "customStop",
                                            executablePath: "",
                                            arguments: [],
                                          };
                                        break;
                                      case "forceOnly":
                                        supporting.application.launchRecipe.shutdownStrategy =
                                          {
                                            kind: "forceOnly",
                                          };
                                        break;
                                      default:
                                        supporting.application.launchRecipe.shutdownStrategy =
                                          {
                                            kind: "closeWindows",
                                          };
                                    }
                                  },
                                )
                              }
                            >
                              <option value="closeWindows">
                                Close windows
                              </option>
                              <option value="consoleInterrupt">
                                Console interrupt
                              </option>
                              <option value="customStop">
                                Custom stop executable
                              </option>
                              <option value="forceOnly">
                                No graceful strategy
                              </option>
                            </select>
                          </label>
                          <label className="check-row compact-check">
                            <input
                              type="checkbox"
                              checked={supportingApplication.keepRunning}
                              onChange={(event) =>
                                updateSupportingApplication(
                                  index,
                                  (supporting) => {
                                    supporting.keepRunning =
                                      event.currentTarget.checked;
                                  },
                                )
                              }
                            />
                            <span>
                              <strong>
                                Keep {supportingApplication.application.name}{" "}
                                running
                              </strong>
                              <small>Detach it after Close Session.</small>
                            </span>
                          </label>
                        </div>

                        <ApplicationRecipeFields
                          application={supportingApplication.application}
                          label={supportingApplication.application.name}
                          includeShutdownStrategy={false}
                          onPickExecutablePath={onPickExecutablePath}
                          onChange={(application) =>
                            updateSupportingApplication(index, (supporting) => {
                              supporting.application = application;
                            })
                          }
                        />
                      </div>
                    )}
                  </article>
                );
              },
            )}
          </div>

          <div className="game-divider">
            <span />
            <small>Primary Sim · locked last</small>
          </div>
          <div className="application-row game-row locked-game-row">
            <span className="application-icon game-icon">
              {profileApplicationIcon(
                profile.primarySim.id,
                applicationIcons,
                <FlagIcon />,
              )}
            </span>
            <span className="application-copy">
              <strong>{profile.primarySim.name}</strong>
              <small>Always launches after Supporting Applications</small>
            </span>
            <span className="locked-label">Locked</span>
          </div>
        </section>
      </div>
      {dragPreview &&
        previewSupportingApplication &&
        createPortal(
          <div
            aria-hidden="true"
            className="supporting-drag-preview"
            style={{
              transform: `translate3d(${dragPreview.x}px, ${dragPreview.y}px, 0)`,
              width: dragPreview.width,
            }}
          >
            <span className="supporting-drag-preview-handle">
              <span className="drag-dots">
                <i />
                <i />
                <i />
                <i />
                <i />
                <i />
              </span>
            </span>
            <span className="supporting-application-icon">
              {profileApplicationIcon(
                previewSupportingApplication.application.id,
                applicationIcons,
                <FlagIcon />,
              )}
            </span>
            <span className="supporting-editor-toggle-copy">
              <strong>{previewSupportingApplication.application.name}</strong>
            </span>
            <span className="requirement-chip">
              {previewSupportingApplication.requirement === "required"
                ? "Required"
                : "Optional"}
            </span>
            <span className="supporting-drag-preview-action">…</span>
            <span className="supporting-drag-preview-remove">×</span>
          </div>,
          document.body,
        )}
    </form>
  );
}

interface ApplicationRecipeFieldsProps {
  application: ProfileApplication;
  label: string;
  includeShutdownStrategy?: boolean;
  onPickExecutablePath(initialPath?: string | null): Promise<string | null>;
  onChange(application: ProfileApplication): void;
}

function ApplicationRecipeFields({
  application,
  label,
  includeShutdownStrategy = true,
  onPickExecutablePath,
  onChange,
}: ApplicationRecipeFieldsProps) {
  const update = (change: (next: ProfileApplication) => void) => {
    const next = structuredClone(application);
    change(next);
    onChange(next);
  };
  const source = application.launchRecipe.source;
  const shutdown = application.launchRecipe.shutdownStrategy;
  const setDirectExecutable = (next: ProfileApplication, path: string) => {
    const nextSource = next.launchRecipe.source;
    if (nextSource.kind !== "directExecutable") {
      return;
    }
    nextSource.executablePath = path;
    next.pathNeedsRepair = path.length === 0;
    next.launchRecipe.workingDirectory = directoryFromPath(path);
    next.launchRecipe.monitoredProcess = executableNameFromPath(path);
    next.launchRecipe.monitoredExecutablePath = path || null;
  };
  const selectExecutable = async (
    initialPath: string | null,
    change: (next: ProfileApplication, path: string) => void,
  ) => {
    const path = await onPickExecutablePath(initialPath);
    if (path) {
      update((next) => change(next, path));
    }
  };

  return (
    <>
      <div className="recipe-source-fields">
        <div className="field-grid">
          <label className="field">
            <span>{label} source</span>
            <select
              value={source.kind}
              onChange={(event) =>
                update((next) => {
                  next.launchRecipe.source =
                    event.currentTarget.value === "steam"
                      ? { kind: "steam", appId: 0, selector: null }
                      : { kind: "directExecutable", executablePath: "" };
                  next.pathNeedsRepair =
                    next.launchRecipe.source.kind === "directExecutable";
                })
              }
            >
              <option value="directExecutable">Direct executable</option>
              <option value="steam">Steam</option>
            </select>
          </label>
          <label className="field">
            <span>
              {source.kind === "steam"
                ? `${label} Steam App ID`
                : `${label} executable path`}
            </span>
            <div className="path-input">
              <input
                value={
                  source.kind === "steam"
                    ? String(source.appId || "")
                    : displayWindowsPath(source.executablePath)
                }
                onChange={(event) =>
                  update((next) => {
                    const nextSource = next.launchRecipe.source;
                    if (nextSource.kind === "steam") {
                      nextSource.appId =
                        Number.parseInt(event.currentTarget.value, 10) || 0;
                      next.pathNeedsRepair = false;
                    } else {
                      setDirectExecutable(next, event.currentTarget.value);
                    }
                  })
                }
              />
              {source.kind === "directExecutable" && (
                <button
                  type="button"
                  className="secondary-button path-browse-button"
                  aria-label={`Browse for ${label} executable`}
                  onClick={() =>
                    void selectExecutable(
                      source.kind === "directExecutable"
                        ? source.executablePath
                        : null,
                      (next, path) => {
                        const nextSource = next.launchRecipe.source;
                        if (nextSource.kind !== "directExecutable") {
                          return;
                        }
                        setDirectExecutable(next, path);
                      },
                    )
                  }
                >
                  Browse
                </button>
              )}
            </div>
          </label>
        </div>
      </div>
      <details className="recipe-details">
        <summary>Launch Arguments</summary>
        <div className="recipe-fields">
          <ArgumentList
            label={`${label} arguments`}
            arguments={application.launchRecipe.arguments}
            onChange={(nextArguments) =>
              update((next) => {
                next.launchRecipe.arguments = nextArguments;
              })
            }
          />
        </div>
      </details>
      {source.kind === "steam" && (
        <details className="recipe-subsection">
          <summary>Steam launch settings</summary>
          <div className="recipe-subsection-content field-grid">
            <label className="field compact-field">
              <span>{label} Steam launch option</span>
              <select
                value={source.selector?.kind ?? "curated"}
                onChange={(event) =>
                  update((next) => {
                    const nextSource = next.launchRecipe.source;
                    if (nextSource.kind !== "steam") {
                      return;
                    }
                    switch (event.currentTarget.value) {
                      case "default":
                        nextSource.selector = { kind: "default" };
                        break;
                      case "openVr":
                        nextSource.selector = { kind: "openVr" };
                        break;
                      case "oculus":
                        nextSource.selector = { kind: "oculus" };
                        break;
                      case "option":
                        nextSource.selector = { kind: "option", index: 1 };
                        break;
                      default:
                        nextSource.selector = null;
                    }
                  })
                }
              >
                <option value="curated">Use Curated Catalog</option>
                <option value="default">Default</option>
                <option value="openVr">OpenVR / SteamVR</option>
                <option value="oculus">Oculus</option>
                <option value="option">Numbered launch option</option>
              </select>
            </label>
            {source.selector?.kind === "option" && (
              <label className="field compact-field">
                <span>Launch option index</span>
                <input
                  type="number"
                  min="0"
                  max="255"
                  value={source.selector.index}
                  onChange={(event) =>
                    update((next) => {
                      const nextSource = next.launchRecipe.source;
                      if (
                        nextSource.kind === "steam" &&
                        nextSource.selector?.kind === "option"
                      ) {
                        nextSource.selector.index = Math.min(
                          255,
                          Math.max(0, event.currentTarget.valueAsNumber || 0),
                        );
                      }
                    })
                  }
                />
              </label>
            )}
          </div>
        </details>
      )}
      {source.kind === "steam" && (
        <details className="recipe-subsection">
          <summary>Process matching</summary>
          <div className="recipe-subsection-content">
            <div className="field-grid">
              <label className="field compact-field">
                <span>{label} working directory</span>
                <input
                  value={application.launchRecipe.workingDirectory ?? ""}
                  onChange={(event) =>
                    update((next) => {
                      next.launchRecipe.workingDirectory =
                        event.currentTarget.value || null;
                    })
                  }
                />
              </label>
              <label className="field compact-field">
                <span>{label} monitored process</span>
                <input
                  value={application.launchRecipe.monitoredProcess ?? ""}
                  onChange={(event) =>
                    update((next) => {
                      next.launchRecipe.monitoredProcess =
                        event.currentTarget.value || null;
                      next.launchRecipe.monitoredExecutablePath = null;
                    })
                  }
                />
              </label>
            </div>
            <label className="field">
              <span>{label} monitored executable path</span>
              <div className="path-input">
                <input
                  value={displayWindowsPath(
                    application.launchRecipe.monitoredExecutablePath ?? "",
                  )}
                  onChange={(event) =>
                    update((next) => {
                      next.launchRecipe.monitoredExecutablePath =
                        event.currentTarget.value || null;
                    })
                  }
                />
                <button
                  type="button"
                  className="secondary-button path-browse-button"
                  aria-label={`Browse for ${label} monitored executable`}
                  onClick={() =>
                    void selectExecutable(
                      application.launchRecipe.monitoredExecutablePath ?? null,
                      (next, path) => {
                        next.launchRecipe.monitoredExecutablePath = path;
                      },
                    )
                  }
                >
                  Browseâ€¦
                </button>
              </div>
              <small>
                Required before a launcher-discovered process can be
                Session-owned. Test Game Launch can learn this path for review.
              </small>
            </label>
          </div>
        </details>
      )}
      <details className="recipe-subsection recipe-runtime-settings">
        <summary>Shutdown and advanced launch settings</summary>
        <div className="recipe-subsection-content">
          {includeShutdownStrategy && (
            <label className="field recipe-shutdown-field">
              <span>Shutdown Method</span>
              <select
                value={shutdown.kind}
                onChange={(event) =>
                  update((next) => {
                    switch (event.currentTarget.value) {
                      case "consoleInterrupt":
                        next.launchRecipe.shutdownStrategy = {
                          kind: "consoleInterrupt",
                        };
                        break;
                      case "customStop":
                        next.launchRecipe.shutdownStrategy = {
                          kind: "customStop",
                          executablePath: "",
                          arguments: [],
                        };
                        break;
                      case "forceOnly":
                        next.launchRecipe.shutdownStrategy = {
                          kind: "forceOnly",
                        };
                        break;
                      default:
                        next.launchRecipe.shutdownStrategy = {
                          kind: "closeWindows",
                        };
                    }
                  })
                }
              >
                <option value="closeWindows">Close windows</option>
                <option value="consoleInterrupt">Console interrupt</option>
                <option value="customStop">Custom stop executable</option>
                <option value="forceOnly">No graceful strategy</option>
              </select>
            </label>
          )}
          <div className="recipe-launch-flags">
            <label className="check-row compact-check">
              <input
                type="checkbox"
                checked={application.launchRecipe.elevated}
                onChange={(event) =>
                  update((next) => {
                    next.launchRecipe.elevated = event.currentTarget.checked;
                  })
                }
              />
              <span>
                <strong>Launch elevated</strong>
                <small>
                  Uses the one-shot helper when elevation is required.
                </small>
              </span>
            </label>
          </div>
          <div className="recipe-number-grid">
            <label className="field compact-field">
              <span>Startup timeout · seconds</span>
              <input
                type="number"
                min="1"
                value={application.launchRecipe.startupTimeoutSeconds}
                onChange={(event) =>
                  update((next) => {
                    next.launchRecipe.startupTimeoutSeconds =
                      event.currentTarget.valueAsNumber || 30;
                  })
                }
              />
            </label>
            <label className="field compact-field">
              <span>Post-start delay · ms</span>
              <input
                type="number"
                min="0"
                value={application.launchRecipe.postStartDelayMilliseconds}
                onChange={(event) =>
                  update((next) => {
                    next.launchRecipe.postStartDelayMilliseconds =
                      event.currentTarget.valueAsNumber || 0;
                  })
                }
              />
            </label>
            <label className="check-row compact-check">
              <input
                type="checkbox"
                aria-label="Hide console"
                checked={
                  application.launchRecipe.consoleVisibility === "hidden"
                }
                onChange={(event) =>
                  update((next) => {
                    next.launchRecipe.consoleVisibility = event.currentTarget
                      .checked
                      ? "hidden"
                      : "visible";
                  })
                }
              />
              <span>
                <strong>Hide console</strong>
                <small>
                  Keep a console window out of the way while it runs.
                </small>
              </span>
            </label>
          </div>
        </div>
      </details>
      {shutdown.kind === "customStop" && (
        <div className="field-grid">
          <label className="field">
            <span>Stop executable path</span>
            <div className="path-input">
              <input
                value={displayWindowsPath(shutdown.executablePath)}
                onChange={(event) =>
                  update((next) => {
                    const nextShutdown = next.launchRecipe.shutdownStrategy;
                    if (nextShutdown.kind === "customStop") {
                      nextShutdown.executablePath = event.currentTarget.value;
                    }
                  })
                }
              />
              <button
                type="button"
                className="secondary-button path-browse-button"
                aria-label={`Browse for ${label} stop executable`}
                onClick={() =>
                  void selectExecutable(
                    shutdown.kind === "customStop"
                      ? shutdown.executablePath
                      : null,
                    (next, path) => {
                      const nextShutdown = next.launchRecipe.shutdownStrategy;
                      if (nextShutdown.kind === "customStop") {
                        nextShutdown.executablePath = path;
                      }
                    },
                  )
                }
              >
                Browseâ€¦
              </button>
            </div>
          </label>
          <ArgumentList
            label="Stop arguments"
            arguments={shutdown.arguments}
            onChange={(nextArguments) =>
              update((next) => {
                const nextShutdown = next.launchRecipe.shutdownStrategy;
                if (nextShutdown.kind === "customStop") {
                  nextShutdown.arguments = nextArguments;
                }
              })
            }
          />
        </div>
      )}
    </>
  );
}

interface ArgumentListProps {
  label: string;
  arguments: string[];
  onChange(nextArguments: string[]): void;
}

function ArgumentList({
  label,
  arguments: values,
  onChange,
}: ArgumentListProps) {
  const removeArgument = (index: number) => {
    onChange(values.filter((_, candidateIndex) => candidateIndex !== index));
  };

  return (
    <section className="argument-list" aria-label={label}>
      <div className="argument-list-heading">
        <span>{label}</span>
        <button
          type="button"
          className="tertiary-button argument-add-button"
          aria-label={`Add ${label.slice(0, -1)}`}
          onClick={() => onChange([...values, ""])}
        >
          <PlusIcon />
          Add argument
        </button>
      </div>
      {values.length > 0 ? (
        <div className="argument-list-rows">
          {values.map((argument, index) => (
            <div className="argument-row" key={`argument-${index}`}>
              <input
                aria-label={`${label} ${index + 1}`}
                value={argument}
                onChange={(event) =>
                  onChange(
                    values.map((value, candidateIndex) =>
                      candidateIndex === index
                        ? event.currentTarget.value
                        : value,
                    ),
                  )
                }
                onBlur={() => {
                  if (argument.length === 0) {
                    removeArgument(index);
                  }
                }}
              />
              <button
                type="button"
                className="tertiary-button argument-remove-button"
                aria-label={`Remove ${label.slice(0, -1)} ${index + 1}`}
                onClick={() => removeArgument(index)}
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      ) : (
        <p className="argument-list-empty">No arguments.</p>
      )}
    </section>
  );
}
