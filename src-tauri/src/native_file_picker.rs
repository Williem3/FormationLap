//! Windows-native file selection for user-requested executable paths.

#[cfg(windows)]
pub(crate) fn pick_executable_path() -> std::io::Result<Option<String>> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
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
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: filter.as_ptr(),
        lpstrFile: selected_path.as_mut_ptr(),
        nMaxFile: selected_path.len() as u32,
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
pub(crate) fn pick_executable_path() -> std::io::Result<Option<String>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the native executable picker is available only on Windows",
    ))
}
