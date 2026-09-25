// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use serde_json::Value;

use crate::api::settings::{LauncherSettings, ManagedFolder, UpdateInfo};
use crate::paths::Layout;
use crate::{PanelError, PanelResult};

pub const SCHEMA_VERSION: u32 = 1;
const RELEASES_URL: &str = "https://api.github.com/repos/Retr0dev-jpg/VoxelPanel/releases/latest";

fn cache() -> &'static RwLock<Option<LauncherSettings>> {
    static CACHE: OnceLock<RwLock<Option<LauncherSettings>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(None))
}

fn settings_path() -> PathBuf {
    crate::platform::app_data_root().join("settings.json")
}

pub fn current() -> LauncherSettings {
    if let Some(settings) = cache().read().unwrap_or_else(|error| error.into_inner()).as_ref() {
        return settings.clone();
    }
    let loaded = load(&settings_path());
    *cache().write().unwrap_or_else(|error| error.into_inner()) = Some(loaded.clone());
    loaded
}

/// A broken file is kept as `settings.json.bak` and the defaults are used instead.
fn load(path: &Path) -> LauncherSettings {
    let Ok(text) = std::fs::read_to_string(path) else {
        return with_schema(LauncherSettings::default());
    };
    match parse(&text) {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!("settings.json non valido, uso i valori predefiniti: {error}");
            let _ = std::fs::copy(path, path.with_extension("json.bak"));
            with_schema(LauncherSettings::default())
        }
    }
}

fn with_schema(mut settings: LauncherSettings) -> LauncherSettings {
    settings.schema_version = SCHEMA_VERSION;
    settings
}

pub fn parse(text: &str) -> PanelResult<LauncherSettings> {
    let value: Value = serde_json::from_str(text)?;
    let migrated = migrate(value)?;
    let mut settings: LauncherSettings = serde_json::from_value(migrated)?;
    normalize(&mut settings);
    Ok(settings)
}

/// Upgrades older files step by step. Files from a newer VoxelPanel are refused
/// instead of being silently downgraded.
pub fn migrate(mut value: Value) -> PanelResult<Value> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| PanelError::invalid("Il file delle impostazioni deve contenere un oggetto JSON."))?;
    let version = object.get("schema_version").and_then(Value::as_u64).unwrap_or(0) as u32;
    if version > SCHEMA_VERSION {
        return Err(PanelError::invalid(format!(
            "Impostazioni create da una versione più recente di VoxelPanel (schema {version})."
        )));
    }
    if version < 1 {
        // Schema 0 had a flat `stop_timeout` next to the sections.
        if let Some(timeout) = object.remove("stop_timeout") {
            let general = object.entry("general").or_insert_with(|| Value::Object(Default::default()));
            if let Some(general) = general.as_object_mut() {
                general.entry("stop_timeout_secs").or_insert(timeout);
            }
        }
    }
    object.insert("schema_version".into(), Value::from(SCHEMA_VERSION));
    Ok(value)
}

/// Clamps numeric values into supported ranges so a hand-edited file cannot break the app.
fn normalize(settings: &mut LauncherSettings) {
    settings.schema_version = SCHEMA_VERSION;
    let general = &mut settings.general;
    general.stop_timeout_secs = general.stop_timeout_secs.clamp(5, 600);
    if !matches!(general.language.as_str(), "system" | "it" | "en") {
        general.language = "system".into();
    }
    let appearance = &mut settings.appearance;
    appearance.text_scale = appearance.text_scale.clamp(0.8, 1.6);
    appearance.accent_color |= 0xFF00_0000;
    let console = &mut settings.console;
    console.max_lines = console.max_lines.clamp(200, 50_000);
    console.font_size = console.font_size.clamp(9.0, 24.0);
    let backup = &mut settings.backup;
    backup.retention = backup.retention.min(1000);
    backup.compression_level = backup.compression_level.min(9);
    backup.exclusions.retain(|entry| !entry.trim().is_empty());
    let network = &mut settings.network;
    network.parallel_downloads = network.parallel_downloads.clamp(1, 16);
    network.timeout_secs = network.timeout_secs.clamp(5, 300);
    settings.defaults.port = settings.defaults.port.clamp(1024, 65535);
}

