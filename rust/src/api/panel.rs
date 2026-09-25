// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use crate::frb_generated::StreamSink;
use crate::install::{self, AutoRequest, ManualRequest};
use crate::paths::Layout;
use crate::process::{self, RunStatus, RuntimeSnapshot};
use crate::{PanelError, PanelResult};

use super::progress::report;
use super::types::*;

fn to_status(status: RunStatus) -> ServerStatus {
    match status {
        RunStatus::Stopped => ServerStatus::Stopped,
        RunStatus::Starting => ServerStatus::Starting,
        RunStatus::Running => ServerStatus::Running,
        RunStatus::Stopping => ServerStatus::Stopping,
    }
}

fn to_runtime(snapshot: RuntimeSnapshot) -> ServerRuntime {
    ServerRuntime {
        server_id: snapshot.id,
        status: to_status(snapshot.status),
        pid: snapshot.pid,
        started_unix: snapshot.started_unix,
        players: snapshot.players,
        cpu_percent: snapshot.cpu_percent,
        memory_bytes: snapshot.memory_bytes as i64,
        last_exit_code: snapshot.last_exit_code,
        crashed: snapshot.crashed,
    }
}

pub async fn list_servers() -> PanelResult<Vec<ServerSummary>> {
    let layout = Layout::app();
    let records = crate::catalog::refresh(&layout)?;
    Ok(records
        .into_iter()
        .map(|record| {
            let settings = crate::properties::read_settings(&record.root);
            ServerSummary {
                id: record.id,
                name: record.name,
                root: record.root.to_string_lossy().to_string(),
                paper_version: record.paper_version,
                java_major: record.java_major,
                ram_min: record.ram_min,
                ram_max: record.ram_max,
                port: settings.port,
                max_players: settings.max_players,
            }
        })
        .collect())
}

pub async fn get_server(id: String) -> PanelResult<ServerDetails> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let settings = crate::properties::read_settings(&record.root);
    Ok(ServerDetails {
        java_home: record
            .java_home
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        jar_path: record
            .jar_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        id: record.id,
        name: record.name,
        root: record.root.to_string_lossy().to_string(),
        paper_version: record.paper_version,
        java_major: record.java_major,
        ram_min: record.ram_min,
        ram_max: record.ram_max,
        jvm_flags: record.jvm_flags,
        eula_accepted: record.eula_accepted,
        created_unix: record.created_unix,
        port: settings.port,
        max_players: settings.max_players,
    })
}

/// Current runtime state of every server that has been started in this session.
pub async fn runtime_snapshots() -> Vec<ServerRuntime> {
    process::all_snapshots().into_iter().map(to_runtime).collect()
}

