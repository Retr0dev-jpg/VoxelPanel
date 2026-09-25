// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::platform;
use crate::scan::derive_java_version;
use crate::{PanelError, PanelResult, ProgressTx};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSource {
    /// Downloaded by VoxelPanel into the runtimes folder.
    Managed,
    /// Bundled inside a server folder (`runtime/`) or referenced by a server.
    Server,
    /// Installed on the system (JAVA_HOME, PATH or the usual install folders).
    System,
}

#[derive(Debug, Clone)]
pub struct JavaRuntime {
    pub name: String,
    pub major: Option<u32>,
    pub home: PathBuf,
    pub source: RuntimeSource,
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

pub use platform::java_executable;

/// Reads `JAVA_VERSION` from the JDK `release` file, falling back to folder names
/// (`jdk-21.0.2+13`, also above the macOS `Contents/Home` nesting).
pub fn derive_from_home(home: &Path) -> Option<u32> {
    if let Ok(release) = std::fs::read_to_string(home.join("release")) {
        if let Some(major) = parse_release_file(&release) {
            return Some(major);
        }
    }
    home.ancestors()
        .take(3)
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
        .find_map(derive_java_version)
}

pub fn parse_release_file(content: &str) -> Option<u32> {
    let value = content
        .lines()
        .find_map(|line| line.strip_prefix("JAVA_VERSION="))?
        .trim()
        .trim_matches('"');
    let mut parts = value.split(['.', '_', '+', '-']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        return parts.next()?.parse().ok();
    }
    Some(first)
}

pub fn scan_runtime_dir(runtime_root: &Path) -> Vec<JavaRuntime> {
    scan_dir(runtime_root, RuntimeSource::Managed)
}

fn scan_dir(runtime_root: &Path, source: RuntimeSource) -> Vec<JavaRuntime> {
    let mut runtimes = Vec::new();
    let Ok(entries) = std::fs::read_dir(runtime_root) else {
        return runtimes;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(home) = platform::resolve_java_home(&path) else {
            continue;
        };
        runtimes.push(JavaRuntime {
            major: derive_from_home(&home),
            name: entry.file_name().to_string_lossy().to_string(),
            home,
            source,
        });
    }
    sort_runtimes(&mut runtimes);
    runtimes
}

fn sort_runtimes(runtimes: &mut [JavaRuntime]) {
    runtimes.sort_by(|left, right| right.major.cmp(&left.major).then_with(|| left.name.cmp(&right.name)));
}

pub fn collect(layout: &crate::paths::Layout) -> Vec<JavaRuntime> {
    let mut runtimes = scan_runtime_dir(&layout.runtimes());
    if let Ok(records) = crate::catalog::list(layout) {
        for record in records {
            push_unique(&mut runtimes, scan_dir(&record.root.join("runtime"), RuntimeSource::Server));
            if let Some(home) = record.java_home {
                if java_executable(&home).exists() {
                    push_unique(&mut runtimes, vec![runtime_at(home, RuntimeSource::Server)]);
                }
            }
        }
    }
    push_unique(&mut runtimes, detect_system());
    sort_runtimes(&mut runtimes);
    runtimes
}

fn runtime_at(home: PathBuf, source: RuntimeSource) -> JavaRuntime {
    let name = home
        .ancestors()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
        .find(|name| !matches!(*name, "Home" | "Contents"))
        .unwrap_or("java")
        .to_string();
    JavaRuntime {
        major: derive_from_home(&home),
        name,
        home,
        source,
    }
}

/// Folders where Java installers usually put JDKs on each system.
fn system_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    if cfg!(windows) {
        let program_files = std::env::var_os("ProgramFiles").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
        ["Java", "Eclipse Adoptium", "Microsoft", "Zulu", "Amazon Corretto", "BellSoft"]
            .iter()
            .map(|vendor| program_files.join(vendor))
            .collect()
    } else if cfg!(target_os = "macos") {
        vec![PathBuf::from("/Library/Java/JavaVirtualMachines"), home.join("Library/Java/JavaVirtualMachines")]
    } else {
        vec![PathBuf::from("/usr/lib/jvm"), PathBuf::from("/opt/java"), home.join(".sdkman/candidates/java"), home.join(".jdks")]
    }
}

