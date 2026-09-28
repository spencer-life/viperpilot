//! The intentionally small native Windows surface for the fixed Viper profiles:
//! one tray icon plus one compact status window in the same process.
//!
//! Device I/O is confined to one worker thread. The UI thread is a normal Win32
//! `GetMessageW` loop, so it does not poll while idle and never observes mouse
//! input. The window renders state decided by the platform-independent
//! `gui_logic` module and performs no HID work of its own.

use core::ffi::c_void;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{
    COLORREF, CloseHandle, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, GetLastError, HANDLE, HWND,
    LPARAM, LRESULT, POINT, RECT, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    BITMAP, BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateCompatibleDC, CreateFontW,
    CreatePen, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER, DT_SINGLELINE,
    DT_VCENTER, DeleteDC, DeleteObject, DrawFocusRect, DrawTextW, Ellipse, EndPaint, FONT_WEIGHT,
    FW_BOLD, FW_NORMAL, FillRect, GetObjectW, HALFTONE, HBITMAP, HBRUSH, HDC, HFONT, HGDIOBJ,
    InvalidateRect, LineTo, MoveToEx, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, RoundRect,
    SRCCOPY, SelectObject, SetBkMode, SetStretchBltMode, SetTextColor, StretchBlt, TRANSPARENT,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, REG_SZ, RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegSetValueExW,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_DISABLED, ODS_FOCUS, ODS_SELECTED};
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
    SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, SetFocus, UnregisterHotKey,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CREATESTRUCTW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DestroyWindow, DispatchMessageW, FindWindowW, GWLP_USERDATA, GetClientRect,
    GetCursorPos, GetMessageW, GetWindowLongPtrW, HICON, ICON_BIG, ICON_SMALL, IDC_ARROW,
    IDI_APPLICATION, IMAGE_BITMAP, IMAGE_ICON, IsDialogMessageW, IsIconic, LR_LOADFROMFILE,
    LoadCursorW, LoadIconW, LoadImageW, MF_CHECKED, MF_GRAYED, MF_STRING, MSG, MoveWindow,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SW_HIDE, SW_RESTORE,
    SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SendMessageW, SetForegroundWindow,
    SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, TPM_RIGHTBUTTON, TrackPopupMenu,
    TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU,
    WM_CTLCOLORSTATIC, WM_DESTROY, WM_DEVICECHANGE, WM_DPICHANGED, WM_DRAWITEM, WM_HOTKEY,
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NCCREATE, WM_PAINT, WM_RBUTTONUP, WM_SETFONT, WM_SETICON,
    WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
    WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{BOOL, PCWSTR, w};

use crate::engine::{ProfileMatch, apply_write_plan, classify_profile, next_profile_for_hotkey};
use crate::gui_logic::{
    self, BusyKind, ConnectionStatus, LaunchDecision, StatusPresentation, Tone, VerificationOutcome,
};
use crate::model::{
    DpiPair, ProfileName, ProfileSpec, RawButtonAssignment, UtilityConfigV1, WirelessPowerSettings,
};
use crate::planning::{plan_profile_with_baseline, require_proven_polling_writes};
use crate::storage::{
    StoragePaths, append_diagnostic, load_config, read_snapshot, save_config, write_plan_journal,
    write_verification_report,
};
use crate::windows::{RazerDevice, refuse_if_synapse_running};

const WINDOW_CLASS: PCWSTR = w!("ViperV4UtilityTrayWindowV1");
const WINDOW_TITLE: PCWSTR = w!("Viper V4 Pro Utility");
const MUTEX_NAME: PCWSTR = w!("Local\\ViperV4UtilityTrayV1");
const SHOW_MESSAGE_NAME: PCWSTR = w!("ViperV4UtilityShowWindowV1");
const STARTUP_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const STARTUP_VALUE: PCWSTR = w!("ViperV4Utility");
const ICON_FILE_NAME: &str = "viper-utility-icon.ico";
const DASHBOARD_MOUSE_BMP: &str = "viper-v4-black-dashboard.bmp";
const TRAY_ICON_ID: u32 = 1;
const TRAY_CALLBACK: u32 = WM_APP + 1;
const WORKER_EVENT: u32 = WM_APP + 2;
const HOTKEY_ID: i32 = 0x5654;

const MENU_OPEN: usize = 1000;
const MENU_DEVELOPER: usize = 1001;
const MENU_GAMING: usize = 1002;
const MENU_STARTUP: usize = 1003;
const MENU_DIAGNOSTICS: usize = 1004;
const MENU_EXIT_AFTER_GAMING: usize = 1005;
const MENU_EXIT: usize = 1006;
const BUTTON_DEVELOPER: usize = 2001;
const BUTTON_GAMING: usize = 2002;
const HOTSPOT_LEFT: usize = 2101;
const HOTSPOT_RIGHT: usize = 2102;
const HOTSPOT_MIDDLE: usize = 2103;
const HOTSPOT_REAR_SIDE: usize = 2104;
const HOTSPOT_FRONT_SIDE: usize = 2105;
const HOTSPOT_DPI: usize = 2106;

// Button and static styles are numeric window styles. The named constants live
// in the very large Win32_System_SystemServices feature group, so the few raw
// values needed here are pinned instead of enabling that group.
const BS_OWNERDRAW_STYLE: u32 = 0x0000_000B;
const SS_NOPREFIX_STYLE: u32 = 0x0000_0080;
const SS_EDITCONTROL_STYLE: u32 = 0x0000_2000;

const MAIN_WINDOW_STYLE: WINDOW_STYLE = WINDOW_STYLE(
    WS_OVERLAPPED.0 | WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_CLIPCHILDREN.0,
);

// Fixed layout in 96-DPI units, scaled by the window's live DPI.
const BASE_MARGIN: i32 = 24;
const BASE_CLIENT_WIDTH: i32 = 880;
const BASE_CLIENT_HEIGHT: i32 = 680;
const BASE_HEADER_TITLE_TOP: i32 = 17;
const BASE_HEADER_SUBTITLE_TOP: i32 = 45;
const BASE_MAIN_TOP: i32 = 98;
const BASE_LEFT_PANEL_LEFT: i32 = BASE_MARGIN;
const BASE_LEFT_PANEL_WIDTH: i32 = 316;
const BASE_COLUMN_GAP: i32 = 20;
const BASE_RIGHT_PANEL_LEFT: i32 = BASE_LEFT_PANEL_LEFT + BASE_LEFT_PANEL_WIDTH + BASE_COLUMN_GAP;
const BASE_RIGHT_PANEL_WIDTH: i32 = BASE_CLIENT_WIDTH - BASE_RIGHT_PANEL_LEFT - BASE_MARGIN;
const BASE_STATUS_CARD_HEIGHT: i32 = 86;
const BASE_STATUS_CARD_GAP: i32 = 12;
const BASE_STATUS_CARD_WIDTH: i32 = (BASE_RIGHT_PANEL_WIDTH - 3 * BASE_STATUS_CARD_GAP) / 4;
const BASE_ASSIGNMENT_PANEL_TOP: i32 = BASE_MAIN_TOP + BASE_STATUS_CARD_HEIGHT + 12;
const BASE_MAIN_PANEL_BOTTOM: i32 = 498;
const BASE_MAIN_PANEL_HEIGHT: i32 = BASE_MAIN_PANEL_BOTTOM - BASE_MAIN_TOP;
const BASE_ASSIGNMENT_PANEL_HEIGHT: i32 = BASE_MAIN_PANEL_BOTTOM - BASE_ASSIGNMENT_PANEL_TOP;
const BASE_MOUSE_IMAGE_LEFT: i32 = BASE_LEFT_PANEL_LEFT + 63;
const BASE_MOUSE_IMAGE_TOP: i32 = BASE_MAIN_TOP + 40;
const BASE_MOUSE_IMAGE_WIDTH: i32 = 190;
const BASE_MOUSE_IMAGE_HEIGHT: i32 = 240;
const BASE_VERIFICATION_TOP: i32 = 514;
const BASE_VERIFICATION_HEIGHT: i32 = 74;
const BASE_BUTTON_TOP: i32 = 604;
const BASE_BUTTON_HEIGHT: i32 = 52;

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    COLORREF((blue as u32) << 16 | (green as u32) << 8 | red as u32)
}

// Catppuccin Mocha.
const CAT_BASE: COLORREF = rgb(30, 30, 46);
const CAT_CRUST: COLORREF = rgb(17, 17, 27);
const CAT_SURFACE0: COLORREF = rgb(49, 50, 68);
const CAT_SURFACE1: COLORREF = rgb(69, 71, 90);
const CAT_OVERLAY0: COLORREF = rgb(108, 112, 134);
const CAT_TEXT: COLORREF = rgb(205, 214, 244);
const CAT_SUBTEXT0: COLORREF = rgb(166, 173, 200);
const CAT_BLUE: COLORREF = rgb(137, 180, 250);
const CAT_GREEN: COLORREF = rgb(166, 227, 161);
const CAT_YELLOW: COLORREF = rgb(249, 226, 175);
const CAT_RED: COLORREF = rgb(243, 139, 168);
const CAT_MAUVE: COLORREF = rgb(203, 166, 247);
const CAT_PEACH: COLORREF = rgb(250, 179, 135);
const MOUSE_BLACK: COLORREF = rgb(11, 11, 15);

#[derive(Clone, Copy, Debug)]
enum WorkerCommand {
    Refresh,
    Apply(ProfileName),
    SetStartWithWindows(bool),
    SetDiagnostics(bool),
    SetExitAfterGaming(bool),
}

