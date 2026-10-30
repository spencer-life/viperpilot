//! Explicit, session-scoped isolation for owner-supervised device validation.
//! No device access, imports, startup changes or filesystem writes occur here.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::DeviceSnapshotV1;
use crate::storage::StoragePaths;

#[derive(Debug)]
pub struct PilotOptions {
    root: PathBuf,
    serial: String,
    allow_established_profiles: bool,
    pub minimized: bool,
}

impl PilotOptions {
    /// The allow flag is a final launch guard after owner review of the live
    /// identity, first-baseline provenance, exact plans and recovery procedure.
    /// It does not establish evidence or authorize an agent to perform writes.
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut seen = HashSet::new();
        let mut root = None;
        let mut serial = None;
        let mut allow_established_profiles = false;
        let mut minimized = false;
        while let Some(arg) = args.next() {
            let flag = arg.to_str().ok_or("pilot flags must be valid text")?;
            if !seen.insert(flag.to_owned()) {
                return Err(format!("duplicate pilot argument: {flag}"));
            }
            match flag {
                "--pilot" => {}
                "--minimized" => minimized = true,
                "--allow-established-profiles" => allow_established_profiles = true,
                "--root" => {
                    root = Some(PathBuf::from(
                        args.next().ok_or("--root needs a directory")?,
                    ))
                }
                "--confirm-device" => {
                    let value = args
                        .next()
                        .ok_or("--confirm-device needs the full mouse serial")?;
                    let value = value
                        .into_string()
                        .map_err(|_| "mouse serial must be valid text")?;
                    if value.is_empty() || value.trim() != value || value.starts_with("--") {
                        return Err("--confirm-device needs the full mouse serial".to_owned());
                    }
                    serial = Some(value);
                }
                _ => return Err(format!("unknown pilot argument: {flag}")),
            }
        }
        if !seen.contains("--pilot") {
            return Err("explicit --pilot is required".to_owned());
        }
        Ok(Self {
            root: root.ok_or("pilot requires --root with an existing isolated directory")?,
            serial: serial.ok_or("pilot requires --confirm-device with the full mouse serial")?,
            allow_established_profiles,
            minimized,
        })
    }

    pub fn load(self) -> Result<PilotSession, String> {
        // 2026-10-29: never discover/import legacy data or create a baseline.
        // The owner stages the reviewed immutable first baseline explicitly.
        reject_legacy(&self.root)?;
        if !self.root.is_absolute() {
            return Err("pilot root must be an absolute directory".to_owned());
        }
        let root = self
            .root
            .canonicalize()
            .map_err(|error| format!("pilot root: {error}"))?;
        reject_legacy(&root)?;
        if !root.is_dir() || root.parent().is_none() {
            return Err("pilot needs a dedicated existing directory".to_owned());
        }
        let paths = StoragePaths::under(root);
        validate_paths(&paths, &self.serial)?;
        let baseline_bytes = fs::read(paths.baseline_path(&self.serial)).map_err(|error| {
            format!(
                "a reviewed immutable first baseline must be staged before pilot launch: {error}"
            )
        })?;
        let baseline: DeviceSnapshotV1 = serde_json::from_slice(&baseline_bytes)
            .map_err(|error| format!("pilot baseline: {error}"))?;
        baseline.validate()?;
        if baseline.device.serial != self.serial {
            return Err("full mouse serial does not match the reviewed pilot baseline".to_owned());
        }
        Ok(PilotSession {
            paths,
            baseline,
            baseline_bytes,
            allow_established_profiles: self.allow_established_profiles,
        })
    }
}

#[derive(Debug)]
pub struct PilotSession {
    paths: StoragePaths,
    baseline: DeviceSnapshotV1,
    baseline_bytes: Vec<u8>,
    allow_established_profiles: bool,
}

impl PilotSession {
    #[must_use]
    pub fn paths(&self) -> &StoragePaths {
        &self.paths
    }

    #[must_use]
    pub fn baseline(&self) -> &DeviceSnapshotV1 {
        &self.baseline
    }

    #[must_use]
    pub fn writes_enabled(&self) -> bool {
        self.allow_established_profiles
    }

