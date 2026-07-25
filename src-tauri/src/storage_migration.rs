use crate::{CoreError, FormationLapCore};
use std::{
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
};

pub(crate) fn prepare_local_storage(
    local_storage: &Path,
    roaming_storage: &Path,
) -> Result<(), CoreError> {
    if directory_has_entries(local_storage)? || !directory_has_entries(roaming_storage)? {
        return Ok(());
    }

    let parent = local_storage.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "local storage must have a parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;
    let local_name = local_storage.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "local storage must have a final path component",
        )
    })?;
    let temporary = parent.join(format!(
        ".{}.migration-{}",
        local_name.to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    let mut activation = PendingActivation::create(temporary)?;

    copy_directory_safely(roaming_storage, activation.path())?;
    validate_documents(activation.path())?;
    drop(FormationLapCore::open(activation.path())?);

    if local_storage.exists() {
        if directory_has_entries(local_storage)? {
            return Ok(());
        }
        fs::remove_dir(local_storage)?;
    }
    match fs::rename(activation.path(), local_storage) {
        Ok(()) => activation.commit(),
        Err(_error) if directory_has_entries(local_storage).unwrap_or(false) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn directory_has_entries(path: &Path) -> io::Result<bool> {
    match fs::read_dir(path) {
        Ok(mut entries) => Ok(entries.next().transpose()?.is_some()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn copy_directory_safely(source: &Path, destination: &Path) -> Result<(), CoreError> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() || metadata_is_reparse_point(&metadata) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "storage migration refuses linked entry {}",
                    entry.path().display()
                ),
            )
            .into());
        }
        let destination_path = destination.join(entry.file_name());
        if metadata.is_dir() {
            fs::create_dir(&destination_path)?;
            copy_directory_safely(&entry.path(), &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(entry.path(), &destination_path)?;
            OpenOptions::new()
                .write(true)
                .open(&destination_path)?
                .sync_all()?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "storage migration refuses non-file entry {}",
                    entry.path().display()
                ),
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn validate_documents(storage_root: &Path) -> Result<(), CoreError> {
    validate_directory_documents(storage_root)
}

fn validate_directory_documents(directory: &Path) -> Result<(), CoreError> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            validate_directory_documents(&path)?;
            continue;
        }
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("json") => {
                serde_json::from_slice::<serde_json::Value>(&fs::read(&path)?).map_err(
                    |error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("{} is not valid JSON: {error}", path.display()),
                        )
                    },
                )?;
            }
            Some("jsonl") => {
                let document = fs::read_to_string(&path)?;
                for (index, line) in document.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    serde_json::from_str::<serde_json::Value>(line).map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "{} line {} is not valid JSON: {error}",
                                path.display(),
                                index + 1
                            ),
                        )
                    })?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

struct PendingActivation {
    path: PathBuf,
    committed: bool,
}

impl PendingActivation {
    fn create(path: PathBuf) -> io::Result<Self> {
        fs::create_dir(&path)?;
        Ok(Self {
            path,
            committed: false,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn commit(&mut self) -> Result<(), CoreError> {
        self.committed = true;
        Ok(())
    }
}

impl Drop for PendingActivation {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
