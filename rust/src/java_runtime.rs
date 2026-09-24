// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use crate::scan::derive_java_version;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct JavaRuntime {
    pub name: String,
    pub major: Option<u32>,
    pub home: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Asset {
    binary: Binary,
}

#[derive(Debug, Deserialize)]
struct Binary {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    link: String,
}

#[derive(Debug, Deserialize)]
struct Releases {
    available_lts_releases: Vec<u32>,
    available_releases: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct JavaRelease {
    pub major: u32,
    pub lts: bool,
}

pub fn java_executable(home: &Path) -> PathBuf {
    home.join("bin").join("java.exe")
}

pub fn derive_from_home(home: &Path) -> Option<u32> {
    home.file_name()
        .and_then(|name| name.to_str())
        .and_then(derive_java_version)
}

pub fn scan_runtime_dir(runtime_root: &Path) -> Vec<JavaRuntime> {
    let mut runtimes = Vec::new();
    let Ok(entries) = std::fs::read_dir(runtime_root) else {
        return runtimes;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !java_executable(&path).exists() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        runtimes.push(JavaRuntime {
            major: derive_java_version(&name),
            name,
            home: path,
        });
    }
    runtimes.sort_by(|left, right| {
        right
            .major
            .cmp(&left.major)
            .then_with(|| left.name.cmp(&right.name))
    });
    runtimes
}

pub fn collect(layout: &crate::paths::Layout) -> Vec<JavaRuntime> {
    let mut runtimes = scan_runtime_dir(&layout.runtimes());
    if let Ok(records) = crate::catalog::list(layout) {
        for record in records {
            push_unique(&mut runtimes, scan_runtime_dir(&record.root.join("runtime")));
            if let Some(home) = record.java_home {
                if java_executable(&home).exists() {
                    push_unique(
                        &mut runtimes,
                        vec![JavaRuntime {
                            major: derive_from_home(&home),
                            name: home
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("java")
                                .to_string(),
                            home,
                        }],
                    );
                }
            }
        }
    }
    runtimes.sort_by(|left, right| {
        right
            .major
            .cmp(&left.major)
            .then_with(|| left.name.cmp(&right.name))
    });
    runtimes
}

fn push_unique(target: &mut Vec<JavaRuntime>, incoming: Vec<JavaRuntime>) {
    for runtime in incoming {
        let exists = target.iter().any(|item| {
            item.home.to_string_lossy().replace('/', "\\").to_lowercase()
                == runtime.home.to_string_lossy().replace('/', "\\").to_lowercase()
        });
        if !exists {
            target.push(runtime);
        }
    }
}

pub fn find_by_major(runtime_root: &Path, major: u32) -> Option<PathBuf> {
    scan_runtime_dir(runtime_root)
        .into_iter()
        .find(|runtime| runtime.major == Some(major))
        .map(|runtime| runtime.home)
}

pub async fn list_releases() -> Result<Vec<JavaRelease>, String> {
    let releases: Releases = crate::net::http()
        .get("https://api.adoptium.net/v3/info/available_releases")
        .send()
        .await
        .map_err(|error| format!("Adoptium non raggiungibile: {error}"))?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let lts = releases.available_lts_releases.clone();
    let mut items: Vec<JavaRelease> = releases
        .available_releases
        .into_iter()
        .map(|major| JavaRelease {
            lts: lts.contains(&major),
            major,
        })
        .collect();
    items.sort_by(|left, right| right.major.cmp(&left.major));
    Ok(items)
}

pub async fn ensure_major(
    runtime_root: &Path,
    major: u32,
    progress: &std::sync::mpsc::Sender<crate::Progress>,
) -> Result<PathBuf, String> {
    if let Some(existing) = find_by_major(runtime_root, major) {
        let _ = progress.send(crate::Progress {
            stage: "Java".into(),
            message: format!("Java {major} già installato."),
            fraction: Some(1.0),
        });
        return Ok(existing);
    }
    let assets: Vec<Asset> = crate::net::http()
        .get(format!(
            "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture=x64&image_type=jdk&os=windows&vendor=eclipse"
        ))
        .send()
        .await
        .map_err(|error| format!("Pacchetto Java non raggiungibile: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Pacchetto Java {major} non trovato: {error}"))?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let package = assets
        .into_iter()
        .find_map(|asset| {
            if asset.binary.package.link.is_empty() || asset.binary.package.name.is_empty() {
                None
            } else {
                Some(asset.binary.package)
            }
        })
        .ok_or_else(|| format!("Pacchetto Java {major} non trovato da Adoptium"))?;
    crate::paths::ensure_dir(runtime_root)?;
    let archive = runtime_root.join(&package.name);
    let _ = progress.send(crate::Progress {
        stage: "Java".into(),
        message: format!("Download Java {major}..."),
        fraction: Some(0.0),
    });
    crate::net::download(&package.link, &archive, "Java", progress).await?;
    let _ = progress.send(crate::Progress {
        stage: "Java".into(),
        message: format!("Estrazione Java {major}..."),
        fraction: None,
    });
    let archive_path = archive.clone();
    let destination = runtime_root.to_path_buf();
    tokio::task::spawn_blocking(move || crate::net::extract_zip(&archive_path, &destination))
        .await
        .map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(&archive);
    find_by_major(runtime_root, major).ok_or_else(|| {
        format!("Java {major} estratto ma non rilevato in {}", runtime_root.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_path_uses_bin() {
        let home = PathBuf::from(r"C:\runtime\jdk-21.0.2");
        assert!(java_executable(&home)
            .ends_with(r"jdk-21.0.2\bin\java.exe"));
    }
}
