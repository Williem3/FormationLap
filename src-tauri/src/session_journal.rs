use crate::{
    ApplicationProcessSnapshot, CoreError, SessionSnapshot,
    atomic_file::{recover_live_document, replace_with_backup},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const SESSION_JOURNAL_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionJournalSnapshot {
    pub(crate) session: SessionSnapshot,
    pub(crate) application_processes: Vec<ApplicationProcessSnapshot>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionJournalDocument {
    schema_version: u32,
    session: SessionSnapshot,
    application_processes: Vec<ApplicationProcessSnapshot>,
}

pub(crate) struct SessionJournal {
    backup_path: PathBuf,
    journal_path: PathBuf,
}

impl SessionJournal {
    pub(crate) fn open(storage_root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let storage_root = storage_root.as_ref();
        let backups_directory = storage_root.join("backups");
        fs::create_dir_all(&backups_directory)?;
        let journal_path = storage_root.join("active-session.json");
        let backup_path = backups_directory.join("active-session.json");
        let temporary_path = journal_path.with_extension("json.tmp");
        if temporary_path.exists() {
            fs::remove_file(temporary_path)?;
        }
        Ok(Self {
            backup_path,
            journal_path,
        })
    }

    pub(crate) fn load(&self) -> Result<Option<SessionJournalSnapshot>, CoreError> {
        if !self.journal_path.exists() {
            return Ok(None);
        }
        let document = recover_live_document(
            &self.journal_path,
            &self.backup_path,
            Self::load_document,
            |error| matches!(error, CoreError::UnsupportedSessionJournalSchema(_)),
        )?;
        Ok(Some(SessionJournalSnapshot {
            session: document.session,
            application_processes: document.application_processes,
        }))
    }

    fn load_document(path: &Path) -> Result<SessionJournalDocument, CoreError> {
        let document: SessionJournalDocument =
            serde_json::from_slice(&fs::read(path)?).map_err(CoreError::InvalidSessionJournal)?;
        if document.schema_version != SESSION_JOURNAL_SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSessionJournalSchema(
                document.schema_version,
            ));
        }
        Ok(document)
    }

    pub(crate) fn persist(
        &self,
        session: &SessionSnapshot,
        application_processes: &[ApplicationProcessSnapshot],
    ) -> Result<(), CoreError> {
        let document = SessionJournalDocument {
            schema_version: SESSION_JOURNAL_SCHEMA_VERSION,
            session: session.clone(),
            application_processes: application_processes.to_vec(),
        };
        let temporary_path = self.journal_path.with_extension("json.tmp");
        if temporary_path.exists() {
            fs::remove_file(&temporary_path)?;
        }
        let mut serialized =
            serde_json::to_vec_pretty(&document).map_err(CoreError::InvalidSessionJournal)?;
        serialized.push(b'\n');
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);

        if self.journal_path.exists() {
            replace_with_backup(&self.journal_path, &temporary_path, &self.backup_path)?;
        } else {
            fs::rename(&temporary_path, &self.journal_path)?;
        }
        Ok(())
    }

    pub(crate) fn clear(&self) -> Result<(), CoreError> {
        if self.journal_path.exists() {
            fs::remove_file(&self.journal_path)?;
        }
        if self.backup_path.exists() {
            fs::remove_file(&self.backup_path)?;
        }
        Ok(())
    }
}
