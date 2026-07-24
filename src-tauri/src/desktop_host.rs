use std::{io, path::Path};

const START_MINIMIZED_ARGUMENT: &str = "--minimized";

pub(crate) fn started_minimized() -> bool {
    std::env::args_os().any(|argument| argument == START_MINIMIZED_ARGUMENT)
}

fn startup_command(executable: &Path) -> String {
    format!(
        "\"{}\" {START_MINIMIZED_ARGUMENT}",
        executable.to_string_lossy()
    )
}

#[cfg(windows)]
pub(crate) fn set_start_with_windows(enabled: bool) -> io::Result<()> {
    use std::ptr;
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
            RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
        },
    };

    let subkey = wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value_name = wide_null("Formation Lap");
    let mut key: HKEY = ptr::null_mut();
    let result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            ptr::null(),
            &mut key,
            ptr::null_mut(),
        )
    };
    if result != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result as i32));
    }

    let operation_result = if enabled {
        let executable = std::env::current_exe()?.canonicalize()?;
        let command = wide_null(&startup_command(&executable));
        unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast::<u8>(),
                (command.len() * std::mem::size_of::<u16>()) as u32,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key, value_name.as_ptr()) }
    };
    unsafe {
        RegCloseKey(key);
    }

    if operation_result == ERROR_SUCCESS || (!enabled && operation_result == ERROR_FILE_NOT_FOUND) {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(operation_result as i32))
    }
}

#[cfg(not(windows))]
pub(crate) fn set_start_with_windows(_enabled: bool) -> io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub(crate) struct SingleInstanceGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl SingleInstanceGuard {
    pub(crate) fn acquire() -> io::Result<Option<Self>> {
        Self::acquire_named("Local\\FormationLap.SingleInstance.v1")
    }

    fn acquire_named(name: &str) -> io::Result<Option<Self>> {
        use std::ptr;
        use windows_sys::Win32::{
            Foundation::{ERROR_ALREADY_EXISTS, GetLastError},
            System::Threading::CreateMutexW,
        };

        let name = wide_null(name);
        let handle = unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(handle);
            }
            return Ok(None);
        }
        Ok(Some(Self { handle }))
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(not(windows))]
pub(crate) struct SingleInstanceGuard;

#[cfg(not(windows))]
impl SingleInstanceGuard {
    pub(crate) fn acquire() -> io::Result<Option<Self>> {
        Ok(Some(Self))
    }
}

#[cfg(test)]
mod tests {
    use super::{SingleInstanceGuard, startup_command};

    #[test]
    fn windows_startup_command_opens_only_formation_lap_minimized() {
        let command = startup_command(std::path::Path::new(
            r"C:\Program Files\Formation Lap\Formation Lap.exe",
        ));

        assert_eq!(
            command,
            r#""C:\Program Files\Formation Lap\Formation Lap.exe" --minimized"#
        );
        assert!(!command.contains("profile"));
        assert!(!command.contains("session"));
    }

    #[cfg(windows)]
    #[test]
    fn named_mutex_allows_only_one_live_instance() {
        let name = format!("Local\\FormationLap.Test.{}", uuid::Uuid::new_v4());
        let first = SingleInstanceGuard::acquire_named(&name)
            .expect("first mutex request should succeed")
            .expect("first instance should own the mutex");
        assert!(
            SingleInstanceGuard::acquire_named(&name)
                .expect("second mutex request should complete")
                .is_none(),
            "a duplicate instance must be rejected while the first guard lives"
        );
        drop(first);
        assert!(
            SingleInstanceGuard::acquire_named(&name)
                .expect("mutex should be reusable after shutdown")
                .is_some()
        );
    }
}
