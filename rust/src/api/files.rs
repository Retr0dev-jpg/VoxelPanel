// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::sync::mpsc;
use std::time::SystemTime;

use crate::frb_generated::StreamSink;
use crate::paths::Layout;
use crate::process;

use super::panel::status_of;
use super::types::*;

pub async fn list_properties(id: String) -> Result<Vec<PropertyEntry>, String> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    Ok(crate::properties::load_entries(&record.root)?
        .into_iter()
        .map(|(key, value)| PropertyEntry { key, value })
        .collect())
}

pub async fn save_properties(id: String, entries: Vec<PropertyEntry>) -> Result<(), String> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let pairs: Vec<(String, String)> = entries
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect();
    if let Some((_, port)) = pairs.iter().find(|(key, _)| key == "server-port") {
        if let Ok(port) = port.parse::<u32>() {
            let used = crate::catalog::used_ports(&Layout::app(), &id);
            if used.contains(&port) {
                return Err(format!("La porta {port} è già usata da un altro server."));
            }
        }
    }
    crate::properties::write_entries(&record.root, &pairs)
}

pub async fn get_settings(id: String) -> Result<ServerSettings, String> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let settings = crate::properties::read_settings(&record.root);
    Ok(to_settings(settings))
}

pub async fn save_settings(id: String, settings: ServerSettings) -> Result<(), String> {
    let parsed = from_settings(settings);
    crate::install::save_settings(&Layout::app(), &id, &parsed)
}

pub async fn list_plugins(id: String) -> Result<Vec<PluginInfo>, String> {
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

pub async fn set_plugin_enabled(id: String, file_name: String, enabled: bool) -> Result<(), String> {
    crate::plugins::require_stopped(process::pid_of(&id).is_some())?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::plugins::set_enabled(&record.root, &file_name, enabled)
}

pub async fn delete_plugin(id: String, file_name: String) -> Result<(), String> {
    crate::plugins::require_stopped(process::pid_of(&id).is_some())?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::plugins::delete(&record.root, &file_name)
}

pub async fn install_plugin_file(id: String, source_path: String) -> Result<(), String> {
    crate::plugins::require_stopped(process::pid_of(&id).is_some())?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    crate::plugins::install_file(&record.root, std::path::Path::new(&source_path))
}

pub async fn search_modrinth(query: String) -> Result<Vec<ModrinthProject>, String> {
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
) -> Result<(), String> {
    crate::plugins::require_stopped(process::pid_of(&id).is_some())?;
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let version = record
        .paper_version
        .clone()
        .ok_or("Versione Paper sconosciuta: impossibile filtrare i plugin")?;
    let (tx, rx) = mpsc::channel();
    let result = crate::plugins::install_project(&record.root, &project_id, &version, &tx).await;
    drop(tx);
    while let Ok(progress) = rx.try_recv() {
        let _ = sink.add(ProgressEvent {
            stage: progress.stage,
            message: progress.message,
            fraction: progress.fraction,
            done: false,
            error: None,
            server_id: None,
        });
    }
    match result {
        Ok(()) => {
            let _ = sink.add(ProgressEvent {
                stage: "Plugin".into(),
                message: "Plugin installato.".into(),
                fraction: Some(1.0),
                done: true,
                error: None,
                server_id: Some(id),
            });
            Ok(())
        }
        Err(error) => {
            let _ = sink.add(ProgressEvent {
                stage: "Errore".into(),
                message: error.clone(),
                fraction: None,
                done: true,
                error: Some(error.clone()),
                server_id: None,
            });
            Err(error)
        }
    }
}

pub async fn list_worlds(id: String) -> Result<Vec<WorldInfo>, String> {
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

pub async fn set_active_world(id: String, name: String) -> Result<(), String> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let _ = crate::worlds::resolve(&record.root, &name)?;
    let mut settings = crate::properties::read_settings(&record.root);
    settings.level_name = name;
    crate::install::save_settings(&Layout::app(), &id, &settings)
}

pub async fn delete_world(id: String, name: String) -> Result<(), String> {
    if process::pid_of(&id).is_some() {
        return Err("Ferma il server prima di eliminare un mondo.".into());
    }
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let path = crate::worlds::resolve(&record.root, &name)?;
    tokio::fs::remove_dir_all(path)
        .await
        .map_err(|error| error.to_string())
}

pub async fn open_in_explorer(path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&path);
    if !path.is_dir() {
        return Err("Cartella non trovata".into());
    }
    // explorer.exe parses its own command line. Rust quotes arguments that contain
    // spaces (typical under C:\Users\Marco Simone\...), and a quoted directory makes
    // explorer open Documents instead of that folder. ShellExecuteW passes the path
    // as a wide string and skips that parser.
    open_directory(&path)
}