    pub fn validate_storage(&self) -> Result<(), String> {
        validate_paths(&self.paths, &self.baseline.device.serial)?;
        let current = fs::read(self.paths.baseline_path(&self.baseline.device.serial))
            .map_err(|error| format!("pilot baseline is no longer readable: {error}"))?;
        if current != self.baseline_bytes {
            return Err(
                "pilot baseline bytes changed; stop and review before relaunching".to_owned(),
            );
        }
        Ok(())
    }

    pub fn validate_live(&self, snapshot: &DeviceSnapshotV1) -> Result<(), String> {
        self.validate_storage()?;
        snapshot.validate()?;
        // Bind the complete identity, including descriptor serial, firmware,
        // HID path/interface/usage. Changing transport/path needs fresh review.
        if snapshot.device != self.baseline.device {
            return Err("live device identity differs from the reviewed pilot baseline".to_owned());
        }
        Ok(())
    }

    pub fn authorize_profile_write(
        &self,
        before: &DeviceSnapshotV1,
        baseline: &DeviceSnapshotV1,
    ) -> Result<(), String> {
        if !self.writes_enabled() {
            return Err("device changes are disabled in a read-only pilot".to_owned());
        }
        self.validate_live(before)?;
        if baseline != &self.baseline {
            return Err(
                "write planning baseline differs from the reviewed pilot baseline".to_owned(),
            );
        }
        Ok(())
    }
}

fn reject_legacy(path: &Path) -> Result<(), String> {
    if path.components().any(|part| {
        part.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("ViperV4Utility")
    }) {
        return Err("pilot storage must not use the installed legacy app directory".to_owned());
    }
    Ok(())
}

fn validate_paths(paths: &StoragePaths, serial: &str) -> Result<(), String> {
    let canonical = paths
        .root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    reject_legacy(&canonical)?;
    if canonical != paths.root {
        return Err("pilot root changed since session review".to_owned());
    }
    for path in [
        paths.backups.clone(),
        paths.reports.clone(),
        paths.config.clone(),
        paths.profile_drafts.clone(),
        paths.diagnostics.clone(),
        paths.root.join("profiles-v1.json"),
        paths.baseline_path(serial),
    ] {
        validate_contained(&paths.root, &path)?;
    }
    Ok(())
}

