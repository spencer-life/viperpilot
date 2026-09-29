#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let mut reversed_pair = false;
    let mut unavailable_library = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--reversed" => reversed_pair = true,
            "--unavailable" => unavailable_library = true,
            _ => {
                eprintln!("viperpilot-ui-preview: unknown argument {arg:?}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    match viper_v4_utility::tray::run_preview(reversed_pair, unavailable_library) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("viperpilot-ui-preview: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("viperpilot-ui-preview requires the x86_64-pc-windows-msvc build");
    std::process::ExitCode::FAILURE
}