/// Streams runtime changes: current snapshots first, then every status, player or stats update.
pub async fn watch_events(sink: StreamSink<ServerRuntime>) -> PanelResult<()> {
    let mut receiver = process::subscribe_events();
    for snapshot in process::all_snapshots() {
        if sink.add(to_runtime(snapshot)).is_err() {
            return Ok(());
        }
    }
    loop {
        match receiver.recv().await {
            Ok(snapshot) => {
                if sink.add(to_runtime(snapshot)).is_err() {
                    return Ok(());
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

#[flutter_rust_bridge::frb(sync)]
pub fn any_server_running() -> bool {
    process::any_running()
}

#[flutter_rust_bridge::frb(sync)]
pub fn app_paths() -> AppPaths {
    let layout = Layout::app();
    let text = |path: std::path::PathBuf| {
        let _ = std::fs::create_dir_all(&path);
        path.to_string_lossy().to_string()
    };
    AppPaths {
        servers: text(layout.servers()),
        runtimes: text(layout.runtimes()),
        backups: text(layout.backups_root()),
        data: text(layout.root),
    }
}

#[flutter_rust_bridge::frb(sync)]
pub fn ram_presets() -> Vec<RamChoice> {
    crate::ram::PRESETS
        .iter()
        .map(|(megabytes, label)| RamChoice {
            megabytes: *megabytes,
            label: (*label).to_string(),
            value: crate::ram::ram_value(*megabytes),
        })
        .collect()
}

#[flutter_rust_bridge::frb(sync)]
pub fn suggest_ram() -> RamSuggestion {
    let (ram_min, ram_max, total) = crate::ram::suggest();
    RamSuggestion {
        ram_min,
        ram_max,
        total,
    }
}

#[flutter_rust_bridge::frb(sync)]
pub fn jvm_flag_choices() -> Vec<JvmFlagChoice> {
    crate::jvm::JVM_FLAG_CHOICES
        .iter()
        .map(|flag| JvmFlagChoice {
            recommended: crate::jvm::DEFAULT_JVM_FLAGS.contains(flag),
            flag: (*flag).to_string(),
        })
        .collect()
}

pub async fn list_runtimes() -> PanelResult<Vec<JavaRuntimeInfo>> {
    Ok(crate::java_runtime::collect(&Layout::app())
        .into_iter()
        .map(|runtime| JavaRuntimeInfo {
            major: runtime.major.unwrap_or(0),
            name: runtime.name,
            path: runtime.home.to_string_lossy().to_string(),
        })
        .collect())
}

pub async fn list_java_releases() -> PanelResult<Vec<JavaReleaseInfo>> {
    let releases = crate::java_runtime::list_releases().await?;
    Ok(releases
        .into_iter()
        .map(|release| JavaReleaseInfo {
            major: release.major,
            lts: release.lts,
        })
        .collect())
}

pub async fn list_paper_versions() -> PanelResult<Vec<String>> {
    crate::paper::fetch_versions().await
}

pub async fn preview_import(path: String) -> PanelResult<ImportPreview> {
    let root = std::path::PathBuf::from(&path);
    if !root.is_dir() {
        return Err(PanelError::not_found("Cartella non trovata"));
    }
    let detected = crate::scan::detect(&root);
    Ok(ImportPreview {
        name: root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Server")
            .to_string(),
        root: path,
        paper_version: detected.paper_version.unwrap_or_default(),
        java_major: detected.java_major.unwrap_or(0),
        java_home: detected
            .java_home
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default(),
        jar_path: detected
            .jar_path
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default(),
        ram_min: detected.ram_min.unwrap_or_else(|| "2G".into()),
        ram_max: detected.ram_max.unwrap_or_else(|| "4G".into()),
        jvm_flags: detected.jvm_flags,
        plugin_count: detected.plugin_count,
        world_count: detected.world_count,
        has_eula: detected.has_eula,
    })
}

pub async fn import_server(path: String, name: String, accept_eula: bool) -> PanelResult<String> {
    install::import_folder(&Layout::app(), std::path::Path::new(&path), &name, accept_eula)
}

pub async fn accept_server_eula(id: String) -> PanelResult<()> {
    install::accept_eula(&Layout::app(), &id)
}

pub async fn update_runtime_config(
    id: String,
    java_home: String,
    ram_min: String,
    ram_max: String,
    jvm_flags: Vec<String>,
) -> PanelResult<()> {
    install::update_runtime(&Layout::app(), &id, &java_home, &ram_min, &ram_max, &jvm_flags)
}

pub async fn rename_server(id: String, name: String) -> PanelResult<()> {
    install::rename(&Layout::app(), &id, &name)
}

pub async fn delete_server(id: String, delete_files: bool, delete_backups: bool) -> PanelResult<()> {
    install::delete_server(&Layout::app(), &id, delete_files, delete_backups).await
}

pub async fn start_server(id: String) -> PanelResult<()> {
    install::launch(&Layout::app(), &id).await
}

pub async fn stop_server(id: String) -> PanelResult<()> {
    process::stop(&id, process::DEFAULT_STOP_TIMEOUT).await
}

pub async fn restart_server(id: String) -> PanelResult<()> {
    if process::is_running(&id) {
        process::stop(&id, process::DEFAULT_STOP_TIMEOUT).await?;
    }
    install::launch(&Layout::app(), &id).await
}

pub async fn send_command(id: String, command: String) -> PanelResult<()> {
    process::send_command(&id, &command).await
}

pub async fn shutdown_all() -> PanelResult<()> {
    process::shutdown_all(process::DEFAULT_STOP_TIMEOUT).await;
    Ok(())
}

pub async fn install_auto(request: AutoInstallRequest, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    report(sink, "Fatto", created_message, |tx| async move {
        install::install_auto(
            &Layout::app(),
            AutoRequest {
                name: request.name,
                root: request.root,
                paper_version: request.paper_version,
                accept_eula: request.accept_eula,
                ram_min: request.ram_min,
                ram_max: request.ram_max,
            },
            &tx,
        )
        .await
    })
    .await
}

pub async fn install_manual(request: ManualInstallRequest, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    let java_major = (request.java_major != 0).then_some(request.java_major);
    report(sink, "Fatto", created_message, |tx| async move {
        install::install_manual(
            &Layout::app(),
            ManualRequest {
                name: request.name,
                root: request.root,
                paper_version: empty_to_none(request.paper_version),
                jar_path: empty_to_none(request.jar_path),
                java_major,
                java_home: empty_to_none(request.java_home),
                ram_min: request.ram_min,
                ram_max: request.ram_max,
                jvm_flags: request.jvm_flags,
                accept_eula: request.accept_eula,
            },
            &tx,
        )
        .await
    })
    .await
}

pub async fn install_java(major: u32, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    report(
        sink,
        "Java",
        |path: &String| (format!("Java installato in {path}"), None),
        |tx| async move { install::install_java(&Layout::app(), major, &tx).await },
    )
    .await
}

pub async fn watch_console(id: String, sink: StreamSink<String>) -> PanelResult<()> {
    let _ = crate::catalog::get(&Layout::app(), &id)?;
    let (history, mut receiver) = process::subscribe(&id);
    for line in history {
        if sink.add(line).is_err() {
            return Ok(());
        }
    }
    loop {
        match receiver.recv().await {
            Ok(line) => {
                if sink.add(line).is_err() {
                    return Ok(());
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

#[allow(clippy::ptr_arg)]
fn created_message(id: &String) -> (String, Option<String>) {
    ("Operazione completata.".into(), Some(id.to_string()))
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}