pub fn validate(settings: &LauncherSettings) -> PanelResult<()> {
    for value in [&settings.defaults.ram_min, &settings.defaults.ram_max] {
        if !value.is_empty() && !crate::ram::is_memory_value(value) {
            return Err(PanelError::invalid(format!("RAM predefinita non valida: {value}")));
        }
    }
    let proxy = settings.network.proxy.trim();
    if !proxy.is_empty() && reqwest::Proxy::all(proxy).is_err() {
        return Err(PanelError::invalid(format!("Proxy non valido: {proxy}")));
    }
    let paths = &settings.paths;
    for value in [&paths.servers_dir, &paths.backups_dir, &paths.runtimes_dir, &paths.cache_dir] {
        if !value.trim().is_empty() && !Path::new(value.trim()).is_absolute() {
            return Err(PanelError::invalid(format!("Il percorso deve essere assoluto: {value}")));
        }
    }
    for preferred in &settings.java.preferred {
        if !crate::platform::java_executable(Path::new(&preferred.path)).is_file() {
            return Err(PanelError::not_found(format!("Runtime Java non trovato: {}", preferred.path)));
        }
    }
    Ok(())
}

pub fn save(mut settings: LauncherSettings) -> PanelResult<LauncherSettings> {
    normalize(&mut settings);
    validate(&settings)?;
    if settings.general.launch_at_startup != current().general.launch_at_startup {
        crate::platform::set_launch_at_startup(settings.general.launch_at_startup)?;
    }
    write(&settings_path(), &settings)?;
    *cache().write().unwrap_or_else(|error| error.into_inner()) = Some(settings.clone());
    crate::logging::apply(&settings.advanced);
    crate::net::reset_client();
    Ok(settings)
}

