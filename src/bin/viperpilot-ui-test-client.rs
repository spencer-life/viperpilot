use std::process::ExitCode;

#[cfg(windows)]
mod windows_client {
    use super::ExitCode;
    use serde::Serialize;
    use std::env;
    use windows::Win32::{
        Foundation::HWND,
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
        },
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern,
                UIA_ButtonControlTypeId, UIA_InvokePatternId,
            },
            WindowsAndMessaging::{
                GetClassNameW, GetDlgCtrlID, GetDlgItem, GetParent, GetWindowThreadProcessId,
                IsWindow, IsWindowVisible,
            },
        },
    };

    const PREVIEW_CLASS: &str = "ViperPilotDevelopmentPreviewWindowV1";
    const ALLOWED_CONTROLS: [i32; 11] = [
        2001, 2002, 2003, 2004, 2005, 2101, 2102, 2103, 2104, 2105, 2106,
    ];

    struct Apartment;

    impl Apartment {
        fn initialize() -> windows::core::Result<Self> {
            // S_FALSE also means this thread entered COM and therefore needs a matching uninitialize.
            unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
            Ok(Self)
        }
    }

    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct Inspection {
        name: String,
        enabled: bool,
        control_type_id: i32,
        invoke_available: bool,
        visible: bool,
    }

    fn parse_number<T: std::str::FromStr>(name: &str, value: Option<String>) -> Result<T, String> {
        value
            .ok_or_else(|| format!("missing {name}"))?
            .parse()
            .map_err(|_| format!("invalid {name}"))
    }

    fn parse_arguments() -> Result<(String, isize, u32, i32), String> {
        let mut args = env::args().skip(1);
        let action = args.next().ok_or_else(usage)?;
        if action != "inspect" && action != "invoke" {
            return Err(usage());
        }
        let mut parent = None;
        let mut expected_pid = None;
        let mut control = None;
        while let Some(flag) = args.next() {
            let value = args.next().ok_or_else(usage)?;
            match flag.as_str() {
                "--parent-hwnd" if parent.is_none() => parent = Some(value),
                "--expected-pid" if expected_pid.is_none() => expected_pid = Some(value),
                "--control-id" if control.is_none() => control = Some(value),
                _ => return Err(usage()),
            }
        }
        let parent: isize = parse_number("parent HWND", parent)?;
        if parent == 0 {
            return Err("parent HWND must be nonzero".to_owned());
        }
        let pid = parse_number("expected PID", expected_pid)?;
        if pid == 0 {
            return Err("expected PID must be nonzero".to_owned());
        }
        let control = parse_number("control ID", control)?;
        if !ALLOWED_CONTROLS.contains(&control) {
            return Err(format!(
                "control ID {control} is outside the preview allowlist"
            ));
        }
        Ok((action, parent, pid, control))
    }

    fn usage() -> String {
        "usage: viperpilot-ui-test-client <inspect|invoke> --parent-hwnd <decimal> --expected-pid <pid> --control-id <id>".to_owned()
    }

    fn hwnd_from(value: isize) -> HWND {
        HWND(value as *mut _)
    }

    fn checked_child(
        parent_value: isize,
        expected_pid: u32,
        control_id: i32,
    ) -> Result<HWND, String> {
        let parent = hwnd_from(parent_value);
        if !unsafe { IsWindow(Some(parent)) }.as_bool() {
            return Err("parent HWND is not a window".to_owned());
        }
        let mut parent_pid = 0;
        unsafe { GetWindowThreadProcessId(parent, Some(&mut parent_pid)) };
        if parent_pid != expected_pid {
            return Err("parent HWND process ID does not match expected PID".to_owned());
        }
        let mut class_buffer = [0u16; 256];
        let class_length = unsafe { GetClassNameW(parent, &mut class_buffer) };
        if class_length <= 0
            || String::from_utf16_lossy(&class_buffer[..class_length as usize]) != PREVIEW_CLASS
        {
            return Err("parent HWND is not the development preview window class".to_owned());
        }

        let child = unsafe { GetDlgItem(Some(parent), control_id) }
            .map_err(|_| format!("control ID {control_id} is not a child window"))?;
        if child.0.is_null() || !unsafe { IsWindow(Some(child)) }.as_bool() {
            return Err(format!("control ID {control_id} is not a child window"));
        }
        let actual_parent =
            unsafe { GetParent(child) }.map_err(|_| "child parent could not be read")?;
        if actual_parent != parent || unsafe { GetDlgCtrlID(child) } != control_id {
            return Err(
                "control HWND does not belong to the expected preview parent and ID".to_owned(),
            );
        }
        let mut child_pid = 0;
        unsafe { GetWindowThreadProcessId(child, Some(&mut child_pid)) };
        if child_pid != expected_pid {
            return Err("child HWND process ID does not match expected PID".to_owned());
        }
        Ok(child)
    }

    fn inspect(element: &IUIAutomationElement, child: HWND) -> windows::core::Result<Inspection> {
        let name = unsafe { element.CurrentName()? }.to_string();
        let enabled = unsafe { element.CurrentIsEnabled()? }.as_bool();
        let control_type_id = unsafe { element.CurrentControlType()? }.0;
        let invoke_available = unsafe { element.GetCurrentPattern(UIA_InvokePatternId) }.is_ok();
        Ok(Inspection {
            name,
            enabled,
            control_type_id,
            invoke_available,
            visible: unsafe { IsWindowVisible(child) }.as_bool(),
        })
    }

    fn run() -> Result<(), String> {
        let (action, parent, expected_pid, control_id) = parse_arguments()?;
        let child = checked_child(parent, expected_pid, control_id)?;
        let _apartment = Apartment::initialize().map_err(|error| error.to_string())?;
        let automation: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
                .map_err(|error| error.to_string())?;
        let element =
            unsafe { automation.ElementFromHandle(child) }.map_err(|error| error.to_string())?;
        let state = inspect(&element, child).map_err(|error| error.to_string())?;

        if action == "invoke" {
            // 2026-09-30: dashboard/inspector controls stay alive but hidden;
            // direct HWND access must not invoke a control outside its view.
            if !state.visible {
                return Err(format!("refusing to invoke hidden control ID {control_id}"));
            }
            if state.control_type_id != UIA_ButtonControlTypeId.0 {
                return Err(format!(
                    "control ID {control_id} is not exposed as a UIA Button"
                ));
            }
            if !state.enabled {
                return Err(format!(
                    "refusing to invoke disabled control ID {control_id}"
                ));
            }
            if !state.invoke_available {
                return Err(format!(
                    "control ID {control_id} does not expose InvokePattern"
                ));
            }
            let pattern: IUIAutomationInvokePattern =
                unsafe { element.GetCurrentPatternAs(UIA_InvokePatternId) }
                    .map_err(|error| error.to_string())?;
            unsafe { pattern.Invoke() }.map_err(|error| error.to_string())?;
        }

        println!(
            "{}",
            serde_json::to_string(&state).map_err(|error| error.to_string())?
        );
        Ok(())
    }

    pub fn main_entry() -> ExitCode {
        match run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        }
    }
}

fn main() -> ExitCode {
    #[cfg(windows)]
    {
        windows_client::main_entry()
    }
    #[cfg(not(windows))]
    {
        eprintln!("viperpilot-ui-test-client requires a Windows build target");
        ExitCode::FAILURE
    }
}
