//! Platform-independent state and text derivation for the native status window.
//!
//! Everything here is pure so the window's labels, colors, and button
//! enablement can be tested in WSL without a mouse or a Win32 session. The
//! Win32 layer in `tray.rs` only renders what these functions decide.

use crate::engine::ProfileMatch;
use crate::model::{
    BUTTON_DPI, BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_MOUSE4_SLOT, BUTTON_MOUSE5_SLOT, BUTTON_RIGHT,
    DpiPair, FUNCTION_BUTTON_CODE, FUNCTION_KEY_CODE, HID_USAGE_F10, HID_USAGE_F11,
    MODIFIER_LEFT_ALT, MODIFIER_LEFT_CONTROL, ProfileName, RawButtonAssignment,
    WirelessPowerSettings,
};

/// What the serialized device worker is currently doing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BusyKind {
    /// A GET-only refresh or a config change is running.
    Reading,
    /// A profile apply is running; it ends only after independent verification.
    Changing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    /// A device answered, but identity was ambiguous or not the supported mouse.
    WrongDevice,
}

/// Rendering hint for a status value; the Win32 layer maps these to colors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tone {
    Neutral,
    Developer,
    Gaming,
    Busy,
    Warning,
    Error,
    Success,
}

/// The six physical controls exposed by the Viper's GET-only onboard button
/// assignments. Selecting one in the native window only changes the detail
/// panel; it never builds a write plan or sends a setter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseControl {
    Left,
    Right,
    Middle,
    RearSide,
    FrontSide,
    Dpi,
}

impl MouseControl {
    /// Display order used by the native mouse map.
    pub const ALL: [Self; 6] = [
        Self::Left,
        Self::Right,
        Self::Middle,
        Self::RearSide,
        Self::FrontSide,
        Self::Dpi,
    ];

    #[must_use]
    pub const fn protocol_button_id(self) -> u8 {
        match self {
            Self::Left => BUTTON_LEFT,
            Self::Right => BUTTON_RIGHT,
            Self::Middle => BUTTON_MIDDLE,
            Self::RearSide => BUTTON_MOUSE4_SLOT,
            Self::FrontSide => BUTTON_MOUSE5_SLOT,
            Self::Dpi => BUTTON_DPI,
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Left => "Left click",
            Self::Right => "Right click",
            Self::Middle => "Middle click",
            Self::RearSide => "Rear side · Mouse 4",
            Self::FrontSide => "Front side · Mouse 5",
            Self::Dpi => "DPI button",
        }
    }

    #[must_use]
    pub const fn hotspot_label(self) -> &'static str {
        match self {
            Self::Left => "1",
            Self::Right => "2",
            Self::Middle => "3",
            Self::RearSide => "4",
            Self::FrontSide => "5",
            Self::Dpi => "6",
        }
    }
}

/// Human-readable evidence from one immutable snapshot's raw button record.
/// `semantic` only decodes identifiers the utility already knows; unfamiliar
/// data stays explicitly labelled as an unknown protocol value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignmentPresentation {
    pub title: String,
    pub semantic: String,
    pub raw: String,
}

/// Produces the selected native-dashboard assignment card from GET-only data.
/// Missing data is never filled in from a profile assumption.
#[must_use]
pub fn assignment_presentation(
    control: MouseControl,
    assignments: Option<&[RawButtonAssignment]>,
) -> AssignmentPresentation {
    let protocol_button_id = control.protocol_button_id();
    let title = format!("{}  ·  slot 0x{protocol_button_id:02X}", control.title());
    let assignment = assignments.and_then(|items| {
        items
            .iter()
            .find(|item| item.protocol_button_id == protocol_button_id)
    });
    match assignment {
        Some(value) => AssignmentPresentation {
            title,
            semantic: assignment_semantic(value),
            raw: assignment_raw(value),
        },
        None => AssignmentPresentation {
            title,
            semantic: "Not read from the device.".to_owned(),
            raw: "Raw assignment unavailable.".to_owned(),
        },
    }
}

