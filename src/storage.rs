use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::engine::VerificationReport;
use crate::model::{DeviceSnapshotV1, UtilityConfigV1};
use crate::planning::WritePlan;
use crate::profile_library::ProfileLibraryV1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoragePaths {
    pub root: PathBuf,
    pub backups: PathBuf,
    pub reports: PathBuf,
    pub config: PathBuf,
    pub diagnostics: PathBuf,
}

impl StoragePaths {
    pub fn discover() -> Result<Self, StorageError> {
        let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
            StorageError::Path("LOCALAPPDATA is not defined; run the Windows executable".to_owned())
        })?;
        Ok(Self::under(
            PathBuf::from(local_app_data).join("ViperV4Utility"),
        ))
    }

    #[must_use]
    pub fn under(root: PathBuf) -> Self {
        Self {
            backups: root.join("backups"),
            reports: root.join("reports"),
            config: root.join("config-v1.json"),
            diagnostics: root.join("diagnostics-v1.log"),
            root,
        }
    }

    #[must_use]
    pub fn baseline_path(&self, serial: &str) -> PathBuf {
        self.backups
            .join(format!("baseline-v1-{}.json", safe_component(serial)))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotWrite {
    pub path: PathBuf,
    pub baseline_path: PathBuf,
    pub baseline_created: bool,
}

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidSnapshot(String),
    Path(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "storage I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::InvalidSnapshot(error) | Self::Path(error) => formatter.write_str(error),
        }
    }
}

impl Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn write_snapshot(
    paths: &StoragePaths,
    snapshot: &DeviceSnapshotV1,
    requested_output: Option<&Path>,
) -> Result<SnapshotWrite, StorageError> {
    snapshot.validate().map_err(StorageError::InvalidSnapshot)?;
    fs::create_dir_all(&paths.backups)?;
    let baseline_path = paths.baseline_path(&snapshot.device.serial);
    let baseline_created = !baseline_path.exists();
    if baseline_created {
        write_json_new(&baseline_path, snapshot)?;
    }
    let path = if let Some(output) = requested_output {
        output.to_path_buf()
    } else if baseline_created {
        baseline_path.clone()
    } else {
        paths.backups.join(format!(
            "snapshot-v1-{}-{}.json",
            unix_nanos(),
            safe_component(&snapshot.device.serial)
        ))
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path != baseline_path {
        write_json_new(&path, snapshot)?;
    }
    Ok(SnapshotWrite {
        path,
        baseline_path,
        baseline_created,
    })
}

pub fn read_snapshot(path: &Path) -> Result<DeviceSnapshotV1, StorageError> {
    let snapshot: DeviceSnapshotV1 = read_json(path)?;
    snapshot.validate().map_err(StorageError::InvalidSnapshot)?;
    Ok(snapshot)
}

pub fn write_plan_journal(paths: &StoragePaths, plan: &WritePlan) -> Result<PathBuf, StorageError> {
    fs::create_dir_all(&paths.reports)?;
    let path = paths.reports.join(format!(
        "write-plan-v1-{}-{}.json",
        unix_nanos(),
        safe_component(&plan.before.device.serial)
    ));
    write_json_new(&path, plan)?;
    Ok(path)
}

pub fn write_verification_report(
    paths: &StoragePaths,
    report: &VerificationReport,
    serial: &str,
) -> Result<PathBuf, StorageError> {
    fs::create_dir_all(&paths.reports)?;
    let path = paths.reports.join(format!(
        "verification-v1-{}-{}.json",
        unix_nanos(),
        safe_component(serial)
    ));
    write_json_new(&path, report)?;
    Ok(path)
}

pub fn load_config(paths: &StoragePaths) -> Result<UtilityConfigV1, StorageError> {
    let config: UtilityConfigV1 = match read_json(&paths.config) {
        Ok(config) => config,
        Err(StorageError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(UtilityConfigV1::default());
        }
        Err(error) => return Err(error),
    };
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &UtilityConfigV1) -> Result<(), StorageError> {
    if config.schema_version != crate::model::CONFIG_SCHEMA_VERSION {
        return Err(StorageError::Path(format!(
            "unsupported config schema {}",
            config.schema_version
        )));
    }
    Ok(())
}

pub fn save_config(paths: &StoragePaths, config: &UtilityConfigV1) -> Result<(), StorageError> {
    validate_config(config)?;
    write_json_atomic(&paths.config, config)
}

/// Local-only metadata, separate from config-v1 and immutable device baselines.
/// Missing libraries have in-memory defaults; reads never create a file.
pub fn load_profile_library(paths: &StoragePaths) -> Result<ProfileLibraryV1, StorageError> {
    let library: ProfileLibraryV1 = match read_json(&paths.root.join("profiles-v1.json")) {
        Ok(library) => library,
        Err(StorageError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ProfileLibraryV1::default());
        }
        Err(error) => return Err(error),
    };
    library.validate().map_err(StorageError::Path)?;
    Ok(library)
}

