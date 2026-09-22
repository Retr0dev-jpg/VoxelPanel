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
    pub paper_version: Option<String>,
    pub java_major: Option<u32>,
    pub ram_min: String,
    pub ram_max: String,
    pub port: u32,
    pub status: ServerStatus,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ServerDetails {
    pub id: String,
    pub name: String,
    pub root: String,
    pub paper_version: Option<String>,
    pub java_major: Option<u32>,
    pub java_home: String,
    pub jar_path: String,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    pub eula_accepted: bool,
    pub created_unix: i64,
    pub status: ServerStatus,
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
pub struct JvmFlagChoice {
    pub flag: String,
    pub recommended: bool,
}

#[derive(Debug, Clone)]
pub struct JavaRuntimeInfo {
    pub name: String,
    pub major: u32,
    pub path: String,
    pub lts: bool,
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
    pub paper_version: String,
    pub java_major: u32,
    pub java_home: String,
    pub jar_path: String,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    pub plugin_count: u32,
    pub world_count: u32,
    pub has_eula: bool,
}

#[derive(Debug, Clone)]
pub struct AutoInstallRequest {
    pub name: String,
    pub root: String,
    pub paper_version: String,
    pub accept_eula: bool,
    pub ram_min: String,
    pub ram_max: String,
}

#[derive(Debug, Clone)]
pub struct ManualInstallRequest {
    pub name: String,
    pub root: String,
    pub paper_version: String,
    pub jar_path: String,
    pub java_major: u32,
    pub java_home: String,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
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

#[derive(Debug, Clone)]
pub struct ProcessStats {
    pub cpu_percent: f64,
    pub memory_bytes: i64,
    pub pid: u32,
}

#[derive(Debug, Clone)]
pub struct ServerSettings {
    pub motd: String,
    pub port: u32,
    pub max_players: u32,
    pub online_mode: bool,
    pub difficulty: String,
    pub gamemode: String,
    pub view_distance: u32,
    pub simulation_distance: u32,
    pub white_list: bool,
    pub pvp: bool,
    pub spawn_protection: u32,
    pub level_name: String,
    pub level_seed: String,
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub file_name: String,
    pub enabled: bool,
    pub size_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct ModrinthProject {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: i64,
}

#[derive(Debug, Clone)]
pub struct WorldInfo {
    pub name: String,
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
