// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{ContentSource, Dependency, Download, Target, PAGE_SIZE};
use crate::api::types::{AddonKind, ContentPage, ContentProject, ContentSourceKind, ContentVersion, ProviderKind};
use crate::net::Checksum;
use crate::{PanelError, PanelResult};

const API: &str = "https://hangar.papermc.io/api/v1";

/// PaperMC's plugin repository (Paper, Velocity and Waterfall plugins).
pub struct Hangar;
pub static HANGAR: Hangar = Hangar;

pub fn platform(provider: ProviderKind) -> Option<&'static str> {
    match provider {
        ProviderKind::Velocity => Some("VELOCITY"),
        ProviderKind::Waterfall | ProviderKind::BungeeCord => Some("WATERFALL"),
        provider if super::is_bukkit_family(provider) => Some("PAPER"),
        _ => None,
    }
}

async fn get_json(url: &str, query: &[(&str, String)]) -> PanelResult<Value> {
    Ok(crate::net::http()
        .get(url)
        .query(query)
        .send()
        .await
        .map_err(|error| PanelError::network(format!("Hangar non raggiungibile: {error}")))?
        .error_for_status()?
        .json()
        .await?)
}

pub fn project_from_json(value: &Value) -> ContentProject {
    let namespace = value.get("namespace");
    let owner = namespace.and_then(|namespace| namespace.get("owner")).and_then(Value::as_str).unwrap_or_default();
    let slug = namespace.and_then(|namespace| namespace.get("slug")).and_then(Value::as_str).unwrap_or_default();
    ContentProject {
        source: ContentSourceKind::Hangar,
        id: value.get("id").and_then(Value::as_i64).map(|id| id.to_string()).unwrap_or_default(),
        slug: slug.to_string(),
        title: value.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        description: value.get("description").and_then(Value::as_str).unwrap_or_default().to_string(),
        author: owner.to_string(),
        downloads: value.get("stats").and_then(|stats| stats.get("downloads")).and_then(Value::as_i64).unwrap_or(0),
        icon_url: value.get("avatarUrl").and_then(Value::as_str).unwrap_or_default().to_string(),
        page_url: format!("https://hangar.papermc.io/{owner}/{slug}"),
    }
}

pub fn version_from_json(target: &Target, platform: &str, value: &Value) -> ContentVersion {
    let game_versions: Vec<String> = value
        .get("platformDependencies")
        .and_then(|dependencies| dependencies.get(platform))
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();
    let channel = value.get("channel").and_then(|channel| channel.get("name")).and_then(Value::as_str).unwrap_or("Release");
    let has_platform = value.get("downloads").and_then(|downloads| downloads.get(platform)).is_some();
    ContentVersion {
        id: value.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        name: value.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        compatible: has_platform && (game_versions.is_empty() || target.accepts_version(&game_versions)),
        stable: channel.eq_ignore_ascii_case("release"),
        published: value.get("createdAt").and_then(Value::as_str).unwrap_or_default().to_string(),
        loaders: vec![platform.to_lowercase()],
        game_versions,
    }
}

impl ContentSource for Hangar {
    fn kind(&self) -> ContentSourceKind {
        ContentSourceKind::Hangar
    }

    fn supports(&self, target: &Target) -> bool {
        target.kind == AddonKind::Plugin && platform(target.provider).is_some() && !matches!(target.provider, ProviderKind::SpongeVanilla | ProviderKind::SpongeForge)
    }