fn write(path: &Path, settings: &LauncherSettings) -> PanelResult<()> {
    if let Some(parent) = path.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let text = serde_json::to_string_pretty(settings)?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

pub fn export_json() -> PanelResult<String> {
    let mut settings = current();
    // The API key is personal: never write it into a file meant to be shared.
    settings.network.curseforge_api_key.clear();
    Ok(serde_json::to_string_pretty(&settings)?)
}

pub fn import(path: &Path) -> PanelResult<LauncherSettings> {
    let text = std::fs::read_to_string(path)?;
    let mut imported = parse(&text)?;
    if imported.network.curseforge_api_key.is_empty() {
        imported.network.curseforge_api_key = current().network.curseforge_api_key;
    }
    save(imported)
}

pub fn move_folder(folder: ManagedFolder, destination: &str) -> PanelResult<LauncherSettings> {
    let layout = Layout::app();
    let (old, default) = match folder {
        ManagedFolder::Servers => (layout.servers(), layout.root.join("servers")),
        ManagedFolder::Backups => (layout.backups_root(), layout.root.join("backups")),
        ManagedFolder::Runtimes => (layout.runtimes(), layout.root.join("runtimes")),
        ManagedFolder::Cache => (layout.cache(), layout.root.join("cache")),
    };
    let new = if destination.is_empty() { default.clone() } else { PathBuf::from(destination) };
    if !new.is_absolute() {
        return Err(PanelError::invalid("Il percorso deve essere assoluto."));
    }
    if new.starts_with(&old) && !crate::platform::same_path(&new, &old) {
        return Err(PanelError::invalid("La nuova cartella non può stare dentro quella attuale."));
    }
    if !crate::platform::same_path(&old, &new) {
        move_tree(&old, &new)?;
        if matches!(folder, ManagedFolder::Servers | ManagedFolder::Runtimes) {
            rebase_records(&layout, &old, &new)?;
        }
    }
    let mut settings = current();
    let stored = if crate::platform::same_path(&new, &default) { String::new() } else { new.to_string_lossy().to_string() };
    match folder {
        ManagedFolder::Servers => settings.paths.servers_dir = stored,
        ManagedFolder::Backups => settings.paths.backups_dir = stored,
        ManagedFolder::Runtimes => {
            settings.paths.runtimes_dir = stored;
            for preferred in &mut settings.java.preferred {
                if let Some(rebased) = rebase(Path::new(&preferred.path), &old, &new) {
                    preferred.path = rebased.to_string_lossy().to_string();
                }
            }
        }
        ManagedFolder::Cache => settings.paths.cache_dir = stored,
    }
    save(settings)
}

fn rebase(path: &Path, old: &Path, new: &Path) -> Option<PathBuf> {
    path.strip_prefix(old).ok().map(|rest| new.join(rest))
}

/// Updates servers whose folder, jar or Java runtime lived under the moved folder.
fn rebase_records(layout: &Layout, old: &Path, new: &Path) -> PanelResult<()> {
    for mut record in crate::catalog::list(layout)? {
        let mut changed = false;
        if let Some(root) = rebase(&record.root, old, new) {
            record.root = root;
            changed = true;
        }
        if let Some(rebased) = record.java_home.as_deref().and_then(|home| rebase(home, old, new)) {
            record.java_home = Some(rebased);
            changed = true;
        }
        changed |= record.launch.rebase(old, new);
        if changed {
            crate::catalog::save(layout, &record)?;
        }
    }
    Ok(())
}

/// Renames when possible and falls back to copy + delete across file systems.
pub fn move_tree(from: &Path, to: &Path) -> PanelResult<()> {
    if !from.exists() {
        crate::paths::ensure_dir(to)?;
        return Ok(());
    }
    if to.exists() && std::fs::read_dir(to)?.next().is_some() {
        return Err(PanelError::invalid(format!("La cartella di destinazione non è vuota: {}", to.display())));
    }
    if let Some(parent) = to.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    if to.exists() {
        std::fs::remove_dir(to)?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_tree(from, to)?;
    std::fs::remove_dir_all(from)?;
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> PanelResult<()> {
    crate::paths::ensure_dir(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub async fn check_for_update(current_version: &str) -> PanelResult<Option<UpdateInfo>> {
    let release: Value = crate::net::http()
        .get(RELEASES_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(newer_release(current_version, &release))
}

pub fn newer_release(current_version: &str, release: &Value) -> Option<UpdateInfo> {
    let tag = release.get("tag_name")?.as_str()?;
    let version = tag.trim_start_matches('v');
    let current = current_version.split('+').next().unwrap_or(current_version);
    if crate::providers::compare_versions(version, current) != std::cmp::Ordering::Greater {
        return None;
    }
    Some(UpdateInfo {
        version: version.to_string(),
        url: release.get("html_url").and_then(Value::as_str).unwrap_or_default().to_string(),
        notes: release.get("body").and_then(Value::as_str).unwrap_or_default().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_fields_use_defaults() {
        let settings = parse(r#"{"schema_version": 1, "console": {"max_lines": 900}}"#).unwrap();
        assert_eq!(settings.console.max_lines, 900);
        assert_eq!(settings.general.stop_timeout_secs, 30);
        assert_eq!(settings.backup.compression_level, 6);
    }

    #[test]
    fn migrates_schema_zero() {
        let settings = parse(r#"{"stop_timeout": 45}"#).unwrap();
        assert_eq!(settings.schema_version, SCHEMA_VERSION);
        assert_eq!(settings.general.stop_timeout_secs, 45);
    }

    #[test]
    fn refuses_newer_schemas_and_clamps_values() {
        assert!(parse(r#"{"schema_version": 99}"#).is_err());
        let settings = parse(r#"{"console": {"max_lines": 5}, "network": {"parallel_downloads": 99}}"#).unwrap();
        assert_eq!(settings.console.max_lines, 200);
        assert_eq!(settings.network.parallel_downloads, 16);
    }

    #[test]
    fn validates_values() {
        let mut settings = LauncherSettings::default();
        settings.defaults.ram_max = "tanta".into();
        assert!(validate(&settings).is_err());
        let mut settings = LauncherSettings::default();
        settings.paths.backups_dir = "relativo".into();
        assert!(validate(&settings).is_err());
        assert!(validate(&LauncherSettings::default()).is_ok());
    }

    #[test]
    fn detects_newer_releases() {
        let release = json!({"tag_name": "v1.3.0", "html_url": "https://example.com", "body": "note"});
        assert_eq!(newer_release("1.2.9+4", &release).unwrap().version, "1.3.0");
        assert!(newer_release("1.3.0+1", &release).is_none());
    }

    #[test]
    fn moves_trees() {
        let root = std::env::temp_dir().join(format!("voxel-move-{}", uuid::Uuid::new_v4()));
        let from = root.join("a");
        std::fs::create_dir_all(from.join("inner")).unwrap();
        std::fs::write(from.join("inner").join("file.txt"), b"x").unwrap();
        let to = root.join("b");
        move_tree(&from, &to).unwrap();
        assert!(to.join("inner").join("file.txt").is_file());
        assert!(!from.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
