// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::paths::Layout;
use crate::{PanelError, PanelResult, ServerRecord};

#[derive(Debug, Default, Serialize, Deserialize)]
struct CatalogFile {
    servers: Vec<CatalogEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CatalogEntry {
    id: String,
    root: PathBuf,
}

/// Records are cached per catalog file; every write goes through this module and refreshes it.
fn cache() -> &'static Mutex<HashMap<PathBuf, Vec<ServerRecord>>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Vec<ServerRecord>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock() -> std::sync::MutexGuard<'static, HashMap<PathBuf, Vec<ServerRecord>>> {
    cache().lock().unwrap_or_else(|error| error.into_inner())
}

pub fn list(layout: &Layout) -> PanelResult<Vec<ServerRecord>> {
    let mut guard = lock();
    let key = layout.catalog_file();
    if let Some(records) = guard.get(&key) {
        return Ok(records.clone());
    }
    let records = load(&key)?;
    guard.insert(key, records.clone());
    Ok(records)
}

/// Drops the cache so that edits made outside VoxelPanel become visible.
pub fn refresh(layout: &Layout) -> PanelResult<Vec<ServerRecord>> {
    lock().remove(&layout.catalog_file());
    list(layout)
}

pub fn get(layout: &Layout, id: &str) -> PanelResult<ServerRecord> {
    list(layout)?
        .into_iter()
        .find(|record| record.id == id)
        .ok_or_else(|| PanelError::not_found("Server non trovato"))
}

pub fn save(layout: &Layout, record: &ServerRecord) -> PanelResult<()> {
    let mut guard = lock();
    crate::paths::ensure_dir(&record.root)?;
    let key = layout.catalog_file();
    let mut catalog = read_catalog(&key)?;
    if let Some(entry) = catalog.servers.iter_mut().find(|entry| entry.id == record.id) {
        entry.root = record.root.clone();
    } else {
        if catalog
            .servers
            .iter()
            .any(|entry| crate::platform::same_path(&entry.root, &record.root))
        {
            return Err(PanelError::invalid("Questa cartella è già importata."));
        }
        catalog.servers.push(CatalogEntry {
            id: record.id.clone(),
            root: record.root.clone(),
        });
    }
    write_json(&record.root.join("server.json"), record)?;
    crate::paths::ensure_dir(&layout.root)?;
    write_json(&key, &catalog)?;
    guard.remove(&key);
    Ok(())
}

pub fn remove(layout: &Layout, id: &str) -> PanelResult<Option<ServerRecord>> {
    let mut guard = lock();
    let key = layout.catalog_file();
    let mut catalog = read_catalog(&key)?;
    let Some(index) = catalog.servers.iter().position(|entry| entry.id == id) else {
        return Err(PanelError::not_found("Server non trovato"));
    };
    let entry = catalog.servers.remove(index);
    let record = read_or_stub(&entry).ok();
    write_json(&key, &catalog)?;
    guard.remove(&key);
    Ok(record)
}

pub fn used_ports(layout: &Layout, except_id: &str) -> Vec<u32> {
    list(layout)
        .unwrap_or_default()
        .into_iter()
        .filter(|record| record.id != except_id)
        .filter_map(|record| crate::properties::read_port(&record.root))
        .collect()
}

fn load(path: &Path) -> PanelResult<Vec<ServerRecord>> {
    let catalog = read_catalog(path)?;
    catalog.servers.iter().map(read_or_stub).collect()
}

fn read_or_stub(entry: &CatalogEntry) -> PanelResult<ServerRecord> {
    let path = entry.root.join("server.json");
    if path.exists() {
        let text = std::fs::read_to_string(&path)?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| PanelError::invalid(format!("{} non valido: {error}", path.display())))?;
        let (value, migrated) = migrate_record(value, &entry.root);
        let mut record: ServerRecord = serde_json::from_value(value)
            .map_err(|error| PanelError::invalid(format!("{} non valido: {error}", path.display())))?;
        record.root = entry.root.clone();
        if migrated {
            write_json(&path, &record)?;
        }
        return Ok(record);
    }
    let name = entry.root.file_name().and_then(|name| name.to_str()).unwrap_or("Server").to_string();
    Ok(ServerRecord::new(entry.id.clone(), name, entry.root.clone()))
}

