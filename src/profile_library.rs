//! Local profile-library foundation, 2026-09-28.
//!
//! Entries refer to the two existing complete presets; they are not onboard
//! slots, device authorizations, arbitrary assignments, or hardware write plans.
//! The tray may select these local aliases through its existing guarded
//! complete-preset apply path; the library never authorizes a hardware write.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::ProfileName;

pub const LIBRARY_SCHEMA_VERSION: u32 = 1;
pub const MAX_SAVED_PROFILES: usize = 32;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SavedProfile {
    id: String,
    name: String,
    source_preset: ProfileName,
}

impl SavedProfile {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn source_preset(&self) -> ProfileName {
        self.source_preset
    }

    #[must_use]
    pub fn is_builtin(&self) -> bool {
        matches!(self.id.as_str(), "developer" | "gaming")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileLibraryV1 {
    schema_version: u32,
    entries: Vec<SavedProfile>,
    quick_switch: [String; 2],
}

impl Default for ProfileLibraryV1 {
    fn default() -> Self {
        Self {
            schema_version: LIBRARY_SCHEMA_VERSION,
            entries: vec![
                SavedProfile {
                    id: "developer".to_owned(),
                    name: "Developer".to_owned(),
                    source_preset: ProfileName::Developer,
                },
                SavedProfile {
                    id: "gaming".to_owned(),
                    name: "Gaming".to_owned(),
                    source_preset: ProfileName::Gaming,
                },
            ],
            quick_switch: ["developer".to_owned(), "gaming".to_owned()],
        }
    }
}

impl ProfileLibraryV1 {
    #[must_use]
    pub fn entries(&self) -> &[SavedProfile] {
        &self.entries
    }

    #[must_use]
    pub fn quick_switch(&self) -> &[String; 2] {
        &self.quick_switch
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != LIBRARY_SCHEMA_VERSION {
            return Err(format!(
                "unsupported profile library schema {}",
                self.schema_version
            ));
        }
        if !(2..=MAX_SAVED_PROFILES).contains(&self.entries.len()) {
            return Err(format!(
                "a library must contain 2 to {MAX_SAVED_PROFILES} entries"
            ));
        }
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for entry in &self.entries {
            validate_id(&entry.id)?;
            validate_name(&entry.name)?;
            if !ids.insert(entry.id.as_str()) {
                return Err(format!("duplicate profile ID {:?}", entry.id));
            }
            if !names.insert(entry.name.to_lowercase()) {
                return Err(format!("duplicate profile name {:?}", entry.name));
            }
        }
        for builtin in Self::default().entries {
            if self.find(&builtin.id)? != &builtin {
                return Err(format!("built-in profile {:?} is read-only", builtin.id));
            }
        }
        let first = self.find(&self.quick_switch[0])?;
        let second = self.find(&self.quick_switch[1])?;
        if first.source_preset == second.source_preset {
            return Err("quick-switch entries must refer to different complete presets".to_owned());
        }
        Ok(())
    }

    /// Resolve only from a freshly verified hardware profile, never a saved
    /// last-used label. Unknown/out-of-sync state must remain disarmed.
    /// This is a pure selection helper, not permission to send any HID traffic.
    pub fn toggle_target(&self, observed: Option<ProfileName>) -> Result<&SavedProfile, String> {
        self.validate()?;
        let observed = observed.ok_or("quick switch requires a verified current profile")?;
        let first = self.find(&self.quick_switch[0])?;
        let second = self.find(&self.quick_switch[1])?;
        if observed == first.source_preset {
            Ok(second)
        } else if observed == second.source_preset {
            Ok(first)
        } else {
            Err("current profile is outside the quick-switch pair".to_owned())
        }
    }

    /// Make a local named copy, without creating a new hardware combination.
    pub fn duplicate(&mut self, source_id: &str, id: &str, name: &str) -> Result<(), String> {
        self.validate()?;
        let source_preset = self.find(source_id)?.source_preset;
        let mut next = self.clone();
        next.entries.push(SavedProfile {
            id: id.to_owned(),
            name: name.to_owned(),
            source_preset,
        });
        self.accept(next)
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        self.validate()?;
        self.require_editable(id)?;
        let mut next = self.clone();
        for entry in &mut next.entries {
            if entry.id == id {
                name.clone_into(&mut entry.name);
            }
        }
        self.accept(next)
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        self.validate()?;
        self.require_editable(id)?;
        if self.quick_switch.iter().any(|selected| selected == id) {
            return Err(
                "choose a replacement quick-switch entry before deleting this profile".to_owned(),
            );
        }
        let mut next = self.clone();
        next.entries.retain(|entry| entry.id != id);
        self.accept(next)
    }

    pub fn set_quick_switch(&mut self, first: &str, second: &str) -> Result<(), String> {
        self.validate()?;
        let mut next = self.clone();
        next.quick_switch = [first.to_owned(), second.to_owned()];
        self.accept(next)
    }

    fn accept(&mut self, next: Self) -> Result<(), String> {
        next.validate()?;
        *self = next;
        Ok(())
    }

    fn find(&self, id: &str) -> Result<&SavedProfile, String> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| format!("unknown profile ID {id:?}"))
    }

