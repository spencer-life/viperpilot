use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model::{DeviceSnapshotV1, ProfileName};
use crate::planning::{
    WriteOperation, WritePlan, button_assignment_matches, plan_profile_with_baseline,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteFailureKind {
    Rejected,
    Ambiguous,
    UnverifiablePartial,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteFailure {
    pub kind: WriteFailureKind,
    pub message: String,
}

impl WriteFailure {
    pub fn rejected(message: impl Into<String>) -> Self {
        Self {
            kind: WriteFailureKind::Rejected,
            message: message.into(),
        }
    }

    pub fn ambiguous(message: impl Into<String>) -> Self {
        Self {
            kind: WriteFailureKind::Ambiguous,
            message: message.into(),
        }
    }

    pub fn unverifiable_partial(message: impl Into<String>) -> Self {
        Self {
            kind: WriteFailureKind::UnverifiablePartial,
            message: message.into(),
        }
    }
}

impl fmt::Display for WriteFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for WriteFailure {}

pub trait DeviceControl {
    fn read_snapshot(&mut self) -> Result<DeviceSnapshotV1, String>;
    fn write_operation(&mut self, operation: &WriteOperation) -> Result<(), WriteFailure>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WriteAttempt {
    pub field: String,
    pub commands_attempted: u8,
    pub ambiguous_result_observed: bool,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RollbackResult {
    pub attempted_fields: Vec<String>,
    pub verified_fields: Vec<String>,
    pub errors: Vec<String>,
    pub final_state_restored: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerificationReport {
    pub requested_profile: Option<ProfileName>,
    pub writes_attempted: Vec<WriteAttempt>,
    pub values_observed: Vec<DeviceSnapshotV1>,
    pub mismatches: Vec<String>,
    pub rollback_result: Option<RollbackResult>,
    pub final_state: Option<DeviceSnapshotV1>,
    pub success: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileMatch {
    Developer,
    Gaming,
    OutOfSync,
}

#[must_use]
pub fn next_profile_for_hotkey(current: ProfileMatch) -> Option<ProfileName> {
    match current {
        ProfileMatch::Developer => Some(ProfileName::Gaming),
        ProfileMatch::Gaming => Some(ProfileName::Developer),
        ProfileMatch::OutOfSync => None,
    }
}

pub fn classify_profile(
    snapshot: &DeviceSnapshotV1,
    baseline: &DeviceSnapshotV1,
) -> Result<ProfileMatch, String> {
    let developer = plan_profile_with_baseline(snapshot, baseline, ProfileName::Developer)
        .map_err(|error| error.to_string())?;
    if developer.is_noop() {
        return Ok(ProfileMatch::Developer);
    }
    if snapshot.device.product_id == crate::model::VIPER_V4_PRO_WIRELESS_PID {
        let gaming = plan_profile_with_baseline(snapshot, baseline, ProfileName::Gaming)
            .map_err(|error| error.to_string())?;
        if gaming.is_noop() {
            return Ok(ProfileMatch::Gaming);
        }
    }
    Ok(ProfileMatch::OutOfSync)
}

pub fn apply_write_plan<D: DeviceControl>(device: &mut D, plan: &WritePlan) -> VerificationReport {
    let mut report = VerificationReport {
        requested_profile: plan.target.profile,
        writes_attempted: Vec::new(),
        values_observed: vec![plan.before.clone()],
        mismatches: Vec::new(),
        rollback_result: None,
        final_state: None,
        success: false,
    };

    if plan.is_noop() {
        report.final_state = Some(plan.before.clone());
        report.success = true;
        return report;
    }

    let mut attempted_operations = Vec::new();
    for operation in &plan.ordered_operations {
        attempted_operations.push(operation.clone());
        match write_with_readback(
            device,
            operation,
            &plan.before,
            &mut report.values_observed,
            operation.force_write(),
        ) {
            Ok(attempt) => report.writes_attempted.push(attempt),
            Err((attempt, message, force_current_rollback)) => {
                report.writes_attempted.push(attempt);
                report.mismatches.push(message);
                report.rollback_result = Some(rollback(
                    device,
                    &attempted_operations,
                    &plan.before,
                    &mut report.values_observed,
                    force_current_rollback,
                ));
                report.final_state = report.values_observed.last().cloned();
                return report;
            }
        }
    }

    match device.read_snapshot() {
        Ok(final_state) => {
            if let Err(message) = validate_identity(&final_state, &plan.before) {
                report.mismatches.push(message);
            } else {
                report.values_observed.push(final_state.clone());
                if target_matches(&final_state, plan) {
                    report.success = true;
                } else {
                    report
                        .mismatches
                        .push("final state no longer matches the complete target".to_owned());
                }
                report.final_state = Some(final_state);
            }
        }
        Err(error) => report
            .mismatches
            .push(format!("final verification read failed: {error}")),
    }
    if !report.success {
        report.rollback_result = Some(rollback(
            device,
            &attempted_operations,
            &plan.before,
            &mut report.values_observed,
            false,
        ));
        report.final_state = report.values_observed.last().cloned();
    }
    report
}

fn write_with_readback<D: DeviceControl>(
    device: &mut D,
    operation: &WriteOperation,
    identity: &DeviceSnapshotV1,
    observations: &mut Vec<DeviceSnapshotV1>,
    force_write: bool,
) -> Result<WriteAttempt, (WriteAttempt, String, bool)> {
    let mut attempt = WriteAttempt {
        field: operation.field_name(),
        commands_attempted: 0,
        ambiguous_result_observed: false,
        verified: false,
    };
    let preflight = read_validated(device, identity).map_err(|error| {
        (
            attempt.clone(),
            format!(
                "{}: identity/state preflight failed before setter: {error}",
                operation.field_name()
            ),
            false,
        )
    })?;
    let already_applied = operation.matches(&preflight);
    observations.push(preflight);
    if already_applied && !force_write {
        attempt.verified = true;
        return Ok(attempt);
    }
    attempt.commands_attempted = 1;
    match device.write_operation(operation) {
        Ok(()) => {}
        Err(error) if error.kind == WriteFailureKind::Rejected => {
            return Err((
                attempt,
                format!("{}: {error}", operation.field_name()),
                false,
            ));
        }
        Err(error) if error.kind == WriteFailureKind::UnverifiablePartial => {
            attempt.ambiguous_result_observed = true;
            return Err((
                attempt,
                format!(
                    "{}: {error}; aggregate readback cannot prove both physical reports",
                    operation.field_name()
                ),
                true,
            ));
        }
        Err(error) => {
            attempt.ambiguous_result_observed = true;
            let observed = read_validated(device, identity).map_err(|read_error| {
                (
                    attempt.clone(),
                    format!(
                        "{}: ambiguous write ({error}); readback failed: {read_error}",
                        operation.field_name()
                    ),
                    false,
                )
            })?;
            let already_applied = operation.matches(&observed);
            observations.push(observed);
            if already_applied {
                attempt.verified = true;
                return Ok(attempt);
            }
            attempt.commands_attempted = 2;
            if let Err(retry_error) = device.write_operation(operation) {
                return Err((
                    attempt,
                    format!(
                        "{}: target absent after ambiguous result; one allowed resend failed: {retry_error}",
                        operation.field_name()
                    ),
                    false,
                ));
            }
        }
    }

    let observed = read_validated(device, identity).map_err(|error| {
        (
            attempt.clone(),
            format!(
                "{}: verification read failed: {error}",
                operation.field_name()
            ),
            false,
        )
    })?;
    let matches = operation.matches(&observed);
    observations.push(observed);
    if !matches {
        return Err((
            attempt,
            format!(
                "{}: independent readback did not match",
                operation.field_name()
            ),
            false,
        ));
    }
    attempt.verified = true;
    Ok(attempt)
}

fn rollback<D: DeviceControl>(
    device: &mut D,
    attempted_operations: &[WriteOperation],
    original: &DeviceSnapshotV1,
    observations: &mut Vec<DeviceSnapshotV1>,
    force_current_rollback: bool,
) -> RollbackResult {
    let mut result = RollbackResult {
        attempted_fields: Vec::new(),
        verified_fields: Vec::new(),
        errors: Vec::new(),
        final_state_restored: false,
    };
    for (index, operation) in attempted_operations
        .iter()
        .rev()
        .map(WriteOperation::rollback)
        .enumerate()
    {
        let field = operation.field_name();
        result.attempted_fields.push(field.clone());
        match write_with_readback(
            device,
            &operation,
            original,
            observations,
            force_current_rollback && index == 0,
        ) {
            Ok(_) => result.verified_fields.push(field),
            Err((_, error, _)) => result.errors.push(error),
        }
    }
    match device.read_snapshot() {
        Ok(final_state) => {
            let configuration_matches = equivalent_config(&final_state, original);
            observations.push(final_state);
            if !configuration_matches {
                result
                    .errors
                    .push("rollback final state differs from the original snapshot".to_owned());
            }
            result.final_state_restored = configuration_matches && result.errors.is_empty();
        }
        Err(error) => result
            .errors
            .push(format!("rollback final verification read failed: {error}")),
    }
    result
}

fn read_validated<D: DeviceControl>(
    device: &mut D,
    identity: &DeviceSnapshotV1,
) -> Result<DeviceSnapshotV1, String> {
    let snapshot = device.read_snapshot()?;
    snapshot.validate()?;
    validate_identity(&snapshot, identity)?;
    Ok(snapshot)
}

fn validate_identity(
    observed: &DeviceSnapshotV1,
    expected: &DeviceSnapshotV1,
) -> Result<(), String> {
    if observed.device.vendor_id != expected.device.vendor_id
        || observed.device.product_id != expected.device.product_id
        || observed.device.serial != expected.device.serial
    {
        return Err("device identity changed during the operation".to_owned());
    }
    Ok(())
}

fn equivalent_config(left: &DeviceSnapshotV1, right: &DeviceSnapshotV1) -> bool {
    left.device.vendor_id == right.device.vendor_id
        && left.device.product_id == right.device.product_id
        && left.device.serial == right.device.serial
        && left.polling == right.polling
        && left.dpi == right.dpi
        && left.button_assignments == right.button_assignments
}

fn target_matches(snapshot: &DeviceSnapshotV1, plan: &WritePlan) -> bool {
    snapshot.dpi.current == plan.target.dpi
        && plan
            .target
            .dpi_active_stage_id
            .is_none_or(|stage_id| snapshot.dpi.active_stage_id == stage_id)
        && plan
            .target
            .dpi_stages
            .as_ref()
            .is_none_or(|stages| snapshot.dpi.stages == *stages)
        && snapshot.polling.raw_code == plan.target.polling_raw_code
        && plan.target.button_assignments.iter().all(|target| {
            let ignore_mode = plan
                .target
                .button_modes_ignored
                .contains(&target.protocol_button_id);
            snapshot
                .button(target.protocol_button_id)
                .is_some_and(|observed| button_assignment_matches(observed, target, ignore_mode))
        })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::model::{
        ALL_BUTTON_IDS, DeviceIdentity, DpiPair, DpiStage, DpiState, FirmwareVersion, PollingState,
        ProfileName, RawButtonAssignment, SNAPSHOT_SCHEMA_VERSION, TransportKind,
    };
    use crate::planning::{plan_dpi_1600_only, plan_profile};

    #[derive(Clone, Copy)]
    enum Behavior {
        Apply,
        AmbiguousApply,
        AmbiguousNoApply,
        Reject,
        ApplyThenChangeIdentity,
        Disconnect,
        UnverifiablePartial,
    }

    struct MockDevice {
        state: DeviceSnapshotV1,
        behaviors: VecDeque<Behavior>,
        writes: usize,
        disconnected: bool,
    }

    impl DeviceControl for MockDevice {
        fn read_snapshot(&mut self) -> Result<DeviceSnapshotV1, String> {
            if self.disconnected {
                return Err("device disconnected".to_owned());
            }
            Ok(self.state.clone())
        }

        fn write_operation(&mut self, operation: &WriteOperation) -> Result<(), WriteFailure> {
            self.writes += 1;
            match self.behaviors.pop_front().unwrap_or(Behavior::Apply) {
                Behavior::Apply => {
                    mutate(&mut self.state, operation);
                    Ok(())
                }
                Behavior::AmbiguousApply => {
                    mutate(&mut self.state, operation);
                    Err(WriteFailure::ambiguous("lost acknowledgment"))
                }
                Behavior::AmbiguousNoApply => {
                    Err(WriteFailure::ambiguous("timeout before acknowledgment"))
                }
                Behavior::Reject => Err(WriteFailure::rejected("device rejected command")),
                Behavior::ApplyThenChangeIdentity => {
                    mutate(&mut self.state, operation);
                    self.state.device.serial.push_str("-OTHER");
                    Ok(())
                }
                Behavior::Disconnect => {
                    self.disconnected = true;
                    Err(WriteFailure::ambiguous("device disconnected during write"))
                }
                Behavior::UnverifiablePartial => Err(WriteFailure::unverifiable_partial(
                    "second V2 polling report had an ambiguous result",
                )),
            }
        }
    }

    fn base_snapshot() -> DeviceSnapshotV1 {
        DeviceSnapshotV1 {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            captured_at_unix_ms: 1,
            device: DeviceIdentity {
                vendor_id: 0x1532,
                product_id: 0x00e6,
                hid_descriptor_serial: Some("DONGLE123".to_owned()),
                serial: "SERIAL1234".to_owned(),
                firmware: FirmwareVersion { major: 1, minor: 1 },
                path: "path".to_owned(),
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
                active_stage_id: 1,
                stages: vec![DpiStage {
                    id: 1,
                    x: 800,
                    y: 800,
                }],
            },
            power: None,
            button_assignments: ALL_BUTTON_IDS
                .into_iter()
                .map(|id| RawButtonAssignment::mouse_button(id, id))
                .collect(),
        }
    }

    fn mutate(snapshot: &mut DeviceSnapshotV1, operation: &WriteOperation) {
        match operation {
            WriteOperation::SetDpiStages {
                after_active_stage_id,
                after_stages,
                ..
            } => {
                snapshot.dpi.active_stage_id = *after_active_stage_id;
                snapshot.dpi.stages.clone_from(after_stages);
            }
            WriteOperation::SetCurrentDpi { after, .. } => {
                snapshot.dpi.current = *after;
                if let Some(stage) = snapshot
                    .dpi
                    .stages
                    .iter_mut()
                    .find(|stage| stage.id == snapshot.dpi.active_stage_id)
                {
                    stage.x = after.x;
                    stage.y = after.y;
                }
            }
            WriteOperation::SetPolling { after_raw_code, .. } => {
                snapshot.polling.raw_code = *after_raw_code;
            }
            WriteOperation::SetButton { after, .. } => {
                let assignment = snapshot
                    .button_assignments
                    .iter_mut()
                    .find(|assignment| assignment.protocol_button_id == after.protocol_button_id)
                    .expect("mock button exists");
                assignment.clone_from(after);
            }
        }
    }

    #[test]
    fn lost_acknowledgment_reads_before_any_resend() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let expected_writes = plan.ordered_operations.len();
        let mut device = MockDevice {
            state: before,
            behaviors: std::iter::once(Behavior::AmbiguousApply)
                .chain(std::iter::repeat(Behavior::Apply))
                .take(expected_writes)
                .collect(),
            writes: 0,
            disconnected: false,
        };
        let report = apply_write_plan(&mut device, &plan);
        assert!(report.success);
        assert_eq!(device.writes, expected_writes);
        assert!(report.writes_attempted[0].ambiguous_result_observed);
        assert_eq!(report.writes_attempted[0].commands_attempted, 1);
    }

    #[test]
    fn ambiguous_non_write_is_resent_once() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let expected_writes = plan.ordered_operations.len() + 1;
        let mut device = MockDevice {
            state: before,
            behaviors: [Behavior::AmbiguousNoApply, Behavior::Apply]
                .into_iter()
                .chain(std::iter::repeat(Behavior::Apply))
                .take(expected_writes)
                .collect(),
            writes: 0,
            disconnected: false,
        };
        let report = apply_write_plan(&mut device, &plan);
        assert!(report.success);
        assert_eq!(device.writes, expected_writes);
        assert_eq!(report.writes_attempted[0].commands_attempted, 2);
    }

    #[test]
    fn partial_failure_rolls_back_completed_fields() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut device = MockDevice {
            state: before.clone(),
            behaviors: VecDeque::from([Behavior::Apply, Behavior::Reject]),
            writes: 0,
            disconnected: false,
        };
        let report = apply_write_plan(&mut device, &plan);
        assert!(!report.success);
        let rollback = report.rollback_result.expect("rollback was attempted");
        assert!(rollback.final_state_restored);
        assert_eq!(device.state.dpi, before.dpi);
        assert_eq!(device.state.polling, before.polling);
    }

    #[test]
    fn wrong_identity_stops_before_the_next_setter() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::from([Behavior::ApplyThenChangeIdentity]),
            writes: 0,
            disconnected: false,
        };
        let report = apply_write_plan(&mut device, &plan);
        assert!(!report.success);
        assert_eq!(device.writes, 1);
        assert!(
            report
                .mismatches
                .iter()
                .any(|message| message.contains("identity changed"))
        );
    }

    #[test]
    fn disconnect_during_write_does_not_resend() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::from([Behavior::Disconnect]),
            writes: 0,
            disconnected: false,
        };
        let report = apply_write_plan(&mut device, &plan);
        assert!(!report.success);
        assert_eq!(device.writes, 1);
        assert!(
            report
                .rollback_result
                .expect("rollback result")
                .errors
                .iter()
                .any(|message| message.contains("disconnected"))
        );
    }

    #[test]
    fn failed_rollback_reports_unrestored_final_state() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::from([
                Behavior::Apply,
                Behavior::Apply,
                Behavior::Reject,
                Behavior::Apply,
                Behavior::Reject,
                Behavior::Apply,
            ]),
            writes: 0,
            disconnected: false,
        };
        let report = apply_write_plan(&mut device, &plan);
        assert!(!report.success);
        let rollback = report.rollback_result.expect("rollback result");
        assert!(!rollback.final_state_restored);
        assert!(!rollback.errors.is_empty());
    }

    #[test]
    fn partial_v2_polling_forces_both_report_restore_even_if_aggregate_looks_original() {
        let mut before = base_snapshot();
        let developer = crate::model::ProfileSpec::fixed(ProfileName::Developer);
        before.dpi.current = developer.dpi;
        let active_stage = before
            .dpi
            .stages
            .iter_mut()
            .find(|stage| stage.id == before.dpi.active_stage_id)
            .expect("active stage");
        active_stage.x = developer.dpi.x;
        active_stage.y = developer.dpi.y;
        for target in developer.side_buttons {
            let current = before
                .button_assignments
                .iter_mut()
                .find(|assignment| assignment.protocol_button_id == target.protocol_button_id)
                .expect("side button");
            *current = target;
        }
        let plan = plan_profile(&before, ProfileName::Developer).expect("polling-only plan");
        assert_eq!(plan.ordered_operations.len(), 1);
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::from([Behavior::UnverifiablePartial, Behavior::Apply]),
            writes: 0,
            disconnected: false,
        };
        let report = apply_write_plan(&mut device, &plan);
        assert!(!report.success);
        assert_eq!(device.writes, 2);
        assert!(
            report
                .rollback_result
                .expect("forced rollback")
                .final_state_restored
        );
    }

    #[test]
    fn forced_matching_dpi_proof_still_sends_one_command() {
        let mut before = base_snapshot();
        before.dpi.current = DpiPair { x: 1600, y: 1600 };
        let active = before
            .dpi
            .stages
            .iter_mut()
            .find(|stage| stage.id == before.dpi.active_stage_id)
            .expect("active stage");
        active.x = 1600;
        active.y = 1600;
        let plan = plan_dpi_1600_only(&before, true).expect("forced DPI plan");
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::from([Behavior::Apply]),
            writes: 0,
            disconnected: false,
        };
        let report = apply_write_plan(&mut device, &plan);
        assert!(report.success);
        assert_eq!(device.writes, 1);
        assert_eq!(report.writes_attempted[0].commands_attempted, 1);
    }

    #[test]
    fn production_hotkey_toggles_only_complete_profiles() {
        assert_eq!(
            next_profile_for_hotkey(ProfileMatch::Developer),
            Some(ProfileName::Gaming)
        );
        assert_eq!(
            next_profile_for_hotkey(ProfileMatch::Gaming),
            Some(ProfileName::Developer)
        );
        assert_eq!(next_profile_for_hotkey(ProfileMatch::OutOfSync), None);
    }
}
