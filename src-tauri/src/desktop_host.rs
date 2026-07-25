use std::{io, path::Path};

const START_MINIMIZED_ARGUMENT: &str = "--minimized";
const STARTUP_RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const STARTUP_VALUE_NAME: &str = "com.formationlap.desktop.StartWithWindows.v1";
const LEGACY_STARTUP_VALUE_NAME: &str = "Formation Lap";

pub(crate) fn started_minimized() -> bool {
    std::env::args_os().any(|argument| argument == START_MINIMIZED_ARGUMENT)
}

fn startup_command(executable: &Path) -> String {
    format!(
        "\"{}\" {START_MINIMIZED_ARGUMENT}",
        executable.to_string_lossy()
    )
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct StartupRegistrationPlan {
    write_namespaced: bool,
    delete_namespaced: bool,
    delete_legacy: bool,
}

fn plan_startup_registration(
    enabled: bool,
    owned_commands: &[&str],
    namespaced_value: Option<&str>,
    legacy_value: Option<&str>,
) -> io::Result<StartupRegistrationPlan> {
    let is_owned = |value: &str| owned_commands.contains(&value);
    if namespaced_value.is_some_and(|value| !is_owned(value)) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the Formation Lap startup value is not owned by this executable",
        ));
    }

    Ok(StartupRegistrationPlan {
        write_namespaced: enabled && namespaced_value.is_none(),
        delete_namespaced: !enabled && namespaced_value.is_some_and(is_owned),
        delete_legacy: legacy_value.is_some_and(is_owned),
    })
}

