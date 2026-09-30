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
        let observed_before_verification = report.values_observed.len();
        match verify_noop_state(device, plan, &mut report.values_observed) {
            Ok(snapshot) => {
                report.final_state = Some(snapshot);
                report.success = true;
            }
            Err(error) => {
                report.mismatches.push(error);
                // 2026-09-29: values_observed is an audit trail; preserve a
                // mismatched fresh read, but never treat history as current state.
                report.final_state = report
                    .values_observed
                    .get(observed_before_verification)
                    .cloned();
            }
        }
        return report;
    }

    let mut attempted_operations = Vec::new();
    for operation in &plan.ordered_operations {
        match write_with_readback(
            device,
            operation,
            &plan.before,
            &mut report.values_observed,
            operation.force_write(),
            false,
        ) {
            Ok(attempt) => {
                attempted_operations.push(operation.clone());
                report.writes_attempted.push(attempt);
            }
            Err(failure) => {
                if failure.rollback_current {
                    attempted_operations.push(operation.clone());
                }
                report.writes_attempted.push(failure.attempt);
                report.mismatches.push(failure.message);
                let (rollback_result, final_state) = rollback(
                    device,
                    &attempted_operations,
                    &plan.before,
                    &mut report.values_observed,
                    failure.force_current_rollback,
                    failure.stop_transaction_writes,
                );
                report.rollback_result = Some(rollback_result);
                report.final_state = final_state;
                return report;
            }
        }
    }

    let mut stop_rollback_writes = false;
    match device.read_snapshot() {
        Ok(final_state) => {
            if let Err(message) = validate_identity(&final_state, &plan.before) {
                report.mismatches.push(message);
                stop_rollback_writes = true;
            } else {
                report.values_observed.push(final_state.clone());
                if target_matches(&final_state, plan) {
                    report.success = true;
                } else {
                    report
                        .mismatches
                        .push("final state no longer matches the complete target".to_owned());
                    stop_rollback_writes = true;
                }
                report.final_state = Some(final_state);
            }
        }
        Err(error) => {
            report
                .mismatches
                .push(format!("final verification read failed: {error}"));
            stop_rollback_writes = true;
        }
    }
    if !report.success {
        let (rollback_result, final_state) = rollback(
            device,
            &attempted_operations,
            &plan.before,
            &mut report.values_observed,
            false,
            stop_rollback_writes,
        );
        report.rollback_result = Some(rollback_result);
        report.final_state = final_state;
    }
    report
}

fn verify_noop_state<D: DeviceControl>(
    device: &mut D,
    plan: &WritePlan,
    observations: &mut Vec<DeviceSnapshotV1>,
) -> Result<DeviceSnapshotV1, String> {
    let snapshot = read_validated(device, &plan.before, observations).map_err(|error| {
        format!("no-op live verification failed; no setter was called: {error}")
    })?;
    if !target_matches(&snapshot, plan) {
        return Err(
            "no-op state changed since planning; live state does not match the target".to_owned(),
        );
    }
    Ok(snapshot)
}

struct ReadbackFailure {
    attempt: WriteAttempt,
    message: String,
    force_current_rollback: bool,
    rollback_current: bool,
    stop_transaction_writes: bool,
}

fn readback_failure(
    attempt: WriteAttempt,
    message: String,
    force_current_rollback: bool,
    rollback_current: bool,
    stop_transaction_writes: bool,
) -> ReadbackFailure {
    ReadbackFailure {
        attempt,
        message,
        force_current_rollback,
        rollback_current,
        stop_transaction_writes,
    }
}

