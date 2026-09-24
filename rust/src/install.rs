// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use uuid::Uuid;

use crate::paths::Layout;
use crate::Progress;
use crate::ServerRecord;

pub struct AutoRequest {
    pub name: String,
    pub root: String,
    pub paper_version: String,
    pub accept_eula: bool,
    pub ram_min: String,
    pub ram_max: String,
}

pub struct ManualRequest {
    pub name: String,
    pub root: String,
    pub paper_version: Option<String>,
    pub jar_path: Option<String>,
    pub java_major: Option<u32>,
    pub java_home: Option<String>,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    pub accept_eula: bool,
}

pub async fn install_auto(
    layout: &Layout,
    request: AutoRequest,
    progress: &Sender<Progress>,
) -> Result<String, String> {
    let name = clean_name(&request.name)?;
    if !request.accept_eula {
        return Err("Devi accettare l'EULA di Minecraft per creare il server.".into());
    }
    if request.paper_version.trim().is_empty() {
        return Err("Seleziona una versione Paper.".into());
    }
    let id = Uuid::new_v4().to_string();
    let root = prepare_root(layout, &request.root, &id)?;
    emit(progress, "Java", format!("Versione Paper {}.", request.paper_version), None);
    let required = crate::paper::required_java(&request.paper_version).await?;
    emit(progress, "Java", format!("Java richiesto: {required}."), None);
    let java_home = crate::java_runtime::ensure_major(&layout.runtimes(), required, progress).await?;
    let jar = crate::paper::download_paper(&request.paper_version, &root, progress).await?;
    let (auto_min, auto_max, total) = crate::ram::suggest();
    let ram_min = choose_ram(&request.ram_min, &auto_min)?;
    let ram_max = choose_ram(&request.ram_max, &auto_max)?;
    emit(
        progress,
        "RAM",
        format!("RAM rilevata {total}. Il server userà {ram_min} / {ram_max}."),
        None,
    );
    let record = ServerRecord {
        id: id.clone(),
        name,
        root: root.clone(),
        paper_version: Some(request.paper_version),
        java_major: Some(required),
        java_home: Some(java_home),
        jar_path: Some(jar),
        ram_min,
        ram_max,
        jvm_flags: crate::jvm::default_flags(),
        eula_accepted: true,
        created_unix: crate::paths::unix_now(),
    };
    let mut settings = crate::properties::Settings::default();
    settings.port = free_port(layout, &id);
    persist(layout, &record, &settings)?;
    emit(progress, "Fatto", "Installazione automatica completata.".into(), Some(1.0));
    Ok(id)
}

pub async fn install_manual(
    layout: &Layout,
    request: ManualRequest,
    progress: &Sender<Progress>,
) -> Result<String, String> {
    let name = clean_name(&request.name)?;
    if !request.accept_eula {
        return Err("Devi accettare l'EULA di Minecraft per creare il server.".into());
    }
    let ram_min = choose_ram(&request.ram_min, "2G")?;
    let ram_max = choose_ram(&request.ram_max, "4G")?;
    let jvm_flags = crate::jvm::sanitize_flags(&request.jvm_flags)?;
    let id = Uuid::new_v4().to_string();
    let root = prepare_root(layout, &request.root, &id)?;
    let (java_home, java_major) = resolve_java(layout, &request, progress).await?;
    let (jar_path, paper_version) = resolve_jar(&root, &request, progress).await?;
    if let (Some(required_version), Some(major)) = (paper_version.clone(), java_major) {
        if let Ok(required) = crate::paper::required_java(&required_version).await {
            if required != major {
                emit(
                    progress,
                    "Java",
                    format!("Attenzione: Paper {required_version} richiede Java {required}, selezionato Java {major}."),
                    None,
                );
            }
        }
    }
    let record = ServerRecord {
        id: id.clone(),
        name,
        root: root.clone(),
        paper_version,
        java_major,
        java_home: Some(java_home),
        jar_path: Some(jar_path),
        ram_min,
        ram_max,
        jvm_flags,
        eula_accepted: true,
        created_unix: crate::paths::unix_now(),
    };
    let mut settings = crate::properties::Settings::default();
    settings.port = free_port(layout, &id);
    persist(layout, &record, &settings)?;
    emit(progress, "Fatto", "Installazione manuale completata.".into(), Some(1.0));
    Ok(id)
}

pub async fn install_java(layout: &Layout, major: u32, progress: &Sender<Progress>) -> Result<String, String> {
    let home = crate::java_runtime::ensure_major(&layout.runtimes(), major, progress).await?;
    Ok(home.to_string_lossy().to_string())
}

