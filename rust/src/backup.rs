// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub paper_version: Option<String>,
    pub server_name: String,
    pub created_unix: i64,
}

const ROOT_FILES: &[&str] = &[
    "server.properties",
    "eula.txt",
    "whitelist.json",
    "ops.json",
    "banned-players.json",
    "banned-ips.json",
    "usercache.json",
    "bukkit.yml",
    "spigot.yml",
    "commands.yml",
    "permissions.yml",
    "help.yml",
    "wepif.yml",
    "paper.yml",
];

pub fn create_zip(server_root: &Path, manifest: &BackupManifest, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let file = std::fs::File::create(destination).map_err(|error| error.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let body = serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?;
    zip.start_file("manifest.json", options)
        .map_err(|error| error.to_string())?;
    zip.write_all(&body).map_err(|error| error.to_string())?;

    for name in ROOT_FILES {
        let path = server_root.join(name);
        if path.is_file() {
            add_file(&mut zip, server_root, &path, options)?;
        }
    }
    add_tree(&mut zip, server_root, &server_root.join("config"), options)?;
    add_tree(&mut zip, server_root, &server_root.join("plugins"), options)?;
    for world in crate::worlds::list(server_root, None) {
        add_tree(&mut zip, server_root, &world.path, options)?;
    }
    zip.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn add_tree(
    zip: &mut ZipWriter<std::fs::File>,
    root: &Path,
    directory: &Path,
    options: SimpleFileOptions,
) -> Result<(), String> {
    if !directory.is_dir() {
        return Ok(());
    }
    let mut stack = vec![directory.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current).map_err(|error| error.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if include_file(&path) {
                add_file(zip, root, &path, options)?;
            }
        }
    }
    Ok(())
}

fn include_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();
    !name.ends_with(".lock")
}

fn add_file(
    zip: &mut ZipWriter<std::fs::File>,
    root: &Path,
    path: &Path,
    options: SimpleFileOptions,
) -> Result<(), String> {
    let name = relative_name(root, path)?;
    zip.start_file(&name, options)
        .map_err(|error| error.to_string())?;
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    std::io::copy(&mut file, zip).map_err(|error| error.to_string())?;
    Ok(())
}

fn relative_name(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "Percorso fuori dalla cartella del server".to_string())?;
    let name = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/");
    if name.is_empty() || name.split('/').any(|part| part == "..") {
        return Err("Percorso backup non valido".into());
    }
    Ok(name)
}

pub fn restore_zip(server_root: &Path, archive: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|error| error.to_string())?;
    let mut zip = ZipArchive::new(file).map_err(|error| format!("Backup non valido: {error}"))?;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| error.to_string())?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        if name.components().next().and_then(|part| part.as_os_str().to_str()) == Some("manifest.json")
            && name.components().count() == 1
        {
            continue;
        }
        let destination = server_root.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
        } else {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut output = std::fs::File::create(&destination).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn list_zip_names(archive: &Path) -> Result<Vec<String>, String> {
    let file = std::fs::File::open(archive).map_err(|error| error.to_string())?;
    let mut zip = ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut names = Vec::new();
    for index in 0..zip.len() {
        let entry = zip.by_index(index).map_err(|error| error.to_string())?;
        if let Some(name) = entry.enclosed_name() {
            names.push(name.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(names)
}

pub fn backup_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("zip") {
            files.push(path);
        }
    }
    files.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_includes_world_and_skips_runtime_and_jar() {
        let root = std::env::temp_dir().join(format!("voxel-backup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("world")).unwrap();
        std::fs::write(root.join("world").join("level.dat"), b"world").unwrap();
        std::fs::create_dir_all(root.join("plugins")).unwrap();
        std::fs::write(root.join("plugins").join("sample.jar"), b"plugin").unwrap();
        std::fs::write(root.join("server.properties"), b"motd=Ciao\n").unwrap();
        std::fs::create_dir_all(root.join("runtime").join("jdk").join("bin")).unwrap();
        std::fs::write(root.join("runtime").join("jdk").join("bin").join("java.exe"), b"no").unwrap();
        std::fs::write(root.join("paper-1.21.1-1.jar"), b"jar").unwrap();
        std::fs::create_dir_all(root.join("logs")).unwrap();
        std::fs::write(root.join("logs").join("latest.log"), b"log").unwrap();
        let destination = root.join("out.zip");
        create_zip(
            &root,
            &BackupManifest {
                paper_version: Some("1.21.1".into()),
                server_name: "Test".into(),
                created_unix: 10,
            },
            &destination,
        )
        .unwrap();
        let names = list_zip_names(&destination).unwrap();
        assert!(names.iter().any(|name| name == "manifest.json"));
        assert!(names.iter().any(|name| name.replace('\\', "/") == "world/level.dat"));
        assert!(names.iter().any(|name| name.replace('\\', "/") == "plugins/sample.jar"));
        assert!(names.iter().any(|name| name == "server.properties"));
        assert!(names.iter().all(|name| !name.contains("runtime")));
        assert!(names.iter().all(|name| !name.contains("paper-")));
        assert!(names.iter().all(|name| !name.contains("latest.log")));

        std::fs::write(root.join("server.properties"), b"motd=Rotto\n").unwrap();
        restore_zip(&root, &destination).unwrap();
        let restored = std::fs::read_to_string(root.join("server.properties")).unwrap();
        assert!(restored.contains("motd=Ciao"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
