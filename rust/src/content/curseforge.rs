// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use futures::future::BoxFuture;
use serde_json::{json, Value};

use super::{ContentSource, Dependency, Download, Target, PAGE_SIZE};
use crate::api::types::{AddonKind, ContentPage, ContentProject, ContentSourceKind, ContentVersion, ProviderKind};
use crate::net::Checksum;
use crate::{PanelError, PanelResult};

const API: &str = "https://api.curseforge.com/v1";
const MINECRAFT: u32 = 432;
pub const CLASS_MODS: u32 = 6;
pub const CLASS_PLUGINS: u32 = 5;
pub const CLASS_MODPACKS: u32 = 4471;

/// CurseForge requires a personal API key, configured in the launcher settings.
pub struct CurseForge;
pub static CURSEFORGE: CurseForge = CurseForge;

pub fn api_key() -> Option<String> {
    let key = crate::launcher_settings::current().network.curseforge_api_key;
    (!key.trim().is_empty()).then(|| key.trim().to_string())
}

pub fn loader_type(provider: ProviderKind, loaders: &[&str]) -> Option<u32> {
    match provider {
        ProviderKind::Forge | ProviderKind::SpongeForge | ProviderKind::Mohist => Some(1),
        ProviderKind::Fabric => Some(4),
        ProviderKind::Quilt => Some(5),
        ProviderKind::NeoForge => Some(6),
        _ if loaders.contains(&"neoforge") => Some(6),
        _ if loaders.contains(&"fabric") => Some(4),
        _ if loaders.contains(&"forge") => Some(1),
        _ => None,
    }
}

pub async fn get(path: &str, query: &[(&str, String)]) -> PanelResult<Value> {
    let key = api_key().ok_or_else(|| PanelError::invalid("Aggiungi la chiave API di CurseForge nelle impostazioni del launcher"))?;
    let response = crate::net::http()
        .get(format!("{API}{path}"))
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .query(query)
        .send()
        .await
        .map_err(|error| PanelError::network(format!("CurseForge non raggiungibile: {error}")))?;
    if response.status().as_u16() == 403 {
        return Err(PanelError::invalid("Chiave API di CurseForge non valida"));
    }
    Ok(response.error_for_status()?.json().await?)
}

pub async fn post(path: &str, body: Value) -> PanelResult<Value> {
    let key = api_key().ok_or_else(|| PanelError::invalid("Aggiungi la chiave API di CurseForge nelle impostazioni del launcher"))?;
    Ok(crate::net::http()
        .post(format!("{API}{path}"))
        .header("x-api-key", key)
        .json(&body)
        .send()
        .await
        .map_err(|error| PanelError::network(format!("CurseForge non raggiungibile: {error}")))?
        .error_for_status()?
        .json()
        .await?)
}

pub fn project_from_json(value: &Value) -> ContentProject {
    ContentProject {
        source: ContentSourceKind::CurseForge,
        id: value.get("id").and_then(Value::as_i64).map(|id| id.to_string()).unwrap_or_default(),
        slug: value.get("slug").and_then(Value::as_str).unwrap_or_default().to_string(),
        title: value.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        description: value.get("summary").and_then(Value::as_str).unwrap_or_default().to_string(),
        author: value.get("authors").and_then(Value::as_array).and_then(|authors| authors.first()).and_then(|author| author.get("name")).and_then(Value::as_str).unwrap_or_default().to_string(),
        downloads: value.get("downloadCount").and_then(Value::as_f64).unwrap_or(0.0) as i64,
        icon_url: value.get("logo").and_then(|logo| logo.get("thumbnailUrl")).and_then(Value::as_str).unwrap_or_default().to_string(),
        page_url: value.get("links").and_then(|links| links.get("websiteUrl")).and_then(Value::as_str).unwrap_or_default().to_string(),
    }
}

pub async fn search_class(class_id: u32, target: Option<&Target>, query: &str, page: u32) -> PanelResult<ContentPage> {
    let mut params = vec![
        ("gameId", MINECRAFT.to_string()),
        ("classId", class_id.to_string()),
        ("searchFilter", query.to_string()),
        ("sortField", "2".into()),
        ("sortOrder", "desc".into()),
        ("pageSize", PAGE_SIZE.to_string()),
        ("index", (page * PAGE_SIZE).to_string()),
    ];
    if let Some(target) = target {
        if let Some(version) = &target.game_version {
            params.push(("gameVersion", version.clone()));
        }
        if let Some(loader) = loader_type(target.provider, &target.loaders).filter(|_| target.kind == AddonKind::Mod) {
            params.push(("modLoaderType", loader.to_string()));
        }
    }
    let value = get("/mods/search", &params).await?;
    Ok(ContentPage {
        projects: value.get("data").and_then(Value::as_array).map(|items| items.iter().map(project_from_json).collect()).unwrap_or_default(),
        total: value.get("pagination").and_then(|pagination| pagination.get("totalCount")).and_then(Value::as_u64).unwrap_or(0).min(10_000) as u32,
    })
}

