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

    pub fn backups(&self, server_id: &str) -> PathBuf {
        self.root.join("backups").join(server_id)
    }

    pub fn default_server(&self, id: &str) -> PathBuf {
        self.servers().join(id)
    }
}

pub fn app_data_root() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("VoxelPanel")
}

pub fn ensure_dir(path: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|error| format!("Impossibile creare {}: {error}", path.display()))
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}
