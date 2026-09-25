// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::api::types::WorldDimension;
use crate::{PanelError, PanelResult};

#[derive(Debug, Clone)]
pub struct WorldEntry {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_ms: i64,
    pub active: bool,
    pub dimension: WorldDimension,
    pub group: String,
}

const SKIP: &[&str] = &["plugins", "mods", "runtime", "logs", "libraries", "versions", "cache", "config", "backups", "crash-reports"];

/// Bukkit-based servers store dimensions as sibling folders (`world_nether`, `world_the_end`).
pub fn dimension_of(name: &str) -> (WorldDimension, String) {
    if let Some(base) = name.strip_suffix("_nether") {
        return (WorldDimension::Nether, base.to_string());
    }
    if let Some(base) = name.strip_suffix("_the_end") {
        return (WorldDimension::End, base.to_string());
    }
    (WorldDimension::Overworld, name.to_string())
}

pub fn list(root: &Path, active: Option<&str>) -> Vec<WorldEntry> {
    let mut worlds = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return worlds;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_dir() || SKIP.iter().any(|skip| name.eq_ignore_ascii_case(skip)) || !path.join("level.dat").is_file() {
            continue;
        }
        let (dimension, group) = dimension_of(&name);
        worlds.push(WorldEntry {
            active: active == Some(group.as_str()),
            size_bytes: dir_size(&path),
            modified_ms: modified_ms(&path.join("level.dat")),
            name,
            path,
            dimension,
            group,
        });
    }
    worlds.sort_by(|left, right| left.group.cmp(&right.group).then((left.dimension as u8).cmp(&(right.dimension as u8))));
    worlds
}

pub fn resolve(root: &Path, name: &str) -> PanelResult<PathBuf> {
    if !crate::properties::valid_level_name(name) {
        return Err(PanelError::invalid("Nome mondo non valido"));
    }
    let path = root.join(name);
    if !path.join("level.dat").is_file() {
        return Err(PanelError::not_found("Mondo non trovato"));
    }
    Ok(path)
}

/// Folders that make up a world: the overworld plus Bukkit dimension siblings.
pub fn world_folders(root: &Path, group: &str) -> Vec<PathBuf> {
    [group.to_string(), format!("{group}_nether"), format!("{group}_the_end")]
        .into_iter()
        .map(|name| root.join(name))
        .filter(|path| path.is_dir())
        .collect()
}

pub fn rename(root: &Path, name: &str, new_name: &str) -> PanelResult<()> {
    if !crate::properties::valid_level_name(new_name) || SKIP.iter().any(|skip| new_name.eq_ignore_ascii_case(skip)) {
        return Err(PanelError::invalid("Nuovo nome non valido"));
    }
    resolve(root, name)?;
    for folder in world_folders(root, name) {
        let suffix = folder.file_name().and_then(|file| file.to_str()).and_then(|file| file.strip_prefix(name)).unwrap_or("").to_string();
        let target = root.join(format!("{new_name}{suffix}"));
        if target.exists() {
            return Err(PanelError::invalid(format!("Esiste già {}", target.display())));
        }
        std::fs::rename(&folder, &target)?;
    }
    Ok(())
}

/// Deletes the world and its dimensions; the server generates a fresh one on next start.
pub fn reset(root: &Path, group: &str) -> PanelResult<()> {
    resolve(root, group)?;
    for folder in world_folders(root, group) {
        std::fs::remove_dir_all(folder)?;
    }
    Ok(())
}

/// Imports a world from a folder or a zip (the folder containing `level.dat` is used).
pub fn import(root: &Path, source: &Path, name: &str) -> PanelResult<()> {
    if !crate::properties::valid_level_name(name) {
        return Err(PanelError::invalid("Nome mondo non valido"));
    }
    let target = root.join(name);
    if target.exists() {
        return Err(PanelError::invalid("Esiste già un mondo con questo nome"));
    }
    if source.is_dir() {
        let world = find_level_dir(source).ok_or_else(|| PanelError::invalid("Nella cartella non c'è un level.dat"))?;
        return crate::server_files::copy_tree(&world, &target);
    }
    let file = std::fs::File::open(source)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| PanelError::invalid(format!("Zip non valido: {error}")))?;
    let prefix = (0..zip.len())
        .filter_map(|index| zip.by_index(index).ok().and_then(|entry| entry.enclosed_name()))
        .filter(|path| path.file_name().is_some_and(|file| file == "level.dat"))
        .min_by_key(|path| path.components().count())
        .map(|path| path.parent().map(Path::to_path_buf).unwrap_or_default())
        .ok_or_else(|| PanelError::invalid("Nello zip non c'è un level.dat"))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| PanelError::invalid(error.to_string()))?;
        let Some(path) = entry.enclosed_name() else { continue };
        let Ok(relative) = path.strip_prefix(&prefix) else { continue };
        let out = target.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
        }
    }
    Ok(())
}

fn find_level_dir(directory: &Path) -> Option<PathBuf> {
    if directory.join("level.dat").is_file() {
        return Some(directory.to_path_buf());
    }
    std::fs::read_dir(directory).ok()?.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).find_map(|path| {
        path.join("level.dat").is_file().then_some(path)
    })
}

/// Zips a world with its dimensions into `destination`.
pub fn backup(root: &Path, group: &str, destination: &Path) -> PanelResult<()> {
    let folders = world_folders(root, group);
    if folders.is_empty() {
        return Err(PanelError::not_found("Mondo non trovato"));
    }
    if let Some(parent) = destination.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let temp = destination.with_extension("zip.part");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&temp)?);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
    for folder in folders {
        let mut stack = vec![folder];
        while let Some(current) = stack.pop() {
            for entry in std::fs::read_dir(&current)?.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.file_name().is_some_and(|name| name == "session.lock") {
                    continue;
                }
                let name = path.strip_prefix(root).unwrap_or(&path).components().map(|part| part.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                let Ok(mut file) = std::fs::File::open(&path) else { continue };
                zip.start_file(name, options).map_err(|error| PanelError::io(error.to_string()))?;
                std::io::copy(&mut file, &mut zip)?;
            }
        }
    }
    zip.flush()?;
    zip.finish().map_err(|error| PanelError::io(error.to_string()))?;
    std::fs::rename(temp, destination)?;
    Ok(())
}

pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

fn modified_ms(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(root: &Path, name: &str) {
        std::fs::create_dir_all(root.join(name)).unwrap();
        std::fs::write(root.join(name).join("level.dat"), b"x").unwrap();
    }

    #[test]
    fn groups_bukkit_dimensions() {
        let root = std::env::temp_dir().join(format!("voxel-worlds-{}", uuid::Uuid::new_v4()));
        world(&root, "world");
        world(&root, "world_nether");
        world(&root, "world_the_end");
        world(&root, "plugins");
        let worlds = list(&root, Some("world"));
        assert_eq!(worlds.len(), 3);
        assert!(worlds.iter().all(|entry| entry.group == "world" && entry.active));
        assert_eq!(worlds[1].dimension, WorldDimension::Nether);
        rename(&root, "world", "survival").unwrap();
        assert!(root.join("survival_nether").is_dir());
        let backup_file = root.join("backup.zip");
        backup(&root, "survival", &backup_file).unwrap();
        import(&root, &backup_file, "copy").unwrap();
        assert!(root.join("copy").join("level.dat").is_file());
        reset(&root, "survival").unwrap();
        assert!(!root.join("survival_the_end").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
