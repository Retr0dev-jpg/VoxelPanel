// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::PathBuf;

pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn app() -> Self {
        Self {
            root: app_data_root(),
        }
    }

    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn servers(&self) -> PathBuf {
        self.root.join("servers")
    }

    pub fn catalog_file(&self) -> PathBuf {
        self.root.join("catalog.json")
    }

    pub fn backups_root(&self) -> PathBuf {
        self.root.join("backups")
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