fn write_with_readback<D: DeviceControl>(
    device: &mut D,
    operation: &WriteOperation,
    identity: &DeviceSnapshotV1,
    observations: &mut Vec<DeviceSnapshotV1>,
    force_write: bool,
    allow_before_mismatch: bool,
) -> Result<WriteAttempt, ReadbackFailure> {
    let mut attempt = WriteAttempt {
        field: operation.field_name(),
        commands_attempted: 0,
        ambiguous_result_observed: false,
        verified: false,
    };
    let expected_before = observations.last().unwrap_or(identity).clone();
    let preflight = read_validated(device, identity, observations).map_err(|error| {
        readback_failure(
            attempt.clone(),
            format!(
                "{}: identity/state preflight failed before setter: {error}",
                operation.field_name()
            ),
            false,
            false,
            true,
        )
    })?;
    if !allow_before_mismatch && !operation.matches_before(&preflight, &expected_before) {
        return Err(readback_failure(
            attempt,
            format!(
                "{}: expected before value changed since the last verified snapshot; setter was not called",
                operation.field_name()
            ),
            false,
            false,
            true,
        ));
    }
    if operation.matches(&preflight) && !force_write {
        attempt.verified = true;
        return Ok(attempt);
    }

    attempt.commands_attempted = 1;
    match device.write_operation(operation) {
        Ok(()) => {}
        Err(error) if error.kind == WriteFailureKind::Rejected => {
            return Err(classify_rejected_write(
                device,
                operation,
                identity,
                observations,
                &preflight,
                &mut attempt,
                &error,
            ));
        }
        Err(error) if error.kind == WriteFailureKind::UnverifiablePartial => {
            attempt.ambiguous_result_observed = true;
            return Err(readback_failure(
                attempt,
                format!(
                    "{}: {error}; aggregate readback cannot prove both physical reports",
                    operation.field_name()
                ),
                true,
                true,
                false,
            ));
        }
        Err(error) => {
            let target_was_observed = recover_ambiguous_write(
                device,
                operation,
                identity,
                observations,
                &preflight,
                &mut attempt,
                &error,
            )?;
            if target_was_observed {
                return Ok(attempt);
            }
        }
    }

    verify_operation_readback(
        device,
        operation,
        identity,
        observations,
        &mut attempt,
        &preflight,
    )?;
    Ok(attempt)
}

fn classify_rejected_write<D: DeviceControl>(
    device: &mut D,
    operation: &WriteOperation,
    identity: &DeviceSnapshotV1,
    observations: &mut Vec<DeviceSnapshotV1>,
    preflight: &DeviceSnapshotV1,
    attempt: &mut WriteAttempt,
    error: &WriteFailure,
) -> ReadbackFailure {
    let observed = match read_validated(device, identity, observations) {
        Ok(observed) => observed,
        Err(read_error) => {
            attempt.ambiguous_result_observed = true;
            return readback_failure(
                attempt.clone(),
                format!(
                    "{}: device returned rejection ({error}), but post-rejection readback failed; state is uncertain: {read_error}",
                    operation.field_name()
                ),
                false,
                false,
                true,
            );
        }
    };
    if operation.matches_before(&observed, preflight) {
        return readback_failure(
            attempt.clone(),
            format!(
                "{}: {error}; readback confirms the field is unchanged",
                operation.field_name()
            ),
            false,
            false,
            false,
        );
    }
    attempt.ambiguous_result_observed = true;
    if operation.matches(&observed) {
        return readback_failure(
            attempt.clone(),
            format!(
                "{}: {error}; readback shows the target was applied, so rollback will restore the field",
                operation.field_name()
            ),
            false,
            true,
            false,
        );
    }
    readback_failure(
        attempt.clone(),
        format!(
            "{}: {error}; post-rejection readback matches neither the expected before value nor the target; manual recovery is required",
            operation.field_name()
        ),
        false,
        false,
        true,
    )
}

