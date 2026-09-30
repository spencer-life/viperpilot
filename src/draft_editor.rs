//! Headless local editor for user-authored profile intent drafts.
//!
//! This module is available only with `egui-preview`; it has no device or
//! planning path. Each mutation validates a private copy and persists it before
//! replacing the session's in-memory library.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::profile_intent::{ProfileDraftLibraryV1, ProfileIntentV1};
use crate::storage::{self, StoragePaths};

const LOCK_FILE_NAME: &str = ".profile-drafts-v1.lock";

/// A persisted editing session for local semantic profile drafts.
pub struct DraftEditor {
    paths: StoragePaths,
    library: ProfileDraftLibraryV1,
    /// `None` means the draft file was absent when this session opened.
    expected_bytes: Option<Vec<u8>>,
    /// Retain deleted IDs for the lifetime of this session.
    used_ids: HashSet<String>,
}

impl DraftEditor {
    /// Open a session without creating the draft file or any hardware state.
    pub fn open(paths: StoragePaths) -> Result<Self, String> {
        let before = read_optional(&paths.profile_drafts)?;
        let library = storage::load_profile_drafts(&paths).map_err(|error| error.to_string())?;
        let after = read_optional(&paths.profile_drafts)?;
        if before != after {
            return Err("profile drafts changed while the editor was opening; reopen to load the latest file".to_owned());
        }
        let used_ids = library
            .entries()
            .iter()
            .map(|draft| draft.id().to_owned())
            .collect();
        Ok(Self {
            paths,
            library,
            expected_bytes: after,
            used_ids,
        })
    }

    #[must_use]
    pub fn library(&self) -> &ProfileDraftLibraryV1 {
        &self.library
    }

    /// Create a draft with a generated stable ID.
    pub fn create(&mut self, intent: ProfileIntentV1) -> Result<String, String> {
        let id = self.unique_id();
        let mut next = self.library.clone();
        next.create(&id, intent)?;
        self.persist(next)?;
        self.used_ids.insert(id.clone());
        Ok(id)
    }

    pub fn edit(&mut self, id: &str, intent: ProfileIntentV1) -> Result<(), String> {
        let mut next = self.library.clone();
        next.edit(id, intent)?;
        self.persist(next)
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        let mut next = self.library.clone();
        next.rename(id, name)?;
        self.persist(next)
    }

    pub fn duplicate(&mut self, source_id: &str, name: &str) -> Result<String, String> {
        let id = self.unique_id();
        let mut next = self.library.clone();
        next.duplicate(source_id, &id, name)?;
        self.persist(next)?;
        self.used_ids.insert(id.clone());
        Ok(id)
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let mut next = self.library.clone();
        next.delete(id)?;
        self.persist(next)
    }

    fn persist(&mut self, next: ProfileDraftLibraryV1) -> Result<(), String> {
        next.validate()?;
        let _lock = DraftSaveLock::acquire(&self.paths.root)?;
        let actual = read_optional(&self.paths.profile_drafts)?;
        if actual != self.expected_bytes {
            return Err("profile drafts changed outside this editor session; no changes were saved. Reopen the editor to load the latest file".to_owned());
        }

        // Encode before writing so the session can retain the exact bytes saved.
        let mut bytes = serde_json::to_vec_pretty(&next)
            .map_err(|error| format!("cannot encode profile drafts: {error}"))?;
        bytes.push(b'\n');
        storage::save_profile_drafts(&self.paths, &next)
            .map_err(|error| format!("cannot save profile drafts: {error}"))?;
        self.library = next;
        self.expected_bytes = Some(bytes);
        Ok(())
    }

    fn unique_id(&self) -> String {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        loop {
            let ticks = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let counter = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let id = format!("draft-{ticks:x}-{counter:x}");
            if !self.used_ids.contains(&id) {
                return id;
            }
        }
    }
}

fn read_optional(path: &std::path::Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot read profile drafts: {error}")),
    }
}

struct DraftSaveLock {
    path: PathBuf,
    _file: File,
}

