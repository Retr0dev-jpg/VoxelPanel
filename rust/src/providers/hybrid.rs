// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Mod loaders with plugin support: Mohist, Arclight, SpongeVanilla and SpongeForge.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{BuildEntry, ProviderKind, VersionEntry};
use crate::net::Checksum;
use crate::{cache, LaunchSpec, PanelError, PanelResult};

const MOHIST: &str = "https://api.mohistmc.com/project/mohist";
const ARCLIGHT_RELEASES: &str = "https://api.github.com/repos/IzzelAliz/Arclight/releases?per_page=50";
const SPONGE: &str = "https://dl-api.spongepowered.org/v2/groups/org.spongepowered/artifacts";

pub struct Mohist;
pub static MOHIST_PROVIDER: Mohist = Mohist;
pub struct Arclight;
pub static ARCLIGHT: Arclight = Arclight;
pub struct Sponge {
    kind: ProviderKind,
    artifact: &'static str,
}
pub static SPONGE_VANILLA: Sponge = Sponge { kind: ProviderKind::SpongeVanilla, artifact: "spongevanilla" };
pub static SPONGE_FORGE: Sponge = Sponge { kind: ProviderKind::SpongeForge, artifact: "spongeforge" };

pub fn mohist_versions(value: &Value) -> Vec<VersionEntry> {
    let mut versions: Vec<VersionEntry> = value
        .as_array()
        .map(|items| items.iter().filter_map(|item| item.get("name")?.as_str()).map(|id| VersionEntry { id: id.to_string(), stable: true }).collect())
        .unwrap_or_default();
    super::sort_versions_desc(&mut versions);
    versions
}

struct MohistBuild {
    id: u64,
    sha256: Option<String>,
    label: String,
}

fn mohist_builds(value: &Value) -> Vec<MohistBuild> {
    let mut builds: Vec<MohistBuild> = value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = item.get("id")?.as_u64()?;
                    let date = item.get("build_date").and_then(Value::as_str).unwrap_or("").get(..10).unwrap_or("");
                    let loader = item.get("loader").and_then(|loader| loader.get("forge_version").or_else(|| loader.get("neoforge_version"))).and_then(Value::as_str).unwrap_or("");
                    Some(MohistBuild {
                        id,
                        sha256: item.get("file_sha256").and_then(Value::as_str).map(str::to_string),
                        label: format!("#{id} · {date}{}", if loader.is_empty() { String::new() } else { format!(" · loader {loader}") }),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    builds.sort_by_key(|build| std::cmp::Reverse(build.id));
    builds
}

impl Provider for Mohist {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Mohist
    }

    /// The API also lists versions without downloadable builds; those are dropped.
    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move {
            let versions = mohist_versions(&cache::json("mohist", &format!("{MOHIST}/versions"), cache::SHORT).await?);
            let checks = versions.iter().map(|entry| async move {
                let url = format!("{MOHIST}/{}/builds", entry.id);
                let builds = cache::json(&format!("mohist-{}", entry.id), &url, cache::SHORT).await.ok()?;
                builds.as_array().is_some_and(|items| !items.is_empty()).then(|| entry.clone())
            });
            Ok(futures::future::join_all(checks).await.into_iter().flatten().collect())
        })
    }

    fn builds<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            let value = cache::json(&format!("mohist-{version}"), &format!("{MOHIST}/{version}/builds"), cache::SHORT).await?;
            Ok(mohist_builds(&value).into_iter().map(|build| BuildEntry { id: build.id.to_string(), stable: true, label: build.label }).collect())
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let version = request.mc_version;
            let builds = mohist_builds(&cache::fetch(&format!("{MOHIST}/{version}/builds")).await?);
            let build = match request.build {
                Some(wanted) => builds.into_iter().find(|build| build.id.to_string() == wanted),
                None => builds.into_iter().next(),
            }
            .ok_or_else(|| PanelError::not_found(format!("Nessuna build di Mohist per {version}")))?;
            let destination = super::jar_destination(request.root, &format!("mohist-{version}-{}.jar", build.id))?;
            request.progress.emit("Mohist", format!("Download di Mohist {version} #{}...", build.id), Some(0.0));
            let url = format!("{MOHIST}/{version}/builds/{}/download", build.id);
            crate::net::download_checked(&url, &destination, "Mohist", request.progress, build.sha256.map(Checksum::Sha256)).await?;
            Ok(Installed { launch: LaunchSpec::Jar { path: destination }, mc_version: version.to_string(), build: Some(build.id.to_string()) })
        })
    }
}

#[derive(Debug, Clone)]
pub struct ArclightAsset {
    pub name: String,
    pub url: String,
    pub loader: String,
    pub minecraft: String,
    pub release: String,
    pub stable: bool,
    pub sha256: Option<String>,
}

