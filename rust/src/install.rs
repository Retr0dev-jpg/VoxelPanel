// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::api::types::{CreateServerRequest, ProviderKind};
use crate::paths::Layout;
use crate::providers::{self, InstallRequest};
use crate::{ErrorCode, LaunchSpec, PanelError, PanelResult, ProgressTx, ServerRecord, RECORD_SCHEMA};

/// Creates a server: resolves Java for the chosen version, installs the software through
/// its provider and writes the initial configuration.
pub async fn create_server(layout: &Layout, request: CreateServerRequest, progress: &ProgressTx) -> PanelResult<String> {
    let name = clean_name(&request.name)?;
    if !request.accept_eula {
        return Err(PanelError::new(ErrorCode::EulaRequired, "Devi accettare l'EULA di Minecraft per creare il server."));
    }
    let custom_jar = (!request.jar_path.trim().is_empty()).then(|| PathBuf::from(request.jar_path.trim()));
    if custom_jar.is_none() && request.mc_version.trim().is_empty() {
        return Err(PanelError::invalid("Seleziona una versione."));
    }
    let (auto_min, auto_max, total) = default_ram();
    let ram_min = choose_ram(&request.ram_min, &auto_min)?;
    let ram_max = choose_ram(&request.ram_max, &auto_max)?;
    let jvm_flags = crate::jvm::sanitize_flags(&request.jvm_flags)?;
    let id = Uuid::new_v4().to_string();
    let root = prepare_root(layout, &request.root, &id)?;
    let result = install_into(layout, &root, &request, custom_jar, progress).await;
    let (installed, java_home, java_major) = match result {
        Ok(value) => value,
        Err(error) => {
            // A failed install must not leave an empty folder that blocks the next attempt.
            if request.root.trim().is_empty() {
                let _ = std::fs::remove_dir_all(&root);
            }
            return Err(error);
        }
    };
    progress.emit("RAM", format!("RAM del sistema {total}. Il server userà {ram_min} / {ram_max}."), None);
    let mut record = ServerRecord::new(id.clone(), name, root.clone());
    record.provider = request.provider;
    record.mc_version = Some(installed.mc_version).filter(|version| !version.is_empty());
    record.build = installed.build;
    record.java_home = Some(java_home);
    record.java_major = Some(java_major);
    record.launch = installed.launch;
    record.ram_min = ram_min;
    record.ram_max = ram_max;
    record.jvm_flags = jvm_flags;
    record.eula_accepted = true;
    let port = if request.port == 0 { free_port(layout, &id) } else { request.port };
    if crate::catalog::used_ports(layout, &id).contains(&port) {
        return Err(PanelError::new(ErrorCode::PortInUse, format!("La porta {port} è già usata da un altro server.")));
    }
    persist(layout, &record, &request, port)?;
    progress.emit("Fatto", "Installazione completata.", Some(1.0));
    Ok(id)
}

async fn install_into(
    layout: &Layout,
    root: &Path,
    request: &CreateServerRequest,
    custom_jar: Option<PathBuf>,
    progress: &ProgressTx,
) -> PanelResult<(providers::Installed, PathBuf, u32)> {
    if let Some(source) = custom_jar {
        let jar = copy_jar(root, &source).await?;
        let (detected, version) = providers::detect(root, Some(&jar));
        let mc_version = Some(request.mc_version.trim().to_string()).filter(|value| !value.is_empty()).or(version);
        let lookup = if detected == ProviderKind::Custom { ProviderKind::Vanilla } else { detected };
        let required = match (&mc_version, providers::get(lookup)) {
            (Some(version), Ok(provider)) => provider.required_java(version).await.unwrap_or(21),
            _ => 21,
        };
        let (java_home, major) = resolve_java(layout, &request.java_home, required, progress).await?;
        let installed = providers::Installed {
            launch: LaunchSpec::Jar { path: jar },
            mc_version: mc_version.unwrap_or_default(),
            build: None,
        };
        return Ok((installed, java_home, major));
    }
    let provider = providers::get(request.provider)?;
    let version = request.mc_version.trim();
    progress.emit("Java", format!("{} {version}: verifico la versione di Java richiesta...", providers::meta(request.provider).name), None);
    let required = provider.required_java(version).await?;
    progress.emit("Java", format!("Java richiesto: {required}."), None);
    let (java_home, major) = resolve_java(layout, &request.java_home, required, progress).await?;
    let java = crate::java_runtime::java_executable(&java_home);
    let build = Some(request.build.trim()).filter(|build| !build.is_empty());
    let installed = provider
        .install(InstallRequest { root, mc_version: version, build, java: &java, progress })
        .await?;
    Ok((installed, java_home, major))
}

