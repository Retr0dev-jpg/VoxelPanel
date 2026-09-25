// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! PaperMC projects served by the Fill v3 API: Paper, Folia and Velocity.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{BuildEntry, ProviderKind, VersionEntry};
use crate::net::Checksum;
use crate::{cache, LaunchSpec, PanelError, PanelResult};

const FILL: &str = "https://fill.papermc.io/v3/projects";

pub struct Fill {
    kind: ProviderKind,
    project: &'static str,
}

pub static PAPER: Fill = Fill { kind: ProviderKind::Paper, project: "paper" };
pub static FOLIA: Fill = Fill { kind: ProviderKind::Folia, project: "folia" };
pub static VELOCITY: Fill = Fill { kind: ProviderKind::Velocity, project: "velocity" };
pub static WATERFALL: Fill = Fill { kind: ProviderKind::Waterfall, project: "waterfall" };

/// Flattens the `versions` object (grouped by family) and sorts newest first.
pub fn versions_from_project_json(value: &Value, snapshots: bool) -> PanelResult<Vec<VersionEntry>> {
    let versions = value.get("versions").ok_or("Campo versions mancante nella risposta di PaperMC")?;
    let strings = |items: &[Value]| items.iter().filter_map(Value::as_str).map(str::to_string).collect::<Vec<_>>();
    let mut found: Vec<String> = match versions {
        Value::Array(items) => strings(items),
        Value::Object(map) => map
            .values()
            .flat_map(|value| match value {
                Value::String(text) => vec![text.clone()],
                Value::Array(items) => strings(items),
                _ => Vec::new(),
            })
            .collect(),
        _ => return Err(PanelError::invalid("Formato delle versioni PaperMC non riconosciuto")),
    };
    found.sort();
    found.dedup();
    let mut entries: Vec<VersionEntry> = found
        .into_iter()
        .map(|id| VersionEntry { stable: !id.contains('-'), id })
        .filter(|entry| snapshots || entry.stable)
        .collect();
    super::sort_versions_desc(&mut entries);
    Ok(entries)
}

pub fn builds_from_json(value: &Value) -> Vec<BuildEntry> {
    let mut builds: Vec<BuildEntry> = value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|build| {
                    let id = build.get("id")?.as_u64()?;
                    let channel = build.get("channel").and_then(Value::as_str).unwrap_or("STABLE");
                    let date = build.get("time").and_then(Value::as_str).unwrap_or("").get(..10).unwrap_or("");
                    Some(BuildEntry {
                        id: id.to_string(),
                        stable: matches!(channel, "STABLE" | "RECOMMENDED"),
                        label: format!("#{id} · {date} · {}", channel.to_lowercase()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    builds.sort_by_key(|build| std::cmp::Reverse(build.id.parse::<u64>().unwrap_or(0)));
    builds
}

/// Velocity 3.3 moved to Java 17 and 3.4 to Java 21.
fn velocity_java(version: &str) -> u32 {
    let mut parts = version.split(['.', '-']).filter_map(|part| part.parse::<u32>().ok());
    match (parts.next().unwrap_or(0), parts.next().unwrap_or(0)) {
        (major, _) if major < 3 => 11,
        (3, minor) if minor < 3 => 11,
        (3, 3) => 17,
        _ => 21,
    }
}

impl Provider for Fill {
    fn kind(&self) -> ProviderKind {
        self.kind
    }

    fn versions(&self, snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move {
            let url = format!("{FILL}/{}", self.project);
            let project = cache::json(&format!("fill-{}", self.project), &url, cache::SHORT).await?;
            versions_from_project_json(&project, snapshots)
        })
    }

    fn builds<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            let url = format!("{FILL}/{}/versions/{version}/builds", self.project);
            let value = cache::json(&format!("fill-{}-{version}", self.project), &url, cache::SHORT).await?;
            Ok(builds_from_json(&value))
        })
    }

    fn required_java<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<u32>> {
        Box::pin(async move {
            match self.kind {
                ProviderKind::Velocity => Ok(velocity_java(version)),
                ProviderKind::Waterfall => Ok(17),
                _ => super::mojang::required_java(version).await,
            }
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let build = request.build.unwrap_or("latest");
            let url = format!("{FILL}/{}/versions/{}/builds/{build}", self.project, request.mc_version);
            let value = cache::fetch(&url).await?;
            let id = value.get("id").and_then(Value::as_u64).ok_or("Build senza id")?;
            let download = value
                .get("downloads")
                .and_then(|entry| entry.get("server:default"))
                .ok_or("Download server:default mancante")?;
            let name = download.get("name").and_then(Value::as_str).ok_or("Nome del jar mancante")?;
            let url = download.get("url").and_then(Value::as_str).ok_or("URL del jar mancante")?;
            let checksum = download
                .get("checksums")
                .and_then(|checksums| checksums.get("sha256"))
                .and_then(Value::as_str)
                .map(|sha| Checksum::Sha256(sha.to_string()));
            let destination = super::jar_destination(request.root, name)?;
            if !destination.exists() {
                request.progress.emit(self.project, format!("Download {name}..."), Some(0.0));
                crate::net::download_checked(url, &destination, self.project, request.progress, checksum).await?;
            }
            Ok(Installed {
                launch: LaunchSpec::Jar { path: destination },
                mc_version: request.mc_version.to_string(),
                build: Some(id.to_string()),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flattens_and_sorts_versions() {
        let value = json!({"versions": {"26.3": ["26.3", "26.3-rc-3"], "1.21": ["1.21.11", "1.21.11-rc3", "1.21"]}});
        let stable: Vec<String> = versions_from_project_json(&value, false).unwrap().into_iter().map(|entry| entry.id).collect();
        assert_eq!(stable, vec!["26.3", "1.21.11", "1.21"]);
        assert_eq!(versions_from_project_json(&value, true).unwrap().len(), 5);
    }

    #[test]
    fn reads_builds_newest_first() {
        let value = json!([
            {"id": 131, "time": "2026-05-03T16:08:07Z", "channel": "STABLE"},
            {"id": 132, "time": "2026-05-11T11:43:09Z", "channel": "BETA"}
        ]);
        let builds = builds_from_json(&value);
        assert_eq!(builds[0].id, "132");
        assert!(!builds[0].stable);
        assert!(builds[1].label.starts_with("#131 · 2026-05-03"));
    }

    #[test]
    fn velocity_java_requirements() {
        assert_eq!(velocity_java("3.1.1"), 11);
        assert_eq!(velocity_java("3.3.0-SNAPSHOT"), 17);
        assert_eq!(velocity_java("3.5.0"), 21);
        assert_eq!(velocity_java("4.2.0"), 21);
    }
}
