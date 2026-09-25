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
            .any(|entry| same_path(&entry.root, &record.root))
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
        let mut record: ServerRecord = serde_json::from_str(&text)
            .map_err(|error| PanelError::invalid(format!("{} non valido: {error}", path.display())))?;
        record.root = entry.root.clone();
        return Ok(record);
    }
    Ok(ServerRecord {
        id: entry.id.clone(),
        name: entry
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Server")
            .to_string(),
        root: entry.root.clone(),
        paper_version: None,
        java_major: None,
        java_home: None,
        jar_path: None,
        ram_min: "2G".into(),
        ram_max: "4G".into(),
        jvm_flags: Vec::new(),
        eula_accepted: false,
        created_unix: crate::paths::unix_now(),
    })
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

fn same_path(left: &Path, right: &Path) -> bool {
    normalize(left) == normalize(right)
}

fn normalize(path: &Path) -> String {
    let text = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_lists_a_server() {
        let layout = Layout {
            root: std::env::temp_dir().join(format!("voxel-catalog-{}", uuid::Uuid::new_v4())),
        };
        let root = layout.root.join("srv");
        let record = ServerRecord {
            id: "abc".into(),
            name: "Survival".into(),
            root: root.clone(),
            paper_version: Some("1.21.1".into()),
            java_major: Some(21),
            java_home: None,
            jar_path: None,
            ram_min: "2G".into(),
            ram_max: "4G".into(),
            jvm_flags: vec!["-XX:+UseG1GC".into()],
            eula_accepted: true,
            created_unix: 1,
        };
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
}
