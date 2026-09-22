use std::sync::mpsc;

use crate::frb_generated::StreamSink;
use crate::install::{self, AutoRequest, ManualRequest};
use crate::paths::Layout;
use crate::process::{self, RunStatus};
use crate::Progress;

use super::types::*;

pub fn status_of(id: &str) -> ServerStatus {
    match process::status_of(id) {
        RunStatus::Stopped => ServerStatus::Stopped,
        RunStatus::Starting => ServerStatus::Starting,
        RunStatus::Running => ServerStatus::Running,
        RunStatus::Stopping => ServerStatus::Stopping,
    }
}

pub async fn list_servers() -> Result<Vec<ServerSummary>, String> {
    let layout = Layout::app();
    let records = crate::catalog::list(&layout)?;
    Ok(records
        .into_iter()
        .map(|record| {
            let id = record.id.clone();
            ServerSummary {
                port: crate::properties::read_port(&record.root).unwrap_or(25565),
                status: status_of(&id),
                pid: process::pid_of(&id),
                id,
                name: record.name,
                root: record.root.to_string_lossy().to_string(),
                paper_version: record.paper_version,
                java_major: record.java_major,
                ram_min: record.ram_min,
                ram_max: record.ram_max,
            }
        })
        .collect())
}

pub async fn get_server(id: String) -> Result<ServerDetails, String> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    Ok(ServerDetails {
        status: status_of(&record.id),
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
    })
}

#[flutter_rust_bridge::frb(sync)]
pub fn any_server_running() -> bool {
    process::any_running()
}

#[flutter_rust_bridge::frb(sync)]
pub fn app_data_dir() -> String {
    crate::paths::app_data_root().to_string_lossy().to_string()
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

pub async fn list_runtimes() -> Result<Vec<JavaRuntimeInfo>, String> {
    Ok(crate::java_runtime::collect(&Layout::app())
        .into_iter()
        .map(|runtime| JavaRuntimeInfo {
            major: runtime.major.unwrap_or(0),
            lts: false,
            name: runtime.name,
            path: runtime.home.to_string_lossy().to_string(),
        })
        .collect())
}

pub async fn list_java_releases() -> Result<Vec<JavaReleaseInfo>, String> {
    let releases = crate::java_runtime::list_releases().await?;
    Ok(releases
        .into_iter()
        .map(|release| JavaReleaseInfo {
            major: release.major,
            lts: release.lts,
        })
        .collect())
}

pub async fn list_paper_versions() -> Result<Vec<String>, String> {
    crate::paper::fetch_versions().await
}

pub async fn required_java_for(version: String) -> Result<u32, String> {
    crate::paper::required_java(&version).await
}

pub async fn preview_import(path: String) -> Result<ImportPreview, String> {
    let root = std::path::PathBuf::from(&path);
    if !root.is_dir() {
        return Err("Cartella non trovata".into());
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

pub async fn import_server(path: String, name: String, accept_eula: bool) -> Result<String, String> {
    install::import_folder(&Layout::app(), std::path::Path::new(&path), &name, accept_eula)
}

pub async fn accept_server_eula(id: String) -> Result<(), String> {
    install::accept_eula(&Layout::app(), &id)
}

pub async fn update_runtime_config(
    id: String,
    java_home: String,
    ram_min: String,
    ram_max: String,
    jvm_flags: Vec<String>,
) -> Result<(), String> {
    install::update_runtime(&Layout::app(), &id, &java_home, &ram_min, &ram_max, &jvm_flags)
}

pub async fn delete_server(id: String, delete_files: bool) -> Result<(), String> {
    install::delete_server(&Layout::app(), &id, delete_files).await
}

pub async fn start_server(id: String) -> Result<(), String> {
    install::launch(&Layout::app(), &id).await
}

pub async fn stop_server(id: String) -> Result<(), String> {
    process::stop(&id).await
}

pub async fn restart_server(id: String) -> Result<(), String> {
    if process::pid_of(&id).is_some() {
        process::stop(&id).await?;
    }
    install::launch(&Layout::app(), &id).await
}

pub async fn send_command(id: String, command: String) -> Result<(), String> {
    process::send_command(&id, &command).await
}

pub async fn console_history(id: String) -> Result<Vec<String>, String> {
    let _ = crate::catalog::get(&Layout::app(), &id)?;
    Ok(process::console_history(&id))
}

pub async fn server_stats(id: String) -> Result<Option<ProcessStats>, String> {
    let Some(pid) = process::pid_of(&id) else {
        return Ok(None);
    };
    Ok(process::process_usage(pid).map(|(cpu_percent, memory_bytes)| ProcessStats {
        cpu_percent,
        memory_bytes,
        pid,
    }))
}

pub async fn shutdown_all() -> Result<(), String> {
    process::shutdown_all().await;
    Ok(())
}

pub async fn install_auto(
    request: AutoInstallRequest,
    sink: StreamSink<ProgressEvent>,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let result = install::install_auto(
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
    .await;
    drop(tx);
    finish_progress(&sink, &rx, result)
}

pub async fn install_manual(
    request: ManualInstallRequest,
    sink: StreamSink<ProgressEvent>,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let java_major = if request.java_major == 0 {
        None
    } else {
        Some(request.java_major)
    };
    let result = install::install_manual(
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
    .await;
    drop(tx);
    finish_progress(&sink, &rx, result)
}

pub async fn install_java(major: u32, sink: StreamSink<ProgressEvent>) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let result = install::install_java(&Layout::app(), major, &tx).await;
    drop(tx);
    forward_progress(&sink, &rx);
    match result {
        Ok(path) => {
            let _ = sink.add(done_event("Java", format!("Java installato in {path}"), None));
            Ok(())
        }
        Err(error) => {
            let _ = sink.add(error_event(error.clone()));
            Err(error)
        }
    }
}

pub async fn watch_console(id: String, sink: StreamSink<String>) -> Result<(), String> {
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

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn finish_progress(
    sink: &StreamSink<ProgressEvent>,
    rx: &mpsc::Receiver<Progress>,
    result: Result<String, String>,
) -> Result<(), String> {
    forward_progress(sink, rx);
    match result {
        Ok(id) => {
            let _ = sink.add(done_event("Fatto", "Operazione completata.".into(), Some(id)));
            Ok(())
        }
        Err(error) => {
            let _ = sink.add(error_event(error.clone()));
            Err(error)
        }
    }
}

fn forward_progress(sink: &StreamSink<ProgressEvent>, rx: &mpsc::Receiver<Progress>) {
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
}

fn done_event(stage: &str, message: String, server_id: Option<String>) -> ProgressEvent {
    ProgressEvent {
        stage: stage.to_string(),
        message,
        fraction: Some(1.0),
        done: true,
        error: None,
        server_id,
    }
}

fn error_event(error: String) -> ProgressEvent {
    ProgressEvent {
        stage: "Errore".into(),
        message: error.clone(),
        fraction: None,
        done: true,
        error: Some(error),
        server_id: None,
    }
}
