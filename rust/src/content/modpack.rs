// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Server creation from a Modrinth `.mrpack` or a CurseForge modpack zip.

use std::io::Read;
use std::path::{Path, PathBuf};

use futures::stream::{self, StreamExt};
use serde_json::Value;

use crate::api::types::{ContentSourceKind, CreateServerRequest, ModpackRequest, ProviderKind};
use crate::net::Checksum;
use crate::paths::Layout;
use crate::{PanelError, PanelResult, ProgressTx};

#[derive(Debug, Clone)]
pub struct PackFile {
    pub path: String,
    pub urls: Vec<String>,
    pub checksum: Option<Checksum>,
}

#[derive(Debug, Clone)]
pub enum PackFormat {
    Modrinth { files: Vec<PackFile> },
    CurseForge { file_ids: Vec<i64> },
}

#[derive(Debug, Clone)]
pub struct PackIndex {
    pub name: String,
    pub mc_version: String,
    pub provider: ProviderKind,
    pub loader_version: Option<String>,
    pub overrides: Vec<String>,
    pub format: PackFormat,
}

fn read_json(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Value> {
    let mut entry = zip.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

/// `modrinth.index.json`: files whose server environment is not "unsupported".
pub fn parse_modrinth_index(index: &Value) -> PanelResult<PackIndex> {
    let dependencies = index.get("dependencies").and_then(Value::as_object).ok_or("modrinth.index.json senza dipendenze")?;
    let mc_version = dependencies.get("minecraft").and_then(Value::as_str).ok_or("Versione di Minecraft del modpack mancante")?.to_string();
    let loader = |key: &str| dependencies.get(key).and_then(Value::as_str).map(str::to_string);
    let (provider, loader_version) = if let Some(version) = loader("neoforge") {
        (ProviderKind::NeoForge, Some(version))
    } else if let Some(version) = loader("forge") {
        (ProviderKind::Forge, Some(version))
    } else if let Some(version) = loader("quilt-loader") {
        (ProviderKind::Quilt, Some(version))
    } else if let Some(version) = loader("fabric-loader") {
        (ProviderKind::Fabric, Some(version))
    } else {
        (ProviderKind::Vanilla, None)
    };
    let files = index
        .get("files")
        .and_then(Value::as_array)
        .map(|files| {
            files
                .iter()
                .filter(|file| file.get("env").and_then(|env| env.get("server")).and_then(Value::as_str) != Some("unsupported"))
                .filter_map(|file| {
                    let hashes = file.get("hashes");
                    let checksum = hashes
                        .and_then(|hashes| hashes.get("sha512"))
                        .and_then(Value::as_str)
                        .map(|hash| Checksum::Sha512(hash.to_string()))
                        .or_else(|| hashes.and_then(|hashes| hashes.get("sha1")).and_then(Value::as_str).map(|hash| Checksum::Sha1(hash.to_string())));
                    Some(PackFile {
                        path: file.get("path")?.as_str()?.to_string(),
                        urls: file.get("downloads")?.as_array()?.iter().filter_map(Value::as_str).map(str::to_string).collect(),
                        checksum,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(PackIndex {
        name: index.get("name").and_then(Value::as_str).unwrap_or("Modpack").to_string(),
        mc_version,
        provider,
        loader_version,
        overrides: vec!["overrides".into(), "server-overrides".into()],
        format: PackFormat::Modrinth { files },
    })
}

/// CurseForge `manifest.json`: loader like `forge-47.2.0`, files by project and file id.
pub fn parse_curseforge_manifest(manifest: &Value) -> PanelResult<PackIndex> {
    let minecraft = manifest.get("minecraft").ok_or("manifest.json senza sezione minecraft")?;
    let mc_version = minecraft.get("version").and_then(Value::as_str).ok_or("Versione di Minecraft del modpack mancante")?.to_string();
    let loader_id = minecraft
        .get("modLoaders")
        .and_then(Value::as_array)
        .and_then(|loaders| loaders.iter().find(|loader| loader.get("primary").and_then(Value::as_bool) == Some(true)).or_else(|| loaders.first()))
        .and_then(|loader| loader.get("id"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let (name, version) = loader_id.split_once('-').unwrap_or((loader_id, ""));
    let provider = match name {
        "neoforge" => ProviderKind::NeoForge,
        "forge" => ProviderKind::Forge,
        "fabric" => ProviderKind::Fabric,
        "quilt" => ProviderKind::Quilt,
        _ => ProviderKind::Vanilla,
    };
    let file_ids = manifest
        .get("files")
        .and_then(Value::as_array)
        .map(|files| files.iter().filter(|file| file.get("required").and_then(Value::as_bool) != Some(false)).filter_map(|file| file.get("fileID").and_then(Value::as_i64)).collect())
        .unwrap_or_default();
    Ok(PackIndex {
        name: manifest.get("name").and_then(Value::as_str).unwrap_or("Modpack").to_string(),
        mc_version,
        provider,
        loader_version: (!version.is_empty()).then(|| version.to_string()),
        overrides: vec![manifest.get("overrides").and_then(Value::as_str).unwrap_or("overrides").to_string()],
        format: PackFormat::CurseForge { file_ids },
    })
}

pub fn read_index(archive: &Path) -> PanelResult<PackIndex> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| PanelError::invalid(format!("Modpack non valido: {error}")))?;
    if let Some(index) = read_json(&mut zip, "modrinth.index.json") {
        return parse_modrinth_index(&index);
    }
    if let Some(manifest) = read_json(&mut zip, "manifest.json") {
        return parse_curseforge_manifest(&manifest);
    }
    Err(PanelError::invalid("Il file non è un modpack Modrinth (.mrpack) né CurseForge (manifest.json)"))
}

/// Copies override folders from the archive into the server, refusing paths that escape it.
pub fn extract_overrides(archive: &Path, root: &Path, prefixes: &[String]) -> PanelResult<u32> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| PanelError::invalid(error.to_string()))?;
    let mut count = 0;
    for prefix in prefixes {
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|error| PanelError::invalid(error.to_string()))?;
            let Some(name) = entry.enclosed_name() else { continue };
            let Ok(relative) = name.strip_prefix(prefix) else { continue };
            if relative.as_os_str().is_empty() {
                continue;
            }
            let target = crate::server_files::resolve(root, &relative.to_string_lossy())?;
            if entry.is_dir() {
                std::fs::create_dir_all(&target)?;
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut entry, &mut std::fs::File::create(&target)?)?;
            count += 1;
        }
    }
    Ok(count)
}

async fn fetch_archive(request: &ModpackRequest, progress: &ProgressTx) -> PanelResult<PathBuf> {
    if !request.file_path.trim().is_empty() {
        return Ok(PathBuf::from(request.file_path.trim()));
    }
    let cache = Layout::app().cache().join("modpacks");
    crate::paths::ensure_dir(&cache)?;
    match request.source {
        Some(ContentSourceKind::Modrinth) => {
            let version_id = if request.version_id.is_empty() {
                let versions = super::modrinth::project_versions(&request.project_id).await?;
                versions.first().and_then(|version| version.get("id")).and_then(Value::as_str).ok_or("Il modpack non ha versioni")?.to_string()
            } else {
                request.version_id.clone()
            };
            let version = super::modrinth::version(&version_id).await?;
            let (url, file_name, checksum) = super::modrinth::primary_file(&version).ok_or("Il modpack non ha file")?;
            let path = crate::providers::jar_destination(&cache, &file_name)?;
            progress.emit("Modpack", format!("Download di {file_name}..."), Some(0.0));
            crate::net::download_checked(&url, &path, "Modpack", progress, checksum).await?;
            Ok(path)
        }
        Some(ContentSourceKind::CurseForge) => {
            let file = super::curseforge::get(&format!("/mods/{}/files/{}", request.project_id, request.version_id), &[]).await?;
            let download = super::curseforge::download_from_file(file.get("data").ok_or("Risposta CurseForge senza dati")?)?;
            let path = crate::providers::jar_destination(&cache, &download.file_name)?;
            progress.emit("Modpack", format!("Download di {}...", download.file_name), Some(0.0));
            crate::net::download_checked(&download.url, &path, "Modpack", progress, download.checksum).await?;
            Ok(path)
        }
        _ => Err(PanelError::invalid("Scegli un file di modpack o un modpack da un catalogo")),
    }
}

/// Downloads with the parallelism set in the launcher settings; returns the files that failed.
async fn download_all(root: &Path, jobs: Vec<(String, Vec<String>, Option<Checksum>)>, progress: &ProgressTx) -> PanelResult<Vec<String>> {
    let parallel = crate::launcher_settings::current().network.parallel_downloads.max(1) as usize;
    let total = jobs.len().max(1);
    let silent = ProgressTx::silent();
    let mut done = 0usize;
    let mut failed = Vec::new();
    let mut results = stream::iter(jobs.into_iter().map(|(path, urls, checksum)| {
        let silent = silent.clone();
        async move {
            let target = crate::server_files::resolve(root, &path)?;
            let mut last = PanelError::not_found(format!("Nessun URL per {path}"));
            for url in &urls {
                match crate::net::download_checked(url, &target, "Modpack", &silent, checksum.clone()).await {
                    Ok(()) => return Ok(path),
                    Err(error) => last = error,
                }
            }
            Err(PanelError::from(format!("{path}: {}", last.message)))
        }
    }))
    .buffer_unordered(parallel);
    while let Some(result) = results.next().await {
        done += 1;
        match result {
            Ok(path) => progress.emit("Modpack", format!("{done}/{total} · {path}"), Some(done as f64 / total as f64)),
            Err(error) => failed.push(error.message),
        }
    }
    Ok(failed)
}

pub async fn create(layout: &Layout, request: ModpackRequest, progress: &ProgressTx) -> PanelResult<String> {
    let archive = fetch_archive(&request, progress).await?;
    let pack = {
        let archive = archive.clone();
        tokio::task::spawn_blocking(move || read_index(&archive)).await??
    };
    progress.emit("Modpack", format!("{}: Minecraft {} con {}", pack.name, pack.mc_version, crate::providers::meta(pack.provider).name), None);
    let name = if request.name.trim().is_empty() { pack.name.clone() } else { request.name.clone() };
    let create = CreateServerRequest {
        name,
        root: request.root.clone(),
        provider: pack.provider,
        mc_version: pack.mc_version.clone(),
        build: pack.loader_version.clone().unwrap_or_default(),
        jar_path: String::new(),
        java_home: request.java_home.clone(),
        ram_min: request.ram_min.clone(),
        ram_max: request.ram_max.clone(),
        jvm_flags: request.jvm_flags.clone(),
        port: 0,
        motd: pack.name.clone(),
        max_players: 20,
        gamemode: "survival".into(),
        difficulty: "normal".into(),
        online_mode: true,
        level_seed: String::new(),
        accept_eula: request.accept_eula,
    };
    let id = crate::install::create_server(layout, create, progress).await?;
    let root = crate::catalog::get(layout, &id)?.root;
    let jobs: Vec<(String, Vec<String>, Option<Checksum>)> = match &pack.format {
        PackFormat::Modrinth { files } => files.iter().map(|file| (file.path.clone(), file.urls.clone(), file.checksum.clone())).collect(),
        PackFormat::CurseForge { file_ids } => {
            progress.emit("Modpack", format!("Risoluzione di {} file su CurseForge...", file_ids.len()), None);
            let mut jobs = Vec::new();
            for chunk in file_ids.chunks(200) {
                for file in super::curseforge::files(chunk).await? {
                    match super::curseforge::download_from_file(&file) {
                        Ok(download) => jobs.push((format!("mods/{}", download.file_name), vec![download.url], download.checksum)),
                        Err(error) => progress.emit("Modpack", format!("{}: {}", file.get("fileName").and_then(Value::as_str).unwrap_or("?"), error.message), None),
                    }
                }
            }
            jobs
        }
    };
    progress.emit("Modpack", format!("Download di {} file...", jobs.len()), Some(0.0));
    let failed = download_all(&root, jobs, progress).await?;
    let overrides = {
        let (archive, root, prefixes) = (archive.clone(), root.clone(), pack.overrides.clone());
        tokio::task::spawn_blocking(move || extract_overrides(&archive, &root, &prefixes)).await??
    };
    progress.emit("Modpack", format!("{overrides} file di configurazione copiati dal modpack."), None);
    if !failed.is_empty() {
        progress.emit("Modpack", format!("{} file non scaricati: {}", failed.len(), failed.join("; ")), None);
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_modrinth_indexes() {
        let index = json!({
            "formatVersion": 1, "game": "minecraft", "name": "Pack",
            "dependencies": {"minecraft": "1.21.1", "fabric-loader": "0.16.5"},
            "files": [
                {"path": "mods/a.jar", "hashes": {"sha1": "a", "sha512": "b"}, "downloads": ["https://cdn/a.jar"], "env": {"client": "required", "server": "required"}},
                {"path": "mods/client.jar", "hashes": {"sha1": "c"}, "downloads": ["https://cdn/c.jar"], "env": {"client": "required", "server": "unsupported"}}
            ]
        });
        let pack = parse_modrinth_index(&index).unwrap();
        assert_eq!((pack.provider, pack.loader_version.as_deref(), pack.mc_version.as_str()), (ProviderKind::Fabric, Some("0.16.5"), "1.21.1"));
        let PackFormat::Modrinth { files } = pack.format else { panic!() };
        assert_eq!(files.len(), 1);
        assert!(matches!(&files[0].checksum, Some(Checksum::Sha512(hash)) if hash == "b"));
    }

    #[test]
    fn parses_curseforge_manifests() {
        let manifest = json!({"minecraft": {"version": "1.20.1", "modLoaders": [{"id": "forge-47.2.0", "primary": true}]}, "name": "CF", "files": [{"projectID": 1, "fileID": 11, "required": true}, {"projectID": 2, "fileID": 22, "required": false}], "overrides": "overrides"});
        let pack = parse_curseforge_manifest(&manifest).unwrap();
        assert_eq!((pack.provider, pack.loader_version.as_deref()), (ProviderKind::Forge, Some("47.2.0")));
        let PackFormat::CurseForge { file_ids } = pack.format else { panic!() };
        assert_eq!(file_ids, vec![11]);
    }

    #[test]
    fn extracts_overrides_safely() {
        let dir = std::env::temp_dir().join(format!("voxel-pack-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("server")).unwrap();
        let archive = dir.join("pack.mrpack");
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("modrinth.index.json", options).unwrap();
            std::io::Write::write_all(&mut zip, br#"{"dependencies": {"minecraft": "1.21.1"}, "files": []}"#).unwrap();
            zip.start_file("overrides/config/a.toml", options).unwrap();
            std::io::Write::write_all(&mut zip, b"a = 1").unwrap();
            zip.start_file("server-overrides/server.properties", options).unwrap();
            std::io::Write::write_all(&mut zip, b"motd=Pack").unwrap();
            zip.finish().unwrap();
        }
        let pack = read_index(&archive).unwrap();
        assert_eq!(pack.provider, ProviderKind::Vanilla);
        let copied = extract_overrides(&archive, &dir.join("server"), &pack.overrides).unwrap();
        assert_eq!(copied, 2);
        assert!(dir.join("server/config/a.toml").is_file());
        assert!(dir.join("server/server.properties").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