/// Asset names look like `arclight-neoforge-1.21.1-1.0.1-8ec9529.jar`.
pub fn arclight_assets(releases: &Value) -> Vec<ArclightAsset> {
    let mut assets = Vec::new();
    for release in releases.as_array().into_iter().flatten() {
        let tag = release.get("tag_name").and_then(Value::as_str).unwrap_or_default();
        let prerelease = release.get("prerelease").and_then(Value::as_bool).unwrap_or(false);
        for asset in release.get("assets").and_then(Value::as_array).into_iter().flatten() {
            let Some(name) = asset.get("name").and_then(Value::as_str) else { continue };
            let Some(url) = asset.get("browser_download_url").and_then(Value::as_str) else { continue };
            let mut parts = name.trim_end_matches(".jar").split('-');
            if parts.next() != Some("arclight") {
                continue;
            }
            let (Some(loader), Some(minecraft)) = (parts.next(), parts.next()) else { continue };
            assets.push(ArclightAsset {
                name: name.to_string(),
                url: url.to_string(),
                loader: loader.to_string(),
                minecraft: minecraft.to_string(),
                release: tag.to_string(),
                stable: !prerelease && !name.contains("SNAPSHOT"),
                sha256: asset.get("digest").and_then(Value::as_str).and_then(|digest| digest.strip_prefix("sha256:")).map(str::to_string),
            });
        }
    }
    assets
}

async fn arclight_catalog() -> PanelResult<Vec<ArclightAsset>> {
    Ok(arclight_assets(&cache::json("arclight", ARCLIGHT_RELEASES, cache::SHORT).await?))
}

fn loader_name(loader: &str) -> &str {
    match loader {
        "neoforge" => "NeoForge",
        "forge" => "Forge",
        "fabric" => "Fabric",
        other => other,
    }
}

impl Provider for Arclight {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Arclight
    }

    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move {
            let mut versions: Vec<VersionEntry> = Vec::new();
            for asset in arclight_catalog().await? {
                match versions.iter_mut().find(|entry| entry.id == asset.minecraft) {
                    Some(entry) => entry.stable |= asset.stable,
                    None => versions.push(VersionEntry { id: asset.minecraft, stable: asset.stable }),
                }
            }
            super::sort_versions_desc(&mut versions);
            Ok(versions)
        })
    }

    /// The "build" of Arclight is the loader flavour (Forge, NeoForge, Fabric) plus the release.
    fn builds<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            Ok(arclight_catalog()
                .await?
                .into_iter()
                .filter(|asset| asset.minecraft == version)
                .map(|asset| BuildEntry {
                    label: format!("{} · {}{}", loader_name(&asset.loader), asset.release, if asset.stable { "" } else { " · sperimentale" }),
                    stable: asset.stable,
                    id: asset.name,
                })
                .collect())
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let candidates: Vec<ArclightAsset> = arclight_catalog().await?.into_iter().filter(|asset| asset.minecraft == request.mc_version).collect();
            let asset = match request.build {
                Some(name) => candidates.into_iter().find(|asset| asset.name == name),
                None => candidates.iter().find(|asset| asset.stable).cloned().or_else(|| candidates.first().cloned()),
            }
            .ok_or_else(|| PanelError::not_found(format!("Nessuna build di Arclight per {}", request.mc_version)))?;
            let destination = super::jar_destination(request.root, &asset.name)?;
            request.progress.emit("Arclight", format!("Download di {}...", asset.name), Some(0.0));
            crate::net::download_checked(&asset.url, &destination, "Arclight", request.progress, asset.sha256.clone().map(Checksum::Sha256)).await?;
            Ok(Installed { launch: LaunchSpec::Jar { path: destination }, mc_version: request.mc_version.to_string(), build: Some(asset.name) })
        })
    }
}

#[derive(Debug, Clone)]
pub struct SpongeArtifact {
    pub version: String,
    pub minecraft: String,
    pub forge: Option<String>,
    pub recommended: bool,
}

