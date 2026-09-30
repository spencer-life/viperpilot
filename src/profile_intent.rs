//! User-owned semantic profile intent, added 2026-09-29.
//!
//! This model is deliberately independent of the Developer/Gaming personal
//! presets and of protocol bytes. It validates user intent and reports measured
//! field evidence only; it does not create a hardware plan or authorize writes.
//!
//! Capability provenance (2026-09-29): the repository records measurements for
//! the wireless Razer Viper V4 Pro (VID 0x1532, PID 0x00e6): 1000 <-> 4000 Hz
//! polling transitions, isolated Mouse4/Mouse5 tests for Ctrl+Alt+F10/F11, and
//! native pass-through behavior for Mouse4/Mouse5. DPI was forced to 1600 while
//! already at 1600 and independently read back; this is observed-value evidence,
//! not proof of a change from another DPI. Per-field evidence does not prove an
//! arbitrary combination of fields is safe as one profile.

use serde::{Deserialize, Deserializer, Serialize};

pub const PROFILE_INTENT_SCHEMA_VERSION: u32 = 1;
const MAX_PROFILE_NAME_CHARS: usize = 64;

/// A user-authored configuration request, not a hardware profile or write plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileIntentV1 {
    pub schema_version: u32,
    pub name: String,
    pub dpi: DpiTarget,
    pub polling_hz: u16,
    pub mouse4: ButtonActionIntent,
    pub mouse5: ButtonActionIntent,
}

impl ProfileIntentV1 {
    /// Validate that this intent is well-formed. This does not assess device support.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PROFILE_INTENT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported profile intent schema {}, expected {PROFILE_INTENT_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.name.trim() != self.name
            || self.name.is_empty()
            || self.name.chars().count() > MAX_PROFILE_NAME_CHARS
            || self.name.chars().any(char::is_control)
        {
            return Err(format!(
                "profile names must contain 1 to {MAX_PROFILE_NAME_CHARS} characters, without surrounding whitespace or control characters"
            ));
        }
        if self.dpi.x == 0 || self.dpi.y == 0 {
            return Err("DPI X and Y targets must both be greater than zero".to_owned());
        }
        if self.polling_hz == 0 {
            return Err("polling target must be greater than zero".to_owned());
        }
        validate_action(&self.mouse4)?;
        validate_action(&self.mouse5)?;
        Ok(())
    }

    /// Report evidence for individual fields on a target device.
    ///
    /// A measured field result is not a write authorization. In particular, the
    /// returned fields do not establish that their combination is safe to apply.
    pub fn assess_fields(&self, target: CapabilityTarget) -> Result<FieldAssessments, String> {
        self.validate()?;
        let Some(transport) = crate::model::TransportKind::for_product_id(target.product_id).ok()
        else {
            return Ok(FieldAssessments {
                target,
                name: FieldCapability::LocalOnly,
                dpi: FieldCapability::ResearchRequired,
                polling: FieldCapability::ResearchRequired,
                mouse4: FieldCapability::ResearchRequired,
                mouse5: FieldCapability::ResearchRequired,
            });
        };
        Ok(crate::capability::classify_intent(
            self,
            Some(crate::capability::CapabilityScope {
                vendor_id: target.vendor_id,
                product_id: target.product_id,
                firmware_major: target.firmware_major,
                firmware_minor: target.firmware_minor,
                transport,
                interface_number: target.interface_number,
                usage_page: target.usage_page,
                usage: target.usage,
            }),
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DpiTarget {
    pub x: u16,
    pub y: u16,
}

/// Semantic actions a user can express without storing or accepting raw HID bytes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ButtonActionIntent {
    /// Keep the button's native mouse-button behavior.
    PassThrough,
    /// Request a keyboard chord. Most combinations require device-specific research.
    KeyboardShortcut {
        control: bool,
        alt: bool,
        shift: bool,
        windows: bool,
        key: KeyboardKey,
    },
    /// Leave the button without an assigned action, subject to device support research.
    Unassigned,
    /// Preserve an idea this schema cannot model yet; always research-only.
    Unmodeled { description: String },
}

// Serde's internally tagged unit-variant visitor ignores extra map entries,
// even when `deny_unknown_fields` is set on the enum. Decode unit cases through
// strict empty structs so malformed/future action properties are rejected while
// the public enum and serialized JSON shape remain unchanged.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StrictButtonActionIntent {
    PassThrough(StrictEmptyAction),
    KeyboardShortcut {
        control: bool,
        alt: bool,
        shift: bool,
        windows: bool,
        key: KeyboardKey,
    },
    Unassigned(StrictEmptyAction),
    Unmodeled {
        description: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictEmptyAction {}

impl<'de> Deserialize<'de> for ButtonActionIntent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match StrictButtonActionIntent::deserialize(deserializer)? {
            StrictButtonActionIntent::PassThrough(StrictEmptyAction {}) => Ok(Self::PassThrough),
            StrictButtonActionIntent::KeyboardShortcut {
                control,
                alt,
                shift,
                windows,
                key,
            } => Ok(Self::KeyboardShortcut {
                control,
                alt,
                shift,
                windows,
                key,
            }),
            StrictButtonActionIntent::Unassigned(StrictEmptyAction {}) => Ok(Self::Unassigned),
            StrictButtonActionIntent::Unmodeled { description } => {
                Ok(Self::Unmodeled { description })
            }
        }
    }
}

/// Semantic key names allow a broad set of user choices without raw key bytes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Enter,
    Escape,
    Space,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

/// Checked model/firmware/HID-interface scope for capability comparison.
///
/// This omits serial and path intentionally: it describes the scope of field
/// evidence, not a selected physical device and not permission to apply intent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityTarget {
    pub vendor_id: u16,
    pub product_id: u16,
    pub firmware_major: u8,
    pub firmware_minor: u8,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
}

