// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Plugins and mods: jar files in `plugins/` or `mods/`, toggled by a `.disabled` suffix.

use std::path::{Path, PathBuf};

use crate::api::types::{AddonKind, ProviderKind};
use crate::{PanelError, PanelResult, ServerRecord};

#[derive(Debug, Clone)]
pub struct AddonEntry {
    pub file_name: String,
    pub enabled: bool,
    pub size_bytes: u64,
    pub metadata: Option<crate::content::metadata::JarMetadata>,
}

pub fn folder(kind: AddonKind) -> &'static str {
    match kind {
        AddonKind::Plugin => "plugins",
        AddonKind::Mod => "mods",
    }
}

/// Modrinth loader tags accepted by this server for plugins or mods.
pub fn loaders(record: &ServerRecord, kind: AddonKind) -> Vec<&'static str> {
    let bukkit = vec!["paper", "purpur", "spigot", "bukkit", "folia"];
    match (kind, record.provider) {
        (AddonKind::Plugin, ProviderKind::Folia) => vec!["folia"],
        (AddonKind::Plugin, ProviderKind::Spigot) => vec!["spigot", "bukkit"],
        (AddonKind::Plugin, ProviderKind::Velocity) => vec!["velocity"],
        (AddonKind::Plugin, ProviderKind::BungeeCord | ProviderKind::Waterfall) => vec!["bungeecord", "waterfall"],
        (AddonKind::Plugin, ProviderKind::SpongeVanilla | ProviderKind::SpongeForge) => vec!["sponge"],
        (AddonKind::Plugin, _) => bukkit,
        (AddonKind::Mod, ProviderKind::Fabric) => vec!["fabric"],
        (AddonKind::Mod, ProviderKind::Quilt) => vec!["quilt", "fabric"],
        (AddonKind::Mod, ProviderKind::NeoForge) => vec!["neoforge"],
        (AddonKind::Mod, ProviderKind::Forge | ProviderKind::SpongeForge | ProviderKind::Mohist) => vec!["forge"],
        (AddonKind::Mod, ProviderKind::Arclight) => {
            let build = record.build.as_deref().unwrap_or_default();
            if build.contains("neoforge") {
                vec!["neoforge"]
            } else if build.contains("fabric") {
                vec!["fabric"]
            } else {
                vec!["forge"]
            }
        }
        (AddonKind::Mod, _) => vec!["fabric", "forge", "neoforge", "quilt"],
    }
}

pub fn list(root: &Path, kind: AddonKind) -> Vec<AddonEntry> {
    let mut items = Vec::new();
    let Ok(entries) = std::fs::read_dir(root.join(folder(kind))) else {
        return items;
    };
    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        let enabled = lower.ends_with(".jar");
        if !enabled && !lower.ends_with(".jar.disabled") {
            continue;
        }
        items.push(AddonEntry {
            size_bytes: entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            metadata: crate::content::metadata::read(&entry.path()),
            file_name: name,
            enabled,
        });
    }
    items.sort_by_key(|item| item.file_name.to_lowercase());
    items
}

pub fn set_enabled(root: &Path, kind: AddonKind, file_name: &str, enabled: bool) -> PanelResult<()> {
    let current = addon_path(root, kind, file_name)?;
    let target_name = if enabled {
        file_name.trim_end_matches(".disabled").to_string()
    } else if file_name.to_lowercase().ends_with(".jar.disabled") {
        file_name.to_string()
    } else {
        format!("{file_name}.disabled")
    };
    if target_name == file_name {
        return Ok(());
    }
    let target = root.join(folder(kind)).join(&target_name);
    if target.exists() {
        return Err(PanelError::invalid("Esiste già un file con questo nome"));
    }
    std::fs::rename(current, target)?;
    Ok(())
}

pub fn delete(root: &Path, kind: AddonKind, file_name: &str) -> PanelResult<()> {
    std::fs::remove_file(addon_path(root, kind, file_name)?)?;
    Ok(())
}

pub fn install_file(root: &Path, kind: AddonKind, source: &Path) -> PanelResult<()> {
    if !source.is_file() {
        return Err(PanelError::not_found("File non trovato"));
    }
    let name = source.file_name().and_then(|name| name.to_str()).ok_or("Nome file non valido")?;
    if !name.to_lowercase().ends_with(".jar") || name.contains(['\\', '/', ':']) {
        return Err(PanelError::invalid("Il file deve essere un .jar"));
    }
    let directory = root.join(folder(kind));
    crate::paths::ensure_dir(&directory)?;
    let destination = directory.join(name);
    if source != destination {
        std::fs::copy(source, &destination)?;
    }
    Ok(())
}

fn addon_path(root: &Path, kind: AddonKind, file_name: &str) -> PanelResult<PathBuf> {
    if file_name.contains(['\\', '/', ':']) || file_name.contains("..") {
        return Err(PanelError::invalid("Nome file non valido"));
    }
    let path = root.join(folder(kind)).join(file_name);
    if !path.is_file() {
        return Err(PanelError::not_found("File non trovato"));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_enabled_and_disabled_jars() {
        let root = std::env::temp_dir().join(format!("voxel-addons-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("mods")).unwrap();
        std::fs::write(root.join("mods").join("alpha.jar"), b"a").unwrap();
        std::fs::write(root.join("mods").join("beta.jar.disabled"), b"b").unwrap();
        std::fs::write(root.join("mods").join("note.txt"), b"c").unwrap();
        let mods = list(&root, AddonKind::Mod);
        assert_eq!(mods.len(), 2);
        assert!(list(&root, AddonKind::Plugin).is_empty());
        set_enabled(&root, AddonKind::Mod, "alpha.jar", false).unwrap();
        assert!(root.join("mods").join("alpha.jar.disabled").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn picks_loaders_for_each_server() {
        let mut record = ServerRecord::new("a".into(), "a".into(), "srv".into());
        record.provider = ProviderKind::Quilt;
        assert_eq!(loaders(&record, AddonKind::Mod), vec!["quilt", "fabric"]);
        record.provider = ProviderKind::Velocity;
        assert_eq!(loaders(&record, AddonKind::Plugin), vec!["velocity"]);
        record.provider = ProviderKind::Arclight;
        record.build = Some("arclight-neoforge-1.21.1-1.0.1.jar".into());
        assert_eq!(loaders(&record, AddonKind::Mod), vec!["neoforge"]);
    }

}
