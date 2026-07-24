use crate::CoreError;
use std::{fs, path::Path};

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
