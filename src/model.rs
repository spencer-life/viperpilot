use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const CONFIG_SCHEMA_VERSION: u32 = 1;
pub const RAZER_VENDOR_ID: u16 = 0x1532;
pub const VIPER_V4_PRO_WIRED_PID: u16 = 0x00e5;
pub const VIPER_V4_PRO_WIRELESS_PID: u16 = 0x00e6;
pub const PROFILE_ID: u8 = 0x01;
pub const NORMAL_BUTTON_MODE: u8 = 0x00;

pub const BUTTON_LEFT: u8 = 0x01;
pub const BUTTON_RIGHT: u8 = 0x02;
pub const BUTTON_MIDDLE: u8 = 0x03;
// Empirically measured on this mouse: the rear/Windows XButton1 control is
// protocol slot 0x04, while the front/Windows XButton2 control is slot 0x05.
pub const BUTTON_MOUSE4_SLOT: u8 = 0x04;
pub const BUTTON_MOUSE5_SLOT: u8 = 0x05;
pub const BUTTON_DPI: u8 = 0x60;
pub const ALL_BUTTON_IDS: [u8; 6] = [
    BUTTON_LEFT,
    BUTTON_RIGHT,
    BUTTON_MIDDLE,
    BUTTON_MOUSE5_SLOT,
    BUTTON_MOUSE4_SLOT,
    BUTTON_DPI,
];

pub const FUNCTION_BUTTON_CODE: u8 = 0x01;
pub const FUNCTION_KEY_CODE: u8 = 0x02;
pub const MODIFIER_LEFT_CONTROL: u8 = 0x01;
pub const MODIFIER_LEFT_ALT: u8 = 0x04;
pub const HID_USAGE_F10: u8 = 0x43;
pub const HID_USAGE_F11: u8 = 0x44;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    WiredLegacy,
    WirelessV2,
}