pub fn version_from_file(target: Option<&Target>, file: &Value) -> ContentVersion {
    let tags: Vec<String> = file.get("gameVersions").and_then(Value::as_array).map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    let (loaders, game_versions): (Vec<String>, Vec<String>) = tags.into_iter().partition(|tag| !tag.chars().next().is_some_and(|ch| ch.is_ascii_digit()));
    let loaders: Vec<String> = loaders.into_iter().map(|loader| loader.to_lowercase()).collect();
    ContentVersion {
        id: file.get("id").and_then(Value::as_i64).map(|id| id.to_string()).unwrap_or_default(),
        name: file.get("displayName").and_then(Value::as_str).unwrap_or_default().to_string(),
        compatible: target.is_none_or(|target| target.accepts_version(&game_versions) && (target.kind == AddonKind::Plugin || target.accepts_loader(&loaders))),
        stable: file.get("releaseType").and_then(Value::as_i64) == Some(1),
        published: file.get("fileDate").and_then(Value::as_str).unwrap_or_default().to_string(),
        loaders,
        game_versions,
    }
}

pub fn download_from_file(file: &Value) -> PanelResult<Download> {
    let url = file
        .get("downloadUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| PanelError::invalid("L'autore non permette il download tramite app: scaricalo dal sito di CurseForge"))?;
    let checksum = file
        .get("hashes")
        .and_then(Value::as_array)
        .and_then(|hashes| hashes.iter().find(|hash| hash.get("algo").and_then(Value::as_i64) == Some(1)))
        .and_then(|hash| hash.get("value"))
        .and_then(Value::as_str)
        .map(|hash| Checksum::Sha1(hash.to_string()));
    let dependencies = file
        .get("dependencies")
        .and_then(Value::as_array)
        .map(|dependencies| {
            dependencies
                .iter()
                .filter(|dependency| dependency.get("relationType").and_then(Value::as_i64) == Some(3))
                .filter_map(|dependency| dependency.get("modId").and_then(Value::as_i64))
                .map(|id| Dependency { source: ContentSourceKind::CurseForge, project_id: id.to_string(), version_id: None })
                .collect()
        })
        .unwrap_or_default();
    Ok(Download {
        url: url.to_string(),
        file_name: file.get("fileName").and_then(Value::as_str).unwrap_or("mod.jar").to_string(),
        checksum,
        dependencies,
        manual: Vec::new(),
    })
}

/// Files by id in one request, used by CurseForge modpacks.
pub async fn files(ids: &[i64]) -> PanelResult<Vec<Value>> {
    let value = post("/mods/files", json!({"fileIds": ids})).await?;
    Ok(value.get("data").and_then(Value::as_array).cloned().unwrap_or_default())
}

impl ContentSource for CurseForge {
    fn kind(&self) -> ContentSourceKind {
        ContentSourceKind::CurseForge
    }

    fn supports(&self, target: &Target) -> bool {
        api_key().is_some() && (target.kind == AddonKind::Mod || super::is_bukkit_family(target.provider))
    }

    fn search<'a>(&'a self, target: &'a Target, query: &'a str, page: u32) -> BoxFuture<'a, PanelResult<ContentPage>> {
        Box::pin(async move {
            let class = if target.kind == AddonKind::Mod { CLASS_MODS } else { CLASS_PLUGINS };
            search_class(class, Some(target), query, page).await
        })
    }

    fn versions<'a>(&'a self, target: &'a Target, project_id: &'a str) -> BoxFuture<'a, PanelResult<Vec<ContentVersion>>> {
        Box::pin(async move {
            let value = get(&format!("/mods/{project_id}/files"), &[("pageSize", "50".into())]).await?;
            let mut versions: Vec<ContentVersion> = value.get("data").and_then(Value::as_array).map(|files| files.iter().map(|file| version_from_file(Some(target), file)).collect()).unwrap_or_default();
            super::sort_versions(&mut versions);
            Ok(versions)
        })
    }

    fn download<'a>(&'a self, target: &'a Target, project_id: &'a str, version_id: &'a str) -> BoxFuture<'a, PanelResult<Download>> {
        Box::pin(async move {
            let version_id = super::pick_version(&self.versions(target, project_id).await?, version_id)?;
            let value = get(&format!("/mods/{project_id}/files/{version_id}"), &[]).await?;
            download_from_file(value.get("data").ok_or("Risposta CurseForge senza dati")?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_loaders_and_versions() {
        let target = Target { kind: AddonKind::Mod, provider: ProviderKind::NeoForge, loaders: vec!["neoforge"], game_version: Some("1.21.1".into()) };
        let file = json!({"id": 5, "displayName": "Mod 1.0", "gameVersions": ["1.21.1", "NeoForge"], "releaseType": 1, "downloadUrl": "https://edge/x.jar", "fileName": "x.jar",
            "hashes": [{"value": "abc", "algo": 1}], "dependencies": [{"modId": 9, "relationType": 3}, {"modId": 10, "relationType": 2}]});
        let version = version_from_file(Some(&target), &file);
        assert!(version.compatible && version.stable);
        assert_eq!(version.loaders, vec!["neoforge"]);
        let download = download_from_file(&file).unwrap();
        assert_eq!(download.dependencies.len(), 1);
        assert!(download_from_file(&json!({"id": 6})).is_err());
        assert_eq!(loader_type(ProviderKind::Quilt, &[]), Some(5));
    }
}
