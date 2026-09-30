//! Device evidence contract and fail-closed preflight for local draft requests.
//!
//! Registry scope is dated 2026-09-29. The only recorded evidence currently
//! covers wireless Viper V4 Pro VID 0x1532/PID 0x00e6, firmware 1.4, `MI_03`,
//! usage 0x000c:0x0001. Other scopes inherit no evidence. This registry records
//! observations; it does not authorize writes or promote custom combinations.

use serde::{Deserialize, Serialize};

use crate::model::{
    BUTTON_MOUSE4_SLOT, BUTTON_MOUSE5_SLOT, DeviceIdentity, DeviceSnapshotV1, DpiPair,
    FUNCTION_BUTTON_CODE, FUNCTION_KEY_CODE, MODIFIER_LEFT_ALT, MODIFIER_LEFT_CONTROL,
    RawButtonAssignment, TransportKind,
};
use crate::profile_intent::{
    ButtonActionIntent, CapabilityTarget, DpiTarget, FieldAssessments, FieldCapability,
    KeyboardKey, ProfileIntentV1,
};

pub const DRAFT_REQUEST_SCHEMA_VERSION: u32 = 1;
pub const CAPABILITY_CONTRACT_ID: &str = "viperpilot.viper-v4-pro.wireless";
pub const CAPABILITY_CONTRACT_REVISION: u32 = 1;

/// Evidence scope only. It intentionally excludes serial numbers and paths.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityScope {
    pub vendor_id: u16,
    pub product_id: u16,
    pub firmware_major: u8,
    pub firmware_minor: u8,
    pub transport: TransportKind,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
}

impl CapabilityScope {
    /// The sole scope with recorded measurements as of 2026-09-29.
    pub const RECORDED_WIRELESS_VIPER_V4_PRO: Self = Self {
        vendor_id: 0x1532,
        product_id: 0x00e6,
        firmware_major: 1,
        firmware_minor: 4,
        transport: TransportKind::WirelessV2,
        interface_number: 3,
        usage_page: 0x000c,
        usage: 0x0001,
    };

