// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use futures::future::BoxFuture;
use serde_json::Value;

use super::{ContentSource, Download, Target, PAGE_SIZE};
use crate::api::types::{AddonKind, ContentPage, ContentProject, ContentSourceKind, ContentVersion, ProviderKind};
use crate::{PanelError, PanelResult};

const API: &str = "https://api.spiget.org/v2";
const FIELDS: &str = "id,name,tag,downloads,external,premium,icon,author,testedVersions";

/// SpigotMC resources through Spiget. Only free resources hosted on SpigotMC can be downloaded.
pub struct Spiget;
pub static SPIGET: Spiget = Spiget;

pub fn project_from_json(value: &Value) -> Option<ContentProject> {
    if value.get("premium").and_then(Value::as_bool) == Some(true) || value.get("external").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let id = value.get("id")?.as_i64()?;
    let icon = value.get("icon").and_then(|icon| icon.get("url")).and_then(Value::as_str).unwrap_or_default();
    Some(ContentProject {
        source: ContentSourceKind::Spiget,
        id: id.to_string(),
        slug: id.to_string(),
        title: value.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        description: value.get("tag").and_then(Value::as_str).unwrap_or_default().to_string(),
        author: value.get("author").and_then(|author| author.get("id")).and_then(Value::as_i64).map(|author| format!("#{author}")).unwrap_or_default(),
        downloads: value.get("downloads").and_then(Value::as_i64).unwrap_or(0),
        icon_url: if icon.is_empty() { String::new() } else { format!("https://www.spigotmc.org/{icon}") },
        page_url: format!("https://www.spigotmc.org/resources/{id}/"),
    })
}

async fn get_json(url: &str) -> PanelResult<Value> {
    Ok(crate::net::http()
        .get(url)
        .send()
        .await
        .map_err(|error| PanelError::network(format!("Spiget non raggiungibile: {error}")))?
        .error_for_status()?
        .json()
        .await?)
}

impl ContentSource for Spiget {
    fn kind(&self) -> ContentSourceKind {
        ContentSourceKind::Spiget
    }

    fn supports(&self, target: &Target) -> bool {
        target.kind == AddonKind::Plugin && (super::is_bukkit_family(target.provider) || matches!(target.provider, ProviderKind::BungeeCord | ProviderKind::Waterfall))
    }

    fn search<'a>(&'a self, _target: &'a Target, query: &'a str, page: u32) -> BoxFuture<'a, PanelResult<ContentPage>> {
        Box::pin(async move {
            let page_param = page + 1;
            let url = if query.trim().is_empty() {
                format!("{API}/resources?size={PAGE_SIZE}&page={page_param}&sort=-downloads&fields={FIELDS}")
            } else {
                let encoded: String = query.trim().bytes().map(|byte| if byte.is_ascii_alphanumeric() { (byte as char).to_string() } else { format!("%{byte:02X}") }).collect();
                format!("{API}/search/resources/{encoded}?size={PAGE_SIZE}&page={page_param}&sort=-downloads&fields={FIELDS}")
            };
            let value = get_json(&url).await?;
            let projects: Vec<ContentProject> = value.as_array().map(|items| items.iter().filter_map(project_from_json).collect()).unwrap_or_default();
            // Spiget does not return totals: assume more pages while pages are full.
            let total = page * PAGE_SIZE + projects.len() as u32 + if projects.len() as u32 >= PAGE_SIZE / 2 { PAGE_SIZE } else { 0 };
            Ok(ContentPage { projects, total })
        })
    }

    fn versions<'a>(&'a self, target: &'a Target, project_id: &'a str) -> BoxFuture<'a, PanelResult<Vec<ContentVersion>>> {
        Box::pin(async move {
            let resource = get_json(&format!("{API}/resources/{project_id}?fields=testedVersions,version")).await?;
            let tested: Vec<String> = resource.get("testedVersions").and_then(Value::as_array).map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
            let latest = get_json(&format!("{API}/resources/{project_id}/versions/latest")).await?;
            // Only the latest version can be downloaded through Spiget.
            Ok(vec![ContentVersion {
                id: "latest".into(),
                name: latest.get("name").and_then(Value::as_str).unwrap_or("latest").to_string(),
                compatible: tested.is_empty() || target.accepts_version(&tested) || target.game_version.is_none(),
                stable: true,
                published: latest.get("releaseDate").and_then(Value::as_i64).map(|date| date.to_string()).unwrap_or_default(),
                loaders: vec!["spigot".into()],
                game_versions: tested,
            }])
        })
    }

    fn download<'a>(&'a self, _target: &'a Target, project_id: &'a str, _version_id: &'a str) -> BoxFuture<'a, PanelResult<Download>> {
        Box::pin(async move {
            let resource = get_json(&format!("{API}/resources/{project_id}?fields=name,external,premium,file")).await?;
            if resource.get("premium").and_then(Value::as_bool) == Some(true) || resource.get("external").and_then(Value::as_bool) == Some(true) {
                return Err(PanelError::invalid("Risorsa a pagamento o esterna: scaricala dalla pagina di SpigotMC"));
            }
            if resource.get("file").and_then(|file| file.get("type")).and_then(Value::as_str) != Some(".jar") {
                return Err(PanelError::invalid("La risorsa non è un singolo jar: scaricala dalla pagina di SpigotMC"));
            }
            let name: String = resource.get("name").and_then(Value::as_str).unwrap_or("plugin").chars().filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')).collect();
            Ok(Download {
                url: format!("{API}/resources/{project_id}/download"),
                file_name: format!("{}-{project_id}.jar", if name.is_empty() { "plugin".into() } else { name }),
                checksum: None,
                dependencies: Vec::new(),
                manual: Vec::new(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn skips_premium_and_external_resources() {
        assert!(project_from_json(&json!({"id": 1, "name": "x", "premium": true})).is_none());
        assert!(project_from_json(&json!({"id": 2, "name": "x", "external": true})).is_none());
        let project = project_from_json(&json!({"id": 28140, "name": "LuckPerms", "icon": {"url": "data/resource_icons/28/28140.jpg"}})).unwrap();
        assert_eq!(project.icon_url, "https://www.spigotmc.org/data/resource_icons/28/28140.jpg");
    }
}