/// Java runtimes already installed on the machine (JAVA_HOME, PATH, vendor folders).
pub fn detect_system() -> Vec<JavaRuntime> {
    let mut found: Vec<JavaRuntime> = Vec::new();
    let mut homes: Vec<PathBuf> = Vec::new();
    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        homes.push(PathBuf::from(java_home));
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(platform::JAVA_BINARY);
            if let Ok(real) = std::fs::canonicalize(&candidate) {
                if let Some(home) = real.parent().and_then(Path::parent) {
                    homes.push(home.to_path_buf());
                }
            }
        }
    }
    for home in homes {
        if let Some(home) = platform::resolve_java_home(&home) {
            push_unique(&mut found, vec![runtime_at(home, RuntimeSource::System)]);
        }
    }
    for root in system_roots() {
        push_unique(&mut found, scan_dir(&root, RuntimeSource::System));
    }
    found
}

fn push_unique(target: &mut Vec<JavaRuntime>, incoming: Vec<JavaRuntime>) {
    for runtime in incoming {
        if !target.iter().any(|item| platform::same_path(&item.home, &runtime.home)) {
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

pub async fn list_releases() -> PanelResult<Vec<JavaRelease>> {
    let releases: Releases = crate::net::http()
        .get("https://api.adoptium.net/v3/info/available_releases")
        .send()
        .await
        .map_err(|error| PanelError::network(format!("Adoptium non raggiungibile: {error}")))?
        .error_for_status()?
        .json()
        .await?;
    let lts = releases.available_lts_releases.clone();
    let mut items: Vec<JavaRelease> = releases
        .available_releases
        .into_iter()
        .map(|major| JavaRelease {
            lts: lts.contains(&major),
            major,
        })
        .collect();
    items.sort_by_key(|item| std::cmp::Reverse(item.major));
    Ok(items)
}

pub fn adoptium_assets_url(major: u32) -> String {
    format!(
        "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture={}&image_type=jdk&os={}&vendor=eclipse",
        platform::adoptium_arch(),
        platform::adoptium_os()
    )
}

pub async fn ensure_major(runtime_root: &Path, major: u32, progress: &ProgressTx) -> PanelResult<PathBuf> {
    if let Some(existing) = find_by_major(runtime_root, major) {
        progress.emit("Java", format!("Java {major} già installato."), Some(1.0));
        return Ok(existing);
    }
    let assets: Vec<Asset> = crate::net::http()
        .get(adoptium_assets_url(major))
        .send()
        .await
        .map_err(|error| PanelError::network(format!("Pacchetto Java non raggiungibile: {error}")))?
        .error_for_status()
        .map_err(|error| PanelError::not_found(format!("Pacchetto Java {major} non trovato: {error}")))?
        .json()
        .await?;
    let package = assets
        .into_iter()
        .map(|asset| asset.binary.package)
        .find(|package| !package.link.is_empty() && !package.name.is_empty())
        .ok_or_else(|| PanelError::not_found(format!("Pacchetto Java {major} non trovato da Adoptium")))?;
    crate::paths::ensure_dir(runtime_root)?;
    let archive = runtime_root.join(&package.name);
    progress.emit("Java", format!("Download Java {major}..."), Some(0.0));
    crate::net::download(&package.link, &archive, "Java", progress).await?;
    progress.emit("Java", format!("Estrazione Java {major}..."), None);
    let archive_path = archive.clone();
    let destination = runtime_root.to_path_buf();
    tokio::task::spawn_blocking(move || crate::net::extract_archive(&archive_path, &destination)).await??;
    let _ = std::fs::remove_file(&archive);
    find_by_major(runtime_root, major).ok_or_else(|| {
        PanelError::from(format!("Java {major} estratto ma non rilevato in {}", runtime_root.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_release_files() {
        assert_eq!(parse_release_file("IMPLEMENTOR=\"Eclipse\"\nJAVA_VERSION=\"21.0.2\"\n"), Some(21));
        assert_eq!(parse_release_file("JAVA_VERSION=\"1.8.0_402\"\n"), Some(8));
        assert_eq!(parse_release_file("OTHER=1\n"), None);
    }

    #[test]
    fn scans_runtimes_with_release_file() {
        let root = std::env::temp_dir().join(format!("voxel-runtimes-{}", uuid::Uuid::new_v4()));
        let home = root.join("custom-jdk");
        std::fs::create_dir_all(home.join("bin")).unwrap();
        std::fs::write(java_executable(&home), b"").unwrap();
        std::fs::write(home.join("release"), "JAVA_VERSION=\"17.0.9\"\n").unwrap();
        let runtimes = scan_runtime_dir(&root);
        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0].major, Some(17));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn asks_adoptium_for_this_platform() {
        let url = adoptium_assets_url(21);
        assert!(url.contains(&format!("os={}", platform::adoptium_os())));
        assert!(url.contains(&format!("architecture={}", platform::adoptium_arch())));
    }
}
