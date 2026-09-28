use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::engine::VerificationReport;
use crate::model::{DeviceSnapshotV1, UtilityConfigV1};
use crate::planning::WritePlan;

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
    if !paths.config.exists() {
        return Ok(UtilityConfigV1::default());
    }
    let config: UtilityConfigV1 = read_json(&paths.config)?;
    if config.schema_version != crate::model::CONFIG_SCHEMA_VERSION {
        return Err(StorageError::Path(format!(
            "unsupported config schema {}",
            config.schema_version
        )));
    }
    Ok(config)
}

pub fn save_config(paths: &StoragePaths, config: &UtilityConfigV1) -> Result<(), StorageError> {
    fs::create_dir_all(&paths.root)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&paths.config)?;
    write_json(file, config)
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
}
