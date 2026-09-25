// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{PanelError, PanelResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    #[serde(alias = "paper_version")]
    pub mc_version: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    pub server_name: String,
    pub created_unix: i64,
}

impl BackupManifest {
    pub fn of(record: &crate::ServerRecord) -> Self {
        Self {
            mc_version: record.mc_version.clone(),
            provider: Some(crate::providers::id_of(record.provider).to_string()),
            server_name: record.name.clone(),
            created_unix: crate::paths::unix_now(),
        }
    }
}

/// Backup taken automatically before a risky operation; returns the archive name.
pub async fn safety_backup(layout: &crate::paths::Layout, record: &crate::ServerRecord, prefix: &str) -> PanelResult<String> {
    let directory = layout.backups(&record.id);
    crate::paths::ensure_dir(&directory)?;
    let file_name = format!("{prefix}-{}.zip", crate::paths::unix_now());
    let destination = directory.join(&file_name);
    let manifest = BackupManifest::of(record);
    let root = record.root.clone();
    let options = BackupOptions::from_settings();
    tokio::task::spawn_blocking(move || create_zip(&root, &manifest, &destination, &options)).await??;
    Ok(file_name)
}

#[derive(Debug, Clone)]
pub struct BackupOptions {
    pub compression_level: u32,
    pub exclusions: Vec<String>,
}

impl BackupOptions {
    pub fn from_settings() -> Self {
        let backup = crate::launcher_settings::current().backup;
        Self {
            compression_level: backup.compression_level,
            exclusions: backup.exclusions,
        }
    }
}

/// Content that can be downloaded again and would only bloat every backup.
const ALWAYS_SKIPPED: &[&str] = &["runtime", "libraries", "versions", "session.lock", "*.lock", "server.json.tmp"];

fn matches(pattern: &str, relative: &str) -> bool {
    let pattern = pattern.trim().trim_matches('/').replace('\\', "/");
    if pattern.is_empty() {
        return false;
    }
    if let Some(extension) = pattern.strip_prefix("*.") {
        return relative.rsplit('/').next().is_some_and(|name| name.to_lowercase().ends_with(&format!(".{}", extension.to_lowercase())));
    }
    relative == pattern || relative.starts_with(&format!("{pattern}/"))
}

fn excluded(relative: &str, is_root_file: bool, options: &BackupOptions) -> bool {
    // Server jars at the root are re-downloaded on install; plugin and mod jars are kept.
    if is_root_file && relative.to_lowercase().ends_with(".jar") {
        return true;
    }
    ALWAYS_SKIPPED
        .iter()
        .map(|pattern| pattern.to_string())
        .chain(options.exclusions.iter().cloned())
        .any(|pattern| matches(&pattern, relative))
}

pub fn create_zip(server_root: &Path, manifest: &BackupManifest, destination: &Path, options: &BackupOptions) -> PanelResult<()> {
    if let Some(parent) = destination.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let temp = destination.with_extension("zip.part");
    let file = std::fs::File::create(&temp)?;
    let mut zip = ZipWriter::new(file);
    let level = options.compression_level.min(9);
    let zip_options = if level == 0 {
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
    } else {
        SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(i64::from(level)))
    }
    .large_file(true);
    let body = serde_json::to_vec_pretty(manifest)?;
    zip.start_file("manifest.json", zip_options).map_err(zip_error)?;
    zip.write_all(&body)?;
    let backups_inside = destination.parent().filter(|parent| parent.starts_with(server_root)).map(Path::to_path_buf);
    let mut stack = vec![server_root.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)?.flatten() {
            let path = entry.path();
            if backups_inside.as_ref().is_some_and(|inside| path.starts_with(inside)) {
                continue;
            }
            let relative = relative_name(server_root, &path)?;
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            if excluded(&relative, !is_dir && current == server_root, options) {
                continue;
            }
            if is_dir {
                stack.push(path);
            } else {
                add_file(&mut zip, &relative, &path, zip_options)?;
            }
        }
    }
    zip.finish().map_err(zip_error)?;
    std::fs::rename(&temp, destination)?;
    Ok(())
}

fn zip_error(error: zip::result::ZipError) -> PanelError {
    PanelError::io(error.to_string())
}

fn add_file(zip: &mut ZipWriter<std::fs::File>, name: &str, path: &Path, options: SimpleFileOptions) -> PanelResult<()> {
    // Files locked by a running server (e.g. region files being written) are skipped, not fatal.
    let Ok(mut file) = std::fs::File::open(path) else {
        return Ok(());
    };
    zip.start_file(name, options).map_err(zip_error)?;
    std::io::copy(&mut file, zip)?;
    Ok(())
}

