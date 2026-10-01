//! Offline projection of the guarded preset aliases and local profile drafts.
//!
//! Added 2026-09-29 as opt-in derived data; production V1 stores stay unchanged.
//!
//! The catalog is derived data, not a replacement store. Preset aliases retain
//! their original IDs and source presets. Drafts are frozen copies namespaced
//! as `draft:<source-id>` so they cannot shadow alias IDs. Nothing in this
//! module creates a write plan or enables custom-profile application.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    capability::{
        CapabilityScope, DraftRequestV1, GateError, PreflightRejection, preflight_draft_request,
    },
    model::{DeviceSnapshotV1, ProfileName},
    profile_intent::{ProfileDraftLibraryV1, ProfileIntentV1},
    profile_library::ProfileLibraryV1,
};

/// Catalog format version. A version change requires explicit migration.
pub const PROFILE_CATALOG_SCHEMA_VERSION: u32 = 1;
/// Maximum projected entries, matching the two bounded source libraries.
pub const MAX_PROFILE_CATALOG_ENTRIES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogEntryKind {
    PresetAlias,
    LocalDraft,
}

/// One namespaced catalog item. Struct variants keep strict decoding for all
/// variants, including those that may be added by a later schema revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfileCatalogEntryV1 {
    PresetAlias {
        id: String,
        name: String,
        source_preset: ProfileName,
    },
    LocalDraft {
        id: String,
        source_draft_id: String,
        intent: ProfileIntentV1,
    },
}

impl ProfileCatalogEntryV1 {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::PresetAlias { id, .. } | Self::LocalDraft { id, .. } => id,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::PresetAlias { name, .. } => name,
            Self::LocalDraft { intent, .. } => &intent.name,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> CatalogEntryKind {
        match self {
            Self::PresetAlias { .. } => CatalogEntryKind::PresetAlias,
            Self::LocalDraft { .. } => CatalogEntryKind::LocalDraft,
        }
    }
}

/// A deterministic, read-only projection of the current alias and draft files.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileCatalogV1 {
    schema_version: u32,
    entries: Vec<ProfileCatalogEntryV1>,
    quick_switch: [String; 2],
}

impl ProfileCatalogV1 {
    /// Freeze the current sources into a catalog. Later source edits require
    /// calling this migration again; this value never tracks source mutation.
    pub fn from_sources(
        aliases: &ProfileLibraryV1,
        drafts: &ProfileDraftLibraryV1,
    ) -> Result<Self, String> {
        aliases.validate()?;
        drafts.validate()?;

        let mut entries = aliases
            .entries()
            .iter()
            .map(|alias| ProfileCatalogEntryV1::PresetAlias {
                id: alias.id().to_owned(),
                name: alias.name().to_owned(),
                source_preset: alias.source_preset(),
            })
            .collect::<Vec<_>>();
        entries.extend(
            drafts
                .entries()
                .iter()
                .map(|draft| ProfileCatalogEntryV1::LocalDraft {
                    id: draft_catalog_id(draft.id()),
                    source_draft_id: draft.id().to_owned(),
                    intent: draft.intent().clone(),
                }),
        );

        let catalog = Self {
            schema_version: PROFILE_CATALOG_SCHEMA_VERSION,
            entries,
            quick_switch: aliases.quick_switch().clone(),
        };
        catalog.validate()?;
        Ok(catalog)
    }

    #[must_use]
    pub fn entries(&self) -> &[ProfileCatalogEntryV1] {
        &self.entries
    }