#[derive(Clone, Debug)]
enum WorkerEvent {
    State {
        profile: ProfileMatch,
        polling_hz: Option<u16>,
        dpi: Option<DpiPair>,
        power: Option<WirelessPowerSettings>,
        button_assignments: Vec<RawButtonAssignment>,
        config: UtilityConfigV1,
    },
    Applied {
        profile: ProfileName,
        config: UtilityConfigV1,
        no_changes_needed: bool,
        report_path: Option<String>,
        dpi: Option<DpiPair>,
        power: Option<WirelessPowerSettings>,
        button_assignments: Option<Vec<RawButtonAssignment>>,
    },
    ApplyFailed {
        profile: ProfileName,
        detail: String,
        report_path: Option<String>,
        final_profile: Option<ProfileMatch>,
        polling_hz: Option<u16>,
        dpi: Option<DpiPair>,
        power: Option<WirelessPowerSettings>,
        button_assignments: Option<Vec<RawButtonAssignment>>,
    },
    Config(UtilityConfigV1),
    Error(String),
}

struct StatusWindow {
    brand_title: HWND,
    brand_subtitle: HWND,
    labels: [HWND; 8],
    profile_value: HWND,
    dpi_value: HWND,
    polling_value: HWND,
    battery_value: HWND,
    sleep_value: HWND,
    low_power_value: HWND,
    connection_value: HWND,
    assignment_heading: HWND,
    assignment_name: HWND,
    assignment_semantic: HWND,
    assignment_raw: HWND,
    assignment_hint: HWND,
    mouse_hotspots: [HWND; 6],
    verification_value: HWND,
    developer_button: HWND,
    gaming_button: HWND,
    brand_font: HFONT,
    title_font: HFONT,
    body_font: HFONT,
    button_font: HFONT,
}

/// A packaged, opaque BMP used only for the native dashboard's product visual.
/// If it is absent, the window falls back to a local vector placeholder and
/// stays fully usable without changing any device state.
struct DashboardBitmap {
    handle: HBITMAP,
    width: i32,
    height: i32,
}

struct TrayApp {
    command_tx: Sender<WorkerCommand>,
    event_rx: Receiver<WorkerEvent>,
    current: ProfileMatch,
    config: UtilityConfigV1,
    busy: Option<BusyKind>,
    // A complete read-verified profile may toggle; an out-of-sync state may not.
    hotkey_armed: bool,
    connection: ConnectionStatus,
    polling_hz: Option<u16>,
    dpi: Option<DpiPair>,
    power: Option<WirelessPowerSettings>,
    button_assignments: Option<Vec<RawButtonAssignment>>,
    selected_mouse_control: gui_logic::MouseControl,
    last_verification: Option<VerificationOutcome>,
    last_error: Option<String>,
    presentation: Option<StatusPresentation>,
    show_message: u32,
    status: Option<StatusWindow>,
    base_brush: HBRUSH,
    surface_brush: HBRUSH,
    mouse_bitmap: Option<DashboardBitmap>,
}

impl Drop for StatusWindow {
    fn drop(&mut self) {
        for font in [
            self.brand_font,
            self.title_font,
            self.body_font,
            self.button_font,
        ] {
            if !font.0.is_null() {
                let _ = unsafe { DeleteObject(HGDIOBJ(font.0)) };
            }
        }
    }
}

impl Drop for TrayApp {
    fn drop(&mut self) {
        for brush in [self.base_brush, self.surface_brush] {
            if !brush.0.is_null() {
                let _ = unsafe { DeleteObject(HGDIOBJ(brush.0)) };
            }
        }
        if let Some(bitmap) = self.mouse_bitmap.as_ref() {
            let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.handle.0)) };
        }
    }
}

/// Starts the single-instance, native tray-and-window process. If another
/// instance already holds the mutex, this signals its window to show itself
/// and exits successfully instead of starting a second instance.
pub fn run(show_window_at_start: bool) -> Result<(), String> {
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let show_message = unsafe { RegisterWindowMessageW(SHOW_MESSAGE_NAME) };
    let instance_mutex = match create_single_instance_mutex()? {
        InstanceGate::Primary(handle) => handle,
        InstanceGate::AlreadyRunning => return signal_existing_instance(show_message),
    };
    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    let app = Box::into_raw(Box::new(TrayApp {
        command_tx,
        event_rx,
        current: ProfileMatch::OutOfSync,
        config: UtilityConfigV1::default(),
        busy: Some(BusyKind::Reading),
        hotkey_armed: false,
        connection: ConnectionStatus::Connected,
        polling_hz: None,
        dpi: None,
        power: None,
        button_assignments: None,
        selected_mouse_control: gui_logic::MouseControl::RearSide,
        last_verification: None,
        last_error: None,
        presentation: None,
        show_message,
        status: None,
        base_brush: unsafe { CreateSolidBrush(CAT_BASE) },
        surface_brush: unsafe { CreateSolidBrush(CAT_SURFACE0) },
        mouse_bitmap: load_dashboard_mouse_bitmap(),
    }));

    let window = match create_main_window(unsafe { &mut *app }) {
        Ok(window) => window,
        Err(error) => {
            drop(unsafe { Box::from_raw(app) });
            return Err(error);
        }
    };
    let controls = match create_status_controls(window) {
        Ok(controls) => controls,
        Err(error) => {
            let _ = unsafe { DestroyWindow(window) };
            drop(unsafe { Box::from_raw(app) });
            return Err(error);
        }
    };
    unsafe { &mut *app }.status = Some(controls);
    if let Some(status) = unsafe { &mut *app }.status.as_mut() {
        layout_status(window, status);
    }
    unsafe { &mut *app }.update_ui(window);

    let worker_window = window.0 as usize;
    let worker = thread::Builder::new()
        .name("viper-device-worker".to_owned())
        .spawn(move || worker_loop(command_rx, event_tx, worker_window))
        .map_err(|error| format!("could not start serialized device worker: {error}"));
    if let Err(error) = worker {
        let _ = unsafe { DestroyWindow(window) };
        drop(unsafe { Box::from_raw(app) });
        return Err(error);
    }

    if let Err(error) = add_tray_icon(window) {
        let _ = unsafe { DestroyWindow(window) };
        drop(unsafe { Box::from_raw(app) });
        return Err(error);
    }
    let hotkey = unsafe {
        RegisterHotKey(
            Some(window),
            HOTKEY_ID,
            MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
            u32::from(b'P'),
        )
    }
    .map_err(|error| format!("could not register Ctrl+Alt+P: {error}"));
    if let Err(error) = hotkey {
        remove_tray_icon(window);
        let _ = unsafe { DestroyWindow(window) };
        drop(unsafe { Box::from_raw(app) });
        return Err(error);
    }
    let startup_request = unsafe { &*app }
        .command_tx
        .send(WorkerCommand::Refresh)
        .map_err(|error| format!("could not request startup state: {error}"));
    if let Err(error) = startup_request {
        let _ = unsafe { UnregisterHotKey(Some(window), HOTKEY_ID) };
        remove_tray_icon(window);
        let _ = unsafe { DestroyWindow(window) };
        drop(unsafe { Box::from_raw(app) });
        return Err(error);
    }
    if show_window_at_start {
        unsafe { &*app }.show_window_now(window);
    }

    let result = message_loop(window);
    let _ = unsafe { UnregisterHotKey(Some(window), HOTKEY_ID) };
    remove_tray_icon(window);
    let _ = unsafe { DestroyWindow(window) };
    // `app` is owned by the window through GWLP_USERDATA until message-loop exit.
    drop(unsafe { Box::from_raw(app) });
    drop(instance_mutex);
    result
}

enum InstanceGate {
    Primary(OwnedHandle),
    AlreadyRunning,
}

fn create_single_instance_mutex() -> Result<InstanceGate, String> {
    let handle = unsafe { CreateMutexW(None, false, MUTEX_NAME) }
        .map_err(|error| format!("could not create single-instance mutex: {error}"))?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) }
            .map_err(|error| format!("could not close duplicate-instance mutex: {error}"))?;
        return Ok(InstanceGate::AlreadyRunning);
    }
    Ok(InstanceGate::Primary(OwnedHandle(handle)))
}

/// A second launch never starts a second tray: it asks the existing window to
/// show itself. The short retry covers the primary instance still starting up.
fn signal_existing_instance(show_message: u32) -> Result<(), String> {
    for _ in 0..20 {
        let existing = unsafe { FindWindowW(WINDOW_CLASS, PCWSTR::null()) }.ok();
        match gui_logic::launch_decision(true, existing.is_some(), false) {
            LaunchDecision::ShowExisting => {
                let window = existing.expect("decision requires a found window");
                if show_message != 0 {
                    let _ =
                        unsafe { PostMessageW(Some(window), show_message, WPARAM(0), LPARAM(0)) };
                }
                let _ = unsafe { SetForegroundWindow(window) };
                return Ok(());
            }
            LaunchDecision::FailAlreadyRunning => {
                thread::sleep(Duration::from_millis(100));
            }
            LaunchDecision::StartTray { .. } => unreachable!("mutex already exists"),
        }
    }
    Err(
        "the Viper V4 Pro utility is already running, but its window did not answer; \
         use the tray icon's Open item"
            .to_owned(),
    )
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn create_main_window(app: &mut TrayApp) -> Result<HWND, String> {
    let large_icon = load_app_icon(32, 32)?;
    let small_icon = load_app_icon(16, 16)?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        lpszClassName: WINDOW_CLASS,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        hIcon: large_icon,
        hbrBackground: app.base_brush,
        ..Default::default()
    };
    let atom = unsafe { RegisterClassW(&class) };
    if atom == 0 {
        return Err(format!(
            "could not register tray window class: {}",
            unsafe { GetLastError().0 }
        ));
    }
    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            WINDOW_CLASS,
            WINDOW_TITLE,
            MAIN_WINDOW_STYLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            BASE_CLIENT_WIDTH,
            BASE_CLIENT_HEIGHT,
            None,
            None,
            None,
            Some((app as *mut TrayApp).cast()),
        )
    }
    .map_err(|error| format!("could not create the utility window: {error}"))?;
    unsafe {
        SendMessageW(
            window,
            WM_SETICON,
            Some(WPARAM(ICON_BIG as usize)),
            Some(LPARAM(large_icon.0 as isize)),
        );
        SendMessageW(
            window,
            WM_SETICON,
            Some(WPARAM(ICON_SMALL as usize)),
            Some(LPARAM(small_icon.0 as isize)),
        );
    }
    let dark_mode = BOOL(1);
    let _ = unsafe {
        DwmSetWindowAttribute(
            window,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark_mode).cast(),
            u32::try_from(size_of::<BOOL>()).unwrap_or(4),
        )
    };
    Ok(window)
}