async fn copy_jar(root: &Path, source: &Path) -> PanelResult<PathBuf> {
    if !source.is_file() {
        return Err(PanelError::not_found("Jar del server non trovato"));
    }
    let file_name = source.file_name().and_then(|name| name.to_str()).ok_or("Nome jar non valido")?;
    let destination = providers::jar_destination(root, file_name)?;
    if source != destination {
        tokio::fs::copy(source, &destination)
            .await
            .map_err(|error| PanelError::io(format!("Copia jar fallita: {error}")))?;
    }
    Ok(destination)
}

/// Uses the Java chosen by the user, then the preferred runtime for `required`, then a managed download.
async fn resolve_java(layout: &Layout, chosen: &str, required: u32, progress: &ProgressTx) -> PanelResult<(PathBuf, u32)> {
    let chosen = chosen.trim();
    if !chosen.is_empty() {
        let home = PathBuf::from(chosen);
        if !crate::java_runtime::java_executable(&home).is_file() {
            return Err(PanelError::not_found("Runtime Java selezionato non trovato"));
        }
        let major = crate::java_runtime::derive_from_home(&home).unwrap_or(required);
        if major < required {
            progress.emit("Java", format!("Attenzione: questa versione richiede Java {required}, selezionato Java {major}."), None);
        }
        return Ok((home, major));
    }
    Ok((java_for_major(layout, required, progress).await?, required))
}

pub async fn install_java(layout: &Layout, major: u32, progress: &ProgressTx) -> PanelResult<String> {
    let home = crate::java_runtime::ensure_major(&layout.runtimes(), major, progress).await?;
    Ok(home.to_string_lossy().to_string())
}

/// Uses the runtime chosen in the launcher settings for this major, otherwise a managed one.
async fn java_for_major(layout: &Layout, major: u32, progress: &ProgressTx) -> PanelResult<PathBuf> {
    let preferred = crate::launcher_settings::current()
        .java
        .preferred
        .into_iter()
        .find(|entry| entry.major == major)
        .map(|entry| PathBuf::from(entry.path))
        .filter(|home| crate::java_runtime::java_executable(home).is_file());
    if let Some(home) = preferred {
        progress.emit("Java", format!("Uso il Java {major} scelto nelle impostazioni."), Some(1.0));
        return Ok(home);
    }
    crate::java_runtime::ensure_major(&layout.runtimes(), major, progress).await
}

/// RAM defaults from the launcher settings, or suggested from the system memory.
fn default_ram() -> (String, String, String) {
    let (auto_min, auto_max, total) = crate::ram::suggest();
    let defaults = crate::launcher_settings::current().defaults;
    let pick = |value: String, fallback: String| if value.is_empty() { fallback } else { value };
    (pick(defaults.ram_min, auto_min), pick(defaults.ram_max, auto_max), total)
}

