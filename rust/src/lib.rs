// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

pub mod api;
mod backup;
mod cache;
mod catalog;
mod frb_generated;
mod install;
mod java_runtime;
mod jvm;
mod launcher_settings;
mod logging;
mod net;
mod paths;
mod platform;
mod addons;
mod process;
mod players;
mod properties;
mod properties_schema;
mod rcon;
mod providers;
mod ram;
mod scan;
mod server_files;
mod server_logs;
mod script;
mod worlds;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

pub use api::error::{ErrorCode, PanelError, PanelResult};

pub use api::types::ProviderKind;

pub const RECORD_SCHEMA: u32 = 2;

/// What goes after the JVM options on the command line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LaunchSpec {
    #[default]
    Unset,
    /// `-jar <path>`
    Jar { path: PathBuf },
    /// `@<path>`: argument files written by the Forge and NeoForge installers.
    ArgsFile { path: PathBuf },
}

impl LaunchSpec {
    pub fn jar(&self) -> Option<&PathBuf> {
        match self {
            LaunchSpec::Jar { path } => Some(path),
            _ => None,
        }
    }

    pub fn target(&self) -> Option<&PathBuf> {
        match self {
            LaunchSpec::Jar { path } | LaunchSpec::ArgsFile { path } => Some(path),
            LaunchSpec::Unset => None,
        }
    }

    pub fn rebase(&mut self, old: &std::path::Path, new: &std::path::Path) -> bool {
        match self {
            LaunchSpec::Jar { path } | LaunchSpec::ArgsFile { path } => match path.strip_prefix(old) {
                Ok(rest) => {
                    *path = new.join(rest);
                    true
                }
                Err(_) => false,
            },
            LaunchSpec::Unset => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRecord {
    #[serde(default)]
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    #[serde(default)]
    pub provider: ProviderKind,
    #[serde(default)]
    pub mc_version: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
    pub java_major: Option<u32>,
    pub java_home: Option<PathBuf>,
    #[serde(default)]
    pub launch: LaunchSpec,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    pub eula_accepted: bool,
    pub created_unix: i64,
    /// Sent to stdin to stop the server; `None` uses `stop` (or `end` for proxies).
    #[serde(default)]
    pub stop_command: Option<String>,
    /// Overrides the launcher-wide stop timeout.
    #[serde(default)]
    pub stop_timeout_secs: Option<u32>,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub auto_restart: bool,
    /// Let VoxelPanel enable RCON on localhost for player lists and metrics.
    #[serde(default = "default_true")]
    pub manage_rcon: bool,
    #[serde(default)]
    pub rcon: Option<RconConfig>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RconConfig {
    pub port: u32,
    pub password: String,
}

impl ServerRecord {
    pub fn new(id: String, name: String, root: PathBuf) -> Self {
        Self {
            schema_version: RECORD_SCHEMA,
            id,
            name,
            root,
            provider: ProviderKind::Custom,
            mc_version: None,
            build: None,
            java_major: None,
            java_home: None,
            launch: LaunchSpec::Unset,
            ram_min: "2G".into(),
            ram_max: "4G".into(),
            jvm_flags: Vec::new(),
            eula_accepted: false,
            created_unix: crate::paths::unix_now(),
            stop_command: None,
            stop_timeout_secs: None,
            autostart: false,
            auto_restart: false,
            manage_rcon: true,
            rcon: None,
        }
    }

    pub fn stop_command(&self) -> String {
        self.stop_command
            .clone()
            .filter(|command| !command.trim().is_empty())
            .unwrap_or_else(|| if crate::providers::is_proxy(self.provider) { "end".into() } else { "stop".into() })
    }
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub stage: String,
    pub message: String,
    pub fraction: Option<f64>,
}

/// Delivers progress immediately to whoever listens (usually a Dart stream).
#[derive(Clone)]
pub struct ProgressTx(Arc<dyn Fn(Progress) + Send + Sync>);

impl ProgressTx {
    pub fn new(callback: impl Fn(Progress) + Send + Sync + 'static) -> Self {
        Self(Arc::new(callback))
    }

    pub fn silent() -> Self {
        Self::new(|_| {})
    }

    pub fn send(&self, progress: Progress) {
        (self.0)(progress);
    }

    pub fn emit(&self, stage: &str, message: impl Into<String>, fraction: Option<f64>) {
        self.send(Progress {
            stage: stage.to_string(),
            message: message.into(),
            fraction,
        });
    }
}
