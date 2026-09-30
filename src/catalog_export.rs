//! Offline catalog preview and no-clobber export.
//!
//! Added 2026-09-29 as opt-in derived data; production V1 stores stay unchanged.
//!
//! This module reads only the local preset aliases and profile drafts. Export
//! creates one derived file and never writes either source library.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::profile_catalog::ProfileCatalogV1;
use crate::profile_intent::ProfileDraftLibraryV1;
use crate::profile_library::ProfileLibraryV1;
use crate::storage::{self, StoragePaths};

const CATALOG_FILE_NAME: &str = "profile-catalog-v1.json";
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct CatalogSources {
    catalog: ProfileCatalogV1,
    aliases_bytes: Option<Vec<u8>>,
    drafts_bytes: Option<Vec<u8>>,
    target: PathBuf,
}

/// Build the catalog entirely in memory after validating both source files.
pub fn preview(root: &Path) -> Result<ProfileCatalogV1, String> {
    Ok(load_sources(root)?.catalog)
}

/// Export a frozen catalog without replacing any existing file.
pub fn export(root: &Path) -> Result<ProfileCatalogV1, String> {
    let sources = load_sources(root)?;
    let json = sources.catalog.to_json()?;
    // Validate the exact serialized representation before creating a temp file.
    let round_trip = ProfileCatalogV1::from_json(&json)?;
    if round_trip != sources.catalog {
        return Err("serialized profile catalog did not round-trip exactly".to_owned());
    }

    let mut bytes = json.into_bytes();
    bytes.push(b'\n');
    let temp_path = create_synced_temp(&sources.target, &bytes)?;
    let _cleanup = TempFileGuard(temp_path.clone());

    // The source byte check is deliberately close to publication. This is an
    // offline consistency check, not a lock against concurrent writers.
    ensure_sources_unchanged(
        root,
        sources.aliases_bytes.as_deref(),
        sources.drafts_bytes.as_deref(),
    )?;
    fs::hard_link(&temp_path, &sources.target).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            format!(
                "refusing to overwrite existing {}",
                sources.target.display()
            )
        } else {
            format!(
                "could not publish {} without replacing an existing file: {error}",
                sources.target.display()
            )
        }
    })?;
    fs::remove_file(&temp_path).map_err(|error| {
        format!("catalog was published but temporary link cleanup failed: {error}")
    })?;
    Ok(sources.catalog)
}

fn load_sources(root: &Path) -> Result<CatalogSources, String> {
    let metadata = fs::metadata(root)
        .map_err(|error| format!("could not inspect catalog root {}: {error}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "catalog root is not a directory: {}",
            root.display()
        ));
    }

    let paths = StoragePaths::under(root.to_path_buf());
    let aliases_path = root.join("profiles-v1.json");
    let aliases_bytes = read_optional(&aliases_path)?;
    let drafts_bytes = read_optional(&paths.profile_drafts)?;

    // Use the established storage readers so missing files get in-memory
    // defaults and all existing schema and semantic validation stays central.
    let aliases = storage::load_profile_library(&paths)
        .map_err(|error| format!("could not load preset aliases: {error}"))?;
    let drafts = storage::load_profile_drafts(&paths)
        .map_err(|error| format!("could not load local profile drafts: {error}"))?;

    // Tie the helper results to the exact byte snapshots kept for the publish
    // check. A source that changed while loading is rejected.
    if parse_or_default::<ProfileLibraryV1>(aliases_bytes.as_deref(), ProfileLibraryV1::default())?
        != aliases
        || parse_or_default::<ProfileDraftLibraryV1>(
            drafts_bytes.as_deref(),
            ProfileDraftLibraryV1::default(),
        )? != drafts
    {
        return Err("catalog sources changed while they were being loaded".to_owned());
    }
    ensure_sources_unchanged(root, aliases_bytes.as_deref(), drafts_bytes.as_deref())?;

    let catalog = ProfileCatalogV1::from_sources(&aliases, &drafts)?;
    catalog.validate()?;
    Ok(CatalogSources {
        catalog,
        aliases_bytes,
        drafts_bytes,
        target: root.join(CATALOG_FILE_NAME),
    })
}

fn parse_or_default<T>(bytes: Option<&[u8]>, default: T) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    match bytes {
        Some(bytes) => serde_json::from_slice(bytes)
            .map_err(|error| format!("could not decode catalog source snapshot: {error}")),
        None => Ok(default),
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("could not read {}: {error}", path.display())),
    }
}

