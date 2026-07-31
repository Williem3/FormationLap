import type { AppSnapshot } from "../../generated/bindings";
import { ModalDialog } from "../../ui/ModalDialog";
import type { useDashboardController } from "./useDashboardController";

type DashboardDialogController = Pick<
  ReturnType<typeof useDashboardController>,
  | "pendingProcessAction"
  | "setPendingProcessAction"
  | "outputApplication"
  | "setOutputApplication"
  | "dashboardError"
  | "dashboardIsBusy"
  | "confirmProcessAction"
  | "confirmNativeProcessAction"
  | "cancelProcessAction"
>;

interface DashboardDialogsProps {
  controller: DashboardDialogController;
  snapshot: AppSnapshot | null;
}

export function DashboardDialogs({
  controller,
  snapshot,
}: DashboardDialogsProps) {
  const {
    pendingProcessAction,
    setPendingProcessAction,
    outputApplication,
    setOutputApplication,
    dashboardError,
    dashboardIsBusy,
    confirmNativeProcessAction,
    confirmProcessAction,
    cancelProcessAction,
  } = controller;
  const nativeConfirmation = snapshot?.pendingProcessConfirmation;
  const confirmedApplication = nativeConfirmation
    ? snapshot?.selectedProfile?.supportingApplications
        .map((supporting) => supporting.application)
        .concat(snapshot.selectedProfile.primarySim)
        .find((application) => application.id === nativeConfirmation.applicationId)
    : null;

  return (
    <>
      {nativeConfirmation && confirmedApplication ? (
        <ModalDialog
          labelledBy="process-confirmation-title"
          onClose={() => void cancelProcessAction(nativeConfirmation.token)}
        >
          <p className="eyebrow">Force termination</p>
          <h2 id="process-confirmation-title">
            {nativeConfirmation.action === "restart"
              ? `Force stop and restart ${confirmedApplication.name}?`
              : `Force stop ${confirmedApplication.name}?`}
          </h2>
          <p>
            Graceful shutdown did not complete. Force stopping this exact Process may lose unsaved work.
          </p>
          <div className="dialog-actions">
            <button type="button" className="secondary-button" onClick={() => void cancelProcessAction(nativeConfirmation.token)}>Cancel</button>
            <button type="button" className="danger-button" disabled={dashboardIsBusy} onClick={() => void confirmNativeProcessAction(nativeConfirmation.token)}>
              {nativeConfirmation.action === "restart" ? `Force stop and restart ${confirmedApplication.name}` : `Force stop ${confirmedApplication.name}`}
            </button>
          </div>
        </ModalDialog>
      ) : pendingProcessAction && (
        <ModalDialog
          labelledBy="process-confirmation-title"
          onClose={() => setPendingProcessAction(null)}
        >
          <p className="eyebrow">
            {pendingProcessAction.kind === "force"
              ? "Force termination"
              : "Ownership confirmation"}
          </p>
          <h2 id="process-confirmation-title">
            {pendingProcessAction.kind === "force"
              ? `Force stop ${pendingProcessAction.application.name}?`
              : `${pendingProcessAction.kind === "restart" ? "Restart" : "Control"} a Pre-existing Process?`}
          </h2>
          <p>
            {pendingProcessAction.kind === "force"
              ? `Graceful shutdown did not complete. Force stopping ${pendingProcessAction.application.name} may lose unsaved work.`
              : `${pendingProcessAction.application.name} was already running before Formation Lap observed it. This explicit action will control a Process that the current Session does not own.`}
          </p>
          {dashboardError && (
            <p className="form-error" role="alert">
              {dashboardError}
            </p>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setPendingProcessAction(null)}
            >
              Cancel
            </button>
            <button
              type="button"
              className={
                pendingProcessAction.kind === "force"
                  ? "danger-button"
                  : "primary-button"
              }
              disabled={dashboardIsBusy}
              onClick={confirmProcessAction}
            >
              {pendingProcessAction.kind === "force"
                ? `Force stop ${pendingProcessAction.application.name}`
                : `${pendingProcessAction.kind === "restart" ? "Restart" : "Exit"} ${pendingProcessAction.application.name}`}
            </button>
          </div>
        </ModalDialog>
      )}

      {outputApplication &&
        (() => {
          const processOutput = snapshot?.applicationProcesses.find(
            (process) => process.applicationId === outputApplication.id,
          )?.output;
          return (
            <ModalDialog
              className="console-dialog"
              labelledBy="console-output-title"
              onClose={() => setOutputApplication(null)}
            >
              <p className="eyebrow">Bounded local output</p>
              <h2 id="console-output-title">{outputApplication.name} output</h2>
              <p>
                Formation Lap keeps only the most recent local stdout and stderr
                tail.
              </p>
              <pre className="console-output">
                {processOutput
                  ? [
                      processOutput.stdout,
                      processOutput.stderr,
                      processOutput.truncated
                        ? "\n[Earlier output was discarded.]"
                        : "",
                    ].join("")
                  : "No captured output."}
              </pre>
              <div className="dialog-actions">
                <button
                  type="button"
                  className="primary-button"
                  onClick={() => setOutputApplication(null)}
                >
                  Close output
                </button>
              </div>
            </ModalDialog>
          );
        })()}
    </>
  );
}