/// Replaces the server software with another version or build, after an automatic backup.
pub async fn change_version(layout: &Layout, id: &str, mc_version: &str, build: &str, progress: &ProgressTx) -> PanelResult<String> {
    crate::process::ensure_stopped(id)?;
    let mut record = crate::catalog::get(layout, id)?;
    let provider = providers::get(record.provider)?;
    let mc_version = mc_version.trim();
    if mc_version.is_empty() {
        return Err(PanelError::invalid("Seleziona una versione."));
    }
    progress.emit("Backup", "Backup automatico prima del cambio di versione...", None);
    let backup = crate::backup::safety_backup(layout, &record, "pre-update").await?;
    progress.emit("Backup", format!("Backup creato: {backup}"), None);
    let required = provider.required_java(mc_version).await?;
    let (java_home, major) = match record.java_home.clone().filter(|_| record.java_major.is_some_and(|current| current >= required)) {
        Some(home) if crate::java_runtime::java_executable(&home).is_file() => (home, record.java_major.unwrap_or(required)),
        _ => (java_for_major(layout, required, progress).await?, required),
    };
    let java = crate::java_runtime::java_executable(&java_home);
    let build = Some(build.trim()).filter(|build| !build.is_empty());
    let installed = provider
        .install(InstallRequest { root: &record.root, mc_version, build, java: &java, progress })
        .await?;
    let previous = record.launch.jar().cloned();
    if let (Some(old), Some(new)) = (previous, installed.launch.jar()) {
        if old != *new && old.starts_with(&record.root) && old.is_file() {
            let _ = std::fs::remove_file(old);
        }
    }
    record.mc_version = Some(installed.mc_version.clone());
    record.build = installed.build;
    record.launch = installed.launch;
    record.java_home = Some(java_home);
    record.java_major = Some(major);
    crate::script::generate(&record.root, &record)?;
    crate::catalog::save(layout, &record)?;
    Ok(installed.mc_version)
}

pub fn import_folder(layout: &Layout, path: &Path, name: &str, accept_eula: bool) -> PanelResult<String> {
    if !path.is_dir() {
        return Err(PanelError::not_found("Cartella non trovata"));
    }
    let detected = crate::scan::detect(path);
    let name = if name.trim().is_empty() {
        path.file_name().and_then(|value| value.to_str()).unwrap_or("Server").to_string()
    } else {
        clean_name(name)?
    };
    let has_eula_file = std::fs::read_to_string(path.join("eula.txt"))
        .ok()
        .is_some_and(|text| text.to_lowercase().contains("eula=true"));
    if accept_eula {
        crate::script::write_eula(path)?;
    }
    let id = Uuid::new_v4().to_string();
    let mut record = ServerRecord::new(id.clone(), name, path.to_path_buf());
    record.provider = detected.provider;
    record.mc_version = detected.mc_version;
    record.java_major = detected.java_major;
    record.java_home = detected.java_home;
    record.launch = detected.launch;
    record.ram_min = detected.ram_min.unwrap_or_else(|| "2G".into());
    record.ram_max = detected.ram_max.unwrap_or_else(|| "4G".into());
    record.jvm_flags = detected.jvm_flags;
    record.eula_accepted = accept_eula || has_eula_file;
    if record.java_home.is_some() && record.launch != LaunchSpec::Unset {
        crate::script::generate(path, &record)?;
    }
    crate::catalog::save(layout, &record)?;
    Ok(id)
}

pub fn update_runtime(layout: &Layout, id: &str, java_home: &str, ram_min: &str, ram_max: &str, jvm_flags: &[String]) -> PanelResult<()> {
    crate::process::ensure_stopped(id)?;
    if !crate::ram::is_memory_value(ram_min) || !crate::ram::is_memory_value(ram_max) {
        return Err(PanelError::invalid("RAM non valida. Usa valori come 2G o 4096M."));
    }
    let flags = crate::jvm::sanitize_flags(jvm_flags)?;
    let home = PathBuf::from(java_home);
    if !crate::java_runtime::java_executable(&home).exists() {
        return Err(PanelError::not_found("Runtime Java non trovato"));
    }
    let mut record = crate::catalog::get(layout, id)?;
    record.java_major = crate::java_runtime::derive_from_home(&home);
    record.java_home = Some(home);
    record.ram_min = ram_min.to_string();
    record.ram_max = ram_max.to_string();
    record.jvm_flags = flags;
    if record.launch != LaunchSpec::Unset {
        crate::script::generate(&record.root, &record)?;
    }
    crate::catalog::save(layout, &record)
}

