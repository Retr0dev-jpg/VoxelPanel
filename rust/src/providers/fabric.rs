// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Fabric (server launcher jar) and Quilt (installer run in server mode).

use futures::future::BoxFuture;
use serde_json::Value;

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{BuildEntry, ProviderKind, VersionEntry};
use crate::net::Checksum;
use crate::{cache, LaunchSpec, PanelError, PanelResult};

const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
const QUILT_META: &str = "https://meta.quiltmc.org/v3";

pub struct Fabric;
pub static FABRIC: Fabric = Fabric;
pub struct Quilt;
pub static QUILT: Quilt = Quilt;

/// `[{"version": "1.21.1", "stable": true}, ...]` as returned by both meta services.
pub fn entries_from_json(value: &Value, snapshots: bool) -> Vec<VersionEntry> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = item.get("version")?.as_str()?;
                    let stable = item.get("stable").and_then(Value::as_bool).unwrap_or(!id.contains('-'));
                    (snapshots || stable).then(|| VersionEntry { id: id.to_string(), stable })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn loaders_from_json(value: &Value) -> Vec<BuildEntry> {
    entries_from_json(value, true)
        .into_iter()
        .map(|entry| BuildEntry {
            label: if entry.stable { entry.id.clone() } else { format!("{} · beta", entry.id) },
            stable: entry.stable && !entry.id.contains("beta"),
            id: entry.id,
        })
        .collect()
}

async fn maven_sha256(url: &str) -> Option<String> {
    let text = crate::net::http().get(format!("{url}.sha256")).send().await.ok()?.error_for_status().ok()?.text().await.ok()?;
    let hash = text.split_whitespace().next()?.to_lowercase();
    (hash.len() == 64 && hash.chars().all(|ch| ch.is_ascii_hexdigit())).then_some(hash)
}

fn newest_stable(builds: &[BuildEntry]) -> Option<String> {
    builds.iter().find(|build| build.stable).or_else(|| builds.first()).map(|build| build.id.clone())
}

impl Provider for Fabric {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Fabric
    }

    fn versions(&self, snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move {
            let value = cache::json("fabric-game", &format!("{FABRIC_META}/versions/game"), cache::SHORT).await?;
            Ok(entries_from_json(&value, snapshots))
        })
    }

    fn builds<'a>(&'a self, _version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            let value = cache::json("fabric-loader", &format!("{FABRIC_META}/versions/loader"), cache::SHORT).await?;
            Ok(loaders_from_json(&value))
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let loader = match request.build {
                Some(build) => build.to_string(),
                None => newest_stable(&self.builds(request.mc_version).await?).ok_or("Nessun loader Fabric disponibile")?,
            };
            let installers = cache::json("fabric-installer", &format!("{FABRIC_META}/versions/installer"), cache::SHORT).await?;
            let installer = entries_from_json(&installers, false).into_iter().next().ok_or("Nessun installer Fabric disponibile")?.id;
            let version = request.mc_version;
            let name = format!("fabric-server-mc.{version}-loader.{loader}-launcher.{installer}.jar");
            let destination = super::jar_destination(request.root, &name)?;
            let url = format!("{FABRIC_META}/versions/loader/{version}/{loader}/{installer}/server/jar");
            request.progress.emit("Fabric", format!("Download del launcher Fabric {loader}..."), Some(0.0));
            crate::net::download(&url, &destination, "Fabric", request.progress).await?;
            // The launcher downloads the vanilla server on first start; fetching it now avoids a slow first boot.
            request.progress.emit("Fabric", format!("Download di Minecraft {version}..."), None);
            let vanilla = super::mojang::version_meta(version).await?;
            if let Some(server) = vanilla.get("downloads").and_then(|downloads| downloads.get("server")) {
                let url = server.get("url").and_then(Value::as_str).ok_or("URL del server Vanilla mancante")?;
                let checksum = server.get("sha1").and_then(Value::as_str).map(|sha| Checksum::Sha1(sha.to_string()));
                crate::net::download_checked(url, &request.root.join("server.jar"), "Minecraft", request.progress, checksum).await?;
            }
            Ok(Installed {
                launch: LaunchSpec::Jar { path: destination },
                mc_version: version.to_string(),
                build: Some(loader),
            })
        })
    }
}

impl Provider for Quilt {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Quilt
    }

    fn versions(&self, snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move {
            let value = cache::json("quilt-game", &format!("{QUILT_META}/versions/game"), cache::SHORT).await?;
            Ok(entries_from_json(&value, snapshots))
        })
    }

    fn builds<'a>(&'a self, _version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            let value = cache::json("quilt-loader", &format!("{QUILT_META}/versions/loader"), cache::SHORT).await?;
            Ok(loaders_from_json(&value))
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let loader = match request.build {
                Some(build) => build.to_string(),
                None => newest_stable(&self.builds(request.mc_version).await?).ok_or("Nessun loader Quilt disponibile")?,
            };
            let installers = cache::json("quilt-installer", &format!("{QUILT_META}/versions/installer"), cache::SHORT).await?;
            let installer = installers.as_array().and_then(|items| items.first()).ok_or("Nessun installer Quilt disponibile")?;
            let url = installer.get("url").and_then(Value::as_str).ok_or("URL dell'installer Quilt mancante")?;
            // The meta service publishes stale hashes; the `.sha256` next to the jar on Maven is authoritative.
            let checksum = maven_sha256(url).await.map(Checksum::Sha256);
            let jar = request.root.join("quilt-installer.jar");
            request.progress.emit("Quilt", "Download dell'installer Quilt...", Some(0.0));
            crate::net::download_checked(url, &jar, "Quilt", request.progress, checksum).await?;
            let install_dir = format!("--install-dir={}", request.root.display());
            super::forge::run_installer(
                request.java,
                &jar,
                &["install", "server", request.mc_version, &loader, "--download-server", &install_dir],
                request.root,
                "Quilt",
                request.progress,
            )
            .await?;
            let _ = std::fs::remove_file(&jar);
            let launcher = request.root.join("quilt-server-launch.jar");
            if !launcher.is_file() {
                return Err(PanelError::not_found("L'installer di Quilt non ha creato quilt-server-launch.jar"));
            }
            Ok(Installed {
                launch: LaunchSpec::Jar { path: launcher },
                mc_version: request.mc_version.to_string(),
                build: Some(loader),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_meta_versions_and_loaders() {
        let game = json!([{"version": "26.4-snapshot-1", "stable": false}, {"version": "26.3", "stable": true}]);
        assert_eq!(entries_from_json(&game, false).len(), 1);
        assert_eq!(entries_from_json(&game, true).len(), 2);
        let loaders = loaders_from_json(&json!([{"version": "0.31.0-beta.4"}, {"version": "0.30.0"}]));
        assert!(!loaders[0].stable);
        assert_eq!(newest_stable(&loaders).as_deref(), Some("0.30.0"));
    }
}