/// Schema 1 (Paper only) stored `paper_version` and `jar_path`; schema 2 adds the provider,
/// the build and a launch spec.
pub fn migrate_record(mut value: serde_json::Value, root: &Path) -> (serde_json::Value, bool) {
    let Some(object) = value.as_object_mut() else {
        return (value, false);
    };
    let version = object.get("schema_version").and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;
    if version >= crate::RECORD_SCHEMA {
        return (value, false);
    }
    let paper_version = object.remove("paper_version").filter(|value| !value.is_null());
    let jar = object
        .remove("jar_path")
        .and_then(|value| value.as_str().map(std::path::PathBuf::from));
    let (detected, detected_version) = crate::providers::detect(root, jar.as_deref());
    let provider = if detected == crate::ProviderKind::Custom && paper_version.is_some() {
        crate::ProviderKind::Paper
    } else {
        detected
    };
    object.insert("provider".into(), serde_json::to_value(provider).unwrap_or_default());
    let mc_version = paper_version.or_else(|| detected_version.map(serde_json::Value::from)).unwrap_or(serde_json::Value::Null);
    object.insert("mc_version".into(), mc_version);
    let launch = jar.map(|path| crate::LaunchSpec::Jar { path }).unwrap_or_default();
    object.insert("launch".into(), serde_json::to_value(launch).unwrap_or_default());
    object.insert("schema_version".into(), serde_json::Value::from(crate::RECORD_SCHEMA));
    (value, true)
}

fn read_catalog(path: &Path) -> PanelResult<CatalogFile> {
    if !path.exists() {
        return Ok(CatalogFile::default());
    }
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| PanelError::invalid(format!("catalog.json non valido: {error}")))
}

/// Writes through a temporary file so a crash never leaves a truncated JSON behind.
fn write_json<T: Serialize>(path: &Path, value: &T) -> PanelResult<()> {
    if let Some(parent) = path.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let text = serde_json::to_string_pretty(value)?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_lists_a_server() {
        let layout = Layout::at(std::env::temp_dir().join(format!("voxel-catalog-{}", uuid::Uuid::new_v4())));
        let root = layout.root.join("srv");
        let mut record = ServerRecord::new("abc".into(), "Survival".into(), root.clone());
        record.mc_version = Some("1.21.1".into());
        save(&layout, &record).unwrap();
        let listed = list(&layout).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Survival");
        let again = save(&layout, &record);
        assert!(again.is_ok());
        let error = get(&layout, "missing").unwrap_err();
        assert_eq!(error.code, crate::ErrorCode::NotFound);
        remove(&layout, "abc").unwrap();
        assert!(list(&layout).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&layout.root);
    }

    #[test]
    fn migrates_schema_one_records() {
        let root = std::path::Path::new("srv");
        let old = serde_json::json!({
            "id": "abc", "name": "Old", "root": "srv", "paper_version": "1.21.1", "java_major": 21,
            "java_home": null, "jar_path": "srv/paper-1.21.1-10.jar", "ram_min": "2G", "ram_max": "4G",
            "jvm_flags": [], "eula_accepted": true, "created_unix": 1
        });
        let (value, migrated) = migrate_record(old, root);
        assert!(migrated);
        let record: ServerRecord = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(record.provider, crate::ProviderKind::Paper);
        assert_eq!(record.mc_version.as_deref(), Some("1.21.1"));
        assert_eq!(record.launch, crate::LaunchSpec::Jar { path: "srv/paper-1.21.1-10.jar".into() });
        assert_eq!(record.schema_version, crate::RECORD_SCHEMA);
        assert!(!migrate_record(value, root).1);
    }

    #[test]
    fn removes_servers() {
        let layout = Layout::at(std::env::temp_dir().join(format!("voxel-catalog-{}", uuid::Uuid::new_v4())));
        let record = ServerRecord::new("abc".into(), "Survival".into(), layout.root.join("srv"));
        save(&layout, &record).unwrap();
        remove(&layout, "abc").unwrap();
        assert!(list(&layout).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&layout.root);
    }
}
