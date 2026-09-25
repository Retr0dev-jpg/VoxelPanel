// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct WorldEntry {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_ms: i64,
    pub active: bool,
}

const SKIP: &[&str] = &[
    "plugins",
    "runtime",
    "logs",
    "libraries",
    "versions",
    "cache",
    "config",
    "backups",
    "crash-reports",
];

pub fn list(root: &Path, active: Option<&str>) -> Vec<WorldEntry> {
    let mut worlds = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return worlds;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP.iter().any(|skip| name.eq_ignore_ascii_case(skip)) {
            continue;
        }
        if !path.join("level.dat").is_file() {
            continue;
        }
        worlds.push(WorldEntry {
            active: active == Some(name.as_str()),
            size_bytes: dir_size(&path),
            modified_ms: modified_ms(&path.join("level.dat")),
            name,
            path,
        });
    }
    worlds.sort_by(|left, right| left.name.cmp(&right.name));
    worlds
}

pub fn resolve(root: &Path, name: &str) -> crate::PanelResult<PathBuf> {
    if !crate::properties::valid_level_name(name) {
        return Err("Nome mondo non valido".into());
    }
    let path = root.join(name);
    if !path.join("level.dat").is_file() {
        return Err("Mondo non trovato".into());
    }
    Ok(path)
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

    #[test]
    fn finds_world_folders_with_level_dat() {
        let root = std::env::temp_dir().join(format!("voxel-worlds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("world")).unwrap();
        std::fs::write(root.join("world").join("level.dat"), b"x").unwrap();
        std::fs::create_dir_all(root.join("plugins")).unwrap();
        std::fs::write(root.join("plugins").join("level.dat"), b"no").unwrap();
        let worlds = list(&root, Some("world"));
        assert_eq!(worlds.len(), 1);
        assert!(worlds[0].active);
        assert_eq!(worlds[0].name, "world");
        let _ = std::fs::remove_dir_all(&root);
    }
}
