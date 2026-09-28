use std::env;
use std::process::ExitCode;

#[cfg(windows)]
use serde::Serialize;
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use viper_v4_utility::engine::{DeviceControl, apply_write_plan};
#[cfg(windows)]
use viper_v4_utility::model::{DeviceSnapshotV1, ProfileName};
#[cfg(windows)]
use viper_v4_utility::planning::{
    PacketIntent, WritePlan, plan_button_only, plan_dpi_1600_only, plan_polling_only,
    plan_profile_with_baseline, plan_restore, require_clean_button_proof_start,
    require_proven_polling_writes,
};
#[cfg(windows)]
use viper_v4_utility::storage::{
    StoragePaths, load_config, read_snapshot, save_config, write_plan_journal, write_snapshot,
    write_verification_report,
};
#[cfg(windows)]
use viper_v4_utility::windows::{
    RazerDevice, enumerate_devices, listen, probe_wireless_polling_paths, refuse_if_synapse_running,
};

#[cfg(windows)]
#[derive(Serialize)]
struct PlanOutput<'a> {
    write_plan: &'a WritePlan,
    packet_intents: Vec<PacketIntent>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.is_empty() || matches!(arguments[0].as_str(), "help" | "--help" | "-h") {
        print_help();
        return Ok(());
    }
    run_platform(&arguments)
}

#[cfg(not(windows))]
fn run_platform(_arguments: &[String]) -> Result<(), String> {
    Err("viperctl hardware commands require the x86_64-pc-windows-msvc build".to_owned())
}

#[cfg(windows)]
fn run_platform(arguments: &[String]) -> Result<(), String> {
    match arguments[0].as_str() {
        "enumerate" => command_enumerate(&arguments[1..])?,
        "probe-polling" => {
            require_exact_argument_count(arguments, 1)?;
            refuse_if_synapse_running()?;
            print_json(&probe_wireless_polling_paths()?)?;
        }
        "snapshot" => command_snapshot(&arguments[1..])?,
        "plan" => {
            let profile = parse_profile(arguments.get(1))?;
            require_exact_argument_count(arguments, 2)?;
            command_profile_read(profile, false)?;
        }
        "plan-dpi-1600" => {
            require_exact_argument_count(arguments, 1)?;
            command_dpi_read(true, false)?;
        }
        "verify-dpi-1600" => {
            require_exact_argument_count(arguments, 1)?;
            command_dpi_read(false, true)?;
        }
        "plan-button" => {
            let button_id = parse_side_button(arguments.get(1))?;
            let profile = parse_profile(arguments.get(2))?;
            require_exact_argument_count(arguments, 3)?;
            command_button_read(button_id, profile, false)?;
        }
        "verify-button" => {
            let button_id = parse_side_button(arguments.get(1))?;
            let profile = parse_profile(arguments.get(2))?;
            require_exact_argument_count(arguments, 3)?;
            command_button_read(button_id, profile, true)?;
        }
        "plan-polling" => {
            let polling_hz = parse_polling_hz(arguments.get(1))?;
            require_exact_argument_count(arguments, 2)?;
            command_polling_read(polling_hz)?;
        }
        "verify" => {
            let profile = parse_profile(arguments.get(1))?;
            require_exact_argument_count(arguments, 2)?;
            command_profile_read(profile, true)?;
        }
        "apply" => command_apply(arguments)?,
        "apply-dpi-1600" => command_apply_dpi(arguments)?,
        "apply-button" => command_apply_button(arguments)?,
        "apply-polling" => command_apply_polling(arguments)?,
        "restore" => command_restore(arguments)?,
        "listen" => {
            require_exact_argument_count(arguments, 1)?;
            listen()?;
        }
        other => return Err(format!("unknown command {other:?}; run viperctl --help")),
    }
    Ok(())
}

