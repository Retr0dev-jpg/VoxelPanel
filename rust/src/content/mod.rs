// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Plugin and mod catalogues (Modrinth, Hangar, Spiget, CurseForge), installed-file metadata,
//! updates and modpacks.

use futures::future::BoxFuture;

use crate::api::types::{AddonKind, ContentPage, ContentSourceKind, ContentVersion, ProviderKind};
use crate::net::Checksum;
use crate::{PanelError, PanelResult, ServerRecord};

pub mod curseforge;
pub mod hangar;
pub mod metadata;
pub mod modpack;
pub mod modrinth;
pub mod spiget;

/// What a server accepts: plugin or mod, loaders and Minecraft version.
#[derive(Debug, Clone)]
pub struct Target {
    pub kind: AddonKind,
    pub provider: ProviderKind,
    pub loaders: Vec<&'static str>,
    pub game_version: Option<String>,
}

impl Target {
    pub fn of(record: &ServerRecord, kind: AddonKind) -> Self {
        Self {
            kind,
            provider: record.provider,
            loaders: crate::addons::loaders(record, kind),
            game_version: record.mc_version.clone().filter(|version| !version.is_empty() && version != "latest"),
        }
    }

    pub fn accepts_version(&self, versions: &[String]) -> bool {
        match &self.game_version {
            None => true,
            Some(server) => versions.iter().any(|version| version == server) || server.rsplit_once('.').is_some_and(|(prefix, _)| versions.iter().any(|version| version == prefix)),
        }
    }

    pub fn accepts_loader(&self, loaders: &[String]) -> bool {
        loaders.is_empty() || loaders.iter().any(|loader| self.loaders.contains(&loader.to_lowercase().as_str()))
    }
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub source: ContentSourceKind,
    pub project_id: String,
    pub version_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Download {
    pub url: String,
    pub file_name: String,
    pub checksum: Option<Checksum>,
    pub dependencies: Vec<Dependency>,
    /// Required dependencies VoxelPanel cannot fetch (external links).
    pub manual: Vec<String>,
}

pub trait ContentSource: Send + Sync {
    fn kind(&self) -> ContentSourceKind;

    fn supports(&self, target: &Target) -> bool;

    fn search<'a>(&'a self, target: &'a Target, query: &'a str, page: u32) -> BoxFuture<'a, PanelResult<ContentPage>>;

    fn versions<'a>(&'a self, target: &'a Target, project_id: &'a str) -> BoxFuture<'a, PanelResult<Vec<ContentVersion>>>;

    fn download<'a>(&'a self, target: &'a Target, project_id: &'a str, version_id: &'a str) -> BoxFuture<'a, PanelResult<Download>>;
}

pub const PAGE_SIZE: u32 = 20;

pub fn all() -> Vec<&'static dyn ContentSource> {
    vec![&modrinth::MODRINTH, &hangar::HANGAR, &spiget::SPIGET, &curseforge::CURSEFORGE]
}

pub fn get(kind: ContentSourceKind) -> &'static dyn ContentSource {
    all().into_iter().find(|source| source.kind() == kind).unwrap_or(&modrinth::MODRINTH)
}

pub fn available(target: &Target) -> Vec<ContentSourceKind> {
    all().into_iter().filter(|source| source.supports(target)).map(|source| source.kind()).collect()
}

/// Picks the version to install: the requested one, otherwise the newest compatible stable one.
pub fn pick_version(versions: &[ContentVersion], wanted: &str) -> PanelResult<String> {
    if !wanted.is_empty() {
        return Ok(wanted.to_string());
    }
    versions
        .iter()
        .find(|version| version.compatible && version.stable)
        .or_else(|| versions.iter().find(|version| version.compatible))
        .map(|version| version.id.clone())
        .ok_or_else(|| PanelError::not_found("Nessuna versione è compatibile con questo server"))
}

/// Orders versions: compatible first, then newest first (by publication date).
pub fn sort_versions(versions: &mut [ContentVersion]) {
    versions.sort_by(|left, right| right.compatible.cmp(&left.compatible).then(right.published.cmp(&left.published)));
}

pub fn is_bukkit_family(provider: ProviderKind) -> bool {
    matches!(
        provider,
        ProviderKind::Paper | ProviderKind::Folia | ProviderKind::Purpur | ProviderKind::Pufferfish | ProviderKind::Leaf | ProviderKind::Spigot | ProviderKind::Mohist | ProviderKind::Arclight | ProviderKind::Custom
    )
}

