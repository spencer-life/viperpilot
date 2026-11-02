use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

#[cfg(windows)]
use serde::Serialize;
use std::path::Path;
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
    if let Some(json) = run_offline_command(&arguments)? {
        println!("{json}");
        return Ok(());
    }
    run_platform(&arguments)
}

/// Handle offline catalog and snapshot inspection before platform dispatch.
/// These commands are available in the native CLI on Windows, Linux, and macOS.
fn run_offline_command(arguments: &[String]) -> Result<Option<String>, String> {
    if arguments.first().map(String::as_str) == Some("inspect-snapshot") {
        if arguments.len() != 2 {
            return Err("expected inspect-snapshot PATH".to_owned());
        }
        let snapshot = viper_v4_utility::storage::read_snapshot(Path::new(&arguments[1]))
            .map_err(|error| error.to_string())?;
        return serde_json::to_string_pretty(&snapshot)
            .map(Some)
            .map_err(|error| format!("could not render JSON: {error}"));
    }
    let command = match arguments.first().map(String::as_str) {
        Some("catalog-preview") => viper_v4_utility::catalog_export::preview,
        Some("catalog-export") => viper_v4_utility::catalog_export::export,
        _ => return Ok(None),
    };
    if arguments.len() != 3 || arguments[1] != "--root" {
        return Err(format!("expected {} --root DIRECTORY", arguments[0]));
    }
    let catalog = command(&PathBuf::from(&arguments[2]))?;
    catalog.to_json().map(Some)
}

#[cfg(any(windows, test))]
fn validate_observe_arguments(arguments: &[String]) -> Result<(), String> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err("observe does not accept arguments".to_owned())
    }
}

#[cfg(not(windows))]
fn run_platform(_arguments: &[String]) -> Result<(), String> {
    Err("viperctl hardware commands require the x86_64-pc-windows-msvc build".to_owned())
}

#[cfg(windows)]
fn run_platform(arguments: &[String]) -> Result<(), String> {
    match arguments[0].as_str() {
        "enumerate" => command_enumerate(&arguments[1..])?,
        "observe" => command_observe(&arguments[1..])?,
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
        "plan-restore" => command_plan_restore(&arguments[1..])?,
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
fn command_observe(arguments: &[String]) -> Result<(), String> {
    validate_observe_arguments(arguments)?;
    refuse_if_synapse_running()?;
    let mut device = RazerDevice::open_unique()?;
    let snapshot = device.read_snapshot()?;
    snapshot.validate()?;
    print_json(&snapshot)
}

#[cfg(windows)]
fn command_plan_restore(arguments: &[String]) -> Result<(), String> {
    let snapshot_path = parse_plan_restore_arguments(arguments)?;
    refuse_if_synapse_running()?;
    let target = read_snapshot(Path::new(&snapshot_path)).map_err(|error| error.to_string())?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    load_matching_baseline(&paths, &before)?;
    let plan = plan_restore(&before, &target).map_err(|error| error.to_string())?;
    print_plan(&plan)
}

#[cfg(any(windows, test))]
fn parse_plan_restore_arguments(arguments: &[String]) -> Result<String, String> {
    match arguments {
        [path] if !path.starts_with('-') => Ok(path.clone()),
        _ => Err("expected plan-restore PATH".to_owned()),
    }
}

#[cfg(windows)]
fn print_plan(plan: &viper_v4_utility::planning::WritePlan) -> Result<(), String> {
    print_json(&PlanOutput {
        packet_intents: plan.packet_intents().map_err(|error| error.to_string())?,
        write_plan: plan,
    })
}

#[cfg(any(windows, test))]
fn parse_reviewed_guard(
    arguments: &[String],
    positional_count: usize,
) -> Result<(PathBuf, String), String> {
    let mut positionals = Vec::new();
    let mut reviewed_plan = None;
    let mut confirm_device = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--reviewed-plan" | "--confirm-device" => {
                let flag = arguments[index].as_str();
                let value = arguments
                    .get(index + 1)
                    .ok_or_else(|| format!("{flag} requires a value"))?;
                if value.starts_with('-') {
                    return Err(format!("{flag} requires a value"));
                }
                let slot = if flag == "--reviewed-plan" {
                    &mut reviewed_plan
                } else {
                    &mut confirm_device
                };
                if slot.replace(value.clone()).is_some() {
                    return Err(format!("{flag} may be specified only once"));
                }
                index += 2;
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown guard argument {value:?}"));
            }
            value => {
                positionals.push(value.to_owned());
                index += 1;
            }
        }
    }
    if positionals.len() != positional_count {
        return Err("unexpected or missing positional arguments".to_owned());
    }
    let reviewed_plan =
        reviewed_plan.ok_or_else(|| "--reviewed-plan PATH is required".to_owned())?;
    let confirm_device =
        confirm_device.ok_or_else(|| "--confirm-device VALUE is required".to_owned())?;
    if reviewed_plan.is_empty() {
        return Err("--reviewed-plan requires a nonempty path".to_owned());
    }
    if confirm_device.chars().count() < 4 {
        return Err("--confirm-device requires at least four serial-suffix characters".to_owned());
    }
    Ok((PathBuf::from(reviewed_plan), confirm_device))
}

