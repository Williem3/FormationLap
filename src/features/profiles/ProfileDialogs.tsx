import type { RefObject } from "react";
import type { RacingProfile } from "../../generated/bindings";
import { ModalDialog } from "../../ui/ModalDialog";
import type { useProfileWorkspace } from "./useProfileWorkspace";

type ProfileDialogWorkspace = Pick<
  ReturnType<typeof useProfileWorkspace>,
  | "duplicateName"
  | "setDuplicateName"
  | "isDuplicateOpen"
  | "setIsDuplicateOpen"
  | "isDeleteOpen"
  | "setIsDeleteOpen"
  | "isExportOpen"
  | "setIsExportOpen"
  | "exportDocument"
  | "isImportOpen"
  | "setIsImportOpen"
  | "importDocument"
  | "setImportDocument"
  | "profileIsSaving"
  | "profileError"
  | "duplicateProfile"
  | "deleteProfile"
  | "importProfile"
>;

interface ProfileDialogsProps {
  workspace: ProfileDialogWorkspace;
  selectedProfile: RacingProfile | null;
  newProfileButton: RefObject<HTMLButtonElement | null>;
}

export function ProfileDialogs({
  workspace,
  selectedProfile,
  newProfileButton,
}: ProfileDialogsProps) {
  const {
    duplicateName,
    setDuplicateName,
    isDuplicateOpen,
    setIsDuplicateOpen,
    isDeleteOpen,
    setIsDeleteOpen,
    isExportOpen,
    setIsExportOpen,
    exportDocument,
    isImportOpen,
    setIsImportOpen,
    importDocument,
    setImportDocument,
    profileIsSaving,
    profileError,
    duplicateProfile,
    deleteProfile,
    importProfile,
  } = workspace;

  return (
    <>
      {isDuplicateOpen && selectedProfile && (
        <ModalDialog
          labelledBy="duplicate-profile-title"
          onClose={() => setIsDuplicateOpen(false)}
        >
          <p className="eyebrow">Profile action</p>
          <h2 id="duplicate-profile-title">Duplicate {selectedProfile.name}</h2>
          <p>
            Create an independent copy with the same startup order and settings.
          </p>
          <form onSubmit={duplicateProfile}>
            <label className="field">
              <span>Duplicate name</span>
              <input
                autoFocus
                required
                value={duplicateName}
                onChange={(event) =>
                  setDuplicateName(event.currentTarget.value)
                }
              />
            </label>
            {profileError && (
              <p className="form-error" role="alert">
                {profileError}
              </p>
            )}
            <div className="dialog-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setIsDuplicateOpen(false)}
              >
                Cancel
              </button>
              <button
                type="submit"
                className="primary-button"
                disabled={profileIsSaving}
              >
                {profileIsSaving ? "Duplicating…" : "Create duplicate"}
              </button>
            </div>
          </form>
        </ModalDialog>
      )}

      {isDeleteOpen && selectedProfile && (
        <ModalDialog
          labelledBy="delete-profile-title"
          onClose={() => setIsDeleteOpen(false)}
          returnFocusRef={newProfileButton}
        >
          <p className="eyebrow">Destructive action</p>
          <h2 id="delete-profile-title">Delete {selectedProfile.name}?</h2>
          <p>
            This removes the Racing Profile from your library. Formation Lap
            keeps a bounded local backup for recovery.
          </p>
          {profileError && (
            <p className="form-error" role="alert">
              {profileError}
            </p>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setIsDeleteOpen(false)}
            >
              Cancel
            </button>
            <button
              type="button"
              className="danger-button"
              disabled={profileIsSaving}
              onClick={() => void deleteProfile()}
            >
              {profileIsSaving ? "Deleting…" : `Delete ${selectedProfile.name}`}
            </button>
          </div>
        </ModalDialog>
      )}

      {isExportOpen && selectedProfile && (
        <ModalDialog
          className="transfer-dialog"
          labelledBy="export-profile-title"
          onClose={() => setIsExportOpen(false)}
        >
          <p className="eyebrow">Portable profile</p>
          <h2 id="export-profile-title">Export {selectedProfile.name}</h2>
          <p>
            Copy this JSON into a local text file. Machine-specific paths are
            marked for repair when imported elsewhere.
          </p>
          <label className="field">
            <span>Portable profile JSON</span>
            <textarea
              readOnly
              rows={12}
              value={exportDocument}
              aria-busy={profileIsSaving}
            />
          </label>
          {profileError && (
            <p className="form-error" role="alert">
              {profileError}
            </p>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="primary-button"
              onClick={() => setIsExportOpen(false)}
            >
              Close export
            </button>
          </div>
        </ModalDialog>
      )}

      {isImportOpen && (
        <ModalDialog
          className="transfer-dialog"
          labelledBy="import-profile-title"
          onClose={() => setIsImportOpen(false)}
        >
          <p className="eyebrow">Portable profile</p>
          <h2 id="import-profile-title">Import Racing Profile</h2>
          <p>
            Paste a Formation Lap profile document. A fresh local identity is
            assigned and missing paths remain visible for repair.
          </p>
          <form onSubmit={importProfile}>
            <label className="field">
              <span>Portable profile JSON</span>
              <textarea
                autoFocus
                required
                rows={12}
                value={importDocument}
                onChange={(event) =>
                  setImportDocument(event.currentTarget.value)
                }
              />
            </label>
            {profileError && (
              <p className="form-error" role="alert">
                {profileError}
              </p>
            )}
            <div className="dialog-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setIsImportOpen(false)}
              >
                Cancel
              </button>
              <button
                type="submit"
                className="primary-button"
                disabled={profileIsSaving}
              >
                {profileIsSaving ? "Importing…" : "Import Racing Profile"}
              </button>
            </div>
          </form>
        </ModalDialog>
      )}
    </>
  );
}