#[cfg(windows)]
pub(crate) fn set_start_with_windows(enabled: bool) -> io::Result<()> {
    use std::{ffi::c_void, ptr};
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS},
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE,
            REG_SZ, RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW,
            RegSetValueExW,
        },
    };

    let executable = std::env::current_exe()?;
    let expected_command = startup_command(&executable);
    let legacy_canonical_command = executable
        .canonicalize()
        .ok()
        .map(|path| startup_command(&path));
    let mut owned_commands = vec![expected_command.as_str()];
    if let Some(command) = legacy_canonical_command.as_deref()
        && command != expected_command
    {
        owned_commands.push(command);
    }
    let subkey = wide_null(STARTUP_RUN_SUBKEY);
    let mut key: HKEY = ptr::null_mut();
    let result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            ptr::null(),
            &mut key,
            ptr::null_mut(),
        )
    };
    if result != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result as i32));
    }

    let operation_result = (|| {
        let namespaced_value =
            read_registry_string(STARTUP_VALUE_NAME, &mut |value_name, buffer, size| unsafe {
                RegGetValueW(
                    key,
                    ptr::null(),
                    value_name,
                    RRF_RT_REG_SZ,
                    ptr::null_mut(),
                    buffer,
                    size,
                )
            })?;
        let legacy_value = read_registry_string(
            LEGACY_STARTUP_VALUE_NAME,
            &mut |value_name, buffer, size| unsafe {
                RegGetValueW(
                    key,
                    ptr::null(),
                    value_name,
                    RRF_RT_REG_SZ,
                    ptr::null_mut(),
                    buffer,
                    size,
                )
            },
        )?;
        let plan = plan_startup_registration(
            enabled,
            &owned_commands,
            namespaced_value.as_deref(),
            legacy_value.as_deref(),
        )?;

        if plan.write_namespaced {
            let value_name = wide_null(STARTUP_VALUE_NAME);
            let command = wide_null(&expected_command);
            check_registry_result(unsafe {
                RegSetValueExW(
                    key,
                    value_name.as_ptr(),
                    0,
                    REG_SZ,
                    command.as_ptr().cast::<u8>(),
                    (command.len() * std::mem::size_of::<u16>()) as u32,
                )
            })?;
        }
        if plan.delete_namespaced {
            delete_registry_value(key, STARTUP_VALUE_NAME)?;
        }
        if plan.delete_legacy {
            delete_registry_value(key, LEGACY_STARTUP_VALUE_NAME)?;
        }
        Ok(())
    })();
    unsafe {
        RegCloseKey(key);
    }
    return operation_result;

    fn read_registry_string(
        value_name: &str,
        get_value: &mut impl FnMut(*const u16, *mut c_void, *mut u32) -> u32,
    ) -> io::Result<Option<String>> {
        let value_name = wide_null(value_name);
        for _ in 0..3 {
            let mut byte_count = 0;
            let result = get_value(value_name.as_ptr(), ptr::null_mut(), &mut byte_count);
            if result == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            check_registry_result(result)?;
            let mut value = vec![0_u16; (byte_count as usize).div_ceil(2)];
            let result = get_value(
                value_name.as_ptr(),
                value.as_mut_ptr().cast::<c_void>(),
                &mut byte_count,
            );
            if result == ERROR_MORE_DATA {
                continue;
            }
            check_registry_result(result)?;
            value.truncate((byte_count as usize).div_ceil(2));
            while value.last() == Some(&0) {
                value.pop();
            }
            return String::from_utf16(&value).map(Some).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "the startup registry value is not valid UTF-16",
                )
            });
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the startup registry value changed while it was being read",
        ))
    }

    fn delete_registry_value(key: HKEY, value_name: &str) -> io::Result<()> {
        let value_name = wide_null(value_name);
        let result = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
        if result == ERROR_SUCCESS || result == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(result as i32))
        }
    }

    fn check_registry_result(result: u32) -> io::Result<()> {
        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(result as i32))
        }
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
    use super::{
        SingleInstanceGuard, StartupRegistrationPlan, plan_startup_registration, startup_command,
    };

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

    #[test]
    fn startup_registration_migrates_only_the_exact_legacy_command() {
        let expected = r#""C:\Program Files\Formation Lap\formation-lap.exe" --minimized"#;

        assert_eq!(
            plan_startup_registration(true, &[expected], None, Some(expected))
                .expect("the exact legacy value should be migratable"),
            StartupRegistrationPlan {
                write_namespaced: true,
                delete_namespaced: false,
                delete_legacy: true,
            }
        );
        assert_eq!(
            plan_startup_registration(
                true,
                &[expected],
                None,
                Some(r#""C:\Other App\other.exe" --minimized"#),
            )
            .expect("an unrelated legacy value should be preserved"),
            StartupRegistrationPlan {
                write_namespaced: true,
                delete_namespaced: false,
                delete_legacy: false,
            }
        );
    }

    #[test]
    fn startup_registration_never_overwrites_or_deletes_a_foreign_namespaced_value() {
        let expected = r#""C:\Program Files\Formation Lap\formation-lap.exe" --minimized"#;
        let foreign = r#""C:\Other App\other.exe" --minimized"#;

        assert!(
            plan_startup_registration(true, &[expected], Some(foreign), None).is_err(),
            "enable must not overwrite a foreign namespaced value"
        );
        assert!(
            plan_startup_registration(false, &[expected], Some(foreign), None).is_err(),
            "disable must not delete a foreign namespaced value"
        );
    }

    #[test]
    fn disabling_startup_deletes_only_exact_owned_values() {
        let expected = r#""C:\Program Files\Formation Lap\formation-lap.exe" --minimized"#;
        let foreign = r#""C:\Other App\other.exe" --minimized"#;

        assert_eq!(
            plan_startup_registration(false, &[expected], Some(expected), Some(expected),)
                .expect("exact owned values should be removable"),
            StartupRegistrationPlan {
                write_namespaced: false,
                delete_namespaced: true,
                delete_legacy: true,
            }
        );
        assert_eq!(
            plan_startup_registration(false, &[expected], None, Some(foreign))
                .expect("a foreign legacy value should be left alone"),
            StartupRegistrationPlan::default()
        );
    }

    #[test]
    fn startup_registration_accepts_the_previous_canonical_path_form() {
        let expected = r#""C:\Program Files\Formation Lap\formation-lap.exe" --minimized"#;
        let previous = r#""\\?\C:\Program Files\Formation Lap\formation-lap.exe" --minimized"#;

        assert_eq!(
            plan_startup_registration(true, &[expected, previous], None, Some(previous))
                .expect("the exact previous canonical command should be migratable"),
            StartupRegistrationPlan {
                write_namespaced: true,
                delete_namespaced: false,
                delete_legacy: true,
            }
        );
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
