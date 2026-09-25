// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use serde::{Deserialize, Serialize};

use crate::launcher_settings;
use crate::paths::Layout;
use crate::{PanelError, PanelResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    #[default]
    Ask,
    StopAndExit,
    MinimizeToTray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JvmPreset {
    #[default]
    Aikar,
    G1,
    Zgc,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppLogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    /// "system", "it" or "en".
    pub language: String,
    pub check_updates: bool,
    pub launch_at_startup: bool,
    pub close_behavior: CloseBehavior,
    pub autostart_servers: bool,
    pub stop_timeout_secs: u32,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "system".into(),
            check_updates: true,
            launch_at_startup: false,
            close_behavior: CloseBehavior::Ask,
            autostart_servers: true,
            stop_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: ThemePreference,
    /// ARGB, as used by Flutter's `Color`.
    pub accent_color: u32,
    pub text_scale: f64,
    pub compact: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            accent_color: 0xFF7C4DFF,
            text_scale: 1.0,
            compact: false,
        }
    }
}

/// Empty strings mean "use the default folder inside the data directory".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PathSettings {
    pub servers_dir: String,
    pub backups_dir: String,
    pub runtimes_dir: String,
    pub cache_dir: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreferredJava {
    pub major: u32,
    pub path: String,
}

/// Distribution used when VoxelPanel downloads a Java runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JavaVendor {
    #[default]
    Temurin,
    Zulu,
    Corretto,
    Microsoft,
    Liberica,
    SapMachine,
    GraalVm,
}

#[derive(Debug, Clone)]
pub struct JavaVendorInfo {
    pub vendor: JavaVendor,
    pub name: String,
    pub publisher: String,
    pub website: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct JavaSettings {
    pub preferred: Vec<PreferredJava>,
    pub vendor: JavaVendor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DefaultsSettings {
    /// Empty means "suggest from system memory".
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_preset: JvmPreset,
    pub port: u32,
    pub provider: String,
}

impl Default for DefaultsSettings {
    fn default() -> Self {
        Self {
            ram_min: String::new(),
            ram_max: String::new(),
            jvm_preset: JvmPreset::Aikar,
            port: 25565,
            provider: "paper".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConsoleSettings {
    pub max_lines: u32,
    pub timestamps: bool,
    pub font_size: f64,
    pub wrap: bool,
}

impl Default for ConsoleSettings {
    fn default() -> Self {
        Self {
            max_lines: 2000,
            timestamps: false,
            font_size: 13.0,
            wrap: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupSettings {
    /// Backups kept per server; 0 keeps all of them.
    pub retention: u32,
    pub compression_level: u32,
    /// Paths relative to the server folder, or `*.ext` patterns.
    pub exclusions: Vec<String>,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            retention: 10,
            compression_level: 6,
            exclusions: vec!["cache".into(), "logs".into(), "*.lock".into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkSettings {
    pub proxy: String,
    pub parallel_downloads: u32,
    pub timeout_secs: u32,
    pub curseforge_api_key: String,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            proxy: String::new(),
            parallel_downloads: 4,
            timeout_secs: 30,
            curseforge_api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationSettings {
    pub crash: bool,
    pub ready: bool,
    pub player_join: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            crash: true,
            ready: false,
            player_join: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedSettings {
    pub logging: bool,
    pub log_level: AppLogLevel,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            logging: true,
            log_level: AppLogLevel::Info,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LauncherSettings {
    pub schema_version: u32,
    pub general: GeneralSettings,
    pub appearance: AppearanceSettings,
    pub paths: PathSettings,
    pub java: JavaSettings,
    pub defaults: DefaultsSettings,
    pub console: ConsoleSettings,
    pub backup: BackupSettings,
    pub network: NetworkSettings,
    pub notifications: NotificationSettings,
    pub advanced: AdvancedSettings,
}

#[derive(Debug, Clone)]
pub struct JvmPresetInfo {
    pub preset: JvmPreset,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedFolder {
    Servers,
    Backups,
    Runtimes,
    Cache,
}

pub async fn get_launcher_settings() -> LauncherSettings {
    launcher_settings::current()
}

pub async fn save_launcher_settings(settings: LauncherSettings) -> PanelResult<LauncherSettings> {
    launcher_settings::save(settings)
}

pub async fn reset_launcher_settings() -> PanelResult<LauncherSettings> {
    launcher_settings::save(LauncherSettings::default())
}

/// JSON of the current settings without the CurseForge API key.
pub async fn export_launcher_settings() -> PanelResult<String> {
    launcher_settings::export_json()
}

pub async fn import_launcher_settings(path: String) -> PanelResult<LauncherSettings> {
    launcher_settings::import(std::path::Path::new(&path))
}

/// Moves the content of a managed folder and points the settings at `destination`.
/// Servers that reference moved Java runtimes are updated too.
pub async fn move_managed_folder(folder: ManagedFolder, destination: String) -> PanelResult<LauncherSettings> {
    if crate::process::any_running() {
        return Err(PanelError::running());
    }
    let destination = destination.trim().to_string();
    tokio::task::spawn_blocking(move || launcher_settings::move_folder(folder, &destination)).await?
}

pub async fn clear_cache() -> PanelResult<()> {
    let cache = Layout::app().cache();
    if cache.exists() {
        tokio::fs::remove_dir_all(&cache).await?;
    }
    Ok(())
}

#[flutter_rust_bridge::frb(sync)]
pub fn jvm_presets() -> Vec<JvmPresetInfo> {
    [JvmPreset::Aikar, JvmPreset::G1, JvmPreset::Zgc, JvmPreset::None]
        .into_iter()
        .map(|preset| JvmPresetInfo {
            preset,
            flags: crate::jvm::preset_flags(preset),
        })
        .collect()
}

#[flutter_rust_bridge::frb(sync)]
pub fn java_vendors() -> Vec<JavaVendorInfo> {
    crate::java_runtime::VENDORS
        .iter()
        .map(|&vendor| {
            let meta = crate::java_runtime::meta(vendor);
            JavaVendorInfo {
                vendor,
                name: meta.name.into(),
                publisher: meta.publisher.into(),
                website: meta.website.into(),
            }
        })
        .collect()
}

#[flutter_rust_bridge::frb(sync)]
pub fn app_log_dir() -> String {
    Layout::app().logs().to_string_lossy().to_string()
}

pub async fn check_for_update(current_version: String) -> PanelResult<Option<UpdateInfo>> {
    launcher_settings::check_for_update(&current_version).await
}

pub async fn delete_java_runtime(path: String) -> PanelResult<()> {
    let path = std::path::PathBuf::from(path);
    let runtimes = Layout::app().runtimes();
    let top = path
        .ancestors()
        .find(|ancestor| ancestor.parent().is_some_and(|parent| crate::platform::same_path(parent, &runtimes)))
        .ok_or_else(|| PanelError::invalid("Si possono eliminare solo i runtime installati da VoxelPanel."))?
        .to_path_buf();
    let in_use = crate::catalog::list(&Layout::app())?
        .into_iter()
        .filter_map(|record| record.java_home)
        .any(|home| home.starts_with(&top));
    if in_use {
        return Err(PanelError::invalid("Questo runtime è usato da almeno un server."));
    }
    tokio::fs::remove_dir_all(top).await?;
    Ok(())
}
