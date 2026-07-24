use serde_json::json;
use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
    sync::{atomic::AtomicBool, atomic::Ordering, mpsc},
    thread,
    time::Duration,
};

static CONSOLE_STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
unsafe extern "system" fn console_control_handler(control: u32) -> i32 {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};

    if control == CTRL_C_EVENT || control == CTRL_BREAK_EVENT {
        CONSOLE_STOP_REQUESTED.store(true, Ordering::SeqCst);
        1
    } else {
        0
    }
}

#[cfg(windows)]
mod fixture_window {
    use std::{io, process, ptr, sync::mpsc::Sender, thread, time::Duration};
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG,
            PostQuitMessage, RegisterClassW, TranslateMessage, WM_DESTROY, WNDCLASSW,
            WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    };

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    unsafe extern "system" fn window_procedure(
        window: HWND,
        message: u32,
        word: WPARAM,
        long: LPARAM,
    ) -> LRESULT {
        if message == WM_DESTROY {
            unsafe {
                PostQuitMessage(0);
            }
            return 0;
        }
        unsafe { DefWindowProcW(window, message, word, long) }
    }

    pub fn run(state: String, ready: Sender<Result<(), String>>) {
        let class_name = wide("FormationLapProcessFixture");
        let window_name = wide("Formation Lap Process Fixture");
        let instance = unsafe { GetModuleHandleW(ptr::null()) };
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(window_procedure),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..WNDCLASSW::default()
        };
        if unsafe { RegisterClassW(&window_class) } == 0 {
            let _ = ready.send(Err(io::Error::last_os_error().to_string()));
            return;
        }
        let window = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_name.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                480,
                240,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null(),
            )
        };
        if window.is_null() {
            let _ = ready.send(Err(io::Error::last_os_error().to_string()));
            return;
        }
        let _ = ready.send(Ok(()));

        if state == "hung" {
            loop {
                thread::sleep(Duration::from_secs(60));
            }
        }

        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        process::exit(0);
    }
}

fn execute() -> Result<u8, Box<dyn std::error::Error>> {
    let mut arguments = env::args().skip(1);
    if arguments.next().as_deref() != Some("--report") {
        return Err("expected --report followed by a path".into());
    }
    let report_path = PathBuf::from(arguments.next().ok_or("missing report path")?);
    if arguments.next().as_deref() != Some("--lifetime-ms") {
        return Err("expected --lifetime-ms followed by a duration".into());
    }
    let lifetime = arguments.next().ok_or("missing lifetime")?.parse::<u64>()?;
    let mut received_arguments = Vec::new();
    let mut window_state = None;
    let mut stop_file = None;
    let mut console_control = false;
    let mut output_bytes = 0_usize;
    let mut startup_delay = Duration::ZERO;
    let mut exit_code = 0_u8;
    while let Some(argument) = arguments.next() {
        if argument == "--window-state" {
            window_state = Some(arguments.next().ok_or("missing window state")?);
        } else if argument == "--stop-file" {
            stop_file = Some(PathBuf::from(arguments.next().ok_or("missing stop file")?));
        } else if argument == "--console-control" {
            console_control = true;
        } else if argument == "--output-bytes" {
            output_bytes = arguments
                .next()
                .ok_or("missing output byte count")?
                .parse()?;
        } else if argument == "--startup-delay-ms" {
            startup_delay =
                Duration::from_millis(arguments.next().ok_or("missing startup delay")?.parse()?);
        } else if argument == "--exit-code" {
            exit_code = arguments.next().ok_or("missing exit code")?.parse()?;
        } else {
            received_arguments.push(argument);
        }
    }

    thread::sleep(startup_delay);

    if console_control {
        #[cfg(windows)]
        if unsafe {
            windows_sys::Win32::System::Console::SetConsoleCtrlHandler(
                Some(console_control_handler),
                1,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }

    if let Some(window_state) = window_state {
        #[cfg(windows)]
        {
            let (ready_sender, ready_receiver) = mpsc::channel();
            thread::spawn(move || fixture_window::run(window_state, ready_sender));
            ready_receiver
                .recv_timeout(Duration::from_secs(3))
                .map_err(|_| "fixture window did not become ready")?
                .map_err(|error| format!("fixture window failed: {error}"))?;
        }
        #[cfg(not(windows))]
        {
            let _ = window_state;
            return Err("window fixtures require Windows".into());
        }
    }

    if output_bytes > 0 {
        io::stdout().write_all(&vec![b'O'; output_bytes])?;
        io::stdout().write_all(b"\nSTDOUT-END\n")?;
        io::stdout().flush()?;
        io::stderr().write_all(&vec![b'E'; output_bytes])?;
        io::stderr().write_all(b"\nSTDERR-END\n")?;
        io::stderr().flush()?;
    }

    let report = json!({
        "arguments": received_arguments,
        "workingDirectory": env::current_dir()?,
    });
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;
    let deadline = std::time::Instant::now() + Duration::from_millis(lifetime);
    while std::time::Instant::now() < deadline {
        if stop_file.as_ref().is_some_and(|path| path.exists()) {
            break;
        }
        if console_control && CONSOLE_STOP_REQUESTED.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    Ok(exit_code)
}

fn main() -> ExitCode {
    match execute() {
        Ok(exit_code) => ExitCode::from(exit_code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