impl TransportKind {
    pub fn for_product_id(product_id: u16) -> Result<Self, String> {
        match product_id {
            VIPER_V4_PRO_WIRED_PID => Ok(Self::WiredLegacy),
            VIPER_V4_PRO_WIRELESS_PID => Ok(Self::WirelessV2),
            other => Err(format!("unsupported Viper V4 Pro product ID 0x{other:04x}")),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeviceIdentity {
    pub vendor_id: u16,
    pub product_id: u16,
    #[serde(default)]
    pub hid_descriptor_serial: Option<String>,
    pub serial: String,
    pub firmware: FirmwareVersion,
    pub path: String,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
}

impl DeviceIdentity {
    pub fn validate(&self) -> Result<(), String> {
        if self.vendor_id != RAZER_VENDOR_ID {
            return Err(format!(
                "wrong vendor ID: expected 0x{RAZER_VENDOR_ID:04x}, observed 0x{:04x}",
                self.vendor_id
            ));
        }
        TransportKind::for_product_id(self.product_id)?;
        if self.serial.trim().is_empty() {
            return Err("device returned an empty serial number".to_owned());
        }
        if self.path.is_empty() {
            return Err("device path is empty".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FirmwareVersion {
    pub major: u8,
    pub minor: u8,
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PollingState {
    pub transport: TransportKind,
    pub raw_code: u8,
}

impl PollingState {
    #[must_use]
    pub fn hertz(self) -> Option<u16> {
        match self.transport {
            TransportKind::WiredLegacy => match self.raw_code {
                0x01 => Some(1000),
                0x02 => Some(500),
                0x08 => Some(125),
                _ => None,
            },
            TransportKind::WirelessV2 => match self.raw_code {
                0x01 => Some(8000),
                0x02 => Some(4000),
                0x04 => Some(2000),
                0x08 => Some(1000),
                0x10 => Some(500),
                0x20 => Some(250),
                0x40 => Some(125),
                _ => None,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DpiPair {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DpiStage {
    pub id: u8,
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DpiState {
    pub profile_id: u8,
    pub current: DpiPair,
    pub active_stage_id: u8,
    pub stages: Vec<DpiStage>,
}

impl DpiState {
    #[must_use]
    pub fn active_stage(&self) -> Option<&DpiStage> {
        self.stages
            .iter()
            .find(|stage| stage.id == self.active_stage_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RawButtonAssignment {
    pub profile_id: u8,
    pub protocol_button_id: u8,
    // Raw GET response byte. The official setter uses NORMAL (0x00), and this
    // mouse proved the returned byte does not round-trip for every side slot.
    pub mode: u8,
    pub function_id: u8,
    pub data_size: u8,
    pub data: [u8; 5],
}

impl RawButtonAssignment {
    #[must_use]
    pub fn keyboard_chord(protocol_button_id: u8, usage: u8) -> Self {
        Self {
            profile_id: PROFILE_ID,
            protocol_button_id,
            mode: NORMAL_BUTTON_MODE,
            function_id: FUNCTION_KEY_CODE,
            data_size: 2,
            data: [MODIFIER_LEFT_CONTROL | MODIFIER_LEFT_ALT, usage, 0, 0, 0],
        }
    }

    #[must_use]
    pub fn mouse_button(protocol_button_id: u8, target_button_id: u8) -> Self {
        Self {
            profile_id: PROFILE_ID,
            protocol_button_id,
            mode: NORMAL_BUTTON_MODE,
            function_id: FUNCTION_BUTTON_CODE,
            data_size: 1,
            data: [target_button_id, 0, 0, 0, 0],
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.data_size > 5 {
            return Err(format!(
                "button 0x{:02x} has invalid data size {}",
                self.protocol_button_id, self.data_size
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeviceSnapshotV1 {
    pub schema_version: u32,
    pub captured_at_unix_ms: u64,
    pub device: DeviceIdentity,
    pub polling: PollingState,
    pub dpi: DpiState,
    /// GET-only wireless power values. This optional v1 extension keeps older
    /// immutable baselines readable and is never used to build a write plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<WirelessPowerSettings>,
    pub button_assignments: Vec<RawButtonAssignment>,
}

impl DeviceSnapshotV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SNAPSHOT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported snapshot schema {}, expected {SNAPSHOT_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        self.device.validate()?;
        let expected_transport = TransportKind::for_product_id(self.device.product_id)?;
        if self.polling.transport != expected_transport {
            return Err("polling transport does not match product ID".to_owned());
        }
        if self.dpi.profile_id != PROFILE_ID {
            return Err(format!(
                "unexpected DPI profile ID 0x{:02x}",
                self.dpi.profile_id
            ));
        }
        if self.dpi.stages.is_empty() {
            return Err("snapshot has no DPI stages".to_owned());
        }
        if self.dpi.active_stage().is_none() {
            return Err(format!(
                "active DPI stage {} is absent from the stage table",
                self.dpi.active_stage_id
            ));
        }
        if self.button_assignments.len() != ALL_BUTTON_IDS.len() {
            return Err(format!(
                "snapshot contains {} button assignments, expected {}",
                self.button_assignments.len(),
                ALL_BUTTON_IDS.len()
            ));
        }
        for expected_id in ALL_BUTTON_IDS {
            let count = self
                .button_assignments
                .iter()
                .filter(|assignment| assignment.protocol_button_id == expected_id)
                .count();
            if count != 1 {
                return Err(format!(
                    "snapshot contains {count} assignments for protocol button 0x{expected_id:02x}"
                ));
            }
        }
        for assignment in &self.button_assignments {
            assignment.validate()?;
            if assignment.profile_id != PROFILE_ID {
                return Err(format!(
                    "button 0x{:02x} is not the expected profile",
                    assignment.protocol_button_id
                ));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn button(&self, protocol_button_id: u8) -> Option<&RawButtonAssignment> {
        self.button_assignments
            .iter()
            .find(|assignment| assignment.protocol_button_id == protocol_button_id)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WirelessPowerSettings {
    /// Raw 0..=255 battery level retained for measured provenance.
    pub battery_raw: u8,
    /// Raw protocol value, in seconds.
    pub idle_seconds: u16,
    /// Raw 0x0d..=0xff threshold retained for measured provenance.
    pub low_power_threshold_raw: u8,
}

impl WirelessPowerSettings {
    #[must_use]
    pub fn battery_percent(self) -> u8 {
        u8::try_from((u16::from(self.battery_raw) * 100 + 127) / 255).unwrap_or(100)
    }

    #[must_use]
    pub fn idle_minutes(self) -> u16 {
        // Razer's UI and ClickSync both expose minute slots from 1 through 15.
        self.idle_seconds.clamp(60, 900).div_ceil(60)
    }

    #[must_use]
    pub fn low_power_percent(self) -> u8 {
        // ClickSync converts raw/255 to a percentage and rounds to a 5% slot.
        let raw = u16::from(self.low_power_threshold_raw.clamp(0x0d, 0xff));
        u8::try_from(((raw * 20 + 127) / 255) * 5).unwrap_or(100)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileName {
    Developer,
    Gaming,
}

impl fmt::Display for ProfileName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Developer => formatter.write_str("developer"),
            Self::Gaming => formatter.write_str("gaming"),
        }
    }
}

impl FromStr for ProfileName {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "developer" => Ok(Self::Developer),
            "gaming" => Ok(Self::Gaming),
            _ => Err(format!(
                "unknown profile {value:?}; expected developer or gaming"
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProfileSpec {
    pub name: ProfileName,
    pub dpi: DpiPair,
    pub polling_hz: u16,
    pub side_buttons: Vec<RawButtonAssignment>,
}

impl ProfileSpec {
    #[must_use]
    pub fn fixed(name: ProfileName) -> Self {
        let side_buttons = match name {
            ProfileName::Developer => vec![
                RawButtonAssignment::keyboard_chord(BUTTON_MOUSE4_SLOT, HID_USAGE_F10),
                RawButtonAssignment::keyboard_chord(BUTTON_MOUSE5_SLOT, HID_USAGE_F11),
            ],
            ProfileName::Gaming => vec![
                RawButtonAssignment::mouse_button(BUTTON_MOUSE4_SLOT, BUTTON_MOUSE4_SLOT),
                RawButtonAssignment::mouse_button(BUTTON_MOUSE5_SLOT, BUTTON_MOUSE5_SLOT),
            ],
        };
        Self {
            name,
            dpi: DpiPair { x: 1600, y: 1600 },
            polling_hz: match name {
                ProfileName::Developer => 1000,
                ProfileName::Gaming => 4000,
            },
            side_buttons,
        }
    }

    pub fn polling_code(&self, transport: TransportKind) -> Result<u8, String> {
        match (transport, self.polling_hz) {
            (TransportKind::WiredLegacy, 1000) => Ok(0x01),
            (TransportKind::WirelessV2, 1000) => Ok(0x08),
            (TransportKind::WirelessV2, 4000) => Ok(0x02),
            (TransportKind::WiredLegacy, 4000) => Err(
                "Gaming/4000 Hz is disabled for wired PID 0x00e5 until V2 polling is proven"
                    .to_owned(),
            ),
            (_, other) => Err(format!("unsupported polling target {other} Hz")),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UtilityConfigV1 {
    pub schema_version: u32,
    pub last_verified_profile: Option<ProfileName>,
    pub start_with_windows: bool,
    pub exit_after_gaming: bool,
    pub diagnostics_enabled: bool,
}

impl Default for UtilityConfigV1 {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            last_verified_profile: None,
            start_with_windows: false,
            exit_after_gaming: false,
            diagnostics_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_chords_use_ctrl_alt_f10_and_f11() {
        let spec = ProfileSpec::fixed(ProfileName::Developer);
        assert_eq!(spec.side_buttons[0].data_size, 2);
        assert_eq!(spec.side_buttons[0].data, [0x05, 0x43, 0, 0, 0]);
        assert_eq!(spec.side_buttons[1].data, [0x05, 0x44, 0, 0, 0]);
        assert_eq!(spec.side_buttons[0].protocol_button_id, 0x04);
        assert_eq!(spec.side_buttons[1].protocol_button_id, 0x05);
    }

    #[test]
    fn gaming_uses_native_side_button_codes() {
        let spec = ProfileSpec::fixed(ProfileName::Gaming);
        assert_eq!(spec.side_buttons[0].function_id, FUNCTION_BUTTON_CODE);
        assert_eq!(spec.side_buttons[0].data, [0x04, 0, 0, 0, 0]);
        assert_eq!(spec.side_buttons[1].data, [0x05, 0, 0, 0, 0]);
    }

    #[test]
    fn polling_codes_and_wired_fail_closed() {
        let developer = ProfileSpec::fixed(ProfileName::Developer);
        let gaming = ProfileSpec::fixed(ProfileName::Gaming);
        assert_eq!(developer.polling_code(TransportKind::WiredLegacy), Ok(0x01));
        assert_eq!(developer.polling_code(TransportKind::WirelessV2), Ok(0x08));
        assert_eq!(gaming.polling_code(TransportKind::WirelessV2), Ok(0x02));
        assert!(gaming.polling_code(TransportKind::WiredLegacy).is_err());
    }

    #[test]
    fn config_defaults_do_not_write_or_exit() {
        let config = UtilityConfigV1::default();
        assert_eq!(config.schema_version, 1);
        assert_eq!(config.last_verified_profile, None);
        assert!(!config.start_with_windows);
        assert!(!config.exit_after_gaming);
        assert!(!config.diagnostics_enabled);
    }

    #[test]
    fn fixed_profiles_round_trip_through_versioned_json() {
        for profile in [ProfileName::Developer, ProfileName::Gaming] {
            let original = ProfileSpec::fixed(profile);
            let encoded = serde_json::to_string(&original).expect("serialize profile");
            let decoded: ProfileSpec = serde_json::from_str(&encoded).expect("deserialize profile");
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn read_only_power_values_use_razer_ui_units() {
        let power = WirelessPowerSettings {
            battery_raw: 0xcc,
            idle_seconds: 180,
            low_power_threshold_raw: 0x0d,
        };
        assert_eq!(power.battery_percent(), 80);
        assert_eq!(power.idle_minutes(), 3);
        assert_eq!(power.low_power_percent(), 5);
    }

    #[test]
    fn snapshot_validation_rejects_wrong_product_id() {
        let identity = DeviceIdentity {
            vendor_id: RAZER_VENDOR_ID,
            product_id: 0xffff,
            hid_descriptor_serial: None,
            serial: "SERIAL".to_owned(),
            firmware: FirmwareVersion { major: 1, minor: 0 },
            path: "path".to_owned(),
            interface_number: 4,
            usage_page: 0x0c,
            usage: 1,
        };
        assert!(identity.validate().is_err());
    }
}
