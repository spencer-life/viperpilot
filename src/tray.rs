//! The intentionally small native Windows surface for the fixed Viper profiles:
//! one tray icon plus one compact status window in the same process.
//!
//! Device I/O is confined to one worker thread. The UI thread is a normal Win32
//! `GetMessageW` loop, so it does not poll while idle and never observes mouse
//! input. The window renders state decided by the platform-independent
//! `gui_logic` module and performs no HID work of its own.

use core::ffi::c_void;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
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
    BITMAP, BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CLR_INVALID, CreateCompatibleDC,
    CreateFontW, CreatePen, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CALCRECT,
    DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawFocusRect, DrawTextW,
    Ellipse, EndPaint, FONT_WEIGHT, FW_BOLD, FW_NORMAL, FillRect, GetCurrentObject, GetObjectW,
    HALFTONE, HBITMAP, HBRUSH, HDC, HFONT, HGDIOBJ, InvalidateRect, LineTo, MoveToEx, OBJ_BRUSH,
    OBJ_PEN, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, RDW_ALLCHILDREN, RDW_INVALIDATE,
    RedrawWindow, RestoreDC, RoundRect, SRCCOPY, SaveDC, SelectObject, SetBkMode,
    SetStretchBltMode, SetTextColor, StretchBlt, TRANSPARENT,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, REG_SZ, RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegSetValueExW,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Accessibility::{
    CLSID_AccPropServices, HCF_HIGHCONTRASTON, HIGHCONTRASTW, IAccPropServices, Name_Property_GUID,
};
use windows::Win32::UI::Controls::{
    CDDS_PREPAINT, CDIS_DISABLED, CDIS_FOCUS, CDIS_HOT, CDIS_SELECTED, CDRF_SKIPDEFAULT,
    DRAWITEMSTRUCT, NM_CUSTOMDRAW, NMCUSTOMDRAW, NMHDR, ODS_DISABLED, ODS_FOCUS, ODS_SELECTED,
};
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
    AppendMenuW, BN_CLICKED, CREATESTRUCTW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, FindWindowW, GWLP_USERDATA,
    GetClientRect, GetCursorPos, GetDlgItem, GetMessageW, GetWindowLongPtrW, HICON, HMENU,
    ICON_BIG, ICON_SMALL, IDC_ARROW, IDI_APPLICATION, IMAGE_BITMAP, IMAGE_ICON, IsDialogMessageW,
    IsIconic, LR_LOADFROMFILE, LoadCursorW, LoadIconW, LoadImageW, MF_CHECKED, MF_GRAYED, MF_POPUP,
    MF_STRING, MSG, MoveWindow, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SPI_GETHIGHCONTRAST, SW_HIDE, SW_RESTORE, SW_SHOW, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, SystemParametersInfoW, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_CONTEXTMENU, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DEVICECHANGE, WM_DPICHANGED, WM_DRAWITEM,
    WM_HOTKEY, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NCCREATE, WM_NCDESTROY, WM_NOTIFY, WM_PAINT,
    WM_RBUTTONUP, WM_SETFONT, WM_SETICON, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN,
    WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{BOOL, PCWSTR, w};

use crate::engine::{ProfileMatch, apply_write_plan, classify_profile};
use crate::gui_logic::{
    self, BusyKind, ConnectionStatus, LaunchDecision, StatusPresentation, Tone, VerificationOutcome,
};
use crate::model::{
    DpiPair, ProfileName, ProfileSpec, RawButtonAssignment, UtilityConfigV1, WirelessPowerSettings,
};
use crate::planning::{plan_profile_with_baseline, require_proven_polling_writes};
use crate::profile_library::{MAX_SAVED_PROFILES, ProfileLibraryV1};
use crate::storage::{
    StoragePaths, append_diagnostic, load_config, load_profile_library, read_snapshot, save_config,
    save_profile_library, write_plan_journal, write_verification_report,
};
use crate::windows::{RazerDevice, refuse_if_synapse_running};

// 2026-09-30: reserve development-only Windows identities so future reviewed
// launches cannot signal or reconcile startup for the installed legacy app.
// The early production launch gate remains mandatory and unchanged.
const WINDOW_CLASS: PCWSTR = w!("ViperPilotDevelopmentTrayWindowV1");
const PREVIEW_WINDOW_CLASS: PCWSTR = w!("ViperPilotDevelopmentPreviewWindowV1");
const WINDOW_TITLE: PCWSTR = w!("ViperPilot");
const MUTEX_NAME: PCWSTR = w!("Local\\ViperPilotDevelopmentTrayV1");
const SHOW_MESSAGE_NAME: PCWSTR = w!("ViperPilotDevelopmentShowWindowV1");
const STARTUP_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const STARTUP_VALUE: PCWSTR = w!("ViperPilotDevelopment");
const ICON_FILE_NAME: &str = "viper-utility-icon.ico";
const DASHBOARD_MOUSE_BMP: &str = "viper-v4-black-dashboard.bmp";
const COMPACT_MOUSE_BMP: &str = "viperpilot-mouse-silhouette.bmp";
const TRAY_ICON_ID: u32 = 1;
const TRAY_CALLBACK: u32 = WM_APP + 1;
const WORKER_EVENT: u32 = WM_APP + 2;
const DEFERRED_DRAIN_EVENT: u32 = WM_APP + 3;
const HOTKEY_ID: i32 = 0x5654;

const MENU_OPEN: usize = 1000;
const MENU_DEVELOPER: usize = 1001;
const MENU_GAMING: usize = 1002;
const MENU_STARTUP: usize = 1003;
const MENU_DIAGNOSTICS: usize = 1004;
const MENU_EXIT_AFTER_GAMING: usize = 1005;
const MENU_EXIT: usize = 1006;
const MENU_RELOAD_LIBRARY: usize = 1007;
const MENU_QUICK_APPLY_FIRST: usize = 1008;
const MENU_QUICK_APPLY_SECOND: usize = 1009;
const MENU_SAVED_APPLY_BASE: usize = 3000;
const MENU_QUICK_FIRST_BASE: usize = 4000;
const MENU_QUICK_SECOND_BASE: usize = MENU_QUICK_FIRST_BASE + MAX_SAVED_PROFILES;
const BUTTON_DEVELOPER: usize = 2001;
const BUTTON_GAMING: usize = 2002;
const BUTTON_VIEW_MODE: usize = 2003;
const HOTSPOT_LEFT: usize = 2101;
const HOTSPOT_RIGHT: usize = 2102;
const HOTSPOT_MIDDLE: usize = 2103;
const HOTSPOT_REAR_SIDE: usize = 2104;
const HOTSPOT_FRONT_SIDE: usize = 2105;
const HOTSPOT_DPI: usize = 2106;

// Numeric button/static styles follow the existing WINDOW_STYLE representation.
// 2026-09-30: button values verified against the pinned windows 0.62.2
// Win32_UI_WindowsAndMessaging constants (BS_PUSHBUTTON=0, BS_MULTILINE=8192).
// 2026-09-30: earlier UIA evidence exposed owner-drawn buttons as panes without
// InvokePattern. Use standard BUTTON semantics; verify actual UIA on Windows.
const BS_PUSHBUTTON_STYLE: u32 = 0x0000_0000;
const BS_MULTILINE_STYLE: u32 = 0x0000_2000;
const SS_NOPREFIX_STYLE: u32 = 0x0000_0080;
const SS_EDITCONTROL_STYLE: u32 = 0x0000_2000;

const MAIN_WINDOW_STYLE: WINDOW_STYLE = WINDOW_STYLE(
    WS_OVERLAPPED.0 | WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_CLIPCHILDREN.0,
);

// Fixed layout in 96-DPI units, scaled by the window's live DPI.
const BASE_MARGIN: i32 = 24;
const BASE_CLIENT_WIDTH: i32 = 880;
const BASE_CLIENT_HEIGHT: i32 = 680;
const BASE_COMPACT_WIDTH: i32 = 900;
const BASE_COMPACT_HEIGHT: i32 = 520;
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

// 2026-09-30: align the native preview with the approved ViperPilot
// rose-and-charcoal visual reference. This is presentation only.
const CAT_BASE: COLORREF = rgb(15, 20, 25);
const CAT_CRUST: COLORREF = rgb(15, 20, 25);
const CAT_SURFACE0: COLORREF = rgb(26, 31, 36);
const CAT_SURFACE1: COLORREF = rgb(35, 41, 50);
const CAT_BORDER: COLORREF = rgb(58, 65, 75);
const CAT_OVERLAY0: COLORREF = rgb(163, 172, 183);
const CAT_TEXT: COLORREF = rgb(234, 239, 244);
const CAT_SUBTEXT0: COLORREF = rgb(163, 172, 183);
const CAT_BLUE: COLORREF = rgb(137, 180, 250);
const CAT_GREEN: COLORREF = rgb(60, 203, 143);
const CAT_YELLOW: COLORREF = rgb(249, 226, 175);
const CAT_RED: COLORREF = rgb(243, 139, 168);
const VIPER_ROSE: COLORREF = rgb(232, 125, 145);
const VIPER_ACTIVE: COLORREF = rgb(162, 75, 92);
const CAT_MAUVE: COLORREF = rgb(203, 166, 247);
const CAT_PEACH: COLORREF = rgb(250, 179, 135);
const MOUSE_BLACK: COLORREF = rgb(11, 11, 15);

#[derive(Clone, Debug)]
enum WorkerCommand {
    LoadLibrary,
    Refresh,
    Apply(ProfileName),
    ApplySaved {
        id: String,
        expected_preset: ProfileName,
    },
    SetQuickSwitch {
        slot: usize,
        profile_id: String,
    },
    ToggleQuickSwitch,
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
        final_connection: ConnectionStatus,
        final_profile: Option<ProfileMatch>,
        polling_hz: Option<u16>,
        dpi: Option<DpiPair>,
        power: Option<WirelessPowerSettings>,
        button_assignments: Option<Vec<RawButtonAssignment>>,
    },
    Config(UtilityConfigV1),
    LibraryUpdated(ProfileLibraryV1),
    LibraryError(String),
    LocalError(String),
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QuickSwitchAction {
    profile_id: String,
    expected_preset: ProfileName,
    label: String,
}

fn quick_switch_action(
    library: &ProfileLibraryV1,
    slot: usize,
) -> Result<QuickSwitchAction, String> {
    library.validate()?;
    let profile_id = library
        .quick_switch()
        .get(slot)
        .ok_or_else(|| format!("quick-switch slot {slot} is unavailable"))?;
    let entry = library
        .entries()
        .iter()
        .find(|entry| entry.id() == profile_id)
        .ok_or_else(|| format!("quick-switch profile {profile_id:?} is unavailable"))?;
    Ok(QuickSwitchAction {
        profile_id: entry.id().to_owned(),
        expected_preset: entry.source_preset(),
        label: format!(
            "Switch to {} — complete {} preset",
            entry.name(),
            preset_display_name(entry.source_preset())
        ),
    })
}

fn preset_display_name(preset: ProfileName) -> &'static str {
    match preset {
        ProfileName::Developer => "Developer",
        ProfileName::Gaming => "Gaming",
    }
}

fn quick_switch_slot_for_command(command: usize) -> Option<usize> {
    match command {
        MENU_QUICK_APPLY_FIRST => Some(0),
        MENU_QUICK_APPLY_SECOND => Some(1),
        _ => None,
    }
}