/// Persist a validated local library only. This does not update the last
/// verified hardware profile, authorize writes, or modify a device baseline.
/// Future callers must serialize read-modify-save operations through one owner.
pub fn save_profile_library(
    paths: &StoragePaths,
    library: &ProfileLibraryV1,
) -> Result<(), StorageError> {
    library.validate().map_err(StorageError::Path)?;
    write_json_atomic(&paths.root.join("profiles-v1.json"), library)
}

// 2026-09-28: replace the in-place config truncation with a complete, synced
// sibling file followed by rename. Never delete the destination as a fallback.
// Windows filesystem replacement and power-loss behavior still need native QA;
// sync_all plus rename is not a claim of universal power-loss durability.
fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    // Finish serialization before touching even a temporary file.
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| StorageError::Path("JSON destination must name a file".to_owned()))?;
    fs::create_dir_all(parent)?;
    let mut temp_name = file_name.to_os_string();
    temp_name.push(format!(
        ".tmp-{}-{}-{}",
        std::process::id(),
        unix_nanos(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let temp_path = parent.join(temp_name);
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)?;
    let result = file.write_all(&bytes).and_then(|()| file.sync_all());
    drop(file); // Windows must be able to rename the closed sibling file.
    if let Err(error) = result {
        let _ = fs::remove_file(&temp_path);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error.into());
    }
    Ok(())
}

const DIAGNOSTIC_LOG_LIMIT: u64 = 256 * 1024;

pub fn append_diagnostic(paths: &StoragePaths, event: &str) -> Result<(), StorageError> {
    fs::create_dir_all(&paths.root)?;
    let event = event.replace(['\r', '\n'], " ");
    let entry = format!("{} {event}\n", unix_nanos());
    let existing = fs::metadata(&paths.diagnostics).map_or(0, |metadata| metadata.len());
    if existing.saturating_add(entry.len() as u64) > DIAGNOSTIC_LOG_LIMIT {
        let mut file = File::create(&paths.diagnostics)?;
        file.write_all(
            b"diagnostic log rotated; device identity and HID paths are never logged\n",
        )?;
        file.sync_all()?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.diagnostics)?;
    file.write_all(entry.as_bytes())?;
    file.flush()?;
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, StorageError> {
    let file = File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(file))?)
}

fn write_json_new<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let file = OpenOptions::new().create_new(true).write(true).open(path)?;
    write_json(file, value)
}