impl DraftSaveLock {
    fn acquire(root: &std::path::Path) -> Result<Self, String> {
        fs::create_dir_all(root)
            .map_err(|error| format!("cannot prepare profile draft storage: {error}"))?;
        let path = root.join(LOCK_FILE_NAME);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => Ok(Self { path, _file: file }),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => Err(format!(
                "profile draft save is locked by another editor or a stale lock at {}; close other editor sessions, then remove this lock file if no editor is running",
                path.display()
            )),
            Err(error) => Err(format!("cannot acquire profile draft save lock: {error}")),
        }
    }
}

impl Drop for DraftSaveLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_intent::{
        ButtonActionIntent, DpiTarget, KeyboardKey, PROFILE_INTENT_SCHEMA_VERSION,
    };
    use crate::storage::StoragePaths;

    fn root(label: &str) -> PathBuf {
        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "viperpilot-draft-editor-{label}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn intent(name: &str) -> ProfileIntentV1 {
        ProfileIntentV1 {
            schema_version: PROFILE_INTENT_SCHEMA_VERSION,
            name: name.to_owned(),
            dpi: DpiTarget { x: 1200, y: 2400 },
            polling_hz: 2000,
            mouse4: ButtonActionIntent::KeyboardShortcut {
                control: true,
                alt: true,
                shift: false,
                windows: false,
                key: KeyboardKey::F10,
            },
            mouse5: ButtonActionIntent::Unassigned,
        }
    }

    fn clean(root: &std::path::Path) {
        if root.is_file() {
            let _ = fs::remove_file(root);
        } else {
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn crud_persists_every_intent_field_and_reopens() {
        let root = root("crud");
        let paths = StoragePaths::under(root.clone());
        let mut editor = DraftEditor::open(paths.clone()).unwrap();
        assert!(!paths.profile_drafts.exists());
        let id = editor.create(intent("First draft")).unwrap();
        assert!(id.starts_with("draft-"));
        editor.rename(&id, "Renamed draft").unwrap();

        let mut replacement = intent("Edited draft");
        replacement.dpi = DpiTarget { x: 800, y: 1600 };
        replacement.polling_hz = 4000;
        replacement.mouse4 = ButtonActionIntent::PassThrough;
        replacement.mouse5 = ButtonActionIntent::KeyboardShortcut {
            control: false,
            alt: true,
            shift: true,
            windows: false,
            key: KeyboardKey::F11,
        };
        editor.edit(&id, replacement.clone()).unwrap();
        let copy_id = editor.duplicate(&id, "Copied draft").unwrap();
        assert_ne!(id, copy_id);
        assert_eq!(
            editor.library().get(&copy_id).unwrap().intent().name,
            "Copied draft"
        );
        editor.delete(&id).unwrap();

        let reopened = DraftEditor::open(paths).unwrap();
        assert_eq!(reopened.library().entries().len(), 1);
        let saved = reopened.library().get(&copy_id).unwrap().intent();
        assert_eq!(saved.name, "Copied draft");
        assert_eq!(saved.dpi, replacement.dpi);
        assert_eq!(saved.polling_hz, replacement.polling_hz);
        assert_eq!(saved.mouse4, replacement.mouse4);
        assert_eq!(saved.mouse5, replacement.mouse5);
        clean(&root);
    }

    #[test]
    fn invalid_future_and_corrupt_inputs_preserve_existing_bytes() {
        let root = root("invalid");
        let paths = StoragePaths::under(root.clone());
        let mut editor = DraftEditor::open(paths.clone()).unwrap();
        let id = editor.create(intent("Valid draft")).unwrap();
        let original = fs::read(&paths.profile_drafts).unwrap();

        assert!(editor.create(intent("Valid draft")).is_err());
        assert!(editor.rename(&id, "  bad ").is_err());
        let mut future = intent("Future schema");
        future.schema_version += 1;
        assert!(editor.edit(&id, future).is_err());
        assert_eq!(fs::read(&paths.profile_drafts).unwrap(), original);
        assert!(DraftEditor::open(paths.clone()).is_ok());

        fs::write(&paths.profile_drafts, b"{corrupt").unwrap();
        assert!(DraftEditor::open(paths).is_err());
        assert_eq!(
            fs::read(root.join("profile-drafts-v1.json")).unwrap(),
            b"{corrupt"
        );
        clean(&root);
    }

    #[test]
    fn stale_sessions_and_missing_to_created_changes_never_overwrite() {
        let root = root("stale");
        let paths = StoragePaths::under(root.clone());
        let mut first = DraftEditor::open(paths.clone()).unwrap();
        let mut second = DraftEditor::open(paths.clone()).unwrap();
        let first_id = first.create(intent("First writer")).unwrap();
        let on_disk = fs::read(&paths.profile_drafts).unwrap();
        assert!(
            second
                .create(intent("Second writer"))
                .unwrap_err()
                .contains("changed outside")
        );
        assert!(second.library().entries().is_empty());
        assert_eq!(fs::read(&paths.profile_drafts).unwrap(), on_disk);

        let mut opened_existing = DraftEditor::open(paths.clone()).unwrap();
        fs::remove_file(&paths.profile_drafts).unwrap();
        assert!(
            opened_existing
                .rename(&first_id, "Must not recreate")
                .unwrap_err()
                .contains("changed outside")
        );
        assert!(!paths.profile_drafts.exists());

        let root_missing = root.join("missing-case");
        let missing_paths = StoragePaths::under(root_missing.clone());
        let mut opened_missing = DraftEditor::open(missing_paths.clone()).unwrap();
        fs::create_dir_all(&root_missing).unwrap();
        fs::write(&missing_paths.profile_drafts, b"external").unwrap();
        let external = fs::read(&missing_paths.profile_drafts).unwrap();
        assert!(opened_missing.create(intent("Must stay local")).is_err());
        assert!(opened_missing.library().entries().is_empty());
        assert_eq!(fs::read(&missing_paths.profile_drafts).unwrap(), external);
        clean(&root);
    }

    #[test]
    fn save_failure_keeps_session_and_existing_file() {
        let root = root("failed-save");
        let paths = StoragePaths::under(root.clone());
        fs::create_dir_all(&root).unwrap();
        let mut editor = DraftEditor::open(paths.clone()).unwrap();
        // A regular file cannot serve as the parent directory for the save lock.
        fs::remove_dir(&root).unwrap();
        fs::write(&root, b"sentinel").unwrap();
        let before = editor.library().clone();
        assert!(editor.create(intent("Cannot save")).is_err());
        assert_eq!(editor.library(), &before);
        assert_eq!(fs::read(&root).unwrap(), b"sentinel");
        clean(&root);
    }

    #[test]
    fn lock_conflict_preserves_session_and_draft_file() {
        let root = root("lock");
        let paths = StoragePaths::under(root.clone());
        let mut editor = DraftEditor::open(paths.clone()).unwrap();
        fs::create_dir_all(&root).unwrap();
        let lock_path = root.join(LOCK_FILE_NAME);
        fs::write(&lock_path, b"active editor").unwrap();
        assert!(
            editor
                .create(intent("Blocked save"))
                .unwrap_err()
                .contains("stale lock")
        );
        assert!(editor.library().entries().is_empty());
        assert!(!paths.profile_drafts.exists());
        assert_eq!(fs::read(&lock_path).unwrap(), b"active editor");
        clean(&root);
    }

    #[test]
    fn editor_storage_stays_separate_from_config_library_reports_and_baseline() {
        let root = root("separation");
        let paths = StoragePaths::under(root.clone());
        fs::create_dir_all(&paths.backups).unwrap();
        fs::create_dir_all(&paths.reports).unwrap();
        let aliases = root.join("profiles-v1.json");
        let report = paths.reports.join("report sentinel.json");
        fs::write(&paths.config, b"config sentinel").unwrap();
        fs::write(&aliases, b"alias sentinel").unwrap();
        fs::write(&report, b"report sentinel").unwrap();
        fs::write(&paths.diagnostics, b"diagnostic sentinel").unwrap();
        let baseline = paths.baseline_path("SERIAL");
        fs::write(&baseline, b"baseline sentinel").unwrap();
        let sentinels = [
            (&paths.config, b"config sentinel".as_slice()),
            (&aliases, b"alias sentinel".as_slice()),
            (&report, b"report sentinel".as_slice()),
            (&paths.diagnostics, b"diagnostic sentinel".as_slice()),
            (&baseline, b"baseline sentinel".as_slice()),
        ];

        DraftEditor::open(paths.clone())
            .unwrap()
            .create(intent("Separate draft"))
            .unwrap();

        for (path, expected) in sentinels {
            assert_eq!(fs::read(path).unwrap(), expected);
        }
        assert!(paths.profile_drafts.exists());
        clean(&root);
    }
}
