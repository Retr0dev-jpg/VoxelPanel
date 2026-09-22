use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::paths::Layout;
use crate::ServerRecord;

#[derive(Debug, Default, Serialize, Deserialize)]
struct CatalogFile {
    servers: Vec<CatalogEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CatalogEntry {
    id: String,
    root: PathBuf,
}

fn io_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

pub fn list(layout: &Layout) -> Result<Vec<ServerRecord>, String> {
    let _guard = io_lock().lock().unwrap_or_else(|error| error.into_inner());
    let catalog = read_catalog(&layout.catalog_file())?;
    let mut records = Vec::new();
    for entry in catalog.servers {
        records.push(read_or_stub(&entry)?);
    }
    Ok(records)
}

pub fn get(layout: &Layout, id: &str) -> Result<ServerRecord, String> {
    list(layout)?
        .into_iter()
        .find(|record| record.id == id)
        .ok_or_else(|| "Server non trovato".to_string())
}

pub fn save(layout: &Layout, record: &ServerRecord) -> Result<(), String> {
    let _guard = io_lock().lock().unwrap_or_else(|error| error.into_inner());
    crate::paths::ensure_dir(&record.root)?;
    write_json(&record.root.join("server.json"), record)?;
    let mut catalog = read_catalog(&layout.catalog_file())?;
    if let Some(entry) = catalog.servers.iter_mut().find(|entry| entry.id == record.id) {
        entry.root = record.root.clone();
    } else {
        if catalog
            .servers
            .iter()
            .any(|entry| same_path(&entry.root, &record.root))
        {
            return Err("Questa cartella è già importata.".into());
        }
        catalog.servers.push(CatalogEntry {
            id: record.id.clone(),
            root: record.root.clone(),
        });
    }
    crate::paths::ensure_dir(&layout.root)?;
    write_json(&layout.catalog_file(), &catalog)
}

pub fn remove(layout: &Layout, id: &str) -> Result<Option<ServerRecord>, String> {
    let _guard = io_lock().lock().unwrap_or_else(|error| error.into_inner());
    let mut catalog = read_catalog(&layout.catalog_file())?;
    let Some(index) = catalog.servers.iter().position(|entry| entry.id == id) else {
        return Err("Server non trovato".into());
    };
    let entry = catalog.servers.remove(index);
    let record = read_or_stub(&entry).ok();
    write_json(&layout.catalog_file(), &catalog)?;
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

fn read_or_stub(entry: &CatalogEntry) -> Result<ServerRecord, String> {
    let path = entry.root.join("server.json");
    if path.exists() {
        let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let mut record: ServerRecord = serde_json::from_str(&text).map_err(|error| error.to_string())?;
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

fn read_catalog(path: &Path) -> Result<CatalogFile, String> {
    if !path.exists() {
        return Ok(CatalogFile::default());
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| format!("catalog.json non valido: {error}"))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    std::fs::write(path, text).map_err(|error| error.to_string())
}

fn same_path(left: &Path, right: &Path) -> bool {
    normalize(left) == normalize(right)
}

fn normalize(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase()
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
        let _ = std::fs::remove_dir_all(&layout.root);
    }
}
