// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use crate::paths::Layout;
use crate::{process, PanelError, PanelResult};

use super::types::*;

fn record(id: &str) -> PanelResult<crate::ServerRecord> {
    crate::catalog::get(&Layout::app(), id)
}

pub async fn get_server_config(id: String) -> PanelResult<ServerConfig> {
    let record = record(&id)?;
    Ok(ServerConfig {
        name: record.name.clone(),
        java_home: record.java_home.as_ref().map(|path| path.to_string_lossy().to_string()).unwrap_or_default(),
        ram_min: record.ram_min.clone(),
        ram_max: record.ram_max.clone(),
        jvm_flags: record.jvm_flags.clone(),
        stop_command: record.stop_command.clone().unwrap_or_default(),
        stop_timeout_secs: record.stop_timeout_secs.unwrap_or(0),
        autostart: record.autostart,
        auto_restart: record.auto_restart,
        manage_rcon: record.manage_rcon,
    })
}

/// Saves the per-server options. Runtime changes (Java, RAM, flags) need a stopped server.
pub async fn save_server_config(id: String, config: ServerConfig) -> PanelResult<()> {
    let layout = Layout::app();
    let mut record = record(&id)?;
    let runtime_changed = record.java_home.as_ref().map(|path| path.to_string_lossy().to_string()).unwrap_or_default() != config.java_home
        || record.ram_min != config.ram_min
        || record.ram_max != config.ram_max
        || record.jvm_flags != config.jvm_flags;
    if runtime_changed {
        crate::install::update_runtime(&layout, &id, &config.java_home, &config.ram_min, &config.ram_max, &config.jvm_flags)?;
        record = crate::catalog::get(&layout, &id)?;
    }
    let name = config.name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.contains(['\n', '\r', '/', '\\']) {
        return Err(PanelError::invalid("Nome server non valido"));
    }
    let stop_command = config.stop_command.trim();
    if stop_command.contains(['\n', '\r']) {
        return Err(PanelError::invalid("Comando di arresto non valido"));
    }
    if config.stop_timeout_secs != 0 && !(5..=3600).contains(&config.stop_timeout_secs) {
        return Err(PanelError::invalid("Il timeout di arresto deve essere tra 5 e 3600 secondi"));
    }
    record.name = name.to_string();
    record.stop_command = (!stop_command.is_empty()).then(|| stop_command.to_string());
    record.stop_timeout_secs = (config.stop_timeout_secs != 0).then_some(config.stop_timeout_secs);
    record.autostart = config.autostart;
    record.auto_restart = config.auto_restart;
    record.manage_rcon = config.manage_rcon;
    crate::catalog::save(&layout, &record)
}

/// Path of `server-icon.png`, empty when the server has none.
pub async fn server_icon_path(id: String) -> PanelResult<String> {
    let path = record(&id)?.root.join("server-icon.png");
    Ok(if path.is_file() { path.to_string_lossy().to_string() } else { String::new() })
}

/// Converts any common image to the 64×64 PNG Minecraft expects.
pub async fn set_server_icon(id: String, source_path: String) -> PanelResult<()> {
    let destination = record(&id)?.root.join("server-icon.png");
    tokio::task::spawn_blocking(move || -> PanelResult<()> {
        let image = image::open(&source_path).map_err(|error| PanelError::invalid(format!("Immagine non leggibile: {error}")))?;
        let icon = image.resize_to_fill(64, 64, image::imageops::FilterType::Lanczos3);
        icon.save_with_format(&destination, image::ImageFormat::Png).map_err(|error| PanelError::io(error.to_string()))?;
        Ok(())
    })
    .await?
}

pub async fn remove_server_icon(id: String) -> PanelResult<()> {
    let path = record(&id)?.root.join("server-icon.png");
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[flutter_rust_bridge::frb(sync)]
pub fn properties_schema() -> Vec<PropertySchema> {
    crate::properties_schema::schema()
}

pub async fn list_config_files(id: String) -> PanelResult<Vec<String>> {
    Ok(crate::server_files::config_files(&record(&id)?))
}

pub async fn list_directory(id: String, relative: String) -> PanelResult<Vec<FileEntry>> {
    crate::server_files::list(&record(&id)?.root, &relative)
}

pub async fn read_text_file(id: String, relative: String) -> PanelResult<String> {
    crate::server_files::read_text(&record(&id)?.root, &relative)
}

pub async fn write_text_file(id: String, relative: String, content: String) -> PanelResult<()> {
    let root = record(&id)?.root;
    if relative.trim().trim_start_matches('/') == "server.properties" {
        // The typed editor validates values; the raw editor must not bypass that.
        let entries = crate::properties::read_entries(&content);
        for (key, value) in &entries {
            crate::properties_schema::validate(key, value).map_err(PanelError::invalid)?;
        }
    }
    crate::server_files::write_text(&root, &relative, &content)
}

pub async fn delete_server_path(id: String, relative: String) -> PanelResult<()> {
    let root = record(&id)?.root;
    if process::is_running(&id) && ["world", "plugins", "mods", "libraries"].iter().any(|protected| relative.trim_start_matches('/').starts_with(protected)) {
        return Err(PanelError::running());
    }
    crate::server_files::delete(&root, &relative)
}

pub async fn create_server_directory(id: String, relative: String) -> PanelResult<()> {
    crate::server_files::create_directory(&record(&id)?.root, &relative)
}

pub async fn rename_server_path(id: String, from: String, to: String) -> PanelResult<()> {
    crate::server_files::rename(&record(&id)?.root, &from, &to)
}

pub async fn import_server_files(id: String, relative_dir: String, sources: Vec<String>) -> PanelResult<u32> {
    let root = record(&id)?.root;
    tokio::task::spawn_blocking(move || crate::server_files::import(&root, &relative_dir, &sources)).await?
}

pub async fn export_server_path(id: String, relative: String, destination_dir: String) -> PanelResult<String> {
    let root = record(&id)?.root;
    let target = tokio::task::spawn_blocking(move || crate::server_files::export(&root, &relative, std::path::Path::new(&destination_dir))).await??;
    Ok(target.to_string_lossy().to_string())
}

pub async fn list_logs(id: String) -> PanelResult<Vec<LogFileInfo>> {
    Ok(crate::server_logs::list(&record(&id)?.root))
}

pub async fn read_log(id: String, relative: String) -> PanelResult<String> {
    let root = record(&id)?.root;
    tokio::task::spawn_blocking(move || crate::server_logs::read(&root, &relative)).await?
}
