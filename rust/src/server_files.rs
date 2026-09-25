// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! File manager confined to a server folder.

use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use crate::api::types::FileEntry;
use crate::{PanelError, PanelResult, ServerRecord};

/// Largest file the text editor opens; bigger files are usually logs or data.
pub const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

const TEXT_EXTENSIONS: &[&str] = &[
    "properties", "yml", "yaml", "toml", "json", "json5", "conf", "cfg", "txt", "log", "md", "ini", "sh", "bat", "cmd", "mcmeta", "snbt", "csv", "xml",
];

/// Resolves `relative` inside `root`, refusing absolute paths, `..` and symlinks that escape.
pub fn resolve(root: &Path, relative: &str) -> PanelResult<PathBuf> {
    let relative = relative.trim().replace('\\', "/");
    let mut path = root.to_path_buf();
    for component in Path::new(relative.trim_start_matches('/')).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            _ => return Err(PanelError::invalid("Percorso non consentito")),
        }
    }
    let canonical_root = std::fs::canonicalize(root)?;
    let mut existing = path.as_path();
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| PanelError::invalid("Percorso non consentito"))?;
    }
    if !std::fs::canonicalize(existing)?.starts_with(&canonical_root) {
        return Err(PanelError::invalid("Percorso fuori dalla cartella del server"));
    }
    Ok(path)
}

fn relative_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).components().map(|part| part.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

pub fn is_text(path: &Path) -> bool {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_lowercase();
    name == "eula.txt" || TEXT_EXTENSIONS.iter().any(|extension| name.ends_with(&format!(".{extension}")))
}

fn modified_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}

pub fn list(root: &Path, relative: &str) -> PanelResult<Vec<FileEntry>> {
    let directory = resolve(root, relative)?;
    let mut entries: Vec<FileEntry> = std::fs::read_dir(&directory)?
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            let path = entry.path();
            Some(FileEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                relative: relative_of(root, &path),
                is_dir: meta.is_dir(),
                size_bytes: if meta.is_dir() { 0 } else { meta.len() as i64 },
                modified_ms: modified_ms(&meta),
                editable: meta.is_file() && is_text(&path) && meta.len() <= MAX_TEXT_BYTES,
            })
        })
        .collect();
    entries.sort_by(|left, right| right.is_dir.cmp(&left.is_dir).then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase())));
    Ok(entries)
}

pub fn read_text(root: &Path, relative: &str) -> PanelResult<String> {
    let path = resolve(root, relative)?;
    let meta = std::fs::metadata(&path)?;
    if meta.len() > MAX_TEXT_BYTES {
        return Err(PanelError::invalid("File troppo grande per l'editor (oltre 2 MB)"));
    }
    let bytes = std::fs::read(&path)?;
    if bytes.contains(&0) {
        return Err(PanelError::invalid("Il file non è di testo"));
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Atomic write: a crash while saving never leaves a half-written config.
pub fn write_text(root: &Path, relative: &str, content: &str) -> PanelResult<()> {
    let path = resolve(root, relative)?;
    if path.is_dir() {
        return Err(PanelError::invalid("È una cartella"));
    }
    if let Some(parent) = path.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let temp = path.with_extension("voxel-tmp");
    std::fs::write(&temp, content)?;
    std::fs::rename(&temp, &path)?;
    Ok(())
}

pub fn delete(root: &Path, relative: &str) -> PanelResult<()> {
    let path = resolve(root, relative)?;
    if crate::platform::same_path(&path, root) {
        return Err(PanelError::invalid("Non si può eliminare la cartella del server"));
    }
    if path.is_dir() {
        std::fs::remove_dir_all(&path)?;
    } else {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn create_directory(root: &Path, relative: &str) -> PanelResult<()> {
    let path = resolve(root, relative)?;
    std::fs::create_dir_all(path)?;
    Ok(())
}

pub fn rename(root: &Path, from: &str, to: &str) -> PanelResult<()> {
    let source = resolve(root, from)?;
    let target = resolve(root, to)?;
    if target.exists() {
        return Err(PanelError::invalid("Esiste già un file con questo nome"));
    }
    std::fs::rename(source, target)?;
    Ok(())
}

pub fn copy_tree(from: &Path, to: &Path) -> PanelResult<()> {
    if from.is_file() {
        if let Some(parent) = to.parent() {
            crate::paths::ensure_dir(parent)?;
        }
        std::fs::copy(from, to)?;
        return Ok(());
    }
    crate::paths::ensure_dir(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        copy_tree(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

/// Copies files or folders from outside into `relative_dir`.
pub fn import(root: &Path, relative_dir: &str, sources: &[String]) -> PanelResult<u32> {
    let directory = resolve(root, relative_dir)?;
    crate::paths::ensure_dir(&directory)?;
    let mut copied = 0;
    for source in sources {
        let source = PathBuf::from(source);
        let name = source.file_name().ok_or_else(|| PanelError::invalid("Nome file non valido"))?;
        copy_tree(&source, &directory.join(name))?;
        copied += 1;
    }
    Ok(copied)
}

/// Copies a file or folder of the server into `destination_dir` (outside the server).
pub fn export(root: &Path, relative: &str, destination_dir: &Path) -> PanelResult<PathBuf> {
    let source = resolve(root, relative)?;
    let name = source.file_name().ok_or_else(|| PanelError::invalid("Nome file non valido"))?;
    let target = destination_dir.join(name);
    copy_tree(&source, &target)?;
    Ok(target)
}

/// Config files worth a shortcut: those of the server type plus everything in `config/`.
pub fn config_files(record: &ServerRecord) -> Vec<String> {
    let root = &record.root;
    let mut files: Vec<String> = crate::providers::meta(record.provider)
        .config_files
        .into_iter()
        .filter(|relative| root.join(relative).is_file())
        .collect();
    let mut stack = vec![root.join("config")];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if relative_of(root, &path).matches('/').count() < 3 {
                    stack.push(path);
                }
            } else if is_text(&path) {
                let relative = relative_of(root, &path);
                if !files.contains(&relative) {
                    files.push(relative);
                }
            }
        }
    }
    for extra in ["server.properties", "eula.txt", "velocity.toml", "config.yml", "waterfall.yml", "purpur.yml", "pufferfish.yml"] {
        if root.join(extra).is_file() && !files.iter().any(|file| file == extra) {
            files.push(extra.to_string());
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_paths_inside_the_server() {
        let root = std::env::temp_dir().join(format!("voxel-files-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).unwrap();
        assert!(resolve(&root, "config/new.yml").is_ok());
        assert!(resolve(&root, "../outside").is_err());
        assert!(resolve(&root, "/etc/passwd").unwrap().starts_with(&root));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(std::env::temp_dir(), root.join("escape")).unwrap();
            assert!(resolve(&root, "escape/file").is_err());
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn edits_lists_and_protects_the_root() {
        let root = std::env::temp_dir().join(format!("voxel-files-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        write_text(&root, "config/paper-global.yml", "a: 1\n").unwrap();
        assert_eq!(read_text(&root, "config/paper-global.yml").unwrap(), "a: 1\n");
        let entries = list(&root, "config").unwrap();
        assert_eq!(entries[0].relative, "config/paper-global.yml");
        assert!(entries[0].editable);
        rename(&root, "config/paper-global.yml", "config/renamed.yml").unwrap();
        assert!(delete(&root, "").is_err());
        delete(&root, "config").unwrap();
        assert!(!root.join("config").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
