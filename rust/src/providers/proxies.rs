// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{BuildEntry, ProviderKind, VersionEntry};
use crate::{cache, LaunchSpec, PanelResult};

const JENKINS: &str = "https://ci.md-5.net/job/BungeeCord";
const ARTIFACT: &str = "bootstrap/target/BungeeCord.jar";

/// BungeeCord has no Minecraft-specific versions: one rolling "latest" line with Jenkins builds.
pub struct BungeeCord;
pub static BUNGEECORD: BungeeCord = BungeeCord;

pub fn builds_from_jenkins(value: &Value) -> Vec<BuildEntry> {
    value
        .get("builds")
        .and_then(Value::as_array)
        .map(|builds| {
            builds
                .iter()
                .filter(|build| build.get("result").and_then(Value::as_str) == Some("SUCCESS"))
                .filter_map(|build| build.get("number")?.as_u64())
                .map(|number| BuildEntry { id: number.to_string(), stable: true, label: format!("#{number}") })
                .collect()
        })
        .unwrap_or_default()
}

impl Provider for BungeeCord {
    fn kind(&self) -> ProviderKind {
        ProviderKind::BungeeCord
    }

    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async { Ok(vec![VersionEntry { id: "latest".into(), stable: true }]) })
    }

    fn builds<'a>(&'a self, _version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            let value = cache::json("bungeecord", &format!("{JENKINS}/api/json?tree=builds[number,result]{{0,25}}"), cache::SHORT).await?;
            Ok(builds_from_jenkins(&value))
        })
    }

    fn required_java<'a>(&'a self, _version: &'a str) -> BoxFuture<'a, PanelResult<u32>> {
        Box::pin(async { Ok(21) })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let build = match request.build {
                Some(build) => build.to_string(),
                None => cache::fetch(&format!("{JENKINS}/lastSuccessfulBuild/api/json?tree=number"))
                    .await?
                    .get("number")
                    .and_then(Value::as_u64)
                    .ok_or("Build BungeeCord senza numero")?
                    .to_string(),
            };
            let destination = super::jar_destination(request.root, "BungeeCord.jar")?;
            request.progress.emit("BungeeCord", format!("Download di BungeeCord #{build}..."), Some(0.0));
            crate::net::download(&format!("{JENKINS}/{build}/artifact/{ARTIFACT}"), &destination, "BungeeCord", request.progress).await?;
            Ok(Installed { launch: LaunchSpec::Jar { path: destination }, mc_version: "latest".into(), build: Some(build) })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_successful_builds() {
        let value = json!({"builds": [{"number": 1900, "result": "SUCCESS"}, {"number": 1899, "result": "FAILURE"}, {"number": 1898, "result": "SUCCESS"}]});
        let builds = builds_from_jenkins(&value);
        assert_eq!(builds.iter().map(|build| build.id.as_str()).collect::<Vec<_>>(), vec!["1900", "1898"]);
    }
}