    /// The selected pair remains the exact pair of legacy preset aliases.
    #[must_use]
    pub fn quick_switch(&self) -> &[String; 2] {
        &self.quick_switch
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PROFILE_CATALOG_SCHEMA_VERSION {
            return Err(format!(
                "unsupported profile catalog schema {}, expected {PROFILE_CATALOG_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if !(2..=MAX_PROFILE_CATALOG_ENTRIES).contains(&self.entries.len()) {
            return Err(format!(
                "a catalog must contain 2 to {MAX_PROFILE_CATALOG_ENTRIES} entries"
            ));
        }

        let mut catalog_ids = HashSet::new();
        let mut aliases = Vec::new();
        let mut drafts = ProfileDraftLibraryV1::default();
        for entry in &self.entries {
            if !catalog_ids.insert(entry.id()) {
                return Err(format!("duplicate catalog entry ID {:?}", entry.id()));
            }
            match entry {
                ProfileCatalogEntryV1::PresetAlias {
                    id,
                    name,
                    source_preset,
                } => aliases.push(serde_json::json!({
                    "id": id,
                    "name": name,
                    "source_preset": source_preset,
                })),
                ProfileCatalogEntryV1::LocalDraft {
                    id,
                    source_draft_id,
                    intent,
                } => {
                    if id != &draft_catalog_id(source_draft_id) {
                        return Err(format!(
                            "catalog draft ID {id:?} does not match source draft ID {source_draft_id:?}"
                        ));
                    }
                    drafts.create(source_draft_id, intent.clone())?;
                }
            }
        }

        // Reuse the complete legacy validator so alias IDs, names, built-ins,
        // and quick-pair behavior remain governed by the existing contract.
        let aliases_value = serde_json::json!({
            "schema_version": crate::profile_library::LIBRARY_SCHEMA_VERSION,
            "entries": aliases,
            "quick_switch": self.quick_switch,
        });
        let aliases: ProfileLibraryV1 = serde_json::from_value(aliases_value)
            .map_err(|error| format!("invalid catalog preset aliases: {error}"))?;
        aliases
            .validate()
            .map_err(|error| format!("invalid catalog preset aliases: {error}"))?;
        drafts
            .validate()
            .map_err(|error| format!("invalid catalog local drafts: {error}"))?;

        // Draft IDs use their own namespace and cannot collide with aliases;
        // name collisions across the two entry kinds remain explicitly valid.
        for entry in &self.entries {
            if matches!(entry, ProfileCatalogEntryV1::LocalDraft { .. })
                && self.entries.iter().any(|candidate| {
                    matches!(candidate, ProfileCatalogEntryV1::PresetAlias { id, .. } if id == entry.id())
                })
            {
                return Err(format!(
                    "draft catalog ID {:?} collides with a preset alias",
                    entry.id()
                ));
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|error| format!("encode profile catalog: {error}"))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(json)
            .map_err(|error| format!("decode profile catalog: {error}"))?;
        catalog.validate()?;
        Ok(catalog)
    }

    /// Create a versioned request from the frozen catalog copy. This is data
    /// preparation only; it does not approve or execute a custom profile.
    pub fn promotion_request(
        &self,
        catalog_draft_id: &str,
        target_scope: CapabilityScope,
    ) -> Result<DraftRequestV1, String> {
        self.validate()?;
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.id() == catalog_draft_id)
            .ok_or_else(|| format!("unknown catalog draft ID {catalog_draft_id:?}"))?;
        let ProfileCatalogEntryV1::LocalDraft { intent, .. } = entry else {
            return Err("preset aliases are not custom draft requests".to_owned());
        };
        let request = DraftRequestV1 {
            schema_version: crate::capability::DRAFT_REQUEST_SCHEMA_VERSION,
            contract_revision: crate::capability::CAPABILITY_CONTRACT_REVISION,
            target_scope,
            intent: intent.clone(),
        };
        request.validate()?;
        Ok(request)
    }

    /// Read-only promotion preflight delegates to the capability gate. Even a
    /// structurally valid request remains blocked pending combination evidence.
    pub fn preflight_draft(
        &self,
        catalog_draft_id: &str,
        target_scope: CapabilityScope,
        baseline: Option<&DeviceSnapshotV1>,
        expected: Option<&DeviceSnapshotV1>,
        current: Option<&DeviceSnapshotV1>,
    ) -> Result<(), PreflightRejection> {
        let request = self
            .promotion_request(catalog_draft_id, target_scope)
            .map_err(|reason| PreflightRejection {
                category: GateError::InvalidRequest,
                reason,
            })?;
        preflight_draft_request(&request, baseline, expected, current)
    }
}

fn draft_catalog_id(source_draft_id: &str) -> String {
    format!("draft:{source_draft_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        capability::{CapabilityScope, GateError},
        model::{
            BUTTON_DPI, BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_MOUSE4_SLOT, BUTTON_MOUSE5_SLOT,
            BUTTON_RIGHT, DeviceIdentity, DeviceSnapshotV1, DpiPair, DpiStage, DpiState,
            FirmwareVersion, PollingState, RAZER_VENDOR_ID, RawButtonAssignment,
            SNAPSHOT_SCHEMA_VERSION, TransportKind, VIPER_V4_PRO_WIRELESS_PID,
        },
        profile_intent::{
            ButtonActionIntent, DpiTarget, KeyboardKey, PROFILE_INTENT_SCHEMA_VERSION,
        },
    };

    fn intent(name: &str) -> ProfileIntentV1 {
        ProfileIntentV1 {
            schema_version: PROFILE_INTENT_SCHEMA_VERSION,
            name: name.to_owned(),
            dpi: DpiTarget { x: 1200, y: 1600 },
            polling_hz: 2000,
            mouse4: ButtonActionIntent::KeyboardShortcut {
                control: true,
                alt: true,
                shift: false,
                windows: false,
                key: KeyboardKey::Z,
            },
            mouse5: ButtonActionIntent::PassThrough,
        }
    }

    fn sources() -> (ProfileLibraryV1, ProfileDraftLibraryV1) {
        let aliases = ProfileLibraryV1::default();
        let mut drafts = ProfileDraftLibraryV1::default();
        drafts.create("developer", intent("Developer")).unwrap();
        drafts.create("travel", intent("Travel")).unwrap();
        (aliases, drafts)
    }

    fn scope() -> CapabilityScope {
        CapabilityScope::RECORDED_WIRELESS_VIPER_V4_PRO
    }

    fn measured_intent(name: &str) -> ProfileIntentV1 {
        ProfileIntentV1 {
            schema_version: PROFILE_INTENT_SCHEMA_VERSION,
            name: name.to_owned(),
            dpi: DpiTarget { x: 1600, y: 1600 },
            polling_hz: 1000,
            mouse4: ButtonActionIntent::PassThrough,
            mouse5: ButtonActionIntent::PassThrough,
        }
    }

    fn snapshot() -> DeviceSnapshotV1 {
        DeviceSnapshotV1 {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            captured_at_unix_ms: 1,
            device: DeviceIdentity {
                vendor_id: RAZER_VENDOR_ID,
                product_id: VIPER_V4_PRO_WIRELESS_PID,
                hid_descriptor_serial: Some("descriptor".to_owned()),
                serial: "serial".to_owned(),
                firmware: FirmwareVersion { major: 1, minor: 4 },
                path: "hid-path".to_owned(),
                interface_number: 3,
                usage_page: 0x000c,
                usage: 0x0001,
            },
            polling: PollingState {
                transport: TransportKind::WirelessV2,
                raw_code: 0x08,
            },
            dpi: DpiState {
                profile_id: 1,
                current: DpiPair { x: 1600, y: 1600 },
                active_stage_id: 1,
                stages: vec![DpiStage {
                    id: 1,
                    x: 1600,
                    y: 1600,
                }],
            },
            power: None,
            button_assignments: [
                BUTTON_LEFT,
                BUTTON_RIGHT,
                BUTTON_MIDDLE,
                BUTTON_MOUSE4_SLOT,
                BUTTON_MOUSE5_SLOT,
                BUTTON_DPI,
            ]
            .into_iter()
            .map(|slot| RawButtonAssignment::mouse_button(slot, slot))
            .collect(),
        }
    }

    #[test]
    fn migration_is_deterministic_and_preserves_alias_pair_and_builtin_sources() {
        let (aliases, drafts) = sources();
        let first = ProfileCatalogV1::from_sources(&aliases, &drafts).unwrap();
        let second = ProfileCatalogV1::from_sources(&aliases, &drafts).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.quick_switch(), aliases.quick_switch());
        assert_eq!(
            &first.entries()[0..2],
            &[
                ProfileCatalogEntryV1::PresetAlias {
                    id: "developer".into(),
                    name: "Developer".into(),
                    source_preset: ProfileName::Developer,
                },
                ProfileCatalogEntryV1::PresetAlias {
                    id: "gaming".into(),
                    name: "Gaming".into(),
                    source_preset: ProfileName::Gaming,
                },
            ]
        );
        assert!(
            matches!(first.entries()[2], ProfileCatalogEntryV1::LocalDraft { ref id, ref source_draft_id, .. } if id == "draft:developer" && source_draft_id == "developer")
        );
    }

    #[test]
    fn migration_freezes_draft_copy_and_separates_colliding_name_and_id() {
        let (aliases, mut drafts) = sources();
        let catalog = ProfileCatalogV1::from_sources(&aliases, &drafts).unwrap();
        let request = catalog
            .promotion_request("draft:developer", scope())
            .unwrap();
        assert_eq!(request.intent.name, "Developer");
        drafts.rename("developer", "Renamed later").unwrap();
        assert_eq!(request.intent.name, "Developer");
        let refreshed = ProfileCatalogV1::from_sources(&aliases, &drafts).unwrap();
        assert_eq!(
            refreshed
                .promotion_request("draft:developer", scope())
                .unwrap()
                .intent
                .name,
            "Renamed later"
        );
        assert!(catalog.promotion_request("developer", scope()).is_err());
    }

    #[test]
    fn strict_json_round_trips_and_rejects_future_or_corrupt_entries() {
        let (aliases, drafts) = sources();
        let catalog = ProfileCatalogV1::from_sources(&aliases, &drafts).unwrap();
        let json = catalog.to_json().unwrap();
        assert_eq!(ProfileCatalogV1::from_json(&json).unwrap(), catalog);

        let mut future: serde_json::Value = serde_json::from_str(&json).unwrap();
        future["schema_version"] = serde_json::json!(2);
        assert!(ProfileCatalogV1::from_json(&future.to_string()).is_err());

        let mut unknown: serde_json::Value = serde_json::from_str(&json).unwrap();
        unknown["entries"][2]["unknown"] = serde_json::json!(true);
        assert!(ProfileCatalogV1::from_json(&unknown.to_string()).is_err());

        let mut unknown_variant: serde_json::Value = serde_json::from_str(&json).unwrap();
        unknown_variant["entries"][2]["kind"] = serde_json::json!("future_entry");
        assert!(ProfileCatalogV1::from_json(&unknown_variant.to_string()).is_err());

        let mut corrupt: serde_json::Value = serde_json::from_str(&json).unwrap();
        corrupt["entries"][2]["id"] = serde_json::json!("travel");
        assert!(ProfileCatalogV1::from_json(&corrupt.to_string()).is_err());
    }

    #[test]
    fn aliases_cannot_become_draft_requests_and_valid_drafts_stay_blocked() {
        let aliases = ProfileLibraryV1::default();
        let mut drafts = ProfileDraftLibraryV1::default();
        drafts.create("travel", measured_intent("Travel")).unwrap();
        let catalog = ProfileCatalogV1::from_sources(&aliases, &drafts).unwrap();
        assert!(catalog.promotion_request("developer", scope()).is_err());
        let baseline = snapshot();
        let result = catalog.preflight_draft(
            "draft:travel",
            scope(),
            Some(&baseline),
            Some(&baseline),
            Some(&baseline),
        );
        assert_eq!(
            result.unwrap_err().category,
            GateError::CustomCombinationUnapproved
        );
    }
}