pub fn rename(layout: &Layout, id: &str, name: &str) -> PanelResult<()> {
    let mut record = crate::catalog::get(layout, id)?;
    record.name = clean_name(name)?;
    crate::catalog::save(layout, &record)
}

pub fn accept_eula(layout: &Layout, id: &str) -> PanelResult<()> {
    let mut record = crate::catalog::get(layout, id)?;
    crate::script::write_eula(&record.root)?;
    record.eula_accepted = true;
    crate::catalog::save(layout, &record)
}

pub async fn delete_server(layout: &Layout, id: &str, delete_files: bool, delete_backups: bool) -> PanelResult<()> {
    crate::process::ensure_stopped(id)?;
    let record = crate::catalog::remove(layout, id)?;
    if delete_files {
        if let Some(record) = record.filter(|record| record.root.exists()) {
            tokio::fs::remove_dir_all(&record.root).await?;
        }
    }
    if delete_backups {
        let backups = layout.backups(id);
        if backups.exists() {
            tokio::fs::remove_dir_all(backups).await?;
        }
    }
    Ok(())
}

pub async fn launch(layout: &Layout, id: &str) -> PanelResult<()> {
    let record = crate::catalog::get(layout, id)?;
    if !record.eula_accepted && !providers::is_proxy(record.provider) {
        return Err(PanelError::new(ErrorCode::EulaRequired, "Accetta l'EULA prima di avviare il server."));
    }
    if !providers::is_proxy(record.provider) {
        crate::script::write_eula(&record.root)?;
    }
    let (java, args) = crate::script::command_line(&record)?;
    if !java.exists() {
        return Err(PanelError::not_found(format!("Java non trovato: {}", java.display())));
    }
    if let Some(target) = record.launch.target().filter(|target| !target.exists()) {
        return Err(PanelError::not_found(format!("File di avvio non trovato: {}", target.display())));
    }
    crate::script::generate(&record.root, &record)?;
    crate::process::start(id, &java, &args, &record.root).await
}

pub fn save_settings(layout: &Layout, id: &str, settings: &crate::properties::Settings) -> PanelResult<()> {
    let record = crate::catalog::get(layout, id)?;
    if crate::catalog::used_ports(layout, id).contains(&settings.port) {
        return Err(PanelError::new(ErrorCode::PortInUse, format!("La porta {} è già usata da un altro server.", settings.port)));
    }
    crate::properties::write_settings(&record.root, settings)
}

fn persist(layout: &Layout, record: &ServerRecord, request: &CreateServerRequest, port: u32) -> PanelResult<()> {
    if providers::is_proxy(record.provider) {
        crate::paths::ensure_dir(&record.root.join("plugins"))?;
    } else {
        crate::script::write_eula(&record.root)?;
        let defaults = crate::properties::Settings::default();
        let settings = crate::properties::Settings {
            port,
            motd: Some(request.motd.trim()).filter(|motd| !motd.is_empty()).map(str::to_string).unwrap_or(defaults.motd.clone()),
            max_players: if request.max_players == 0 { defaults.max_players } else { request.max_players },
            gamemode: Some(request.gamemode.as_str()).filter(|value| !value.is_empty()).map(str::to_string).unwrap_or(defaults.gamemode.clone()),
            difficulty: Some(request.difficulty.as_str()).filter(|value| !value.is_empty()).map(str::to_string).unwrap_or(defaults.difficulty.clone()),
            online_mode: request.online_mode,
            level_seed: request.level_seed.trim().to_string(),
            ..defaults
        };
        crate::properties::write_settings(&record.root, &settings)?;
        let meta = providers::meta(record.provider);
        if meta.supports_plugins {
            crate::paths::ensure_dir(&record.root.join("plugins"))?;
        }
        if meta.supports_mods {
            crate::paths::ensure_dir(&record.root.join("mods"))?;
        }
    }
    crate::script::generate(&record.root, record)?;
    let mut record = record.clone();
    record.schema_version = RECORD_SCHEMA;
    crate::catalog::save(layout, &record)
}