#[cfg(windows)]
fn command_enumerate(arguments: &[String]) -> Result<(), String> {
    let json = parse_flag_only(arguments, "--json")?;
    let devices = enumerate_devices()?;
    if json {
        return print_json(&devices);
    }
    if devices.is_empty() {
        println!("No Viper V4 Pro HID collections found.");
        return Ok(());
    }
    for device in devices {
        println!(
            "VID={:04x} PID={:04x} interface={} usage={:04x}:{:04x} rank={} serial={} path={}",
            device.vendor_id,
            device.product_id,
            device.interface_number,
            device.usage_page,
            device.usage,
            device.control_preference_rank,
            device.serial_descriptor.as_deref().unwrap_or("<none>"),
            device.path
        );
    }
    Ok(())
}

#[cfg(windows)]
fn command_snapshot(arguments: &[String]) -> Result<(), String> {
    let output = parse_optional_path(arguments, "--output")?;
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let snapshot = device.read_snapshot()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let written =
        write_snapshot(&paths, &snapshot, output.as_deref()).map_err(|error| error.to_string())?;
    print_json(&snapshot)?;
    println!("Snapshot: {}", written.path.display());
    println!("Immutable baseline: {}", written.baseline_path.display());
    println!("Baseline created now: {}", written.baseline_created);
    Ok(())
}

#[cfg(windows)]
fn command_profile_read(profile: ProfileName, require_match: bool) -> Result<(), String> {
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let snapshot = device.read_snapshot()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let baseline = load_matching_baseline(&paths, &snapshot)?;
    let plan = plan_profile_with_baseline(&snapshot, &baseline, profile)
        .map_err(|error| error.to_string())?;
    print_json(&PlanOutput {
        packet_intents: plan.packet_intents().map_err(|error| error.to_string())?,
        write_plan: &plan,
    })?;
    if require_match && !plan.is_noop() {
        return Err(format!("hardware does not match the {profile} profile"));
    }
    if require_match {
        println!("Verified {profile}; no setters were sent.");
    }
    Ok(())
}

#[cfg(windows)]
fn command_dpi_read(force_write: bool, require_match: bool) -> Result<(), String> {
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let snapshot = device.read_snapshot()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let _baseline = load_matching_baseline(&paths, &snapshot)?;
    let plan = plan_dpi_1600_only(&snapshot, force_write).map_err(|error| error.to_string())?;
    print_json(&PlanOutput {
        packet_intents: plan.packet_intents().map_err(|error| error.to_string())?,
        write_plan: &plan,
    })?;
    if require_match && !plan.is_noop() {
        return Err(
            "hardware does not have 1600 DPI in both current and active-stage reads".to_owned(),
        );
    }
    if require_match {
        println!("Verified 1600 DPI through current-DPI and stage-table GETs; no setter was sent.");
    }
    Ok(())
}

#[cfg(windows)]
fn command_button_read(
    button_id: u8,
    profile: ProfileName,
    require_match: bool,
) -> Result<(), String> {
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let snapshot = device.read_snapshot()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let baseline = load_matching_baseline(&paths, &snapshot)?;
    let plan = plan_button_only(&snapshot, &baseline, profile, button_id)
        .map_err(|error| error.to_string())?;
    print_json(&PlanOutput {
        packet_intents: plan.packet_intents().map_err(|error| error.to_string())?,
        write_plan: &plan,
    })?;
    if require_match && !plan.is_noop() {
        return Err(format!(
            "protocol button 0x{button_id:02x} does not match the {profile} assignment"
        ));
    }
    if require_match {
        println!("Verified protocol button 0x{button_id:02x} as {profile}; no setter was sent.");
    }
    Ok(())
}

#[cfg(windows)]
fn command_polling_read(polling_hz: u16) -> Result<(), String> {
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let snapshot = device.read_snapshot()?;
    let plan = plan_polling_only(&snapshot, polling_hz).map_err(|error| error.to_string())?;
    print_json(&PlanOutput {
        packet_intents: plan.packet_intents().map_err(|error| error.to_string())?,
        write_plan: &plan,
    })?;
    Ok(())
}

