// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{BuildEntry, ProviderKind, VersionEntry};
use crate::net::Checksum;
use crate::{cache, LaunchSpec, PanelResult};

const API: &str = "https://api.purpurmc.org/v2/purpur";

pub struct Purpur;
pub static PURPUR: Purpur = Purpur;

pub fn versions_from_json(value: &Value) -> Vec<VersionEntry> {
    let mut entries: Vec<VersionEntry> = value
        .get("versions")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(|id| VersionEntry { id: id.to_string(), stable: true }).collect())
        .unwrap_or_default();
    super::sort_versions_desc(&mut entries);
    entries
}

pub fn builds_from_json(value: &Value) -> Vec<BuildEntry> {
    let mut builds: Vec<BuildEntry> = value
        .get("builds")
        .and_then(|builds| builds.get("all"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|id| BuildEntry { id: id.to_string(), stable: true, label: format!("#{id}") })
                .collect()
        })
        .unwrap_or_default();
    builds.sort_by_key(|build| std::cmp::Reverse(build.id.parse::<u64>().unwrap_or(0)));
    builds
}

impl Provider for Purpur {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Purpur
    }

    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move { Ok(versions_from_json(&cache::json("purpur", API, cache::SHORT).await?)) })
    }

    fn builds<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            let value = cache::json(&format!("purpur-{version}"), &format!("{API}/{version}"), cache::SHORT).await?;
            Ok(builds_from_json(&value))
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let version = request.mc_version;
            let info = cache::fetch(&format!("{API}/{version}/{}", request.build.unwrap_or("latest"))).await?;
            let build = info.get("build").and_then(Value::as_str).ok_or("Build Purpur senza numero")?.to_string();
            let checksum = info.get("md5").and_then(Value::as_str).map(|md5| Checksum::Md5(md5.to_string()));
            let destination = super::jar_destination(request.root, &format!("purpur-{version}-{build}.jar"))?;
            request.progress.emit("Purpur", format!("Download Purpur {version} build {build}..."), Some(0.0));
            crate::net::download_checked(&format!("{API}/{version}/{build}/download"), &destination, "Purpur", request.progress, checksum).await?;
            Ok(Installed {
                launch: LaunchSpec::Jar { path: destination },
                mc_version: version.to_string(),
                build: Some(build),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_versions_and_builds() {
        let versions = versions_from_json(&json!({"versions": ["1.21.10", "26.2", "1.21.11"]}));
        assert_eq!(versions[0].id, "26.2");
        let builds = builds_from_json(&json!({"builds": {"latest": "2568", "all": ["2536", "2568"]}}));
        assert_eq!(builds[0].id, "2568");
    }
}