impl CapabilityTarget {
    /// Measured wireless Viper V4 Pro, firmware 1.4, `MI_03`, usage 0x000c:0x0001.
    pub const MEASURED_WIRELESS_VIPER_V4_PRO_FW_1_4_MI_03: Self = Self {
        vendor_id: 0x1532,
        product_id: 0x00e6,
        firmware_major: 1,
        firmware_minor: 4,
        interface_number: 3,
        usage_page: 0x000c,
        usage: 0x0001,
    };
}

impl From<&crate::model::DeviceIdentity> for CapabilityTarget {
    fn from(identity: &crate::model::DeviceIdentity) -> Self {
        Self {
            vendor_id: identity.vendor_id,
            product_id: identity.product_id,
            firmware_major: identity.firmware.major,
            firmware_minor: identity.firmware.minor,
            interface_number: identity.interface_number,
            usage_page: identity.usage_page,
            usage: identity.usage,
        }
    }
}

/// Evidence classification only; none of these values grants permission to write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldCapability {
    /// A name exists in this app and does not describe a device field.
    LocalOnly,
    /// The requested transition was physically measured on the named device.
    MeasuredTransition,
    /// This value was observed/read back, but a transition to it was not proven.
    ObservedValueOnly,
    /// Evidence is missing for this value or device.
    ResearchRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldAssessments {
    pub target: CapabilityTarget,
    pub name: FieldCapability,
    pub dpi: FieldCapability,
    pub polling: FieldCapability,
    pub mouse4: FieldCapability,
    pub mouse5: FieldCapability,
}

fn validate_action(action: &ButtonActionIntent) -> Result<(), String> {
    if let ButtonActionIntent::Unmodeled { description } = action
        && (description.trim() != description
            || description.is_empty()
            || description.chars().count() > 160
            || description.chars().any(char::is_control))
    {
        return Err(
            "unmodeled action descriptions must contain 1 to 160 characters without surrounding whitespace or control characters"
                .to_owned(),
        );
    }
    Ok(())
}

pub const PROFILE_DRAFT_LIBRARY_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROFILE_DRAFTS: usize = 32;

