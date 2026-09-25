// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{ProviderKind, VersionEntry};
use crate::net::Checksum;
use crate::{cache, LaunchSpec, PanelError, PanelResult};

const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
/// Oldest release shipping a standalone server jar in the manifest (1.2.5).
const FIRST_SERVER_RELEASE: &str = "2012-03-29";

pub struct Vanilla;
pub static VANILLA: Vanilla = Vanilla;

pub async fn manifest() -> PanelResult<Value> {
    cache::json("mojang-manifest", MANIFEST, cache::SHORT).await
}

pub fn versions_from_manifest(manifest: &Value, snapshots: bool) -> Vec<VersionEntry> {
    manifest
        .get("versions")
        .and_then(Value::as_array)
        .map(|versions| {
            versions
                .iter()
                .filter(|entry| entry.get("releaseTime").and_then(Value::as_str).unwrap_or("") >= FIRST_SERVER_RELEASE)
                .filter_map(|entry| {
                    let id = entry.get("id")?.as_str()?;
                    let kind = entry.get("type")?.as_str()?;
                    match kind {
                        "release" => Some(VersionEntry { id: id.to_string(), stable: true }),
                        "snapshot" if snapshots => Some(VersionEntry { id: id.to_string(), stable: false }),
                        _ => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Per-version metadata; immutable, so it is cached for a long time.
pub async fn version_meta(version: &str) -> PanelResult<Value> {
    let manifest = manifest().await?;
    let url = manifest
        .get("versions")
        .and_then(Value::as_array)
        .and_then(|versions| versions.iter().find(|entry| entry.get("id").and_then(Value::as_str) == Some(version)))
        .and_then(|entry| entry.get("url"))
        .and_then(Value::as_str)
        .ok_or_else(|| PanelError::not_found(format!("Versione Minecraft {version} non trovata nel manifest Mojang")))?
        .to_string();
    cache::json(&format!("mojang-{version}"), &url, cache::LONG).await
}

pub fn java_major_from_meta(value: &Value) -> u32 {
    value
        .get("javaVersion")
        .and_then(|entry| entry.get("majorVersion"))
        .and_then(Value::as_u64)
        .unwrap_or(8) as u32
}

pub async fn required_java(version: &str) -> PanelResult<u32> {
    Ok(java_major_from_meta(&version_meta(version).await?))
}

impl Provider for Vanilla {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Vanilla
    }

    fn versions(&self, snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move { Ok(versions_from_manifest(&manifest().await?, snapshots)) })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let meta = version_meta(request.mc_version).await?;
            let server = meta
                .get("downloads")
                .and_then(|downloads| downloads.get("server"))
                .ok_or_else(|| PanelError::not_found(format!("Minecraft {} non ha un server scaricabile.", request.mc_version)))?;
            let url = server.get("url").and_then(Value::as_str).ok_or("URL del server Vanilla mancante")?;
            let checksum = server.get("sha1").and_then(Value::as_str).map(|sha1| Checksum::Sha1(sha1.to_string()));
            let destination = super::jar_destination(request.root, &format!("minecraft_server.{}.jar", request.mc_version))?;
            request.progress.emit("Vanilla", format!("Download Minecraft {}...", request.mc_version), Some(0.0));
            crate::net::download_checked(url, &destination, "Vanilla", request.progress, checksum).await?;
            Ok(Installed {
                launch: LaunchSpec::Jar { path: destination },
                mc_version: request.mc_version.to_string(),
                build: None,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn filters_releases_and_snapshots() {
        let manifest = json!({"versions": [
            {"id": "26.4-snapshot-1", "type": "snapshot", "releaseTime": "2026-09-22T13:38:53+00:00"},
            {"id": "26.3", "type": "release", "releaseTime": "2026-09-01T00:00:00+00:00"},
            {"id": "b1.7.3", "type": "old_beta", "releaseTime": "2011-07-08T00:00:00+00:00"},
            {"id": "1.0", "type": "release", "releaseTime": "2011-11-17T00:00:00+00:00"}
        ]});
        let stable = versions_from_manifest(&manifest, false);
        assert_eq!(stable.len(), 1);
        assert_eq!(stable[0].id, "26.3");
        let all = versions_from_manifest(&manifest, true);
        assert_eq!(all.len(), 2);
        assert!(!all[0].stable);
    }

    #[test]
    fn reads_java_major() {
        assert_eq!(java_major_from_meta(&json!({"javaVersion": {"majorVersion": 21}})), 21);
        assert_eq!(java_major_from_meta(&json!({})), 8);
    }
}
