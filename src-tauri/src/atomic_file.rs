use crate::CoreError;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const CORRUPT_ARTIFACT_SLOTS: usize = 2;

pub(crate) fn recover_live_document<T>(
    live: &Path,
    backup: &Path,
    load: impl Fn(&Path) -> Result<T, CoreError>,
    is_unsupported_schema: impl Fn(&CoreError) -> bool,
) -> Result<T, CoreError> {
    match load(live) {
        Ok(document) => Ok(document),
        Err(error) if is_unsupported_schema(&error) => Err(error),
        Err(live_error) if !backup.exists() => Err(live_error),
        Err(live_error) => match load(backup) {
            Ok(document) => {
                preserve_corrupt_live_document(live)?;
                restore_backup_without_replacing(backup, live)?;
                Ok(document)
            }
            Err(error) if is_unsupported_schema(&error) => Err(error),
            Err(_) => Err(live_error),
        },
    }
}

pub(crate) fn restore_backup_without_replacing(
    backup: &Path,
    live: &Path,
) -> Result<(), CoreError> {
    let temporary = live.with_extension("json.recovery.tmp");
    remove_if_exists(&temporary)?;
    copy_file_synchronized(backup, &temporary)?;
    replace_without_backup(live, &temporary)
}

fn preserve_corrupt_live_document(live: &Path) -> Result<(), CoreError> {
    let parent = live.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "live document has no parent",
        )
    })?;
    let backups = parent.join("backups");
    let file_name = live.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "live document has no file name",
        )
    })?;
    let artifact = (0..CORRUPT_ARTIFACT_SLOTS)
        .map(|slot| backups.join(format!("{}.corrupt.{slot}", file_name.to_string_lossy())))
        .find(|candidate| !candidate.exists())
        .unwrap_or_else(|| backups.join(format!("{}.corrupt.0", file_name.to_string_lossy())));
    let temporary = PathBuf::from(format!("{}.tmp", artifact.to_string_lossy()));
    remove_if_exists(&temporary)?;
    copy_file_synchronized(live, &temporary)?;
    replace_without_backup(&artifact, &temporary)
}

fn copy_file_synchronized(source: &Path, destination: &Path) -> Result<(), CoreError> {
    let mut source_file = fs::File::open(source)?;
    let mut destination_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut buffer = [0_u8; 8_192];
    loop {
        let read = source_file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        destination_file.write_all(&buffer[..read])?;
    }
    destination_file.sync_all()?;
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<(), CoreError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(windows)]
fn replace_without_backup(destination: &Path, temporary: &Path) -> Result<(), CoreError> {
    if !destination.exists() {
        fs::rename(temporary, destination)?;
        return Ok(());
    }
    use std::{iter, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    let temporary_wide = temporary
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    let replaced = unsafe {
        ReplaceFileW(
            destination_wide.as_ptr(),
            temporary_wide.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            ptr::null(),
        )
    };
    if replaced == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_without_backup(destination: &Path, temporary: &Path) -> Result<(), CoreError> {
    fs::rename(temporary, destination)?;
    Ok(())
}

#[cfg(windows)]
pub(crate) fn replace_with_backup(
    destination: &Path,
    temporary: &Path,
    backup: &Path,
) -> Result<(), CoreError> {
    use std::{iter, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    if backup.exists() {
        fs::remove_file(backup)?;
    }

    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    let temporary_wide = temporary
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    let backup_wide = backup
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();

    // SAFETY: Each pointer references a live, NUL-terminated UTF-16 buffer for
    // the duration of the call. The two reserved pointer parameters are null.
    let replaced = unsafe {
        ReplaceFileW(
            destination_wide.as_ptr(),
            temporary_wide.as_ptr(),
            backup_wide.as_ptr(),
            0,
            ptr::null(),
            ptr::null(),
        )
    };
    if replaced == 0 {
        let error = std::io::Error::last_os_error();
        let _ = fs::remove_file(temporary);
        return Err(error.into());
    }

    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn replace_with_backup(
    destination: &Path,
    temporary: &Path,
    backup: &Path,
) -> Result<(), CoreError> {
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    fs::rename(destination, backup)?;
    if let Err(error) = fs::rename(temporary, destination) {
        let _ = fs::rename(backup, destination);
        return Err(error.into());
    }
    Ok(())
}