fn resolve_saved_apply(
    library: &ProfileLibraryV1,
    profile_id: &str,
    expected_preset: ProfileName,
) -> Result<ProfileName, String> {
    library.validate()?;
    let entry = library
        .entries()
        .iter()
        .find(|entry| entry.id() == profile_id)
        .ok_or_else(|| format!("saved local profile {profile_id:?} no longer exists"))?;
    if entry.source_preset() != expected_preset {
        return Err(format!(
            "saved local profile {profile_id:?} changed since the menu was opened; reload saved profiles and try again"
        ));
    }
    Ok(entry.source_preset())
}

fn quick_slot_entry_eligible(library: &ProfileLibraryV1, slot: usize, profile_id: &str) -> bool {
    if library.validate().is_err() || slot > 1 {
        return false;
    }
    let other_preset = library
        .quick_switch()
        .get(1 - slot)
        .and_then(|other_id| {
            library
                .entries()
                .iter()
                .find(|entry| entry.id() == other_id)
        })
        .map(|entry| entry.source_preset());
    let candidate = library
        .entries()
        .iter()
        .find(|entry| entry.id() == profile_id);
    matches!((candidate, other_preset), (Some(entry), Some(other)) if entry.source_preset() != other)
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
    hotspot_names: HotspotNames,
    verification_value: HWND,
    developer_button: HWND,
    gaming_button: HWND,
    view_mode_button: HWND,
    brand_font: HFONT,
    title_font: HFONT,
    body_font: HFONT,
    button_font: HFONT,
}

// 2026-09-30: retain compact visual numbers while annotating standard buttons
// with semantic UIA names. This is Windows' annotation service, not a custom
// provider. Its COM initialization and annotations are scoped to the UI window.
struct ComApartment;

impl ComApartment {
    fn initialize() -> Result<Self, String> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(|error| format!("could not initialize accessible button names: {error}"))?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct HotspotNames {
    services: IAccPropServices,
    controls: Vec<HWND>,
    // Must outlive the interface above; fields are dropped in declaration order.
    _apartment: ComApartment,
}

impl HotspotNames {
    fn new(controls: &[HWND; 6]) -> Result<Self, String> {
        let apartment = ComApartment::initialize()?;
        let services =
            unsafe { CoCreateInstance(&CLSID_AccPropServices, None, CLSCTX_INPROC_SERVER) }
                .map_err(|error| {
                    format!("could not create accessible button name service: {error}")
                })?;
        let mut names = Self {
            services,
            controls: Vec::new(),
            _apartment: apartment,
        };
        for (&control, semantic) in controls.iter().zip(gui_logic::MouseControl::ALL) {
            // OBJID_CLIENT is DWORD(-4), and CHILDID_SELF is zero.
            unsafe {
                names.services.SetHwndPropStr(
                    control,
                    0xffff_fffc,
                    0,
                    Name_Property_GUID,
                    PCWSTR(wide(semantic.title()).as_ptr()),
                )
            }
            .map_err(|error| format!("could not name {} button: {error}", semantic.title()))?;
            names.controls.push(control);
        }
        Ok(names)
    }

    fn clear(&mut self) {
        for control in self.controls.drain(..) {
            let _ = unsafe {
                self.services
                    .ClearHwndProps(control, 0xffff_fffc, 0, &[Name_Property_GUID])
            };
        }
    }
}

impl Drop for HotspotNames {
    fn drop(&mut self) {
        self.clear();
    }
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
    library: Option<ProfileLibraryV1>,
    initializing: bool,
    busy: Option<BusyKind>,
    connection: ConnectionStatus,
    polling_hz: Option<u16>,
    dpi: Option<DpiPair>,
    power: Option<WirelessPowerSettings>,
    button_assignments: Option<Vec<RawButtonAssignment>>,
    selected_mouse_control: gui_logic::MouseControl,
    last_verification: Option<VerificationOutcome>,
    last_error: Option<String>,
    presentation: Option<StatusPresentation>,
    detailed: bool,
    status: Option<StatusWindow>,
    base_brush: HBRUSH,
    surface_brush: HBRUSH,
    mouse_bitmap: Option<DashboardBitmap>,
    compact_mouse_bitmap: Option<DashboardBitmap>,
    preview_mode: bool,
    quit_requested: bool,
}

/// Stable state installed in GWLP_USERDATA. Win32 can synchronously send
/// messages while a handler calls layout, paints an owner-drawn control, or
/// tracks a popup menu, so handlers must never create aliased `&mut TrayApp`s.
struct WindowRuntime {
    app: RefCell<TrayApp>,
    deferred: RefCell<VecDeque<DeferredAction>>,
    draining: Cell<bool>,
    paint_pending: Cell<bool>,
    popup_pending: Cell<bool>,
    closing: Cell<bool>,
    preview_mode: bool,
    show_message: u32,
    fallback_brush: HBRUSH,
}

#[derive(Clone, Copy)]
enum DeferredAction {
    Command(usize),
    Hotkey,
    WorkerEvent,
    Refresh,
    TrayOpen,
    TrayMenu,
    Dpi(RECT),
    ShowWindow,
}

fn is_known_child_command(id: usize, notification: u16) -> bool {
    notification == BN_CLICKED as u16
        && matches!(
            id,
            BUTTON_DEVELOPER
                | BUTTON_GAMING
                | BUTTON_VIEW_MODE
                | HOTSPOT_LEFT
                | HOTSPOT_RIGHT
                | HOTSPOT_MIDDLE
                | HOTSPOT_REAR_SIDE
                | HOTSPOT_FRONT_SIDE
                | HOTSPOT_DPI
        )
}

fn is_known_menu_command(id: usize) -> bool {
    matches!(
        id,
        MENU_OPEN
            | MENU_DEVELOPER
            | MENU_GAMING
            | MENU_STARTUP
            | MENU_DIAGNOSTICS
            | MENU_EXIT_AFTER_GAMING
            | MENU_EXIT
            | MENU_RELOAD_LIBRARY
            | MENU_QUICK_APPLY_FIRST
            | MENU_QUICK_APPLY_SECOND
    ) || (MENU_SAVED_APPLY_BASE..MENU_SAVED_APPLY_BASE + MAX_SAVED_PROFILES).contains(&id)
        || (MENU_QUICK_FIRST_BASE..MENU_QUICK_SECOND_BASE + MAX_SAVED_PROFILES).contains(&id)
}

fn ignored_while_closing(closing: bool, show_message: u32, message: u32) -> bool {
    closing
        && (matches!(
            message,
            DEFERRED_DRAIN_EVENT
                | WORKER_EVENT
                | WM_HOTKEY
                | WM_DEVICECHANGE
                | WM_COMMAND
                | TRAY_CALLBACK
        ) || (show_message != 0 && message == show_message))
}

fn valid_window_command(window: HWND, wparam: WPARAM, lparam: LPARAM) -> Option<usize> {
    let id = wparam.0 & 0xffff;
    if lparam.0 == 0 {
        return is_known_menu_command(id).then_some(id);
    }
    let notification = ((wparam.0 >> 16) & 0xffff) as u16;
    if !is_known_child_command(id, notification) {
        return None;
    }
    let child = HWND(lparam.0 as *mut c_void);
    unsafe { GetDlgItem(Some(window), id as i32) }
        .ok()
        .filter(|expected| *expected == child)
        .map(|_| id)
}

#[cfg(test)]
mod window_command_tests {
    use super::*;

    #[test]
    fn child_commands_require_known_button_and_click_notification() {
        assert!(is_known_child_command(BUTTON_DEVELOPER, BN_CLICKED as u16));
        assert!(is_known_child_command(HOTSPOT_DPI, BN_CLICKED as u16));
        assert!(!is_known_child_command(9999, BN_CLICKED as u16));
        assert!(!is_known_child_command(BUTTON_DEVELOPER, 5));
        assert!(!is_known_menu_command(BUTTON_DEVELOPER));
    }

    #[test]
    fn registered_show_signal_is_ignored_only_while_closing() {
        let show_message = WM_APP + 0x154;
        assert!(!ignored_while_closing(false, show_message, show_message));
        assert!(ignored_while_closing(true, show_message, show_message));
        assert!(!ignored_while_closing(true, show_message, WM_APP + 0x155));
    }

    #[test]
    fn menu_commands_accept_only_declared_ids_and_ranges() {
        assert!(is_known_menu_command(MENU_OPEN));
        assert!(is_known_menu_command(MENU_SAVED_APPLY_BASE));
        assert!(is_known_menu_command(
            MENU_SAVED_APPLY_BASE + MAX_SAVED_PROFILES - 1
        ));
        assert!(!is_known_menu_command(
            MENU_SAVED_APPLY_BASE + MAX_SAVED_PROFILES
        ));
        assert!(is_known_menu_command(MENU_QUICK_FIRST_BASE));
        assert!(is_known_menu_command(
            MENU_QUICK_SECOND_BASE + MAX_SAVED_PROFILES - 1
        ));
        assert!(!is_known_menu_command(
            MENU_QUICK_SECOND_BASE + MAX_SAVED_PROFILES
        ));
        assert!(!is_known_menu_command(9999));
    }
}

impl WindowRuntime {
    fn new(app: TrayApp, preview_mode: bool, show_message: u32) -> Self {
        Self {
            app: RefCell::new(app),
            deferred: RefCell::new(VecDeque::new()),
            draining: Cell::new(false),
            paint_pending: Cell::new(false),
            popup_pending: Cell::new(false),
            closing: Cell::new(false),
            preview_mode,
            show_message,
            fallback_brush: unsafe { CreateSolidBrush(CAT_SURFACE0) },
        }
    }

    fn defer(&self, action: DeferredAction) {
        self.deferred.borrow_mut().push_back(action);
    }

    fn defer_before_pending_events(&self, action: DeferredAction) {
        self.deferred.borrow_mut().push_front(action);
    }
}

impl Drop for WindowRuntime {
    fn drop(&mut self) {
        if !self.fallback_brush.0.is_null() {
            let _ = unsafe { DeleteObject(HGDIOBJ(self.fallback_brush.0)) };
        }
    }
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
        for bitmap in [
            self.mouse_bitmap.as_ref(),
            self.compact_mouse_bitmap.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.handle.0)) };
        }
    }
}

