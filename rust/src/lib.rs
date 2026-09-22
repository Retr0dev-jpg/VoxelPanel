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