fn recover_ambiguous_write<D: DeviceControl>(
    device: &mut D,
    operation: &WriteOperation,
    identity: &DeviceSnapshotV1,
    observations: &mut Vec<DeviceSnapshotV1>,
    preflight: &DeviceSnapshotV1,
    attempt: &mut WriteAttempt,
    error: &WriteFailure,
) -> Result<bool, ReadbackFailure> {
    attempt.ambiguous_result_observed = true;
    let observed = read_validated(device, identity, observations).map_err(|read_error| {
        readback_failure(
            attempt.clone(),
            format!(
                "{}: ambiguous write ({error}); readback failed: {read_error}",
                operation.field_name()
            ),
            false,
            false,
            true,
        )
    })?;
    if operation.matches(&observed) {
        if operation.force_write() && operation.matches_before(&observed, preflight) {
            return Err(readback_failure(
                attempt.clone(),
                format!(
                    "{}: forced write readback matches the pre-write state and target; the command result is uncertain and needs manual review",
                    operation.field_name()
                ),
                false,
                false,
                true,
            ));
        }
        attempt.verified = true;
        return Ok(true);
    }
    if !operation.matches_before(&observed, preflight) {
        return Err(readback_failure(
            attempt.clone(),
            format!(
                "{}: ambiguous write readback matches neither the expected before value nor the target; refusing resend",
                operation.field_name()
            ),
            false,
            false,
            true,
        ));
    }
    attempt.commands_attempted = 2;
    if let Err(retry_error) = device.write_operation(operation) {
        return Err(readback_failure(
            attempt.clone(),
            format!(
                "{}: target absent after ambiguous result; one allowed resend failed: {retry_error}",
                operation.field_name()
            ),
            false,
            false,
            true,
        ));
    }
    Ok(false)
}

fn verify_operation_readback<D: DeviceControl>(
    device: &mut D,
    operation: &WriteOperation,
    identity: &DeviceSnapshotV1,
    observations: &mut Vec<DeviceSnapshotV1>,
    attempt: &mut WriteAttempt,
    expected_before: &DeviceSnapshotV1,
) -> Result<(), ReadbackFailure> {
    let observed = read_validated(device, identity, observations).map_err(|error| {
        readback_failure(
            attempt.clone(),
            format!(
                "{}: verification read failed: {error}",
                operation.field_name()
            ),
            false,
            false,
            true,
        )
    })?;
    if !operation.matches(&observed) {
        let unchanged = operation.matches_before(&observed, expected_before);
        return Err(readback_failure(
            attempt.clone(),
            if unchanged {
                format!(
                    "{}: independent readback confirms the field is unchanged",
                    operation.field_name()
                )
            } else {
                format!(
                    "{}: independent readback matches neither the expected before value nor the target; manual recovery is required",
                    operation.field_name()
                )
            },
            false,
            false,
            !unchanged,
        ));
    }
    attempt.verified = true;
    Ok(())
}

fn rollback<D: DeviceControl>(
    device: &mut D,
    attempted_operations: &[WriteOperation],
    original: &DeviceSnapshotV1,
    observations: &mut Vec<DeviceSnapshotV1>,
    force_current_rollback: bool,
    stop_transaction_writes: bool,
) -> (RollbackResult, Option<DeviceSnapshotV1>) {
    let mut result = RollbackResult {
        attempted_fields: Vec::new(),
        verified_fields: Vec::new(),
        errors: Vec::new(),
        final_state_restored: false,
    };
    if stop_transaction_writes {
        result.errors.push(
            "rollback writes stopped because device state is uncertain; manual recovery is required"
                .to_owned(),
        );
    }
    for (index, operation) in attempted_operations
        .iter()
        .rev()
        .map(WriteOperation::rollback)
        .enumerate()
        .take(if stop_transaction_writes {
            0
        } else {
            usize::MAX
        })
    {
        let field = operation.field_name();
        result.attempted_fields.push(field.clone());
        match write_with_readback(
            device,
            &operation,
            original,
            observations,
            force_current_rollback && index == 0,
            force_current_rollback && index == 0,
        ) {
            Ok(_) => result.verified_fields.push(field),
            Err(failure) => {
                result.errors.push(failure.message);
                if failure.stop_transaction_writes {
                    result.errors.push(
                        "rollback writes stopped because device state is uncertain; manual recovery is required"
                            .to_owned(),
                    );
                    break;
                }
            }
        }
    }
    let final_state = match device.read_snapshot() {
        Ok(final_state) => {
            let identity_error = validate_identity(&final_state, original).err();
            let identity_matches = identity_error.is_none();
            let configuration_matches = equivalent_config(&final_state, original);
            observations.push(final_state.clone());
            if let Some(error) = identity_error {
                result.errors.push(error);
            } else if !configuration_matches {
                result
                    .errors
                    .push("rollback final state differs from the original snapshot".to_owned());
            }
            result.final_state_restored =
                identity_matches && configuration_matches && result.errors.is_empty();
            Some(final_state)
        }
        Err(error) => {
            result
                .errors
                .push(format!("rollback final verification read failed: {error}"));
            None
        }
    };
    (result, final_state)
}

