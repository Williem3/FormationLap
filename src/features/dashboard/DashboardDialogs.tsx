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
    confirmProcessAction,
  } = controller;

  return (
    <>
      {pendingProcessAction && (
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
