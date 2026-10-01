use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

use viper_v4_utility::catalog_export;
use viper_v4_utility::profile_catalog::ProfileCatalogV1;
use viper_v4_utility::profile_intent::{
    ButtonActionIntent, DpiTarget, PROFILE_INTENT_SCHEMA_VERSION, ProfileDraftLibraryV1,
    ProfileIntentV1,
};
use viper_v4_utility::profile_library::ProfileLibraryV1;
use viper_v4_utility::storage::StoragePaths;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "viper-catalog-migration-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    root
}

fn write_valid_sources(root: &Path) -> (Vec<u8>, Vec<u8>) {
    let aliases = serde_json::to_vec_pretty(&ProfileLibraryV1::default()).unwrap();
    let mut drafts = ProfileDraftLibraryV1::default();
    drafts
        .create(
            "custom-one",
            ProfileIntentV1 {
                schema_version: PROFILE_INTENT_SCHEMA_VERSION,
                name: "Personal custom profile".into(),
                dpi: DpiTarget { x: 1600, y: 1600 },
                polling_hz: 1000,
                mouse4: ButtonActionIntent::PassThrough,
                mouse5: ButtonActionIntent::Unassigned,
            },
        )
        .unwrap();
    let drafts = serde_json::to_vec_pretty(&drafts).unwrap();
    let paths = StoragePaths::under(root.to_path_buf());
    fs::write(root.join("profiles-v1.json"), &aliases).unwrap();
    fs::write(&paths.profile_drafts, &drafts).unwrap();
    (aliases, drafts)
}

#[test]
fn migration_rejects_unknown_fields_on_both_entry_variants_and_nested_intent() {
    let aliases = ProfileLibraryV1::default();
    let mut drafts = ProfileDraftLibraryV1::default();
    drafts
        .create(
            "custom-one",
            ProfileIntentV1 {
                schema_version: PROFILE_INTENT_SCHEMA_VERSION,
                name: "Personal custom profile".into(),
                dpi: DpiTarget { x: 1600, y: 1600 },
                polling_hz: 1000,
                mouse4: ButtonActionIntent::PassThrough,
                mouse5: ButtonActionIntent::Unassigned,
            },
        )
        .unwrap();
    let catalog = ProfileCatalogV1::from_sources(&aliases, &drafts).unwrap();
    let valid = catalog.to_json().unwrap();
    assert_eq!(ProfileCatalogV1::from_json(&valid).unwrap(), catalog);

    let base: serde_json::Value = serde_json::from_str(&valid).unwrap();
    for entry_index in [0, 2] {
        let mut unknown = base.clone();
        unknown["entries"][entry_index]["future_field"] = serde_json::json!(true);
        assert!(
            ProfileCatalogV1::from_json(&unknown.to_string()).is_err(),
            "accepted an unknown field on entry {entry_index}"
        );
    }

    let mut nested_unknown = base;
    nested_unknown["entries"][2]["intent"]["future_field"] = serde_json::json!(true);
    assert!(ProfileCatalogV1::from_json(&nested_unknown.to_string()).is_err());
}

#[test]
fn serialized_catalog_cannot_promote_drafts_or_redefine_protected_presets() {
    let aliases = ProfileLibraryV1::default();
    let mut drafts = ProfileDraftLibraryV1::default();
    drafts
        .create(
            "custom-one",
            ProfileIntentV1 {
                schema_version: PROFILE_INTENT_SCHEMA_VERSION,
                name: "Personal custom profile".into(),
                dpi: DpiTarget { x: 1600, y: 1600 },
                polling_hz: 1000,
                mouse4: ButtonActionIntent::PassThrough,
                mouse5: ButtonActionIntent::Unassigned,
            },
        )
        .unwrap();
    let catalog = ProfileCatalogV1::from_sources(&aliases, &drafts).unwrap();
    let json = catalog.to_json().unwrap();
    let base: serde_json::Value = serde_json::from_str(&json).unwrap();

    let mut promoted_pair = base.clone();
    promoted_pair["quick_switch"][0] = serde_json::json!("draft:custom-one");
    assert!(ProfileCatalogV1::from_json(&promoted_pair.to_string()).is_err());

    let mut renamed_builtin = base.clone();
    renamed_builtin["entries"][0]["name"] = serde_json::json!("My Developer");
    assert!(ProfileCatalogV1::from_json(&renamed_builtin.to_string()).is_err());

    let mut replaced_builtin = base;
    replaced_builtin["entries"][0]["source_preset"] = serde_json::json!("gaming");
    assert!(ProfileCatalogV1::from_json(&replaced_builtin.to_string()).is_err());
}