/// Installs a project and its required dependencies (Modrinth, Hangar and CurseForge declare them).
/// Returns the names of the files written.
pub async fn install(record: &ServerRecord, kind: AddonKind, source: ContentSourceKind, project_id: &str, version_id: &str, progress: &crate::ProgressTx) -> PanelResult<Vec<String>> {
    let target = Target::of(record, kind);
    let directory = record.root.join(crate::addons::folder(kind));
    crate::paths::ensure_dir(&directory)?;
    let mut installed_projects = installed_modrinth_projects(&directory).await;
    let mut queue = vec![Dependency { source, project_id: project_id.to_string(), version_id: Some(version_id.to_string()).filter(|id| !id.is_empty()) }];
    let mut seen = std::collections::HashSet::new();
    let mut written = Vec::new();
    let mut first = true;
    while let Some(item) = queue.pop() {
        if !seen.insert((item.source, item.project_id.clone())) || seen.len() > 30 {
            continue;
        }
        if !first && item.source == ContentSourceKind::Modrinth && installed_projects.contains(&item.project_id) {
            continue;
        }
        let download = get(item.source).download(&target, &item.project_id, item.version_id.as_deref().unwrap_or("")).await;
        let download = match download {
            Ok(download) => download,
            Err(error) if !first => {
                progress.emit("Dipendenze", format!("Dipendenza {} non installata: {}", item.project_id, error.message), None);
                continue;
            }
            Err(error) => return Err(error),
        };
        first = false;
        let destination = crate::providers::jar_destination(&directory, &download.file_name)?;
        if destination.exists() {
            progress.emit("Contenuti", format!("{} è già installato.", download.file_name), None);
        } else {
            progress.emit("Contenuti", format!("Download {}...", download.file_name), Some(0.0));
            crate::net::download_checked(&download.url, &destination, "Contenuti", progress, download.checksum.clone()).await?;
            written.push(download.file_name.clone());
        }
        if item.source == ContentSourceKind::Modrinth {
            installed_projects.insert(item.project_id.clone());
        }
        for name in &download.manual {
            progress.emit("Dipendenze", format!("Installa a mano la dipendenza richiesta: {name}"), None);
        }
        queue.extend(download.dependencies);
    }
    Ok(written)
}

fn jar_hashes(directory: &std::path::Path) -> Vec<(String, String)> {
    std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().to_lowercase().ends_with(".jar"))
        .filter_map(|entry| metadata::sha1_of(&entry.path()).map(|hash| (entry.file_name().to_string_lossy().to_string(), hash)))
        .collect()
}

