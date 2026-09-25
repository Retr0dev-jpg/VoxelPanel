// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::time::SystemTime;

use crate::frb_generated::StreamSink;
use crate::paths::Layout;
use crate::process;
use crate::{ErrorCode, PanelError, PanelResult};

use super::progress::report;
use super::types::*;

pub async fn list_properties(id: String) -> PanelResult<Vec<PropertyEntry>> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    Ok(crate::properties::load_entries(&record.root)?
        .into_iter()
        .map(|(key, value)| PropertyEntry { key, value })
        .collect())
}

pub async fn save_properties(id: String, entries: Vec<PropertyEntry>) -> PanelResult<()> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let pairs: Vec<(String, String)> = entries
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect();
    if let Some((_, port)) = pairs.iter().find(|(key, _)| key == "server-port") {
        if let Ok(port) = port.parse::<u32>() {
            if crate::catalog::used_ports(&Layout::app(), &id).contains(&port) {
                return Err(PanelError::new(
                    ErrorCode::PortInUse,
                    format!("La porta {port} è già usata da un altro server."),
                ));
            }
        }
    }
    crate::properties::write_entries(&record.root, &pairs)
}

pub async fn list_addons(id: String, kind: AddonKind) -> PanelResult<Vec<AddonInfo>> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    Ok(crate::addons::list(&record.root, kind)
        .into_iter()
        .map(|addon| AddonInfo {
            file_name: addon.file_name,
            enabled: addon.enabled,
            size_bytes: addon.size_bytes as i64,
        })
        .collect())
}

pub async fn set_addon_enabled(id: String, kind: AddonKind, file_name: String, enabled: bool) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::addons::set_enabled(&record.root, kind, &file_name, enabled)
}

pub async fn delete_addon(id: String, kind: AddonKind, file_name: String) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::addons::delete(&record.root, kind, &file_name)
}

pub async fn install_addon_file(id: String, kind: AddonKind, source_path: String) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::addons::install_file(&record.root, kind, std::path::Path::new(&source_path))
}

/// Searches Modrinth for plugins or mods compatible with this server's software and version.
pub async fn search_modrinth(id: String, kind: AddonKind, query: String) -> PanelResult<Vec<ModrinthProject>> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    Ok(crate::addons::search(&record, kind, &query)
        .await?
        .into_iter()
        .map(|hit| ModrinthProject {
            project_id: hit.project_id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            downloads: hit.downloads,
        })
        .collect())
}

pub async fn install_modrinth_project(id: String, kind: AddonKind, project_id: String, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    report(
        sink,
        "Modrinth",
        move |_: &()| ("Installazione completata.".into(), Some(id)),
        |tx| async move { crate::addons::install_project(&record, kind, &project_id, &tx).await },
    )
    .await
}

pub async fn list_worlds(id: String) -> PanelResult<Vec<WorldInfo>> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let active = crate::properties::read_settings(&record.root).level_name;
    Ok(crate::worlds::list(&record.root, Some(&active))
        .into_iter()
        .map(|world| WorldInfo {
            name: world.name,
            dimension: world.dimension,
            group: world.group,
            path: world.path.to_string_lossy().to_string(),
            size_bytes: world.size_bytes as i64,
            modified_ms: world.modified_ms,
            active: world.active,
        })
        .collect())
}

pub async fn set_active_world(id: String, name: String) -> PanelResult<()> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let _ = crate::worlds::resolve(&record.root, &name)?;
    let mut settings = crate::properties::read_settings(&record.root);
    settings.level_name = name;
    crate::install::save_settings(&Layout::app(), &id, &settings)
}

pub async fn delete_world(id: String, name: String) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let path = crate::worlds::resolve(&record.root, &name)?;
    tokio::fs::remove_dir_all(path).await?;
    Ok(())
}

/// Imports a world from a folder or a `.zip` under the given name.
pub async fn import_world(id: String, source_path: String, name: String) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let root = crate::catalog::get(&Layout::app(), &id)?.root;
    tokio::task::spawn_blocking(move || crate::worlds::import(&root, std::path::Path::new(&source_path), name.trim())).await?
}

/// Renames a world with its dimensions, keeping it active if it was.
pub async fn rename_world(id: String, name: String, new_name: String) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let new_name = new_name.trim().to_string();
    crate::worlds::rename(&record.root, &name, &new_name)?;
    let mut settings = crate::properties::read_settings(&record.root);
    if settings.level_name == name {
        settings.level_name = new_name;
        crate::install::save_settings(&Layout::app(), &id, &settings)?;
    }
    Ok(())
}

/// Deletes a world and its dimensions so the next start generates a new one, optionally with a new seed.
pub async fn reset_world(id: String, name: String, seed: String) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let root = record.root.clone();
    let group = name.clone();
    tokio::task::spawn_blocking(move || crate::worlds::reset(&root, &group)).await??;
    let mut settings = crate::properties::read_settings(&record.root);
    if settings.level_name == name {
        settings.level_seed = seed.trim().to_string();
        crate::install::save_settings(&Layout::app(), &id, &settings)?;
    }
    Ok(())
}