#[cfg(windows)]
fn read_reviewed_plan(path: &Path) -> Result<serde_json::Value, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("could not read reviewed plan: {error}"))?;
    reviewed_json::parse(&bytes)
}

#[cfg(windows)]
fn verify_reviewed_plan(
    reviewed: serde_json::Value,
    plan: &viper_v4_utility::planning::WritePlan,
) -> Result<(), String> {
    let expected = serde_json::to_value(PlanOutput {
        packet_intents: plan.packet_intents().map_err(|error| error.to_string())?,
        write_plan: plan,
    })
    .map_err(|error| format!("could not render recomputed plan: {error}"))?;
    verify_reviewed_plan_value(reviewed, expected)
}

// 2026-11-02: reject duplicate object keys before Value can discard evidence,
// including duplicates inside fields excluded from the eventual comparison.
#[cfg(any(windows, test))]
mod reviewed_json {
    use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
    use serde_json::Value;
    use std::fmt;

    pub(super) fn parse(bytes: &[u8]) -> Result<Value, String> {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let value = UniqueValue
            .deserialize(&mut deserializer)
            .map_err(|error| format!("reviewed plan is not valid JSON: {error}"))?;
        deserializer
            .end()
            .map_err(|error| format!("reviewed plan is not valid JSON: {error}"))?;
        Ok(value)
    }

    struct UniqueValue;

    impl<'de> DeserializeSeed<'de> for UniqueValue {
        type Value = Value;

        fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
            deserializer.deserialize_any(self)
        }
    }

    impl<'de> Visitor<'de> for UniqueValue {
        type Value = Value;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("JSON without duplicate object keys")
        }

        fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
            Ok(Value::Bool(value))
        }

        fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
            Ok(Value::Number(value.into()))
        }

        fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
            Ok(Value::Number(value.into()))
        }

        fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
            serde_json::Number::from_f64(value)
                .map(Value::Number)
                .ok_or_else(|| E::custom("non-finite JSON number"))
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
            Ok(Value::String(value.to_owned()))
        }

        fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
            Ok(Value::String(value))
        }

        fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
            let mut values = Vec::new();
            while let Some(value) = sequence.next_element_seed(UniqueValue)? {
                values.push(value);
            }
            Ok(Value::Array(values))
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
            let mut values = serde_json::Map::new();
            while let Some(key) = map.next_key::<String>()? {
                if values.contains_key(&key) {
                    return Err(de::Error::custom(format!("duplicate object key {key:?}")));
                }
                values.insert(key, map.next_value_seed(UniqueValue)?);
            }
            Ok(Value::Object(values))
        }
    }
}

#[cfg(any(windows, test))]
fn restore_review_hint(target: &Path, suffix: &str) -> String {
    format!(
        "inspect the report and resolve the current device state; prepare a fresh GET-only plan with: viperctl plan-restore \"{target}\"; review the exact plan and transaction retry/recovery behavior, save the reviewed JSON, and obtain separate approval before running: viperctl restore \"{target}\" --reviewed-plan \"<fresh reviewed restore plan path>\" --confirm-device {suffix}",
        target = target.display()
    )
}

