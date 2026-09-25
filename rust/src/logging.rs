// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::sync::OnceLock;

use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;

use crate::api::settings::{AdvancedSettings, AppLogLevel};

type FilterHandle = reload::Handle<LevelFilter, tracing_subscriber::Registry>;

fn handle() -> &'static OnceLock<FilterHandle> {
    static HANDLE: OnceLock<FilterHandle> = OnceLock::new();
    &HANDLE
}

fn level_of(settings: &AdvancedSettings) -> LevelFilter {
    if !settings.logging {
        return LevelFilter::OFF;
    }
    match settings.log_level {
        AppLogLevel::Error => LevelFilter::ERROR,
        AppLogLevel::Warn => LevelFilter::WARN,
        AppLogLevel::Info => LevelFilter::INFO,
        AppLogLevel::Debug => LevelFilter::DEBUG,
    }
}

/// Daily rotated files in `<data>/logs`, the last 7 days are kept.
pub fn init() {
    if handle().get().is_some() {
        return;
    }
    let settings = crate::launcher_settings::current();
    let directory = crate::paths::Layout::app().logs();
    let _ = std::fs::create_dir_all(&directory);
    let Ok(appender) = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("voxelpanel")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&directory)
    else {
        return;
    };
    let (filter, reload_handle) = reload::Layer::new(level_of(&settings.advanced));
    let fmt = tracing_subscriber::fmt::layer().with_writer(appender).with_ansi(false);
    if tracing_subscriber::registry().with(filter).with(fmt).try_init().is_ok() {
        let _ = handle().set(reload_handle);
        tracing::info!("VoxelPanel avviato");
    }
}

pub fn apply(settings: &AdvancedSettings) {
    if let Some(handle) = handle().get() {
        let _ = handle.modify(|filter| *filter = level_of(settings));
    }
}