#[test]
fn export_preserves_alias_draft_config_and_baseline_bytes() {
    let root = temp_root("isolation");
    let (aliases, drafts) = write_valid_sources(&root);
    let paths = StoragePaths::under(root.clone());
    let config = b"synthetic config sentinel";
    let baseline = b"synthetic immutable baseline sentinel";
    fs::write(&paths.config, config).unwrap();
    fs::create_dir_all(&paths.backups).unwrap();
    let baseline_path = paths.baseline_path("synthetic-device");
    fs::write(&baseline_path, baseline).unwrap();

    let catalog = catalog_export::export(&root).unwrap();
    assert_eq!(
        ProfileCatalogV1::from_json(&catalog.to_json().unwrap()).unwrap(),
        catalog
    );
    assert_eq!(fs::read(root.join("profiles-v1.json")).unwrap(), aliases);
    assert_eq!(fs::read(&paths.profile_drafts).unwrap(), drafts);
    assert_eq!(fs::read(&paths.config).unwrap(), config);
    assert_eq!(fs::read(baseline_path).unwrap(), baseline);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_refuses_valid_sources_when_corrupt_output_already_exists() {
    let root = temp_root("corrupt-target");
    write_valid_sources(&root);
    let target = root.join("profile-catalog-v1.json");
    let sentinel = b"existing unrelated bytes; preserve exactly";
    fs::write(&target, sentinel).unwrap();

    let error = catalog_export::export(&root).unwrap_err();
    assert!(
        error.contains("refusing to overwrite"),
        "unexpected error: {error}"
    );
    assert_eq!(fs::read(target).unwrap(), sentinel);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_exports_publish_exactly_one_complete_catalog() {
    let root = temp_root("race");
    let (aliases, drafts) = write_valid_sources(&root);
    let root = Arc::new(root);
    let start = Arc::new(Barrier::new(3));
    let workers = (0..2)
        .map(|_| {
            let root = Arc::clone(&root);
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                start.wait();
                catalog_export::export(&root)
            })
        })
        .collect::<Vec<_>>();
    start.wait();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .any(|error| error.contains("refusing to overwrite"))
    );
    let encoded = fs::read_to_string(root.join("profile-catalog-v1.json")).unwrap();
    assert!(ProfileCatalogV1::from_json(&encoded).is_ok());
    assert_eq!(fs::read(root.join("profiles-v1.json")).unwrap(), aliases);
    assert_eq!(
        fs::read(root.join("profile-drafts-v1.json")).unwrap(),
        drafts
    );
    let remaining_files = fs::read_dir(root.as_ref())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(
        remaining_files
            .iter()
            .all(|name| !name.starts_with(".profile-catalog-v1."))
    );

    fs::remove_dir_all(root.as_ref()).unwrap();
}

#[test]
fn actual_cli_preview_and_export_are_offline_and_no_clobber() {
    let root = temp_root("cli");
    write_valid_sources(&root);
    let root_arg = root.to_string_lossy().into_owned();
    let binary = env!("CARGO_BIN_EXE_viperctl");

    let preview_result = Command::new(binary)
        .args(["catalog-preview", "--root", &root_arg])
        .output()
        .unwrap();
    assert!(
        preview_result.status.success(),
        "preview failed: {}",
        String::from_utf8_lossy(&preview_result.stderr)
    );
    let preview_json = String::from_utf8(preview_result.stdout).unwrap();
    let preview_catalog = ProfileCatalogV1::from_json(&preview_json).unwrap();
    let catalog_path = root.join("profile-catalog-v1.json");
    assert!(!catalog_path.exists());

    let export_result = Command::new(binary)
        .args(["catalog-export", "--root", &root_arg])
        .output()
        .unwrap();
    assert!(
        export_result.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&export_result.stderr)
    );
    let export_json = String::from_utf8(export_result.stdout).unwrap();
    assert_eq!(
        ProfileCatalogV1::from_json(&export_json).unwrap(),
        preview_catalog
    );
    let published_bytes = fs::read(&catalog_path).unwrap();

    let second_export = Command::new(binary)
        .args(["catalog-export", "--root", &root_arg])
        .output()
        .unwrap();
    assert!(!second_export.status.success());
    assert!(String::from_utf8_lossy(&second_export.stderr).contains("refusing to overwrite"));
    assert_eq!(fs::read(catalog_path).unwrap(), published_bytes);

    fs::remove_dir_all(root).unwrap();
}