#[cfg(any(windows, test))]
fn verify_reviewed_plan_value(
    reviewed: serde_json::Value,
    expected: serde_json::Value,
) -> Result<(), String> {
    if normalize_reviewed_plan(reviewed) != normalize_reviewed_plan(expected) {
        return Err("reviewed plan does not match the recomputed command plan".to_owned());
    }
    Ok(())
}

#[cfg(any(windows, test))]
fn normalize_reviewed_plan(mut value: serde_json::Value) -> serde_json::Value {
    if let Some(before) = value
        .pointer_mut("/write_plan/before")
        .and_then(serde_json::Value::as_object_mut)
    {
        before.remove("captured_at_unix_ms");
        before.remove("power");
    }
    value
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
    let (reviewed_path, suffix) = parse_reviewed_guard(&arguments[2..], 0)?;
    let reviewed_plan = read_reviewed_plan(&reviewed_path)?;
    refuse_if_synapse_running()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    confirm_device(&before.device.serial, &suffix)?;
    let baseline = load_matching_baseline(&paths, &before)?;
    let plan = plan_profile_with_baseline(&before, &baseline, profile)
        .map_err(|error| error.to_string())?;
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    verify_reviewed_plan(reviewed_plan, &plan)?;
    let journal = write_plan_journal(&paths, &plan).map_err(|error| error.to_string())?;
    println!("Persisted write plan before setters: {}", journal.display());
    let report = apply_write_plan(&mut device, &plan);
    let report_path = write_verification_report(&paths, &report, &before.device.serial)
        .map_err(|error| error.to_string())?;
    print_json(&report)?;
    println!("Verification report: {}", report_path.display());
    if !report.success {
        return Err(format!(
            "profile apply failed; {}",
            restore_review_hint(&paths.baseline_path(&before.device.serial), &suffix)
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
    let (reviewed_path, suffix) = parse_reviewed_guard(&arguments[1..], 0)?;
    let reviewed_plan = read_reviewed_plan(&reviewed_path)?;
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
    verify_reviewed_plan(reviewed_plan, &plan)?;
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
            "DPI-only proof failed; {}",
            restore_review_hint(&paths.baseline_path(&before.device.serial), &suffix)
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
    let (reviewed_path, suffix) = parse_reviewed_guard(&arguments[3..], 0)?;
    let reviewed_plan = read_reviewed_plan(&reviewed_path)?;
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
    verify_reviewed_plan(reviewed_plan, &plan)?;
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
            "one-button apply failed; {}",
            restore_review_hint(&paths.baseline_path(&before.device.serial), &suffix)
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
    let (reviewed_path, suffix) = parse_reviewed_guard(&arguments[2..], 0)?;
    let reviewed_plan = read_reviewed_plan(&reviewed_path)?;
    refuse_if_synapse_running()?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    confirm_device(&before.device.serial, &suffix)?;
    let _baseline = load_matching_baseline(&paths, &before)?;
    let plan = plan_polling_only(&before, polling_hz).map_err(|error| error.to_string())?;
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    verify_reviewed_plan(reviewed_plan, &plan)?;
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
            "polling-only apply failed; {}",
            restore_review_hint(&paths.baseline_path(&before.device.serial), &suffix)
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
    let (reviewed_path, suffix) = parse_reviewed_guard(&arguments[2..], 0)?;
    let reviewed_plan = read_reviewed_plan(&reviewed_path)?;
    refuse_if_synapse_running()?;
    let target = read_snapshot(Path::new(snapshot_path)).map_err(|error| error.to_string())?;
    let paths = StoragePaths::discover().map_err(|error| error.to_string())?;
    let mut device = RazerDevice::open_unique()?;
    let before = device.read_snapshot()?;
    confirm_device(&before.device.serial, &suffix)?;
    let _baseline = load_matching_baseline(&paths, &before)?;
    let plan = plan_restore(&before, &target).map_err(|error| error.to_string())?;
    require_proven_polling_writes(&plan).map_err(|error| error.to_string())?;
    verify_reviewed_plan(reviewed_plan, &plan)?;
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
    if baseline.device != current.device {
        return Err(
            "immutable baseline full device identity does not match the connected device"
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
  viperctl catalog-preview --root DIRECTORY
  viperctl catalog-export --root DIRECTORY
  viperctl inspect-snapshot PATH
  viperctl enumerate [--json]
  viperctl observe
  viperctl probe-polling
  viperctl snapshot [--output PATH]
  viperctl plan developer|gaming
  viperctl plan-dpi-1600
  viperctl verify-dpi-1600
  viperctl plan-button mouse4|mouse5 developer|gaming
  viperctl verify-button mouse4|mouse5 developer|gaming
  viperctl plan-polling 1000|4000
  viperctl plan-restore PATH
  viperctl apply developer|gaming --reviewed-plan PATH --confirm-device SERIAL_SUFFIX
  viperctl apply-dpi-1600 --reviewed-plan PATH --confirm-device SERIAL_SUFFIX
  viperctl apply-button mouse4|mouse5 developer|gaming --reviewed-plan PATH --confirm-device SERIAL_SUFFIX
  viperctl apply-polling 1000|4000 --reviewed-plan PATH --confirm-device SERIAL_SUFFIX
  viperctl verify developer|gaming
  viperctl restore PATH --reviewed-plan PATH --confirm-device SERIAL_SUFFIX
  viperctl listen

catalog-preview reads the local alias and draft libraries without writing
files. catalog-export freezes the same catalog to profile-catalog-v1.json
without replacing an existing file. Both commands are offline and hardware-free.
inspect-snapshot validates and prints a local snapshot JSON file; it does not
open a device or write files. Its output can contain private device identity.
plan-restore reads a target snapshot and emits a GET-only restore plan after
fresh device, matching-baseline and full-identity checks. It writes no files.

enumerate performs OS enumeration only. observe performs a validated live GET
and prints JSON without creating a baseline or other files. snapshot, plan,
plan-polling, and verify use documented GET reports only. apply, isolated apply
commands, and restore require an immutable baseline, an exact reviewed-plan
artifact match and exact serial suffix,
deduplicate writes, independently read back each field, and roll back on
failure. The isolated commands can only touch their named field. The tray
application remains gated on successful hardware proof."
    );
}

#[cfg(test)]
mod catalog_command_tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::run_offline_command;
    use viper_v4_utility::profile_catalog::ProfileCatalogV1;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "viperctl-catalog-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn catalog_commands_are_available_before_platform_dispatch() {
        let root = temp_root();
        let root_arg = root.to_string_lossy().into_owned();
        let preview = run_offline_command(&[
            "catalog-preview".to_owned(),
            "--root".to_owned(),
            root_arg.clone(),
        ])
        .unwrap()
        .unwrap();
        let preview_catalog = ProfileCatalogV1::from_json(&preview).unwrap();
        assert_eq!(preview_catalog.entries().len(), 2);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);

        let exported =
            run_offline_command(&["catalog-export".to_owned(), "--root".to_owned(), root_arg])
                .unwrap()
                .unwrap();
        assert_eq!(
            ProfileCatalogV1::from_json(&exported).unwrap(),
            preview_catalog
        );
        assert!(root.join("profile-catalog-v1.json").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn catalog_commands_reject_malformed_arguments() {
        assert!(run_offline_command(&["catalog-preview".to_owned()]).is_err());
        assert!(
            run_offline_command(&[
                "catalog-export".to_owned(),
                "--root".to_owned(),
                "/tmp".to_owned(),
                "extra".to_owned(),
            ])
            .is_err()
        );
        assert!(
            run_offline_command(&["snapshot".to_owned()])
                .unwrap()
                .is_none()
        );
    }
}

#[cfg(test)]
mod inspect_snapshot_command_tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::run_offline_command;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "viperctl-inspect-snapshot-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn inspect_snapshot_requires_exactly_one_path() {
        assert_eq!(
            run_offline_command(&["inspect-snapshot".to_owned()]).unwrap_err(),
            "expected inspect-snapshot PATH"
        );
        assert_eq!(
            run_offline_command(&[
                "inspect-snapshot".to_owned(),
                "snapshot.json".to_owned(),
                "extra".to_owned(),
            ])
            .unwrap_err(),
            "expected inspect-snapshot PATH"
        );
    }

    #[test]
    fn inspect_snapshot_rejects_invalid_schema_without_writing_files() {
        let root = temp_root();
        let path = root.join("invalid-snapshot.json");
        fs::write(
            &path,
            r#"{"schema_version":999,"captured_at_unix_ms":1,"device":{"vendor_id":5426,"product_id":230,"serial":"fixture","firmware":{"major":1,"minor":0},"path":"fixture","interface_number":3,"usage_page":12,"usage":1},"polling":{"transport":"wireless_v2","raw_code":8},"dpi":{"profile_id":1,"current":{"x":1600,"y":1600},"active_stage_id":1,"stages":[{"id":1,"x":1600,"y":1600}]},"button_assignments":[{"profile_id":1,"protocol_button_id":1,"mode":0,"function_id":1,"data_size":1,"data":[1,0,0,0,0]},{"profile_id":1,"protocol_button_id":2,"mode":0,"function_id":1,"data_size":1,"data":[2,0,0,0,0]},{"profile_id":1,"protocol_button_id":3,"mode":0,"function_id":1,"data_size":1,"data":[3,0,0,0,0]},{"profile_id":1,"protocol_button_id":4,"mode":0,"function_id":1,"data_size":1,"data":[4,0,0,0,0]},{"profile_id":1,"protocol_button_id":5,"mode":0,"function_id":1,"data_size":1,"data":[5,0,0,0,0]},{"profile_id":1,"protocol_button_id":96,"mode":0,"function_id":1,"data_size":1,"data":[96,0,0,0,0]}]}"#,
        )
        .unwrap();
        let result = run_offline_command(&[
            "inspect-snapshot".to_owned(),
            path.to_string_lossy().into_owned(),
        ]);
        assert!(result.is_err());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod observe_argument_tests {
    use super::validate_observe_arguments;

    #[test]
    fn observe_rejects_all_arguments_before_device_work() {
        assert!(validate_observe_arguments(&[]).is_ok());
        for argument in ["--unknown", "--output", "anything"] {
            assert_eq!(
                validate_observe_arguments(&[argument.to_owned()]).unwrap_err(),
                "observe does not accept arguments"
            );
        }
    }
}

#[cfg(test)]
mod reviewed_plan_guard_tests {
    use super::{
        normalize_reviewed_plan, parse_plan_restore_arguments, parse_reviewed_guard,
        restore_review_hint, reviewed_json, verify_reviewed_plan_value,
    };
    use serde_json::{Value, json};

    #[test]
    fn setter_guard_requires_each_unique_known_guard_before_side_effects() {
        for args in [
            vec!["--confirm-device".into(), "abcd".into()],
            vec!["--reviewed-plan".into(), "plan.json".into()],
            vec![
                "--reviewed-plan".into(),
                "a".into(),
                "--reviewed-plan".into(),
                "b".into(),
                "--confirm-device".into(),
                "abcd".into(),
            ],
            vec![
                "--reviewed-plan".into(),
                "a".into(),
                "--confirm-device".into(),
                "abcd".into(),
                "--confirm-device".into(),
                "abcd".into(),
            ],
            vec![
                "--reviewed-plan".into(),
                "a".into(),
                "--confirm-device".into(),
                "abcd".into(),
                "--unknown".into(),
            ],
        ] {
            assert!(parse_reviewed_guard(&args, 0).is_err(), "accepted {args:?}");
        }
        assert_eq!(
            parse_reviewed_guard(
                &[
                    "--reviewed-plan".into(),
                    "plan.json".into(),
                    "--confirm-device".into(),
                    "abcd".into()
                ],
                0
            )
            .unwrap(),
            (std::path::PathBuf::from("plan.json"), "abcd".to_owned())
        );
    }

    #[test]
    fn setter_guard_rejects_empty_path_and_short_suffix_before_side_effects() {
        for (path, suffix) in [
            ("", "abcd"),
            ("plan.json", ""),
            ("plan.json", "a"),
            ("plan.json", "abc"),
            ("plan.json", "ééé"),
        ] {
            assert!(
                parse_reviewed_guard(
                    &[
                        "--reviewed-plan".into(),
                        path.into(),
                        "--confirm-device".into(),
                        suffix.into()
                    ],
                    0,
                )
                .is_err(),
                "accepted path {path:?} and suffix {suffix:?}"
            );
        }
    }

    #[test]
    fn raw_json_duplicate_rollback_cannot_hide_an_altered_recovery_operation() {
        let expected =
            json!({"write_plan":{"rollback_operations":[{"set":"baseline"}]},"packet_intents":[]});
        let raw = br#"{"write_plan":{"rollback_operations":[{"set":"altered"}],"rollback_operations":[{"set":"baseline"}]},"packet_intents":[]}"#;
        // Ordinary Value parsing would erase the first, altered recovery plan.
        assert_eq!(serde_json::from_slice::<Value>(raw).unwrap(), expected);
        assert!(
            reviewed_json::parse(raw)
                .unwrap_err()
                .contains("duplicate object key")
        );
    }

    #[test]
    fn raw_json_duplicate_write_plan_cannot_erase_unknown_content() {
        let expected = json!({"write_plan":{"rollback_operations":[]},"packet_intents":[]});
        let raw = br#"{"write_plan":{"unknown_content":true},"write_plan":{"rollback_operations":[]},"packet_intents":[]}"#;
        assert_eq!(serde_json::from_slice::<Value>(raw).unwrap(), expected);
        assert!(
            reviewed_json::parse(raw)
                .unwrap_err()
                .contains("duplicate object key")
        );
    }

    #[test]
    fn raw_json_rejects_duplicates_in_arrays_and_normalized_fields() {
        for raw in [
            r#"{"packet_intents":[{"arguments":[{"x":1,"x":2}]}]}"#,
            r#"{"write_plan":{"before":{"captured_at_unix_ms":1,"\u0063aptured_at_unix_ms":2}}}"#,
            r#"{"write_plan":{"before":{"power":null,"power":{}}}}"#,
            r#"{"write_plan":{"before":{"power":{"nested":[{"x":1,"x":2}]}}}}"#,
        ] {
            assert!(
                reviewed_json::parse(raw.as_bytes())
                    .unwrap_err()
                    .contains("duplicate object key"),
                "accepted {raw}"
            );
        }
    }

    #[test]
    fn raw_json_preserves_unique_content_and_rejects_malformed_or_trailing_json() {
        let raw = br#"{"write_plan":{"before":{"captured_at_unix_ms":1,"power":{"sleep":2}},"rollback_operations":[]},"packet_intents":[{"values":[true,false,null,-1,18446744073709551615,1.25,"escaped\ntext"]}]}"#;
        let expected: Value = serde_json::from_slice(raw).unwrap();
        let reviewed = reviewed_json::parse(raw).unwrap();
        assert_eq!(reviewed, expected);
        assert!(verify_reviewed_plan_value(reviewed, expected.clone()).is_ok());
        let mut unknown = reviewed_json::parse(raw).unwrap();
        unknown["unknown"] = json!(true);
        assert!(verify_reviewed_plan_value(unknown, expected).is_err());
        for raw in [b"{} {}".as_slice(), b"{", b"{\"x\":NaN}"] {
            assert!(reviewed_json::parse(raw).is_err());
        }
    }

    #[test]
    fn failure_hint_requires_a_fresh_restore_plan_review_and_separate_approval() {
        let hint = restore_review_hint(std::path::Path::new("baseline.json"), "abcd");
        assert!(hint.contains("fresh GET-only plan with: viperctl plan-restore \"baseline.json\""));
        assert!(hint.contains("review the exact plan and transaction retry/recovery behavior"));
        assert!(hint.contains("obtain separate approval before running: viperctl restore \"baseline.json\" --reviewed-plan \"<fresh reviewed restore plan path>\" --confirm-device abcd"));
    }

    #[test]
    fn reviewed_plan_ignores_only_capture_time_and_power() {
        let expected = json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":{"x":1},"device":{"serial":"a"}},"target":{"x":1}},"packet_intents":[]});
        let mut reviewed = expected.clone();
        reviewed["write_plan"]["before"]["captured_at_unix_ms"] = json!(999);
        reviewed["write_plan"]["before"]["power"] = json!({"x":9});
        assert!(verify_reviewed_plan_value(reviewed, expected.clone()).is_ok());
        assert_eq!(
            normalize_reviewed_plan(expected.clone()),
            normalize_reviewed_plan(expected)
        );
    }

    #[test]
    fn reviewed_plan_rejects_identity_state_target_force_packets_and_unknown_content() {
        let expected = json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":null,"device":{"serial":"a"},"dpi":{"current":{"x":800}}},"target":{"dpi":1600,"force":true},"ordered_operations":[{"set":"dpi"}],"rollback_operations":[{"set":"old-dpi"}],"skipped_fields":[]},"packet_intents":[{"command_id":1}]});
        let mutations: [(&str, Value); 7] = [
            (
                "identity",
                json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":null,"device":{"serial":"b"},"dpi":{"current":{"x":800}}},"target":{"dpi":1600,"force":true},"ordered_operations":[{"set":"dpi"}],"rollback_operations":[{"set":"old-dpi"}],"skipped_fields":[]},"packet_intents":[{"command_id":1}]}),
            ),
            (
                "state",
                json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":null,"device":{"serial":"a"},"dpi":{"current":{"x":400}}},"target":{"dpi":1600,"force":true},"ordered_operations":[{"set":"dpi"}],"rollback_operations":[{"set":"old-dpi"}],"skipped_fields":[]},"packet_intents":[{"command_id":1}]}),
            ),
            (
                "target",
                json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":null,"device":{"serial":"a"},"dpi":{"current":{"x":800}}},"target":{"dpi":800,"force":true},"ordered_operations":[{"set":"dpi"}],"rollback_operations":[{"set":"old-dpi"}],"skipped_fields":[]},"packet_intents":[{"command_id":1}]}),
            ),
            (
                "operation",
                json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":null,"device":{"serial":"a"},"dpi":{"current":{"x":800}}},"target":{"dpi":1600,"force":true},"ordered_operations":[{"set":"other"}],"rollback_operations":[{"set":"old-dpi"}],"skipped_fields":[]},"packet_intents":[{"command_id":1}]}),
            ),
            (
                "rollback",
                json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":null,"device":{"serial":"a"},"dpi":{"current":{"x":800}}},"target":{"dpi":1600,"force":true},"ordered_operations":[{"set":"dpi"}],"rollback_operations":[],"skipped_fields":[]},"packet_intents":[{"command_id":1}]}),
            ),
            (
                "packet",
                json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":null,"device":{"serial":"a"},"dpi":{"current":{"x":800}}},"target":{"dpi":1600,"force":true},"ordered_operations":[{"set":"dpi"}],"rollback_operations":[{"set":"old-dpi"}],"skipped_fields":[]},"packet_intents":[{"command_id":2}]}),
            ),
            (
                "extra",
                json!({"write_plan":{"before":{"captured_at_unix_ms":1,"power":null,"device":{"serial":"a"},"dpi":{"current":{"x":800}}},"target":{"dpi":1600,"force":true},"ordered_operations":[{"set":"dpi"}],"rollback_operations":[{"set":"old-dpi"}],"skipped_fields":[],"extra":true},"packet_intents":[{"command_id":1}]}),
            ),
        ];
        for (label, changed) in mutations {
            assert!(
                verify_reviewed_plan_value(changed, expected.clone()).is_err(),
                "accepted {label}"
            );
        }
        let mut force_changed = expected.clone();
        force_changed["write_plan"]["target"]["force"] = json!(false);
        assert!(verify_reviewed_plan_value(force_changed, expected.clone()).is_err());
        let mut skipped_changed = expected.clone();
        skipped_changed["write_plan"]["skipped_fields"] = json!(["unexpected"]);
        assert!(verify_reviewed_plan_value(skipped_changed, expected).is_err());
    }

    #[test]
    fn plan_restore_arguments_are_pure_and_exact() {
        assert_eq!(
            parse_plan_restore_arguments(&["target.json".into()]).unwrap(),
            "target.json"
        );
        for args in [
            vec![],
            vec!["target.json".into(), "extra".into()],
            vec!["--unknown".into()],
        ] {
            assert!(parse_plan_restore_arguments(&args).is_err());
        }
    }
}