fn ensure_sources_unchanged(
    root: &Path,
    aliases: Option<&[u8]>,
    drafts: Option<&[u8]>,
) -> Result<(), String> {
    let paths = StoragePaths::under(root.to_path_buf());
    if read_optional(&root.join("profiles-v1.json"))?.as_deref() != aliases
        || read_optional(&paths.profile_drafts)?.as_deref() != drafts
    {
        return Err("catalog source files changed before publication; retry the export".to_owned());
    }
    Ok(())
}

fn create_synced_temp(target: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    for _ in 0..32 {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            ".profile-catalog-v1.{}.{}.tmp",
            std::process::id(),
            sequence
        );
        let temp = target
            .parent()
            .ok_or_else(|| "catalog target has no parent directory".to_owned())?
            .join(name);
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!("could not create catalog temporary file: {error}"));
            }
        };
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&temp);
            return Err(format!(
                "could not write complete catalog temporary file: {error}"
            ));
        }
        return Ok(temp);
    }
    Err("could not allocate a unique catalog temporary file".to_owned())
}

struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "viper-catalog-{label}-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }

    fn directory_names(root: &Path) -> Vec<String> {
        let mut names = fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[test]
    fn preview_uses_missing_defaults_without_creating_files() {
        let root = temp_root("preview");
        let expected = ProfileCatalogV1::from_sources(
            &ProfileLibraryV1::default(),
            &ProfileDraftLibraryV1::default(),
        )
        .unwrap();

        assert_eq!(preview(&root).unwrap(), expected);
        assert!(directory_names(&root).is_empty());
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn export_round_trips_without_changing_sources_and_refuses_second_export() {
        let root = temp_root("export");
        let paths = StoragePaths::under(root.clone());
        let aliases = root.join("profiles-v1.json");
        let alias_bytes = serde_json::to_vec_pretty(&ProfileLibraryV1::default()).unwrap();
        let draft_bytes = serde_json::to_vec_pretty(&ProfileDraftLibraryV1::default()).unwrap();
        fs::write(&aliases, &alias_bytes).unwrap();
        fs::write(&paths.profile_drafts, &draft_bytes).unwrap();

        let catalog = export(&root).unwrap();
        let catalog_path = root.join(CATALOG_FILE_NAME);
        let encoded = fs::read_to_string(&catalog_path).unwrap();
        assert_eq!(ProfileCatalogV1::from_json(&encoded).unwrap(), catalog);
        assert_eq!(fs::read(&aliases).unwrap(), alias_bytes);
        assert_eq!(fs::read(&paths.profile_drafts).unwrap(), draft_bytes);
        assert_eq!(
            directory_names(&root),
            vec![
                CATALOG_FILE_NAME.to_owned(),
                "profile-drafts-v1.json".to_owned(),
                "profiles-v1.json".to_owned(),
            ]
        );

        let existing_catalog_bytes = fs::read(&catalog_path).unwrap();
        assert!(export(&root).unwrap_err().contains("refusing to overwrite"));
        assert_eq!(fs::read(catalog_path).unwrap(), existing_catalog_bytes);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_source_and_existing_corrupt_output_are_preserved() {
        let root = temp_root("corrupt");
        let aliases = root.join("profiles-v1.json");
        fs::write(&aliases, b"{broken").unwrap();
        assert!(preview(&root).is_err());
        assert!(export(&root).is_err());
        assert!(!root.join(CATALOG_FILE_NAME).exists());

        let output = root.join(CATALOG_FILE_NAME);
        let corrupt_bytes = b"user data: keep me";
        fs::write(&output, corrupt_bytes).unwrap();
        assert!(export(&root).is_err());
        assert_eq!(fs::read(output).unwrap(), corrupt_bytes);
        assert_eq!(
            directory_names(&root),
            vec![CATALOG_FILE_NAME.to_owned(), "profiles-v1.json".to_owned(),]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_missing_or_non_directory_root_without_creating_it() {
        let missing = std::env::temp_dir().join(format!(
            "viper-catalog-missing-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(preview(&missing).is_err());
        assert!(!missing.exists());

        let root = temp_root("file-root");
        let file_root = root.join("not-a-directory");
        fs::write(&file_root, b"x").unwrap();
        assert!(export(&file_root).is_err());
        assert_eq!(fs::read(&file_root).unwrap(), b"x");
        fs::remove_dir_all(root).unwrap();
    }
}
