// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::PathBuf;

/// Resolved folders used by VoxelPanel. Folders can be moved from the launcher settings.
pub struct Layout {
    pub root: PathBuf,
    servers: Option<PathBuf>,
    backups: Option<PathBuf>,
    runtimes: Option<PathBuf>,
    cache: Option<PathBuf>,
}

impl Layout {
    pub fn app() -> Self {
        let paths = crate::launcher_settings::current().paths;
        let custom = |value: &str| (!value.trim().is_empty()).then(|| PathBuf::from(value.trim()));
        Self {
            root: app_data_root(),
            servers: custom(&paths.servers_dir),
            backups: custom(&paths.backups_dir),
            runtimes: custom(&paths.runtimes_dir),
            cache: custom(&paths.cache_dir),
        }
    }

    /// Default layout rooted at `root`, ignoring the settings.
    #[cfg(test)]
    pub fn at(root: PathBuf) -> Self {
        Self {
            root,
            servers: None,
            backups: None,
            runtimes: None,
            cache: None,
        }
    }

    pub fn runtimes(&self) -> PathBuf {
        self.runtimes.clone().unwrap_or_else(|| self.root.join("runtimes"))
    }

    pub fn servers(&self) -> PathBuf {
        self.servers.clone().unwrap_or_else(|| self.root.join("servers"))
    }

    pub fn cache(&self) -> PathBuf {
        self.cache.clone().unwrap_or_else(|| self.root.join("cache"))
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn catalog_file(&self) -> PathBuf {
        self.root.join("catalog.json")
    }

    pub fn backups_root(&self) -> PathBuf {
        self.backups.clone().unwrap_or_else(|| self.root.join("backups"))
    }

    pub fn backups(&self, server_id: &str) -> PathBuf {
        self.backups_root().join(server_id)
    }

    pub fn default_server(&self, id: &str) -> PathBuf {
        self.servers().join(id)
    }
}

pub use crate::platform::app_data_root;

pub fn ensure_dir(path: &std::path::Path) -> crate::PanelResult<()> {
    std::fs::create_dir_all(path).map_err(|error| crate::PanelError::io(format!("Impossibile creare {}: {error}", path.display())))
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}