fn assignment_semantic(assignment: &RawButtonAssignment) -> String {
    match assignment.function_id {
        FUNCTION_BUTTON_CODE if assignment.data_size == 0 => {
            "Mouse-button function without a target byte.".to_owned()
        }
        FUNCTION_BUTTON_CODE => mouse_button_semantic(assignment.data[0]),
        FUNCTION_KEY_CODE if assignment.data_size < 2 => {
            "Keyboard function without a complete modifier and HID-usage pair.".to_owned()
        }
        FUNCTION_KEY_CODE => keyboard_semantic(assignment.data[0], assignment.data[1]),
        function_id => format!("Unknown onboard function 0x{function_id:02X}."),
    }
}

fn mouse_button_semantic(target: u8) -> String {
    let action = match target {
        BUTTON_LEFT => "left click".to_owned(),
        BUTTON_RIGHT => "right click".to_owned(),
        BUTTON_MIDDLE => "middle click".to_owned(),
        BUTTON_MOUSE4_SLOT => "rear side · Mouse 4".to_owned(),
        BUTTON_MOUSE5_SLOT => "front side · Mouse 5".to_owned(),
        BUTTON_DPI => "DPI control".to_owned(),
        other => format!("protocol target 0x{other:02X}"),
    };
    format!("Mouse button: {action}")
}

fn keyboard_semantic(modifiers: u8, usage: u8) -> String {
    let mut parts = Vec::new();
    if modifiers & MODIFIER_LEFT_CONTROL != 0 {
        parts.push("Ctrl".to_owned());
    }
    if modifiers & MODIFIER_LEFT_ALT != 0 {
        parts.push("Alt".to_owned());
    }
    let unknown_bits = modifiers & !(MODIFIER_LEFT_CONTROL | MODIFIER_LEFT_ALT);
    if unknown_bits != 0 {
        parts.push(format!("modifier bits 0x{unknown_bits:02X}"));
    }
    let key = match usage {
        HID_USAGE_F10 => "F10".to_owned(),
        HID_USAGE_F11 => "F11".to_owned(),
        other => format!("HID usage 0x{other:02X}"),
    };
    parts.push(key);
    format!("Keyboard: {}", parts.join(" + "))
}

