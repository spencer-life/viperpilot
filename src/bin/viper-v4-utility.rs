#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
#[cfg(windows)]
use windows::core::{HSTRING, w};

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = if args.iter().any(|arg| arg == "--pilot") {
        viper_v4_utility::pilot::PilotOptions::parse(args).and_then(|options| {
            let show = !options.minimized;
            options
                .load()
                .and_then(|session| viper_v4_utility::tray::run_pilot(show, session))
        })
    } else if args.iter().any(|arg| arg != "--minimized") {
        Err("Unknown launch arguments. Pilot launch requires --pilot --root DIRECTORY --confirm-device FULL_SERIAL.".to_owned())
    } else {
        viper_v4_utility::tray::run(!args.iter().any(|arg| arg == "--minimized"))
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("viper-v4-utility: {error}");
            let message = HSTRING::from(error);
            let _ = unsafe {
                MessageBoxW(
                    None,
                    &message,
                    w!("Viper V4 Pro Utility"),
                    MB_OK | MB_ICONERROR,
                )
            };
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("viper-v4-utility requires the x86_64-pc-windows-msvc build");
    std::process::ExitCode::FAILURE
}