/// Starts the single-instance, native tray-and-window process. If another
/// instance already holds the mutex, this signals its window to show itself
/// and exits successfully instead of starting a second instance.
pub fn run(show_window_at_start: bool) -> Result<(), String> {
    gui_logic::require_isolated_tray_launch().map_err(str::to_owned)?;
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let show_message = unsafe { RegisterWindowMessageW(SHOW_MESSAGE_NAME) };
    let instance_mutex = match create_single_instance_mutex()? {
        InstanceGate::Primary(handle) => handle,
        InstanceGate::AlreadyRunning => return signal_existing_instance(show_message),
    };
    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    let runtime = Box::new(WindowRuntime::new(
        TrayApp {
            command_tx,
            event_rx,
            current: ProfileMatch::OutOfSync,
            config: UtilityConfigV1::default(),
            library: None,
            initializing: true,
            busy: Some(BusyKind::Reading),
            connection: ConnectionStatus::Connected,
            polling_hz: None,
            dpi: None,
            power: None,
            button_assignments: None,
            selected_mouse_control: gui_logic::MouseControl::RearSide,
            last_verification: None,
            last_error: None,
            presentation: None,
            detailed: false,
            status: None,
            base_brush: unsafe { CreateSolidBrush(CAT_BASE) },
            surface_brush: unsafe { CreateSolidBrush(CAT_SURFACE0) },
            mouse_bitmap: load_mouse_bitmap(DASHBOARD_MOUSE_BMP),
            compact_mouse_bitmap: load_mouse_bitmap(COMPACT_MOUSE_BMP),
            preview_mode: false,
            quit_requested: false,
        },
        false,
        show_message,
    ));

    let window = create_main_window(&runtime)?;
    let controls = match create_status_controls(window) {
        Ok(controls) => controls,
        Err(error) => {
            destroy_status_window(&runtime, window);
            return Err(error);
        }
    };
    {
        let mut app = runtime.app.borrow_mut();
        app.status = Some(controls);
        if let Some(status) = app.status.as_mut() {
            layout_status(window, status, false);
        }
        app.update_ui(window);
    }
    drain_deferred(&runtime, window);

    let worker_window = window.0 as usize;
    let worker = thread::Builder::new()
        .name("viper-device-worker".to_owned())
        .spawn(move || worker_loop(command_rx, event_tx, worker_window))
        .map_err(|error| format!("could not start serialized device worker: {error}"));
    if let Err(error) = worker {
        destroy_status_window(&runtime, window);
        return Err(error);
    }

    if let Err(error) = add_tray_icon(window) {
        destroy_status_window(&runtime, window);
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
        destroy_status_window(&runtime, window);
        return Err(error);
    }
    let startup_request = runtime
        .app
        .borrow()
        .command_tx
        .send(WorkerCommand::LoadLibrary)
        .and_then(|()| runtime.app.borrow().command_tx.send(WorkerCommand::Refresh))
        .map_err(|error| format!("could not request startup state: {error}"));
    if let Err(error) = startup_request {
        let _ = unsafe { UnregisterHotKey(Some(window), HOTKEY_ID) };
        remove_tray_icon(window);
        destroy_status_window(&runtime, window);
        return Err(error);
    }
    if show_window_at_start {
        runtime.app.borrow().show_window_now(window);
        drain_deferred(&runtime, window);
    }

    let result = message_loop(window);
    let _ = unsafe { UnregisterHotKey(Some(window), HOTKEY_ID) };
    remove_tray_icon(window);
    destroy_status_window(&runtime, window);
    drop(runtime);
    drop(instance_mutex);
    result
}

/// Opens the existing native controls with synthetic state only. This opt-in
/// path deliberately never starts the device worker or initializes tray,
/// startup, hotkey, mutex, storage, or Synapse integrations.
#[cfg(feature = "ui-preview")]
pub fn run_preview(reversed_pair: bool, unavailable_library: bool) -> Result<(), String> {
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    drop(event_tx);
    drop(command_rx);
    let mut library = (!unavailable_library).then(ProfileLibraryV1::default);
    if reversed_pair && let Some(library) = library.as_mut() {
        library.set_quick_switch("gaming", "developer")?;
    }
    let runtime = Box::new(WindowRuntime::new(
        TrayApp {
            command_tx,
            event_rx,
            current: ProfileMatch::Developer,
            config: UtilityConfigV1::default(),
            library,
            initializing: false,
            busy: None,
            connection: ConnectionStatus::Connected,
            polling_hz: Some(1000),
            dpi: Some(DpiPair { x: 1600, y: 1600 }),
            power: None,
            button_assignments: None,
            selected_mouse_control: gui_logic::MouseControl::RearSide,
            last_verification: None,
            last_error: None,
            presentation: None,
            detailed: false,
            status: None,
            base_brush: unsafe { CreateSolidBrush(CAT_BASE) },
            surface_brush: unsafe { CreateSolidBrush(CAT_SURFACE0) },
            mouse_bitmap: load_mouse_bitmap(DASHBOARD_MOUSE_BMP),
            compact_mouse_bitmap: load_mouse_bitmap(COMPACT_MOUSE_BMP),
            preview_mode: true,
            quit_requested: false,
        },
        true,
        0,
    ));
    let window = create_main_window(&runtime)?;
    let controls = match create_status_controls(window) {
        Ok(controls) => controls,
        Err(error) => {
            destroy_status_window(&runtime, window);
            return Err(error);
        }
    };
    {
        let mut app = runtime.app.borrow_mut();
        app.status = Some(controls);
        if let Some(status) = app.status.as_mut() {
            layout_status(window, status, false);
            set_text(
                status.brand_subtitle,
                "PREVIEW / NO DEVICE ACCESS — synthetic in-memory state",
            );
        }
        app.update_ui(window);
    }
    drain_deferred(&runtime, window);
    let _ = unsafe { ShowWindow(window, SW_SHOW) };
    let _ = unsafe { SetForegroundWindow(window) };
    let result = message_loop(window);
    destroy_status_window(&runtime, window);
    drop(runtime);
    result
}