fn create_status_controls(window: HWND) -> Result<StatusWindow, String> {
    let static_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | SS_NOPREFIX_STYLE);
    let wrap_style =
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | SS_NOPREFIX_STYLE | SS_EDITCONTROL_STYLE);
    let brand_title = create_child(window, w!("STATIC"), "Viper V4 Pro", static_style, 0)?;
    let brand_subtitle = create_child(
        window,
        w!("STATIC"),
        "Verified onboard profiles  ·  Ctrl + Alt + P switches",
        static_style,
        0,
    )?;
    let labels = [
        create_child(window, w!("STATIC"), "PROFILE", static_style, 0)?,
        create_child(window, w!("STATIC"), "DPI", static_style, 0)?,
        create_child(window, w!("STATIC"), "POLLING", static_style, 0)?,
        create_child(window, w!("STATIC"), "BATT", static_style, 0)?,
        create_child(window, w!("STATIC"), "SLEEP", static_style, 0)?,
        create_child(window, w!("STATIC"), "LOW", static_style, 0)?,
        create_child(window, w!("STATIC"), "CONNECTION", static_style, 0)?,
        create_child(window, w!("STATIC"), "VERIFICATION", static_style, 0)?,
    ];
    let profile_value = create_child(window, w!("STATIC"), "Reading\u{2026}", static_style, 0)?;
    let dpi_value = create_child(window, w!("STATIC"), "Unavailable", static_style, 0)?;
    let polling_value = create_child(window, w!("STATIC"), "Unknown", static_style, 0)?;
    let battery_value = create_child(window, w!("STATIC"), "Unavailable", static_style, 0)?;
    let sleep_value = create_child(window, w!("STATIC"), "Unavailable", static_style, 0)?;
    let low_power_value = create_child(window, w!("STATIC"), "Unavailable", static_style, 0)?;
    let connection_value = create_child(window, w!("STATIC"), "Connected", static_style, 0)?;
    let assignment_heading = create_child(
        window,
        w!("STATIC"),
        "SELECTED ONBOARD ASSIGNMENT  ·  READ-ONLY",
        static_style,
        0,
    )?;
    let assignment_name = create_child(
        window,
        w!("STATIC"),
        "Rear side  ·  Mouse 4  ·  slot 0x04",
        static_style,
        0,
    )?;
    let assignment_semantic = create_child(
        window,
        w!("STATIC"),
        "Not read from the device.",
        wrap_style,
        0,
    )?;
    let assignment_raw = create_child(
        window,
        w!("STATIC"),
        "Raw assignment unavailable.",
        wrap_style,
        0,
    )?;
    let assignment_hint = create_child(
        window,
        w!("STATIC"),
        "Choose a numbered hotspot to inspect its last GET snapshot. This panel never changes the mouse.",
        wrap_style,
        0,
    )?;
    let hotspot_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_OWNERDRAW_STYLE);
    let mouse_hotspots = [
        create_child(window, w!("BUTTON"), "1", hotspot_style, HOTSPOT_LEFT)?,
        create_child(window, w!("BUTTON"), "2", hotspot_style, HOTSPOT_RIGHT)?,
        create_child(window, w!("BUTTON"), "3", hotspot_style, HOTSPOT_MIDDLE)?,
        create_child(window, w!("BUTTON"), "4", hotspot_style, HOTSPOT_REAR_SIDE)?,
        create_child(window, w!("BUTTON"), "5", hotspot_style, HOTSPOT_FRONT_SIDE)?,
        create_child(window, w!("BUTTON"), "6", hotspot_style, HOTSPOT_DPI)?,
    ];
    let verification_value = create_child(
        window,
        w!("STATIC"),
        "No profile change has run in this session.",
        wrap_style,
        0,
    )?;
    let developer_button = create_child(
        window,
        w!("BUTTON"),
        "Developer",
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_OWNERDRAW_STYLE),
        BUTTON_DEVELOPER,
    )?;
    let gaming_button = create_child(
        window,
        w!("BUTTON"),
        "Gaming",
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_OWNERDRAW_STYLE),
        BUTTON_GAMING,
    )?;
    Ok(StatusWindow {
        brand_title,
        brand_subtitle,
        labels,
        profile_value,
        dpi_value,
        polling_value,
        battery_value,
        sleep_value,
        low_power_value,
        connection_value,
        assignment_heading,
        assignment_name,
        assignment_semantic,
        assignment_raw,
        assignment_hint,
        mouse_hotspots,
        verification_value,
        developer_button,
        gaming_button,
        brand_font: HFONT::default(),
        title_font: HFONT::default(),
        body_font: HFONT::default(),
        button_font: HFONT::default(),
    })
}

fn create_child(
    window: HWND,
    class: PCWSTR,
    text: &str,
    style: WINDOW_STYLE,
    control_id: usize,
) -> Result<HWND, String> {
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let menu = if control_id == 0 {
        None
    } else {
        Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
            control_id as *mut c_void,
        ))
    };
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            PCWSTR(wide.as_ptr()),
            style,
            0,
            0,
            0,
            0,
            Some(window),
            menu,
            None,
            None,
        )
    }
    .map_err(|error| format!("could not create a window control: {error}"))
}

fn scale(value: i32, dpi: u32) -> i32 {
    value * i32::try_from(dpi).unwrap_or(96) / 96
}

fn status_card_bounds(index: i32, dpi: u32) -> RECT {
    let left = BASE_RIGHT_PANEL_LEFT + index * (BASE_STATUS_CARD_WIDTH + BASE_STATUS_CARD_GAP);
    RECT {
        left: scale(left, dpi),
        top: scale(BASE_MAIN_TOP, dpi),
        right: scale(left + BASE_STATUS_CARD_WIDTH, dpi),
        bottom: scale(BASE_MAIN_TOP + BASE_STATUS_CARD_HEIGHT, dpi),
    }
}

fn mouse_image_bounds(dpi: u32) -> RECT {
    RECT {
        left: scale(BASE_MOUSE_IMAGE_LEFT, dpi),
        top: scale(BASE_MOUSE_IMAGE_TOP, dpi),
        right: scale(BASE_MOUSE_IMAGE_LEFT + BASE_MOUSE_IMAGE_WIDTH, dpi),
        bottom: scale(BASE_MOUSE_IMAGE_TOP + BASE_MOUSE_IMAGE_HEIGHT, dpi),
    }
}

fn mouse_hotspot_bounds(control: gui_logic::MouseControl) -> (i32, i32, i32, i32) {
    match control {
        gui_logic::MouseControl::Left => (
            BASE_MOUSE_IMAGE_LEFT + 38,
            BASE_MOUSE_IMAGE_TOP + 37,
            34,
            34,
        ),
        gui_logic::MouseControl::Right => (
            BASE_MOUSE_IMAGE_LEFT + 100,
            BASE_MOUSE_IMAGE_TOP + 37,
            34,
            34,
        ),
        gui_logic::MouseControl::Middle => (
            BASE_MOUSE_IMAGE_LEFT + 78,
            BASE_MOUSE_IMAGE_TOP + 66,
            34,
            34,
        ),
        gui_logic::MouseControl::RearSide => (
            BASE_MOUSE_IMAGE_LEFT + 32,
            BASE_MOUSE_IMAGE_TOP + 90,
            34,
            34,
        ),
        gui_logic::MouseControl::FrontSide => (
            BASE_MOUSE_IMAGE_LEFT + 32,
            BASE_MOUSE_IMAGE_TOP + 122,
            34,
            34,
        ),
        gui_logic::MouseControl::Dpi => (
            BASE_MOUSE_IMAGE_LEFT + 78,
            BASE_MOUSE_IMAGE_TOP + 97,
            34,
            34,
        ),
    }
}

fn mouse_control_for_hotspot(command: usize) -> Option<gui_logic::MouseControl> {
    match command {
        HOTSPOT_LEFT => Some(gui_logic::MouseControl::Left),
        HOTSPOT_RIGHT => Some(gui_logic::MouseControl::Right),
        HOTSPOT_MIDDLE => Some(gui_logic::MouseControl::Middle),
        HOTSPOT_REAR_SIDE => Some(gui_logic::MouseControl::RearSide),
        HOTSPOT_FRONT_SIDE => Some(gui_logic::MouseControl::FrontSide),
        HOTSPOT_DPI => Some(gui_logic::MouseControl::Dpi),
        _ => None,
    }
}

fn create_font(dpi: u32, point_size: i32, weight: FONT_WEIGHT) -> HFONT {
    let height = -(point_size * i32::try_from(dpi).unwrap_or(96) / 72);
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            i32::try_from(weight.0).unwrap_or(400),
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            u32::from(DEFAULT_PITCH.0),
            w!("Segoe UI"),
        )
    }
}

fn set_font(control: HWND, font: HFONT) {
    let _ = unsafe {
        SendMessageW(
            control,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        )
    };
}