#[cfg(windows)]
fn command_apply(arguments: &[String]) -> Result<(), String> {
    let profile = parse_profile(arguments.get(1))?;
    let suffix = parse_required_value(&arguments[2..], "--confirm-device")?;
    refuse_if_synapse_running()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    confirm_device(&before.device.serial, &suffix)?;
    let baseline = load_matching_baseline(&paths, &before)?;
    let plan = plan_profile_with_baseline(&before, &baseline, profile)
        .map_err(|error| error.to_string())?;
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    let journal = write_plan_journal(&paths, &plan).map_err(|error| error.to_string())?;
    println!("Persisted write plan before setters: {}", journal.display());
    let report = apply_write_plan(&mut device, &plan);
    let report_path = write_verification_report(&paths, &report, &before.device.serial)
        .map_err(|error| error.to_string())?;
    print_json(&report)?;
    println!("Verification report: {}", report_path.display());
    if !report.success {
        return Err(format!(
            "profile apply failed; inspect the report and run: viperctl restore \"{}\" --confirm-device {}",
            paths.baseline_path(&before.device.serial).display(),
            suffix
        ));
    }
    let mut config = load_config(&paths).map_err(|error| error.to_string())?;
    config.last_verified_profile = Some(profile);
    save_config(&paths, &config).map_err(|error| error.to_string())?;
    println!("Applied and independently verified {profile}.");
    Ok(())
}

#[cfg(windows)]
fn command_apply_dpi(arguments: &[String]) -> Result<(), String> {
    let suffix = parse_required_value(&arguments[1..], "--confirm-device")?;
    refuse_if_synapse_running()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    confirm_device(&before.device.serial, &suffix)?;
    let baseline = load_matching_baseline(&paths, &before)?;
    let baseline_plan = plan_restore(&before, &baseline).map_err(|error| error.to_string())?;
    if !baseline_plan.is_noop() {
        return Err(
            "DPI-only proof must start from the complete immutable baseline; restore it first"
                .to_owned(),
        );
    }
    let plan = plan_dpi_1600_only(&before, true).map_err(|error| error.to_string())?;
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    let journal = write_plan_journal(&paths, &plan).map_err(|error| error.to_string())?;
    println!(
        "Persisted forced 1600-DPI-only plan before setter: {}",
        journal.display()
    );
    let report = apply_write_plan(&mut device, &plan);
    let report_path = write_verification_report(&paths, &report, &before.device.serial)
        .map_err(|error| error.to_string())?;
    print_json(&report)?;
    println!("Verification report: {}", report_path.display());
    if !report.success {
        return Err(format!(
            "DPI-only proof failed; inspect the report and run: viperctl restore \"{}\" --confirm-device {}",
            paths.baseline_path(&before.device.serial).display(),
            suffix
        ));
    }
    println!(
        "Forced and independently verified 1600 DPI through both DPI GETs; no polling or button setter was sent."
    );
    Ok(())
}

#[cfg(windows)]
fn command_apply_button(arguments: &[String]) -> Result<(), String> {
    let button_id = parse_side_button(arguments.get(1))?;
    let profile = parse_profile(arguments.get(2))?;
    let suffix = parse_required_value(&arguments[3..], "--confirm-device")?;
    refuse_if_synapse_running()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    confirm_device(&before.device.serial, &suffix)?;
    let baseline = load_matching_baseline(&paths, &before)?;
    require_clean_button_proof_start(&before, &baseline, profile, button_id)
        .map_err(|error| error.to_string())?;
    let plan = plan_button_only(&before, &baseline, profile, button_id)
        .map_err(|error| error.to_string())?;
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    let journal = write_plan_journal(&paths, &plan).map_err(|error| error.to_string())?;
    println!(
        "Persisted one-button plan before setter: {}",
        journal.display()
    );
    let report = apply_write_plan(&mut device, &plan);
    let report_path = write_verification_report(&paths, &report, &before.device.serial)
        .map_err(|error| error.to_string())?;
    print_json(&report)?;
    println!("Verification report: {}", report_path.display());
    if !report.success {
        return Err(format!(
            "one-button apply failed; inspect the report and run: viperctl restore \"{}\" --confirm-device {}",
            paths.baseline_path(&before.device.serial).display(),
            suffix
        ));
    }
    println!(
        "Applied and independently verified only protocol button 0x{button_id:02x} as {profile}; no DPI, polling, or other-button setter was sent."
    );
    Ok(())
}

