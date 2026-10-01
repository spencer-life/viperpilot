use std::error::Error;
use std::fmt;

use crate::model::{
    DpiPair, DpiStage, FirmwareVersion, PROFILE_ID, RawButtonAssignment, TransportKind,
    WirelessPowerSettings,
};

pub const REPORT_LEN: usize = 90;
pub const HID_REPORT_LEN: usize = REPORT_LEN + 1;
pub const WRITE_ATTEMPTS: usize = 11;
pub const ARGUMENT_LEN: usize = 80;
pub const REPORT_ID: u8 = 0;

pub const STATUS_NEW_COMMAND: u8 = 0x00;
pub const STATUS_BUSY: u8 = 0x01;
pub const STATUS_SUCCESS: u8 = 0x02;
pub const STATUS_FAILURE: u8 = 0x03;
pub const STATUS_TIMEOUT: u8 = 0x04;
pub const STATUS_NOT_SUPPORTED: u8 = 0x05;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandHeader {
    pub transaction_id: u8,
    pub command_class: u8,
    pub command_id: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Command {
    pub header: CommandHeader,
    pub data_size: u8,
    pub arguments: Vec<u8>,
}

impl Command {
    pub fn new(
        transaction_id: u8,
        command_class: u8,
        command_id: u8,
        data_size: u8,
        arguments: &[u8],
    ) -> Result<Self, ProtocolError> {
        if usize::from(data_size) > ARGUMENT_LEN {
            return Err(ProtocolError::InvalidDataSize(data_size));
        }
        if arguments.len() > ARGUMENT_LEN {
            return Err(ProtocolError::TooManyArguments(arguments.len()));
        }
        Ok(Self {
            header: CommandHeader {
                transaction_id,
                command_class,
                command_id,
            },
            data_size,
            arguments: arguments.to_vec(),
        })
    }

    #[must_use]
    pub fn encode(&self) -> [u8; REPORT_LEN] {
        let mut report = [0_u8; REPORT_LEN];
        report[1] = self.header.transaction_id;
        report[5] = self.data_size;
        report[6] = self.header.command_class;
        report[7] = self.header.command_id;
        let argument_end = 8 + self.arguments.len();
        report[8..argument_end].copy_from_slice(&self.arguments);
        report[88] = xor_checksum(&report);
        report
    }

    #[must_use]
    pub fn encode_hid(&self) -> [u8; HID_REPORT_LEN] {
        let mut hid_report = [0_u8; HID_REPORT_LEN];
        hid_report[0] = REPORT_ID;
        hid_report[1..].copy_from_slice(&self.encode());
        hid_report
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response {
    pub status: u8,
    pub header: CommandHeader,
    pub data_size: u8,
    pub arguments: Vec<u8>,
}

impl Response {
    pub fn decode(report: &[u8], expected: CommandHeader) -> Result<Self, ProtocolError> {
        if report.len() != REPORT_LEN {
            return Err(ProtocolError::WrongReportLength {
                expected: REPORT_LEN,
                observed: report.len(),
            });
        }
        if xor_checksum(report) != report[88] {
            return Err(ProtocolError::ChecksumMismatch {
                expected: xor_checksum(report),
                observed: report[88],
            });
        }
        let observed = CommandHeader {
            transaction_id: report[1],
            command_class: report[6],
            command_id: report[7],
        };
        if observed.transaction_id != expected.transaction_id {
            return Err(ProtocolError::TransactionMismatch {
                expected: expected.transaction_id,
                observed: observed.transaction_id,
            });
        }
        if observed.command_class != expected.command_class
            || observed.command_id != expected.command_id
        {
            return Err(ProtocolError::CommandMismatch { expected, observed });
        }
        let data_size = report[5];
        if usize::from(data_size) > ARGUMENT_LEN {
            return Err(ProtocolError::InvalidDataSize(data_size));
        }
        Ok(Self {
            status: report[0],
            header: observed,
            data_size,
            arguments: report[8..8 + usize::from(data_size)].to_vec(),
        })
    }

    pub fn require_success(&self) -> Result<(), ProtocolError> {
        match self.status {
            STATUS_SUCCESS => Ok(()),
            STATUS_NEW_COMMAND => Err(ProtocolError::NewCommand),
            STATUS_BUSY => Err(ProtocolError::Busy),
            STATUS_FAILURE => Err(ProtocolError::DeviceFailure),
            STATUS_TIMEOUT => Err(ProtocolError::DeviceTimeout),
            STATUS_NOT_SUPPORTED => Err(ProtocolError::NotSupported),
            other => Err(ProtocolError::UnknownStatus(other)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    WrongHidReportLength {
        expected: usize,
        observed: usize,
    },
    ReportIdMismatch {
        expected: u8,
        observed: u8,
    },
    WrongReportLength {
        expected: usize,
        observed: usize,
    },
    InvalidDataSize(u8),
    TooManyArguments(usize),
    ChecksumMismatch {
        expected: u8,
        observed: u8,
    },
    TransactionMismatch {
        expected: u8,
        observed: u8,
    },
    CommandMismatch {
        expected: CommandHeader,
        observed: CommandHeader,
    },
    NewCommand,
    Busy,
    DeviceFailure,
    DeviceTimeout,
    NotSupported,
    UnknownStatus(u8),
    MalformedPayload(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongHidReportLength { expected, observed } => write!(
                formatter,
                "wrong HID report length: expected {expected}, observed {observed}"
            ),
            Self::ReportIdMismatch { expected, observed } => write!(
                formatter,
                "HID report ID mismatch: expected {expected}, observed {observed}"
            ),
            Self::WrongReportLength { expected, observed } => {
                write!(
                    formatter,
                    "wrong report length: expected {expected}, observed {observed}"
                )
            }
            Self::InvalidDataSize(size) => write!(formatter, "invalid report data size {size}"),
            Self::TooManyArguments(count) => {
                write!(formatter, "too many report arguments: {count}")
            }
            Self::ChecksumMismatch { expected, observed } => write!(
                formatter,
                "checksum mismatch: expected 0x{expected:02x}, observed 0x{observed:02x}"
            ),
            Self::TransactionMismatch { expected, observed } => write!(
                formatter,
                "transaction mismatch: expected 0x{expected:02x}, observed 0x{observed:02x}"
            ),
            Self::CommandMismatch { expected, observed } => write!(
                formatter,
                "command mismatch: expected {:02x}/{:02x}, observed {:02x}/{:02x}",
                expected.command_class,
                expected.command_id,
                observed.command_class,
                observed.command_id
            ),
            Self::NewCommand => formatter.write_str("device requested a new command"),
            Self::Busy => formatter.write_str("device is busy"),
            Self::DeviceFailure => formatter.write_str("device rejected the command"),
            Self::DeviceTimeout => formatter.write_str("device reported a timeout"),
            Self::NotSupported => formatter.write_str("device does not support the command"),
            Self::UnknownStatus(status) => {
                write!(formatter, "device returned unknown status 0x{status:02x}")
            }
            Self::MalformedPayload(message) => formatter.write_str(message),
        }
    }
}

impl Error for ProtocolError {}

impl ProtocolError {
    #[must_use]
    pub fn is_retryable_get(&self) -> bool {
        matches!(
            self,
            Self::ChecksumMismatch { .. }
                | Self::TransactionMismatch { .. }
                | Self::CommandMismatch { .. }
                | Self::NewCommand
                | Self::Busy
                | Self::DeviceFailure
                | Self::DeviceTimeout
        )
    }

    /// A BUSY status means the device did not accept the command yet, so the
    /// exact request may be resent. Other write failures remain ambiguous and
    /// must be resolved through independent readback before any resend.
    #[must_use]
    pub const fn should_resend_write(&self, attempt: usize) -> bool {
        matches!(self, Self::Busy) && attempt < WRITE_ATTEMPTS
    }

    /// A validated response for another transaction/command is stale traffic,
    /// not an acknowledgement of the current write. It may be drained without
    /// resending the current request.
    #[must_use]
    pub const fn is_stale_write_ack(&self) -> bool {
        matches!(
            self,
            Self::TransactionMismatch { .. } | Self::CommandMismatch { .. }
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransactionSequence {
    next: u8,
}

impl TransactionSequence {
    pub fn next_id(&mut self) -> u8 {
        let current = self.next;
        self.next = if current >= 0x1e { 0 } else { current + 1 };
        current
    }
}

#[must_use]
pub fn xor_checksum(report: &[u8]) -> u8 {
    report[2..88]
        .iter()
        .fold(0, |checksum, byte| checksum ^ byte)
}

pub fn hid_payload(report: &[u8]) -> Result<&[u8], ProtocolError> {
    if report.len() != HID_REPORT_LEN {
        return Err(ProtocolError::WrongHidReportLength {
            expected: HID_REPORT_LEN,
            observed: report.len(),
        });
    }
    if report[0] != REPORT_ID {
        return Err(ProtocolError::ReportIdMismatch {
            expected: REPORT_ID,
            observed: report[0],
        });
    }
    Ok(&report[1..])
}

#[must_use]
pub fn get_firmware(transaction_id: u8) -> Command {
    Command::new(transaction_id, 0x00, 0x81, 0x02, &[]).expect("fixed command is valid")
}

#[must_use]
pub fn get_serial(transaction_id: u8) -> Command {
    Command::new(transaction_id, 0x00, 0x82, 0x16, &[]).expect("fixed command is valid")
}

#[must_use]
pub fn get_polling(transaction_id: u8, transport: TransportKind) -> Command {
    match transport {
        TransportKind::WiredLegacy => {
            Command::new(transaction_id, 0x00, 0x85, 0x01, &[]).expect("fixed command is valid")
        }
        TransportKind::WirelessV2 => {
            Command::new(transaction_id, 0x00, 0xc0, 0x02, &[PROFILE_ID, 0x00])
                .expect("fixed command is valid")
        }
    }
}

#[must_use]
pub fn set_polling(
    transaction_ids: [u8; 2],
    transport: TransportKind,
    raw_code: u8,
) -> Vec<Command> {
    match transport {
        TransportKind::WiredLegacy => vec![
            Command::new(transaction_ids[0], 0x00, 0x05, 0x01, &[raw_code])
                .expect("fixed command is valid"),
        ],
        TransportKind::WirelessV2 => [0x00, 0x01]
            .into_iter()
            .zip(transaction_ids)
            .map(|(selector, transaction_id)| {
                Command::new(transaction_id, 0x00, 0x40, 0x02, &[selector, raw_code])
                    .expect("fixed command is valid")
            })
            .collect(),
    }
}

#[must_use]
pub fn get_current_dpi(transaction_id: u8) -> Command {
    Command::new(transaction_id, 0x04, 0x85, 0x07, &[PROFILE_ID]).expect("fixed command is valid")
}

#[must_use]
pub fn set_current_dpi(transaction_id: u8, dpi: DpiPair) -> Command {
    let [x_high, x_low] = dpi.x.to_be_bytes();
    let [y_high, y_low] = dpi.y.to_be_bytes();
    Command::new(
        transaction_id,
        0x04,
        0x05,
        0x07,
        &[PROFILE_ID, x_high, x_low, y_high, y_low, 0, 0],
    )
    .expect("fixed command is valid")
}

#[must_use]
pub fn get_dpi_stages(transaction_id: u8) -> Command {
    Command::new(transaction_id, 0x04, 0x86, 0x50, &[PROFILE_ID]).expect("fixed command is valid")
}

/// Reads the wireless sleep timeout. `ClickSync` command 0x07/0x83.
#[must_use]
pub fn get_idle_timeout(transaction_id: u8) -> Command {
    Command::new(transaction_id, 0x07, 0x83, 0x02, &[]).expect("fixed command is valid")
}

/// Reads the low-power threshold. `ClickSync` command 0x07/0x81.
#[must_use]
pub fn get_low_power_threshold(transaction_id: u8) -> Command {
    Command::new(transaction_id, 0x07, 0x81, 0x01, &[]).expect("fixed command is valid")
}

/// Reads the current battery level. `ClickSync` command 0x07/0x80.
#[must_use]
pub fn get_battery_level(transaction_id: u8) -> Command {
    Command::new(transaction_id, 0x07, 0x80, 0x02, &[]).expect("fixed command is valid")
}

pub fn set_dpi_stages(
    transaction_id: u8,
    active_stage_id: u8,
    stages: &[DpiStage],
) -> Result<Command, ProtocolError> {
    if stages.is_empty() || stages.len() > 11 {
        return Err(ProtocolError::MalformedPayload(format!(
            "DPI stage count {} is outside 1..=11",
            stages.len()
        )));
    }
    let mut arguments = Vec::with_capacity(3 + stages.len() * 7);
    arguments.extend_from_slice(&[
        PROFILE_ID,
        active_stage_id,
        u8::try_from(stages.len()).expect("stage count was bounded to eleven"),
    ]);
    for stage in stages {
        let [x_high, x_low] = stage.x.to_be_bytes();
        let [y_high, y_low] = stage.y.to_be_bytes();
        arguments.extend_from_slice(&[stage.id, x_high, x_low, y_high, y_low, 0, 0]);
    }
    Command::new(
        transaction_id,
        0x04,
        0x06,
        u8::try_from(arguments.len()).expect("stage arguments fit in a report"),
        &arguments,
    )
}

#[must_use]
pub fn get_button_assignment(transaction_id: u8, protocol_button_id: u8) -> Command {
    Command::new(
        transaction_id,
        0x02,
        0x8c,
        0x50,
        &[PROFILE_ID, protocol_button_id, 0x00],
    )
    .expect("fixed command is valid")
}

pub fn set_button_assignment(
    transaction_id: u8,
    assignment: &RawButtonAssignment,
) -> Result<Command, ProtocolError> {
    assignment
        .validate()
        .map_err(ProtocolError::MalformedPayload)?;
    let mut arguments = [0_u8; 10];
    arguments[0] = assignment.profile_id;
    arguments[1] = assignment.protocol_button_id;
    arguments[2] = assignment.mode;
    arguments[3] = assignment.function_id;
    arguments[4] = assignment.data_size;
    arguments[5..].copy_from_slice(&assignment.data);
    Command::new(transaction_id, 0x02, 0x0c, 0x50, &arguments)
}

pub fn parse_firmware(response: &Response) -> Result<FirmwareVersion, ProtocolError> {
    require_arguments(response, 2, "firmware")?;
    Ok(FirmwareVersion {
        major: response.arguments[0],
        minor: response.arguments[1],
    })
}

pub fn parse_serial(response: &Response) -> Result<String, ProtocolError> {
    let end = response
        .arguments
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(response.arguments.len());
    if end == 0 {
        return Err(ProtocolError::MalformedPayload(
            "device returned an empty serial number".to_owned(),
        ));
    }
    let serial = std::str::from_utf8(&response.arguments[..end]).map_err(|error| {
        ProtocolError::MalformedPayload(format!("device serial is not valid UTF-8: {error}"))
    })?;
    Ok(serial.to_owned())
}

pub fn parse_polling(response: &Response, transport: TransportKind) -> Result<u8, ProtocolError> {
    let index = match transport {
        TransportKind::WiredLegacy => 0,
        TransportKind::WirelessV2 => 1,
    };
    require_arguments(response, index + 1, "polling")?;
    Ok(response.arguments[index])
}

pub fn parse_current_dpi(response: &Response) -> Result<DpiPair, ProtocolError> {
    require_arguments(response, 5, "current DPI")?;
    if response.arguments[0] != PROFILE_ID {
        return Err(ProtocolError::MalformedPayload(format!(
            "unexpected DPI profile ID 0x{:02x}",
            response.arguments[0]
        )));
    }
    Ok(DpiPair {
        x: u16::from_be_bytes([response.arguments[1], response.arguments[2]]),
        y: u16::from_be_bytes([response.arguments[3], response.arguments[4]]),
    })
}

pub fn parse_dpi_stages(response: &Response) -> Result<(u8, Vec<DpiStage>), ProtocolError> {
    require_arguments(response, 3, "DPI stages")?;
    if response.arguments[0] != PROFILE_ID {
        return Err(ProtocolError::MalformedPayload(format!(
            "unexpected DPI stage profile ID 0x{:02x}",
            response.arguments[0]
        )));
    }
    let active_stage_id = response.arguments[1];
    let count = usize::from(response.arguments[2]);
    if count == 0 || count > 11 {
        return Err(ProtocolError::MalformedPayload(format!(
            "invalid DPI stage count {count}"
        )));
    }
    require_arguments(response, 3 + count * 7, "DPI stages")?;
    let mut stages = Vec::with_capacity(count);
    for chunk in response.arguments[3..3 + count * 7].as_chunks::<7>().0 {
        stages.push(DpiStage {
            id: chunk[0],
            x: u16::from_be_bytes([chunk[1], chunk[2]]),
            y: u16::from_be_bytes([chunk[3], chunk[4]]),
        });
    }
    if !stages.iter().any(|stage| stage.id == active_stage_id) {
        return Err(ProtocolError::MalformedPayload(format!(
            "active DPI stage {active_stage_id} is absent from the stage table"
        )));
    }
    Ok((active_stage_id, stages))
}

pub fn parse_power_settings(
    battery_response: &Response,
    idle_response: &Response,
    threshold_response: &Response,
) -> Result<WirelessPowerSettings, ProtocolError> {
    require_arguments(battery_response, 2, "battery level")?;
    require_arguments(idle_response, 2, "idle timeout")?;
    require_arguments(threshold_response, 1, "low-power threshold")?;
    let idle_seconds = u16::from_be_bytes([idle_response.arguments[0], idle_response.arguments[1]]);
    if !(60..=900).contains(&idle_seconds) {
        return Err(ProtocolError::MalformedPayload(format!(
            "idle timeout {idle_seconds}s is outside Razer's 60..=900s range"
        )));
    }
    let low_power_threshold_raw = threshold_response.arguments[0];
    if low_power_threshold_raw < 0x0d {
        return Err(ProtocolError::MalformedPayload(format!(
            "low-power threshold 0x{low_power_threshold_raw:02x} is below Razer's minimum"
        )));
    }
    Ok(WirelessPowerSettings {
        battery_raw: battery_response.arguments[1],
        idle_seconds,
        low_power_threshold_raw,
    })
}

pub fn parse_button_assignment(
    response: &Response,
    expected_button_id: u8,
) -> Result<RawButtonAssignment, ProtocolError> {
    require_arguments(response, 10, "button assignment")?;
    if response.arguments[0] != PROFILE_ID || response.arguments[1] != expected_button_id {
        return Err(ProtocolError::MalformedPayload(format!(
            "button response identity mismatch: requested profile/button 01/{expected_button_id:02x}, observed {:02x}/{:02x} (returned mode {:02x})",
            response.arguments[0], response.arguments[1], response.arguments[2]
        )));
    }
    let data_size = response.arguments[4];
    if data_size > 5 {
        return Err(ProtocolError::MalformedPayload(format!(
            "button 0x{expected_button_id:02x} returned invalid data size {data_size}"
        )));
    }
    let mut data = [0_u8; 5];
    data.copy_from_slice(&response.arguments[5..10]);
    Ok(RawButtonAssignment {
        profile_id: response.arguments[0],
        protocol_button_id: response.arguments[1],
        mode: response.arguments[2],
        function_id: response.arguments[3],
        data_size,
        data,
    })
}

fn require_arguments(
    response: &Response,
    minimum: usize,
    description: &str,
) -> Result<(), ProtocolError> {
    if response.arguments.len() < minimum {
        return Err(ProtocolError::MalformedPayload(format!(
            "{description} response has {} bytes, expected at least {minimum}",
            response.arguments.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BUTTON_MOUSE4_SLOT, HID_USAGE_F10, RawButtonAssignment};

    fn successful_response(command: &Command, arguments: &[u8]) -> [u8; REPORT_LEN] {
        let mut report = command.encode();
        report[0] = STATUS_SUCCESS;
        report[5] = u8::try_from(arguments.len()).expect("test payload fits");
        report[8..8 + arguments.len()].copy_from_slice(arguments);
        report[88] = xor_checksum(&report);
        report
    }

    #[test]
    fn clicksync_polling_vectors_are_stable() {
        let get = get_polling(0x0a, TransportKind::WirelessV2).encode();
        assert_eq!(&get[0..10], &[0, 0x0a, 0, 0, 0, 2, 0, 0xc0, 1, 0]);
        assert_eq!(get[88], 0xc3);
        assert_eq!(get[89], 0);

        let set = set_polling([0x0b, 0x0c], TransportKind::WirelessV2, 0x02);
        assert_eq!(&set[0].encode()[5..10], &[2, 0, 0x40, 0, 2]);
        assert_eq!(&set[1].encode()[5..10], &[2, 0, 0x40, 1, 2]);
        assert_ne!(set[0].header.transaction_id, set[1].header.transaction_id);
    }

    #[test]
    fn clicksync_keyboard_vector_is_stable() {
        let assignment = RawButtonAssignment::keyboard_chord(BUTTON_MOUSE4_SLOT, HID_USAGE_F10);
        let report = set_button_assignment(3, &assignment)
            .expect("valid assignment")
            .encode();
        assert_eq!(
            &report[5..18],
            &[0x50, 0x02, 0x0c, 1, 4, 0, 2, 2, 5, 0x43, 0, 0, 0]
        );
        assert_eq!(report[88], xor_checksum(&report));
    }

    #[test]
    fn current_dpi_vector_is_stable() {
        let report = set_current_dpi(7, DpiPair { x: 1600, y: 1600 }).encode();
        assert_eq!(
            &report[5..15],
            &[0x07, 0x04, 0x05, 0x01, 0x06, 0x40, 0x06, 0x40, 0, 0]
        );
    }

    #[test]
    fn transaction_sequence_wraps_after_thirty() {
        let mut sequence = TransactionSequence::default();
        let observed: Vec<u8> = (0..33).map(|_| sequence.next_id()).collect();
        assert_eq!(&observed[..31], &(0_u8..=30).collect::<Vec<_>>());
        assert_eq!(&observed[31..], &[0, 1]);
    }

    #[test]
    fn rejects_corrupt_or_mismatched_responses() {
        let command = get_firmware(9);
        let mut report = successful_response(&command, &[1, 2]);
        report[12] ^= 0xff;
        assert!(matches!(
            Response::decode(&report, command.header),
            Err(ProtocolError::ChecksumMismatch { .. })
        ));

        let report = successful_response(&command, &[1, 2]);
        let wrong_header = CommandHeader {
            transaction_id: 10,
            ..command.header
        };
        assert!(matches!(
            Response::decode(&report, wrong_header),
            Err(ProtocolError::TransactionMismatch { .. })
        ));
    }

    #[test]
    fn parses_and_preserves_unknown_button_assignment() {
        let command = get_button_assignment(4, BUTTON_MOUSE4_SLOT);
        let report = successful_response(
            &command,
            &[1, BUTTON_MOUSE4_SLOT, 1, 0xfe, 5, 1, 2, 3, 4, 5],
        );
        let response = Response::decode(&report, command.header).expect("valid response");
        let parsed = parse_button_assignment(&response, BUTTON_MOUSE4_SLOT)
            .expect("valid assignment payload");
        assert_eq!(parsed.function_id, 0xfe);
        assert_eq!(parsed.mode, 1);
        assert_eq!(parsed.data, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn stage_table_round_trips_without_reordering_ids() {
        let stages = vec![
            DpiStage {
                id: 3,
                x: 800,
                y: 900,
            },
            DpiStage {
                id: 7,
                x: 1600,
                y: 1700,
            },
        ];
        let command = set_dpi_stages(2, 7, &stages).expect("valid stages");
        assert_eq!(command.arguments[0..3], [1, 7, 2]);
        assert_eq!(command.arguments[3], 3);
        assert_eq!(command.arguments[10], 7);
    }

    #[test]
    fn read_only_power_commands_match_clicksync_vectors() {
        let battery = get_battery_level(0x09).encode();
        assert_eq!(battery[1], 0x09);
        assert_eq!(battery[5..8], [0x02, 0x07, 0x80]);

        let idle = get_idle_timeout(0x0a).encode();
        assert_eq!(idle[1], 0x0a);
        assert_eq!(idle[5..8], [0x02, 0x07, 0x83]);

        let threshold = get_low_power_threshold(0x0b).encode();
        assert_eq!(threshold[1], 0x0b);
        assert_eq!(threshold[5..8], [0x01, 0x07, 0x81]);
    }

    #[test]
    fn power_settings_parse_the_measured_razer_values() {
        let idle_command = get_idle_timeout(1);
        let threshold_command = get_low_power_threshold(2);
        let battery_command = get_battery_level(3);
        let battery_report = successful_response(&battery_command, &[0x00, 0xcc]);
        let idle_report = successful_response(&idle_command, &[0x00, 0xb4]);
        let threshold_report = successful_response(&threshold_command, &[0x0d]);
        let idle = Response::decode(&idle_report, idle_command.header).expect("idle frame");
        let threshold =
            Response::decode(&threshold_report, threshold_command.header).expect("threshold frame");
        let battery =
            Response::decode(&battery_report, battery_command.header).expect("battery frame");
        let power =
            parse_power_settings(&battery, &idle, &threshold).expect("valid power settings");
        assert_eq!(power.battery_percent(), 80);
        assert_eq!(power.idle_seconds, 180);
        assert_eq!(power.idle_minutes(), 3);
        assert_eq!(power.low_power_threshold_raw, 0x0d);
        assert_eq!(power.low_power_percent(), 5);
    }

    #[test]
    fn status_codes_fail_closed() {
        let command = get_firmware(1);
        for (status, expected) in [
            (STATUS_NEW_COMMAND, ProtocolError::NewCommand),
            (STATUS_BUSY, ProtocolError::Busy),
            (STATUS_FAILURE, ProtocolError::DeviceFailure),
            (STATUS_TIMEOUT, ProtocolError::DeviceTimeout),
            (STATUS_NOT_SUPPORTED, ProtocolError::NotSupported),
            (0xff, ProtocolError::UnknownStatus(0xff)),
        ] {
            let mut report = successful_response(&command, &[1, 2]);
            report[0] = status;
            report[88] = xor_checksum(&report);
            let response = Response::decode(&report, command.header).expect("valid frame");
            assert_eq!(response.require_success(), Err(expected));
        }
    }

    #[test]
    fn only_source_backed_transient_get_errors_are_retryable() {
        assert!(ProtocolError::NewCommand.is_retryable_get());
        assert!(ProtocolError::Busy.is_retryable_get());
        assert!(ProtocolError::DeviceFailure.is_retryable_get());
        assert!(ProtocolError::DeviceTimeout.is_retryable_get());
        assert!(
            ProtocolError::TransactionMismatch {
                expected: 1,
                observed: 2,
            }
            .is_retryable_get()
        );
        assert!(!ProtocolError::NotSupported.is_retryable_get());
        assert!(!ProtocolError::UnknownStatus(0xff).is_retryable_get());
        assert!(!ProtocolError::MalformedPayload("bad payload".to_owned()).is_retryable_get());
    }

    #[test]
    fn only_busy_is_a_safe_immediate_write_retry() {
        for attempt in 1..WRITE_ATTEMPTS {
            assert!(ProtocolError::Busy.should_resend_write(attempt));
        }
        assert!(!ProtocolError::Busy.should_resend_write(WRITE_ATTEMPTS));
        assert!(!ProtocolError::DeviceFailure.should_resend_write(1));
        assert!(!ProtocolError::DeviceTimeout.should_resend_write(1));
        assert!(!ProtocolError::NotSupported.should_resend_write(1));
    }

    #[test]
    fn only_validated_response_mismatches_are_stale_write_acks() {
        assert!(
            ProtocolError::TransactionMismatch {
                expected: 0x18,
                observed: 0x17,
            }
            .is_stale_write_ack()
        );
        assert!(
            ProtocolError::CommandMismatch {
                expected: CommandHeader {
                    transaction_id: 0x18,
                    command_class: 0,
                    command_id: 0x40,
                },
                observed: CommandHeader {
                    transaction_id: 0x17,
                    command_class: 2,
                    command_id: 0x0c,
                },
            }
            .is_stale_write_ack()
        );
        assert!(
            !ProtocolError::ChecksumMismatch {
                expected: 1,
                observed: 2,
            }
            .is_stale_write_ack()
        );
        assert!(!ProtocolError::Busy.is_stale_write_ack());
    }

    #[test]
    fn hid_report_id_is_validated_before_packet_decode() {
        let command = get_firmware(1);
        let mut report = command.encode_hid();
        report[0] = 7;
        assert_eq!(
            hid_payload(&report),
            Err(ProtocolError::ReportIdMismatch {
                expected: 0,
                observed: 7,
            })
        );
        assert!(matches!(
            hid_payload(&report[..REPORT_LEN]),
            Err(ProtocolError::WrongHidReportLength { .. })
        ));
    }
}
