// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Vanilla,
    #[default]
    Paper,
    Folia,
    Purpur,
    Pufferfish,
    Leaf,
    Spigot,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    Velocity,
    BungeeCord,
    Waterfall,
    Mohist,
    Arclight,
    SpongeVanilla,
    SpongeForge,
    /// A jar chosen by the user that VoxelPanel does not know how to update.
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderCategory {
    Vanilla,
    Plugins,
    Modded,
    Proxy,
    Hybrid,
    Other,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub kind: ProviderKind,
    pub id: String,
    pub name: String,
    pub category: ProviderCategory,
    pub description: String,
    pub supports_plugins: bool,
    pub supports_mods: bool,
    pub has_worlds: bool,
    pub is_proxy: bool,
    /// Offers snapshot / pre-release versions.
    pub has_snapshots: bool,
    /// Lets the user pick a specific build or loader version.
    pub has_builds: bool,
    pub deprecated: bool,
    /// Extra requirement shown in the wizard (e.g. Git for BuildTools).
    pub note: String,
    /// Config files worth editing for this server type, relative to the server folder.
    pub config_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VersionEntry {
    pub id: String,
    pub stable: bool,
}

#[derive(Debug, Clone)]
pub struct BuildEntry {
    pub id: String,
    pub stable: bool,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
}

#[derive(Debug, Clone)]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
    pub root: String,
    pub provider: ProviderKind,
    pub mc_version: Option<String>,
    pub build: Option<String>,
    pub java_major: Option<u32>,
    pub ram_min: String,
    pub ram_max: String,
    pub port: u32,
    pub max_players: u32,
}

#[derive(Debug, Clone)]
pub struct ServerDetails {
    pub id: String,
    pub name: String,
    pub root: String,
    pub provider: ProviderKind,
    pub mc_version: Option<String>,
    pub build: Option<String>,
    pub java_major: Option<u32>,
    pub java_home: String,
    pub jar_path: String,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    pub eula_accepted: bool,
    pub created_unix: i64,
    pub port: u32,
    pub max_players: u32,
    pub autostart: bool,
    pub auto_restart: bool,
    pub schedule_count: u32,
}

#[derive(Debug, Clone)]
pub struct RamChoice {
    pub megabytes: u32,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct RamSuggestion {
    pub ram_min: String,
    pub ram_max: String,
    pub total: String,
}

#[derive(Debug, Clone)]
pub struct JavaRuntimeInfo {
    pub name: String,
    pub major: u32,
    pub path: String,
    /// Only managed runtimes can be deleted from the launcher.
    pub managed: bool,
    pub system: bool,
    /// Read from the JDK `release` file; unknown for other distributions.
    pub vendor: Option<crate::api::settings::JavaVendor>,
}

#[derive(Debug, Clone)]
pub struct JavaReleaseInfo {
    pub major: u32,
    pub lts: bool,
}

#[derive(Debug, Clone)]
pub struct ImportPreview {
    pub name: String,
    pub root: String,
    pub provider: ProviderKind,
    pub mc_version: String,
    pub java_major: u32,
    pub java_home: String,
    pub jar_path: String,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    pub plugin_count: u32,
    pub mod_count: u32,
    pub world_count: u32,
    pub has_eula: bool,
}

#[derive(Debug, Clone)]
pub struct CreateServerRequest {
    pub name: String,
    /// Empty: a new folder inside the servers folder.
    pub root: String,
    pub provider: ProviderKind,
    pub mc_version: String,
    /// Empty: latest build.
    pub build: String,
    /// Local jar used instead of a download (provider `Custom` or a known jar).
    pub jar_path: String,
    /// Empty: required Java chosen automatically (preferred runtime or download).
    pub java_home: String,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    /// 0: first free port from the launcher settings.
    pub port: u32,
    pub motd: String,
    pub max_players: u32,
    pub gamemode: String,
    pub difficulty: String,
    pub online_mode: bool,
    pub level_seed: String,
    pub accept_eula: bool,
}

#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub stage: String,
    pub message: String,
    pub fraction: Option<f64>,
    pub done: bool,
    pub error: Option<String>,
    pub server_id: Option<String>,
}