#[cfg(windows)]
fn command_apply_polling(arguments: &[String]) -> Result<(), String> {
    let polling_hz = parse_polling_hz(arguments.get(1))?;
    let suffix = parse_required_value(&arguments[2..], "--confirm-device")?;
    refuse_if_synapse_running()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    confirm_device(&before.device.serial, &suffix)?;
    let _baseline = load_matching_baseline(&paths, &before)?;
    let plan = plan_polling_only(&before, polling_hz).map_err(|error| error.to_string())?;
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    let journal = write_plan_journal(&paths, &plan).map_err(|error| error.to_string())?;
    println!(
        "Persisted polling-only plan before setters: {}",
        journal.display()
    );
    let report = apply_write_plan(&mut device, &plan);
    let report_path = write_verification_report(&paths, &report, &before.device.serial)
        .map_err(|error| error.to_string())?;
    print_json(&report)?;
    println!("Verification report: {}", report_path.display());
    if !report.success {
        return Err(format!(
            "polling-only apply failed; inspect the report and run: viperctl restore \"{}\" --confirm-device {}",
            paths.baseline_path(&before.device.serial).display(),
            suffix
        ));
    }
    println!(
        "Applied and independently verified {polling_hz} Hz; no DPI or button setter was sent."
    );
    Ok(())
}

#[cfg(windows)]
fn command_restore(arguments: &[String]) -> Result<(), String> {
    let snapshot_path = arguments
        .get(1)
        .ok_or_else(|| "restore requires a snapshot path".to_owned())?;
    let suffix = parse_required_value(&arguments[2..], "--confirm-device")?;
    refuse_if_synapse_running()?;
    let target = read_snapshot(Path::new(snapshot_path)).map_err(|error| error.to_string())?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    confirm_device(&before.device.serial, &suffix)?;
    let _baseline = load_matching_baseline(&paths, &before)?;
    let plan = plan_restore(&before, &target).map_err(|error| error.to_string())?;
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    let journal = write_plan_journal(&paths, &plan).map_err(|error| error.to_string())?;
    println!(
        "Persisted restore plan before setters: {}",
        journal.display()
    );
    let report = apply_write_plan(&mut device, &plan);
    let report_path = write_verification_report(&paths, &report, &before.device.serial)
        .map_err(|error| error.to_string())?;
    print_json(&report)?;
    println!("Verification report: {}", report_path.display());
    if !report.success {
        return Err(format!(
            "restore failed; keep the mouse connected and inspect {}",
            report_path.display()
        ));
    }
    println!("Exact snapshot restore verified.");
    Ok(())
}

#[cfg(windows)]
fn load_matching_baseline(
    paths: &StoragePaths,
    current: &DeviceSnapshotV1,
) -> Result<DeviceSnapshotV1, String> {
    let baseline_path = paths.baseline_path(&current.device.serial);
    let baseline = read_snapshot(&baseline_path).map_err(|error| {
        format!(
            "a complete immutable baseline is required before setters: {} ({error})",
            baseline_path.display()
        )
    })?;
    if baseline.device.vendor_id != current.device.vendor_id
        || baseline.device.product_id != current.device.product_id
        || baseline.device.hid_descriptor_serial != current.device.hid_descriptor_serial
        || baseline.device.serial != current.device.serial
        || baseline.device.firmware != current.device.firmware
        || baseline.device.interface_number != current.device.interface_number
        || baseline.device.usage_page != current.device.usage_page
        || baseline.device.usage != current.device.usage
    {
        return Err(
            "immutable baseline VID/PID, dongle/body serial, firmware, or control collection does not match the connected device"
                .to_owned(),
        );
    }
    Ok(baseline)
}