/// Local editable drafts. Entries have stable IDs separate from user-editable names.
/// This type has no planner conversion and never grants apply permission.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDraftLibraryV1 {
    schema_version: u32,
    entries: Vec<ProfileDraft>,
}

impl Default for ProfileDraftLibraryV1 {
    fn default() -> Self {
        Self {
            schema_version: PROFILE_DRAFT_LIBRARY_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDraft {
    id: String,
    intent: ProfileIntentV1,
}

impl ProfileDraft {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn intent(&self) -> &ProfileIntentV1 {
        &self.intent
    }
}

impl ProfileDraftLibraryV1 {
    #[must_use]
    pub fn entries(&self) -> &[ProfileDraft] {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Result<&ProfileDraft, String> {
        self.entries
            .iter()
            .find(|draft| draft.id == id)
            .ok_or_else(|| format!("unknown profile draft ID {id:?}"))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PROFILE_DRAFT_LIBRARY_SCHEMA_VERSION {
            return Err(format!(
                "unsupported profile draft library schema {}",
                self.schema_version
            ));
        }
        if self.entries.len() > MAX_PROFILE_DRAFTS {
            return Err(format!(
                "a draft library can contain at most {MAX_PROFILE_DRAFTS} entries"
            ));
        }
        let mut ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for draft in &self.entries {
            validate_draft_id(&draft.id)?;
            draft.intent.validate()?;
            if !ids.insert(draft.id.as_str()) {
                return Err(format!("duplicate profile draft ID {:?}", draft.id));
            }
            if !names.insert(draft.intent.name.to_lowercase()) {
                return Err(format!(
                    "duplicate profile draft name {:?}",
                    draft.intent.name
                ));
            }
        }
        Ok(())
    }

    /// Add a new user-authored draft using a caller-supplied stable ID.
    pub fn create(&mut self, id: &str, intent: ProfileIntentV1) -> Result<(), String> {
        self.validate()?;
        let mut next = self.clone();
        next.entries.push(ProfileDraft {
            id: id.to_owned(),
            intent,
        });
        self.accept(next)
    }

    /// Change only the display name while retaining the stable draft ID.
    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        self.validate()?;
        let mut next = self.clone();
        let draft = next
            .entries
            .iter_mut()
            .find(|draft| draft.id == id)
            .ok_or_else(|| format!("unknown profile draft ID {id:?}"))?;
        name.clone_into(&mut draft.intent.name);
        self.accept(next)
    }

    /// Replace user settings while retaining the stable draft ID.
    pub fn edit(&mut self, id: &str, intent: ProfileIntentV1) -> Result<(), String> {
        self.validate()?;
        let mut next = self.clone();
        let draft = next
            .entries
            .iter_mut()
            .find(|draft| draft.id == id)
            .ok_or_else(|| format!("unknown profile draft ID {id:?}"))?;
        draft.intent = intent;
        self.accept(next)
    }

    /// Copy settings into a new draft with a new ID and user-chosen name.
    pub fn duplicate(&mut self, source_id: &str, id: &str, name: &str) -> Result<(), String> {
        self.validate()?;
        let mut intent = self.get(source_id)?.intent.clone();
        name.clone_into(&mut intent.name);
        self.create(id, intent)
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        self.validate()?;
        if !self.entries.iter().any(|draft| draft.id == id) {
            return Err(format!("unknown profile draft ID {id:?}"));
        }
        let mut next = self.clone();
        next.entries.retain(|draft| draft.id != id);
        self.accept(next)
    }

    fn accept(&mut self, next: Self) -> Result<(), String> {
        next.validate()?;
        *self = next;
        Ok(())
    }
}

fn validate_draft_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 48
        || !id.as_bytes()[0].is_ascii_lowercase()
        || !id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(
            "draft IDs must start with a-z and contain at most 48 ASCII letters, digits, - or _"
                .to_owned(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom_intent() -> ProfileIntentV1 {
        ProfileIntentV1 {
            schema_version: PROFILE_INTENT_SCHEMA_VERSION,
            name: "My own editing setup".to_owned(),
            dpi: DpiTarget { x: 1200, y: 1600 },
            polling_hz: 2000,
            mouse4: ButtonActionIntent::KeyboardShortcut {
                control: true,
                alt: false,
                shift: true,
                windows: false,
                key: KeyboardKey::Z,
            },
            mouse5: ButtonActionIntent::Unassigned,
        }
    }

    #[test]
    fn user_can_create_and_round_trip_intent_without_personal_presets() {
        let intent = custom_intent();
        intent.validate().unwrap();
        let encoded = serde_json::to_string(&intent).unwrap();
        let decoded: ProfileIntentV1 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, intent);
        assert_ne!(intent.name, "Developer");
        assert_ne!(intent.name, "Gaming");
    }

    #[test]
    fn measured_fields_and_research_required_changes_are_distinguished() {
        let intent = custom_intent();
        let assessment = intent
            .assess_fields(CapabilityTarget::MEASURED_WIRELESS_VIPER_V4_PRO_FW_1_4_MI_03)
            .unwrap();
        assert_eq!(assessment.name, FieldCapability::LocalOnly);
        assert_eq!(assessment.dpi, FieldCapability::ResearchRequired);
        assert_eq!(assessment.polling, FieldCapability::ResearchRequired);
        assert_eq!(assessment.mouse4, FieldCapability::ResearchRequired);
        assert_eq!(assessment.mouse5, FieldCapability::ResearchRequired);

        let measured = ProfileIntentV1 {
            name: "A user-chosen name".to_owned(),
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
            ..custom_intent()
        }
        .assess_fields(CapabilityTarget::MEASURED_WIRELESS_VIPER_V4_PRO_FW_1_4_MI_03)
        .unwrap();
        assert_eq!(measured.dpi, FieldCapability::ObservedValueOnly);
        assert_eq!(measured.polling, FieldCapability::MeasuredTransition);
        assert_eq!(measured.mouse4, FieldCapability::MeasuredTransition);
        assert_eq!(measured.mouse5, FieldCapability::MeasuredTransition);
    }

    #[test]
    fn unmodeled_future_actions_are_valid_intent_but_research_only() {
        let mut intent = custom_intent();
        intent.mouse4 = ButtonActionIntent::Unmodeled {
            description: "Launch an application".to_owned(),
        };
        intent.validate().unwrap();
        let assessment = intent
            .assess_fields(CapabilityTarget::MEASURED_WIRELESS_VIPER_V4_PRO_FW_1_4_MI_03)
            .unwrap();
        assert_eq!(assessment.mouse4, FieldCapability::ResearchRequired);
    }

    #[test]
    fn other_device_never_inherits_viper_measurements() {
        let assessment = ProfileIntentV1 {
            name: "Personal".to_owned(),
            dpi: DpiTarget { x: 1600, y: 1600 },
            polling_hz: 1000,
            mouse4: ButtonActionIntent::PassThrough,
            mouse5: ButtonActionIntent::PassThrough,
            ..custom_intent()
        }
        .assess_fields(CapabilityTarget {
            product_id: 0x00e5,
            ..CapabilityTarget::MEASURED_WIRELESS_VIPER_V4_PRO_FW_1_4_MI_03
        })
        .unwrap();
        assert_eq!(assessment.name, FieldCapability::LocalOnly);
        assert_eq!(assessment.dpi, FieldCapability::ResearchRequired);
        assert_eq!(assessment.polling, FieldCapability::ResearchRequired);
        assert_eq!(assessment.mouse4, FieldCapability::ResearchRequired);
        assert_eq!(assessment.mouse5, FieldCapability::ResearchRequired);
    }

    #[test]
    fn changed_firmware_or_hid_interface_does_not_inherit_measurements() {
        let intent = custom_intent();
        let measured = CapabilityTarget::MEASURED_WIRELESS_VIPER_V4_PRO_FW_1_4_MI_03;
        for target in [
            CapabilityTarget {
                firmware_minor: 5,
                ..measured
            },
            CapabilityTarget {
                interface_number: 2,
                ..measured
            },
            CapabilityTarget {
                usage: 0x0002,
                ..measured
            },
        ] {
            let assessment = intent.assess_fields(target).unwrap();
            assert_eq!(assessment.target, target);
            assert_eq!(assessment.polling, FieldCapability::ResearchRequired);
            assert_eq!(assessment.mouse4, FieldCapability::ResearchRequired);
        }
    }

    #[test]
    fn profile_draft_crud_round_trips_and_keeps_ids_stable() {
        let mut library = ProfileDraftLibraryV1::default();
        let first = custom_intent();
        library.create("custom-one", first.clone()).unwrap();
        assert_eq!(library.get("custom-one").unwrap().intent(), &first);

        library.rename("custom-one", "My renamed setup").unwrap();
        let renamed = library.get("custom-one").unwrap();
        assert_eq!(renamed.id(), "custom-one");
        assert_eq!(renamed.intent().name, "My renamed setup");
        assert_eq!(renamed.intent().dpi, first.dpi);

        let edited = ProfileIntentV1 {
            dpi: DpiTarget { x: 800, y: 800 },
            ..renamed.intent().clone()
        };
        library.edit("custom-one", edited.clone()).unwrap();
        assert_eq!(library.get("custom-one").unwrap().intent(), &edited);

        library
            .duplicate("custom-one", "custom-two", "Copied setup")
            .unwrap();
        assert_eq!(library.entries().len(), 2);
        assert_eq!(
            library.get("custom-two").unwrap().intent().name,
            "Copied setup"
        );
        assert_eq!(library.get("custom-two").unwrap().intent().dpi, edited.dpi);

        let encoded = serde_json::to_string(&library).unwrap();
        let decoded: ProfileDraftLibraryV1 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, library);
        library.delete("custom-one").unwrap();
        assert_eq!(library.entries().len(), 1);
        assert_eq!(library.entries()[0].id(), "custom-two");
    }

    #[test]
    fn failed_draft_mutations_leave_library_unchanged_and_names_are_unique() {
        let mut library = ProfileDraftLibraryV1::default();
        library.create("custom-one", custom_intent()).unwrap();
        let original = library.clone();
        assert!(
            library
                .create(
                    "custom-two",
                    ProfileIntentV1 {
                        name: "my own EDITING setup".to_owned(),
                        ..custom_intent()
                    }
                )
                .is_err()
        );
        assert_eq!(library, original);
        assert!(library.rename("custom-one", "Bad name ").is_err());
        assert_eq!(library, original);
        assert!(library.edit("missing", custom_intent()).is_err());
        assert_eq!(library, original);
        assert!(library.delete("missing").is_err());
        assert_eq!(library, original);
        assert!(
            library
                .duplicate("custom-one", "custom-one", "Copy")
                .is_err()
        );
        assert_eq!(library, original);
    }

    #[test]
    fn draft_library_is_bounded() {
        let mut library = ProfileDraftLibraryV1::default();
        for index in 0..MAX_PROFILE_DRAFTS {
            library
                .create(
                    &format!("custom-{index}"),
                    ProfileIntentV1 {
                        name: format!("Custom profile {index}"),
                        ..custom_intent()
                    },
                )
                .unwrap();
        }
        let full = library.clone();
        assert!(
            library
                .create(
                    "custom-over-limit",
                    ProfileIntentV1 {
                        name: "Over limit".to_owned(),
                        ..custom_intent()
                    }
                )
                .is_err()
        );
        assert_eq!(library, full);
    }

    #[test]
    fn malformed_name_dpi_polling_and_schema_fail_validation() {
        let mut intent = custom_intent();
        intent.name = " trimmed ".to_owned();
        assert!(intent.validate().is_err());

        let mut intent = custom_intent();
        intent.dpi.x = 0;
        assert!(intent.validate().is_err());

        let mut intent = custom_intent();
        intent.polling_hz = 0;
        assert!(intent.validate().is_err());

        let mut intent = custom_intent();
        intent.schema_version += 1;
        assert!(intent.validate().is_err());
    }
}
