use std::collections::BTreeSet;
use std::ffi::CStr;
use std::os::windows::process::CommandExt;
use std::process::Command as ProcessCommand;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hidapi::{DeviceInfo, HidApi, HidDevice};
use serde::Serialize;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey, VK_F10,
    VK_F11, VK_XBUTTON1, VK_XBUTTON2,
};
use windows::Win32::UI::WindowsAndMessaging::{MSG, PM_REMOVE, PeekMessageW, WM_HOTKEY};

use crate::engine::{DeviceControl, WriteFailure, WriteFailureKind};
use crate::model::{
    ALL_BUTTON_IDS, DeviceIdentity, DeviceSnapshotV1, DpiState, PollingState, RAZER_VENDOR_ID,
    SNAPSHOT_SCHEMA_VERSION, TransportKind, VIPER_V4_PRO_WIRED_PID, VIPER_V4_PRO_WIRELESS_PID,
};
use crate::planning::WriteOperation;
use crate::protocol::{
    self, Command, ProtocolError, Response, TransactionSequence, WRITE_ATTEMPTS,
};

const GET_ATTEMPTS: usize = 11;
const WRITE_ACK_READ_ATTEMPTS: usize = 11;
const POST_OPEN_SETTLE_DELAY: Duration = Duration::from_millis(60);
const COMMAND_SETTLE_DELAY: Duration = Duration::from_millis(8);
const BUSY_RETRY_DELAY: Duration = Duration::from_millis(16);
// Measured on this mouse: a successful 4000 -> 1000 V2 write can become
// visible only after the old immediate verifier has already started rollback.
// Wait after both acknowledgements, never between the two selector packets.
const POLLING_V2_COMMIT_SETTLE_DELAY: Duration = Duration::from_millis(1_000);
const BUTTON_IO_DELAY: Duration = Duration::from_millis(24);
const BUTTON_GET_ATTEMPTS: usize = 3;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, Serialize)]
pub struct EnumeratedDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_descriptor: Option<String>,
    pub path: String,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub control_preference_rank: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct PollingPathProbe {
    pub path: String,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub raw_code: Option<u8>,
    pub polling_hz: Option<u16>,
    pub error: Option<String>,
}

pub fn enumerate_devices() -> Result<Vec<EnumeratedDevice>, String> {
    let api = HidApi::new().map_err(|error| format!("HID initialization failed: {error}"))?;
    let mut devices: Vec<_> = api
        .device_list()
        .filter(|info| is_viper(info))
        .map(candidate_from_info)
        .collect();
    devices.sort_by_key(|device| {
        (
            product_preference_rank(device.product_id),
            device.control_preference_rank,
            device.interface_number,
            device.path.clone(),
        )
    });
    Ok(devices)
}

/// Sends only the documented wireless V2 polling GET to every wireless Viper
/// collection. This is a narrow diagnostic for identifying the dongle's live
/// readback path; failures are reported per collection and never retried as a
/// different command.
pub fn probe_wireless_polling_paths() -> Result<Vec<PollingPathProbe>, String> {
    let api = HidApi::new().map_err(|error| format!("HID initialization failed: {error}"))?;
    let mut infos: Vec<&DeviceInfo> = api
        .device_list()
        .filter(|info| {
            info.vendor_id() == RAZER_VENDOR_ID && info.product_id() == VIPER_V4_PRO_WIRELESS_PID
        })
        .collect();
    infos.sort_by_key(|info| {
        (
            control_rank(info),
            info.interface_number(),
            cstr_to_string(info.path()),
        )
    });

    let mut probes = Vec::with_capacity(infos.len());
    for info in infos {
        let candidate = candidate_from_info(info);
        let result = info
            .open_device(&api)
            .map_err(|error| format!("open failed: {error}"))
            .and_then(|device| {
                let opened = RazerDevice {
                    device,
                    candidate: candidate.clone(),
                    serial: String::new(),
                    transactions: TransactionSequence::default(),
                };
                thread::sleep(POST_OPEN_SETTLE_DELAY);
                let command = protocol::get_polling(0, TransportKind::WirelessV2);
                let response = opened.exchange_get(&command)?;
                protocol::parse_polling(&response, TransportKind::WirelessV2)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(raw_code) => {
                let state = PollingState {
                    transport: TransportKind::WirelessV2,
                    raw_code,
                };
                probes.push(PollingPathProbe {
                    path: candidate.path,
                    interface_number: candidate.interface_number,
                    usage_page: candidate.usage_page,
                    usage: candidate.usage,
                    raw_code: Some(raw_code),
                    polling_hz: state.hertz(),
                    error: None,
                });
            }
            Err(error) => probes.push(PollingPathProbe {
                path: candidate.path,
                interface_number: candidate.interface_number,
                usage_page: candidate.usage_page,
                usage: candidate.usage,
                raw_code: None,
                polling_hz: None,
                error: Some(error),
            }),
        }
    }
    Ok(probes)
}

pub fn refuse_if_synapse_running() -> Result<(), String> {
    let output = ProcessCommand::new("tasklist.exe")
        .args(["/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| {
            format!("could not inspect Windows processes with tasklist.exe: {error}")
        })?;
    if !output.status.success() {
        return Err(format!(
            "tasklist.exe failed with status {}; refusing device access",
            output.status
        ));
    }
    let process_list = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    let conflicts = [
        "razer synapse",
        "razerappengine",
        "razeringameengine",
        "rzsdkserver",
    ];
    let observed: Vec<_> = conflicts
        .into_iter()
        .filter(|name| process_list.contains(name))
        .collect();
    if observed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Razer Synapse components are running ({}). Close Synapse yourself; this utility never terminates processes",
            observed.join(", ")
        ))
    }
}