#[cfg(feature = "ui-preview")]
unsafe fn preview_window_proc(
    app: &mut TrayApp,
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_DEVICECHANGE | WM_HOTKEY | TRAY_CALLBACK | WORKER_EVENT => LRESULT(0),
        WM_COMMAND => {
            let command = wparam.0 & 0xffff;
            if command == BUTTON_VIEW_MODE {
                app.detailed = !app.detailed;
                if let Some(status) = app.status.as_mut() {
                    layout_status(window, status, app.detailed);
                    set_text(
                        status.brand_subtitle,
                        "PREVIEW / NO DEVICE ACCESS — synthetic in-memory state",
                    );
                }
                app.update_ui(window);
            } else if let Some(control) = mouse_control_for_hotspot(command) {
                app.selected_mouse_control = control;
                app.update_ui(window);
            } else {
                let slot = match command {
                    BUTTON_DEVELOPER => Some(0),
                    BUTTON_GAMING => Some(1),
                    _ => None,
                };
                if let Some(slot) = slot {
                    let target = if app.detailed {
                        Some(if slot == 0 {
                            ProfileName::Developer
                        } else {
                            ProfileName::Gaming
                        })
                    } else {
                        app.library
                            .as_ref()
                            .and_then(|library| quick_switch_action(library, slot).ok())
                            .map(|action| action.expected_preset)
                    };
                    if let Some(target) = target {
                        app.current = match target {
                            ProfileName::Developer => ProfileMatch::Developer,
                            ProfileName::Gaming => ProfileMatch::Gaming,
                        };
                        app.update_ui(window);
                    }
                }
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
                layout_status(window, status, app.detailed);
                set_text(
                    status.brand_subtitle,
                    "PREVIEW / NO DEVICE ACCESS — synthetic in-memory state",
                );
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            // Exit the loop first; run_preview destroys the window only after
            // this callback's mutable TrayApp borrow has ended.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
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

fn create_main_window(runtime: &WindowRuntime) -> Result<HWND, String> {
    let (window_class, title, base_brush) = {
        let app = runtime.app.borrow();
        (
            if app.preview_mode {
                PREVIEW_WINDOW_CLASS
            } else {
                WINDOW_CLASS
            },
            if app.preview_mode {
                w!("PREVIEW — ViperPilot")
            } else {
                WINDOW_TITLE
            },
            app.base_brush,
        )
    };
    let large_icon = load_app_icon(32, 32)?;
    let small_icon = load_app_icon(16, 16)?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        lpszClassName: window_class,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        hIcon: large_icon,
        hbrBackground: base_brush,
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
            window_class,
            title,
            MAIN_WINDOW_STYLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            BASE_COMPACT_WIDTH,
            BASE_COMPACT_HEIGHT,
            None,
            None,
            None,
            Some((runtime as *const WindowRuntime as *mut WindowRuntime).cast()),
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

fn destroy_status_window(runtime: &WindowRuntime, window: HWND) {
    // Call outside callbacks/TrayApp borrows, before child HWNDs are destroyed.
    if let Ok(mut app) = runtime.app.try_borrow_mut()
        && let Some(status) = app.status.as_mut()
    {
        status.hotspot_names.clear();
    }
    let _ = unsafe { DestroyWindow(window) };
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
    let hotspot_style =
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_PUSHBUTTON_STYLE);
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
        "Apply Developer recovery preset",
        WINDOW_STYLE(
            WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_PUSHBUTTON_STYLE | BS_MULTILINE_STYLE,
        ),
        BUTTON_DEVELOPER,
    )?;
    let gaming_button = create_child(
        window,
        w!("BUTTON"),
        "Apply Gaming recovery preset",
        WINDOW_STYLE(
            WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_PUSHBUTTON_STYLE | BS_MULTILINE_STYLE,
        ),
        BUTTON_GAMING,
    )?;
    let view_mode_button = create_child(
        window,
        w!("BUTTON"),
        "Details",
        WINDOW_STYLE(
            WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_PUSHBUTTON_STYLE | BS_MULTILINE_STYLE,
        ),
        BUTTON_VIEW_MODE,
    )?;
    let hotspot_names = HotspotNames::new(&mouse_hotspots)?;
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
        hotspot_names,
        verification_value,
        developer_button,
        gaming_button,
        view_mode_button,
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
            BASE_MOUSE_IMAGE_TOP + 30,
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
            BASE_MOUSE_IMAGE_TOP + 126,
            34,
            34,
        ),
        gui_logic::MouseControl::Dpi => (
            BASE_MOUSE_IMAGE_LEFT + 78,
            BASE_MOUSE_IMAGE_TOP + 102,
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
fn layout_status(window: HWND, status: &mut StatusWindow, detailed: bool) {
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
    set_font(status.view_mode_button, status.button_font);

    if !detailed {
        layout_compact_status(window, status, dpi);
        return;
    }
    set_status_details_visible(status, true);
    let _ = unsafe { SetWindowTextW(status.brand_title, PCWSTR(wide("ViperPilot").as_ptr())) };
    let _ = unsafe {
        SetWindowTextW(
            status.brand_subtitle,
            PCWSTR(wide("Verified onboard profiles  ·  Ctrl + Alt + P switches").as_ptr()),
        )
    };
    let _ = unsafe { SetWindowTextW(status.labels[0], PCWSTR(wide("PROFILE").as_ptr())) };
    let _ = unsafe { SetWindowTextW(status.labels[1], PCWSTR(wide("DPI").as_ptr())) };
    let _ = unsafe { SetWindowTextW(status.labels[2], PCWSTR(wide("POLLING").as_ptr())) };
    let _ = unsafe { SetWindowTextW(status.labels[6], PCWSTR(wide("CONNECTION").as_ptr())) };
    let _ = unsafe { SetWindowTextW(status.labels[7], PCWSTR(wide("VERIFICATION").as_ptr())) };
    let _ = unsafe {
        SetWindowTextW(
            status.assignment_heading,
            PCWSTR(wide("SELECTED ONBOARD ASSIGNMENT  ·  READ-ONLY").as_ptr()),
        )
    };

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
    move_child(
        status.view_mode_button,
        client_width - scale(124, dpi),
        scale(12, dpi),
        scale(100, dpi),
        scale(34, dpi),
    );

    let [profile_card, performance_card, power_card, link_card] = [
        status_card_bounds(0, dpi),
        status_card_bounds(1, dpi),
        status_card_bounds(2, dpi),
        status_card_bounds(3, dpi),
    ];
    let card_inset = scale(12, dpi);
    let label_height = scale(15, dpi);
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
        power_card.top + scale(4, dpi),
        card_width(power_card) - 2 * card_inset,
        scale(12, dpi),
    );
    move_child(
        status.battery_value,
        power_card.left + card_inset,
        power_card.top + scale(16, dpi),
        card_width(power_card) - 2 * card_inset,
        scale(15, dpi),
    );
    move_child(
        status.labels[4],
        power_card.left + card_inset,
        power_card.top + scale(30, dpi),
        card_width(power_card) - 2 * card_inset,
        scale(12, dpi),
    );
    move_child(
        status.sleep_value,
        power_card.left + card_inset,
        power_card.top + scale(42, dpi),
        card_width(power_card) - 2 * card_inset,
        scale(15, dpi),
    );
    move_child(
        status.labels[5],
        power_card.left + card_inset,
        power_card.top + scale(56, dpi),
        card_width(power_card) - 2 * card_inset,
        scale(12, dpi),
    );
    move_child(
        status.low_power_value,
        power_card.left + card_inset,
        power_card.top + scale(68, dpi),
        card_width(power_card) - 2 * card_inset,
        scale(15, dpi),
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

fn set_status_details_visible(status: &StatusWindow, visible: bool) {
    let show = if visible { SW_SHOW } else { SW_HIDE };
    let hide = if visible { SW_HIDE } else { SW_SHOW };
    let show_detail_control = |window: HWND| unsafe { ShowWindow(window, show) };
    let hide_detail_control = |window: HWND| unsafe { ShowWindow(window, hide) };
    let detail_controls = [
        status.labels[3],
        status.labels[4],
        status.labels[5],
        status.battery_value,
        status.sleep_value,
        status.low_power_value,
        status.assignment_heading,
        status.assignment_name,
        status.assignment_semantic,
        status.assignment_raw,
        status.assignment_hint,
    ];
    if visible {
        for control in detail_controls {
            let _ = show_detail_control(control);
        }
        for hotspot in status.mouse_hotspots {
            let _ = show_detail_control(hotspot);
        }
    } else {
        for control in detail_controls {
            let _ = hide_detail_control(control);
        }
        for hotspot in status.mouse_hotspots {
            let _ = hide_detail_control(hotspot);
        }
    }
    let _ = unsafe {
        SetWindowTextW(
            status.view_mode_button,
            PCWSTR(wide(if visible { "Quick switch" } else { "Details" }).as_ptr()),
        )
    };
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn high_contrast_is_on_or_unknown() -> bool {
    let mut settings = HIGHCONTRASTW {
        cbSize: size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            0,
            Some((&mut settings as *mut HIGHCONTRASTW).cast()),
            Default::default(),
        )
    }
    .map_or(true, |_| settings.dwFlags.contains(HCF_HIGHCONTRASTON))
}

fn layout_compact_status(window: HWND, status: &mut StatusWindow, dpi: u32) {
    set_status_details_visible(status, false);
    let place = |child: HWND, x: i32, y: i32, width: i32, height: i32| {
        let _ = unsafe {
            MoveWindow(
                child,
                scale(x, dpi),
                scale(y, dpi),
                scale(width, dpi),
                scale(height, dpi),
                true,
            )
        };
    };
    let _ = unsafe { SetWindowTextW(status.brand_title, PCWSTR(wide("ViperPilot").as_ptr())) };
    let _ = unsafe {
        SetWindowTextW(
            status.brand_subtitle,
            PCWSTR(wide("Viper V4 Pro  ·  Ctrl + Alt + P switches").as_ptr()),
        )
    };
    let _ = unsafe { SetWindowTextW(status.labels[0], PCWSTR(wide("CURRENT PROFILE").as_ptr())) };
    let _ = unsafe { SetWindowTextW(status.labels[1], PCWSTR(wide("DPI").as_ptr())) };
    let _ = unsafe { SetWindowTextW(status.labels[2], PCWSTR(wide("POLLING RATE").as_ptr())) };
    let _ = unsafe { SetWindowTextW(status.labels[6], PCWSTR(wide("DEVICE CONNECTION").as_ptr())) };
    let _ = unsafe { SetWindowTextW(status.labels[7], PCWSTR(wide("LATEST STATUS").as_ptr())) };
    let _ = unsafe {
        SetWindowTextW(
            status.assignment_heading,
            PCWSTR(wide("SELECTED QUICK-SWITCH PROFILES").as_ptr()),
        )
    };
    let _ = unsafe { ShowWindow(status.assignment_heading, SW_SHOW) };

    place(status.brand_title, 24, 15, 500, 30);
    place(status.brand_subtitle, 24, 48, 560, 20);
    place(status.view_mode_button, 780, 17, 96, 34);

    // The left column holds the actual Figma-derived 190×250 silhouette.
    place(status.labels[6], 24, 86, 240, 18);
    place(status.connection_value, 24, 108, 260, 28);

    // Device readbacks, selected local pair, and actions remain together in
    // the right column for quick access without opening the detailed view.
    place(status.labels[0], 318, 84, 550, 18);
    place(status.profile_value, 318, 105, 550, 30);
    place(status.labels[1], 318, 151, 265, 18);
    place(status.dpi_value, 318, 172, 265, 30);
    place(status.labels[2], 598, 151, 270, 18);
    place(status.polling_value, 598, 172, 270, 30);
    place(status.assignment_heading, 318, 221, 550, 20);
    place(status.developer_button, 318, 248, 265, 76);
    place(status.gaming_button, 598, 248, 270, 76);
    place(status.labels[7], 318, 347, 550, 18);
    place(status.verification_value, 318, 370, 550, 80);

    let client_width = scale(BASE_COMPACT_WIDTH, dpi);
    let client_height = scale(BASE_COMPACT_HEIGHT, dpi);
    let mut outer = RECT {
        left: 0,
        top: 0,
        right: client_width,
        bottom: client_height,
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
        unsafe { SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize) };
    }
    let runtime_ptr = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) as *mut WindowRuntime };
    if runtime_ptr.is_null() {
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    }
    let runtime = unsafe { &*runtime_ptr };

    // These messages are deliberately independent of TrayApp. DestroyWindow
    // sends them synchronously, including when called during teardown.
    match message {
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            return LRESULT(0);
        }
        WM_NCDESTROY => {
            let result = unsafe { DefWindowProcW(window, message, wparam, lparam) };
            unsafe { SetWindowLongPtrW(window, GWLP_USERDATA, 0) };
            return result;
        }
        WM_CLOSE if runtime.preview_mode => {
            unsafe { PostQuitMessage(0) };
            return LRESULT(0);
        }
        WM_CLOSE => {
            let _ = unsafe { ShowWindow(window, SW_HIDE) };
            return LRESULT(0);
        }
        _ => {}
    }

    if ignored_while_closing(runtime.closing.get(), runtime.show_message, message) {
        return LRESULT(0);
    }

    if runtime.show_message != 0 && message == runtime.show_message {
        runtime.defer(DeferredAction::ShowWindow);
        drain_deferred(runtime, window);
        return LRESULT(0);
    }

    match message {
        DEFERRED_DRAIN_EVENT => {
            drain_deferred(runtime, window);
            return LRESULT(0);
        }
        WORKER_EVENT if !runtime.preview_mode => runtime.defer(DeferredAction::WorkerEvent),
        WM_HOTKEY if !runtime.preview_mode && wparam.0 as i32 == HOTKEY_ID => {
            runtime.defer(DeferredAction::Hotkey)
        }
        WM_DEVICECHANGE if !runtime.preview_mode => runtime.defer(DeferredAction::Refresh),
        WM_COMMAND => {
            if let Some(command) = valid_window_command(window, wparam, lparam) {
                runtime.defer(DeferredAction::Command(command));
            }
        }
        TRAY_CALLBACK => match u32::from(crate::tray_logic::notification_event_code(lparam.0)) {
            WM_CONTEXTMENU | WM_RBUTTONUP if !runtime.preview_mode => {
                if !runtime.popup_pending.replace(true) {
                    runtime.defer(DeferredAction::TrayMenu);
                }
            }
            WM_LBUTTONUP | WM_LBUTTONDBLCLK if !runtime.preview_mode => {
                runtime.defer(DeferredAction::TrayOpen)
            }
            _ => {}
        },
        WM_PAINT => {
            #[allow(unused_mut)]
            if let Ok(mut app) = runtime.app.try_borrow_mut() {
                #[cfg(feature = "ui-preview")]
                if runtime.preview_mode {
                    unsafe { preview_window_proc(&mut app, window, message, wparam, lparam) };
                } else {
                    app.paint_dashboard(window);
                }
                #[cfg(not(feature = "ui-preview"))]
                app.paint_dashboard(window);
                drop(app);
                drain_deferred(runtime, window);
            } else {
                validate_deferred_paint(runtime, window);
            }
            return LRESULT(0);
        }
        WM_NOTIFY if runtime.preview_mode => {
            if lparam.0 == 0 {
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            }
            let header = unsafe { &*(lparam.0 as *const NMHDR) };
            if header.code != NM_CUSTOMDRAW {
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            }
            let result = match runtime.app.try_borrow() {
                Ok(app) if app.is_preview_button(header.hwndFrom, header.idFrom) => {
                    let draw = unsafe { &*(lparam.0 as *const NMCUSTOMDRAW) };
                    if draw.dwDrawStage != CDDS_PREPAINT || high_contrast_is_on_or_unknown() {
                        None
                    } else {
                        let state = draw.uItemState;
                        let mut button_state = 0;
                        if state.0 & CDIS_DISABLED.0 != 0 {
                            button_state |= ODS_DISABLED.0;
                        }
                        if state.0 & CDIS_SELECTED.0 != 0 {
                            button_state |= ODS_SELECTED.0;
                        }
                        if state.0 & CDIS_FOCUS.0 != 0 {
                            button_state |= ODS_FOCUS.0;
                        }
                        let drawn = app.draw_styled_button(
                            header.idFrom,
                            header.hwndFrom,
                            draw.hdc,
                            draw.rc,
                            button_state,
                            state.0 & CDIS_HOT.0 != 0,
                        );
                        drawn.then_some(LRESULT(CDRF_SKIPDEFAULT as isize))
                    }
                }
                Ok(_) => None,
                Err(_) => {
                    runtime.paint_pending.set(true);
                    None
                }
            };
            if let Some(result) = result {
                drain_deferred(runtime, window);
                return result;
            }
            return unsafe { DefWindowProcW(window, message, wparam, lparam) };
        }
        WM_DRAWITEM => {
            #[allow(unused_mut)]
            if let Ok(mut app) = runtime.app.try_borrow_mut() {
                #[cfg(feature = "ui-preview")]
                let handled = if runtime.preview_mode {
                    unsafe { preview_window_proc(&mut app, window, message, wparam, lparam) }.0 != 0
                } else {
                    let item = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
                    app.draw_owner_button(item)
                };
                #[cfg(not(feature = "ui-preview"))]
                let handled = {
                    let item = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
                    app.draw_owner_button(item)
                };
                drop(app);
                drain_deferred(runtime, window);
                if handled {
                    return LRESULT(1);
                }
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            } else {
                // DRAWITEMSTRUCT contains callback-lifetime pointers and must
                // never be retained. Let the current paint finish, then redraw.
                runtime.paint_pending.set(true);
                return LRESULT(1);
            }
        }
        WM_CTLCOLORSTATIC => {
            let control = HWND(lparam.0 as *mut c_void);
            let hdc = HDC(wparam.0 as *mut c_void);
            if let Ok(app) = runtime.app.try_borrow() {
                let (color, brush) = app.static_text_style(control);
                let _ = unsafe { SetTextColor(hdc, color) };
                let _ = unsafe { SetBkMode(hdc, TRANSPARENT) };
                let result = LRESULT(brush.0 as isize);
                drop(app);
                drain_deferred(runtime, window);
                return result;
            }
            let _ = unsafe { SetTextColor(hdc, CAT_TEXT) };
            let _ = unsafe { SetBkMode(hdc, TRANSPARENT) };
            runtime.paint_pending.set(true);
            return LRESULT(runtime.fallback_brush.0 as isize);
        }
        WM_DPICHANGED => {
            let suggested = unsafe { *(lparam.0 as *const RECT) };
            runtime.defer(DeferredAction::Dpi(suggested));
        }
        _ => return unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
    drain_deferred(runtime, window);
    LRESULT(0)
}

fn validate_deferred_paint(runtime: &WindowRuntime, window: HWND) {
    runtime.paint_pending.set(true);
    let mut paint = PAINTSTRUCT::default();
    let dc = unsafe { BeginPaint(window, &mut paint) };
    let _ = unsafe { EndPaint(window, &paint) };
    let _ = dc;
}

fn drain_deferred(runtime: &WindowRuntime, window: HWND) {
    if runtime.draining.replace(true) {
        return;
    }
    loop {
        if runtime.closing.get() {
            break;
        }
        let action = runtime.deferred.borrow_mut().pop_front();
        let Some(action) = action else { break };
        if !dispatch_deferred(runtime, window, action) {
            runtime.deferred.borrow_mut().push_front(action);
            let _ =
                unsafe { PostMessageW(Some(window), DEFERRED_DRAIN_EVENT, WPARAM(0), LPARAM(0)) };
            break;
        }
    }
    runtime.draining.set(false);
    if runtime.paint_pending.replace(false) && !runtime.closing.get() {
        // No RDW_UPDATENOW: invalidation schedules a later WM_PAINT after the
        // current app borrow and synchronous message stack have unwound.
        let _ = unsafe { RedrawWindow(Some(window), None, None, RDW_INVALIDATE | RDW_ALLCHILDREN) };
    }
}

fn dispatch_deferred(runtime: &WindowRuntime, window: HWND, action: DeferredAction) -> bool {
    match action {
        DeferredAction::TrayMenu => {
            if runtime.preview_mode {
                runtime.popup_pending.set(false);
                return true;
            }
            let menu = {
                let Ok(app) = runtime.app.try_borrow() else {
                    return false;
                };
                app.build_menu()
            };
            if let Some(menu) = menu {
                // TrackPopupMenu runs a modal loop. Its menu is a snapshot, and
                // no TrayApp borrow crosses that loop.
                let selected_command = track_popup_menu(window, menu);
                runtime.popup_pending.set(false);
                if selected_command != 0 {
                    // The returned ID refers to the menu snapshot. Apply it
                    // before worker events queued by TrackPopupMenu's modal loop.
                    runtime.defer_before_pending_events(DeferredAction::Command(selected_command));
                }
            } else {
                runtime.popup_pending.set(false);
            }
            true
        }
        _ => {
            let Ok(mut app) = runtime.app.try_borrow_mut() else {
                return false;
            };
            match action {
                DeferredAction::Command(command) => {
                    #[cfg(feature = "ui-preview")]
                    if runtime.preview_mode {
                        unsafe {
                            preview_window_proc(
                                &mut app,
                                window,
                                WM_COMMAND,
                                WPARAM(command),
                                LPARAM(0),
                            );
                        }
                    } else {
                        app.handle_menu_command(window, command);
                    }
                    #[cfg(not(feature = "ui-preview"))]
                    app.handle_menu_command(window, command);
                }
                DeferredAction::Hotkey => app.handle_hotkey(window),
                DeferredAction::WorkerEvent => app.drain_worker_events(window),
                DeferredAction::Refresh => {
                    if app.busy.is_none() {
                        app.request_refresh(window);
                    }
                }
                DeferredAction::TrayOpen | DeferredAction::ShowWindow => {
                    app.show_window_now(window);
                }
                DeferredAction::Dpi(suggested) => {
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
                    let detailed = app.detailed;
                    if let Some(status) = app.status.as_mut() {
                        layout_status(window, status, detailed);
                        #[cfg(feature = "ui-preview")]
                        if runtime.preview_mode {
                            set_text(
                                status.brand_subtitle,
                                "PREVIEW / NO DEVICE ACCESS — synthetic in-memory state",
                            );
                        }
                    }
                }
                DeferredAction::TrayMenu => unreachable!("handled before borrowing app"),
            }
            let should_quit = app.quit_requested;
            drop(app);
            if should_quit {
                runtime.closing.set(true);
                runtime.deferred.borrow_mut().clear();
                unsafe { PostQuitMessage(0) };
            }
            true
        }
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
        if !self.preview_mode {
            set_tooltip(
                window,
                &format!("Viper V4 Pro: {}", presentation.profile_line),
            );
        }
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
            set_text(
                status.developer_button,
                &self.profile_action_button_label(0),
            );
            set_text(status.gaming_button, &self.profile_action_button_label(1));
            let quick_pair_enabled = presentation.buttons_enabled
                && self.library.as_ref().is_some_and(|library| {
                    quick_switch_action(library, 0).is_ok()
                        && quick_switch_action(library, 1).is_ok()
                });
            let developer_enabled = if self.detailed {
                presentation.buttons_enabled
            } else {
                quick_pair_enabled
            };
            let gaming_enabled = developer_enabled;
            let _ = unsafe { EnableWindow(status.developer_button, developer_enabled) };
            let _ = unsafe { EnableWindow(status.gaming_button, gaming_enabled) };
            let _ = unsafe { InvalidateRect(Some(status.developer_button), None, true) };
            let _ = unsafe { InvalidateRect(Some(status.gaming_button), None, true) };
            for hotspot in status.mouse_hotspots {
                let _ = unsafe { InvalidateRect(Some(hotspot), None, true) };
            }
            let _ = unsafe { InvalidateRect(Some(window), None, true) };
        }
        self.presentation = Some(presentation);
    }

    /// Draws non-control dashboard chrome behind standard Windows buttons.
    /// The current-profile and selected-assignment text provide status separately
    /// from the buttons' native system appearance.
    fn paint_compact_switcher(&self, window: HWND) {
        let mut paint = PAINTSTRUCT::default();
        let hdc = unsafe { BeginPaint(window, &mut paint) };
        let mut client = RECT::default();
        let _ = unsafe { GetClientRect(window, &mut client) };
        let _ = unsafe { FillRect(hdc, &client, self.base_brush) };
        let dpi = unsafe { GetDpiForWindow(window) };
        let card = |left: i32, top: i32, right: i32, bottom: i32| RECT {
            left: scale(left, dpi),
            top: scale(top, dpi),
            right: scale(right, dpi),
            bottom: scale(bottom, dpi),
        };
        draw_panel(
            hdc,
            card(18, 78, 290, 480),
            CAT_SURFACE0,
            CAT_BORDER,
            scale(14, dpi),
        );
        draw_panel(
            hdc,
            card(308, 77, 882, 140),
            CAT_SURFACE0,
            CAT_BORDER,
            scale(12, dpi),
        );
        draw_panel(
            hdc,
            card(308, 145, 588, 210),
            CAT_SURFACE0,
            CAT_BORDER,
            scale(12, dpi),
        );
        draw_panel(
            hdc,
            card(588, 145, 882, 210),
            CAT_SURFACE0,
            CAT_BORDER,
            scale(12, dpi),
        );
        draw_panel(
            hdc,
            card(308, 216, 882, 332),
            CAT_SURFACE0,
            CAT_BORDER,
            scale(12, dpi),
        );
        draw_panel(
            hdc,
            card(308, 337, 882, 456),
            CAT_SURFACE0,
            CAT_BORDER,
            scale(12, dpi),
        );
        let mouse = card(66, 162, 256, 412);
        let _ = self.draw_bitmap(hdc, mouse, self.compact_mouse_bitmap.as_ref());
        if let Some(status) = self.status.as_ref() {
            let caption = card(38, 105, 270, 128);
            draw_centered_text(
                hdc,
                caption,
                "VIPERPILOT  ·  V4 PRO",
                status.body_font,
                CAT_SUBTEXT0,
            );
        }
        let _ = unsafe { EndPaint(window, &paint) };
    }

    fn paint_dashboard(&self, window: HWND) {
        if !self.detailed {
            self.paint_compact_switcher(window);
            return;
        }
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
        draw_panel(hdc, left_panel, CAT_SURFACE0, CAT_BORDER, scale(12, dpi));
        for index in 0..4 {
            draw_panel(
                hdc,
                status_card_bounds(index, dpi),
                CAT_SURFACE0,
                CAT_BORDER,
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
            CAT_BORDER,
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
            CAT_BORDER,
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
        self.draw_bitmap(hdc, destination, self.mouse_bitmap.as_ref())
    }

    fn draw_bitmap(&self, hdc: HDC, destination: RECT, bitmap: Option<&DashboardBitmap>) -> bool {
        let Some(bitmap) = bitmap else {
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
            || control == status.view_mode_button
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

    fn is_preview_button(&self, control: HWND, id: usize) -> bool {
        let Some(status) = self.status.as_ref() else {
            return false;
        };
        match id {
            BUTTON_DEVELOPER => status.developer_button == control,
            BUTTON_GAMING => status.gaming_button == control,
            BUTTON_VIEW_MODE => status.view_mode_button == control,
            HOTSPOT_LEFT..=HOTSPOT_DPI => {
                status.mouse_hotspots.contains(&control) && mouse_control_for_hotspot(id).is_some()
            }
            _ => false,
        }
    }

    fn draw_owner_button(&self, item: &DRAWITEMSTRUCT) -> bool {
        self.draw_styled_button(
            item.CtlID as usize,
            item.hwndItem,
            item.hDC,
            item.rcItem,
            item.itemState.0,
            false,
        )
    }

    fn draw_styled_button(
        &self,
        control_id: usize,
        control: HWND,
        hdc: HDC,
        rect: RECT,
        state: u32,
        hot: bool,
    ) -> bool {
        let quick_label = if !self.detailed {
            let slot = match control_id {
                BUTTON_DEVELOPER => Some(0),
                BUTTON_GAMING => Some(1),
                _ => None,
            };
            slot.map(|slot| self.profile_action_button_label(slot))
        } else {
            None
        };
        let (fixed_label, active, corner_radius, is_hotspot) = match control_id {
            BUTTON_DEVELOPER => (
                if self.detailed {
                    "Apply Developer recovery preset"
                } else {
                    "Saved profile unavailable"
                },
                if self.detailed {
                    self.current == ProfileMatch::Developer
                } else {
                    self.quick_slot_is_current(0)
                },
                12,
                false,
            ),
            BUTTON_GAMING => (
                if self.detailed {
                    "Apply Gaming recovery preset"
                } else {
                    "Saved profile unavailable"
                },
                if self.detailed {
                    self.current == ProfileMatch::Gaming
                } else {
                    self.quick_slot_is_current(1)
                },
                12,
                false,
            ),
            BUTTON_VIEW_MODE => (
                if self.detailed {
                    "Quick switch"
                } else {
                    "Details"
                },
                false,
                8,
                false,
            ),
            hotspot => {
                let Some(control) = mouse_control_for_hotspot(hotspot) else {
                    return false;
                };
                (
                    control.hotspot_label(),
                    self.selected_mouse_control == control,
                    8,
                    true,
                )
            }
        };
        let label = quick_label.as_deref().unwrap_or(fixed_label);
        let disabled = state & ODS_DISABLED.0 != 0;
        let selected = state & ODS_SELECTED.0 != 0;
        let focused = state & ODS_FOCUS.0 != 0;
        let (fill, border, text) = if disabled {
            (CAT_SURFACE0, CAT_BORDER, CAT_OVERLAY0)
        } else if selected {
            (VIPER_ACTIVE, VIPER_ROSE, CAT_TEXT)
        } else if hot {
            (CAT_SURFACE1, VIPER_ROSE, CAT_TEXT)
        } else if active && is_hotspot {
            (CAT_CRUST, VIPER_ROSE, VIPER_ROSE)
        } else if active {
            (VIPER_ACTIVE, VIPER_ROSE, CAT_TEXT)
        } else {
            (CAT_SURFACE0, VIPER_ROSE, CAT_TEXT)
        };
        let dpi = unsafe { GetDpiForWindow(control) };
        let font = self
            .status
            .as_ref()
            .map_or(HFONT::default(), |status| status.button_font);
        let parent_background =
            if !self.detailed && matches!(control_id, BUTTON_DEVELOPER | BUTTON_GAMING) {
                self.surface_brush
            } else {
                self.base_brush
            };
        draw_button_gdi(
            hdc,
            rect,
            label,
            font,
            parent_background,
            ButtonPaintStyle {
                fill,
                border,
                text_color: text,
                corner_radius: scale(corner_radius, dpi),
                pen_width: scale(if active && is_hotspot { 3 } else { 1 }, dpi),
                is_hotspot,
                focused,
                multiline: !self.detailed && matches!(control_id, BUTTON_DEVELOPER | BUTTON_GAMING),
                text_inset: if !self.detailed
                    && matches!(control_id, BUTTON_DEVELOPER | BUTTON_GAMING)
                {
                    scale(10, dpi)
                } else {
                    0
                },
            },
        )
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
        // Keep the library snapshot fixed while the native popup is open. Its
        // command IDs and labels must continue to refer to what the user saw.
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
                    self.initializing = false;
                    self.busy = None;
                    self.connection = ConnectionStatus::Connected;
                    self.polling_hz = polling_hz;
                    self.dpi = dpi;
                    self.power = power;
                    self.button_assignments = Some(button_assignments);
                    self.last_error = None;
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
                        self.quit_requested = true;
                    }
                }
                Ok(WorkerEvent::ApplyFailed {
                    profile,
                    detail,
                    report_path,
                    final_connection,
                    final_profile,
                    polling_hz,
                    dpi,
                    power,
                    button_assignments,
                }) => {
                    self.busy = None;
                    let state_is_usable = final_connection == ConnectionStatus::Connected;
                    self.current = if state_is_usable {
                        final_profile.unwrap_or(ProfileMatch::OutOfSync)
                    } else {
                        ProfileMatch::OutOfSync
                    };
                    self.connection = final_connection;
                    if state_is_usable {
                        self.polling_hz = polling_hz;
                        self.dpi = dpi;
                        self.power = power;
                        self.button_assignments = button_assignments;
                    } else {
                        self.polling_hz = None;
                        self.dpi = None;
                        self.power = None;
                        self.button_assignments = None;
                    }
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
                Ok(WorkerEvent::LibraryUpdated(library)) => {
                    self.library = Some(library);
                    if !self.initializing {
                        self.busy = None;
                    }
                    self.update_ui(window);
                }
                Ok(WorkerEvent::LibraryError(error)) => {
                    self.library = None;
                    if !self.initializing {
                        self.busy = None;
                    }
                    self.update_ui(window);
                    notify(window, "Saved profiles unavailable", &error, true);
                }
                Ok(WorkerEvent::LocalError(error)) => {
                    self.busy = None;
                    self.update_ui(window);
                    notify(window, "Quick-switch preference not saved", &error, true);
                }
                Ok(WorkerEvent::Error(error)) => {
                    self.initializing = false;
                    self.busy = None;
                    self.current = ProfileMatch::OutOfSync;
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
            if self.quit_requested {
                break;
            }
        }
    }

    fn handle_hotkey(&mut self, window: HWND) {
        if self.connection != ConnectionStatus::Connected {
            return;
        }
        // The worker rereads the mouse and library before resolving the pair.
        // This UI-side busy gate only deduplicates repeated shortcut input.
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(BusyKind::Changing);
        if self
            .command_tx
            .send(WorkerCommand::ToggleQuickSwitch)
            .is_err()
        {
            self.busy = None;
        }
        self.update_ui(window);
    }

    fn handle_menu_command(&mut self, window: HWND, command: usize) {
        if let Some(control) = mouse_control_for_hotspot(command) {
            // The map is a read-only inspector. It deliberately does not use
            // the worker, create a plan, or send any HID command.
            self.selected_mouse_control = control;
            self.update_ui(window);
            return;
        }
        if let Some(slot) = quick_switch_slot_for_command(command) {
            self.apply_quick_switch_slot(window, slot);
            return;
        }
        if !self.detailed && command == BUTTON_DEVELOPER {
            self.apply_quick_switch_slot(window, 0);
            return;
        }
        if !self.detailed && command == BUTTON_GAMING {
            self.apply_quick_switch_slot(window, 1);
            return;
        }
        if (MENU_SAVED_APPLY_BASE..MENU_SAVED_APPLY_BASE + MAX_SAVED_PROFILES).contains(&command) {
            let index = command - MENU_SAVED_APPLY_BASE;
            if let Some((id, expected_preset)) = self.library.as_ref().and_then(|library| {
                library
                    .entries()
                    .get(index)
                    .map(|entry| (entry.id().to_owned(), entry.source_preset()))
            }) {
                self.request_saved_apply(window, id, expected_preset);
            }
            return;
        }
        if (MENU_QUICK_FIRST_BASE..MENU_QUICK_SECOND_BASE + MAX_SAVED_PROFILES).contains(&command) {
            let (slot, base) = if command < MENU_QUICK_SECOND_BASE {
                (0, MENU_QUICK_FIRST_BASE)
            } else {
                (1, MENU_QUICK_SECOND_BASE)
            };
            let index = command - base;
            if let Some(id) = self.library.as_ref().and_then(|library| {
                library
                    .entries()
                    .get(index)
                    .map(|entry| entry.id().to_owned())
            }) {
                self.request_config(
                    window,
                    WorkerCommand::SetQuickSwitch {
                        slot,
                        profile_id: id,
                    },
                );
            }
            return;
        }
        match command {
            MENU_OPEN => self.show_window_now(window),
            BUTTON_VIEW_MODE => {
                self.detailed = !self.detailed;
                if let Some(status) = self.status.as_mut() {
                    layout_status(window, status, self.detailed);
                    let focus = if self.detailed {
                        status.view_mode_button
                    } else {
                        status.developer_button
                    };
                    let _ = unsafe { SetFocus(Some(focus)) };
                }
                self.update_ui(window);
                let _ = unsafe { InvalidateRect(Some(window), None, true) };
            }
            MENU_RELOAD_LIBRARY if self.busy.is_none() => {
                self.request_config(window, WorkerCommand::LoadLibrary);
            }
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
                self.quit_requested = true;
            }
            _ => {}
        }
    }

    fn profile_action_button_label(&self, slot: usize) -> String {
        if self.detailed {
            return match slot {
                0 => "Apply Developer recovery preset".to_owned(),
                1 => "Apply Gaming recovery preset".to_owned(),
                _ => "Profile action unavailable".to_owned(),
            };
        }
        self.library
            .as_ref()
            .and_then(|library| quick_switch_action(library, slot).ok())
            .map_or_else(
                || "Saved profile unavailable".to_owned(),
                |action| action.label,
            )
    }

    fn quick_slot_is_current(&self, slot: usize) -> bool {
        let Some(action) = self
            .library
            .as_ref()
            .and_then(|library| quick_switch_action(library, slot).ok())
        else {
            return false;
        };
        matches!(
            (self.current, action.expected_preset),
            (ProfileMatch::Developer, ProfileName::Developer)
                | (ProfileMatch::Gaming, ProfileName::Gaming)
        )
    }

    fn apply_quick_switch_slot(&mut self, window: HWND, slot: usize) {
        let action = self
            .library
            .as_ref()
            .and_then(|library| quick_switch_action(library, slot).ok());
        if let Some(action) = action {
            self.request_saved_apply(window, action.profile_id, action.expected_preset);
        } else {
            notify(
                window,
                "Quick-switch pair unavailable",
                "Reload saved profiles before switching.",
                true,
            );
        }
    }

    fn request_saved_apply(
        &mut self,
        window: HWND,
        profile_id: String,
        expected_preset: ProfileName,
    ) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(BusyKind::Changing);
        if self
            .command_tx
            .send(WorkerCommand::ApplySaved {
                id: profile_id,
                expected_preset,
            })
            .is_err()
        {
            self.busy = None;
        }
        self.update_ui(window);
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

    fn build_menu(&self) -> Option<HMENU> {
        let menu = unsafe { CreatePopupMenu() }.ok()?;
        // Keep the popup in the native system theme. A dark MIM_BACKGROUND
        // without matching owner-drawn text can make the tray menu unreadable.
        let disabled = if self.busy.is_some() {
            MF_GRAYED
        } else {
            MF_STRING
        };
        append_menu(menu, MF_STRING, MENU_OPEN, "Open dashboard");
        append_menu(menu, disabled, MENU_RELOAD_LIBRARY, "Reload saved profiles");
        if let Some(library) = self.library.as_ref() {
            for (slot, command) in [MENU_QUICK_APPLY_FIRST, MENU_QUICK_APPLY_SECOND]
                .into_iter()
                .enumerate()
            {
                match quick_switch_action(library, slot) {
                    Ok(action) => append_menu(menu, disabled, command, &action.label),
                    Err(_) => append_menu(
                        menu,
                        MF_GRAYED,
                        command,
                        if slot == 0 {
                            "Quick switch first profile (unavailable)"
                        } else {
                            "Quick switch second profile (unavailable)"
                        },
                    ),
                }
            }
        } else {
            append_menu(
                menu,
                MF_GRAYED,
                MENU_QUICK_APPLY_FIRST,
                "Quick switch first profile (unavailable)",
            );
            append_menu(
                menu,
                MF_GRAYED,
                MENU_QUICK_APPLY_SECOND,
                "Quick switch second profile (unavailable)",
            );
        }
        append_menu(menu, MF_GRAYED, 0, "Apply complete built-in preset");
        append_menu(
            menu,
            disabled,
            MENU_DEVELOPER,
            "Developer (recovery preset)",
        );
        append_menu(menu, disabled, MENU_GAMING, "Gaming (recovery preset)");
        append_menu(
            menu,
            MF_GRAYED,
            0,
            "Saved entries are local names for complete presets",
        );

        if let Some(library) = self.library.as_ref() {
            if let Ok(saved_menu) = unsafe { CreatePopupMenu() } {
                for (index, entry) in library.entries().iter().enumerate() {
                    let id = MENU_SAVED_APPLY_BASE + index;
                    let label = format!(
                        "{} — complete {} preset",
                        entry.name(),
                        entry.source_preset()
                    );
                    append_menu(saved_menu, disabled, id, &label);
                }
                append_popup(menu, saved_menu, "Apply saved local profile");

                if let Ok(pair_menu) = unsafe { CreatePopupMenu() } {
                    append_quick_slot_menu(pair_menu, library, 0, MENU_QUICK_FIRST_BASE, disabled);
                    append_quick_slot_menu(pair_menu, library, 1, MENU_QUICK_SECOND_BASE, disabled);
                    let first = library.quick_switch()[0].as_str();
                    let second = library.quick_switch()[1].as_str();
                    let first_name = library
                        .entries()
                        .iter()
                        .find(|entry| entry.id() == first)
                        .map_or("Unknown", |entry| entry.name());
                    let second_name = library
                        .entries()
                        .iter()
                        .find(|entry| entry.id() == second)
                        .map_or("Unknown", |entry| entry.name());
                    append_menu(
                        pair_menu,
                        MF_GRAYED,
                        0,
                        &format!("Current local pair: {first_name} ↔ {second_name}"),
                    );
                    append_popup(
                        menu,
                        pair_menu,
                        "Choose quick-switch pair (Ctrl+Alt+P; selection does not apply)",
                    );
                }
            }
        } else {
            append_menu(menu, MF_GRAYED, 0, "Saved profiles unavailable");
        }

        let profile_line = self
            .presentation
            .as_ref()
            .map_or_else(|| "Reading…".to_owned(), |p| p.profile_line.clone());
        append_menu(
            menu,
            MF_GRAYED,
            0,
            &format!("Observed device: {profile_line}"),
        );
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

        Some(menu)
    }
}

fn track_popup_menu(window: HWND, menu: HMENU) -> usize {
    let mut selected_command = 0;
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_ok() {
        let _ = unsafe { SetForegroundWindow(window) };
        selected_command = unsafe {
            TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_RETURNCMD,
                point.x,
                point.y,
                Some(0),
                window,
                None,
            )
        }
        .0 as usize;
    }
    let _ = unsafe { DestroyMenu(menu) };
    selected_command
}

fn append_popup(
    menu: windows::Win32::UI::WindowsAndMessaging::HMENU,
    submenu: windows::Win32::UI::WindowsAndMessaging::HMENU,
    text: &str,
) {
    let mut wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let _ = unsafe {
        AppendMenuW(
            menu,
            MF_POPUP,
            submenu.0 as usize,
            PCWSTR(wide.as_mut_ptr()),
        )
    };
}

fn append_quick_slot_menu(
    parent: windows::Win32::UI::WindowsAndMessaging::HMENU,
    library: &ProfileLibraryV1,
    slot: usize,
    command_base: usize,
    disabled: windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS,
) {
    let Ok(submenu) = (unsafe { CreatePopupMenu() }) else {
        return;
    };
    let selected = &library.quick_switch()[slot];
    for (index, entry) in library.entries().iter().enumerate() {
        let marker = if entry.id() == selected { "✓ " } else { "" };
        let label = format!(
            "{marker}{} — saved local alias · {} preset",
            entry.name(),
            entry.source_preset()
        );
        let entry_flags = if quick_slot_entry_eligible(library, slot, entry.id()) {
            disabled
        } else {
            MF_GRAYED
        };
        append_menu(submenu, entry_flags, command_base + index, &label);
    }
    let selected_name = library
        .entries()
        .iter()
        .find(|entry| entry.id() == selected)
        .map_or("Unavailable", |entry| entry.name());
    let label = if slot == 0 {
        "First target"
    } else {
        "Second target"
    };
    append_popup(parent, submenu, &format!("{label}: {selected_name}"));
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

struct ButtonPaintStyle {
    fill: COLORREF,
    border: COLORREF,
    text_color: COLORREF,
    corner_radius: i32,
    pen_width: i32,
    is_hotspot: bool,
    focused: bool,
    multiline: bool,
    text_inset: i32,
}

fn draw_button_gdi(
    hdc: HDC,
    rect: RECT,
    label: &str,
    font: HFONT,
    background: HBRUSH,
    style: ButtonPaintStyle,
) -> bool {
    let saved_dc = unsafe { SaveDC(hdc) };
    if saved_dc == 0 {
        return false;
    }

    let brush = unsafe { CreateSolidBrush(style.fill) };
    if brush.0.is_null() {
        let _ = unsafe { RestoreDC(hdc, saved_dc) };
        return false;
    }
    let pen = unsafe { CreatePen(PS_SOLID, style.pen_width, style.border) };
    if pen.0.is_null() {
        let _ = unsafe { RestoreDC(hdc, saved_dc) };
        let _ = unsafe { DeleteObject(HGDIOBJ(brush.0)) };
        return false;
    }

    let mut old_brush = None;
    let mut old_pen = None;
    let mut old_font = None;
    let mut old_text_color = None;
    let mut old_background_mode = None;
    let mut draw_ok = true;

    if unsafe { FillRect(hdc, &rect, background) } == 0 {
        draw_ok = false;
    }
    if draw_ok {
        let previous = unsafe { SelectObject(hdc, HGDIOBJ(brush.0)) };
        if gdi_selection_failed(previous) {
            draw_ok = false;
        } else {
            old_brush = Some(previous);
        }
    }
    if draw_ok {
        let previous = unsafe { SelectObject(hdc, HGDIOBJ(pen.0)) };
        if gdi_selection_failed(previous) {
            draw_ok = false;
        } else {
            old_pen = Some(previous);
        }
    }
    if draw_ok && !font.0.is_null() {
        let previous = unsafe { SelectObject(hdc, HGDIOBJ(font.0)) };
        if gdi_selection_failed(previous) {
            draw_ok = false;
        } else {
            old_font = Some(previous);
        }
    }
    if draw_ok {
        let previous = unsafe { SetTextColor(hdc, style.text_color) };
        if previous.0 == CLR_INVALID {
            draw_ok = false;
        } else {
            old_text_color = Some(previous);
        }
    }
    if draw_ok {
        let previous = unsafe { SetBkMode(hdc, TRANSPARENT) };
        if previous == 0 {
            draw_ok = false;
        } else {
            old_background_mode = Some(previous);
        }
    }
    if draw_ok {
        let shape_drawn = if style.is_hotspot {
            unsafe { Ellipse(hdc, rect.left, rect.top, rect.right, rect.bottom) }.as_bool()
        } else {
            unsafe {
                RoundRect(
                    hdc,
                    rect.left,
                    rect.top,
                    rect.right,
                    rect.bottom,
                    style.corner_radius,
                    style.corner_radius,
                )
            }
            .as_bool()
        };
        draw_ok = shape_drawn;
    }
    if draw_ok {
        let mut wide: Vec<u16> = label.encode_utf16().collect();
        let format = if style.multiline {
            DT_CENTER | windows::Win32::Graphics::Gdi::DT_WORDBREAK
        } else {
            DT_CENTER | DT_VCENTER | DT_SINGLELINE
        };
        if style.multiline {
            let content = RECT {
                left: rect.left + style.text_inset,
                top: rect.top + style.text_inset,
                right: rect.right - style.text_inset,
                bottom: rect.bottom - style.text_inset,
            };
            if content.right <= content.left || content.bottom <= content.top {
                draw_ok = false;
            } else {
                let mut measured = content;
                let required =
                    unsafe { DrawTextW(hdc, &mut wide, &mut measured, format | DT_CALCRECT) };
                let required_height = measured.bottom - measured.top;
                let content_height = content.bottom - content.top;
                if required <= 0 || required_height <= 0 || required_height > content_height {
                    draw_ok = false;
                } else {
                    let top = content.top + (content_height - required_height) / 2;
                    let mut text_rect = RECT {
                        top,
                        bottom: top + required_height,
                        ..content
                    };
                    draw_ok = unsafe { DrawTextW(hdc, &mut wide, &mut text_rect, format) } > 0;
                }
            }
        } else {
            let mut text_rect = rect;
            draw_ok = unsafe { DrawTextW(hdc, &mut wide, &mut text_rect, format) } > 0;
        }
    }
    if draw_ok && style.focused {
        let mut focus = rect;
        focus.left += 4;
        focus.top += 4;
        focus.right -= 4;
        focus.bottom -= 4;
        draw_ok = unsafe { DrawFocusRect(hdc, &focus) }.as_bool();
    }

    let dc_restored = unsafe { RestoreDC(hdc, saved_dc) }.as_bool();
    if !dc_restored {
        if let Some(previous) = old_font {
            draw_ok &= !gdi_selection_failed(unsafe { SelectObject(hdc, previous) });
        }
        if let Some(previous) = old_pen {
            draw_ok &= !gdi_selection_failed(unsafe { SelectObject(hdc, previous) });
        }
        if let Some(previous) = old_brush {
            draw_ok &= !gdi_selection_failed(unsafe { SelectObject(hdc, previous) });
        }
        if let Some(previous) = old_text_color {
            draw_ok &= unsafe { SetTextColor(hdc, previous) }.0 != CLR_INVALID;
        }
        if let Some(previous) = old_background_mode {
            draw_ok &= unsafe {
                SetBkMode(
                    hdc,
                    windows::Win32::Graphics::Gdi::BACKGROUND_MODE(previous as u32),
                )
            } != 0;
        }
    }

    let pen_deleted = delete_created_gdi_object(hdc, HGDIOBJ(pen.0), OBJ_PEN, dc_restored);
    let brush_deleted = delete_created_gdi_object(hdc, HGDIOBJ(brush.0), OBJ_BRUSH, dc_restored);
    draw_ok && dc_restored && pen_deleted && brush_deleted
}

fn gdi_selection_failed(object: HGDIOBJ) -> bool {
    object.0.is_null() || object.0 as isize == -1
}

fn delete_created_gdi_object(
    hdc: HDC,
    object: HGDIOBJ,
    object_type: windows::Win32::Graphics::Gdi::OBJ_TYPE,
    dc_restored: bool,
) -> bool {
    if !dc_restored {
        let selected = unsafe { GetCurrentObject(hdc, object_type) };
        if gdi_selection_failed(selected) || selected == object {
            return false;
        }
    }
    unsafe { DeleteObject(object) }.as_bool()
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

    let divider = unsafe { CreatePen(PS_SOLID, scale(1, dpi), CAT_BORDER) };
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
        Tone::Developer => VIPER_ROSE,
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
fn load_mouse_bitmap(file_name: &str) -> Option<DashboardBitmap> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.push(directory.join(file_name));
        candidates.push(directory.join("assets").join(file_name));
    }
    if let Ok(directory) = std::env::current_dir() {
        candidates.push(directory.join("assets").join(file_name));
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
    if matches!(&command, WorkerCommand::LoadLibrary) {
        return Ok(match load_profile_library(&paths) {
            Ok(library) => WorkerEvent::LibraryUpdated(library),
            Err(error) => WorkerEvent::LibraryError(error.to_string()),
        });
    }
    let mut config = load_config(&paths).map_err(|error| error.to_string())?;
    let diagnostic_event = match &command {
        WorkerCommand::LoadLibrary => "load saved profiles",
        WorkerCommand::Refresh => "refresh",
        WorkerCommand::Apply(ProfileName::Developer) => "apply developer",
        WorkerCommand::Apply(ProfileName::Gaming) => "apply gaming",
        WorkerCommand::ApplySaved { .. } => "apply saved local profile",
        WorkerCommand::SetQuickSwitch { .. } => "change quick-switch pair",
        WorkerCommand::ToggleQuickSwitch => "toggle quick-switch pair",
        WorkerCommand::SetStartWithWindows(_) => "change startup option",
        WorkerCommand::SetDiagnostics(_) => "change diagnostics option",
        WorkerCommand::SetExitAfterGaming(_) => "change exit-after-gaming option",
    };
    if config.diagnostics_enabled {
        append_diagnostic(&paths, &format!("{diagnostic_event} started"))
            .map_err(|error| error.to_string())?;
    }
    let result = match command {
        WorkerCommand::LoadLibrary => unreachable!("handled before loading config"),
        WorkerCommand::Refresh => refresh_state(&paths, &mut config),
        WorkerCommand::Apply(profile) => apply_profile(&paths, &mut config, profile),
        WorkerCommand::ApplySaved {
            id,
            expected_preset,
        } => {
            let library = match load_profile_library(&paths) {
                Ok(library) => library,
                Err(error) => return Ok(WorkerEvent::LibraryError(error.to_string())),
            };
            let profile = match resolve_saved_apply(&library, &id, expected_preset) {
                Ok(profile) => profile,
                Err(error) => return Ok(WorkerEvent::LibraryError(error)),
            };
            apply_profile(&paths, &mut config, profile)
        }
        WorkerCommand::SetQuickSwitch { slot, profile_id } => {
            if slot > 1 {
                return Ok(WorkerEvent::LocalError(
                    "quick-switch pair slot must be 0 or 1".to_owned(),
                ));
            }
            let mut library = match load_profile_library(&paths) {
                Ok(library) => library,
                Err(error) => return Ok(WorkerEvent::LibraryError(error.to_string())),
            };
            let mut pair = library.quick_switch().clone();
            pair[slot] = profile_id;
            if let Err(error) = library.set_quick_switch(&pair[0], &pair[1]) {
                return Ok(WorkerEvent::LocalError(error));
            }
            if let Err(error) = save_profile_library(&paths, &library) {
                return Ok(WorkerEvent::LocalError(error.to_string()));
            }
            Ok(WorkerEvent::LibraryUpdated(library))
        }
        WorkerCommand::ToggleQuickSwitch => {
            let library = match load_profile_library(&paths) {
                Ok(library) => library,
                Err(error) => return Ok(WorkerEvent::LibraryError(error.to_string())),
            };
            toggle_saved_profile(&paths, &mut config, &library)
        }
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

fn toggle_saved_profile(
    paths: &StoragePaths,
    config: &mut UtilityConfigV1,
    library: &ProfileLibraryV1,
) -> Result<WorkerEvent, String> {
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let before = crate::engine::DeviceControl::read_snapshot(&mut device)?;
    let baseline = read_snapshot(&paths.baseline_path(&before.device.serial)).map_err(|error| {
        format!("immutable baseline is required before quick switching: {error}")
    })?;
    let observed = match classify_profile(&before, &baseline)? {
        ProfileMatch::Developer => Some(ProfileName::Developer),
        ProfileMatch::Gaming => Some(ProfileName::Gaming),
        ProfileMatch::OutOfSync => None,
    };
    let target = library
        .toggle_target(observed)
        .map_err(|error| error.to_string())?;
    // Reuse this device, observed snapshot, and baseline for both selection
    // and planning; the write engine retains its own preflight and readback gates.
    apply_profile_from_snapshot(
        paths,
        config,
        &mut device,
        before,
        baseline,
        target.source_preset(),
    )
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
    apply_profile_from_snapshot(paths, config, &mut device, before, baseline, profile)
}

fn apply_profile_from_snapshot<D: crate::engine::DeviceControl>(
    paths: &StoragePaths,
    config: &mut UtilityConfigV1,
    device: &mut D,
    before: crate::model::DeviceSnapshotV1,
    baseline: crate::model::DeviceSnapshotV1,
    profile: ProfileName,
) -> Result<WorkerEvent, String> {
    let plan = plan_profile_with_baseline(&before, &baseline, profile)
        .map_err(|error| error.to_string())?;
    // This gate is deliberately before both the journal and apply engine: an
    // unproven polling origin/target causes zero setters and zero write report.
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    write_plan_journal(paths, &plan).map_err(|error| error.to_string())?;
    let no_changes_needed = plan.is_noop();
    let report = apply_write_plan(device, &plan);
    let report_path = write_verification_report(paths, &report, &before.device.serial)
        .map_err(|error| error.to_string())?;
    let final_dpi = report.final_state.as_ref().map(|state| state.dpi.current);
    let final_power = report.final_state.as_ref().and_then(|state| state.power);
    let final_button_assignments = report
        .final_state
        .as_ref()
        .map(|state| state.button_assignments.clone());
    if !report.success {
        let final_profile_result = report
            .final_state
            .as_ref()
            .map(|state| classify_profile(state, &baseline));
        let final_profile = final_profile_result
            .as_ref()
            .and_then(|result| result.as_ref().ok().copied());
        let final_connection = gui_logic::failure_connection_status(
            report.final_state.is_some(),
            final_profile.is_some(),
        );
        let polling_hz = report
            .final_state
            .as_ref()
            .and_then(|state| state.polling.hertz());
        return Ok(WorkerEvent::ApplyFailed {
            profile,
            detail: gui_logic::failure_detail_for_final_state(
                report
                    .rollback_result
                    .as_ref()
                    .map(|rollback| rollback.final_state_restored),
                final_profile,
                report.final_state.is_some(),
            ),
            report_path: Some(report_path.display().to_string()),
            final_connection,
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

#[cfg(test)]
mod quick_switch_tray_tests {
    use super::*;

    #[test]
    fn button_renderer_falls_back_for_an_invalid_device_context() {
        assert!(!draw_button_gdi(
            HDC::default(),
            RECT::default(),
            "Preview button",
            HFONT::default(),
            HBRUSH::default(),
            ButtonPaintStyle {
                fill: CAT_SURFACE0,
                border: CAT_BORDER,
                text_color: CAT_TEXT,
                corner_radius: 8,
                pen_width: 1,
                is_hotspot: false,
                focused: false,
                multiline: false,
                text_inset: 0,
            },
        ));
    }

    #[test]
    fn numbered_mouse_hotspots_stay_separate_and_inside_the_image_at_common_dpis() {
        for dpi in [96, 144, 192] {
            let image = mouse_image_bounds(dpi);
            let hotspots: Vec<RECT> = gui_logic::MouseControl::ALL
                .into_iter()
                .map(|control| {
                    let (left, top, width, height) = mouse_hotspot_bounds(control);
                    RECT {
                        left: scale(left, dpi),
                        top: scale(top, dpi),
                        right: scale(left + width, dpi),
                        bottom: scale(top + height, dpi),
                    }
                })
                .collect();

            for (index, hotspot) in hotspots.iter().enumerate() {
                assert!(hotspot.left >= image.left, "left of image at {dpi} DPI");
                assert!(hotspot.top >= image.top, "above image at {dpi} DPI");
                assert!(hotspot.right <= image.right, "right of image at {dpi} DPI");
                assert!(hotspot.bottom <= image.bottom, "below image at {dpi} DPI");
                for other in &hotspots[index + 1..] {
                    let overlaps = hotspot.left < other.right
                        && other.left < hotspot.right
                        && hotspot.top < other.bottom
                        && other.top < hotspot.bottom;
                    assert!(!overlaps, "mouse hotspots overlap at {dpi} DPI");
                }
            }
        }
    }

    #[test]
    fn direct_actions_resolve_pair_aliases_and_explain_the_preset() {
        let library = ProfileLibraryV1::default();
        let first = quick_switch_action(&library, 0).unwrap();
        let second = quick_switch_action(&library, 1).unwrap();

        assert_eq!(first.profile_id, "developer");
        assert_eq!(first.expected_preset, ProfileName::Developer);
        assert_eq!(
            first.label,
            "Switch to Developer — complete Developer preset"
        );
        assert_eq!(second.profile_id, "gaming");
        assert_eq!(second.expected_preset, ProfileName::Gaming);
        assert_eq!(second.label, "Switch to Gaming — complete Gaming preset");
        assert_eq!(
            quick_switch_slot_for_command(MENU_QUICK_APPLY_FIRST),
            Some(0)
        );
        assert_eq!(
            quick_switch_slot_for_command(MENU_QUICK_APPLY_SECOND),
            Some(1)
        );
        assert_eq!(
            quick_switch_slot_for_command(MENU_QUICK_APPLY_FIRST + 2),
            None
        );
    }

    #[test]
    fn saved_apply_requires_the_menu_displayed_preset_to_match() {
        let mut displayed = ProfileLibraryV1::default();
        displayed.duplicate("developer", "work", "Work").unwrap();
        assert_eq!(
            resolve_saved_apply(&displayed, "work", ProfileName::Developer).unwrap(),
            ProfileName::Developer
        );

        // A different process can replace a valid local library while the
        // popup is open. The same saved ID must not silently change the write.
        let mut changed = serde_json::to_value(&displayed).unwrap();
        let entries = changed["entries"].as_array_mut().unwrap();
        let work = entries
            .iter_mut()
            .find(|entry| entry["id"] == "work")
            .unwrap();
        work["source_preset"] = serde_json::Value::String("gaming".to_owned());
        let changed: ProfileLibraryV1 = serde_json::from_value(changed).unwrap();
        changed.validate().unwrap();
        let error = resolve_saved_apply(&changed, "work", ProfileName::Developer).unwrap_err();
        assert!(error.contains("changed since the menu was opened"));
        assert!(error.contains("reload saved profiles"));
        assert!(resolve_saved_apply(&changed, "missing", ProfileName::Developer).is_err());
    }

    #[test]
    fn pair_selector_disables_entries_that_cannot_form_an_opposite_preset_pair() {
        let mut library = ProfileLibraryV1::default();
        library
            .duplicate("developer", "developer-copy", "Work")
            .unwrap();
        assert!(!quick_slot_entry_eligible(&library, 0, "gaming"));
        assert!(quick_slot_entry_eligible(&library, 0, "developer-copy"));
        assert!(!quick_slot_entry_eligible(&library, 1, "developer-copy"));
        assert!(!quick_slot_entry_eligible(&library, 0, "unknown"));
        assert!(!quick_slot_entry_eligible(&library, 2, "gaming"));
    }
}