fn write_json<T: Serialize>(file: File, value: &T) -> Result<(), StorageError> {
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

fn safe_component(value: &str) -> String {
    let filtered: String = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .collect();
    if filtered.is_empty() {
        "unknown".to_owned()
    } else {
        filtered
    }
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ALL_BUTTON_IDS, DeviceIdentity, DpiPair, DpiStage, DpiState, FirmwareVersion, PollingState,
        RawButtonAssignment, SNAPSHOT_SCHEMA_VERSION, TransportKind, WirelessPowerSettings,
    };

    fn snapshot() -> DeviceSnapshotV1 {
        DeviceSnapshotV1 {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            captured_at_unix_ms: 1,
            device: DeviceIdentity {
                vendor_id: 0x1532,
                product_id: 0x00e6,
                hid_descriptor_serial: Some("DONGLE123".to_owned()),
                serial: "SERIAL/ONE".to_owned(),
                firmware: FirmwareVersion { major: 1, minor: 0 },
                path: "path".to_owned(),
                interface_number: 4,
                usage_page: 0x0c,
                usage: 1,
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
            button_assignments: ALL_BUTTON_IDS
                .into_iter()
                .map(|id| RawButtonAssignment::mouse_button(id, id))
                .collect(),
        }
    }

    #[test]
    fn first_baseline_is_never_overwritten() {
        let root = std::env::temp_dir().join(format!(
            "viper-v4-storage-test-{}-{}",
            std::process::id(),
            unix_nanos()
        ));
        let paths = StoragePaths::under(root.clone());
        let first = write_snapshot(&paths, &snapshot(), None).expect("first snapshot");
        assert!(first.baseline_created);
        let baseline_bytes = fs::read(&first.baseline_path).expect("baseline readable");

        let mut second_snapshot = snapshot();
        second_snapshot.polling.raw_code = 0x02;
        let second = write_snapshot(&paths, &second_snapshot, None).expect("second snapshot");
        assert!(!second.baseline_created);
        assert_ne!(second.path, first.path);
        assert_eq!(
            fs::read(&first.baseline_path).expect("baseline still readable"),
            baseline_bytes
        );
        fs::remove_dir_all(root).expect("remove isolated test directory");
    }

    #[test]
    fn snapshot_json_round_trips_unknown_raw_data() {
        let root = std::env::temp_dir().join(format!(
            "viper-v4-roundtrip-test-{}-{}",
            std::process::id(),
            unix_nanos()
        ));
        let paths = StoragePaths::under(root.clone());
        let mut original = snapshot();
        original.button_assignments[0].function_id = 0xfe;
        original.button_assignments[0].data_size = 5;
        original.button_assignments[0].data = [1, 2, 3, 4, 5];
        original.power = Some(WirelessPowerSettings {
            battery_raw: 0xcc,
            idle_seconds: 180,
            low_power_threshold_raw: 0x0d,
        });
        let written = write_snapshot(&paths, &original, None).expect("write snapshot");
        let decoded = read_snapshot(&written.path).expect("read snapshot");
        assert_eq!(decoded, original);
        fs::remove_dir_all(root).expect("remove isolated test directory");
    }

    #[test]
    fn older_v1_baseline_without_power_fields_stays_readable() {
        let encoded = serde_json::to_value(snapshot()).expect("serialize old snapshot");
        let decoded: DeviceSnapshotV1 =
            serde_json::from_value(encoded).expect("deserialize old v1 baseline");
        assert_eq!(decoded.power, None);
        decoded.validate().expect("old baseline remains valid");
    }

    #[test]
    fn unknown_snapshot_schema_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "viper-v4-schema-test-{}-{}",
            std::process::id(),
            unix_nanos()
        ));
        fs::create_dir_all(&root).expect("create isolated test directory");
        let path = root.join("future.json");
        let mut future = snapshot();
        future.schema_version = SNAPSHOT_SCHEMA_VERSION + 1;
        write_json_new(&path, &future).expect("write future snapshot");
        assert!(matches!(
            read_snapshot(&path),
            Err(StorageError::InvalidSnapshot(_))
        ));
        fs::remove_dir_all(root).expect("remove isolated test directory");
    }

    #[test]
    fn custom_output_also_creates_the_required_default_baseline() {
        let root = std::env::temp_dir().join(format!(
            "viper-v4-custom-output-test-{}-{}",
            std::process::id(),
            unix_nanos()
        ));
        let paths = StoragePaths::under(root.clone());
        let custom = root.join("review-copy.json");
        let written = write_snapshot(&paths, &snapshot(), Some(&custom)).expect("write snapshot");
        assert!(written.baseline_created);
        assert_eq!(written.path, custom);
        assert!(written.path.exists());
        assert!(written.baseline_path.exists());
        assert_eq!(
            fs::read(&written.path).expect("custom output readable"),
            fs::read(&written.baseline_path).expect("baseline readable")
        );
        fs::remove_dir_all(root).expect("remove isolated test directory");
    }

    #[test]
    fn diagnostic_log_is_single_line_and_size_capped() {
        let root = std::env::temp_dir().join(format!(
            "viper-v4-diagnostics-test-{}-{}",
            std::process::id(),
            unix_nanos()
        ));
        let paths = StoragePaths::under(root.clone());
        fs::create_dir_all(&root).expect("create isolated test directory");
        fs::write(
            &paths.diagnostics,
            vec![b'x'; usize::try_from(DIAGNOSTIC_LOG_LIMIT).expect("limit fits usize")],
        )
        .expect("seed full log");
        append_diagnostic(&paths, "apply developer\nverified").expect("append diagnostic");
        let log = fs::read_to_string(&paths.diagnostics).expect("read diagnostic log");
        assert!(log.len() < 1024);
        assert!(log.contains("apply developer verified"));
        assert_eq!(log.lines().count(), 2);
        fs::remove_dir_all(root).expect("remove isolated test directory");
    }

    fn isolated_paths(label: &str) -> StoragePaths {
        StoragePaths::under(std::env::temp_dir().join(format!(
            "viper-v4-{label}-{}-{}",
            std::process::id(),
            unix_nanos()
        )))
    }

    #[test]
    fn missing_config_and_library_return_defaults_without_creating_files() {
        let paths = isolated_paths("defaults");
        assert_eq!(load_config(&paths).unwrap(), UtilityConfigV1::default());
        assert_eq!(
            load_profile_library(&paths).unwrap(),
            ProfileLibraryV1::default()
        );
        assert!(!paths.root.exists());
    }

    #[test]
    fn config_replacement_and_library_save_leave_baselines_and_config_schema_alone() {
        let paths = isolated_paths("atomic-replace");
        let snapshot_write = write_snapshot(&paths, &snapshot(), None).unwrap();
        let baseline_before = fs::read(&snapshot_write.baseline_path).unwrap();
        let original = UtilityConfigV1::default();
        save_config(&paths, &original).unwrap();
        let changed = UtilityConfigV1 {
            diagnostics_enabled: true,
            ..original
        };
        save_config(&paths, &changed).unwrap();
        assert_eq!(load_config(&paths).unwrap(), changed);
        let config_before = fs::read(&paths.config).unwrap();
        let mut library = ProfileLibraryV1::default();
        library.duplicate("developer", "work", "Work").unwrap();
        save_profile_library(&paths, &library).unwrap();
        assert_eq!(load_profile_library(&paths).unwrap(), library);
        assert_eq!(fs::read(&paths.config).unwrap(), config_before);
        assert_eq!(
            fs::read(&snapshot_write.baseline_path).unwrap(),
            baseline_before
        );
        assert!(fs::read_dir(&paths.root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp-")
        }));
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn unsupported_config_save_does_not_replace_existing_bytes() {
        let paths = isolated_paths("unsupported-config");
        save_config(&paths, &UtilityConfigV1::default()).unwrap();
        let before = fs::read(&paths.config).unwrap();
        let future = UtilityConfigV1 {
            schema_version: crate::model::CONFIG_SCHEMA_VERSION + 1,
            ..UtilityConfigV1::default()
        };
        assert!(save_config(&paths, &future).is_err());
        assert_eq!(fs::read(&paths.config).unwrap(), before);
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn serialization_failure_keeps_existing_destination_unchanged() {
        struct Invalid;
        impl Serialize for Invalid {
            fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("injected serialization failure"))
            }
        }
        let paths = isolated_paths("serialization-failure");
        save_config(&paths, &UtilityConfigV1::default()).unwrap();
        let before = fs::read(&paths.config).unwrap();
        assert!(write_json_atomic(&paths.config, &Invalid).is_err());
        assert_eq!(fs::read(&paths.config).unwrap(), before);
        assert_eq!(fs::read_dir(&paths.root).unwrap().count(), 1);
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn failed_rename_never_deletes_destination_and_cleans_its_temporary_file() {
        let paths = isolated_paths("rename-failure");
        fs::create_dir_all(&paths.config).unwrap();
        let sentinel = paths.config.join("preserve.txt");
        fs::write(&sentinel, b"keep").unwrap();
        assert!(save_config(&paths, &UtilityConfigV1::default()).is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
        assert_eq!(fs::read_dir(&paths.root).unwrap().count(), 1);
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn corrupt_and_future_libraries_fail_without_reset_or_rewrite() {
        let paths = isolated_paths("bad-library");
        fs::create_dir_all(&paths.root).unwrap();
        let path = paths.root.join("profiles-v1.json");
        let mut future = serde_json::to_value(ProfileLibraryV1::default()).unwrap();
        future["schema_version"] = serde_json::json!(999);
        for bytes in [b"{bad json".to_vec(), serde_json::to_vec(&future).unwrap()] {
            fs::write(&path, &bytes).unwrap();
            assert!(load_profile_library(&paths).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn invalid_library_is_rejected_before_saving() {
        let paths = isolated_paths("bad-save");
        let mut value = serde_json::to_value(ProfileLibraryV1::default()).unwrap();
        value["quick_switch"] = serde_json::json!(["developer", "developer"]);
        let invalid: ProfileLibraryV1 = serde_json::from_value(value).unwrap();
        assert!(save_profile_library(&paths, &invalid).is_err());
        assert!(!paths.root.exists());
    }

    #[test]
    fn invalid_library_save_preserves_a_preexisting_library() {
        let paths = isolated_paths("invalid-library-replace");
        save_profile_library(&paths, &ProfileLibraryV1::default()).unwrap();
        let path = paths.root.join("profiles-v1.json");
        let before = fs::read(&path).unwrap();
        let mut value = serde_json::to_value(ProfileLibraryV1::default()).unwrap();
        value["schema_version"] = serde_json::json!(999);
        let invalid: ProfileLibraryV1 = serde_json::from_value(value).unwrap();
        assert!(save_profile_library(&paths, &invalid).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(&paths.root).unwrap().count(), 1);
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn corrupt_and_future_config_reads_preserve_original_bytes() {
        let paths = isolated_paths("bad-config-read");
        fs::create_dir_all(&paths.root).unwrap();
        let mut future = serde_json::to_value(UtilityConfigV1::default()).unwrap();
        future["schema_version"] = serde_json::json!(999);
        for bytes in [b"{bad json".to_vec(), serde_json::to_vec(&future).unwrap()] {
            fs::write(&paths.config, &bytes).unwrap();
            assert!(load_config(&paths).is_err());
            assert_eq!(fs::read(&paths.config).unwrap(), bytes);
        }
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn directory_in_place_of_library_is_an_error_not_a_default() {
        let paths = isolated_paths("library-is-directory");
        let path = paths.root.join("profiles-v1.json");
        fs::create_dir_all(&path).unwrap();
        assert!(load_profile_library(&paths).is_err());
        assert!(path.is_dir());
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_readonly_destination_is_preserved_after_failed_save() {
        let paths = isolated_paths("windows-readonly");
        save_config(&paths, &UtilityConfigV1::default()).unwrap();
        let before = fs::read(&paths.config).unwrap();
        let original_permissions = fs::metadata(&paths.config).unwrap().permissions();
        let mut readonly_permissions = original_permissions.clone();
        readonly_permissions.set_readonly(true);
        fs::set_permissions(&paths.config, readonly_permissions).unwrap();
        let changed = UtilityConfigV1 {
            diagnostics_enabled: true,
            ..UtilityConfigV1::default()
        };
        let result = save_config(&paths, &changed);
        let after = fs::read(&paths.config).unwrap();
        fs::set_permissions(&paths.config, original_permissions).unwrap();
        assert!(result.is_err());
        assert_eq!(after, before);
        assert_eq!(fs::read_dir(&paths.root).unwrap().count(), 1);
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_handle_without_delete_sharing_preserves_destination() {
        use std::os::windows::fs::OpenOptionsExt;

        let paths = isolated_paths("windows-sharing");
        save_config(&paths, &UtilityConfigV1::default()).unwrap();
        let before = fs::read(&paths.config).unwrap();
        // FILE_SHARE_READ | FILE_SHARE_WRITE, deliberately not FILE_SHARE_DELETE.
        let held = OpenOptions::new()
            .read(true)
            .share_mode(0x0000_0003)
            .open(&paths.config)
            .unwrap();
        let changed = UtilityConfigV1 {
            diagnostics_enabled: true,
            ..UtilityConfigV1::default()
        };
        let result = save_config(&paths, &changed);
        let after = fs::read(&paths.config).unwrap();
        drop(held);
        assert!(result.is_err());
        assert_eq!(after, before);
        assert_eq!(fs::read_dir(&paths.root).unwrap().count(), 1);
        fs::remove_dir_all(&paths.root).unwrap();
    }
}