pub struct RazerDevice {
    device: HidDevice,
    candidate: EnumeratedDevice,
    serial: String,
    transactions: TransactionSequence,
}

impl RazerDevice {
    pub fn open_unique() -> Result<Self, String> {
        let api = HidApi::new().map_err(|error| format!("HID initialization failed: {error}"))?;
        let infos: Vec<&DeviceInfo> = api.device_list().filter(|info| is_viper(info)).collect();
        if infos.is_empty() {
            return Err(
                "no Viper V4 Pro HID collections found for VID 0x1532 and PID 0x00e5/0x00e6"
                    .to_owned(),
            );
        }
        let mut failures = Vec::new();
        for product_id in product_probe_order() {
            let group: Vec<&DeviceInfo> = infos
                .iter()
                .copied()
                .filter(|info| info.product_id() == product_id)
                .collect();
            let (selected, group_failures) = Self::probe_product_group(&api, group)?;
            failures.extend(group_failures);
            if let Some(device) = selected {
                return Ok(device);
            }
        }
        Err(format!(
            "no HID collection accepted a validated serial GET; {}",
            failures.join(" | ")
        ))
    }

    fn probe_product_group(
        api: &HidApi,
        mut infos: Vec<&DeviceInfo>,
    ) -> Result<(Option<Self>, Vec<String>), String> {
        if infos.is_empty() {
            return Ok((None, Vec::new()));
        }
        infos.sort_by_key(|info| {
            let candidate = candidate_from_info(info);
            (
                candidate.control_preference_rank,
                candidate.interface_number,
                candidate.path,
            )
        });

        let descriptor_serials: BTreeSet<String> = infos
            .iter()
            .filter_map(|info| info.serial_number().map(ToOwned::to_owned))
            .filter(|serial| !serial.trim().is_empty())
            .collect();
        if descriptor_serials.len() > 1 {
            return Err(format!(
                "multiple Viper V4 Pro serial descriptors were found for PID 0x{:04x}: {}",
                infos[0].product_id(),
                descriptor_serials
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        let mut successful = Vec::new();
        let mut failures = Vec::new();
        for info in infos {
            let candidate = candidate_from_info(info);
            let path = candidate.path.clone();
            let device = match info.open_device(api) {
                Ok(device) => device,
                Err(error) => {
                    failures.push(format!("{path}: open failed: {error}"));
                    continue;
                }
            };
            let mut opened = Self {
                device,
                candidate,
                serial: String::new(),
                transactions: TransactionSequence::default(),
            };
            thread::sleep(POST_OPEN_SETTLE_DELAY);
            match opened.read_serial() {
                Ok(serial) => {
                    opened.serial.clone_from(&serial);
                    successful.push(opened);
                }
                Err(error) => {
                    failures.push(format!("{path}: validated serial GET failed: {error}"));
                }
            }
        }
        if successful.is_empty() {
            return Ok((None, failures));
        }
        let serials: BTreeSet<String> = successful
            .iter()
            .map(|device| device.serial.clone())
            .collect();
        if serials.len() > 1 {
            return Err(format!(
                "multiple physical Viper V4 Pro devices answered GET commands: {}",
                serials.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        Ok((Some(successful.remove(0)), failures))
    }

    fn read_serial(&mut self) -> Result<String, String> {
        let transaction_id = self.transactions.next_id();
        let response = self.exchange_get(&protocol::get_serial(transaction_id))?;
        protocol::parse_serial(&response).map_err(|error| error.to_string())
    }

    fn exchange_get(&self, command: &Command) -> Result<Response, String> {
        self.exchange_get_with_delay(command, COMMAND_SETTLE_DELAY)
    }

    fn exchange_get_with_delay(
        &self,
        command: &Command,
        settle_delay: Duration,
    ) -> Result<Response, String> {
        let report = command.encode_hid();
        let mut failures = Vec::new();
        for attempt in 1..=GET_ATTEMPTS {
            self.device
                .send_feature_report(&report)
                .map_err(|error| format!("feature report send failed: {error}"))?;
            thread::sleep(settle_delay);
            let mut buffer = [0_u8; protocol::HID_REPORT_LEN];
            buffer[0] = protocol::REPORT_ID;
            let read = self
                .device
                .get_feature_report(&mut buffer)
                .map_err(|error| format!("feature report read failed: {error}"))?;
            let response = protocol::hid_payload(&buffer[..read])
                .and_then(|payload| Response::decode(payload, command.header));
            let result = response.and_then(|response| {
                response.require_success()?;
                Ok(response)
            });
            match result {
                Ok(response) => return Ok(response),
                Err(error) if error.is_retryable_get() && attempt < GET_ATTEMPTS => {
                    failures.push(format!("attempt {attempt}: {error}"));
                    thread::sleep(BUSY_RETRY_DELAY);
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        Err(format!(
            "GET failed after {GET_ATTEMPTS} validated attempts: {}",
            failures.join(" | ")
        ))
    }

    fn read_button_assignment(
        &mut self,
        button_id: u8,
    ) -> Result<crate::model::RawButtonAssignment, String> {
        let mut failures = Vec::new();
        for attempt in 1..=BUTTON_GET_ATTEMPTS {
            let command = protocol::get_button_assignment(self.transactions.next_id(), button_id);
            let result = self
                .exchange_get_with_delay(&command, BUTTON_IO_DELAY)
                .and_then(|response| {
                    protocol::parse_button_assignment(&response, button_id)
                        .map_err(|error| error.to_string())
                });
            match result {
                Ok(assignment) => return Ok(assignment),
                Err(error) => failures.push(format!("attempt {attempt}: {error}")),
            }
            if attempt < BUTTON_GET_ATTEMPTS {
                thread::sleep(BUTTON_IO_DELAY);
            }
        }
        Err(format!(
            "button 0x{button_id:02x} GET failed after {BUTTON_GET_ATTEMPTS} attempts: {}",
            failures.join(" | ")
        ))
    }

    fn exchange_write(&self, command: &Command) -> Result<(), WriteFailure> {
        let report = command.encode_hid();
        // ClickSync retries the complete request after BUSY. A BUSY reply is
        // the only immediate resend we allow. A fully validated reply for an
        // earlier transaction is drained without duplicating this write;
        // timeouts and malformed acknowledgements remain ambiguous.
        'request: for attempt in 1..=WRITE_ATTEMPTS {
            self.device.send_feature_report(&report).map_err(|error| {
                WriteFailure::ambiguous(format!("feature report send result is ambiguous: {error}"))
            })?;
            for read_attempt in 1..=WRITE_ACK_READ_ATTEMPTS {
                let mut buffer = [0_u8; protocol::HID_REPORT_LEN];
                buffer[0] = protocol::REPORT_ID;
                let read = self
                    .device
                    .get_feature_report(&mut buffer)
                    .map_err(|error| {
                        WriteFailure::ambiguous(format!(
                            "write acknowledgment read is ambiguous: {error}"
                        ))
                    })?;
                let payload = protocol::hid_payload(&buffer[..read]).map_err(|error| {
                    WriteFailure::ambiguous(format!("write acknowledgment was invalid: {error}"))
                })?;
                let response = match Response::decode(payload, command.header) {
                    Ok(response) => response,
                    Err(error)
                        if error.is_stale_write_ack() && read_attempt < WRITE_ACK_READ_ATTEMPTS =>
                    {
                        eprintln!(
                            "discarding stale write acknowledgment for {:02x}/{:02x} ({error}); waiting for the current transaction",
                            command.header.command_class, command.header.command_id
                        );
                        thread::sleep(BUSY_RETRY_DELAY);
                        continue;
                    }
                    Err(error) => {
                        return Err(WriteFailure::ambiguous(format!(
                            "write acknowledgment was invalid: {error}"
                        )));
                    }
                };
                match response.require_success() {
                    Ok(()) => return Ok(()),
                    Err(error) if error.should_resend_write(attempt) => {
                        eprintln!(
                            "device returned BUSY for {:02x}/{:02x}; resending the same packet (attempt {}/{WRITE_ATTEMPTS})",
                            command.header.command_class,
                            command.header.command_id,
                            attempt + 1
                        );
                        thread::sleep(BUSY_RETRY_DELAY);
                        continue 'request;
                    }
                    Err(ProtocolError::DeviceFailure | ProtocolError::NotSupported) => {
                        return Err(WriteFailure::rejected(format!(
                            "device rejected {:02x}/{:02x} with status 0x{:02x}",
                            command.header.command_class,
                            command.header.command_id,
                            response.status
                        )));
                    }
                    Err(error) => {
                        return Err(WriteFailure::ambiguous(format!(
                            "write result is ambiguous: {error}"
                        )));
                    }
                }
            }
        }
        Err(WriteFailure::ambiguous(
            "device remained busy after eleven complete write attempts",
        ))
    }
}

impl DeviceControl for RazerDevice {
    fn read_snapshot(&mut self) -> Result<DeviceSnapshotV1, String> {
        let serial = self.read_serial()?;
        if !self.serial.is_empty() && serial != self.serial {
            return Err(format!(
                "protocol serial changed from {:?} to {:?}",
                self.serial, serial
            ));
        }
        self.serial.clone_from(&serial);

        let firmware_command = protocol::get_firmware(self.transactions.next_id());
        let firmware = protocol::parse_firmware(&self.exchange_get(&firmware_command)?)
            .map_err(|error| error.to_string())?;
        let transport = TransportKind::for_product_id(self.candidate.product_id)?;
        let polling_command = protocol::get_polling(self.transactions.next_id(), transport);
        let polling_raw = protocol::parse_polling(&self.exchange_get(&polling_command)?, transport)
            .map_err(|error| error.to_string())?;
        let dpi_command = protocol::get_current_dpi(self.transactions.next_id());
        let current_dpi = protocol::parse_current_dpi(&self.exchange_get(&dpi_command)?)
            .map_err(|error| error.to_string())?;
        let stages_command = protocol::get_dpi_stages(self.transactions.next_id());
        let (active_stage_id, stages) =
            protocol::parse_dpi_stages(&self.exchange_get(&stages_command)?)
                .map_err(|error| error.to_string())?;
        let mut button_assignments = Vec::with_capacity(ALL_BUTTON_IDS.len());
        for button_id in ALL_BUTTON_IDS {
            button_assignments.push(self.read_button_assignment(button_id)?);
        }
        // These are display-only values. Query them after every required field
        // and fail soft so a firmware quirk cannot disable verified profiles.
        let power = (|| {
            let battery_command = protocol::get_battery_level(self.transactions.next_id());
            let battery = self.exchange_get(&battery_command).ok()?;
            let idle_command = protocol::get_idle_timeout(self.transactions.next_id());
            let idle = self.exchange_get(&idle_command).ok()?;
            let threshold_command = protocol::get_low_power_threshold(self.transactions.next_id());
            let threshold = self.exchange_get(&threshold_command).ok()?;
            protocol::parse_power_settings(&battery, &idle, &threshold).ok()
        })();
        let snapshot = DeviceSnapshotV1 {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            captured_at_unix_ms: current_unix_ms(),
            device: DeviceIdentity {
                vendor_id: self.candidate.vendor_id,
                product_id: self.candidate.product_id,
                hid_descriptor_serial: self.candidate.serial_descriptor.clone(),
                serial,
                firmware,
                path: self.candidate.path.clone(),
                interface_number: self.candidate.interface_number,
                usage_page: self.candidate.usage_page,
                usage: self.candidate.usage,
            },
            polling: PollingState {
                transport,
                raw_code: polling_raw,
            },
            dpi: DpiState {
                profile_id: crate::model::PROFILE_ID,
                current: current_dpi,
                active_stage_id,
                stages,
            },
            power,
            button_assignments,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn write_operation(&mut self, operation: &WriteOperation) -> Result<(), WriteFailure> {
        match operation {
            WriteOperation::SetDpiStages {
                after_active_stage_id,
                after_stages,
                ..
            } => {
                let command = protocol::set_dpi_stages(
                    self.transactions.next_id(),
                    *after_active_stage_id,
                    after_stages,
                )
                .map_err(|error| WriteFailure::rejected(error.to_string()))?;
                self.exchange_write(&command)
            }
            WriteOperation::SetCurrentDpi { after, .. } => {
                let command = protocol::set_current_dpi(self.transactions.next_id(), *after);
                self.exchange_write(&command)
            }
            WriteOperation::SetPolling {
                transport,
                after_raw_code,
                ..
            } => {
                let transaction_ids = [self.transactions.next_id(), self.transactions.next_id()];
                let commands = protocol::set_polling(transaction_ids, *transport, *after_raw_code);
                let is_multi_report = commands.len() > 1;
                for (index, command) in commands.into_iter().enumerate() {
                    if let Err(error) = self.exchange_write(&command) {
                        if is_multi_report
                            && (index > 0 || error.kind != WriteFailureKind::Rejected)
                        {
                            return Err(WriteFailure::unverifiable_partial(format!(
                                "wireless V2 polling physical report {} failed after the logical operation began: {error}",
                                index + 1
                            )));
                        }
                        return Err(error);
                    }
                }
                if is_multi_report {
                    thread::sleep(POLLING_V2_COMMIT_SETTLE_DELAY);
                }
                Ok(())
            }
            WriteOperation::SetButton { after, .. } => {
                let command = protocol::set_button_assignment(self.transactions.next_id(), after)
                    .map_err(|error| WriteFailure::rejected(error.to_string()))?;
                self.exchange_write(&command)
            }
        }
    }
}

pub fn listen() -> Result<(), String> {
    const F10_ID: i32 = 0x5A10;
    const F11_ID: i32 = 0x5A11;
    let modifiers = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
    unsafe { RegisterHotKey(None, F10_ID, modifiers, u32::from(VK_F10.0)) }
        .map_err(|error| format!("could not register diagnostic Ctrl+Alt+F10: {error}"))?;
    if let Err(error) = unsafe { RegisterHotKey(None, F11_ID, modifiers, u32::from(VK_F11.0)) } {
        let _ = unsafe { UnregisterHotKey(None, F10_ID) };
        return Err(format!(
            "could not register diagnostic Ctrl+Alt+F11: {error}"
        ));
    }

    println!("Listening for Ctrl+Alt+F10/F11 and XButton1/XButton2. Press Ctrl+C to stop.");
    let mut message = MSG::default();
    let mut xbutton1_down = false;
    let mut xbutton2_down = false;
    loop {
        while unsafe { PeekMessageW(&mut message, None, WM_HOTKEY, WM_HOTKEY, PM_REMOVE) }.as_bool()
        {
            match message.wParam.0 as i32 {
                F10_ID => println!("Ctrl+Alt+F10"),
                F11_ID => println!("Ctrl+Alt+F11"),
                _ => {}
            }
        }
        let xbutton1 = unsafe { GetAsyncKeyState(i32::from(VK_XBUTTON1.0)) } as u16 & 0x8000 != 0;
        let xbutton2 = unsafe { GetAsyncKeyState(i32::from(VK_XBUTTON2.0)) } as u16 & 0x8000 != 0;
        if xbutton1 && !xbutton1_down {
            println!("XButton1 down");
        }
        if xbutton2 && !xbutton2_down {
            println!("XButton2 down");
        }
        xbutton1_down = xbutton1;
        xbutton2_down = xbutton2;
        thread::sleep(Duration::from_millis(2));
    }
}

fn is_viper(info: &DeviceInfo) -> bool {
    info.vendor_id() == RAZER_VENDOR_ID
        && matches!(
            info.product_id(),
            VIPER_V4_PRO_WIRED_PID | VIPER_V4_PRO_WIRELESS_PID
        )
}

fn candidate_from_info(info: &DeviceInfo) -> EnumeratedDevice {
    EnumeratedDevice {
        vendor_id: info.vendor_id(),
        product_id: info.product_id(),
        serial_descriptor: info.serial_number().map(ToOwned::to_owned),
        path: cstr_to_string(info.path()),
        interface_number: info.interface_number(),
        usage_page: info.usage_page(),
        usage: info.usage(),
        control_preference_rank: control_rank(info),
    }
}

fn control_rank(info: &DeviceInfo) -> u8 {
    if info.usage_page() == 0x0c {
        0
    } else if info.interface_number() == 4 {
        1
    } else if (0xff00..=0xffff).contains(&info.usage_page()) {
        2
    } else {
        3
    }
}

fn product_preference_rank(product_id: u16) -> u8 {
    u8::from(product_id != VIPER_V4_PRO_WIRELESS_PID)
}

fn product_probe_order() -> [u16; 2] {
    [VIPER_V4_PRO_WIRELESS_PID, VIPER_V4_PRO_WIRED_PID]
}

fn cstr_to_string(value: &CStr) -> String {
    value.to_string_lossy().into_owned()
}

fn current_unix_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wireless_is_probed_before_wired_fallback() {
        assert_eq!(
            product_probe_order(),
            [VIPER_V4_PRO_WIRELESS_PID, VIPER_V4_PRO_WIRED_PID]
        );
    }
}