/// Positions every control and sizes the outer window for the window's current
/// DPI. Called at creation and again on `WM_DPICHANGED`.
fn layout_status(window: HWND, status: &mut StatusWindow) {
    let dpi = unsafe { GetDpiForWindow(window) };
    for old in [
        status.brand_font,
        status.title_font,
        status.body_font,
        status.button_font,
    ] {
        if !old.0.is_null() {
            let _ = unsafe { DeleteObject(HGDIOBJ(old.0)) };
        }
    }
    status.brand_font = create_font(dpi, 18, FW_BOLD);
    status.title_font = create_font(dpi, 12, FW_BOLD);
    status.body_font = create_font(dpi, 9, FW_NORMAL);
    status.button_font = create_font(dpi, 10, FW_BOLD);
    set_font(status.brand_title, status.brand_font);
    set_font(status.brand_subtitle, status.body_font);
    set_font(status.profile_value, status.title_font);
    set_font(status.dpi_value, status.title_font);
    set_font(status.polling_value, status.title_font);
    set_font(status.connection_value, status.title_font);
    for control in [
        status.battery_value,
        status.sleep_value,
        status.low_power_value,
        status.verification_value,
    ] {
        set_font(control, status.body_font);
    }
    for label in status.labels {
        set_font(label, status.body_font);
    }
    set_font(status.assignment_heading, status.body_font);
    set_font(status.assignment_name, status.title_font);
    set_font(status.assignment_semantic, status.body_font);
    set_font(status.assignment_raw, status.body_font);
    set_font(status.assignment_hint, status.body_font);
    for hotspot in status.mouse_hotspots {
        set_font(hotspot, status.button_font);
    }
    set_font(status.developer_button, status.button_font);
    set_font(status.gaming_button, status.button_font);

    let margin = scale(BASE_MARGIN, dpi);
    let client_width = scale(BASE_CLIENT_WIDTH, dpi);
    let move_child = |child: HWND, x: i32, y: i32, width: i32, height: i32| {
        let _ = unsafe { MoveWindow(child, x, y, width, height, true) };
    };
    move_child(
        status.brand_title,
        margin,
        scale(BASE_HEADER_TITLE_TOP, dpi),
        client_width - 2 * margin,
        scale(27, dpi),
    );
    move_child(
        status.brand_subtitle,
        margin,
        scale(BASE_HEADER_SUBTITLE_TOP, dpi),
        client_width - 2 * margin,
        scale(16, dpi),
    );

    let [profile_card, performance_card, power_card, link_card] = [
        status_card_bounds(0, dpi),
        status_card_bounds(1, dpi),
        status_card_bounds(2, dpi),
        status_card_bounds(3, dpi),
    ];
    let card_inset = scale(12, dpi);
    let label_height = scale(15, dpi);
    let narrow_label_width = scale(42, dpi);
    let narrow_value_left = scale(55, dpi);
    let card_width = |card: RECT| card.right - card.left;

    move_child(
        status.labels[0],
        profile_card.left + card_inset,
        profile_card.top + scale(11, dpi),
        card_width(profile_card) - 2 * card_inset,
        label_height,
    );
    move_child(
        status.profile_value,
        profile_card.left + card_inset,
        profile_card.top + scale(35, dpi),
        card_width(profile_card) - 2 * card_inset,
        scale(26, dpi),
    );
    move_child(
        status.labels[1],
        performance_card.left + card_inset,
        performance_card.top + scale(8, dpi),
        card_width(performance_card) - 2 * card_inset,
        label_height,
    );
    move_child(
        status.dpi_value,
        performance_card.left + card_inset,
        performance_card.top + scale(23, dpi),
        card_width(performance_card) - 2 * card_inset,
        scale(21, dpi),
    );
    move_child(
        status.labels[2],
        performance_card.left + card_inset,
        performance_card.top + scale(49, dpi),
        card_width(performance_card) - 2 * card_inset,
        label_height,
    );
    move_child(
        status.polling_value,
        performance_card.left + card_inset,
        performance_card.top + scale(63, dpi),
        card_width(performance_card) - 2 * card_inset,
        scale(20, dpi),
    );

    move_child(
        status.labels[3],
        power_card.left + card_inset,
        power_card.top + scale(10, dpi),
        narrow_label_width,
        label_height,
    );
    move_child(
        status.battery_value,
        power_card.left + narrow_value_left,
        power_card.top + scale(10, dpi),
        card_width(power_card) - narrow_value_left - card_inset,
        label_height,
    );
    move_child(
        status.labels[4],
        power_card.left + card_inset,
        power_card.top + scale(34, dpi),
        narrow_label_width,
        label_height,
    );
    move_child(
        status.sleep_value,
        power_card.left + narrow_value_left,
        power_card.top + scale(34, dpi),
        card_width(power_card) - narrow_value_left - card_inset,
        label_height,
    );
    move_child(
        status.labels[5],
        power_card.left + card_inset,
        power_card.top + scale(58, dpi),
        narrow_label_width,
        label_height,
    );
    move_child(
        status.low_power_value,
        power_card.left + narrow_value_left,
        power_card.top + scale(58, dpi),
        card_width(power_card) - narrow_value_left - card_inset,
        label_height,
    );
    move_child(
        status.labels[6],
        link_card.left + card_inset,
        link_card.top + scale(11, dpi),
        card_width(link_card) - 2 * card_inset,
        label_height,
    );
    move_child(
        status.connection_value,
        link_card.left + card_inset,
        link_card.top + scale(35, dpi),
        card_width(link_card) - 2 * card_inset,
        scale(26, dpi),
    );

    let panel_left = scale(BASE_RIGHT_PANEL_LEFT, dpi);
    let panel_top = scale(BASE_ASSIGNMENT_PANEL_TOP, dpi);
    let panel_width = scale(BASE_RIGHT_PANEL_WIDTH, dpi);
    move_child(
        status.assignment_heading,
        panel_left + scale(20, dpi),
        panel_top + scale(18, dpi),
        panel_width - scale(40, dpi),
        label_height,
    );
    move_child(
        status.assignment_name,
        panel_left + scale(20, dpi),
        panel_top + scale(44, dpi),
        panel_width - scale(40, dpi),
        scale(27, dpi),
    );
    move_child(
        status.assignment_semantic,
        panel_left + scale(20, dpi),
        panel_top + scale(82, dpi),
        panel_width - scale(40, dpi),
        scale(32, dpi),
    );
    move_child(
        status.assignment_raw,
        panel_left + scale(20, dpi),
        panel_top + scale(126, dpi),
        panel_width - scale(40, dpi),
        scale(48, dpi),
    );
    move_child(
        status.assignment_hint,
        panel_left + scale(20, dpi),
        panel_top + scale(226, dpi),
        panel_width - scale(40, dpi),
        scale(44, dpi),
    );
    for (hotspot, control) in status
        .mouse_hotspots
        .into_iter()
        .zip(gui_logic::MouseControl::ALL)
    {
        let (left, top, width, height) = mouse_hotspot_bounds(control);
        move_child(
            hotspot,
            scale(left, dpi),
            scale(top, dpi),
            scale(width, dpi),
            scale(height, dpi),
        );
    }
    move_child(
        status.labels[7],
        margin + scale(16, dpi),
        scale(BASE_VERIFICATION_TOP + 10, dpi),
        client_width - 2 * margin - scale(32, dpi),
        label_height,
    );
    move_child(
        status.verification_value,
        margin + scale(16, dpi),
        scale(BASE_VERIFICATION_TOP + 29, dpi),
        client_width - 2 * margin - scale(32, dpi),
        scale(36, dpi),
    );
    let button_width = (client_width - 2 * margin - scale(12, dpi)) / 2;
    let button_top = scale(BASE_BUTTON_TOP, dpi);
    let button_height = scale(BASE_BUTTON_HEIGHT, dpi);
    move_child(
        status.developer_button,
        margin,
        button_top,
        button_width,
        button_height,
    );
    move_child(
        status.gaming_button,
        margin + button_width + scale(12, dpi),
        button_top,
        button_width,
        button_height,
    );

    let mut outer = RECT {
        left: 0,
        top: 0,
        right: client_width,
        bottom: scale(BASE_CLIENT_HEIGHT, dpi),
    };
    let _ = unsafe {
        AdjustWindowRectExForDpi(
            &mut outer,
            MAIN_WINDOW_STYLE,
            false,
            WINDOW_EX_STYLE(0),
            dpi,
        )
    };
    let _ = unsafe {
        SetWindowPos(
            window,
            None,
            0,
            0,
            outer.right - outer.left,
            outer.bottom - outer.top,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
}

fn message_loop(window: HWND) -> Result<(), String> {
    let mut message = MSG::default();
    loop {
        let status = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
        if status == -1 {
            return Err(format!("GetMessageW failed: {}", unsafe {
                GetLastError().0
            }));
        }
        if status == 0 {
            return Ok(());
        }
        // Tab, Shift+Tab, Space, and Enter drive the two profile buttons.
        if unsafe { IsDialogMessageW(window, &message) }.as_bool() {
            continue;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            let _ = DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        unsafe {
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
        }
    }
    let app_ptr = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) as *mut TrayApp };
    if app_ptr.is_null() {
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    }
    let app = unsafe { &mut *app_ptr };
    if app.show_message != 0 && message == app.show_message {
        app.show_window_now(window);
        return LRESULT(0);
    }
    match message {
        WORKER_EVENT => {
            app.drain_worker_events(window);
            LRESULT(0)
        }
        WM_HOTKEY if wparam.0 as i32 == HOTKEY_ID => {
            app.handle_hotkey(window);
            LRESULT(0)
        }
        WM_DEVICECHANGE if app.busy.is_none() => {
            app.request_refresh(window);
            LRESULT(0)
        }
        WM_COMMAND => {
            app.handle_menu_command(window, wparam.0 & 0xffff);
            LRESULT(0)
        }
        TRAY_CALLBACK => {
            match u32::from(crate::tray_logic::notification_event_code(lparam.0)) {
                WM_CONTEXTMENU | WM_RBUTTONUP => app.show_menu(window),
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => app.show_window_now(window),
                _ => {}
            }
            LRESULT(0)
        }
        WM_PAINT => {
            app.paint_dashboard(window);
            LRESULT(0)
        }
        WM_DRAWITEM => {
            let item = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
            if app.draw_owner_button(item) {
                LRESULT(1)
            } else {
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
        }
        WM_CTLCOLORSTATIC => {
            let control = HWND(lparam.0 as *mut c_void);
            let (color, brush) = app.static_text_style(control);
            let hdc = HDC(wparam.0 as *mut c_void);
            let _ = unsafe { SetTextColor(hdc, color) };
            let _ = unsafe { SetBkMode(hdc, TRANSPARENT) };
            LRESULT(brush.0 as isize)
        }
        WM_DPICHANGED => {
            let suggested = unsafe { &*(lparam.0 as *const RECT) };
            let _ = unsafe {
                SetWindowPos(
                    window,
                    None,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            if let Some(status) = app.status.as_mut() {
                layout_status(window, status);
            }
            LRESULT(0)
        }
        // Closing the window hides it to the tray; the tray Exit item remains
        // the deliberate way to terminate the utility.
        WM_CLOSE => {
            let _ = unsafe { ShowWindow(window, SW_HIDE) };
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

impl TrayApp {
    fn gui_state(&self) -> gui_logic::GuiState {
        gui_logic::GuiState {
            busy: self.busy,
            profile: self.current,
            polling_hz: self.polling_hz,
            dpi: self.dpi,
            power: self.power,
            connection: self.connection,
            last_verification: self.last_verification.clone(),
            last_error: self.last_error.clone(),
        }
    }

    /// Renders the platform-independent presentation into the Win32 controls,
    /// the button enablement, and the tray tooltip.
    fn update_ui(&mut self, window: HWND) {
        let presentation = gui_logic::present(&self.gui_state());
        let assignment = gui_logic::assignment_presentation(
            self.selected_mouse_control,
            self.button_assignments.as_deref(),
        );
        set_tooltip(
            window,
            &format!("Viper V4 Pro: {}", presentation.profile_line),
        );
        if let Some(status) = &self.status {
            set_text(status.profile_value, &presentation.profile_line);
            set_text(status.dpi_value, &presentation.dpi_line);
            set_text(status.polling_value, &presentation.polling_line);
            set_text(status.battery_value, &presentation.battery_line);
            set_text(status.sleep_value, &presentation.sleep_line);
            set_text(status.low_power_value, &presentation.low_power_line);
            set_text(status.connection_value, &presentation.connection_line);
            set_text(status.assignment_name, &assignment.title);
            set_text(status.assignment_semantic, &assignment.semantic);
            set_text(status.assignment_raw, &assignment.raw);
            set_text(status.verification_value, &presentation.verification_text);
            let _ = unsafe { EnableWindow(status.developer_button, presentation.buttons_enabled) };
            let _ = unsafe { EnableWindow(status.gaming_button, presentation.buttons_enabled) };
            let _ = unsafe { InvalidateRect(Some(status.developer_button), None, true) };
            let _ = unsafe { InvalidateRect(Some(status.gaming_button), None, true) };
            for hotspot in status.mouse_hotspots {
                let _ = unsafe { InvalidateRect(Some(hotspot), None, true) };
            }
            let _ = unsafe { InvalidateRect(Some(window), None, true) };
        }
        self.presentation = Some(presentation);
    }

    /// Draws the non-control dashboard chrome. The colored button controls are
    /// ordinary owner-draw Win32 buttons layered over a packaged product image;
    /// clicking one only updates the selected display card.
    fn paint_dashboard(&self, window: HWND) {
        let mut paint = PAINTSTRUCT::default();
        let hdc = unsafe { BeginPaint(window, &mut paint) };
        let mut client = RECT::default();
        let _ = unsafe { GetClientRect(window, &mut client) };
        let _ = unsafe { FillRect(hdc, &client, self.base_brush) };

        let dpi = unsafe { GetDpiForWindow(window) };
        let margin = scale(BASE_MARGIN, dpi);
        let left_panel = RECT {
            left: scale(BASE_LEFT_PANEL_LEFT, dpi),
            top: scale(BASE_MAIN_TOP, dpi),
            right: scale(BASE_LEFT_PANEL_LEFT + BASE_LEFT_PANEL_WIDTH, dpi),
            bottom: scale(BASE_MAIN_TOP + BASE_MAIN_PANEL_HEIGHT, dpi),
        };
        draw_panel(hdc, left_panel, CAT_SURFACE0, CAT_SURFACE1, scale(12, dpi));
        for index in 0..4 {
            draw_panel(
                hdc,
                status_card_bounds(index, dpi),
                CAT_SURFACE0,
                CAT_SURFACE1,
                scale(12, dpi),
            );
        }
        let assignment_panel = RECT {
            left: scale(BASE_RIGHT_PANEL_LEFT, dpi),
            top: scale(BASE_ASSIGNMENT_PANEL_TOP, dpi),
            right: scale(BASE_RIGHT_PANEL_LEFT + BASE_RIGHT_PANEL_WIDTH, dpi),
            bottom: scale(
                BASE_ASSIGNMENT_PANEL_TOP + BASE_ASSIGNMENT_PANEL_HEIGHT,
                dpi,
            ),
        };
        draw_panel(
            hdc,
            assignment_panel,
            CAT_SURFACE0,
            CAT_SURFACE1,
            scale(14, dpi),
        );
        let verification_panel = RECT {
            left: margin,
            top: scale(BASE_VERIFICATION_TOP, dpi),
            right: client.right - margin,
            bottom: scale(BASE_VERIFICATION_TOP + BASE_VERIFICATION_HEIGHT, dpi),
        };
        draw_panel(
            hdc,
            verification_panel,
            CAT_SURFACE0,
            CAT_SURFACE1,
            scale(12, dpi),
        );
        self.draw_mouse_visual(hdc, dpi);
        let _ = unsafe { EndPaint(window, &paint) };
    }

    fn draw_mouse_visual(&self, hdc: HDC, dpi: u32) {
        if let Some(status) = self.status.as_ref() {
            let panel_left = scale(BASE_LEFT_PANEL_LEFT, dpi);
            let panel_width = scale(BASE_LEFT_PANEL_WIDTH, dpi);
            let caption = RECT {
                left: panel_left + scale(12, dpi),
                top: scale(BASE_MAIN_TOP + 12, dpi),
                right: panel_left + panel_width - scale(12, dpi),
                bottom: scale(BASE_MAIN_TOP + 28, dpi),
            };
            draw_centered_text(
                hdc,
                caption,
                "BLACK VIPER V4 PRO  ·  BUTTON MAP",
                status.body_font,
                CAT_SUBTEXT0,
            );
            let first_legend = RECT {
                left: panel_left + scale(12, dpi),
                top: scale(BASE_MAIN_TOP + 292, dpi),
                right: panel_left + panel_width - scale(12, dpi),
                bottom: scale(BASE_MAIN_TOP + 308, dpi),
            };
            draw_centered_text(
                hdc,
                first_legend,
                "1 Left  ·  2 Right  ·  3 Middle",
                status.body_font,
                CAT_SUBTEXT0,
            );
            let second_legend = RECT {
                left: panel_left + scale(12, dpi),
                top: scale(BASE_MAIN_TOP + 314, dpi),
                right: panel_left + panel_width - scale(12, dpi),
                bottom: scale(BASE_MAIN_TOP + 330, dpi),
            };
            draw_centered_text(
                hdc,
                second_legend,
                "4 Rear side  ·  5 Front side  ·  6 DPI",
                status.body_font,
                CAT_SUBTEXT0,
            );
        }

        let image = mouse_image_bounds(dpi);
        if !self.draw_dashboard_bitmap(hdc, image) {
            draw_mouse_placeholder(hdc, image, dpi);
        }
    }

    fn draw_dashboard_bitmap(&self, hdc: HDC, destination: RECT) -> bool {
        let Some(bitmap) = self.mouse_bitmap.as_ref() else {
            return false;
        };
        let source = unsafe { CreateCompatibleDC(Some(hdc)) };
        if source.0.is_null() {
            return false;
        }
        let old_bitmap = unsafe { SelectObject(source, HGDIOBJ(bitmap.handle.0)) };
        let _ = unsafe { SetStretchBltMode(hdc, HALFTONE) };
        let copied = unsafe {
            StretchBlt(
                hdc,
                destination.left,
                destination.top,
                destination.right - destination.left,
                destination.bottom - destination.top,
                Some(source),
                0,
                0,
                bitmap.width,
                bitmap.height,
                SRCCOPY,
            )
        }
        .as_bool();
        let _ = unsafe { SelectObject(source, old_bitmap) };
        let _ = unsafe { DeleteDC(source) };
        copied
    }

    fn static_text_style(&self, control: HWND) -> (COLORREF, HBRUSH) {
        let Some(status) = self.status.as_ref() else {
            return (CAT_TEXT, self.base_brush);
        };
        let color = if control == status.brand_title {
            CAT_TEXT
        } else if control == status.brand_subtitle
            || status.labels.contains(&control)
            || control == status.assignment_heading
        {
            CAT_SUBTEXT0
        } else if control == status.assignment_name {
            hotspot_color(self.selected_mouse_control)
        } else if control == status.assignment_raw || control == status.assignment_hint {
            CAT_SUBTEXT0
        } else if let Some(presentation) = self.presentation.as_ref() {
            if control == status.profile_value {
                tone_color(presentation.profile_tone)
            } else if control == status.connection_value {
                tone_color(presentation.connection_tone)
            } else if control == status.verification_value {
                tone_color(presentation.verification_tone)
            } else {
                CAT_TEXT
            }
        } else {
            CAT_TEXT
        };
        let status_card = [
            status.labels[0],
            status.labels[1],
            status.labels[2],
            status.labels[3],
            status.labels[4],
            status.labels[5],
            status.labels[6],
            status.profile_value,
            status.dpi_value,
            status.polling_value,
            status.battery_value,
            status.sleep_value,
            status.low_power_value,
            status.connection_value,
        ]
        .contains(&control);
        let assignment_card = [
            status.assignment_heading,
            status.assignment_name,
            status.assignment_semantic,
            status.assignment_raw,
            status.assignment_hint,
        ]
        .contains(&control);
        let verification_card = [status.labels[7], status.verification_value].contains(&control);
        let brush = if status_card || assignment_card || verification_card {
            self.surface_brush
        } else {
            self.base_brush
        };
        (color, brush)
    }

    fn draw_owner_button(&self, item: &DRAWITEMSTRUCT) -> bool {
        let (label, accent, active, corner_radius, is_hotspot) = match item.CtlID as usize {
            BUTTON_DEVELOPER => (
                "Developer  ·  1000 Hz",
                CAT_BLUE,
                self.current == ProfileMatch::Developer,
                12,
                false,
            ),
            BUTTON_GAMING => (
                "Gaming  ·  4000 Hz",
                CAT_GREEN,
                self.current == ProfileMatch::Gaming,
                12,
                false,
            ),
            hotspot => {
                let Some(control) = mouse_control_for_hotspot(hotspot) else {
                    return false;
                };
                (
                    control.hotspot_label(),
                    hotspot_color(control),
                    self.selected_mouse_control == control,
                    8,
                    true,
                )
            }
        };
        let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
        let selected = item.itemState.0 & ODS_SELECTED.0 != 0;
        let focused = item.itemState.0 & ODS_FOCUS.0 != 0;
        let (fill, border, text) = if disabled {
            (CAT_SURFACE0, CAT_SURFACE1, CAT_OVERLAY0)
        } else if selected {
            (CAT_SURFACE1, accent, CAT_TEXT)
        } else if active && is_hotspot {
            (CAT_CRUST, accent, accent)
        } else if active {
            (accent, accent, CAT_CRUST)
        } else {
            (CAT_SURFACE0, accent, accent)
        };
        let dpi = unsafe { GetDpiForWindow(item.hwndItem) };
        if is_hotspot {
            // Owner-draw child buttons otherwise retain the class brush in the
            // corners outside the circle. Match the image's Mocha backdrop so
            // the hotspot reads as a circle instead of a white square.
            let _ = unsafe { FillRect(item.hDC, &item.rcItem, self.base_brush) };
        }
        let brush = unsafe { CreateSolidBrush(fill) };
        let pen_width = if active && is_hotspot { 3 } else { 1 };
        let pen = unsafe { CreatePen(PS_SOLID, scale(pen_width, dpi), border) };
        let old_brush = unsafe { SelectObject(item.hDC, HGDIOBJ(brush.0)) };
        let old_pen = unsafe { SelectObject(item.hDC, HGDIOBJ(pen.0)) };
        if is_hotspot {
            let _ = unsafe {
                Ellipse(
                    item.hDC,
                    item.rcItem.left,
                    item.rcItem.top,
                    item.rcItem.right,
                    item.rcItem.bottom,
                )
            };
        } else {
            let _ = unsafe {
                RoundRect(
                    item.hDC,
                    item.rcItem.left,
                    item.rcItem.top,
                    item.rcItem.right,
                    item.rcItem.bottom,
                    scale(corner_radius, dpi),
                    scale(corner_radius, dpi),
                )
            };
        }
        let old_font = self
            .status
            .as_ref()
            .map(|status| unsafe { SelectObject(item.hDC, HGDIOBJ(status.button_font.0)) });
        let _ = unsafe { SetTextColor(item.hDC, text) };
        let _ = unsafe { SetBkMode(item.hDC, TRANSPARENT) };
        let mut wide: Vec<u16> = label.encode_utf16().collect();
        let mut rect = item.rcItem;
        let _ = unsafe {
            DrawTextW(
                item.hDC,
                &mut wide,
                &mut rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            )
        };
        if focused {
            let mut focus = item.rcItem;
            focus.left += 4;
            focus.top += 4;
            focus.right -= 4;
            focus.bottom -= 4;
            let _ = unsafe { DrawFocusRect(item.hDC, &focus) };
        }
        if let Some(old_font) = old_font {
            let _ = unsafe { SelectObject(item.hDC, old_font) };
        }
        let _ = unsafe { SelectObject(item.hDC, old_pen) };
        let _ = unsafe { SelectObject(item.hDC, old_brush) };
        let _ = unsafe { DeleteObject(HGDIOBJ(pen.0)) };
        let _ = unsafe { DeleteObject(HGDIOBJ(brush.0)) };
        true
    }

    fn show_window_now(&self, window: HWND) {
        let _ = unsafe { ShowWindow(window, SW_SHOW) };
        if unsafe { IsIconic(window) }.as_bool() {
            let _ = unsafe { ShowWindow(window, SW_RESTORE) };
        }
        let _ = unsafe { SetForegroundWindow(window) };
        if let Some(status) = &self.status {
            let _ = unsafe { SetFocus(Some(status.developer_button)) };
        }
    }

    fn drain_worker_events(&mut self, window: HWND) {
        loop {
            match self.event_rx.try_recv() {
                Ok(WorkerEvent::State {
                    profile,
                    polling_hz,
                    dpi,
                    power,
                    button_assignments,
                    config,
                }) => {
                    self.current = profile;
                    self.config = config;
                    self.busy = None;
                    self.connection = ConnectionStatus::Connected;
                    self.polling_hz = polling_hz;
                    self.dpi = dpi;
                    self.power = power;
                    self.button_assignments = Some(button_assignments);
                    self.last_error = None;
                    self.hotkey_armed = profile != ProfileMatch::OutOfSync;
                    self.update_ui(window);
                }
                Ok(WorkerEvent::Applied {
                    profile,
                    config,
                    no_changes_needed,
                    report_path,
                    dpi,
                    power,
                    button_assignments,
                }) => {
                    self.current = match profile {
                        ProfileName::Developer => ProfileMatch::Developer,
                        ProfileName::Gaming => ProfileMatch::Gaming,
                    };
                    self.config = config;
                    self.busy = None;
                    self.connection = ConnectionStatus::Connected;
                    self.polling_hz = Some(ProfileSpec::fixed(profile).polling_hz);
                    self.dpi = dpi;
                    self.power = power;
                    self.button_assignments = button_assignments;
                    self.hotkey_armed = true;
                    self.last_error = None;
                    let detail = gui_logic::success_detail(no_changes_needed);
                    self.last_verification = Some(VerificationOutcome {
                        profile,
                        success: true,
                        detail: detail.to_owned(),
                        time: local_clock(),
                        report_path,
                    });
                    self.update_ui(window);
                    notify(
                        window,
                        "Profile verified",
                        &format!("{profile}: {detail}"),
                        false,
                    );
                    if profile == ProfileName::Gaming && self.config.exit_after_gaming {
                        let _ = unsafe { DestroyWindow(window) };
                    }
                }
                Ok(WorkerEvent::ApplyFailed {
                    profile,
                    detail,
                    report_path,
                    final_profile,
                    polling_hz,
                    dpi,
                    power,
                    button_assignments,
                }) => {
                    self.busy = None;
                    self.current = final_profile.unwrap_or(ProfileMatch::OutOfSync);
                    self.hotkey_armed =
                        matches!(self.current, ProfileMatch::Developer | ProfileMatch::Gaming);
                    self.polling_hz = polling_hz;
                    self.dpi = dpi;
                    self.power = power;
                    self.button_assignments = button_assignments;
                    self.last_error = None;
                    self.last_verification = Some(VerificationOutcome {
                        profile,
                        success: false,
                        detail: detail.clone(),
                        time: local_clock(),
                        report_path,
                    });
                    self.update_ui(window);
                    notify(window, "Profile not verified", &detail, true);
                }
                Ok(WorkerEvent::Config(config)) => {
                    self.config = config;
                    self.busy = None;
                    self.update_ui(window);
                }
                Ok(WorkerEvent::Error(error)) => {
                    self.busy = None;
                    self.current = ProfileMatch::OutOfSync;
                    self.hotkey_armed = false;
                    self.connection = gui_logic::classify_worker_error(&error);
                    self.polling_hz = None;
                    self.dpi = None;
                    self.power = None;
                    self.button_assignments = None;
                    self.last_error = Some(error.clone());
                    self.update_ui(window);
                    notify(window, "Viper utility", &error, true);
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    fn handle_hotkey(&mut self, window: HWND) {
        if self.busy.is_some() || !self.hotkey_armed || self.current == ProfileMatch::OutOfSync {
            return;
        }
        if let Some(profile) = next_profile_for_hotkey(self.current) {
            self.request_apply(window, profile);
        }
    }

    fn handle_menu_command(&mut self, window: HWND, command: usize) {
        if let Some(control) = mouse_control_for_hotspot(command) {
            // The map is a read-only inspector. It deliberately does not use
            // the worker, create a plan, or send any HID command.
            self.selected_mouse_control = control;
            self.update_ui(window);
            return;
        }
        match command {
            MENU_OPEN => self.show_window_now(window),
            MENU_DEVELOPER | BUTTON_DEVELOPER => {
                self.request_apply(window, ProfileName::Developer);
            }
            MENU_GAMING | BUTTON_GAMING => self.request_apply(window, ProfileName::Gaming),
            MENU_STARTUP if self.busy.is_none() => self.request_config(
                window,
                WorkerCommand::SetStartWithWindows(!self.config.start_with_windows),
            ),
            MENU_DIAGNOSTICS if self.busy.is_none() => self.request_config(
                window,
                WorkerCommand::SetDiagnostics(!self.config.diagnostics_enabled),
            ),
            MENU_EXIT_AFTER_GAMING if self.busy.is_none() => self.request_config(
                window,
                WorkerCommand::SetExitAfterGaming(!self.config.exit_after_gaming),
            ),
            MENU_EXIT if self.busy.is_none() => {
                let _ = unsafe { DestroyWindow(window) };
            }
            _ => {}
        }
    }

    fn request_apply(&mut self, window: HWND, profile: ProfileName) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(BusyKind::Changing);
        if self.command_tx.send(WorkerCommand::Apply(profile)).is_err() {
            self.busy = None;
        }
        self.update_ui(window);
    }

    fn request_refresh(&mut self, window: HWND) {
        self.busy = Some(BusyKind::Reading);
        if self.command_tx.send(WorkerCommand::Refresh).is_err() {
            self.busy = None;
        }
        self.update_ui(window);
    }

    fn request_config(&mut self, window: HWND, command: WorkerCommand) {
        self.busy = Some(BusyKind::Reading);
        if self.command_tx.send(command).is_err() {
            self.busy = None;
        }
        self.update_ui(window);
    }

    fn show_menu(&self, window: HWND) {
        let menu = match unsafe { CreatePopupMenu() } {
            Ok(menu) => menu,
            Err(_) => return,
        };
        // Keep the popup in the native system theme. A dark MIM_BACKGROUND
        // without matching owner-drawn text can make the tray menu unreadable.
        let disabled = if self.busy.is_some() {
            MF_GRAYED
        } else {
            MF_STRING
        };
        append_menu(menu, MF_STRING, MENU_OPEN, "Open");
        append_menu(menu, disabled, MENU_DEVELOPER, "Developer");
        append_menu(menu, disabled, MENU_GAMING, "Gaming");
        let profile_line = self
            .presentation
            .as_ref()
            .map_or_else(|| "Reading\u{2026}".to_owned(), |p| p.profile_line.clone());
        append_menu(menu, MF_GRAYED, 0, &format!("Viper V4 Pro: {profile_line}"));
        append_menu(
            menu,
            MF_STRING | checked(self.config.start_with_windows),
            MENU_STARTUP,
            "Start with Windows",
        );
        append_menu(
            menu,
            MF_STRING | checked(self.config.diagnostics_enabled),
            MENU_DIAGNOSTICS,
            "Diagnostics",
        );
        append_menu(
            menu,
            MF_STRING | checked(self.config.exit_after_gaming),
            MENU_EXIT_AFTER_GAMING,
            "Exit after Gaming",
        );
        append_menu(menu, disabled, MENU_EXIT, "Exit");

        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_ok() {
            let _ = unsafe { SetForegroundWindow(window) };
            let _ = unsafe {
                TrackPopupMenu(
                    menu,
                    TPM_RIGHTBUTTON,
                    point.x,
                    point.y,
                    Some(0),
                    window,
                    None,
                )
            };
        }
        let _ = unsafe { DestroyMenu(menu) };
    }
}

fn draw_panel(hdc: HDC, rect: RECT, fill: COLORREF, border: COLORREF, corner_radius: i32) {
    let brush = unsafe { CreateSolidBrush(fill) };
    let pen = unsafe { CreatePen(PS_SOLID, 1, border) };
    let old_brush = unsafe { SelectObject(hdc, HGDIOBJ(brush.0)) };
    let old_pen = unsafe { SelectObject(hdc, HGDIOBJ(pen.0)) };
    let _ = unsafe {
        RoundRect(
            hdc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            corner_radius,
            corner_radius,
        )
    };
    let _ = unsafe { SelectObject(hdc, old_pen) };
    let _ = unsafe { SelectObject(hdc, old_brush) };
    let _ = unsafe { DeleteObject(HGDIOBJ(pen.0)) };
    let _ = unsafe { DeleteObject(HGDIOBJ(brush.0)) };
}

/// Native fallback for a missing BMP. It is intentionally local to rendering:
/// no input handling, state mutation, or device I/O is tied to this path.
fn draw_mouse_placeholder(hdc: HDC, image: RECT, dpi: u32) {
    let body = RECT {
        left: image.left + scale(24, dpi),
        top: image.top + scale(2, dpi),
        right: image.right - scale(24, dpi),
        bottom: image.bottom - scale(2, dpi),
    };
    draw_panel(hdc, body, MOUSE_BLACK, CAT_OVERLAY0, scale(54, dpi));

    let divider = unsafe { CreatePen(PS_SOLID, scale(1, dpi), CAT_SURFACE1) };
    let old_pen = unsafe { SelectObject(hdc, HGDIOBJ(divider.0)) };
    let center = (body.left + body.right) / 2;
    let _ = unsafe { MoveToEx(hdc, center, body.top + scale(12, dpi), None) };
    let _ = unsafe { LineTo(hdc, center, body.top + scale(58, dpi)) };
    let _ = unsafe { SelectObject(hdc, old_pen) };
    let _ = unsafe { DeleteObject(HGDIOBJ(divider.0)) };

    let wheel_brush = unsafe { CreateSolidBrush(CAT_CRUST) };
    let wheel_pen = unsafe { CreatePen(PS_SOLID, scale(1, dpi), CAT_MAUVE) };
    let old_brush = unsafe { SelectObject(hdc, HGDIOBJ(wheel_brush.0)) };
    let old_pen = unsafe { SelectObject(hdc, HGDIOBJ(wheel_pen.0)) };
    let _ = unsafe {
        Ellipse(
            hdc,
            center - scale(10, dpi),
            body.top + scale(72, dpi),
            center + scale(10, dpi),
            body.top + scale(110, dpi),
        )
    };
    let _ = unsafe { SelectObject(hdc, old_pen) };
    let _ = unsafe { SelectObject(hdc, old_brush) };
    let _ = unsafe { DeleteObject(HGDIOBJ(wheel_pen.0)) };
    let _ = unsafe { DeleteObject(HGDIOBJ(wheel_brush.0)) };
}

fn draw_centered_text(hdc: HDC, mut rect: RECT, text: &str, font: HFONT, color: COLORREF) {
    if font.0.is_null() {
        return;
    }
    let old_font = unsafe { SelectObject(hdc, HGDIOBJ(font.0)) };
    let _ = unsafe { SetTextColor(hdc, color) };
    let _ = unsafe { SetBkMode(hdc, TRANSPARENT) };
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let _ = unsafe {
        DrawTextW(
            hdc,
            &mut wide,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        )
    };
    let _ = unsafe { SelectObject(hdc, old_font) };
}

const fn hotspot_color(control: gui_logic::MouseControl) -> COLORREF {
    match control {
        gui_logic::MouseControl::Left => CAT_BLUE,
        gui_logic::MouseControl::Right => CAT_GREEN,
        gui_logic::MouseControl::Middle => CAT_MAUVE,
        gui_logic::MouseControl::RearSide => CAT_YELLOW,
        gui_logic::MouseControl::FrontSide => CAT_RED,
        gui_logic::MouseControl::Dpi => CAT_PEACH,
    }
}

const fn tone_color(tone: Tone) -> COLORREF {
    match tone {
        Tone::Neutral => CAT_TEXT,
        Tone::Developer => CAT_BLUE,
        Tone::Gaming | Tone::Success => CAT_GREEN,
        Tone::Busy => CAT_SUBTEXT0,
        Tone::Warning => CAT_YELLOW,
        Tone::Error => CAT_RED,
    }
}

fn local_clock() -> String {
    let time = unsafe { GetLocalTime() };
    gui_logic::clock_label(time.wHour, time.wMinute, time.wSecond)
}

fn set_text(control: HWND, text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let _ = unsafe { SetWindowTextW(control, PCWSTR(wide.as_ptr())) };
}

fn checked(enabled: bool) -> windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS {
    if enabled { MF_CHECKED } else { MF_STRING }
}

fn append_menu(
    menu: windows::Win32::UI::WindowsAndMessaging::HMENU,
    flags: windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS,
    id: usize,
    text: &str,
) {
    let mut wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    // The API synchronously copies the menu text.
    let _ = unsafe { AppendMenuW(menu, flags, id, PCWSTR(wide.as_mut_ptr())) };
}

fn add_tray_icon(window: HWND) -> Result<(), String> {
    let icon = load_app_icon(32, 32)?;
    let mut data = tray_data(window);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = TRAY_CALLBACK;
    data.hIcon = icon;
    copy_wide(&mut data.szTip, "Viper V4 Pro: starting");
    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        return Err("could not add Viper V4 Pro tray icon".to_owned());
    }
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    let _ = unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) };
    Ok(())
}

fn load_app_icon(width: i32, height: i32) -> Result<HICON, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not locate the utility icon: {error}"))?;
    let icon_path = executable.with_file_name(ICON_FILE_NAME);
    let wide: Vec<u16> = icon_path.as_os_str().encode_wide().chain(Some(0)).collect();
    match unsafe {
        LoadImageW(
            None,
            PCWSTR(wide.as_ptr()),
            IMAGE_ICON,
            width,
            height,
            LR_LOADFROMFILE,
        )
    } {
        Ok(handle) => Ok(HICON(handle.0)),
        Err(file_error) => unsafe { LoadIconW(None, IDI_APPLICATION) }.map_err(|fallback_error| {
            format!(
                "could not load custom icon {} ({file_error}) or fallback icon ({fallback_error})",
                icon_path.display()
            )
        }),
    }
}

/// Loads the optional product visual without making the dashboard depend on an
/// asset being present. The installer should place the BMP beside the EXE;
/// development launches also look under the package working directory's
/// `assets/` folder.
fn load_dashboard_mouse_bitmap() -> Option<DashboardBitmap> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join(DASHBOARD_MOUSE_BMP));
            candidates.push(directory.join("assets").join(DASHBOARD_MOUSE_BMP));
        }
    }
    if let Ok(directory) = std::env::current_dir() {
        candidates.push(directory.join("assets").join(DASHBOARD_MOUSE_BMP));
    }
    for path in candidates {
        if !path.is_file() {
            continue;
        }
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle = match unsafe {
            LoadImageW(
                None,
                PCWSTR(wide.as_ptr()),
                IMAGE_BITMAP,
                0,
                0,
                LR_LOADFROMFILE,
            )
        } {
            Ok(handle) => HBITMAP(handle.0),
            Err(_) => continue,
        };
        let mut metadata = BITMAP::default();
        let copied = unsafe {
            GetObjectW(
                HGDIOBJ(handle.0),
                i32::try_from(size_of::<BITMAP>()).unwrap_or_default(),
                Some((&mut metadata as *mut BITMAP).cast()),
            )
        };
        if copied > 0 && metadata.bmWidth > 0 && metadata.bmHeight > 0 {
            return Some(DashboardBitmap {
                handle,
                width: metadata.bmWidth,
                height: metadata.bmHeight,
            });
        }
        let _ = unsafe { DeleteObject(HGDIOBJ(handle.0)) };
    }
    None
}

fn remove_tray_icon(window: HWND) {
    let data = tray_data(window);
    let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
}

fn set_tooltip(window: HWND, text: &str) {
    let mut data = tray_data(window);
    data.uFlags = NIF_TIP;
    copy_wide(&mut data.szTip, text);
    let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
}

fn notify(window: HWND, title: &str, text: &str, error: bool) {
    let mut data = tray_data(window);
    data.uFlags = NIF_INFO;
    data.dwInfoFlags = if error { NIIF_ERROR } else { NIIF_INFO };
    copy_wide(&mut data.szInfoTitle, title);
    copy_wide(&mut data.szInfo, text);
    let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
}

fn tray_data(window: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: u32::try_from(size_of::<NOTIFYICONDATAW>()).expect("NOTIFYICONDATAW size fits"),
        hWnd: window,
        uID: TRAY_ICON_ID,
        ..Default::default()
    }
}

fn copy_wide(target: &mut [u16], text: &str) {
    target.fill(0);
    for (index, value) in text
        .encode_utf16()
        .take(target.len().saturating_sub(1))
        .enumerate()
    {
        target[index] = value;
    }
}

fn worker_loop(
    command_rx: Receiver<WorkerCommand>,
    event_tx: Sender<WorkerEvent>,
    window_bits: usize,
) {
    for command in command_rx {
        let event = match handle_worker_command(command) {
            Ok(event) => event,
            Err(error) => WorkerEvent::Error(error),
        };
        if event_tx.send(event).is_err() {
            return;
        }
        let window = HWND(window_bits as *mut _);
        if unsafe { PostMessageW(Some(window), WORKER_EVENT, WPARAM(0), LPARAM(0)) }.is_err() {
            return;
        }
    }
}

fn handle_worker_command(command: WorkerCommand) -> Result<WorkerEvent, String> {
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut config = load_config(&paths).map_err(|error| error.to_string())?;
    let diagnostic_event = match command {
        WorkerCommand::Refresh => "refresh",
        WorkerCommand::Apply(ProfileName::Developer) => "apply developer",
        WorkerCommand::Apply(ProfileName::Gaming) => "apply gaming",
        WorkerCommand::SetStartWithWindows(_) => "change startup option",
        WorkerCommand::SetDiagnostics(_) => "change diagnostics option",
        WorkerCommand::SetExitAfterGaming(_) => "change exit-after-gaming option",
    };
    if config.diagnostics_enabled {
        append_diagnostic(&paths, &format!("{diagnostic_event} started"))
            .map_err(|error| error.to_string())?;
    }
    let result = match command {
        WorkerCommand::Refresh => refresh_state(&paths, &mut config),
        WorkerCommand::Apply(profile) => apply_profile(&paths, &mut config, profile),
        WorkerCommand::SetStartWithWindows(enabled) => {
            set_startup_value(enabled)?;
            config.start_with_windows = enabled;
            save_config(&paths, &config).map_err(|error| error.to_string())?;
            Ok(WorkerEvent::Config(config.clone()))
        }
        WorkerCommand::SetDiagnostics(enabled) => {
            config.diagnostics_enabled = enabled;
            save_config(&paths, &config).map_err(|error| error.to_string())?;
            if enabled {
                append_diagnostic(&paths, "diagnostics enabled")
                    .map_err(|error| error.to_string())?;
            }
            Ok(WorkerEvent::Config(config.clone()))
        }
        WorkerCommand::SetExitAfterGaming(enabled) => {
            config.exit_after_gaming = enabled;
            save_config(&paths, &config).map_err(|error| error.to_string())?;
            Ok(WorkerEvent::Config(config.clone()))
        }
    };
    if config.diagnostics_enabled {
        let failed = matches!(&result, Err(_) | Ok(WorkerEvent::ApplyFailed { .. }));
        let outcome = if failed { "failed" } else { "succeeded" };
        append_diagnostic(&paths, &format!("{diagnostic_event} {outcome}"))
            .map_err(|error| error.to_string())?;
    }
    result
}

fn refresh_state(
    paths: &StoragePaths,
    config: &mut UtilityConfigV1,
) -> Result<WorkerEvent, String> {
    // Keep the per-user Run value consistent after a prior crash or moved
    // executable. This never opens the mouse or sends a HID command.
    set_startup_value(config.start_with_windows)?;
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let snapshot = crate::engine::DeviceControl::read_snapshot(&mut device)?;
    let baseline = read_snapshot(&paths.baseline_path(&snapshot.device.serial))
        .map_err(|error| format!("immutable baseline is required before tray use: {error}"))?;
    let profile = classify_profile(&snapshot, &baseline)?;
    if let ProfileMatch::Developer = profile {
        config.last_verified_profile = Some(ProfileName::Developer);
        save_config(paths, config).map_err(|error| error.to_string())?;
    } else if let ProfileMatch::Gaming = profile {
        config.last_verified_profile = Some(ProfileName::Gaming);
        save_config(paths, config).map_err(|error| error.to_string())?;
    }
    Ok(WorkerEvent::State {
        profile,
        polling_hz: snapshot.polling.hertz(),
        dpi: Some(snapshot.dpi.current),
        power: snapshot.power,
        button_assignments: snapshot.button_assignments,
        config: config.clone(),
    })
}

fn apply_profile(
    paths: &StoragePaths,
    config: &mut UtilityConfigV1,
    profile: ProfileName,
) -> Result<WorkerEvent, String> {
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let before = crate::engine::DeviceControl::read_snapshot(&mut device)?;
    let baseline = read_snapshot(&paths.baseline_path(&before.device.serial)).map_err(|error| {
        format!("immutable baseline is required before any tray write: {error}")
    })?;
    let plan = plan_profile_with_baseline(&before, &baseline, profile)
        .map_err(|error| error.to_string())?;
    // This gate is deliberately before both the journal and apply engine: an
    // unproven polling origin/target causes zero setters and zero write report.
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    write_plan_journal(paths, &plan).map_err(|error| error.to_string())?;
    let no_changes_needed = plan.is_noop();
    let report = apply_write_plan(&mut device, &plan);
    let report_path = write_verification_report(paths, &report, &before.device.serial)
        .map_err(|error| error.to_string())?;
    let final_dpi = report.final_state.as_ref().map(|state| state.dpi.current);
    let final_power = report.final_state.as_ref().and_then(|state| state.power);
    let final_button_assignments = report
        .final_state
        .as_ref()
        .map(|state| state.button_assignments.clone());
    if !report.success {
        let final_profile = report
            .final_state
            .as_ref()
            .and_then(|state| classify_profile(state, &baseline).ok());
        let polling_hz = report
            .final_state
            .as_ref()
            .and_then(|state| state.polling.hertz());
        return Ok(WorkerEvent::ApplyFailed {
            profile,
            detail: gui_logic::failure_detail(
                report
                    .rollback_result
                    .as_ref()
                    .map(|rollback| rollback.final_state_restored),
                final_profile,
            ),
            report_path: Some(report_path.display().to_string()),
            final_profile,
            polling_hz,
            dpi: final_dpi,
            power: final_power,
            button_assignments: final_button_assignments,
        });
    }
    config.last_verified_profile = Some(profile);
    save_config(paths, config).map_err(|error| error.to_string())?;
    Ok(WorkerEvent::Applied {
        profile,
        config: config.clone(),
        no_changes_needed,
        report_path: Some(report_path.display().to_string()),
        dpi: final_dpi,
        power: final_power,
        button_assignments: final_button_assignments,
    })
}

fn set_startup_value(enabled: bool) -> Result<(), String> {
    let mut key = HKEY::default();
    registry_ok(
        unsafe { RegCreateKeyW(HKEY_CURRENT_USER, STARTUP_KEY, &mut key) },
        "create startup registry key",
    )?;
    let result = if enabled {
        let executable = std::env::current_exe()
            .map_err(|error| format!("could not find utility executable: {error}"))?;
        // --minimized keeps an automatic login start in the tray; a shortcut
        // launch without the flag shows the window.
        let command = format!("\"{}\" --minimized", executable.display());
        let mut bytes = Vec::with_capacity((command.len() + 1) * 2);
        for value in command.encode_utf16().chain(Some(0)) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        registry_ok(
            unsafe { RegSetValueExW(key, STARTUP_VALUE, Some(0), REG_SZ, Some(&bytes)) },
            "set startup registry value",
        )
    } else {
        let status = unsafe { RegDeleteValueW(key, STARTUP_VALUE) };
        if status == WIN32_ERROR(0) || status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(format!(
                "remove startup registry value failed with Win32 error {}",
                status.0
            ))
        }
    };
    let close = unsafe { RegCloseKey(key) };
    result?;
    registry_ok(close, "close startup registry key")
}

fn registry_ok(status: WIN32_ERROR, action: &str) -> Result<(), String> {
    if status == WIN32_ERROR(0) {
        Ok(())
    } else {
        Err(format!("{action} failed with Win32 error {}", status.0))
    }
}