async fn installed_modrinth_projects(directory: &std::path::Path) -> std::collections::HashSet<String> {
    let hashes: Vec<String> = jar_hashes(directory).into_iter().map(|(_, hash)| hash).collect();
    modrinth::identify(&hashes)
        .await
        .map(|versions| versions.values().filter_map(|version| version.get("project_id").and_then(serde_json::Value::as_str).map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Installed files with a newer compatible version on Modrinth (matched by SHA-1).
pub async fn check_updates(record: &ServerRecord, kind: AddonKind) -> PanelResult<Vec<crate::api::types::AddonUpdate>> {
    let target = Target::of(record, kind);
    let directory = record.root.join(crate::addons::folder(kind));
    let files = tokio::task::spawn_blocking(move || jar_hashes(&directory)).await?;
    let hashes: Vec<String> = files.iter().map(|(_, hash)| hash.clone()).collect();
    let current = modrinth::identify(&hashes).await?;
    let latest = modrinth::latest_for(&hashes, &target).await?;
    let field = |value: &serde_json::Value, key: &str| value.get(key).and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
    Ok(files
        .into_iter()
        .filter_map(|(file_name, hash)| {
            let now = current.get(&hash)?;
            let next = latest.get(&hash)?;
            (field(now, "id") != field(next, "id")).then(|| crate::api::types::AddonUpdate {
                file_name,
                project_id: field(now, "project_id"),
                current_version: field(now, "version_number"),
                new_version_id: field(next, "id"),
                new_version: field(next, "version_number"),
            })
        })
        .collect())
}

/// Downloads the new versions, then removes the files they replace.
pub async fn apply_updates(record: &ServerRecord, kind: AddonKind, updates: &[crate::api::types::AddonUpdate], progress: &crate::ProgressTx) -> PanelResult<u32> {
    let directory = record.root.join(crate::addons::folder(kind));
    let mut done = 0;
    for update in updates {
        let version = modrinth::version(&update.new_version_id).await?;
        let (url, file_name, checksum) = modrinth::primary_file(&version).ok_or_else(|| PanelError::not_found("La versione non ha file scaricabili"))?;
        let destination = crate::providers::jar_destination(&directory, &file_name)?;
        progress.emit("Aggiornamenti", format!("{} → {}", update.file_name, file_name), Some(done as f64 / updates.len().max(1) as f64));
        crate::net::download_checked(&url, &destination, "Aggiornamenti", progress, checksum).await?;
        if file_name != update.file_name {
            let old = crate::providers::jar_destination(&directory, &update.file_name)?;
            if old.exists() {
                std::fs::remove_file(old)?;
            }
        }
        done += 1;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(id: &str, compatible: bool, stable: bool, published: &str) -> ContentVersion {
        ContentVersion {
            id: id.into(),
            name: id.into(),
            game_versions: vec![],
            loaders: vec![],
            published: published.into(),
            stable,
            compatible,
        }
    }

    #[test]
    fn picks_newest_compatible_stable_version() {
        let mut versions = vec![
            version("old", true, true, "2025-01-01"),
            version("beta", true, false, "2026-02-01"),
            version("incompatible", false, true, "2026-03-01"),
            version("new", true, true, "2026-01-01"),
        ];
        sort_versions(&mut versions);
        assert_eq!(versions[0].id, "beta");
        assert_eq!(pick_version(&versions, "").unwrap(), "new");
        assert_eq!(pick_version(&versions, "old").unwrap(), "old");
    }

    #[test]
    fn matches_targets() {
        let target = Target { kind: AddonKind::Mod, provider: ProviderKind::Quilt, loaders: vec!["quilt", "fabric"], game_version: Some("1.21.1".into()) };
        assert!(target.accepts_loader(&["Fabric".into()]));
        assert!(!target.accepts_loader(&["forge".into()]));
        assert!(target.accepts_version(&["1.21".into()]));
        assert!(!target.accepts_version(&["1.20.4".into()]));
    }
}

#[cfg(test)]
mod live {
    use super::*;

    fn record(provider: ProviderKind, version: &str) -> ServerRecord {
        let root = std::env::temp_dir().join(format!("voxel-content-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let mut record = ServerRecord::new("live".into(), "live".into(), root);
        record.provider = provider;
        record.mc_version = Some(version.into());
        record
    }

    #[tokio::test]
    #[ignore]
    async fn live_searches_and_installs_content() {
        let paper = record(ProviderKind::Paper, "1.21.4");
        let plugins = Target::of(&paper, AddonKind::Plugin);
        for source in [ContentSourceKind::Modrinth, ContentSourceKind::Hangar, ContentSourceKind::Spiget] {
            let page = get(source).search(&plugins, "luckperms", 0).await.unwrap_or_else(|error| panic!("{source:?}: {}", error.message));
            assert!(!page.projects.is_empty(), "{source:?} found nothing");
            println!("{source:?}: {} results, first {}", page.total, page.projects[0].title);
        }
        let hangar = get(ContentSourceKind::Hangar).search(&plugins, "ViaVersion", 0).await.unwrap();
        let written = install(&paper, AddonKind::Plugin, ContentSourceKind::Hangar, &hangar.projects[0].id, "", &crate::ProgressTx::silent()).await.unwrap();
        println!("Hangar installed {written:?}");
        assert!(!written.is_empty());

        let fabric = record(ProviderKind::Fabric, "1.21.1");
        let mods = Target::of(&fabric, AddonKind::Mod);
        let page = get(ContentSourceKind::Modrinth).search(&mods, "sodium extra", 0).await.unwrap();
        let extra = page.projects.iter().find(|project| project.slug == "sodium-extra").expect("sodium-extra");
        let written = install(&fabric, AddonKind::Mod, ContentSourceKind::Modrinth, &extra.id, "", &crate::ProgressTx::silent()).await.unwrap();
        println!("Modrinth installed {written:?}");
        assert!(written.len() >= 2, "dependency not installed: {written:?}");
        let updates = check_updates(&fabric, AddonKind::Mod).await.unwrap();
        println!("updates available: {}", updates.len());
        let _ = std::fs::remove_dir_all(&paper.root);
        let _ = std::fs::remove_dir_all(&fabric.root);
    }
}