pub fn import_folder(
    layout: &Layout,
    path: &Path,
    name: &str,
    accept_eula: bool,
) -> Result<String, String> {
    if !path.is_dir() {
        return Err("Cartella non trovata".into());
    }
    let detected = crate::scan::detect(path);
    let name = if name.trim().is_empty() {
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Server")
            .to_string()
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
    let record = ServerRecord {
        id: id.clone(),
        name,
        root: path.to_path_buf(),
        paper_version: detected.paper_version,
        java_major: detected.java_major,
        java_home: detected.java_home,
        jar_path: detected.jar_path,
        ram_min: detected.ram_min.unwrap_or_else(|| "2G".into()),
        ram_max: detected.ram_max.unwrap_or_else(|| "4G".into()),
        jvm_flags: detected.jvm_flags,
        eula_accepted: accept_eula || has_eula_file,
        created_unix: crate::paths::unix_now(),
    };
    if record.java_home.is_some() && record.jar_path.is_some() {
        crate::script::generate(path, &record)?;
    }
    crate::catalog::save(layout, &record)?;
    Ok(id)
}

pub fn update_runtime(
    layout: &Layout,
    id: &str,
    java_home: &str,
    ram_min: &str,
    ram_max: &str,
    jvm_flags: &[String],
) -> Result<(), String> {
    ensure_stopped(id)?;
    if !crate::ram::is_memory_value(ram_min) || !crate::ram::is_memory_value(ram_max) {
        return Err("RAM non valida. Usa valori come 2G o 4096M.".into());
    }
    let flags = crate::jvm::sanitize_flags(jvm_flags)?;
    let home = PathBuf::from(java_home);
    if !crate::java_runtime::java_executable(&home).exists() {
        return Err("Runtime Java non trovato".into());
    }
    let mut record = crate::catalog::get(layout, id)?;
    record.java_home = Some(home.clone());
    record.java_major = crate::java_runtime::derive_from_home(&home);
    record.ram_min = ram_min.to_string();
    record.ram_max = ram_max.to_string();
    record.jvm_flags = flags;
    if record.jar_path.is_some() {
        crate::script::generate(&record.root, &record)?;
    }
    crate::catalog::save(layout, &record)
}

pub fn accept_eula(layout: &Layout, id: &str) -> Result<(), String> {
    let mut record = crate::catalog::get(layout, id)?;
    crate::script::write_eula(&record.root)?;
    record.eula_accepted = true;
    crate::catalog::save(layout, &record)
}

pub async fn delete_server(layout: &Layout, id: &str, delete_files: bool) -> Result<(), String> {
    ensure_stopped(id)?;
    let record = crate::catalog::remove(layout, id)?;
    if delete_files {
        if let Some(record) = record {
            if record.root.exists() {
                tokio::fs::remove_dir_all(&record.root)
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        let backups = layout.backups(id);
        if backups.exists() {
            tokio::fs::remove_dir_all(backups)
                .await
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub async fn launch(layout: &Layout, id: &str) -> Result<(), String> {
    let record = crate::catalog::get(layout, id)?;
    if !record.eula_accepted {
        return Err("Accetta l'EULA prima di avviare il server.".into());
    }
    crate::script::write_eula(&record.root)?;
    let java_home = record.java_home.as_ref().ok_or("Runtime Java non configurato")?;
    let java = crate::java_runtime::java_executable(java_home);
    if !java.exists() {
        return Err(format!("Java non trovato: {}", java.display()));
    }
    let jar = record.jar_path.as_ref().ok_or("Jar server non configurato")?;
    if !jar.exists() {
        return Err(format!("Jar non trovato: {}", jar.display()));
    }
    if !crate::ram::is_memory_value(&record.ram_min) || !crate::ram::is_memory_value(&record.ram_max) {
        return Err("RAM non valida.".into());
    }
    crate::script::generate(&record.root, &record)?;
    let mut args = vec![
        format!("-Xms{}", record.ram_min),
        format!("-Xmx{}", record.ram_max),
    ];
    args.extend(record.jvm_flags.clone());
    args.push("-jar".into());
    args.push(jar.to_string_lossy().to_string());
    args.push("nogui".into());
    crate::process::start(id, &java, &args, &record.root).await
}

pub fn save_settings(layout: &Layout, id: &str, settings: &crate::properties::Settings) -> Result<(), String> {
    let record = crate::catalog::get(layout, id)?;
    let used = crate::catalog::used_ports(layout, id);
    if used.contains(&settings.port) {
        return Err(format!("La porta {} è già usata da un altro server.", settings.port));
    }
    crate::properties::write_settings(&record.root, settings)
}

fn ensure_stopped(id: &str) -> Result<(), String> {
    if crate::process::pid_of(id).is_some() {
        Err("Ferma il server prima di continuare.".into())
    } else {
        Ok(())
    }
}

fn persist(layout: &Layout, record: &ServerRecord, settings: &crate::properties::Settings) -> Result<(), String> {
    crate::script::write_eula(&record.root)?;
    crate::properties::write_settings(&record.root, settings)?;
    crate::paths::ensure_dir(&record.root.join("plugins"))?;
    crate::script::generate(&record.root, record)?;
    crate::catalog::save(layout, record)
}

fn free_port(layout: &Layout, id: &str) -> u32 {
    let used = crate::catalog::used_ports(layout, id);
    let mut port = 25565u32;
    while used.contains(&port) {
        port = port.saturating_add(1);
    }
    port
}

async fn resolve_java(
    layout: &Layout,
    request: &ManualRequest,
    progress: &Sender<Progress>,
) -> Result<(PathBuf, Option<u32>), String> {
    if let Some(home) = request.java_home.as_ref().filter(|value| !value.trim().is_empty()) {
        let home = PathBuf::from(home);
        if !crate::java_runtime::java_executable(&home).exists() {
            return Err("Runtime Java selezionato non trovato".into());
        }
        let major = crate::java_runtime::derive_from_home(&home);
        return Ok((home, major));
    }
    let major = request.java_major.ok_or("Seleziona un runtime Java")?;
    let home = crate::java_runtime::ensure_major(&layout.runtimes(), major, progress).await?;
    Ok((home, Some(major)))
}

async fn resolve_jar(
    root: &Path,
    request: &ManualRequest,
    progress: &Sender<Progress>,
) -> Result<(PathBuf, Option<String>), String> {
    if let Some(jar) = request.jar_path.as_ref().filter(|value| !value.trim().is_empty()) {
        let source = PathBuf::from(jar);
        if !source.is_file() {
            return Err("Jar server non trovato".into());
        }
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Nome jar non valido")?;
        let destination = if source.starts_with(root) {
            source.clone()
        } else {
            let destination = root.join(file_name);
            tokio::fs::copy(&source, &destination)
                .await
                .map_err(|error| format!("Copia jar fallita: {error}"))?;
            destination
        };
        let version = crate::paper::parse_paper_version(file_name).or_else(|| {
            request
                .paper_version
                .clone()
                .filter(|value| !value.trim().is_empty())
        });
        return Ok((destination, version));
    }
    let version = request
        .paper_version
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .ok_or("Seleziona una versione Paper oppure un jar esistente")?;
    let jar = crate::paper::download_paper(version, root, progress).await?;
    Ok((jar, Some(version.clone())))
}

fn prepare_root(layout: &Layout, requested: &str, id: &str) -> Result<PathBuf, String> {
    let root = if requested.trim().is_empty() {
        layout.default_server(id)
    } else {
        PathBuf::from(requested.trim())
    };
    if root.exists() {
        let occupied = std::fs::read_dir(&root)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);
        if occupied {
            return Err("La cartella non è vuota. Usa Importa per una cartella già usata.".into());
        }
    }
    crate::paths::ensure_dir(&root)?;
    Ok(root)
}

fn clean_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.contains(['\n', '\r', '/', '\\']) {
        return Err("Nome server non valido".into());
    }
    Ok(name.to_string())
}

fn choose_ram(value: &str, fallback: &str) -> Result<String, String> {
    let value = if value.trim().is_empty() { fallback } else { value.trim() };
    if !crate::ram::is_memory_value(value) {
        return Err("RAM non valida. Usa valori come 2G o 4096M.".into());
    }
    Ok(value.to_string())
}

fn emit(progress: &Sender<Progress>, stage: &str, message: String, fraction: Option<f64>) {
    let _ = progress.send(Progress {
        stage: stage.to_string(),
        message,
        fraction,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn imports_an_existing_folder_in_place() {
        let layout = Layout {
            root: std::env::temp_dir().join(format!("voxel-import-{}", uuid::Uuid::new_v4())),
        };
        let server = layout.root.join("old-server");
        let runtime = server.join("runtime").join("jdk-21.0.2").join("bin");
        fs::create_dir_all(&runtime).unwrap();
        fs::create_dir_all(server.join("plugins")).unwrap();
        fs::create_dir_all(server.join("world")).unwrap();
        fs::write(runtime.join("java.exe"), b"").unwrap();
        fs::write(server.join("paper-1.21.1-10.jar"), b"jar").unwrap();
        fs::write(server.join("world").join("level.dat"), b"world").unwrap();
        fs::write(server.join("plugins").join("sample.jar"), b"plugin").unwrap();
        fs::write(
            server.join("start.bat"),
            "@echo off\r\n\"runtime\\jdk-21.0.2\\bin\\java.exe\" -Xms2G -Xmx4G -XX:+UseG1GC -jar \"paper-1.21.1-10.jar\" nogui\r\n",
        )
        .unwrap();
        fs::write(server.join("server.properties"), "motd=Vecchio\r\nserver-port=25566\r\n").unwrap();

        let id = import_folder(&layout, &server, "Importato", true).unwrap();
        let record = crate::catalog::get(&layout, &id).unwrap();
        assert_eq!(record.name, "Importato");
        assert_eq!(record.root, server);
        assert_eq!(record.paper_version.as_deref(), Some("1.21.1"));
        assert_eq!(record.java_major, Some(21));
        assert_eq!(record.ram_min, "2G");
        assert!(server.join("world").join("level.dat").exists());
        assert!(std::fs::read_to_string(server.join("eula.txt")).unwrap().contains("eula=true"));
        let _ = fs::remove_dir_all(&layout.root);
    }
}
