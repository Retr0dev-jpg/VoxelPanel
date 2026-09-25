// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{BuildEntry, ProviderKind, VersionEntry};
use crate::net::Checksum;
use crate::{cache, LaunchSpec, PanelError, PanelResult};

const API: &str = "https://api.leafmc.one/v2/projects/leaf";

pub struct Leaf;
pub static LEAF: Leaf = Leaf;

pub fn versions_from_json(value: &Value) -> Vec<VersionEntry> {
    let mut entries: Vec<VersionEntry> = value
        .get("versions")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(|id| VersionEntry { id: id.to_string(), stable: true }).collect())
        .unwrap_or_default();
    super::sort_versions_desc(&mut entries);
    entries
}

struct LeafBuild {
    number: u64,
    stable: bool,
    date: String,
    file: String,
    sha256: Option<String>,
}

fn parse_builds(value: &Value) -> Vec<LeafBuild> {
    let mut builds: Vec<LeafBuild> = value
        .get("builds")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|build| {
                    let primary = build.get("downloads")?.get("primary")?;
                    Some(LeafBuild {
                        number: build.get("build")?.as_u64()?,
                        stable: build.get("channel").and_then(Value::as_str) != Some("experimental"),
                        date: build.get("time").and_then(Value::as_str).unwrap_or("").get(..10).unwrap_or("").to_string(),
                        file: primary.get("name")?.as_str()?.to_string(),
                        sha256: primary.get("sha256").and_then(Value::as_str).map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    builds.sort_by_key(|build| std::cmp::Reverse(build.number));
    builds
}

async fn builds_for(version: &str) -> PanelResult<Vec<LeafBuild>> {
    let value = cache::json(&format!("leaf-{version}"), &format!("{API}/versions/{version}/builds"), cache::SHORT).await?;
    Ok(parse_builds(&value))
}

impl Provider for Leaf {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Leaf
    }

    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move { Ok(versions_from_json(&cache::json("leaf", API, cache::SHORT).await?)) })
    }

    fn builds<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            Ok(builds_for(version)
                .await?
                .into_iter()
                .map(|build| BuildEntry {
                    id: build.number.to_string(),
                    stable: build.stable,
                    label: format!("#{} · {}{}", build.number, build.date, if build.stable { "" } else { " · experimental" }),
                })
                .collect())
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let version = request.mc_version;
            let builds = builds_for(version).await?;
            let build = match request.build {
                Some(wanted) => builds.into_iter().find(|build| build.number.to_string() == wanted),
                None => builds.into_iter().next(),
            }
            .ok_or_else(|| PanelError::not_found(format!("Nessuna build di Leaf per {version}")))?;
            let destination = super::jar_destination(request.root, &build.file)?;
            let url = format!("{API}/versions/{version}/builds/{}/downloads/{}", build.number, build.file);
            request.progress.emit("Leaf", format!("Download {}...", build.file), Some(0.0));
            crate::net::download_checked(&url, &destination, "Leaf", request.progress, build.sha256.map(Checksum::Sha256)).await?;
            Ok(Installed {
                launch: LaunchSpec::Jar { path: destination },
                mc_version: version.to_string(),
                build: Some(build.number.to_string()),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_builds() {
        let value = json!({"builds": [
            {"build": 2, "time": "2026-01-15T22:58:04Z", "channel": "experimental", "downloads": {"primary": {"name": "leaf-1.21.11-2.jar", "sha256": "aa"}}},
            {"build": 5, "time": "2026-02-01T10:00:00Z", "channel": "default", "downloads": {"primary": {"name": "leaf-1.21.11-5.jar", "sha256": "bb"}}}
        ]});
        let builds = parse_builds(&value);
        assert_eq!(builds[0].number, 5);
        assert!(builds[0].stable);
        assert!(!builds[1].stable);
    }
}