#[cfg(windows)]
fn open_directory(path: &std::path::Path) -> Result<(), String> {
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
        return Err(format!(
            "Impossibile aprire la cartella (codice {})",
            code.0 as isize
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_directory(path: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|error| format!("Impossibile aprire la cartella: {error}"))?;
    Ok(())
}

pub async fn list_backups(id: String) -> Result<Vec<BackupInfo>, String> {
    let _ = crate::catalog::get(&Layout::app(), &id)?;
    let directory = Layout::app().backups(&id);
    let files = crate::backup::backup_files(&directory)?;
    let mut infos = Vec::new();
    for path in files {
        let meta = std::fs::metadata(&path).map_err(|error| error.to_string())?;
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

pub async fn create_backup(id: String, sink: StreamSink<ProgressEvent>) -> Result<(), String> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let _ = sink.add(ProgressEvent {
        stage: "Backup".into(),
        message: "Creazione backup...".into(),
        fraction: None,
        done: false,
        error: None,
        server_id: None,
    });
    let directory = Layout::app().backups(&id);
    crate::paths::ensure_dir(&directory)?;
    let file_name = format!("{}.zip", crate::paths::unix_now());
    let destination = directory.join(&file_name);
    let manifest = crate::backup::BackupManifest {
        paper_version: record.paper_version.clone(),
        server_name: record.name.clone(),
        created_unix: crate::paths::unix_now(),
    };
    let root = record.root.clone();
    let result = tokio::task::spawn_blocking(move || crate::backup::create_zip(&root, &manifest, &destination))
        .await
        .map_err(|error| error.to_string())?;
    match result {
        Ok(()) => {
            let _ = sink.add(ProgressEvent {
                stage: "Backup".into(),
                message: format!("Backup {file_name} creato."),
                fraction: Some(1.0),
                done: true,
                error: None,
                server_id: Some(id),
            });
            Ok(())
        }
        Err(error) => {
            let _ = sink.add(ProgressEvent {
                stage: "Errore".into(),
                message: error.clone(),
                fraction: None,
                done: true,
                error: Some(error.clone()),
                server_id: None,
            });
            Err(error)
        }
    }
}

pub async fn restore_backup(id: String, file_name: String, sink: StreamSink<ProgressEvent>) -> Result<(), String> {
    if process::pid_of(&id).is_some() {
        return Err("Ferma il server prima di ripristinare un backup.".into());
    }
    if file_name.contains(['\\', '/', ':']) || file_name.contains("..") {
        return Err("Nome backup non valido".into());
    }
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let archive = Layout::app().backups(&id).join(&file_name);
    if !archive.is_file() {
        return Err("Backup non trovato".into());
    }
    let _ = sink.add(ProgressEvent {
        stage: "Backup".into(),
        message: "Creo una copia di sicurezza prima del ripristino...".into(),
        fraction: None,
        done: false,
        error: None,
        server_id: None,
    });
    let safety = Layout::app()
        .backups(&id)
        .join(format!("pre-restore-{}.zip", crate::paths::unix_now()));
    let manifest = crate::backup::BackupManifest {
        paper_version: record.paper_version.clone(),
        server_name: record.name.clone(),
        created_unix: crate::paths::unix_now(),
    };
    let root = record.root.clone();
    let safety_for_task = safety.clone();
    tokio::task::spawn_blocking(move || crate::backup::create_zip(&root, &manifest, &safety_for_task))
        .await
        .map_err(|error| error.to_string())??;
    let root = record.root.clone();
    let result = tokio::task::spawn_blocking(move || crate::backup::restore_zip(&root, &archive))
        .await
        .map_err(|error| error.to_string())?;
    match result {
        Ok(()) => {
            let _ = sink.add(ProgressEvent {
                stage: "Backup".into(),
                message: "Backup ripristinato.".into(),
                fraction: Some(1.0),
                done: true,
                error: None,
                server_id: Some(id),
            });
            Ok(())
        }
        Err(error) => {
            let _ = sink.add(ProgressEvent {
                stage: "Errore".into(),
                message: error.clone(),
                fraction: None,
                done: true,
                error: Some(error.clone()),
                server_id: None,
            });
            Err(error)
        }
    }
}

pub async fn delete_backup(id: String, file_name: String) -> Result<(), String> {
    if file_name.contains(['\\', '/', ':']) || file_name.contains("..") {
        return Err("Nome backup non valido".into());
    }
    let path = Layout::app().backups(&id).join(&file_name);
    if !path.is_file() {
        return Err("Backup non trovato".into());
    }
    std::fs::remove_file(path).map_err(|error| error.to_string())
}

fn to_settings(settings: crate::properties::Settings) -> ServerSettings {
    ServerSettings {
        motd: settings.motd,
        port: settings.port,
        max_players: settings.max_players,
        online_mode: settings.online_mode,
        difficulty: settings.difficulty,
        gamemode: settings.gamemode,
        view_distance: settings.view_distance,
        simulation_distance: settings.simulation_distance,
        white_list: settings.white_list,
        pvp: settings.pvp,
        spawn_protection: settings.spawn_protection,
        level_name: settings.level_name,
        level_seed: settings.level_seed,
    }
}

fn from_settings(settings: ServerSettings) -> crate::properties::Settings {
    crate::properties::Settings {
        motd: settings.motd,
        port: settings.port,
        max_players: settings.max_players,
        online_mode: settings.online_mode,
        difficulty: settings.difficulty,
        gamemode: settings.gamemode,
        view_distance: settings.view_distance,
        simulation_distance: settings.simulation_distance,
        white_list: settings.white_list,
        pvp: settings.pvp,
        spawn_protection: settings.spawn_protection,
        level_name: settings.level_name,
        level_seed: settings.level_seed,
    }
}

pub fn server_is_running(id: &str) -> bool {
    !matches!(status_of(id), ServerStatus::Stopped)
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
        assert_eq!(error, "Cartella non trovata");
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
