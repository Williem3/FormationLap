//! Windows-native file selection for user-requested executable paths.

use std::path::{Path, PathBuf};

fn initial_directory_from_executable_path(initial_path: Option<&str>) -> Option<PathBuf> {
    let path = Path::new(initial_path?);
    path.is_file()
        .then(|| path.parent().map(Path::to_path_buf))
        .flatten()
}

#[cfg(windows)]
pub(crate) fn pick_executable_path(
    initial_path: Option<String>,
) -> std::io::Result<Option<String>> {
    use std::{
        ffi::OsString,
        os::windows::ffi::{OsStrExt, OsStringExt},
    };
    use windows_sys::Win32::UI::Controls::Dialogs::{
        CommDlgExtendedError, GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY,
        OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

    const MAX_PATH_CODE_UNITS: usize = 32_768;
    let mut selected_path = vec![0_u16; MAX_PATH_CODE_UNITS];
    let filter = "Executable files\0*.exe;*.com;*.bat;*.cmd\0All files\0*.*\0\0"
        .encode_utf16()
        .collect::<Vec<_>>();
    let title = "Select an executable"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let initial_directory = initial_directory_from_executable_path(initial_path.as_deref());
    let initial_directory_wide = initial_directory.map(|directory| {
        directory
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    });
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: filter.as_ptr(),
        lpstrFile: selected_path.as_mut_ptr(),
        nMaxFile: selected_path.len() as u32,
        lpstrInitialDir: initial_directory_wide
            .as_ref()
            .map_or(std::ptr::null(), |path| path.as_ptr()),
        lpstrTitle: title.as_ptr(),
        Flags: OFN_EXPLORER
            | OFN_FILEMUSTEXIST
            | OFN_HIDEREADONLY
            | OFN_NOCHANGEDIR
            | OFN_PATHMUSTEXIST,
        ..Default::default()
    };

    // The buffers above remain alive until the native dialog returns.
    if unsafe { GetOpenFileNameW(&mut dialog) } != 0 {
        let length = selected_path
            .iter()
            .position(|code_unit| *code_unit == 0)
            .unwrap_or(selected_path.len());
        let selected_path = PathBuf::from(OsString::from_wide(&selected_path[..length]));
        let canonical_path = selected_path.canonicalize()?;
        return Ok(Some(canonical_path.to_string_lossy().into_owned()));
    }

    let error = unsafe { CommDlgExtendedError() };
    if error == 0 {
        Ok(None)
    } else {
        Err(std::io::Error::other(format!(
            "Windows could not open the executable picker (error {error})"
        )))
    }
}

#[cfg(not(windows))]
pub(crate) fn pick_executable_path(
    _initial_path: Option<String>,
) -> std::io::Result<Option<String>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the native executable picker is available only on Windows",
    ))
}

#[cfg(test)]
mod tests {
    use super::initial_directory_from_executable_path;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn uses_an_existing_executables_parent_as_the_picker_directory() {
        let executable_path = std::env::temp_dir().join(format!(
            "formation-lap-picker-test-{}-{}.exe",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("the system clock should be after the Unix epoch")
                .as_nanos(),
        ));
        fs::write(&executable_path, []).expect("the test executable should be created");

        assert_eq!(
            initial_directory_from_executable_path(executable_path.to_str()),
            executable_path.parent().map(|parent| parent.to_path_buf()),
        );

        fs::remove_file(executable_path).expect("the test executable should be removed");
    }

    #[test]
    fn ignores_a_missing_executable_path() {
        assert_eq!(
            initial_directory_from_executable_path(Some("C:\\missing\\application.exe")),
            None,
        );
    }
}
