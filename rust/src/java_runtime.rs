// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::{Path, PathBuf};

use serde::Deserialize;

pub use crate::api::settings::JavaVendor;
use crate::platform;
use crate::scan::derive_java_version;
use crate::{PanelError, PanelResult, ProgressTx};

pub const VENDORS: [JavaVendor; 7] = [
    JavaVendor::Temurin,
    JavaVendor::Zulu,
    JavaVendor::Corretto,
    JavaVendor::Microsoft,
    JavaVendor::Liberica,
    JavaVendor::SapMachine,
    JavaVendor::GraalVm,
];

pub struct VendorMeta {
    /// Prefix of the folders VoxelPanel installs, so equal archive names never collide.
    pub id: &'static str,
    pub name: &'static str,
    pub publisher: &'static str,
    pub website: &'static str,
    /// Distribution name in the foojay Disco API.
    foojay: &'static str,
    /// Lowercase fragments of `IMPLEMENTOR` in the JDK `release` file.
    implementors: &'static [&'static str],
}

pub fn meta(vendor: JavaVendor) -> VendorMeta {
    let (id, name, publisher, website, foojay, implementors): (_, _, _, _, _, &'static [&'static str]) = match vendor {
        JavaVendor::Temurin => ("temurin", "Temurin", "Eclipse Adoptium", "https://adoptium.net", "temurin", &["adoptium", "adoptopenjdk", "eclipse"]),
        JavaVendor::Zulu => ("zulu", "Zulu", "Azul", "https://www.azul.com/downloads/", "zulu", &["azul"]),
        JavaVendor::Corretto => ("corretto", "Corretto", "Amazon", "https://aws.amazon.com/corretto/", "corretto", &["amazon"]),
        JavaVendor::Microsoft => ("microsoft", "Microsoft Build of OpenJDK", "Microsoft", "https://learn.microsoft.com/java/openjdk/", "microsoft", &["microsoft"]),
        JavaVendor::Liberica => ("liberica", "Liberica", "BellSoft", "https://bell-sw.com/libericajdk/", "liberica", &["bellsoft"]),
        JavaVendor::SapMachine => ("sapmachine", "SapMachine", "SAP", "https://sap.github.io/SapMachine/", "sap_machine", &["sap"]),
        JavaVendor::GraalVm => ("graalvm", "GraalVM Community", "GraalVM", "https://www.graalvm.org", "graalvm_community", &["graalvm"]),
    };
    VendorMeta { id, name, publisher, website, foojay, implementors }
}

pub fn parse_vendor(release: &str) -> Option<JavaVendor> {
    let implementor = release.lines().find_map(|line| line.strip_prefix("IMPLEMENTOR="))?.trim().trim_matches('"').to_lowercase();
    VENDORS.into_iter().find(|&vendor| meta(vendor).implementors.iter().any(|fragment| implementor.contains(fragment)))
}