    fn require_editable(&self, id: &str) -> Result<(), String> {
        if self.find(id)?.is_builtin() {
            return Err(format!("built-in profile {id:?} is read-only"));
        }
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 48
        || !id.as_bytes()[0].is_ascii_lowercase()
        || !id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(
            "profile IDs must start with a-z and contain at most 48 ASCII letters, digits, - or _"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.trim() != name
        || name.is_empty()
        || name.chars().count() > 64
        || name.chars().any(char::is_control)
    {
        return Err(
            "profile names must contain 1 to 64 characters, without surrounding whitespace or control characters"
                .to_owned(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_library_round_trips_and_toggles_both_directions() {
        let original = ProfileLibraryV1::default();
        original.validate().unwrap();
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: ProfileLibraryV1 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(
            decoded
                .toggle_target(Some(ProfileName::Developer))
                .unwrap()
                .id(),
            "gaming"
        );
        assert_eq!(
            decoded
                .toggle_target(Some(ProfileName::Gaming))
                .unwrap()
                .id(),
            "developer"
        );
    }

    #[test]
    fn unknown_hardware_state_does_not_guess_from_saved_preferences() {
        assert!(ProfileLibraryV1::default().toggle_target(None).is_err());
    }

    #[test]
    fn named_copy_can_be_renamed_selected_and_removed_without_changing_presets() {
        let mut library = ProfileLibraryV1::default();
        library.duplicate("developer", "work", "Work").unwrap();
        library.rename("work", "Coding").unwrap();
        library.set_quick_switch("work", "gaming").unwrap();
        let target = library.toggle_target(Some(ProfileName::Gaming)).unwrap();
        assert_eq!(target.name(), "Coding");
        assert_eq!(target.source_preset(), ProfileName::Developer);
        assert!(!target.is_builtin());
        library.set_quick_switch("developer", "gaming").unwrap();
        library.remove("work").unwrap();
        assert_eq!(library, ProfileLibraryV1::default());
    }

    #[test]
    fn builtins_are_immutable_and_failed_mutations_leave_library_unchanged() {
        let mut library = ProfileLibraryV1::default();
        let before = library.clone();
        assert!(library.rename("developer", "Work").is_err());
        assert!(library.remove("gaming").is_err());
        assert!(library.duplicate("developer", "gaming", "Copy").is_err());
        assert!(library.duplicate("missing", "copy", "Copy").is_err());
        assert!(library.set_quick_switch("developer", "missing").is_err());
        assert_eq!(library, before);
    }

    #[test]
    fn aliases_cannot_turn_toggle_into_noop_or_leave_dangling_references() {
        let mut library = ProfileLibraryV1::default();
        library.duplicate("developer", "work", "Work").unwrap();
        let before = library.clone();
        assert!(library.set_quick_switch("developer", "work").is_err());
        assert_eq!(library, before);
        library.set_quick_switch("work", "gaming").unwrap();
        let before = library.clone();
        assert!(library.remove("work").is_err());
        assert_eq!(library, before);
    }

    #[test]
    fn invalid_ids_names_and_duplicate_names_are_rejected() {
        let mut library = ProfileLibraryV1::default();
        for id in ["", "../baseline", "Upper", "a/b", "9work"] {
            assert!(library.duplicate("developer", id, "Work").is_err());
        }
        for name in ["", " Work", "Work ", "Work\n", "developer"] {
            assert!(library.duplicate("developer", "work", name).is_err());
        }
        assert!(
            library
                .duplicate("developer", &"x".repeat(49), "Work")
                .is_err()
        );
        assert!(
            library
                .duplicate("developer", "work", &"x".repeat(65))
                .is_err()
        );
        assert_eq!(library, ProfileLibraryV1::default());
    }

    #[test]
    fn future_schema_and_tampered_builtins_are_rejected_after_decode() {
        let mut library = ProfileLibraryV1::default();
        library.schema_version += 1;
        assert!(library.validate().is_err());
        let mut library = ProfileLibraryV1::default();
        library.entries[0].source_preset = ProfileName::Gaming;
        assert!(library.validate().is_err());
        let mut library = ProfileLibraryV1::default();
        library.entries.remove(0);
        assert!(library.validate().is_err());
    }

    #[test]
    fn unknown_fields_and_arbitrary_preset_actions_are_not_silently_ignored() {
        let mut value = serde_json::to_value(ProfileLibraryV1::default()).unwrap();
        value["baseline"] = serde_json::json!({"trusted": true});
        assert!(serde_json::from_value::<ProfileLibraryV1>(value).is_err());
        let mut value = serde_json::to_value(ProfileLibraryV1::default()).unwrap();
        value["entries"][0]["dpi"] = serde_json::json!(8000);
        assert!(serde_json::from_value::<ProfileLibraryV1>(value).is_err());
        let mut value = serde_json::to_value(ProfileLibraryV1::default()).unwrap();
        value["entries"][0]["source_preset"] = serde_json::json!("custom");
        assert!(serde_json::from_value::<ProfileLibraryV1>(value).is_err());
    }

    #[test]
    fn library_size_limit_is_transactional() {
        let mut library = ProfileLibraryV1::default();
        for number in 2..MAX_SAVED_PROFILES {
            library
                .duplicate(
                    "developer",
                    &format!("copy-{number}"),
                    &format!("Copy {number}"),
                )
                .unwrap();
        }
        let before = library.clone();
        assert!(
            library
                .duplicate("developer", "overflow", "Overflow")
                .is_err()
        );
        assert_eq!(library, before);
    }

    #[test]
    fn fixture_corpus_preserves_decode_and_semantic_rejection_boundaries() {
        #[derive(Deserialize)]
        struct Case {
            file: String,
            decode: bool,
            valid: bool,
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/profiles");
        let cases: Vec<Case> =
            serde_json::from_str(&std::fs::read_to_string(root.join("cases.json")).unwrap())
                .unwrap();
        for case in cases {
            let bytes = std::fs::read(root.join(&case.file)).unwrap();
            let decoded = serde_json::from_slice::<ProfileLibraryV1>(&bytes);
            assert_eq!(decoded.is_ok(), case.decode, "decode: {}", case.file);
            if let Ok(library) = decoded {
                assert_eq!(
                    library.validate().is_ok(),
                    case.valid,
                    "validate: {}",
                    case.file
                );
            }
        }
    }
}