fn assignment_raw(assignment: &RawButtonAssignment) -> String {
    let data_size = usize::from(assignment.data_size.min(5));
    let bytes = assignment.data[..data_size]
        .iter()
        .map(|value| format!("{value:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    let bytes = if bytes.is_empty() {
        "—".to_owned()
    } else {
        bytes
    };
    format!(
        "Raw: profile 0x{:02X} · mode 0x{:02X}\r\nfunction 0x{:02X} · data[{}] {bytes}",
        assignment.profile_id, assignment.mode, assignment.function_id, assignment.data_size
    )
}

/// The most recent apply outcome shown in the window. Session-scoped; a GET
/// refresh never overwrites it because a refresh verifies nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationOutcome {
    pub profile: ProfileName,
    pub success: bool,
    pub detail: String,
    pub time: String,
    pub report_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuiState {
    pub busy: Option<BusyKind>,
    pub profile: ProfileMatch,
    pub polling_hz: Option<u16>,
    pub dpi: Option<DpiPair>,
    pub power: Option<WirelessPowerSettings>,
    pub connection: ConnectionStatus,
    pub last_verification: Option<VerificationOutcome>,
    /// The most recent worker error that was not a verified apply failure — a
    /// refused write, a failed refresh, a missing baseline. Shown in place of
    /// the verification text until a later worker result clears it.
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusPresentation {
    pub profile_line: String,
    pub profile_tone: Tone,
    pub polling_line: String,
    pub dpi_line: String,
    pub battery_line: String,
    pub sleep_line: String,
    pub low_power_line: String,
    pub connection_line: String,
    pub connection_tone: Tone,
    pub verification_text: String,
    pub verification_tone: Tone,
    pub buttons_enabled: bool,
}

#[must_use]
pub const fn profile_title(profile: ProfileName) -> &'static str {
    match profile {
        ProfileName::Developer => "Developer",
        ProfileName::Gaming => "Gaming",
    }
}

#[must_use]
pub fn polling_label(polling_hz: Option<u16>) -> String {
    match polling_hz {
        Some(hertz) => format!("{hertz} Hz"),
        None => "Unknown".to_owned(),
    }
}

#[must_use]
pub fn dpi_label(dpi: Option<DpiPair>) -> String {
    match dpi {
        Some(value) if value.x == value.y => format!("{} DPI", value.x),
        Some(value) => format!("{} × {} DPI", value.x, value.y),
        None => "Unavailable".to_owned(),
    }
}

#[must_use]
pub fn sleep_label(power: Option<WirelessPowerSettings>) -> String {
    match power.map(WirelessPowerSettings::idle_minutes) {
        Some(1) => "1 minute".to_owned(),
        Some(minutes) => format!("{minutes} minutes"),
        None => "Unavailable".to_owned(),
    }
}

#[must_use]
pub fn low_power_label(power: Option<WirelessPowerSettings>) -> String {
    power.map_or_else(
        || "Unavailable".to_owned(),
        |value| format!("{}%", value.low_power_percent()),
    )
}

#[must_use]
pub fn battery_label(power: Option<WirelessPowerSettings>) -> String {
    power.map_or_else(
        || "Unavailable".to_owned(),
        |value| format!("{}%", value.battery_percent()),
    )
}

#[must_use]
pub fn present(state: &GuiState) -> StatusPresentation {
    let (profile_line, profile_tone) = match state.busy {
        Some(BusyKind::Reading) => ("Reading\u{2026}".to_owned(), Tone::Busy),
        Some(BusyKind::Changing) => ("Changing\u{2026}".to_owned(), Tone::Busy),
        None if state.connection != ConnectionStatus::Connected => {
            ("Unknown".to_owned(), Tone::Busy)
        }
        None => match state.profile {
            ProfileMatch::Developer => ("Developer".to_owned(), Tone::Developer),
            ProfileMatch::Gaming => ("Gaming".to_owned(), Tone::Gaming),
            ProfileMatch::OutOfSync => ("Out of sync".to_owned(), Tone::Warning),
        },
    };
    let (connection_line, connection_tone) = match state.connection {
        ConnectionStatus::Connected => ("Connected".to_owned(), Tone::Neutral),
        ConnectionStatus::Disconnected => ("Disconnected".to_owned(), Tone::Error),
        ConnectionStatus::WrongDevice => ("Wrong or ambiguous device".to_owned(), Tone::Error),
    };
    let (verification_text, verification_tone) = match (&state.last_error, &state.last_verification)
    {
        (Some(error), _) => (error.clone(), Tone::Error),
        (None, None) => (
            "No profile change has run in this session.".to_owned(),
            Tone::Neutral,
        ),
        (None, Some(outcome)) => (
            verification_text(outcome),
            if outcome.success {
                Tone::Success
            } else {
                Tone::Error
            },
        ),
    };
    StatusPresentation {
        profile_line,
        profile_tone,
        polling_line: polling_label(state.polling_hz),
        dpi_line: dpi_label(state.dpi),
        battery_line: battery_label(state.power),
        sleep_line: sleep_label(state.power),
        low_power_line: low_power_label(state.power),
        connection_line,
        connection_tone,
        verification_text,
        verification_tone,
        buttons_enabled: state.busy.is_none() && state.connection == ConnectionStatus::Connected,
    }
}

#[must_use]
pub fn verification_text(outcome: &VerificationOutcome) -> String {
    let headline = if outcome.success {
        format!(
            "{} verified at {}.",
            profile_title(outcome.profile),
            outcome.time
        )
    } else {
        format!(
            "Switch to {} failed at {}.",
            profile_title(outcome.profile),
            outcome.time
        )
    };
    let mut text = format!("{headline}\r\n{}", outcome.detail);
    if let Some(path) = &outcome.report_path {
        text.push_str("\r\nReport: ");
        text.push_str(path);
    }
    text
}

#[must_use]
pub const fn success_detail(no_changes_needed: bool) -> &'static str {
    if no_changes_needed {
        "The mouse already matched this profile."
    } else {
        "Every changed setting was independently read back."
    }
}

/// Plain-English failure explanation from the verification report's rollback
/// result and the classification of the verified final state.
#[must_use]
pub fn failure_detail(
    rollback_restored: Option<bool>,
    final_profile: Option<ProfileMatch>,
) -> String {
    let base = match rollback_restored {
        Some(true) => "The change did not verify, so the previous settings were restored.",
        Some(false) => "The change did not verify and automatic restore also failed.",
        None => "The change did not verify.",
    };
    let final_state = match final_profile {
        Some(ProfileMatch::Developer) => " The mouse is now on Developer.",
        Some(ProfileMatch::Gaming) => " The mouse is now on Gaming.",
        Some(ProfileMatch::OutOfSync) => " The mouse is out of sync.",
        None => " The final mouse state could not be read.",
    };
    format!("{base}{final_state}")
}