pub fn vendor_of(home: &Path) -> Option<JavaVendor> {
    std::fs::read_to_string(home.join("release")).ok().as_deref().and_then(parse_vendor)
}

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
    pub vendor: Option<JavaVendor>,
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
        if !path.is_dir() || entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let Some(home) = platform::resolve_java_home(&path) else {
            continue;
        };
        runtimes.push(JavaRuntime {
            major: derive_from_home(&home),
            vendor: vendor_of(&home),
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
        vendor: vendor_of(&home),
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

/// A managed runtime for `major`; with `vendor` only one from that distribution.
pub fn find_managed(runtime_root: &Path, major: u32, vendor: Option<JavaVendor>) -> Option<PathBuf> {
    scan_runtime_dir(runtime_root)
        .into_iter()
        .find(|runtime| runtime.major == Some(major) && (vendor.is_none() || runtime.vendor == vendor))
        .map(|runtime| runtime.home)
}

#[derive(Debug, Deserialize)]
struct FoojayResponse {
    result: Vec<FoojayPackage>,
}

#[derive(Debug, Deserialize)]
struct FoojayPackage {
    major_version: u32,
    term_of_support: String,
    filename: String,
    links: FoojayLinks,
}

#[derive(Debug, Deserialize)]
struct FoojayLinks {
    pkg_download_redirect: String,
}

/// Latest GA JDK packages of `vendor` for this platform (one per major, or only `major`).
pub fn foojay_packages_url(vendor: JavaVendor, major: Option<u32>) -> String {
    let (os, archive, libc) = if cfg!(windows) {
        ("windows", "zip", "c_std_lib")
    } else if cfg!(target_os = "macos") {
        ("macos", "tar.gz", "libc")
    } else {
        ("linux", "tar.gz", "glibc")
    };
    let version = major.map(|major| format!("&version={major}")).unwrap_or_default();
    format!(
        "https://api.foojay.io/disco/v3.0/packages?distribution={}&architecture={}&operating_system={os}&archive_type={archive}&lib_c_type={libc}&package_type=jdk&release_status=ga&latest=available&javafx_bundled=false&directly_downloadable=true{version}",
        meta(vendor).foojay,
        platform::adoptium_arch(),
    )
}

async fn foojay_packages(vendor: JavaVendor, major: Option<u32>) -> PanelResult<Vec<FoojayPackage>> {
    let response: FoojayResponse = crate::net::http()
        .get(foojay_packages_url(vendor, major))
        .send()
        .await
        .map_err(|error| PanelError::network(format!("Catalogo Java (foojay) non raggiungibile: {error}")))?
        .error_for_status()?
        .json()
        .await?;
    Ok(response.result)
}

pub async fn list_releases(vendor: JavaVendor) -> PanelResult<Vec<JavaRelease>> {
    let mut items: Vec<JavaRelease> = if vendor == JavaVendor::Temurin {
        let releases: Releases = crate::net::http()
            .get("https://api.adoptium.net/v3/info/available_releases")
            .send()
            .await
            .map_err(|error| PanelError::network(format!("Adoptium non raggiungibile: {error}")))?
            .error_for_status()?
            .json()
            .await?;
        let lts = releases.available_lts_releases;
        releases.available_releases.into_iter().map(|major| JavaRelease { lts: lts.contains(&major), major }).collect()
    } else {
        let mut items: Vec<JavaRelease> = Vec::new();
        for package in foojay_packages(vendor, None).await? {
            if !items.iter().any(|item| item.major == package.major_version) {
                items.push(JavaRelease { major: package.major_version, lts: package.term_of_support.eq_ignore_ascii_case("lts") });
            }
        }
        items
    };
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

async fn package_for(vendor: JavaVendor, major: u32) -> PanelResult<Package> {
    let name = meta(vendor).name;
    let package = if vendor == JavaVendor::Temurin {
        let assets: Vec<Asset> = crate::net::http()
            .get(adoptium_assets_url(major))
            .send()
            .await
            .map_err(|error| PanelError::network(format!("Pacchetto Java non raggiungibile: {error}")))?
            .error_for_status()
            .map_err(|error| PanelError::not_found(format!("Pacchetto Java {major} non trovato: {error}")))?
            .json()
            .await?;
        assets.into_iter().map(|asset| asset.binary.package).find(|package| !package.link.is_empty() && !package.name.is_empty())
    } else {
        foojay_packages(vendor, Some(major))
            .await?
            .into_iter()
            .find(|package| package.major_version == major)
            .map(|package| Package { name: package.filename, link: package.links.pkg_download_redirect })
    };
    package.ok_or_else(|| PanelError::not_found(format!("Pacchetto Java {major} di {name} non disponibile per questo sistema")))
}

/// Downloads Java `major` from `vendor` unless a runtime of that distribution is already managed.
pub async fn install(runtime_root: &Path, vendor: JavaVendor, major: u32, progress: &ProgressTx) -> PanelResult<PathBuf> {
    let vendor_meta = meta(vendor);
    if let Some(existing) = find_managed(runtime_root, major, Some(vendor)) {
        progress.emit("Java", format!("{} {major} già installato.", vendor_meta.name), Some(1.0));
        return Ok(existing);
    }
    let package = package_for(vendor, major).await?;
    let staging = runtime_root.join(format!(".staging-{}", uuid::Uuid::new_v4().simple()));
    crate::paths::ensure_dir(&staging)?;
    let result = async {
        let archive = staging.join(&package.name);
        progress.emit("Java", format!("Download {} {major}...", vendor_meta.name), Some(0.0));
        crate::net::download(&package.link, &archive, "Java", progress).await?;
        progress.emit("Java", format!("Estrazione {} {major}...", vendor_meta.name), None);
        let extracted = staging.join("jdk");
        let (archive_path, destination) = (archive.clone(), extracted.clone());
        tokio::task::spawn_blocking(move || crate::net::extract_archive(&archive_path, &destination)).await??;
        let top = std::fs::read_dir(&extracted)?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| platform::resolve_java_home(path).is_some())
            .ok_or_else(|| PanelError::invalid(format!("L'archivio {} non contiene un JDK riconoscibile", package.name)))?;
        let folder = top.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_else(|| major.to_string());
        let target = runtime_root.join(format!("{}-{folder}", vendor_meta.id));
        if !target.exists() {
            std::fs::rename(&top, &target)?;
        }
        platform::resolve_java_home(&target).ok_or_else(|| PanelError::from(format!("Java {major} estratto ma non rilevato in {}", target.display())))
    }
    .await;
    let _ = std::fs::remove_dir_all(&staging);
    result
}

/// Any managed Java `major`, downloading it from the vendor chosen in the settings if missing.
pub async fn ensure_major(runtime_root: &Path, major: u32, progress: &ProgressTx) -> PanelResult<PathBuf> {
    if let Some(existing) = find_managed(runtime_root, major, None) {
        progress.emit("Java", format!("Java {major} già installato."), Some(1.0));
        return Ok(existing);
    }
    install(runtime_root, crate::launcher_settings::current().java.vendor, major, progress).await
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
    fn recognises_vendors_from_release_files() {
        assert_eq!(parse_vendor("IMPLEMENTOR=\"Eclipse Adoptium\"\nJAVA_VERSION=\"21.0.2\""), Some(JavaVendor::Temurin));
        assert_eq!(parse_vendor("IMPLEMENTOR=\"Azul Systems, Inc.\""), Some(JavaVendor::Zulu));
        assert_eq!(parse_vendor("IMPLEMENTOR=\"Amazon.com Inc.\""), Some(JavaVendor::Corretto));
        assert_eq!(parse_vendor("IMPLEMENTOR=\"Microsoft\""), Some(JavaVendor::Microsoft));
        assert_eq!(parse_vendor("IMPLEMENTOR=\"BellSoft\""), Some(JavaVendor::Liberica));
        assert_eq!(parse_vendor("IMPLEMENTOR=\"SAP SE\""), Some(JavaVendor::SapMachine));
        assert_eq!(parse_vendor("IMPLEMENTOR=\"GraalVM Community\""), Some(JavaVendor::GraalVm));
        assert_eq!(parse_vendor("IMPLEMENTOR=\"Oracle Corporation\""), None);
        assert_eq!(parse_vendor("JAVA_VERSION=\"17\""), None);
    }

    #[test]
    fn vendor_folder_prefixes_are_unique() {
        let ids: std::collections::HashSet<_> = VENDORS.iter().map(|&vendor| meta(vendor).id).collect();
        assert_eq!(ids.len(), VENDORS.len());
    }

    #[test]
    fn asks_foojay_for_this_platform() {
        let url = foojay_packages_url(JavaVendor::Zulu, Some(21));
        assert!(url.contains("distribution=zulu"));
        assert!(url.contains("&version=21"));
        assert!(url.contains(&format!("architecture={}", platform::adoptium_arch())));
        assert!(!foojay_packages_url(JavaVendor::SapMachine, None).contains("version="));
    }

    #[tokio::test]
    #[ignore]
    async fn live_lists_java_releases_for_every_vendor() {
        for vendor in VENDORS {
            let releases = list_releases(vendor).await.unwrap();
            println!("{} -> {:?}", meta(vendor).name, releases.iter().map(|release| release.major).collect::<Vec<_>>());
            assert!(releases.iter().any(|release| release.major == 21 && release.lts), "{} has no Java 21 LTS", meta(vendor).name);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_installs_a_non_temurin_runtime() {
        let root = std::env::temp_dir().join(format!("voxel-java-{}", uuid::Uuid::new_v4()));
        let home = install(&root, JavaVendor::Zulu, 21, &ProgressTx::silent()).await.unwrap();
        assert!(java_executable(&home).is_file());
        assert_eq!(vendor_of(&home), Some(JavaVendor::Zulu));
        assert_eq!(find_managed(&root, 21, Some(JavaVendor::Zulu)), Some(home.clone()));
        assert_eq!(find_managed(&root, 21, Some(JavaVendor::Temurin)), None);
        assert!(std::fs::read_dir(&root).unwrap().flatten().all(|entry| !entry.file_name().to_string_lossy().starts_with(".staging")));
        let again = install(&root, JavaVendor::Zulu, 21, &ProgressTx::silent()).await.unwrap();
        assert_eq!(again, home);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn asks_adoptium_for_this_platform() {
        let url = adoptium_assets_url(21);
        assert!(url.contains(&format!("os={}", platform::adoptium_os())));
        assert!(url.contains(&format!("architecture={}", platform::adoptium_arch())));
    }
}
