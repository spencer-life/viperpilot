use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model::{
    BUTTON_MOUSE4_SLOT, BUTTON_MOUSE5_SLOT, DeviceSnapshotV1, DpiPair, DpiStage, ProfileName,
    ProfileSpec, RawButtonAssignment, TransportKind,
};
use crate::protocol;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TargetState {
    pub profile: Option<ProfileName>,
    pub dpi: DpiPair,
    pub dpi_active_stage_id: Option<u8>,
    pub dpi_stages: Option<Vec<DpiStage>>,
    pub polling_raw_code: u8,
    pub polling_hz: Option<u16>,
    pub button_assignments: Vec<RawButtonAssignment>,
    #[serde(default)]
    pub button_modes_ignored: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SkippedField {
    pub field: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WriteOperation {
    SetDpiStages {
        before_active_stage_id: u8,
        before_stages: Vec<DpiStage>,
        after_active_stage_id: u8,
        after_stages: Vec<DpiStage>,
    },
    SetCurrentDpi {
        before: DpiPair,
        after: DpiPair,
        require_active_stage_match: bool,
        #[serde(default)]
        force_write: bool,
    },
    SetPolling {
        transport: TransportKind,
        before_raw_code: u8,
        after_raw_code: u8,
    },
    SetButton {
        before: RawButtonAssignment,
        after: RawButtonAssignment,
        /// Ignore only the opaque GET-returned mode byte; profile, slot,
        /// function, size, and all five data bytes remain mandatory.
        #[serde(default)]
        ignore_returned_mode: bool,
    },
}

impl WriteOperation {
    #[must_use]
    pub fn field_name(&self) -> String {
        match self {
            Self::SetDpiStages { .. } => "dpi.stages".to_owned(),
            Self::SetCurrentDpi { .. } => "dpi.current".to_owned(),
            Self::SetPolling { .. } => "polling".to_owned(),
            Self::SetButton { after, .. } => {
                format!("button.0x{:02x}", after.protocol_button_id)
            }
        }
    }

    #[must_use]
    pub fn rollback(&self) -> Self {
        match self {
            Self::SetDpiStages {
                before_active_stage_id,
                before_stages,
                after_active_stage_id,
                after_stages,
            } => Self::SetDpiStages {
                before_active_stage_id: *after_active_stage_id,
                before_stages: after_stages.clone(),
                after_active_stage_id: *before_active_stage_id,
                after_stages: before_stages.clone(),
            },
            Self::SetCurrentDpi {
                before,
                after,
                require_active_stage_match,
                ..
            } => Self::SetCurrentDpi {
                before: *after,
                after: *before,
                require_active_stage_match: *require_active_stage_match,
                force_write: false,
            },
            Self::SetPolling {
                transport,
                before_raw_code,
                after_raw_code,
            } => Self::SetPolling {
                transport: *transport,
                before_raw_code: *after_raw_code,
                after_raw_code: *before_raw_code,
            },
            Self::SetButton {
                before,
                after,
                ignore_returned_mode,
            } => Self::SetButton {
                before: after.clone(),
                after: before.clone(),
                ignore_returned_mode: *ignore_returned_mode,
            },
        }
    }

    #[must_use]
    pub fn force_write(&self) -> bool {
        matches!(
            self,
            Self::SetCurrentDpi {
                force_write: true,
                ..
            }
        )
    }

    #[must_use]
    pub fn matches(&self, snapshot: &DeviceSnapshotV1) -> bool {
        match self {
            Self::SetDpiStages {
                after_active_stage_id,
                after_stages,
                ..
            } => {
                snapshot.dpi.active_stage_id == *after_active_stage_id
                    && snapshot.dpi.stages == *after_stages
            }
            Self::SetCurrentDpi {
                after,
                require_active_stage_match,
                ..
            } => {
                snapshot.dpi.current == *after
                    && (!require_active_stage_match
                        || snapshot
                            .dpi
                            .active_stage()
                            .is_some_and(|stage| stage.x == after.x && stage.y == after.y))
            }
            Self::SetPolling { after_raw_code, .. } => snapshot.polling.raw_code == *after_raw_code,
            Self::SetButton {
                after,
                ignore_returned_mode,
                ..
            } => snapshot
                .button(after.protocol_button_id)
                .is_some_and(|observed| {
                    button_assignment_matches(observed, after, *ignore_returned_mode)
                }),
        }
    }

    /// Return whether this field is unchanged from the last trusted snapshot.
    /// The reference snapshot advances only after a complete, independently
    /// validated readback, allowing later rollback steps to account for known
    /// side effects from earlier successful setters.
    #[must_use]
    pub fn matches_before(&self, observed: &DeviceSnapshotV1, expected: &DeviceSnapshotV1) -> bool {
        match self {
            Self::SetDpiStages { .. } => {
                observed.dpi.active_stage_id == expected.dpi.active_stage_id
                    && observed.dpi.stages == expected.dpi.stages
            }
            Self::SetCurrentDpi { .. } => {
                observed.dpi.current == expected.dpi.current
                    && observed.dpi.active_stage_id == expected.dpi.active_stage_id
                    && observed.dpi.active_stage() == expected.dpi.active_stage()
            }
            Self::SetPolling { .. } => {
                observed.polling.transport == expected.polling.transport
                    && observed.polling.raw_code == expected.polling.raw_code
            }
            Self::SetButton {
                before,
                ignore_returned_mode,
                ..
            } => observed
                .button(before.protocol_button_id)
                .zip(expected.button(before.protocol_button_id))
                .is_some_and(|(observed, expected)| {
                    button_assignment_matches(observed, expected, *ignore_returned_mode)
                }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WritePlan {
    pub before: DeviceSnapshotV1,
    pub target: TargetState,
    pub skipped_fields: Vec<SkippedField>,
    pub ordered_operations: Vec<WriteOperation>,
    pub rollback_operations: Vec<WriteOperation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PacketIntent {
    pub field: String,
    pub physical_report_index: u8,
    pub command_class: u8,
    pub command_id: u8,
    pub data_size: u8,
    pub arguments: Vec<u8>,
    pub note: String,
}

impl WritePlan {
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.ordered_operations.is_empty()
    }

    pub fn packet_intents(&self) -> Result<Vec<PacketIntent>, crate::protocol::ProtocolError> {
        let mut intents = Vec::new();
        for operation in &self.ordered_operations {
            let commands = commands_for_intent(operation)?;
            for (index, command) in commands.into_iter().enumerate() {
                intents.push(PacketIntent {
                    field: operation.field_name(),
                    physical_report_index: u8::try_from(index + 1)
                        .expect("an operation has at most two reports"),
                    command_class: command.header.command_class,
                    command_id: command.header.command_id,
                    data_size: command.data_size,
                    arguments: command.arguments,
                    note: "transaction ID and checksum are assigned immediately before send"
                        .to_owned(),
                });
            }
        }
        Ok(intents)
    }
}

fn commands_for_intent(
    operation: &WriteOperation,
) -> Result<Vec<protocol::Command>, protocol::ProtocolError> {
    match operation {
        WriteOperation::SetDpiStages {
            after_active_stage_id,
            after_stages,
            ..
        } => Ok(vec![protocol::set_dpi_stages(
            0,
            *after_active_stage_id,
            after_stages,
        )?]),
        WriteOperation::SetCurrentDpi { after, .. } => {
            Ok(vec![protocol::set_current_dpi(0, *after)])
        }
        WriteOperation::SetPolling {
            transport,
            after_raw_code,
            ..
        } => Ok(protocol::set_polling([0, 1], *transport, *after_raw_code)),
        WriteOperation::SetButton { after, .. } => {
            Ok(vec![protocol::set_button_assignment(0, after)?])
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    InvalidSnapshot(String),
    UnsupportedTarget(String),
    MissingButton(u8),
    IdentityMismatch(String),
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSnapshot(message)
            | Self::UnsupportedTarget(message)
            | Self::IdentityMismatch(message) => formatter.write_str(message),
            Self::MissingButton(button_id) => write!(
                formatter,
                "snapshot is missing protocol button 0x{button_id:02x}"
            ),
        }
    }
}

impl Error for PlanError {}

pub fn plan_profile(
    before: &DeviceSnapshotV1,
    profile: ProfileName,
) -> Result<WritePlan, PlanError> {
    before.validate().map_err(PlanError::InvalidSnapshot)?;
    let spec = ProfileSpec::fixed(profile);
    plan_profile_with_assignments(before, &spec, &spec.side_buttons)
}

pub fn plan_profile_with_baseline(
    before: &DeviceSnapshotV1,
    baseline: &DeviceSnapshotV1,
    profile: ProfileName,
) -> Result<WritePlan, PlanError> {
    before.validate().map_err(PlanError::InvalidSnapshot)?;
    baseline.validate().map_err(PlanError::InvalidSnapshot)?;
    require_same_identity(before, baseline)?;
    let spec = ProfileSpec::fixed(profile);
    let assignments = match profile {
        ProfileName::Developer => spec.side_buttons.clone(),
        ProfileName::Gaming => gaming_assignments_from_baseline(baseline)?,
    };
    plan_profile_with_assignments(before, &spec, &assignments)
}

fn plan_profile_with_assignments(
    before: &DeviceSnapshotV1,
    spec: &ProfileSpec,
    side_buttons: &[RawButtonAssignment],
) -> Result<WritePlan, PlanError> {
    let polling_code = spec
        .polling_code(before.polling.transport)
        .map_err(PlanError::UnsupportedTarget)?;
    let mut expected_stages = before.dpi.stages.clone();
    let active_stage = expected_stages
        .iter_mut()
        .find(|stage| stage.id == before.dpi.active_stage_id)
        .ok_or_else(|| {
            PlanError::InvalidSnapshot("active DPI stage disappeared during planning".to_owned())
        })?;
    active_stage.x = spec.dpi.x;
    active_stage.y = spec.dpi.y;
    let mut expected_assignments = before.button_assignments.clone();
    for target_assignment in side_buttons {
        replace_assignment(&mut expected_assignments, target_assignment)?;
    }
    let target = TargetState {
        profile: Some(spec.name),
        dpi: spec.dpi,
        dpi_active_stage_id: Some(before.dpi.active_stage_id),
        dpi_stages: Some(expected_stages),
        polling_raw_code: polling_code,
        polling_hz: Some(spec.polling_hz),
        button_assignments: expected_assignments,
        button_modes_ignored: vec![BUTTON_MOUSE4_SLOT, BUTTON_MOUSE5_SLOT],
    };
    let mut skipped_fields = Vec::new();
    let mut operations = Vec::new();

    let active_stage_matches = before
        .dpi
        .active_stage()
        .is_some_and(|stage| stage.x == spec.dpi.x && stage.y == spec.dpi.y);
    if before.dpi.current == spec.dpi && active_stage_matches {
        skipped_fields.push(skipped("dpi", "current and active stage already match"));
    } else {
        operations.push(WriteOperation::SetCurrentDpi {
            before: before.dpi.current,
            after: spec.dpi,
            require_active_stage_match: true,
            force_write: false,
        });
    }

    if before.polling.raw_code == polling_code {
        skipped_fields.push(skipped("polling", "raw polling code already matches"));
    } else {
        operations.push(WriteOperation::SetPolling {
            transport: before.polling.transport,
            before_raw_code: before.polling.raw_code,
            after_raw_code: polling_code,
        });
    }

    for target_assignment in side_buttons {
        let current =
            before
                .button(target_assignment.protocol_button_id)
                .ok_or(PlanError::MissingButton(
                    target_assignment.protocol_button_id,
                ))?;
        // ClickSync's official OBM path always writes NORMAL mode and verifies
        // the action. This mouse likewise proved that the returned mode byte is
        // opaque for side slots, while function/size/data round-trip exactly.
        if button_assignment_matches(current, target_assignment, true) {
            skipped_fields.push(skipped(
                &format!("button.0x{:02x}", target_assignment.protocol_button_id),
                "raw assignment already matches",
            ));
        } else {
            let mut write_before = current.clone();
            write_before.mode = crate::model::NORMAL_BUTTON_MODE;
            let mut write_after = target_assignment.clone();
            write_after.mode = crate::model::NORMAL_BUTTON_MODE;
            operations.push(WriteOperation::SetButton {
                before: write_before,
                after: write_after,
                ignore_returned_mode: true,
            });
        }
    }

    Ok(finish_plan(before, target, skipped_fields, operations))
}

pub fn plan_dpi_1600_only(
    before: &DeviceSnapshotV1,
    force_write: bool,
) -> Result<WritePlan, PlanError> {
    before.validate().map_err(PlanError::InvalidSnapshot)?;
    let target_dpi = DpiPair { x: 1600, y: 1600 };
    let mut expected_stages = before.dpi.stages.clone();
    let active_stage = expected_stages
        .iter_mut()
        .find(|stage| stage.id == before.dpi.active_stage_id)
        .ok_or_else(|| {
            PlanError::InvalidSnapshot("active DPI stage disappeared during planning".to_owned())
        })?;
    active_stage.x = target_dpi.x;
    active_stage.y = target_dpi.y;
    let already_matches = before.dpi.current == target_dpi
        && before
            .dpi
            .active_stage()
            .is_some_and(|stage| stage.x == target_dpi.x && stage.y == target_dpi.y);
    let target = TargetState {
        profile: None,
        dpi: target_dpi,
        dpi_active_stage_id: Some(before.dpi.active_stage_id),
        dpi_stages: Some(expected_stages),
        polling_raw_code: before.polling.raw_code,
        polling_hz: before.polling.hertz(),
        button_assignments: before.button_assignments.clone(),
        button_modes_ignored: Vec::new(),
    };
    let mut skipped_fields = vec![
        skipped("polling", "preserved by the DPI-only proof"),
        skipped(
            "button_assignments",
            "all six raw assignments are preserved by the DPI-only proof",
        ),
    ];
    let operations = if already_matches && !force_write {
        skipped_fields.push(skipped("dpi", "current and active stage already match"));
        Vec::new()
    } else {
        vec![WriteOperation::SetCurrentDpi {
            before: before.dpi.current,
            after: target_dpi,
            require_active_stage_match: true,
            force_write,
        }]
    };
    Ok(finish_plan(before, target, skipped_fields, operations))
}

pub fn plan_button_only(
    before: &DeviceSnapshotV1,
    baseline: &DeviceSnapshotV1,
    profile: ProfileName,
    protocol_button_id: u8,
) -> Result<WritePlan, PlanError> {
    before.validate().map_err(PlanError::InvalidSnapshot)?;
    baseline.validate().map_err(PlanError::InvalidSnapshot)?;
    require_same_identity(before, baseline)?;
    if !matches!(protocol_button_id, BUTTON_MOUSE4_SLOT | BUTTON_MOUSE5_SLOT) {
        return Err(PlanError::UnsupportedTarget(format!(
            "button-only proof is restricted to protocol slots 0x{BUTTON_MOUSE4_SLOT:02x} and 0x{BUTTON_MOUSE5_SLOT:02x}"
        )));
    }
    let target_assignment = match profile {
        ProfileName::Developer => ProfileSpec::fixed(profile)
            .side_buttons
            .into_iter()
            .find(|assignment| assignment.protocol_button_id == protocol_button_id)
            .ok_or(PlanError::MissingButton(protocol_button_id))?,
        ProfileName::Gaming => gaming_assignments_from_baseline(baseline)?
            .into_iter()
            .find(|assignment| assignment.protocol_button_id == protocol_button_id)
            .ok_or(PlanError::MissingButton(protocol_button_id))?,
    };
    let current = before
        .button(protocol_button_id)
        .ok_or(PlanError::MissingButton(protocol_button_id))?;
    let mut expected_assignments = before.button_assignments.clone();
    replace_assignment(&mut expected_assignments, &target_assignment)?;
    let target = TargetState {
        profile: None,
        dpi: before.dpi.current,
        dpi_active_stage_id: Some(before.dpi.active_stage_id),
        dpi_stages: Some(before.dpi.stages.clone()),
        polling_raw_code: before.polling.raw_code,
        polling_hz: before.polling.hertz(),
        button_assignments: expected_assignments,
        button_modes_ignored: vec![protocol_button_id],
    };
    let mut skipped_fields = vec![
        skipped("dpi", "preserved by the button-only proof"),
        skipped("polling", "preserved by the button-only proof"),
        skipped(
            "other_button_assignments",
            "the other five raw assignments are preserved by the button-only proof",
        ),
    ];
    let operations = if button_assignment_matches(current, &target_assignment, true) {
        skipped_fields.push(skipped(
            &format!("button.0x{protocol_button_id:02x}"),
            "raw assignment already matches",
        ));
        Vec::new()
    } else {
        let mut write_before = current.clone();
        write_before.mode = crate::model::NORMAL_BUTTON_MODE;
        let mut write_after = target_assignment;
        write_after.mode = crate::model::NORMAL_BUTTON_MODE;
        vec![WriteOperation::SetButton {
            before: write_before,
            after: write_after,
            ignore_returned_mode: true,
        }]
    };
    Ok(finish_plan(before, target, skipped_fields, operations))
}

pub fn require_clean_button_proof_start(
    before: &DeviceSnapshotV1,
    baseline: &DeviceSnapshotV1,
    profile: ProfileName,
    protocol_button_id: u8,
) -> Result<(), PlanError> {
    before.validate().map_err(PlanError::InvalidSnapshot)?;
    baseline.validate().map_err(PlanError::InvalidSnapshot)?;
    require_same_identity(before, baseline)?;
    if !matches!(protocol_button_id, BUTTON_MOUSE4_SLOT | BUTTON_MOUSE5_SLOT) {
        return Err(PlanError::UnsupportedTarget(format!(
            "button-only proof is restricted to protocol slots 0x{BUTTON_MOUSE4_SLOT:02x} and 0x{BUTTON_MOUSE5_SLOT:02x}"
        )));
    }
    let mut normalized = before.clone();
    if profile == ProfileName::Gaming {
        let baseline_assignment = baseline
            .button(protocol_button_id)
            .ok_or(PlanError::MissingButton(protocol_button_id))?;
        replace_assignment(&mut normalized.button_assignments, baseline_assignment)?;
    }
    let restore = plan_restore(&normalized, baseline)?;
    if restore.is_noop() {
        Ok(())
    } else {
        Err(PlanError::UnsupportedTarget(match profile {
            ProfileName::Developer => {
                "Developer one-button proof must start from the complete immutable baseline"
                    .to_owned()
            }
            ProfileName::Gaming => format!(
                "Gaming one-button restore permits only protocol button 0x{protocol_button_id:02x} to differ from the immutable baseline"
            ),
        }))
    }
}

pub fn plan_polling_only(
    before: &DeviceSnapshotV1,
    polling_hz: u16,
) -> Result<WritePlan, PlanError> {
    before.validate().map_err(PlanError::InvalidSnapshot)?;
    let profile_for_code = match polling_hz {
        1000 => ProfileName::Developer,
        4000 => ProfileName::Gaming,
        other => {
            return Err(PlanError::UnsupportedTarget(format!(
                "polling-only proof supports exactly 1000 or 4000 Hz, not {other} Hz"
            )));
        }
    };
    let polling_code = ProfileSpec::fixed(profile_for_code)
        .polling_code(before.polling.transport)
        .map_err(PlanError::UnsupportedTarget)?;
    let target = TargetState {
        profile: None,
        dpi: before.dpi.current,
        dpi_active_stage_id: Some(before.dpi.active_stage_id),
        dpi_stages: Some(before.dpi.stages.clone()),
        polling_raw_code: polling_code,
        polling_hz: Some(polling_hz),
        button_assignments: before.button_assignments.clone(),
        button_modes_ignored: Vec::new(),
    };
    let mut skipped_fields = vec![
        skipped("dpi", "preserved by the polling-only proof"),
        skipped(
            "button_assignments",
            "all six raw assignments are preserved by the polling-only proof",
        ),
    ];
    let operations = if before.polling.raw_code == polling_code {
        skipped_fields.push(skipped("polling", "raw polling code already matches"));
        Vec::new()
    } else {
        vec![WriteOperation::SetPolling {
            transport: before.polling.transport,
            before_raw_code: before.polling.raw_code,
            after_raw_code: polling_code,
        }]
    };
    Ok(finish_plan(before, target, skipped_fields, operations))
}

pub fn plan_restore(
    before: &DeviceSnapshotV1,
    target_snapshot: &DeviceSnapshotV1,
) -> Result<WritePlan, PlanError> {
    before.validate().map_err(PlanError::InvalidSnapshot)?;
    target_snapshot
        .validate()
        .map_err(PlanError::InvalidSnapshot)?;
    if before.device.vendor_id != target_snapshot.device.vendor_id
        || before.device.product_id != target_snapshot.device.product_id
        || before.device.serial != target_snapshot.device.serial
    {
        return Err(PlanError::IdentityMismatch(
            "restore snapshot VID/PID/serial does not match the connected device".to_owned(),
        ));
    }
    let target = TargetState {
        profile: None,
        dpi: target_snapshot.dpi.current,
        dpi_active_stage_id: Some(target_snapshot.dpi.active_stage_id),
        dpi_stages: Some(target_snapshot.dpi.stages.clone()),
        polling_raw_code: target_snapshot.polling.raw_code,
        polling_hz: target_snapshot.polling.hertz(),
        button_assignments: target_snapshot.button_assignments.clone(),
        button_modes_ignored: vec![BUTTON_MOUSE4_SLOT, BUTTON_MOUSE5_SLOT],
    };
    let mut skipped_fields = Vec::new();
    let mut operations = Vec::new();

    if before.dpi.active_stage_id == target_snapshot.dpi.active_stage_id
        && before.dpi.stages == target_snapshot.dpi.stages
    {
        skipped_fields.push(skipped("dpi.stages", "stage table already matches"));
    } else {
        operations.push(WriteOperation::SetDpiStages {
            before_active_stage_id: before.dpi.active_stage_id,
            before_stages: before.dpi.stages.clone(),
            after_active_stage_id: target_snapshot.dpi.active_stage_id,
            after_stages: target_snapshot.dpi.stages.clone(),
        });
    }
    if before.dpi.current == target_snapshot.dpi.current {
        skipped_fields.push(skipped("dpi.current", "current DPI already matches"));
    } else {
        operations.push(WriteOperation::SetCurrentDpi {
            before: before.dpi.current,
            after: target_snapshot.dpi.current,
            require_active_stage_match: false,
            force_write: false,
        });
    }
    if before.polling.raw_code == target_snapshot.polling.raw_code {
        skipped_fields.push(skipped("polling", "raw polling code already matches"));
    } else {
        operations.push(WriteOperation::SetPolling {
            transport: before.polling.transport,
            before_raw_code: before.polling.raw_code,
            after_raw_code: target_snapshot.polling.raw_code,
        });
    }
    for target_assignment in &target_snapshot.button_assignments {
        let current =
            before
                .button(target_assignment.protocol_button_id)
                .ok_or(PlanError::MissingButton(
                    target_assignment.protocol_button_id,
                ))?;
        let ignore_returned_mode = matches!(
            target_assignment.protocol_button_id,
            BUTTON_MOUSE4_SLOT | BUTTON_MOUSE5_SLOT
        );
        if button_assignment_matches(current, target_assignment, ignore_returned_mode) {
            skipped_fields.push(skipped(
                &format!("button.0x{:02x}", target_assignment.protocol_button_id),
                "raw assignment already matches",
            ));
        } else {
            let mut write_before = current.clone();
            let mut write_after = target_assignment.clone();
            if ignore_returned_mode {
                write_before.mode = crate::model::NORMAL_BUTTON_MODE;
                write_after.mode = crate::model::NORMAL_BUTTON_MODE;
            }
            operations.push(WriteOperation::SetButton {
                before: write_before,
                after: write_after,
                ignore_returned_mode,
            });
        }
    }
    Ok(finish_plan(before, target, skipped_fields, operations))
}

fn finish_plan(
    before: &DeviceSnapshotV1,
    target: TargetState,
    skipped_fields: Vec<SkippedField>,
    operations: Vec<WriteOperation>,
) -> WritePlan {
    let rollback_operations = operations
        .iter()
        .rev()
        .map(WriteOperation::rollback)
        .collect();
    WritePlan {
        before: before.clone(),
        target,
        skipped_fields,
        ordered_operations: operations,
        rollback_operations,
    }
}

fn skipped(field: &str, reason: &str) -> SkippedField {
    SkippedField {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }
}

fn require_same_identity(
    current: &DeviceSnapshotV1,
    baseline: &DeviceSnapshotV1,
) -> Result<(), PlanError> {
    if current.device.vendor_id != baseline.device.vendor_id
        || current.device.product_id != baseline.device.product_id
        || current.device.hid_descriptor_serial != baseline.device.hid_descriptor_serial
        || current.device.serial != baseline.device.serial
        || current.device.firmware != baseline.device.firmware
        || current.device.interface_number != baseline.device.interface_number
        || current.device.usage_page != baseline.device.usage_page
        || current.device.usage != baseline.device.usage
    {
        return Err(PlanError::IdentityMismatch(
            "immutable baseline VID/PID, dongle/body serial, firmware, or control collection does not match the connected device"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Allows only polling writes that passed repeatable hardware validation.
/// No-op plans are always safe because they send no setter.
pub fn require_proven_polling_writes(plan: &WritePlan) -> Result<(), PlanError> {
    for operation in &plan.ordered_operations {
        let WriteOperation::SetPolling {
            transport,
            before_raw_code,
            after_raw_code,
        } = operation
        else {
            continue;
        };
        let proven = before_raw_code == after_raw_code
            || (*transport == TransportKind::WirelessV2
                && matches!(
                    (*before_raw_code, *after_raw_code),
                    (0x08, 0x02) | (0x02, 0x08)
                ));
        if !proven {
            return Err(PlanError::UnsupportedTarget(format!(
                "polling transition 0x{before_raw_code:02x} -> 0x{after_raw_code:02x} is not repeatably hardware-proven; no setter was sent"
            )));
        }
    }
    Ok(())
}

fn gaming_assignments_from_baseline(
    baseline: &DeviceSnapshotV1,
) -> Result<Vec<RawButtonAssignment>, PlanError> {
    [BUTTON_MOUSE4_SLOT, BUTTON_MOUSE5_SLOT]
        .into_iter()
        .map(|button_id| {
            let assignment = baseline
                .button(button_id)
                .ok_or(PlanError::MissingButton(button_id))?;
            if assignment.function_id != crate::model::FUNCTION_BUTTON_CODE
                || assignment.data_size != 1
                || assignment.data[0] != button_id
            {
                return Err(PlanError::UnsupportedTarget(format!(
                    "immutable baseline slot 0x{button_id:02x} is not its native side-button assignment"
                )));
            }
            Ok(assignment.clone())
        })
        .collect()
}

fn replace_assignment(
    assignments: &mut [RawButtonAssignment],
    target: &RawButtonAssignment,
) -> Result<(), PlanError> {
    let current = assignments
        .iter_mut()
        .find(|assignment| assignment.protocol_button_id == target.protocol_button_id)
        .ok_or(PlanError::MissingButton(target.protocol_button_id))?;
    current.clone_from(target);
    Ok(())
}

#[must_use]
pub fn button_assignment_matches(
    observed: &RawButtonAssignment,
    expected: &RawButtonAssignment,
    ignore_returned_mode: bool,
) -> bool {
    observed.profile_id == expected.profile_id
        && observed.protocol_button_id == expected.protocol_button_id
        && (ignore_returned_mode || observed.mode == expected.mode)
        && observed.function_id == expected.function_id
        && observed.data_size == expected.data_size
        && observed.data == expected.data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ALL_BUTTON_IDS, DeviceIdentity, DpiState, FirmwareVersion, PollingState,
        SNAPSHOT_SCHEMA_VERSION, TransportKind,
    };

    fn snapshot() -> DeviceSnapshotV1 {
        DeviceSnapshotV1 {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            captured_at_unix_ms: 1,
            device: DeviceIdentity {
                vendor_id: 0x1532,
                product_id: 0x00e6,
                hid_descriptor_serial: Some("DONGLE123".to_owned()),
                serial: "ABC123".to_owned(),
                firmware: FirmwareVersion { major: 1, minor: 2 },
                path: "hid-path".to_owned(),
                interface_number: 4,
                usage_page: 0x0c,
                usage: 1,
            },
            polling: PollingState {
                transport: TransportKind::WirelessV2,
                raw_code: 0x04,
            },
            dpi: DpiState {
                profile_id: 1,
                current: DpiPair { x: 800, y: 800 },
                active_stage_id: 2,
                stages: vec![
                    DpiStage {
                        id: 1,
                        x: 400,
                        y: 400,
                    },
                    DpiStage {
                        id: 2,
                        x: 800,
                        y: 800,
                    },
                ],
            },
            power: None,
            button_assignments: ALL_BUTTON_IDS
                .into_iter()
                .map(|id| RawButtonAssignment::mouse_button(id, id))
                .collect(),
        }
    }

    #[test]
    fn profile_plan_is_ordered_and_never_touches_protected_buttons() {
        let plan = plan_profile(&snapshot(), ProfileName::Developer).expect("valid plan");
        assert!(matches!(
            plan.ordered_operations[0],
            WriteOperation::SetCurrentDpi { .. }
        ));
        assert!(matches!(
            plan.ordered_operations[1],
            WriteOperation::SetPolling { .. }
        ));
        let changed_buttons: Vec<u8> = plan
            .ordered_operations
            .iter()
            .filter_map(|operation| match operation {
                WriteOperation::SetButton { after, .. } => Some(after.protocol_button_id),
                _ => None,
            })
            .collect();
        assert_eq!(changed_buttons, vec![0x04, 0x05]);
    }

    #[test]
    fn matching_profile_deduplicates_every_write() {
        let mut current = snapshot();
        let spec = ProfileSpec::fixed(ProfileName::Developer);
        current.dpi.current = spec.dpi;
        let active = current
            .dpi
            .stages
            .iter_mut()
            .find(|stage| stage.id == current.dpi.active_stage_id)
            .expect("active stage");
        active.x = spec.dpi.x;
        active.y = spec.dpi.y;
        current.polling.raw_code = 0x08;
        for assignment in spec.side_buttons {
            let current_assignment = current
                .button_assignments
                .iter_mut()
                .find(|item| item.protocol_button_id == assignment.protocol_button_id)
                .expect("button exists");
            *current_assignment = assignment;
        }
        let plan = plan_profile(&current, ProfileName::Developer).expect("valid plan");
        assert!(plan.is_noop());
        assert_eq!(plan.skipped_fields.len(), 4);
    }

    #[test]
    fn polling_only_plan_never_touches_dpi_or_buttons() {
        let before = snapshot();
        let plan = plan_polling_only(&before, 4000).expect("valid polling-only plan");
        assert_eq!(plan.ordered_operations.len(), 1);
        assert!(matches!(
            plan.ordered_operations[0],
            WriteOperation::SetPolling {
                transport: TransportKind::WirelessV2,
                before_raw_code: 0x04,
                after_raw_code: 0x02,
            }
        ));
        assert_eq!(plan.target.dpi, before.dpi.current);
        assert_eq!(
            plan.target.dpi_active_stage_id,
            Some(before.dpi.active_stage_id)
        );
        assert_eq!(plan.target.dpi_stages, Some(before.dpi.stages.clone()));
        assert_eq!(plan.target.button_assignments, before.button_assignments);
        let intents = plan.packet_intents().expect("valid packet intents");
        assert_eq!(intents.len(), 2);
        assert_eq!(intents[0].arguments, [0x00, 0x02]);
        assert_eq!(intents[1].arguments, [0x01, 0x02]);
    }

    #[test]
    fn matching_polling_only_target_deduplicates_to_zero_writes() {
        let mut before = snapshot();
        before.polling.raw_code = 0x08;
        let plan = plan_polling_only(&before, 1000).expect("valid polling-only plan");
        assert!(plan.is_noop());
    }

    #[test]
    fn production_polling_gate_allows_only_measured_wireless_profile_transitions() {
        let mut baseline = snapshot();
        baseline.polling.raw_code = 0x08;
        let raise = plan_polling_only(&baseline, 4000).expect("1000 to 4000 plan");
        assert_eq!(require_proven_polling_writes(&raise), Ok(()));

        let matching = plan_polling_only(&baseline, 1000).expect("1000 no-op plan");
        assert_eq!(require_proven_polling_writes(&matching), Ok(()));

        baseline.polling.raw_code = 0x02;
        let lower = plan_polling_only(&baseline, 1000).expect("4000 to 1000 plan");
        assert_eq!(require_proven_polling_writes(&lower), Ok(()));

        baseline.polling.raw_code = 0x04;
        let unknown_origin = plan_polling_only(&baseline, 4000).expect("2000 to 4000 plan");
        assert!(require_proven_polling_writes(&unknown_origin).is_err());
    }

    #[test]
    fn baseline_identity_includes_dongle_firmware_and_control_collection() {
        let baseline = snapshot();
        let mut changed = baseline.clone();
        changed.device.firmware.minor += 1;
        assert!(plan_profile_with_baseline(&changed, &baseline, ProfileName::Developer).is_err());

        changed = baseline.clone();
        changed.device.hid_descriptor_serial = Some("OTHER-DONGLE".to_owned());
        assert!(plan_profile_with_baseline(&changed, &baseline, ProfileName::Developer).is_err());

        changed = baseline.clone();
        changed.device.interface_number += 1;
        assert!(plan_profile_with_baseline(&changed, &baseline, ProfileName::Developer).is_err());
    }

    #[test]
    fn forced_dpi_proof_emits_only_the_direct_1600_setter() {
        let mut before = snapshot();
        before.dpi.current = DpiPair { x: 1600, y: 1600 };
        let active = before
            .dpi
            .stages
            .iter_mut()
            .find(|stage| stage.id == before.dpi.active_stage_id)
            .expect("active stage");
        active.x = 1600;
        active.y = 1600;
        let plan = plan_dpi_1600_only(&before, true).expect("valid DPI-only plan");
        assert_eq!(plan.ordered_operations.len(), 1);
        assert!(matches!(
            plan.ordered_operations[0],
            WriteOperation::SetCurrentDpi {
                after: DpiPair { x: 1600, y: 1600 },
                require_active_stage_match: true,
                force_write: true,
                ..
            }
        ));
        assert_eq!(plan.target.polling_raw_code, before.polling.raw_code);
        assert_eq!(plan.target.button_assignments, before.button_assignments);
        assert_eq!(plan.packet_intents().expect("packet intent").len(), 1);
    }

    #[test]
    fn button_only_plan_uses_normal_selector_and_preserves_every_other_field() {
        let mut baseline = snapshot();
        baseline
            .button_assignments
            .iter_mut()
            .find(|assignment| assignment.protocol_button_id == BUTTON_MOUSE4_SLOT)
            .expect("mouse4")
            .mode = 1;
        let before = baseline.clone();
        let plan = plan_button_only(
            &before,
            &baseline,
            ProfileName::Developer,
            BUTTON_MOUSE4_SLOT,
        )
        .expect("valid button-only plan");
        assert_eq!(plan.ordered_operations.len(), 1);
        assert!(matches!(
            &plan.ordered_operations[0],
            WriteOperation::SetButton {
                before,
                after,
                ignore_returned_mode: true,
            } if before.mode == 0
                && after.mode == 0
                && after.data == [0x05, 0x43, 0, 0, 0]
        ));
        assert_eq!(plan.target.dpi, before.dpi.current);
        assert_eq!(plan.target.polling_raw_code, before.polling.raw_code);
        assert_eq!(plan.target.button_modes_ignored, vec![BUTTON_MOUSE4_SLOT]);
        for assignment in &before.button_assignments {
            if assignment.protocol_button_id != BUTTON_MOUSE4_SLOT {
                assert_eq!(
                    plan.target
                        .button_assignments
                        .iter()
                        .find(|candidate| {
                            candidate.protocol_button_id == assignment.protocol_button_id
                        })
                        .expect("preserved assignment"),
                    assignment
                );
            }
        }
    }

    #[test]
    fn baseline_backed_gaming_profile_preserves_measured_side_actions() {
        let mut baseline = snapshot();
        let mouse4 = baseline
            .button_assignments
            .iter_mut()
            .find(|assignment| assignment.protocol_button_id == BUTTON_MOUSE4_SLOT)
            .expect("mouse4");
        mouse4.mode = 1;
        let plan = plan_profile_with_baseline(&baseline, &baseline, ProfileName::Gaming)
            .expect("valid baseline-backed plan");
        let target_mouse4 = plan
            .target
            .button_assignments
            .iter()
            .find(|assignment| assignment.protocol_button_id == BUTTON_MOUSE4_SLOT)
            .expect("target mouse4");
        assert_eq!(target_mouse4.mode, 1);
        assert_eq!(
            target_mouse4.function_id,
            crate::model::FUNCTION_BUTTON_CODE
        );
        assert_eq!(target_mouse4.data[0], BUTTON_MOUSE4_SLOT);
    }

    #[test]
    fn button_proof_rejects_matching_identity_with_nonbaseline_polling() {
        let baseline = snapshot();
        let mut contaminated = baseline.clone();
        contaminated.polling.raw_code = 0x02;
        assert!(
            require_clean_button_proof_start(
                &contaminated,
                &baseline,
                ProfileName::Developer,
                BUTTON_MOUSE4_SLOT,
            )
            .is_err()
        );
    }

    #[test]
    fn button_restore_allows_only_the_selected_assignment_to_differ() {
        let baseline = snapshot();
        let mut developer = baseline.clone();
        replace_assignment(
            &mut developer.button_assignments,
            &RawButtonAssignment::keyboard_chord(BUTTON_MOUSE4_SLOT, crate::model::HID_USAGE_F10),
        )
        .expect("replace mouse4");
        assert_eq!(
            require_clean_button_proof_start(
                &developer,
                &baseline,
                ProfileName::Gaming,
                BUTTON_MOUSE4_SLOT,
            ),
            Ok(())
        );
        developer.polling.raw_code = 0x02;
        assert!(
            require_clean_button_proof_start(
                &developer,
                &baseline,
                ProfileName::Gaming,
                BUTTON_MOUSE4_SLOT,
            )
            .is_err()
        );
    }

    #[test]
    fn rollback_is_reverse_order_with_original_values() {
        let plan = plan_profile(&snapshot(), ProfileName::Developer).expect("valid plan");
        assert_eq!(
            plan.rollback_operations.len(),
            plan.ordered_operations.len()
        );
        for (forward, rollback) in plan
            .ordered_operations
            .iter()
            .zip(plan.rollback_operations.iter().rev())
        {
            assert_eq!(&forward.rollback(), rollback);
        }
    }

    #[test]
    fn restore_preserves_unknown_raw_assignment_and_stage_ids() {
        let mut target = snapshot();
        target.dpi.active_stage_id = 9;
        target.dpi.stages = vec![DpiStage {
            id: 9,
            x: 1234,
            y: 1235,
        }];
        target.button_assignments[0].function_id = 0xfe;
        target.button_assignments[0].data_size = 5;
        target.button_assignments[0].data = [9, 8, 7, 6, 5];
        let plan = plan_restore(&snapshot(), &target).expect("valid restore plan");
        assert!(plan.ordered_operations.iter().any(|operation| matches!(
            operation,
            WriteOperation::SetDpiStages {
                after_active_stage_id: 9,
                after_stages,
                ..
            } if after_stages[0].id == 9
        )));
        assert!(plan.ordered_operations.iter().any(|operation| matches!(
            operation,
            WriteOperation::SetButton { after, .. }
                if after.function_id == 0xfe && after.data == [9, 8, 7, 6, 5]
        )));
    }
}
