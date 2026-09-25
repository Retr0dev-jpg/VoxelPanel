// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

pub mod api;
mod backup;
mod catalog;
mod frb_generated;
mod install;
mod java_runtime;
mod jvm;
mod net;
mod paper;
mod paths;
mod plugins;
mod process;
mod properties;
mod ram;
mod scan;
mod script;
mod worlds;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

pub use api::error::{ErrorCode, PanelError, PanelResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub paper_version: Option<String>,
    pub java_major: Option<u32>,
    pub java_home: Option<PathBuf>,
    pub jar_path: Option<PathBuf>,
    pub ram_min: String,
    pub ram_max: String,
    pub jvm_flags: Vec<String>,
    pub eula_accepted: bool,
    pub created_unix: i64,
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