#[cfg(windows)]
fn confirm_device(serial: &str, suffix: &str) -> Result<(), String> {
    if suffix.chars().count() < 4 {
        return Err("--confirm-device requires at least four serial-suffix characters".to_owned());
    }
    if !serial.ends_with(suffix) {
        return Err(format!(
            "serial confirmation mismatch: connected serial does not end with {suffix:?}"
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn parse_profile(value: Option<&String>) -> Result<ProfileName, String> {
    value
        .ok_or_else(|| "profile argument is required".to_owned())?
        .parse()
}

#[cfg(windows)]
fn parse_polling_hz(value: Option<&String>) -> Result<u16, String> {
    match value.map(String::as_str) {
        Some("1000") => Ok(1000),
        Some("4000") => Ok(4000),
        _ => Err("polling target must be exactly 1000 or 4000".to_owned()),
    }
}

#[cfg(windows)]
fn parse_side_button(value: Option<&String>) -> Result<u8, String> {
    match value.map(String::as_str) {
        Some("mouse4") => Ok(viper_v4_utility::model::BUTTON_MOUSE4_SLOT),
        Some("mouse5") => Ok(viper_v4_utility::model::BUTTON_MOUSE5_SLOT),
        _ => Err("side button must be exactly mouse4 or mouse5".to_owned()),
    }
}

#[cfg(windows)]
fn parse_flag_only(arguments: &[String], flag: &str) -> Result<bool, String> {
    match arguments {
        [] => Ok(false),
        [value] if value == flag => Ok(true),
        _ => Err(format!("expected no arguments or {flag}")),
    }
}

#[cfg(windows)]
fn parse_optional_path(arguments: &[String], flag: &str) -> Result<Option<PathBuf>, String> {
    match arguments {
        [] => Ok(None),
        [observed_flag, value] if observed_flag == flag => Ok(Some(PathBuf::from(value))),
        _ => Err(format!("expected no arguments or {flag} PATH")),
    }
}

#[cfg(windows)]
fn parse_required_value(arguments: &[String], flag: &str) -> Result<String, String> {
    match arguments {
        [observed_flag, value] if observed_flag == flag => Ok(value.clone()),
        _ => Err(format!("expected {flag} VALUE")),
    }
}

#[cfg(windows)]
fn require_exact_argument_count(arguments: &[String], count: usize) -> Result<(), String> {
    if arguments.len() == count {
        Ok(())
    } else {
        Err("unexpected extra arguments".to_owned())
    }
}

#[cfg(windows)]
fn print_json<T: serde::Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| format!("could not render JSON: {error}"))?
    );
    Ok(())
}

fn print_help() {
    println!(
        "\
viperctl — Viper V4 Pro backup, verification, and diagnostics

USAGE:
  viperctl enumerate [--json]
  viperctl probe-polling
  viperctl snapshot [--output PATH]
  viperctl plan developer|gaming
  viperctl plan-dpi-1600
  viperctl verify-dpi-1600
  viperctl plan-button mouse4|mouse5 developer|gaming
  viperctl verify-button mouse4|mouse5 developer|gaming
  viperctl plan-polling 1000|4000
  viperctl apply developer|gaming --confirm-device SERIAL_SUFFIX
  viperctl apply-dpi-1600 --confirm-device SERIAL_SUFFIX
  viperctl apply-button mouse4|mouse5 developer|gaming --confirm-device SERIAL_SUFFIX
  viperctl apply-polling 1000|4000 --confirm-device SERIAL_SUFFIX
  viperctl verify developer|gaming
  viperctl restore PATH --confirm-device SERIAL_SUFFIX
  viperctl listen

enumerate performs OS enumeration only. snapshot, plan, plan-polling, and verify
use documented GET reports only. apply, isolated apply commands, and restore require an
immutable baseline and exact serial suffix, deduplicate writes, independently
read back each field, and roll back on failure. The isolated commands can only
touch their named field. The tray application remains gated on successful hardware proof."
    );
}
