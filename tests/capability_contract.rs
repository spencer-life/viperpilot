use viper_v4_utility::capability::{
    CAPABILITY_CONTRACT_REVISION, CapabilityScope, DRAFT_REQUEST_SCHEMA_VERSION, DraftRequestV1,
    GateError, assess_draft, lookup_contract,
};
use viper_v4_utility::model::{
    ALL_BUTTON_IDS, DeviceIdentity, DeviceSnapshotV1, DpiPair, DpiStage, DpiState, FirmwareVersion,
    PROFILE_ID, PollingState, RawButtonAssignment, SNAPSHOT_SCHEMA_VERSION, TransportKind,
    WirelessPowerSettings,
};
use viper_v4_utility::profile_intent::{
    ButtonActionIntent, DpiTarget, FieldCapability, KeyboardKey, PROFILE_INTENT_SCHEMA_VERSION,
    ProfileIntentV1,
};

fn measured_intent() -> ProfileIntentV1 {
    ProfileIntentV1 {
        schema_version: PROFILE_INTENT_SCHEMA_VERSION,
        name: "Local custom draft".into(),
        dpi: DpiTarget { x: 1600, y: 1600 },
        polling_hz: 4000,
        mouse4: ButtonActionIntent::KeyboardShortcut {
            control: true,
            alt: true,
            shift: false,
            windows: false,
            key: KeyboardKey::F10,
        },
        mouse5: ButtonActionIntent::PassThrough,
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
            stages: vec![
                DpiStage {
                    id: 1,
                    x: 1600,
                    y: 1600,
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
fn every_button_action_variant_rejects_nested_unknown_fields() {
    let mut value = serde_json::to_value(request()).unwrap();
    let action_values = [
        serde_json::json!({"kind": "pass_through", "future": true}),
        serde_json::json!({"kind": "unassigned", "future": true}),
        serde_json::json!({
            "kind": "keyboard_shortcut",
            "control": true,
            "alt": true,
            "shift": false,
            "windows": false,
            "key": "f10",
            "future": true
        }),
        serde_json::json!({
            "kind": "unmodeled",
            "description": "future action",
            "future": true
        }),
    ];

    for action in action_values {
        value["intent"]["mouse4"] = action;
        assert!(
            DraftRequestV1::from_json(&value.to_string()).is_err(),
            "accepted unknown field in action: {}",
            value["intent"]["mouse4"]
        );
    }
}

#[test]
fn no_scope_axis_inherits_the_recorded_contract() {
    let recorded = CapabilityScope::RECORDED_WIRELESS_VIPER_V4_PRO;
    let unsupported = [
        CapabilityScope {
            vendor_id: recorded.vendor_id + 1,
            ..recorded
        },
        CapabilityScope {
            product_id: 0x00e7,
            ..recorded
        },
        CapabilityScope {
            firmware_major: recorded.firmware_major + 1,
            ..recorded
        },
        CapabilityScope {
            firmware_minor: recorded.firmware_minor + 1,
            ..recorded
        },
        CapabilityScope {
            transport: TransportKind::WiredLegacy,
            ..recorded
        },
        CapabilityScope {
            interface_number: recorded.interface_number + 1,
            ..recorded
        },
        CapabilityScope {
            usage_page: recorded.usage_page + 1,
            ..recorded
        },
        CapabilityScope {
            usage: recorded.usage + 1,
            ..recorded
        },
    ];

    for scope in unsupported {
        assert!(
            lookup_contract(scope).is_none(),
            "inherited scope: {scope:?}"
        );
        let assessment = assess_draft(&measured_intent(), Some(scope)).unwrap();
        assert!(!assessment.can_apply());
        assert!(
            assessment
                .fields
                .iter()
                .filter(|field| field.label != "Name")
                .all(|field| field.evidence == FieldCapability::ResearchRequired)
        );
    }
}

#[test]
fn independent_readback_ignores_capture_metadata_and_order_only_changes() {
    let req = request();
    let baseline = snapshot();
    let mut current = baseline.clone();
    current.captured_at_unix_ms += 1000;
    current.power = Some(WirelessPowerSettings {
        battery_raw: 200,
        idle_seconds: 300,
        low_power_threshold_raw: 13,
    });
    current.button_assignments.reverse();
    current.dpi.stages.reverse();

    let rejection = viper_v4_utility::capability::preflight_draft_request(
        &req,
        Some(&baseline),
        Some(&baseline),
        Some(&current),
    )
    .unwrap_err();
    assert_eq!(rejection.category, GateError::CustomCombinationUnapproved);
}