pub async fn backup_world(id: String, name: String, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    report(
        sink,
        "Backup",
        move |file_name: &String| (format!("Backup del mondo {file_name} creato."), Some(id)),
        |tx| async move {
            tx.emit("Backup", format!("Backup del mondo {name}..."), None);
            let directory = Layout::app().backups(&record.id);
            let file_name = format!("world-{name}-{}.zip", crate::paths::unix_now());
            let destination = directory.join(&file_name);
            let root = record.root.clone();
            tokio::task::spawn_blocking(move || crate::worlds::backup(&root, &name, &destination)).await??;
            Ok(file_name)
        },
    )
    .await
}

pub async fn open_in_explorer(path: String) -> PanelResult<()> {
    let path = std::path::PathBuf::from(&path);
    if !path.is_dir() {
        return Err(PanelError::not_found("Cartella non trovata"));
    }
    crate::platform::open_directory(&path)
}

pub async fn list_backups(id: String) -> PanelResult<Vec<BackupInfo>> {
    let _ = crate::catalog::get(&Layout::app(), &id)?;
    let directory = Layout::app().backups(&id);
    let mut infos = Vec::new();
    for path in crate::backup::backup_files(&directory)? {
        let meta = std::fs::metadata(&path)?;
        let created_ms = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|value| value.as_millis() as i64)
            .unwrap_or(0);
        infos.push(BackupInfo {
            file_name: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("backup.zip")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            size_bytes: meta.len() as i64,
            created_ms,
        });
    }
    Ok(infos)
}

pub async fn create_backup(id: String, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    report(
        sink,
        "Backup",
        move |file_name: &String| (format!("Backup {file_name} creato."), Some(id)),
        |tx| async move {
            tx.emit("Backup", "Creazione backup...", None);
            let directory = Layout::app().backups(&record.id);
            crate::paths::ensure_dir(&directory)?;
            let file_name = format!("{}.zip", crate::paths::unix_now());
            let destination = directory.join(&file_name);
            let manifest = crate::backup::BackupManifest::of(&record);
            let root = record.root.clone();
            let options = crate::backup::BackupOptions::from_settings();
            tokio::task::spawn_blocking(move || crate::backup::create_zip(&root, &manifest, &destination, &options))
                .await??;
            let retention = crate::launcher_settings::current().backup.retention;
            let removed = crate::backup::prune(&directory, retention)?;
            if removed > 0 {
                tx.emit("Backup", format!("Rimossi {removed} backup più vecchi."), None);
            }
            Ok(file_name)
        },
    )
    .await
}

pub async fn restore_backup(id: String, file_name: String, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let archive = backup_path(&id, &file_name)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    report(
        sink,
        "Backup",
        move |_: &()| ("Backup ripristinato.".into(), Some(id)),
        |tx| async move {
            tx.emit("Backup", "Creo una copia di sicurezza prima del ripristino...", None);
            crate::backup::safety_backup(&Layout::app(), &record, "pre-restore").await?;
            tx.emit("Backup", "Ripristino in corso...", None);
            let root = record.root.clone();
            tokio::task::spawn_blocking(move || crate::backup::restore_zip(&root, &archive)).await??;
            Ok(())
        },
    )
    .await
}

pub async fn delete_backup(id: String, file_name: String) -> PanelResult<()> {
    let path = backup_path(&id, &file_name)?;
    std::fs::remove_file(path)?;
    Ok(())
}

fn backup_path(id: &str, file_name: &str) -> PanelResult<std::path::PathBuf> {
    if file_name.contains(['\\', '/', ':']) || file_name.contains("..") {
        return Err(PanelError::invalid("Nome backup non valido"));
    }
    let path = Layout::app().backups(id).join(file_name);
    if !path.is_file() {
        return Err(PanelError::not_found("Backup non trovato"));
    }
    Ok(path)
}


#[cfg(test)]
mod tests {
    use super::open_in_explorer;

    #[tokio::test]
    async fn missing_directory_does_not_open_explorer() {
        let missing = std::env::temp_dir().join(format!("voxel-missing-{}", uuid::Uuid::new_v4()));
        let error = open_in_explorer(missing.to_string_lossy().to_string())
            .await
            .expect_err("missing path");
        assert_eq!(error.message, "Cartella non trovata");
        assert_eq!(error.code, crate::ErrorCode::NotFound);
    }

    /// Opens a real Explorer window. Ignored so `cargo test` does not steal focus.
    #[tokio::test]
    #[ignore]
    async fn opens_directory_whose_path_contains_spaces() {
        let directory = std::env::temp_dir().join(format!("voxel panel {}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        open_in_explorer(directory.to_string_lossy().to_string())
            .await
            .unwrap();
        let _ = std::fs::remove_dir_all(&directory);
    }
}