fn validate_contained(root: &Path, path: &Path) -> Result<(), String> {
    let mut existing = path;
    loop {
        match fs::symlink_metadata(existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                existing = existing
                    .parent()
                    .ok_or("pilot storage has no existing ancestor")?;
            }
            Err(error) => return Err(format!("pilot storage cannot be inspected: {error}")),
        }
    }
    let resolved = existing
        .canonicalize()
        .map_err(|error| format!("pilot storage: {error}"))?;
    reject_legacy(&resolved)?;
    if !resolved.starts_with(root) {
        return Err("pilot storage resolves outside its isolated root".to_owned());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::*;
    use crate::storage::write_snapshot;
    use std::sync::atomic::{AtomicU64, Ordering};

    pub(crate) fn fixture(writes: bool) -> PilotSession {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "viperpilot-pilot-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let paths = StoragePaths::under(root);
        let baseline = DeviceSnapshotV1 {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            captured_at_unix_ms: 1,
            device: DeviceIdentity {
                vendor_id: RAZER_VENDOR_ID,
                product_id: VIPER_V4_PRO_WIRELESS_PID,
                hid_descriptor_serial: Some("SYNTHETIC-DONGLE".to_owned()),
                serial: "SYNTHETIC-MOUSE".to_owned(),
                firmware: FirmwareVersion { major: 1, minor: 0 },
                path: "synthetic-path".to_owned(),
                interface_number: 4,
                usage_page: 12,
                usage: 1,
            },
            polling: PollingState {
                transport: TransportKind::WirelessV2,
                raw_code: 8,
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
        };
        write_snapshot(&paths, &baseline, None).unwrap();
        let mut args = vec![
            OsString::from("--pilot"),
            OsString::from("--root"),
            paths.root.into_os_string(),
            OsString::from("--confirm-device"),
            OsString::from("SYNTHETIC-MOUSE"),
        ];
        if writes {
            args.push("--allow-established-profiles".into());
        }
        PilotOptions::parse(args).unwrap().load().unwrap()
    }

    #[test]
    fn parser_rejects_missing_unknown_duplicate_and_incomplete_arguments() {
        for args in [
            vec![],
            vec!["--pilot"],
            vec!["--pilot", "--root"],
            vec!["--pilot", "--pilot"],
            vec!["--pilot", "--install"],
            vec![
                "--pilot",
                "--root",
                "x",
                "--confirm-device",
                "--allow-established-profiles",
            ],
        ] {
            assert!(PilotOptions::parse(args.into_iter().map(OsString::from)).is_err());
        }
    }

    #[test]
    fn read_only_session_rejects_writes_but_accepts_matching_observation() {
        let session = fixture(false);
        session.validate_live(session.baseline()).unwrap();
        assert!(!session.writes_enabled());
        assert!(
            session
                .authorize_profile_write(session.baseline(), session.baseline())
                .is_err()
        );
        fs::remove_dir_all(&session.paths.root).unwrap();
    }

    #[test]
    fn enabled_session_requires_exact_identity_and_complete_baseline() {
        let session = fixture(true);
        session
            .authorize_profile_write(session.baseline(), session.baseline())
            .unwrap();
        for field in 0..5 {
            let mut changed = session.baseline.clone();
            match field {
                0 => changed.device.serial.push('X'),
                1 => changed.device.hid_descriptor_serial = Some("OTHER-DONGLE".to_owned()),
                2 => changed.device.firmware.minor += 1,
                3 => changed.device.path.push('X'),
                _ => changed.device.usage += 1,
            }
            assert!(
                session
                    .authorize_profile_write(&changed, session.baseline())
                    .is_err()
            );
        }
        let mut changed = session.baseline.clone();
        changed.captured_at_unix_ms += 1;
        assert!(
            session
                .authorize_profile_write(session.baseline(), &changed)
                .is_err()
        );
        fs::remove_dir_all(&session.paths.root).unwrap();
    }

    #[test]
    fn baseline_byte_change_or_removal_invalidates_existing_session() {
        let session = fixture(true);
        let path = session.paths.baseline_path(&session.baseline.device.serial);
        let mut bytes = session.baseline_bytes.clone();
        bytes.push(b'\n');
        fs::write(&path, bytes).unwrap();
        assert!(session.validate_storage().is_err());
        assert!(
            session
                .authorize_profile_write(session.baseline(), session.baseline())
                .is_err()
        );
        fs::remove_file(path).unwrap();
        assert!(session.validate_live(session.baseline()).is_err());
        fs::remove_dir_all(&session.paths.root).unwrap();
    }

    #[test]
    fn legacy_and_relative_roots_and_outside_storage_are_rejected() {
        assert!(reject_legacy(Path::new("fixture/vIpErV4uTiLiTy/pilot")).is_err());
        let options = PilotOptions::parse(
            [
                "--pilot",
                "--root",
                "relative",
                "--confirm-device",
                "SYNTHETIC-MOUSE",
            ]
            .into_iter()
            .map(OsString::from),
        )
        .unwrap();
        assert!(options.load().is_err());
        let session = fixture(false);
        assert!(validate_contained(&session.paths.root, &std::env::temp_dir()).is_err());
        fs::remove_dir_all(&session.paths.root).unwrap();
    }

    #[test]
    fn full_serial_and_existing_baseline_are_required_without_creating_files() {
        let session = fixture(false);
        let root = session.paths.root.clone();
        let paths = session.paths.clone();
        let options = PilotOptions {
            root: root.clone(),
            serial: "MOUSE".to_owned(),
            allow_established_profiles: true,
            minimized: false,
        };
        assert!(options.load().is_err());
        assert!(!paths.baseline_path("MOUSE").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn existing_session_rejects_a_reports_junction_to_another_root() {
        let session = fixture(true);
        let outside = fixture(false);
        let literal = |path: &Path| path.to_string_lossy().replace('\'', "''");
        let command = format!(
            "New-Item -ItemType Junction -Path '{}' -Target '{}' -ErrorAction Stop | Out-Null",
            literal(&session.paths.reports),
            literal(&outside.paths.root)
        );
        let result = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &command])
            .status()
            .unwrap();
        assert!(result.success(), "create isolated NTFS junction fixture");
        assert!(session.validate_storage().is_err());
        assert!(
            session
                .authorize_profile_write(session.baseline(), session.baseline())
                .is_err()
        );
        // Remove the junction itself before cleaning the two owned fixtures.
        fs::remove_dir(&session.paths.reports).unwrap();
        outside.validate_storage().unwrap();
        fs::remove_dir_all(&session.paths.root).unwrap();
        fs::remove_dir_all(&outside.paths.root).unwrap();
    }
}