fn free_port(layout: &Layout, id: &str) -> u32 {
    let used = crate::catalog::used_ports(layout, id);
    let mut port = crate::launcher_settings::current().defaults.port;
    while used.contains(&port) {
        port = port.saturating_add(1);
    }
    port
}

fn prepare_root(layout: &Layout, requested: &str, id: &str) -> PanelResult<PathBuf> {
    let root = if requested.trim().is_empty() { layout.default_server(id) } else { PathBuf::from(requested.trim()) };
    if root.exists() && std::fs::read_dir(&root).map(|mut entries| entries.next().is_some()).unwrap_or(false) {
        return Err(PanelError::invalid("La cartella non è vuota. Usa Importa per una cartella già usata."));
    }
    crate::paths::ensure_dir(&root)?;
    Ok(root)
}

fn clean_name(name: &str) -> PanelResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.contains(['\n', '\r', '/', '\\']) {
        return Err(PanelError::invalid("Nome server non valido"));
    }
    Ok(name.to_string())
}

fn choose_ram(value: &str, fallback: &str) -> PanelResult<String> {
    let value = if value.trim().is_empty() { fallback } else { value.trim() };
    if !crate::ram::is_memory_value(value) {
        return Err(PanelError::invalid("RAM non valida. Usa valori come 2G o 4096M."));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn imports_an_existing_folder_in_place() {
        let layout = Layout::at(std::env::temp_dir().join(format!("voxel-import-{}", uuid::Uuid::new_v4())));
        let server = layout.root.join("old-server");
        let runtime = server.join("runtime").join("jdk-21.0.2").join("bin");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join(crate::platform::JAVA_BINARY), b"").unwrap();
        fs::write(runtime.join("java.exe"), b"").unwrap();
        fs::create_dir_all(server.join("plugins")).unwrap();
        fs::create_dir_all(server.join("world")).unwrap();
        fs::write(server.join("purpur-1.21.1-2300.jar"), b"jar").unwrap();
        fs::write(server.join("world").join("level.dat"), b"world").unwrap();
        fs::write(server.join("plugins").join("sample.jar"), b"plugin").unwrap();
        fs::write(
            server.join("start.bat"),
            "@echo off\r\n\"runtime\\jdk-21.0.2\\bin\\java.exe\" -Xms2G -Xmx4G -XX:+UseG1GC -jar \"purpur-1.21.1-2300.jar\" nogui\r\n",
        )
        .unwrap();
        fs::write(server.join("server.properties"), "motd=Vecchio\r\nserver-port=25566\r\n").unwrap();

        let id = import_folder(&layout, &server, "Importato", true).unwrap();
        let record = crate::catalog::get(&layout, &id).unwrap();
        assert_eq!(record.name, "Importato");
        assert_eq!(record.root, server);
        assert_eq!(record.provider, ProviderKind::Purpur);
        assert_eq!(record.mc_version.as_deref(), Some("1.21.1"));
        assert_eq!(record.java_major, Some(21));
        assert_eq!(record.ram_min, "2G");
        assert_eq!(record.launch.jar(), Some(&server.join("purpur-1.21.1-2300.jar")));
        assert!(server.join("world").join("level.dat").exists());
        assert!(fs::read_to_string(server.join("eula.txt")).unwrap().contains("eula=true"));
        let _ = fs::remove_dir_all(&layout.root);
    }
}