fn relative_name(root: &Path, path: &Path) -> PanelResult<String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| PanelError::invalid("Percorso fuori dalla cartella del server"))?;
    let name = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/");
    if name.is_empty() || name.split('/').any(|part| part == "..") {
        return Err(PanelError::invalid("Percorso backup non valido"));
    }
    Ok(name)
}

pub fn restore_zip(server_root: &Path, archive: &Path) -> PanelResult<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = ZipArchive::new(file).map_err(|error| PanelError::invalid(format!("Backup non valido: {error}")))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(zip_error)?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        if name.as_os_str() == "manifest.json" {
            continue;
        }
        let destination = server_root.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut output = std::fs::File::create(&destination)?;
            std::io::copy(&mut entry, &mut output)?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub fn list_zip_names(archive: &Path) -> PanelResult<Vec<String>> {
    let file = std::fs::File::open(archive)?;
    let mut zip = ZipArchive::new(file).map_err(zip_error)?;
    let mut names = Vec::new();
    for index in 0..zip.len() {
        let entry = zip.by_index(index).map_err(zip_error)?;
        if let Some(name) = entry.enclosed_name() {
            names.push(name.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(names)
}

fn modified(path: &Path) -> SystemTime {
    std::fs::metadata(path).and_then(|meta| meta.modified()).unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Backup archives, newest first.
pub fn backup_files(directory: &Path) -> PanelResult<Vec<PathBuf>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("zip") {
            files.push(path);
        }
    }
    files.sort_by_key(|path| std::cmp::Reverse(modified(path)));
    Ok(files)
}

/// Keeps the newest `retention` archives (0 keeps everything) and returns how many were removed.
pub fn prune(directory: &Path, retention: u32) -> PanelResult<usize> {
    if retention == 0 {
        return Ok(0);
    }
    let files = backup_files(directory)?;
    let mut removed = 0;
    for path in files.into_iter().skip(retention as usize) {
        std::fs::remove_file(path)?;
        removed += 1;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> BackupManifest {
        BackupManifest {
            mc_version: Some("1.21.1".into()),
            provider: Some("paper".into()),
            server_name: "Test".into(),
            created_unix: 10,
        }
    }

    #[test]
    fn backup_includes_content_and_skips_downloads() {
        let root = std::env::temp_dir().join(format!("voxel-backup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("world")).unwrap();
        std::fs::write(root.join("world").join("level.dat"), b"world").unwrap();
        std::fs::create_dir_all(root.join("plugins")).unwrap();
        std::fs::write(root.join("plugins").join("sample.jar"), b"plugin").unwrap();
        std::fs::create_dir_all(root.join("mods")).unwrap();
        std::fs::write(root.join("mods").join("mod.jar"), b"mod").unwrap();
        std::fs::write(root.join("server.properties"), b"motd=Ciao\n").unwrap();
        std::fs::create_dir_all(root.join("runtime").join("jdk").join("bin")).unwrap();
        std::fs::write(root.join("runtime").join("jdk").join("bin").join("java"), b"no").unwrap();
        std::fs::write(root.join("paper-1.21.1-1.jar"), b"jar").unwrap();
        std::fs::create_dir_all(root.join("logs")).unwrap();
        std::fs::write(root.join("logs").join("latest.log"), b"log").unwrap();
        std::fs::write(root.join("world").join("session.lock"), b"lock").unwrap();
        let destination = root.join("out").join("backup.zip");
        let options = BackupOptions { compression_level: 1, exclusions: vec!["logs".into()] };
        create_zip(&root, &manifest(), &destination, &options).unwrap();
        let names = list_zip_names(&destination).unwrap();
        for expected in ["manifest.json", "world/level.dat", "plugins/sample.jar", "mods/mod.jar", "server.properties"] {
            assert!(names.iter().any(|name| name == expected), "missing {expected}");
        }
        for skipped in ["runtime", "paper-", "latest.log", "session.lock"] {
            assert!(names.iter().all(|name| !name.contains(skipped)), "unexpected {skipped}");
        }

        std::fs::write(root.join("server.properties"), b"motd=Rotto\n").unwrap();
        restore_zip(&root, &destination).unwrap();
        let restored = std::fs::read_to_string(root.join("server.properties")).unwrap();
        assert!(restored.contains("motd=Ciao"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prunes_old_backups() {
        let directory = std::env::temp_dir().join(format!("voxel-prune-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        for index in 0..4 {
            std::fs::write(directory.join(format!("{index}.zip")), b"x").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(prune(&directory, 2).unwrap(), 2);
        let left: Vec<String> = backup_files(&directory)
            .unwrap()
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(left, vec!["3.zip", "2.zip"]);
        let _ = std::fs::remove_dir_all(&directory);
    }
}