pub fn sponge_artifacts(value: &Value) -> Vec<SpongeArtifact> {
    let mut artifacts: Vec<SpongeArtifact> = value
        .get("artifacts")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(version, info)| {
                    let tags = info.get("tagValues")?;
                    Some(SpongeArtifact {
                        version: version.clone(),
                        minecraft: tags.get("minecraft")?.as_str()?.to_string(),
                        forge: tags.get("forge").and_then(Value::as_str).map(str::to_string),
                        recommended: info.get("recommended").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    artifacts.sort_by(|left, right| super::compare_versions(&right.version, &left.version));
    artifacts
}

impl Sponge {
    async fn artifacts(&self) -> PanelResult<Vec<SpongeArtifact>> {
        let url = format!("{SPONGE}/{}/versions?limit=200", self.artifact);
        Ok(sponge_artifacts(&cache::json(self.artifact, &url, cache::SHORT).await?))
    }
}

impl Provider for Sponge {
    fn kind(&self) -> ProviderKind {
        self.kind
    }

    fn versions(&self, snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move {
            let mut versions: Vec<VersionEntry> = Vec::new();
            for artifact in self.artifacts().await?.into_iter().filter(|artifact| snapshots || !artifact.minecraft.contains('-')) {
                match versions.iter_mut().find(|entry| entry.id == artifact.minecraft) {
                    Some(entry) => entry.stable |= artifact.recommended,
                    None => versions.push(VersionEntry { id: artifact.minecraft, stable: artifact.recommended }),
                }
            }
            super::sort_versions_desc(&mut versions);
            Ok(versions)
        })
    }

    fn builds<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            Ok(self
                .artifacts()
                .await?
                .into_iter()
                .filter(|artifact| artifact.minecraft == version)
                .map(|artifact| BuildEntry {
                    label: if artifact.recommended { format!("{} · consigliata", artifact.version) } else { artifact.version.clone() },
                    stable: artifact.recommended,
                    id: artifact.version,
                })
                .collect())
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let candidates: Vec<SpongeArtifact> = self.artifacts().await?.into_iter().filter(|artifact| artifact.minecraft == request.mc_version).collect();
            let artifact = match request.build {
                Some(wanted) => candidates.into_iter().find(|artifact| artifact.version == wanted),
                None => candidates.iter().find(|artifact| artifact.recommended).cloned().or_else(|| candidates.first().cloned()),
            }
            .ok_or_else(|| PanelError::not_found(format!("Nessuna build di Sponge per {}", request.mc_version)))?;
            let detail = cache::fetch(&format!("{SPONGE}/{}/versions/{}", self.artifact, artifact.version)).await?;
            let universal = detail
                .get("assets")
                .and_then(Value::as_array)
                .and_then(|assets| assets.iter().find(|asset| asset.get("classifier").and_then(Value::as_str) == Some("universal")))
                .ok_or("Jar universal di Sponge non trovato")?;
            let url = universal.get("downloadUrl").and_then(Value::as_str).ok_or("URL di Sponge mancante")?;
            let checksum = universal.get("sha1").and_then(Value::as_str).map(|sha| Checksum::Sha1(sha.to_string()));
            let name = format!("{}-{}-universal.jar", self.artifact, artifact.version);
            if self.kind == ProviderKind::SpongeForge {
                // SpongeForge is a Forge mod: install the matching Forge, then drop the mod into `mods/`.
                let forge = artifact.forge.clone().ok_or("Versione di Forge richiesta da SpongeForge sconosciuta")?;
                let launch = super::forge::Forge::install_version(request.root, request.mc_version, &forge, request.java, request.progress).await?;
                let mods = request.root.join("mods");
                crate::paths::ensure_dir(&mods)?;
                request.progress.emit("Sponge", format!("Download di {name}..."), Some(0.0));
                crate::net::download_checked(url, &mods.join(&name), "Sponge", request.progress, checksum).await?;
                return Ok(Installed { launch, mc_version: request.mc_version.to_string(), build: Some(artifact.version) });
            }
            let destination = super::jar_destination(request.root, &name)?;
            request.progress.emit("Sponge", format!("Download di {name}..."), Some(0.0));
            crate::net::download_checked(url, &destination, "Sponge", request.progress, checksum).await?;
            Ok(Installed { launch: LaunchSpec::Jar { path: destination }, mc_version: request.mc_version.to_string(), build: Some(artifact.version) })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_arclight_releases() {
        let releases = json!([
            {"tag_name": "GroupOfSoldiers/1.0.0-SNAPSHOT", "prerelease": true, "assets": [
                {"name": "arclight-neoforge-1.21.11-1.0.0-SNAPSHOT-8a57510.jar", "browser_download_url": "https://x/a.jar"}
            ]},
            {"tag_name": "FeudalKings/1.0.1", "prerelease": false, "assets": [
                {"name": "arclight-forge-1.21.1-1.0.1-8ec9529.jar", "browser_download_url": "https://x/b.jar", "digest": "sha256:abc"}
            ]}
        ]);
        let assets = arclight_assets(&releases);
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].minecraft, "1.21.11");
        assert!(!assets[0].stable);
        assert_eq!(assets[1].loader, "forge");
        assert_eq!(assets[1].sha256.as_deref(), Some("abc"));
    }

    #[test]
    fn parses_sponge_and_mohist() {
        let sponge = json!({"artifacts": {
            "1.21.10-60.0.1-17.0.0": {"recommended": true, "tagValues": {"api": "17.0.0", "forge": "60.0.1", "minecraft": "1.21.10"}},
            "1.21.8-58.0.5-16.0.0": {"recommended": false, "tagValues": {"minecraft": "1.21.8"}}
        }});
        let artifacts = sponge_artifacts(&sponge);
        assert_eq!(artifacts[0].minecraft, "1.21.10");
        assert_eq!(artifacts[0].forge.as_deref(), Some("60.0.1"));
        let versions = mohist_versions(&json!([{"name": "1.20.1"}, {"name": "1.21.1"}, {"name": "1.7.10"}]));
        assert_eq!(versions[0].id, "1.21.1");
        let builds = mohist_builds(&json!([{"id": 424, "build_date": "2025-12-26T10:19:05Z"}, {"id": 471, "file_sha256": "cb", "build_date": "2026-01-16T13:01:42Z", "loader": {"forge_version": "47.4.13"}}]));
        assert_eq!(builds[0].id, 471);
        assert!(builds[0].label.contains("47.4.13"));
    }
}