/// Live state of one server, pushed by `watch_events` whenever something changes.
#[derive(Debug, Clone)]
pub struct ServerRuntime {
    pub server_id: String,
    pub status: ServerStatus,
    pub pid: Option<u32>,
    pub started_unix: Option<i64>,
    pub players: Vec<String>,
    pub cpu_percent: f64,
    pub memory_bytes: i64,
    pub last_exit_code: Option<i32>,
    pub crashed: bool,
    /// Ticks per second from RCON (`tps` or `tick query`); `None` when not measurable.
    pub tps: Option<f64>,
    /// Milliseconds per tick from RCON.
    pub mspt: Option<f64>,
    /// Size of the server folder, refreshed every minute while running.
    pub disk_bytes: Option<i64>,
    /// Automatic restarts after crashes in the current streak.
    pub restart_attempts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind {
    Restart,
    Command,
    Backup,
    Start,
    Stop,
}

/// A cron-scheduled action, stored in `server.json`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub kind: ScheduleKind,
    /// Standard 5-field cron expression (minute hour day month weekday), local time.
    pub cron: String,
    /// Console command for `Command` tasks.
    #[serde(default)]
    pub command: String,
    /// Restarts: warn players 5 minutes, 1 minute and 10 seconds before.
    #[serde(default)]
    pub warn_players: bool,
    pub enabled: bool,
    #[serde(default)]
    pub last_run_unix: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PropertyEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonKind {
    Plugin,
    Mod,
}

#[derive(Debug, Clone)]
pub struct AddonInfo {
    pub file_name: String,
    pub enabled: bool,
    pub size_bytes: i64,
    /// Read from `plugin.yml`, `fabric.mod.json`, `mods.toml`...; empty when unknown.
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentSourceKind {
    Modrinth,
    Hangar,
    Spiget,
    CurseForge,
}

#[derive(Debug, Clone)]
pub struct ContentProject {
    pub source: ContentSourceKind,
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: i64,
    pub icon_url: String,
    pub page_url: String,
}

#[derive(Debug, Clone)]
pub struct ContentPage {
    pub projects: Vec<ContentProject>,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct ContentVersion {
    pub id: String,
    pub name: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub published: String,
    pub stable: bool,
    /// Matches the server's loaders and Minecraft version.
    pub compatible: bool,
}

#[derive(Debug, Clone)]
pub struct AddonUpdate {
    pub file_name: String,
    pub project_id: String,
    pub current_version: String,
    pub new_version_id: String,
    pub new_version: String,
}

#[derive(Debug, Clone)]
pub struct ModpackRequest {
    pub name: String,
    pub root: String,
    /// `.mrpack` (Modrinth) or `.zip` with `manifest.json` (CurseForge); empty when using `source`.
    pub file_path: String,
    pub source: Option<ContentSourceKind>,
    pub project_id: String,
    pub version_id: String,
    pub java_home: String,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    pub accept_eula: bool,
}

#[derive(Debug, Clone)]
pub struct WorldInfo {
    pub name: String,
    pub dimension: WorldDimension,
    /// Overworld folder this world belongs to (`world` for `world_nether`).
    pub group: String,
    pub path: String,
    pub size_bytes: i64,
    pub modified_ms: i64,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub file_name: String,
    pub path: String,
    pub size_bytes: i64,
    pub created_ms: i64,
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data: String,
    pub servers: String,
    pub runtimes: String,
    pub backups: String,
    pub cache: String,
    pub logs: String,
}

/// Per-server options edited in the server settings section.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub java_home: String,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    /// Empty: `stop` (or `end` for proxies).
    pub stop_command: String,
    /// 0: use the launcher-wide timeout.
    pub stop_timeout_secs: u32,
    pub autostart: bool,
    pub auto_restart: bool,
    pub manage_rcon: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    Boolean,
    Integer,
    Text,
    Choice,
    Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyGroup {
    General,
    Gameplay,
    World,
    Network,
    Performance,
    Administration,
    QueryRcon,
    ResourcePack,
}

#[derive(Debug, Clone)]
pub struct PropertySchema {
    pub key: String,
    pub kind: PropertyKind,
    pub group: PropertyGroup,
    pub default_value: String,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub options: Vec<String>,
    pub description_it: String,
    pub description_en: String,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    /// Path relative to the server folder, with `/` separators.
    pub relative: String,
    pub is_dir: bool,
    pub size_bytes: i64,
    pub modified_ms: i64,
    pub editable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFileKind {
    Latest,
    Archive,
    Crash,
}

#[derive(Debug, Clone)]
pub struct LogFileInfo {
    pub name: String,
    pub relative: String,
    pub kind: LogFileKind,
    pub size_bytes: i64,
    pub modified_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerListKind {
    Whitelist,
    Operators,
    BannedPlayers,
    BannedIps,
}

#[derive(Debug, Clone)]
pub struct PlayerEntry {
    pub name: String,
    pub uuid: String,
    /// Ban reason or operator level.
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct IpBan {
    pub ip: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct PlayerLists {
    pub whitelist: Vec<PlayerEntry>,
    pub operators: Vec<PlayerEntry>,
    pub banned_players: Vec<PlayerEntry>,
    pub banned_ips: Vec<IpBan>,
    pub whitelist_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldDimension {
    Overworld,
    Nether,
    End,
}