fn read_validated<D: DeviceControl>(
    device: &mut D,
    identity: &DeviceSnapshotV1,
    observations: &mut Vec<DeviceSnapshotV1>,
) -> Result<DeviceSnapshotV1, String> {
    let snapshot = device.read_snapshot()?;
    snapshot.validate()?;
    observations.push(snapshot.clone());
    validate_identity(&snapshot, identity)?;
    Ok(snapshot)
}

fn validate_identity(
    observed: &DeviceSnapshotV1,
    expected: &DeviceSnapshotV1,
) -> Result<(), String> {
    if observed.device != expected.device {
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
    use crate::planning::{plan_dpi_1600_only, plan_profile, plan_restore};

    #[derive(Clone, Copy)]
    enum Behavior {
        Apply,
        AmbiguousApply,
        AmbiguousNoApply,
        AmbiguousThirdValue,
        Reject,
        ApplyThenReject,
        RejectThirdValue,
        ApplyThenChangeIdentity,
        Disconnect,
        GetFail,
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
            if matches!(self.behaviors.front(), Some(Behavior::GetFail)) {
                self.behaviors.pop_front();
                return Err("mock GET failed".to_owned());
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
                Behavior::AmbiguousThirdValue => {
                    match operation {
                        WriteOperation::SetPolling { .. } => self.state.polling.raw_code = 0x10,
                        _ => panic!("test third-value behavior expects a polling operation"),
                    }
                    Err(WriteFailure::ambiguous(
                        "third value observed after timeout",
                    ))
                }
                Behavior::Reject => Err(WriteFailure::rejected("device rejected command")),
                Behavior::ApplyThenReject => {
                    mutate(&mut self.state, operation);
                    Err(WriteFailure::rejected(
                        "device returned failure after applying",
                    ))
                }
                Behavior::RejectThirdValue => {
                    match operation {
                        WriteOperation::SetPolling { .. } => self.state.polling.raw_code = 0x10,
                        _ => panic!("test third-value behavior expects a polling operation"),
                    }
                    Err(WriteFailure::rejected(
                        "device returned failure after an external state change",
                    ))
                }
                Behavior::ApplyThenChangeIdentity => {
                    mutate(&mut self.state, operation);
                    self.state.device.serial.push_str("-OTHER");
                    Ok(())
                }
                Behavior::Disconnect => {
                    self.disconnected = true;
                    Err(WriteFailure::ambiguous("device disconnected during write"))
                }
                Behavior::GetFail => panic!("GET failure behavior cannot be used as a write"),
                Behavior::UnverifiablePartial => Err(WriteFailure::unverifiable_partial(
                    "second V2 polling report had an ambiguous result",
                )),
            }
        }
    }

    struct ScriptedReadFailures {
        inner: MockDevice,
        read_count: usize,
        fail_at_reads: Vec<usize>,
    }

    impl DeviceControl for ScriptedReadFailures {
        fn read_snapshot(&mut self) -> Result<DeviceSnapshotV1, String> {
            self.read_count += 1;
            if self.fail_at_reads.contains(&self.read_count) {
                return Err("scripted GET failure".to_owned());
            }
            self.inner.read_snapshot()
        }

        fn write_operation(&mut self, operation: &WriteOperation) -> Result<(), WriteFailure> {
            self.inner.write_operation(operation)
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
    fn ambiguous_third_value_stops_all_transaction_writes() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        assert!(matches!(
            plan.ordered_operations.get(1),
            Some(WriteOperation::SetPolling { .. })
        ));
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::from([Behavior::Apply, Behavior::AmbiguousThirdValue]),
            writes: 0,
            disconnected: false,
        };

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(
            device.writes, 2,
            "uncertain readback must stop resend and rollback"
        );
        assert_eq!(report.writes_attempted[1].commands_attempted, 1);
        assert_eq!(device.state.dpi.current, DpiPair { x: 1600, y: 1600 });
        assert_eq!(device.state.polling.raw_code, 0x10);
        assert!(report.mismatches.iter().any(|message| {
            message.contains("matches neither the expected before value nor the target")
                && message.contains("refusing resend")
        }));
        let rollback = report.rollback_result.expect("rollback report");
        assert!(rollback.attempted_fields.is_empty());
        assert!(!rollback.final_state_restored);
        assert!(
            rollback
                .errors
                .iter()
                .any(|message| message.contains("manual recovery"))
        );
        assert_eq!(report.final_state, Some(device.state.clone()));
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
        assert_eq!(
            device.writes, 3,
            "the rejected polling field is read back, not resent or rolled back"
        );
        assert_eq!(rollback.attempted_fields, vec!["dpi.current"]);
        assert_eq!(device.state.dpi, before.dpi);
        assert_eq!(device.state.polling, before.polling);
    }

    #[test]
    fn rejected_ack_with_target_readback_is_rolled_back() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut device = MockDevice {
            state: before.clone(),
            behaviors: VecDeque::from([Behavior::ApplyThenReject]),
            writes: 0,
            disconnected: false,
        };

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(
            device.writes, 2,
            "target after rejection should be restored"
        );
        assert_eq!(device.state.dpi, before.dpi);
        assert!(
            report
                .mismatches
                .iter()
                .any(|message| { message.contains("readback shows the target was applied") })
        );
        assert!(
            report
                .rollback_result
                .expect("rollback report")
                .final_state_restored
        );
    }

    #[test]
    fn rejected_ack_with_third_value_stops_all_transaction_writes() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::from([Behavior::Apply, Behavior::RejectThirdValue]),
            writes: 0,
            disconnected: false,
        };

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(device.writes, 2);
        assert_eq!(device.state.dpi.current, DpiPair { x: 1600, y: 1600 });
        assert_eq!(device.state.polling.raw_code, 0x10);
        assert!(report.mismatches.iter().any(|message| {
            message.contains("matches neither the expected before value nor the target")
                && message.contains("manual recovery")
        }));
        let rollback = report.rollback_result.expect("rollback report");
        assert!(rollback.attempted_fields.is_empty());
        assert!(!rollback.final_state_restored);
        assert!(
            rollback
                .errors
                .iter()
                .any(|message| message.contains("manual recovery"))
        );
    }

    #[test]
    fn current_dpi_write_accepts_unchanged_stage_mismatch_from_plan() {
        let mut before = base_snapshot();
        before.dpi.current = DpiPair { x: 1600, y: 1600 };
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        assert!(matches!(
            plan.ordered_operations.first(),
            Some(WriteOperation::SetCurrentDpi { .. })
        ));
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::new(),
            writes: 0,
            disconnected: false,
        };

        let report = apply_write_plan(&mut device, &plan);

        assert!(report.success);
        assert_eq!(device.writes, plan.ordered_operations.len());
        assert_eq!(device.state.dpi.current, DpiPair { x: 1600, y: 1600 });
        assert_eq!(device.state.dpi.active_stage().unwrap().x, 1600);
    }

    #[test]
    fn restore_rolls_back_stage_table_after_current_dpi_changes_active_stage() {
        let mut before = base_snapshot();
        before.dpi.stages.push(DpiStage {
            id: 2,
            x: 1000,
            y: 1000,
        });
        let mut target = before.clone();
        target.dpi.active_stage_id = 2;
        target.dpi.current = DpiPair { x: 2200, y: 2200 };
        target.dpi.stages = vec![
            DpiStage {
                id: 1,
                x: 900,
                y: 900,
            },
            DpiStage {
                id: 2,
                x: 2200,
                y: 2200,
            },
        ];
        target.polling.raw_code = 0x08;
        let plan = plan_restore(&before, &target).expect("valid restore plan");
        assert!(matches!(
            plan.ordered_operations.as_slice(),
            [
                WriteOperation::SetDpiStages { .. },
                WriteOperation::SetCurrentDpi { .. },
                WriteOperation::SetPolling { .. },
            ]
        ));
        let mut device = MockDevice {
            state: before.clone(),
            behaviors: VecDeque::from([Behavior::Apply, Behavior::Apply, Behavior::Reject]),
            writes: 0,
            disconnected: false,
        };

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        let rollback = report.rollback_result.expect("rollback report");
        assert!(rollback.final_state_restored, "rollback: {rollback:?}");
        assert_eq!(device.state.dpi, before.dpi);
        assert_eq!(device.state.polling, before.polling);
    }

    #[test]
    fn changed_field_preflight_does_not_write_or_roll_back_that_field() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::new(),
            writes: 0,
            disconnected: false,
        };
        device.state.dpi.current = DpiPair { x: 1200, y: 1200 };
        let active_stage = device
            .state
            .dpi
            .stages
            .iter_mut()
            .find(|stage| stage.id == device.state.dpi.active_stage_id)
            .expect("active stage");
        active_stage.x = 1200;
        active_stage.y = 1200;
        let external_state = device.state.clone();

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(device.writes, 0);
        assert_eq!(device.state, external_state);
        assert!(report.mismatches.iter().any(|message| {
            message.contains("expected before value changed")
                && message.contains("setter was not called")
        }));
        let rollback = report.rollback_result.expect("rollback report");
        assert!(rollback.attempted_fields.is_empty());
        assert!(!rollback.final_state_restored);
        assert_eq!(report.final_state, Some(external_state));
    }

    #[test]
    fn changed_hid_path_identity_stops_before_any_setter() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut device = MockDevice {
            state: before,
            behaviors: VecDeque::new(),
            writes: 0,
            disconnected: false,
        };
        device.state.device.path = "different-hid-interface".to_owned();
        let changed_identity_state = device.state.clone();

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(device.writes, 0);
        assert_eq!(device.state, changed_identity_state);
        assert!(
            report
                .mismatches
                .iter()
                .any(|message| { message.contains("identity changed") })
        );
        let rollback = report.rollback_result.expect("rollback report");
        assert!(rollback.attempted_fields.is_empty());
        assert!(!rollback.final_state_restored);
        assert_eq!(report.final_state, Some(changed_identity_state));
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
        let rollback = report.rollback_result.expect("rollback report");
        assert_eq!(
            device.writes, 1,
            "identity change must block rollback writes"
        );
        assert!(!rollback.final_state_restored);
        assert!(
            rollback
                .errors
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
        assert_eq!(report.final_state, None);
    }

    #[test]
    fn failed_final_and_rollback_gets_do_not_promote_historical_write_readbacks() {
        let before = base_snapshot();
        let plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");

        // Count the normal transaction's reads so the script fails only the
        // complete-target final verification GET and rollback's final GET.
        let mut counting_device = ScriptedReadFailures {
            inner: MockDevice {
                state: before.clone(),
                behaviors: VecDeque::new(),
                writes: 0,
                disconnected: false,
            },
            read_count: 0,
            fail_at_reads: Vec::new(),
        };
        assert!(apply_write_plan(&mut counting_device, &plan).success);
        let normal_read_count = counting_device.read_count;

        let mut device = ScriptedReadFailures {
            inner: MockDevice {
                state: before,
                behaviors: VecDeque::new(),
                writes: 0,
                disconnected: false,
            },
            read_count: 0,
            fail_at_reads: vec![normal_read_count, normal_read_count + 1],
        };
        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(device.inner.writes, plan.ordered_operations.len());
        assert_eq!(report.writes_attempted.len(), plan.ordered_operations.len());
        assert!(
            report
                .writes_attempted
                .iter()
                .all(|attempt| attempt.verified)
        );
        assert!(
            report
                .values_observed
                .iter()
                .any(|snapshot| target_matches(snapshot, &plan))
        );
        assert_eq!(report.final_state, None);
        assert_eq!(device.read_count, normal_read_count + 1);
        assert!(
            report
                .rollback_result
                .expect("rollback report")
                .errors
                .iter()
                .any(|error| error.contains("rollback final verification read failed"))
        );
    }

    #[test]
    fn noop_get_failure_does_not_report_the_planning_snapshot_as_current() {
        let before = base_snapshot();
        let initial_plan = plan_profile(&before, ProfileName::Developer).expect("valid plan");
        let mut target = before;
        for operation in &initial_plan.ordered_operations {
            mutate(&mut target, operation);
        }
        let plan = crate::planning::plan_profile(&target, ProfileName::Developer)
            .expect("valid no-op plan");
        assert!(plan.is_noop());
        let mut device = MockDevice {
            state: target.clone(),
            behaviors: VecDeque::from([Behavior::GetFail]),
            writes: 0,
            disconnected: false,
        };

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(device.writes, 0);
        assert_eq!(report.values_observed, vec![target]);
        assert_eq!(report.final_state, None);
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
    fn forced_matching_dpi_ambiguous_noop_is_not_reported_as_success() {
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
            behaviors: VecDeque::from([Behavior::AmbiguousNoApply]),
            writes: 0,
            disconnected: false,
        };

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(device.writes, 1);
        assert_eq!(report.writes_attempted[0].commands_attempted, 1);
        assert!(report.mismatches.iter().any(|message| {
            message.contains("forced write readback matches the pre-write state and target")
                && message.contains("manual review")
        }));
        let rollback = report.rollback_result.expect("rollback report");
        assert!(rollback.attempted_fields.is_empty());
        assert!(
            rollback
                .errors
                .iter()
                .any(|message| message.contains("manual recovery"))
        );
    }

    #[test]
    fn stale_noop_plan_gets_live_verification_without_setter() {
        let initial = base_snapshot();
        let initial_plan = plan_profile(&initial, ProfileName::Developer).expect("valid plan");
        let mut matching = initial.clone();
        for operation in &initial_plan.ordered_operations {
            mutate(&mut matching, operation);
        }
        let plan = plan_profile(&matching, ProfileName::Developer).expect("no-op plan");
        assert!(plan.is_noop());
        let mut device = MockDevice {
            state: matching,
            behaviors: VecDeque::new(),
            writes: 0,
            disconnected: false,
        };
        device.state.polling.raw_code = 0x10;
        let stale_state = device.state.clone();

        let report = apply_write_plan(&mut device, &plan);

        assert!(!report.success);
        assert_eq!(device.writes, 0);
        assert!(
            report
                .mismatches
                .iter()
                .any(|message| { message.contains("no-op state changed since planning") })
        );
        assert_eq!(report.final_state, Some(stale_state));
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