/// Maps a worker error message onto the connection field. The strings are the
/// stable, user-facing errors produced by `windows.rs` device discovery.
#[must_use]
pub fn classify_worker_error(message: &str) -> ConnectionStatus {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("no viper v4 pro hid collections found") {
        return ConnectionStatus::Disconnected;
    }
    let ambiguous = [
        "multiple viper v4 pro serial descriptors",
        "multiple physical viper v4 pro devices",
        "unsupported viper v4 pro product id",
        "wrong vendor id",
        "identity changed",
        "protocol serial changed",
        "no hid collection accepted",
    ];
    if ambiguous.iter().any(|marker| lowered.contains(marker)) {
        return ConnectionStatus::WrongDevice;
    }
    ConnectionStatus::Connected
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchDecision {
    /// This process becomes the tray, optionally showing the window at start.
    StartTray { show_window: bool },
    /// Another instance owns the mutex and its window was found: signal it.
    ShowExisting,
    /// Another instance owns the mutex but no window answered.
    FailAlreadyRunning,
}

#[must_use]
pub const fn launch_decision(
    mutex_already_exists: bool,
    existing_window_found: bool,
    minimized: bool,
) -> LaunchDecision {
    if !mutex_already_exists {
        LaunchDecision::StartTray {
            show_window: !minimized,
        }
    } else if existing_window_found {
        LaunchDecision::ShowExisting
    } else {
        LaunchDecision::FailAlreadyRunning
    }
}

/// `--minimized` is appended to the Start-with-Windows Run entry so an
/// automatic login start stays in the tray without popping the window.
pub fn wants_minimized(arguments: impl Iterator<Item = String>) -> bool {
    let mut minimized = false;
    for argument in arguments {
        if argument == "--minimized" {
            minimized = true;
        }
    }
    minimized
}

#[must_use]
pub fn clock_label(hour: u16, minute: u16, second: u16) -> String {
    format!("{hour:02}:{minute:02}:{second:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle_state() -> GuiState {
        GuiState {
            busy: None,
            profile: ProfileMatch::Developer,
            polling_hz: Some(1000),
            dpi: Some(DpiPair { x: 1600, y: 1600 }),
            power: Some(WirelessPowerSettings {
                battery_raw: 0xcc,
                idle_seconds: 180,
                low_power_threshold_raw: 0x0d,
            }),
            connection: ConnectionStatus::Connected,
            last_verification: None,
            last_error: None,
        }
    }

    #[test]
    fn a_worker_error_is_shown_in_place_of_the_verification_text() {
        let mut state = idle_state();
        state.last_error = Some("immutable baseline is required before any tray write".to_owned());
        let presentation = present(&state);
        assert_eq!(presentation.verification_tone, Tone::Error);
        assert_eq!(
            presentation.verification_text,
            "immutable baseline is required before any tray write"
        );
    }

    #[test]
    fn idle_developer_state_presents_profile_polling_and_enabled_buttons() {
        let presentation = present(&idle_state());
        assert_eq!(presentation.profile_line, "Developer");
        assert_eq!(presentation.profile_tone, Tone::Developer);
        assert_eq!(presentation.polling_line, "1000 Hz");
        assert_eq!(presentation.dpi_line, "1600 DPI");
        assert_eq!(presentation.battery_line, "80%");
        assert_eq!(presentation.sleep_line, "3 minutes");
        assert_eq!(presentation.low_power_line, "5%");
        assert_eq!(presentation.connection_line, "Connected");
        assert!(presentation.buttons_enabled);
        assert_eq!(
            presentation.verification_text,
            "No profile change has run in this session."
        );
    }

    #[test]
    fn display_labels_preserve_asymmetric_dpi_and_singular_sleep() {
        assert_eq!(
            dpi_label(Some(DpiPair { x: 800, y: 1200 })),
            "800 × 1200 DPI"
        );
        assert_eq!(
            sleep_label(Some(WirelessPowerSettings {
                battery_raw: u8::MAX,
                idle_seconds: 60,
                low_power_threshold_raw: u8::MAX,
            })),
            "1 minute"
        );
        assert_eq!(
            battery_label(Some(WirelessPowerSettings {
                battery_raw: u8::MAX,
                idle_seconds: 60,
                low_power_threshold_raw: u8::MAX,
            })),
            "100%"
        );
        assert_eq!(
            low_power_label(Some(WirelessPowerSettings {
                battery_raw: u8::MAX,
                idle_seconds: 60,
                low_power_threshold_raw: u8::MAX,
            })),
            "100%"
        );
    }

    #[test]
    fn assignment_card_describes_known_keyboard_chord_and_raw_bytes() {
        let assignment = RawButtonAssignment::keyboard_chord(BUTTON_MOUSE4_SLOT, HID_USAGE_F10);
        let card = assignment_presentation(MouseControl::RearSide, Some(&[assignment]));
        assert_eq!(card.title, "Rear side · Mouse 4  ·  slot 0x04");
        assert_eq!(card.semantic, "Keyboard: Ctrl + Alt + F10");
        assert_eq!(
            card.raw,
            "Raw: profile 0x01 · mode 0x00\r\nfunction 0x02 · data[2] 05 43"
        );
    }

    #[test]
    fn assignment_card_keeps_unknown_and_unread_data_explicit() {
        let mut unknown = RawButtonAssignment::mouse_button(BUTTON_DPI, BUTTON_DPI);
        unknown.function_id = 0xfe;
        let known_slot_unknown_function =
            assignment_presentation(MouseControl::Dpi, Some(&[unknown]));
        assert_eq!(
            known_slot_unknown_function.semantic,
            "Unknown onboard function 0xFE."
        );
        assert_eq!(
            assignment_presentation(MouseControl::FrontSide, None).semantic,
            "Not read from the device."
        );
    }

    #[test]
    fn gaming_and_out_of_sync_labels_are_distinct() {
        let mut state = idle_state();
        state.profile = ProfileMatch::Gaming;
        state.polling_hz = Some(4000);
        let gaming = present(&state);
        assert_eq!(gaming.profile_line, "Gaming");
        assert_eq!(gaming.profile_tone, Tone::Gaming);
        assert_eq!(gaming.polling_line, "4000 Hz");

        state.profile = ProfileMatch::OutOfSync;
        let out_of_sync = present(&state);
        assert_eq!(out_of_sync.profile_line, "Out of sync");
        assert_eq!(out_of_sync.profile_tone, Tone::Warning);
        assert!(out_of_sync.buttons_enabled);
    }

    #[test]
    fn busy_states_show_reading_or_changing_and_disable_buttons() {
        let mut state = idle_state();
        state.busy = Some(BusyKind::Reading);
        let reading = present(&state);
        assert_eq!(reading.profile_line, "Reading\u{2026}");
        assert!(!reading.buttons_enabled);

        state.busy = Some(BusyKind::Changing);
        let changing = present(&state);
        assert_eq!(changing.profile_line, "Changing\u{2026}");
        assert_eq!(changing.profile_tone, Tone::Busy);
        assert!(!changing.buttons_enabled);
    }

    #[test]
    fn disconnected_state_shows_unknown_values_and_disables_buttons() {
        let mut state = idle_state();
        state.connection = ConnectionStatus::Disconnected;
        state.polling_hz = None;
        state.dpi = None;
        state.power = None;
        let presentation = present(&state);
        assert_eq!(presentation.profile_line, "Unknown");
        assert_eq!(presentation.polling_line, "Unknown");
        assert_eq!(presentation.dpi_line, "Unavailable");
        assert_eq!(presentation.battery_line, "Unavailable");
        assert_eq!(presentation.sleep_line, "Unavailable");
        assert_eq!(presentation.low_power_line, "Unavailable");
        assert_eq!(presentation.connection_line, "Disconnected");
        assert_eq!(presentation.connection_tone, Tone::Error);
        assert!(!presentation.buttons_enabled);
    }

    #[test]
    fn wrong_device_state_is_labelled_and_disables_buttons() {
        let mut state = idle_state();
        state.connection = ConnectionStatus::WrongDevice;
        let presentation = present(&state);
        assert_eq!(presentation.connection_line, "Wrong or ambiguous device");
        assert!(!presentation.buttons_enabled);
    }

    #[test]
    fn success_presentation_includes_time_detail_and_report_path() {
        let mut state = idle_state();
        state.last_verification = Some(VerificationOutcome {
            profile: ProfileName::Gaming,
            success: true,
            detail: success_detail(false).to_owned(),
            time: clock_label(14, 3, 7),
            report_path: Some("C:\\reports\\verification-v1-1.json".to_owned()),
        });
        let presentation = present(&state);
        assert_eq!(presentation.verification_tone, Tone::Success);
        assert_eq!(
            presentation.verification_text,
            "Gaming verified at 14:03:07.\r\nEvery changed setting was independently read back.\r\nReport: C:\\reports\\verification-v1-1.json"
        );
    }

    #[test]
    fn failure_presentation_reports_rollback_and_final_state() {
        let mut state = idle_state();
        state.last_verification = Some(VerificationOutcome {
            profile: ProfileName::Developer,
            success: false,
            detail: failure_detail(Some(true), Some(ProfileMatch::Gaming)),
            time: clock_label(9, 30, 0),
            report_path: None,
        });
        let presentation = present(&state);
        assert_eq!(presentation.verification_tone, Tone::Error);
        assert_eq!(
            presentation.verification_text,
            "Switch to Developer failed at 09:30:00.\r\nThe change did not verify, so the previous settings were restored. The mouse is now on Gaming."
        );
    }

    #[test]
    fn failure_detail_covers_failed_restore_and_unreadable_final_state() {
        assert_eq!(
            failure_detail(Some(false), Some(ProfileMatch::OutOfSync)),
            "The change did not verify and automatic restore also failed. The mouse is out of sync."
        );
        assert_eq!(
            failure_detail(None, None),
            "The change did not verify. The final mouse state could not be read."
        );
    }

    #[test]
    fn no_op_success_detail_says_the_mouse_already_matched() {
        assert_eq!(
            success_detail(true),
            "The mouse already matched this profile."
        );
    }

    #[test]
    fn worker_errors_classify_connection_status() {
        assert_eq!(
            classify_worker_error(
                "no Viper V4 Pro HID collections found for VID 0x1532 and PID 0x00e5/0x00e6"
            ),
            ConnectionStatus::Disconnected
        );
        assert_eq!(
            classify_worker_error(
                "multiple physical Viper V4 Pro devices answered GET commands: A, B"
            ),
            ConnectionStatus::WrongDevice
        );
        assert_eq!(
            classify_worker_error(
                "multiple Viper V4 Pro serial descriptors were found for PID 0x00e6: A, B"
            ),
            ConnectionStatus::WrongDevice
        );
        assert_eq!(
            classify_worker_error(
                "no HID collection accepted a validated serial GET; path: open failed"
            ),
            ConnectionStatus::WrongDevice
        );
        assert_eq!(
            classify_worker_error("device identity changed during the operation"),
            ConnectionStatus::WrongDevice
        );
        assert_eq!(
            classify_worker_error(
                "Razer Synapse components are running (razer synapse). Close Synapse yourself; this utility never terminates processes"
            ),
            ConnectionStatus::Connected
        );
    }

    #[test]
    fn second_launch_signals_the_existing_window() {
        assert_eq!(
            launch_decision(true, true, false),
            LaunchDecision::ShowExisting
        );
        assert_eq!(
            launch_decision(true, false, false),
            LaunchDecision::FailAlreadyRunning
        );
        assert_eq!(
            launch_decision(false, false, false),
            LaunchDecision::StartTray { show_window: true }
        );
        assert_eq!(
            launch_decision(false, false, true),
            LaunchDecision::StartTray { show_window: false }
        );
    }

    #[test]
    fn minimized_flag_is_detected_anywhere_in_the_arguments() {
        assert!(wants_minimized(["--minimized".to_owned()].into_iter()));
        assert!(wants_minimized(
            ["other".to_owned(), "--minimized".to_owned()].into_iter()
        ));
        assert!(!wants_minimized(["--minimize".to_owned()].into_iter()));
        assert!(!wants_minimized(std::iter::empty()));
    }

    #[test]
    fn clock_label_zero_pads() {
        assert_eq!(clock_label(9, 5, 3), "09:05:03");
    }
}
