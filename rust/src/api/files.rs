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

pub async fn list_plugins(id: String) -> PanelResult<Vec<PluginInfo>> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    Ok(crate::plugins::list(&record.root)
        .into_iter()
        .map(|plugin| PluginInfo {
            file_name: plugin.file_name,
            enabled: plugin.enabled,
            size_bytes: plugin.size_bytes as i64,
        })
        .collect())
}

pub async fn set_plugin_enabled(id: String, file_name: String, enabled: bool) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::plugins::set_enabled(&record.root, &file_name, enabled)
}

pub async fn delete_plugin(id: String, file_name: String) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::plugins::delete(&record.root, &file_name)
}

pub async fn install_plugin_file(id: String, source_path: String) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::plugins::install_file(&record.root, std::path::Path::new(&source_path))
}

pub async fn search_modrinth(query: String) -> PanelResult<Vec<ModrinthProject>> {
    let hits = crate::plugins::search(&query).await?;
    Ok(hits
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

pub async fn install_modrinth_project(
    id: String,
    project_id: String,
    sink: StreamSink<ProgressEvent>,
) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let version = record
        .paper_version
        .clone()
        .ok_or("Versione Paper sconosciuta: impossibile filtrare i plugin")?;
    report(
        sink,
        "Plugin",
        move |_: &()| ("Plugin installato.".into(), Some(id)),
        |tx| async move { crate::plugins::install_project(&record.root, &project_id, &version, &tx).await },
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

pub async fn open_in_explorer(path: String) -> PanelResult<()> {
    let path = std::path::PathBuf::from(&path);
    if !path.is_dir() {
        return Err(PanelError::not_found("Cartella non trovata"));
    }
    // explorer.exe parses its own command line. Rust quotes arguments that contain
    // spaces (typical under C:\Users\Marco Simone\...), and a quoted directory makes
    // explorer open Documents instead of that folder. ShellExecuteW passes the path
    // as a wide string and skips that parser.
    open_directory(&path)
}

#[cfg(windows)]
fn open_directory(path: &std::path::Path) -> PanelResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let mut file: Vec<u16> = path.as_os_str().encode_wide().collect();
    file.push(0);
    let operation: Vec<u16> = "explore".encode_utf16().chain(std::iter::once(0)).collect();
    let code = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // Values <= 32 are Win32 error codes, not an instance handle.
    if (code.0 as isize) <= 32 {
        return Err(PanelError::io(format!(
            "Impossibile aprire la cartella (codice {})",
            code.0 as isize
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_directory(path: &std::path::Path) -> PanelResult<()> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|error| PanelError::io(format!("Impossibile aprire la cartella: {error}")))?;
    Ok(())
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
            let manifest = manifest_of(&record);
            let root = record.root.clone();
            tokio::task::spawn_blocking(move || crate::backup::create_zip(&root, &manifest, &destination))
                .await??;
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
            let safety = Layout::app()
                .backups(&record.id)
                .join(format!("pre-restore-{}.zip", crate::paths::unix_now()));
            let manifest = manifest_of(&record);
            let root = record.root.clone();
            tokio::task::spawn_blocking(move || crate::backup::create_zip(&root, &manifest, &safety)).await??;
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

fn manifest_of(record: &crate::ServerRecord) -> crate::backup::BackupManifest {
    crate::backup::BackupManifest {
        paper_version: record.paper_version.clone(),
        server_name: record.name.clone(),
        created_unix: crate::paths::unix_now(),
    }
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