    fn from_identity(identity: &DeviceIdentity, transport: TransportKind) -> Self {
        Self {
            vendor_id: identity.vendor_id,
            product_id: identity.product_id,
            firmware_major: identity.firmware.major,
            firmware_minor: identity.firmware.minor,
            transport,
            interface_number: identity.interface_number,
            usage_page: identity.usage_page,
            usage: identity.usage,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRule {
    pub field: &'static str,
    pub accepted_local_values: &'static str,
    pub measured_values: &'static str,
    pub evidence: FieldCapability,
    pub rule: FieldEvidenceRule,
    pub provenance: &'static str,
}

/// Typed evidence predicates are the data source used by assessment and
/// preflight. Human-readable strings above are explanatory labels only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldEvidenceRule {
    LocalOnly,
    ObservedDpi(DpiTarget),
    PollingTransitions(&'static [(u16, u16)]),
    ButtonActions(&'static [ButtonActionIntent]),
    ResearchRequired,
}

static MEASURED_POLLING_TRANSITIONS: &[(u16, u16)] = &[(1000, 4000), (4000, 1000)];
static MEASURED_MOUSE4_ACTIONS: &[ButtonActionIntent] = &[
    ButtonActionIntent::PassThrough,
    ButtonActionIntent::KeyboardShortcut {
        control: true,
        alt: true,
        shift: false,
        windows: false,
        key: KeyboardKey::F10,
    },
];
static MEASURED_MOUSE5_ACTIONS: &[ButtonActionIntent] = &[
    ButtonActionIntent::PassThrough,
    ButtonActionIntent::KeyboardShortcut {
        control: true,
        alt: true,
        shift: false,
        windows: false,
        key: KeyboardKey::F11,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityContract {
    pub id: &'static str,
    pub revision: u32,
    pub scope: CapabilityScope,
    pub fields: &'static [FieldRule],
    pub combination_gate: &'static str,
    pub combination_policy: CombinationPolicy,
    pub independent_readback_required: bool,
    pub rollback_required: bool,
    pub recovery_required: bool,
    pub readback_policy: ReadbackPolicy,
    pub recovery_policy: RecoveryPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombinationPolicy {
    NoApprovedCustomCombinations,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadbackPolicy {
    FreshIndependentGetAfterEachSetter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryPolicy {
    StopOnAmbiguousReadbackAndRequireReviewedBaselineRestoration,
}

const FIELD_RULES: &[FieldRule] = &[
    FieldRule {
        field: "name",
        accepted_local_values: "validated local display name",
        measured_values: "none; local metadata only",
        evidence: FieldCapability::LocalOnly,
        rule: FieldEvidenceRule::LocalOnly,
        provenance: "application data, no device field",
    },
    FieldRule {
        field: "dpi",
        accepted_local_values: "positive X/Y values",
        measured_values: "1600x1600 observed and read back while already 1600x1600",
        evidence: FieldCapability::ObservedValueOnly,
        rule: FieldEvidenceRule::ObservedDpi(DpiTarget { x: 1600, y: 1600 }),
        provenance: "historical repository observation; registry reviewed 2026-09-29; no changed-value transition proven",
    },
    FieldRule {
        field: "polling_hz",
        accepted_local_values: "positive integer",
        measured_values: "1000 <-> 4000 Hz only",
        evidence: FieldCapability::MeasuredTransition,
        rule: FieldEvidenceRule::PollingTransitions(MEASURED_POLLING_TRANSITIONS),
        provenance: "isolated wireless Viper V4 Pro transition tests",
    },
    FieldRule {
        field: "mouse4",
        accepted_local_values: "semantic button action",
        measured_values: "native pass-through or exact Ctrl+Alt+F10",
        evidence: FieldCapability::MeasuredTransition,
        rule: FieldEvidenceRule::ButtonActions(MEASURED_MOUSE4_ACTIONS),
        provenance: "isolated Mouse4 test; unrelated chords/actions unproven",
    },
    FieldRule {
        field: "mouse5",
        accepted_local_values: "semantic button action",
        measured_values: "native pass-through or exact Ctrl+Alt+F11",
        evidence: FieldCapability::MeasuredTransition,
        rule: FieldEvidenceRule::ButtonActions(MEASURED_MOUSE5_ACTIONS),
        provenance: "isolated Mouse5 test; unrelated chords/actions unproven",
    },
    FieldRule {
        field: "other / unassigned / unmodeled",
        accepted_local_values: "schema-valid semantic values",
        measured_values: "none",
        evidence: FieldCapability::ResearchRequired,
        rule: FieldEvidenceRule::ResearchRequired,
        provenance: "no evidence recorded",
    },
];

pub static RECORDED_CONTRACT: CapabilityContract = CapabilityContract {
    id: CAPABILITY_CONTRACT_ID,
    revision: CAPABILITY_CONTRACT_REVISION,
    scope: CapabilityScope::RECORDED_WIRELESS_VIPER_V4_PRO,
    fields: FIELD_RULES,
    combination_gate: "no custom profile combination is approved for promotion",
    combination_policy: CombinationPolicy::NoApprovedCustomCombinations,
    independent_readback_required: true,
    rollback_required: true,
    recovery_required: true,
    readback_policy: ReadbackPolicy::FreshIndependentGetAfterEachSetter,
    recovery_policy: RecoveryPolicy::StopOnAmbiguousReadbackAndRequireReviewedBaselineRestoration,
};

/// Exact-match registry lookup; missing models/firmware/transports inherit no rules.
#[must_use]
pub fn lookup_contract(scope: CapabilityScope) -> Option<&'static CapabilityContract> {
    (scope == RECORDED_CONTRACT.scope).then_some(&RECORDED_CONTRACT)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRequestV1 {
    pub schema_version: u32,
    pub contract_revision: u32,
    pub target_scope: CapabilityScope,
    pub intent: ProfileIntentV1,
}

impl DraftRequestV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != DRAFT_REQUEST_SCHEMA_VERSION {
            return Err(format!(
                "unsupported draft request schema {}, expected {DRAFT_REQUEST_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.contract_revision != CAPABILITY_CONTRACT_REVISION {
            return Err(format!(
                "unsupported capability contract revision {}, expected {CAPABILITY_CONTRACT_REVISION}",
                self.contract_revision
            ));
        }
        self.intent.validate()
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|error| format!("encode draft request: {error}"))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let request: Self =
            serde_json::from_str(json).map_err(|error| format!("decode draft request: {error}"))?;
        request.validate()?;
        Ok(request)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityFieldAssessment {
    pub label: String,
    pub evidence: FieldCapability,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityAssessment {
    pub fields: Vec<CapabilityFieldAssessment>,
    pub blockers: Vec<String>,
}

impl CapabilityAssessment {
    /// Custom profiles remain gated pending combination and recovery evidence.
    #[must_use]
    pub const fn can_apply(&self) -> bool {
        false
    }
}

pub fn assess_draft(
    intent: &ProfileIntentV1,
    target: Option<CapabilityScope>,
) -> Result<CapabilityAssessment, String> {
    intent.validate()?;
    let values = classify_intent(intent, target);
    let fields: Vec<CapabilityFieldAssessment> = [
        ("Name", values.name, "Local draft metadata."),
        (
            "DPI",
            values.dpi,
            "1600x1600 was observed only while already at that value; changed DPI is unproven.",
        ),
        (
            "Polling",
            values.polling,
            "Only the measured 1000 Hz <-> 4000 Hz transitions are evidenced.",
        ),
        (
            "Mouse 4",
            values.mouse4,
            "Only native pass-through and exact Ctrl+Alt+F10 are evidenced.",
        ),
        (
            "Mouse 5",
            values.mouse5,
            "Only native pass-through and exact Ctrl+Alt+F11 are evidenced.",
        ),
    ]
    .into_iter()
    .map(|(label, evidence, reason)| CapabilityFieldAssessment {
        label: label.to_owned(),
        evidence,
        reason: reason.to_owned(),
    })
    .collect();
    let mut blockers = fields
        .iter()
        .filter(|field| field.evidence == FieldCapability::ResearchRequired)
        .map(|field| format!("{} requires device-specific research.", field.label))
        .collect::<Vec<_>>();
    match target {
        None => blockers.push(
            "No verified device scope was supplied; field evidence is not device-specific.".into(),
        ),
        Some(scope) if scope != RECORDED_CONTRACT.scope => {
            blockers
                .push("No recorded evidence contract matches the supplied device scope.".into());
        }
        Some(_) => {}
    }
    blockers.push("Custom profile combinations have not passed promotion evidence gates.".into());
    Ok(CapabilityAssessment { fields, blockers })
}

/// Used by the legacy field-only API so evidence rules have one source.
pub(crate) fn classify_intent(
    intent: &ProfileIntentV1,
    scope: Option<CapabilityScope>,
) -> FieldAssessments {
    let scope = scope.unwrap_or(CapabilityScope {
        vendor_id: 0,
        product_id: 0,
        firmware_major: 0,
        firmware_minor: 0,
        transport: TransportKind::WirelessV2,
        interface_number: -1,
        usage_page: 0,
        usage: 0,
    });
    let target = CapabilityTarget {
        vendor_id: scope.vendor_id,
        product_id: scope.product_id,
        firmware_major: scope.firmware_major,
        firmware_minor: scope.firmware_minor,
        interface_number: scope.interface_number,
        usage_page: scope.usage_page,
        usage: scope.usage,
    };
    let contract = lookup_contract(scope);
    let evidence = |field: &str| {
        contract.and_then(|contract| contract.fields.iter().find(|rule| rule.field == field))
    };
    let dpi_rule = evidence("dpi").map(|rule| &rule.rule);
    let polling_rule = evidence("polling_hz").map(|rule| &rule.rule);
    let mouse4_rule = evidence("mouse4").map(|rule| &rule.rule);
    let mouse5_rule = evidence("mouse5").map(|rule| &rule.rule);
    let dpi_supported = match dpi_rule {
        Some(FieldEvidenceRule::ObservedDpi(value)) => intent.dpi == *value,
        _ => false,
    };
    let polling_supported = match polling_rule {
        Some(FieldEvidenceRule::PollingTransitions(transitions)) => transitions
            .iter()
            .any(|(from, to)| intent.polling_hz == *from || intent.polling_hz == *to),
        _ => false,
    };
    let mouse4_supported = matches!(
        mouse4_rule,
        Some(FieldEvidenceRule::ButtonActions(actions)) if actions.contains(&intent.mouse4)
    );
    let mouse5_supported = matches!(
        mouse5_rule,
        Some(FieldEvidenceRule::ButtonActions(actions)) if actions.contains(&intent.mouse5)
    );
    FieldAssessments {
        target,
        name: FieldCapability::LocalOnly,
        dpi: if contract.is_some() && dpi_supported {
            FieldCapability::ObservedValueOnly
        } else {
            FieldCapability::ResearchRequired
        },
        polling: if contract.is_some() && polling_supported {
            FieldCapability::MeasuredTransition
        } else {
            FieldCapability::ResearchRequired
        },
        mouse4: if contract.is_some() && mouse4_supported {
            FieldCapability::MeasuredTransition
        } else {
            FieldCapability::ResearchRequired
        },
        mouse5: if contract.is_some() && mouse5_supported {
            FieldCapability::MeasuredTransition
        } else {
            FieldCapability::ResearchRequired
        },
    }
}

fn action_covered(contract: &CapabilityContract, action: &ButtonActionIntent, field: &str) -> bool {
    match &contract
        .fields
        .iter()
        .find(|rule| rule.field == field)
        .expect("registered button rule")
        .rule
    {
        FieldEvidenceRule::ButtonActions(actions) => actions.contains(action),
        _ => false,
    }
}

#[must_use]
pub fn recorded_evidence_summary() -> &'static str {
    "Recorded scope: wireless Razer Viper V4 Pro (VID 0x1532, PID 0x00e6), firmware 1.4, MI_03, usage 0x000c:0x0001. Reference evidence only; no verified live connection implied."
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateError {
    InvalidRequest,
    UnsupportedContractOrTarget,
    MissingBaseline,
    MissingExpectedState,
    MissingCurrentState,
    InvalidSnapshot,
    IdentityMismatch,
    ScopeMismatch,
    AmbiguousBaseline,
    AmbiguousState,
    StaleExpectedState,
    UnsupportedDpi,
    UnsupportedPollingTransition,
    UnsupportedButtonAction,
    CustomCombinationUnapproved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreflightRejection {
    pub category: GateError,
    pub reason: String,
}

impl PreflightRejection {
    fn new(category: GateError, reason: impl Into<String>) -> Self {
        Self {
            category,
            reason: reason.into(),
        }
    }
}

/// Read-only preflight. Caller-supplied snapshots do not prove a live GET,
/// baseline immutability, or reviewed serial identity. Success is not exposed:
/// every otherwise valid custom request ends at the combination gate.
#[allow(clippy::too_many_lines)] // Keep the gate's ordered rejection flow auditable in one place.
pub fn preflight_draft_request(
    request: &DraftRequestV1,
    baseline: Option<&DeviceSnapshotV1>,
    expected: Option<&DeviceSnapshotV1>,
    current: Option<&DeviceSnapshotV1>,
) -> Result<(), PreflightRejection> {
    request
        .validate()
        .map_err(|reason| PreflightRejection::new(GateError::InvalidRequest, reason))?;
    let contract = lookup_contract(request.target_scope).ok_or_else(|| {
        PreflightRejection::new(
            GateError::UnsupportedContractOrTarget,
            "request target has no registered evidence contract",
        )
    })?;
    let baseline = baseline.ok_or_else(|| {
        PreflightRejection::new(
            GateError::MissingBaseline,
            "complete immutable baseline is required",
        )
    })?;
    let expected = expected.ok_or_else(|| {
        PreflightRejection::new(
            GateError::MissingExpectedState,
            "expected state is required",
        )
    })?;
    let current = current.ok_or_else(|| {
        PreflightRejection::new(GateError::MissingCurrentState, "current state is required")
    })?;
    for snapshot in [baseline, expected, current] {
        snapshot
            .validate()
            .map_err(|reason| PreflightRejection::new(GateError::InvalidSnapshot, reason))?;
    }
    let snapshots = [baseline, expected, current];
    for snapshot in snapshots {
        let transport = TransportKind::for_product_id(snapshot.device.product_id)
            .map_err(|reason| PreflightRejection::new(GateError::ScopeMismatch, reason))?;
        if CapabilityScope::from_identity(&snapshot.device, transport) != request.target_scope {
            return Err(PreflightRejection::new(
                GateError::ScopeMismatch,
                "snapshot identity scope does not exactly match request target, including transport",
            ));
        }
        if snapshot.device != baseline.device {
            return Err(PreflightRejection::new(
                GateError::IdentityMismatch,
                "baseline, expected, and current full device identities must match, including serial and path",
            ));
        }
    }
    for (label, snapshot) in [
        ("baseline", baseline),
        ("expected", expected),
        ("current", current),
    ] {
        if snapshot.polling.hertz().is_none()
            || snapshot.dpi.current.x == 0
            || snapshot.dpi.current.y == 0
            || snapshot
                .dpi
                .stages
                .iter()
                .any(|stage| stage.x == 0 || stage.y == 0)
            || snapshot
                .dpi
                .stages
                .iter()
                .enumerate()
                .any(|(index, stage)| {
                    snapshot.dpi.stages[index + 1..]
                        .iter()
                        .any(|other| other.id == stage.id)
                })
        {
            let category = if label == "baseline" {
                GateError::AmbiguousBaseline
            } else {
                GateError::AmbiguousState
            };
            return Err(PreflightRejection::new(
                category,
                format!("{label} contains unknown polling, zero DPI, or ambiguous DPI stages"),
            ));
        }
        validate_side_assignment(
            snapshot
                .button(BUTTON_MOUSE4_SLOT)
                .expect("validated button count"),
            BUTTON_MOUSE4_SLOT,
        )
        .and_then(|()| {
            validate_side_assignment(
                snapshot
                    .button(BUTTON_MOUSE5_SLOT)
                    .expect("validated button count"),
                BUTTON_MOUSE5_SLOT,
            )
        })
        .map_err(|reason| {
            let category = if label == "baseline" {
                GateError::AmbiguousBaseline
            } else {
                GateError::AmbiguousState
            };
            PreflightRejection::new(category, format!("{label}: {reason}"))
        })?;
    }
    if !same_logical_state(expected, current) {
        return Err(PreflightRejection::new(
            GateError::StaleExpectedState,
            "current logical state differs from the supplied expected state",
        ));
    }
    let dpi_target = DpiPair {
        x: request.intent.dpi.x,
        y: request.intent.dpi.y,
    };
    let observed_dpi = contract
        .fields
        .iter()
        .find(|rule| rule.field == "dpi")
        .and_then(|rule| match rule.rule {
            FieldEvidenceRule::ObservedDpi(value) => Some(DpiPair {
                x: value.x,
                y: value.y,
            }),
            _ => None,
        });
    if observed_dpi != Some(dpi_target) || current.dpi.current != dpi_target {
        return Err(PreflightRejection::new(
            GateError::UnsupportedDpi,
            "only unchanged 1600x1600 has observed-value evidence; no changed DPI transition is approved",
        ));
    }
    let from_hz = current
        .polling
        .hertz()
        .expect("checked known polling above");
    if !polling_transition_allowed(contract, from_hz, request.intent.polling_hz) {
        return Err(PreflightRejection::new(
            GateError::UnsupportedPollingTransition,
            "only 1000<->4000 Hz transitions are measured; unsupported sources such as 8000 Hz remain blocked",
        ));
    }
    if !action_covered(contract, &request.intent.mouse4, "mouse4")
        || !action_covered(contract, &request.intent.mouse5, "mouse5")
    {
        return Err(PreflightRejection::new(
            GateError::UnsupportedButtonAction,
            "button actions must match the individually measured Mouse4/Mouse5 actions",
        ));
    }
    match contract.combination_policy {
        CombinationPolicy::NoApprovedCustomCombinations => Err(PreflightRejection::new(
            GateError::CustomCombinationUnapproved,
            "individual field evidence does not approve this custom profile combination; promotion remains blocked",
        )),
    }
}

fn same_logical_state(left: &DeviceSnapshotV1, right: &DeviceSnapshotV1) -> bool {
    let mut left_stages = left.dpi.stages.clone();
    let mut right_stages = right.dpi.stages.clone();
    left_stages.sort_by_key(|stage| stage.id);
    right_stages.sort_by_key(|stage| stage.id);
    let mut left_buttons = left.button_assignments.clone();
    let mut right_buttons = right.button_assignments.clone();
    left_buttons.sort_by_key(|assignment| assignment.protocol_button_id);
    right_buttons.sort_by_key(|assignment| assignment.protocol_button_id);
    left.dpi.current == right.dpi.current
        && left.dpi.active_stage_id == right.dpi.active_stage_id
        && left_stages == right_stages
        && left.polling == right.polling
        && left_buttons == right_buttons
}

fn polling_transition_allowed(contract: &CapabilityContract, from: u16, to: u16) -> bool {
    match &contract
        .fields
        .iter()
        .find(|rule| rule.field == "polling_hz")
        .expect("registered polling rule")
        .rule
    {
        FieldEvidenceRule::PollingTransitions(transitions) => {
            transitions.contains(&(from, to))
                || (from == to
                    && transitions
                        .iter()
                        .any(|(source, destination)| *source == from || *destination == from))
        }
        _ => false,
    }
}

fn validate_side_assignment(assignment: &RawButtonAssignment, slot: u8) -> Result<(), String> {
    // The GET mode byte is intentionally not interpreted: repository evidence
    // documents that it does not round-trip for every side slot.
    let pass_through = assignment.function_id == FUNCTION_BUTTON_CODE
        && assignment.data_size == 1
        && assignment.data[0] == slot
        && assignment.data[1..] == [0; 4];
    let expected_key = if slot == BUTTON_MOUSE4_SLOT {
        0x43
    } else {
        0x44
    };
    let measured_chord = assignment.function_id == FUNCTION_KEY_CODE
        && assignment.data_size == 2
        && assignment.data[0] == MODIFIER_LEFT_CONTROL | MODIFIER_LEFT_ALT
        && assignment.data[1] == expected_key
        && assignment.data[2..] == [0; 3];
    if pass_through || measured_chord {
        Ok(())
    } else {
        Err(format!(
            "baseline side assignment for slot 0x{slot:02x} is not an unambiguous measured pass-through or measured chord"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ALL_BUTTON_IDS, DpiStage, DpiState, FirmwareVersion, PROFILE_ID, PollingState,
        SNAPSHOT_SCHEMA_VERSION,
    };

    fn measured_intent() -> ProfileIntentV1 {
        ProfileIntentV1 {
            schema_version: crate::profile_intent::PROFILE_INTENT_SCHEMA_VERSION,
            name: "Local Custom Draft".into(),
            dpi: DpiTarget { x: 1600, y: 1600 },
            polling_hz: 4000,
            mouse4: MEASURED_MOUSE4_ACTIONS[1].clone(),
            mouse5: MEASURED_MOUSE5_ACTIONS[1].clone(),
        }
    }

    fn snapshot() -> DeviceSnapshotV1 {
        DeviceSnapshotV1 {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            captured_at_unix_ms: 100,
            device: DeviceIdentity {
                vendor_id: 0x1532,
                product_id: 0x00e6,
                hid_descriptor_serial: Some("HID-A".into()),
                serial: "SERIAL-A".into(),
                firmware: FirmwareVersion { major: 1, minor: 4 },
                path: "hid#device-a".into(),
                interface_number: 3,
                usage_page: 0x000c,
                usage: 0x0001,
            },
            polling: PollingState {
                transport: TransportKind::WirelessV2,
                raw_code: 0x08,
            },
            dpi: DpiState {
                profile_id: PROFILE_ID,
                current: DpiPair { x: 1600, y: 1600 },
                active_stage_id: 1,
                stages: vec![DpiStage {
                    id: 1,
                    x: 1600,
                    y: 1600,
                }],
            },
            power: None,
            button_assignments: ALL_BUTTON_IDS
                .into_iter()
                .map(|id| RawButtonAssignment::mouse_button(id, id))
                .collect(),
        }
    }

    fn request() -> DraftRequestV1 {
        DraftRequestV1 {
            schema_version: DRAFT_REQUEST_SCHEMA_VERSION,
            contract_revision: CAPABILITY_CONTRACT_REVISION,
            target_scope: CapabilityScope::RECORDED_WIRELESS_VIPER_V4_PRO,
            intent: measured_intent(),
        }
    }

    fn category(result: Result<(), PreflightRejection>) -> GateError {
        result.unwrap_err().category
    }

    #[test]
    fn contract_lookup_and_offline_assessment_fail_closed() {
        let scope = CapabilityScope::RECORDED_WIRELESS_VIPER_V4_PRO;
        assert_eq!(
            lookup_contract(scope).map(|contract| contract.revision),
            Some(1)
        );
        assert!(
            lookup_contract(CapabilityScope {
                firmware_minor: 5,
                ..scope
            })
            .is_none()
        );
        assert!(
            lookup_contract(CapabilityScope {
                transport: TransportKind::WiredLegacy,
                ..scope
            })
            .is_none()
        );
        let assessment = assess_draft(&measured_intent(), None).unwrap();
        assert!(!assessment.can_apply());
        assert!(
            assessment
                .blockers
                .iter()
                .any(|blocker| blocker.contains("No verified device scope"))
        );
        assert!(
            assessment
                .fields
                .iter()
                .filter(|field| field.label != "Name")
                .all(|field| field.evidence == FieldCapability::ResearchRequired)
        );
        assert_eq!(
            RECORDED_CONTRACT.readback_policy,
            ReadbackPolicy::FreshIndependentGetAfterEachSetter
        );
        assert_eq!(
            RECORDED_CONTRACT.recovery_policy,
            RecoveryPolicy::StopOnAmbiguousReadbackAndRequireReviewedBaselineRestoration
        );
    }

    #[test]
    fn strict_request_json_rejects_future_unknown_and_nested_unknown_fields() {
        let request = request();
        let json = request.to_json().unwrap();
        assert_eq!(DraftRequestV1::from_json(&json).unwrap(), request);
        assert!(DraftRequestV1::from_json("{").is_err());
        assert!(
            DraftRequestV1::from_json(
                &json.replace("\"schema_version\":1", "\"schema_version\":2")
            )
            .is_err()
        );
        assert!(
            DraftRequestV1::from_json(&json.replace(
                "\"contract_revision\":1",
                "\"extra\":true,\"contract_revision\":1"
            ))
            .is_err()
        );
        assert!(
            DraftRequestV1::from_json(
                &json.replace("\"intent\":{", "\"intent\":{\"future_field\":0,")
            )
            .is_err()
        );
        assert!(
            DraftRequestV1::from_json(&json.replace(
                "\"transport\":\"wireless_v2\"",
                "\"transport\":\"future_transport\""
            ))
            .is_err()
        );
    }

    #[test]
    fn preflight_rejects_missing_incomplete_and_full_identity_mismatches() {
        let request = request();
        let complete = snapshot();
        assert_eq!(
            category(preflight_draft_request(
                &request,
                None,
                Some(&complete),
                Some(&complete)
            )),
            GateError::MissingBaseline
        );
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&complete),
                None,
                Some(&complete)
            )),
            GateError::MissingExpectedState
        );
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&complete),
                Some(&complete),
                None
            )),
            GateError::MissingCurrentState
        );
        let mut incomplete = complete.clone();
        incomplete.button_assignments.pop();
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&incomplete),
                Some(&incomplete),
                Some(&incomplete)
            )),
            GateError::InvalidSnapshot
        );

        let mut other = complete.clone();
        other.device.serial = "SERIAL-B".into();
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&complete),
                Some(&other),
                Some(&other)
            )),
            GateError::IdentityMismatch
        );
        other.device.path.clone_from(&complete.device.path);
        other.device.hid_descriptor_serial = Some("HID-B".into());
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&complete),
                Some(&other),
                Some(&other)
            )),
            GateError::IdentityMismatch
        );
        other.device.serial.clone_from(&complete.device.serial);
        other.device.path = "hid#device-b".into();
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&complete),
                Some(&other),
                Some(&other)
            )),
            GateError::IdentityMismatch
        );
    }

    #[test]
    fn preflight_requires_fresh_logical_state_and_unambiguous_snapshot() {
        let request = request();
        let baseline = snapshot();
        let mut current = baseline.clone();
        current.captured_at_unix_ms += 1;
        current.power = Some(crate::model::WirelessPowerSettings {
            battery_raw: 1,
            idle_seconds: 60,
            low_power_threshold_raw: 13,
        });
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&baseline),
                Some(&baseline),
                Some(&current)
            )),
            GateError::CustomCombinationUnapproved
        );

        let mut reordered = baseline.clone();
        reordered.captured_at_unix_ms += 2;
        reordered.dpi.stages.reverse();
        reordered.button_assignments.reverse();
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&baseline),
                Some(&baseline),
                Some(&reordered)
            )),
            GateError::CustomCombinationUnapproved
        );

        current.polling.raw_code = 0x02;
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&baseline),
                Some(&baseline),
                Some(&current)
            )),
            GateError::StaleExpectedState
        );

        let mut ambiguous = baseline.clone();
        ambiguous.polling.raw_code = 0x80;
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&ambiguous),
                Some(&ambiguous),
                Some(&ambiguous)
            )),
            GateError::AmbiguousBaseline
        );
        let mut ambiguous = baseline.clone();
        ambiguous
            .button_assignments
            .iter_mut()
            .find(|assignment| assignment.protocol_button_id == BUTTON_MOUSE4_SLOT)
            .unwrap()
            .data[0] = BUTTON_MOUSE5_SLOT;
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&ambiguous),
                Some(&ambiguous),
                Some(&ambiguous)
            )),
            GateError::AmbiguousBaseline
        );
    }

    #[test]
    fn preflight_is_source_aware_and_never_promotes_full_measured_values() {
        let request = request();
        let baseline = snapshot();
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&baseline),
                Some(&baseline),
                Some(&baseline)
            )),
            GateError::CustomCombinationUnapproved
        );

        let mut from_8000 = baseline.clone();
        from_8000.polling.raw_code = 0x01;
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&from_8000),
                Some(&from_8000),
                Some(&from_8000)
            )),
            GateError::UnsupportedPollingTransition
        );

        let mut changed_dpi = request.clone();
        changed_dpi.intent.dpi = DpiTarget { x: 800, y: 800 };
        assert_eq!(
            category(preflight_draft_request(
                &changed_dpi,
                Some(&baseline),
                Some(&baseline),
                Some(&baseline)
            )),
            GateError::UnsupportedDpi
        );
        let mut changed_dpi_state = baseline.clone();
        changed_dpi_state.dpi.current = DpiPair { x: 800, y: 800 };
        changed_dpi_state.dpi.stages[0].x = 800;
        changed_dpi_state.dpi.stages[0].y = 800;
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&changed_dpi_state),
                Some(&changed_dpi_state),
                Some(&changed_dpi_state)
            )),
            GateError::UnsupportedDpi
        );

        let mut unsupported_button = request.clone();
        unsupported_button.intent.mouse4 = ButtonActionIntent::Unassigned;
        assert_eq!(
            category(preflight_draft_request(
                &unsupported_button,
                Some(&baseline),
                Some(&baseline),
                Some(&baseline)
            )),
            GateError::UnsupportedButtonAction
        );

        let mut swapped_shortcut = request.clone();
        swapped_shortcut.intent.mouse4 = MEASURED_MOUSE5_ACTIONS[1].clone();
        swapped_shortcut.intent.mouse5 = MEASURED_MOUSE4_ACTIONS[1].clone();
        assert_eq!(
            category(preflight_draft_request(
                &swapped_shortcut,
                Some(&baseline),
                Some(&baseline),
                Some(&baseline)
            )),
            GateError::UnsupportedButtonAction
        );
    }

    #[test]
    fn request_scope_mismatch_is_rejected_without_inheriting_evidence() {
        let mut request = request();
        request.target_scope.firmware_minor = 5;
        let baseline = snapshot();
        assert_eq!(
            category(preflight_draft_request(
                &request,
                Some(&baseline),
                Some(&baseline),
                Some(&baseline)
            )),
            GateError::UnsupportedContractOrTarget
        );

        let assessment = assess_draft(&measured_intent(), Some(request.target_scope)).unwrap();
        assert!(
            assessment
                .fields
                .iter()
                .filter(|field| field.label != "Name")
                .all(|field| field.evidence == FieldCapability::ResearchRequired)
        );
        assert!(
            assessment
                .blockers
                .iter()
                .any(|blocker| blocker.contains("No recorded evidence contract"))
        );
    }
}
