// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{ProviderKind, VersionEntry};
use crate::{cache, LaunchSpec, PanelError, PanelResult};

const JENKINS: &str = "https://ci.pufferfish.host";

/// Pufferfish publishes one Jenkins job per Minecraft line (`Pufferfish-1.21`); the exact
/// version is read from the artifact name of the last successful build.
pub struct Pufferfish;
pub static PUFFERFISH: Pufferfish = Pufferfish;

pub fn versions_from_jobs(value: &Value) -> Vec<VersionEntry> {
    let mut entries: Vec<VersionEntry> = value
        .get("jobs")
        .and_then(Value::as_array)
        .map(|jobs| {
            jobs.iter()
                .filter_map(|job| job.get("name")?.as_str()?.strip_prefix("Pufferfish-"))
                .filter(|line| line.chars().all(|ch| ch.is_ascii_digit() || ch == '.'))
                .map(|line| VersionEntry { id: line.to_string(), stable: true })
                .collect()
        })
        .unwrap_or_default();
    super::sort_versions_desc(&mut entries);
    entries
}

/// Prefers the Mojang-mapped paperclip jar, which modern Paper plugins expect.
pub fn pick_artifact(build: &Value) -> Option<(String, String)> {
    let artifacts = build.get("artifacts")?.as_array()?;
    let jars: Vec<(String, String)> = artifacts
        .iter()
        .filter_map(|artifact| {
            let name = artifact.get("fileName")?.as_str()?;
            let path = artifact.get("relativePath")?.as_str()?;
            (name.ends_with(".jar") && name.contains("paperclip")).then(|| (name.to_string(), path.to_string()))
        })
        .collect();
    jars.iter().find(|(name, _)| name.contains("mojmap")).or_else(|| jars.first()).cloned()
}

impl Provider for Pufferfish {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Pufferfish
    }

    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move {
            let value = cache::json("pufferfish", &format!("{JENKINS}/api/json?tree=jobs[name]"), cache::SHORT).await?;
            Ok(versions_from_jobs(&value))
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let job = format!("{JENKINS}/job/Pufferfish-{}/lastSuccessfulBuild", request.mc_version);
            let build = cache::fetch(&format!("{job}/api/json?tree=number,artifacts[fileName,relativePath]")).await?;
            let number = build.get("number").and_then(Value::as_u64).map(|number| number.to_string());
            let (name, path) = pick_artifact(&build).ok_or_else(|| PanelError::not_found("Nessun jar Pufferfish nell'ultima build"))?;
            let destination = super::jar_destination(request.root, &name)?;
            request.progress.emit("Pufferfish", format!("Download {name}..."), Some(0.0));
            crate::net::download(&format!("{job}/artifact/{path}"), &destination, "Pufferfish", request.progress).await?;
            Ok(Installed {
                mc_version: super::version_in_name(&name).unwrap_or_else(|| request.mc_version.to_string()),
                launch: LaunchSpec::Jar { path: destination },
                build: number,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_jobs_and_artifacts() {
        let jobs = json!({"jobs": [{"name": "Pufferfish-1.18"}, {"name": "Pufferfish-1.21"}, {"name": "Pufferfish-Plus"}]});
        let versions = versions_from_jobs(&jobs);
        assert_eq!(versions.iter().map(|entry| entry.id.as_str()).collect::<Vec<_>>(), vec!["1.21", "1.18"]);
        let build = json!({"number": 39, "artifacts": [
            {"fileName": "pufferfish-paperclip-1.21.10-R0.1-SNAPSHOT-reobf.jar", "relativePath": "a/reobf.jar"},
            {"fileName": "pufferfish-paperclip-1.21.10-R0.1-SNAPSHOT-mojmap.jar", "relativePath": "a/mojmap.jar"}
        ]});
        assert_eq!(pick_artifact(&build).unwrap().1, "a/mojmap.jar");
        assert_eq!(super::super::version_in_name("pufferfish-paperclip-1.21.10-R0.1-SNAPSHOT-mojmap.jar").as_deref(), Some("1.21.10"));
    }
}