    fn search<'a>(&'a self, target: &'a Target, query: &'a str, page: u32) -> BoxFuture<'a, PanelResult<ContentPage>> {
        Box::pin(async move {
            let platform = platform(target.provider).unwrap_or("PAPER");
            let mut params = vec![
                ("q", query.to_string()),
                ("limit", PAGE_SIZE.to_string()),
                ("offset", (page * PAGE_SIZE).to_string()),
                ("platform", platform.to_string()),
                ("sort", if query.is_empty() { "-downloads".into() } else { "-stars".into() }),
            ];
            if let (Some(version), "PAPER") = (&target.game_version, platform) {
                params.push(("version", version.clone()));
            }
            let value = get_json(&format!("{API}/projects"), &params).await?;
            Ok(ContentPage {
                projects: value.get("result").and_then(Value::as_array).map(|items| items.iter().map(project_from_json).collect()).unwrap_or_default(),
                total: value.get("pagination").and_then(|pagination| pagination.get("count")).and_then(Value::as_u64).unwrap_or(0) as u32,
            })
        })
    }

    fn versions<'a>(&'a self, target: &'a Target, project_id: &'a str) -> BoxFuture<'a, PanelResult<Vec<ContentVersion>>> {
        Box::pin(async move {
            let platform = platform(target.provider).unwrap_or("PAPER");
            let value = get_json(&format!("{API}/projects/{project_id}/versions"), &[("limit", "25".into()), ("platform", platform.into())]).await?;
            let mut versions: Vec<ContentVersion> = value.get("result").and_then(Value::as_array).map(|items| items.iter().map(|item| version_from_json(target, platform, item)).collect()).unwrap_or_default();
            super::sort_versions(&mut versions);
            Ok(versions)
        })
    }

    fn download<'a>(&'a self, target: &'a Target, project_id: &'a str, version_id: &'a str) -> BoxFuture<'a, PanelResult<Download>> {
        Box::pin(async move {
            let platform = platform(target.provider).unwrap_or("PAPER");
            let version_id = super::pick_version(&self.versions(target, project_id).await?, version_id)?;
            let encoded: String = version_id.bytes().map(|byte| if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) { (byte as char).to_string() } else { format!("%{byte:02X}") }).collect();
            let value = get_json(&format!("{API}/projects/{project_id}/versions/{encoded}"), &[]).await?;
            let download = value.get("downloads").and_then(|downloads| downloads.get(platform)).ok_or_else(|| PanelError::not_found("Nessun file per questa piattaforma"))?;
            let url = download
                .get("downloadUrl")
                .and_then(Value::as_str)
                .ok_or_else(|| PanelError::invalid("Il plugin è ospitato altrove: scaricalo dalla pagina del progetto"))?;
            let info = download.get("fileInfo");
            let file_name = info.and_then(|info| info.get("name")).and_then(Value::as_str).unwrap_or("plugin.jar").to_string();
            let checksum = info.and_then(|info| info.get("sha256Hash")).and_then(Value::as_str).map(|hash| Checksum::Sha256(hash.to_string()));
            let mut dependencies = Vec::new();
            let mut manual = Vec::new();
            for dependency in value.get("pluginDependencies").and_then(|dependencies| dependencies.get(platform)).and_then(Value::as_array).into_iter().flatten() {
                if dependency.get("required").and_then(Value::as_bool) != Some(true) {
                    continue;
                }
                match dependency.get("projectId").and_then(Value::as_i64) {
                    Some(id) => dependencies.push(Dependency { source: ContentSourceKind::Hangar, project_id: id.to_string(), version_id: None }),
                    None => manual.push(dependency.get("name").and_then(Value::as_str).unwrap_or("?").to_string()),
                }
            }
            Ok(Download { url: url.to_string(), file_name, checksum, dependencies, manual })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_projects_and_versions() {
        let project = project_from_json(&json!({"id": 23, "name": "Essentials", "namespace": {"owner": "EssentialsX", "slug": "Essentials"}, "stats": {"downloads": 10}}));
        assert_eq!(project.id, "23");
        assert_eq!(project.page_url, "https://hangar.papermc.io/EssentialsX/Essentials");
        let target = Target { kind: AddonKind::Plugin, provider: ProviderKind::Paper, loaders: vec!["paper"], game_version: Some("1.21.4".into()) };
        let version = version_from_json(&target, "PAPER", &json!({"name": "2.21.0", "channel": {"name": "Release"}, "platformDependencies": {"PAPER": ["1.21.4"]}, "downloads": {"PAPER": {}}}));
        assert!(version.compatible && version.stable);
        assert_eq!(platform(ProviderKind::Velocity), Some("VELOCITY"));
        assert_eq!(platform(ProviderKind::Fabric), None);
    }
}
