#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
#[cfg(windows)]
use windows::core::{HSTRING, w};

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let minimized = viper_v4_utility::gui_logic::wants_minimized(std::env::args().skip(1));
    match viper_v4_utility::tray::run(!minimized) {
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
