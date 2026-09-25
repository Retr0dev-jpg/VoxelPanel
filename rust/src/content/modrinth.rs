// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::collections::HashMap;

use futures::future::BoxFuture;
use serde_json::{json, Value};

use super::{ContentSource, Dependency, Download, Target, PAGE_SIZE};
use crate::api::types::{AddonKind, ContentPage, ContentProject, ContentSourceKind, ContentVersion};
use crate::net::Checksum;
use crate::{PanelError, PanelResult};

const API: &str = "https://api.modrinth.com/v2";

pub struct Modrinth;
pub static MODRINTH: Modrinth = Modrinth;

fn str_of(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value.get(key).and_then(Value::as_array).map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

pub fn project_from_hit(hit: &Value) -> ContentProject {
    let slug = str_of(hit, "slug");
    let kind = str_of(hit, "project_type");
    ContentProject {
        source: ContentSourceKind::Modrinth,
        id: str_of(hit, "project_id"),
        title: str_of(hit, "title"),
        description: str_of(hit, "description"),
        author: str_of(hit, "author"),
        downloads: hit.get("downloads").and_then(Value::as_i64).unwrap_or(0),
        icon_url: str_of(hit, "icon_url"),
        page_url: format!("https://modrinth.com/{}/{slug}", if kind.is_empty() { "project".into() } else { kind }),
        slug,
    }
}

pub async fn search_type(project_type: &str, facets_extra: Vec<String>, query: &str, page: u32) -> PanelResult<ContentPage> {
    let mut facets = vec![format!("[\"project_type:{project_type}\"]")];
    facets.extend(facets_extra);
    let facets = format!("[{}]", facets.join(","));
    let offset = (page * PAGE_SIZE).to_string();
    let value: Value = crate::net::http()
        .get(format!("{API}/search"))
        .query(&[("query", query), ("limit", &PAGE_SIZE.to_string()), ("offset", &offset), ("index", "relevance"), ("facets", &facets)])
        .send()
        .await
        .map_err(|error| PanelError::network(format!("Modrinth non raggiungibile: {error}")))?
        .error_for_status()?
        .json()
        .await?;
    Ok(ContentPage {
        projects: value.get("hits").and_then(Value::as_array).map(|hits| hits.iter().map(project_from_hit).collect()).unwrap_or_default(),
        total: value.get("total_hits").and_then(Value::as_u64).unwrap_or(0) as u32,
    })
}

pub fn version_from_json(target: Option<&Target>, value: &Value) -> ContentVersion {
    let game_versions = strings(value, "game_versions");
    let loaders = strings(value, "loaders");
    ContentVersion {
        id: str_of(value, "id"),
        name: {
            let number = str_of(value, "version_number");
            let name = str_of(value, "name");
            if name.is_empty() || name == number { number } else { format!("{number} · {name}") }
        },
        compatible: target.is_none_or(|target| target.accepts_version(&game_versions) && target.accepts_loader(&loaders)),
        stable: str_of(value, "version_type") == "release",
        published: str_of(value, "date_published"),
        game_versions,
        loaders,
    }
}

async fn get_json(url: &str) -> PanelResult<Value> {
    Ok(crate::net::http()
        .get(url)
        .send()
        .await
        .map_err(|error| PanelError::network(format!("Modrinth non raggiungibile: {error}")))?
        .error_for_status()?
        .json()
        .await?)
}

pub async fn version(version_id: &str) -> PanelResult<Value> {
    get_json(&format!("{API}/version/{version_id}")).await
}

pub async fn project_versions(project_id: &str) -> PanelResult<Vec<Value>> {
    Ok(get_json(&format!("{API}/project/{project_id}/version")).await?.as_array().cloned().unwrap_or_default())
}

/// The primary file of a version, with its SHA-512 (Modrinth publishes both SHA-1 and SHA-512).
pub fn primary_file(version: &Value) -> Option<(String, String, Option<Checksum>)> {
    let files = version.get("files")?.as_array()?;
    let file = files.iter().find(|file| file.get("primary").and_then(Value::as_bool) == Some(true)).or_else(|| files.first())?;
    let hashes = file.get("hashes");
    let checksum = hashes
        .and_then(|hashes| hashes.get("sha512"))
        .and_then(Value::as_str)
        .map(|hash| Checksum::Sha512(hash.to_string()))
        .or_else(|| hashes.and_then(|hashes| hashes.get("sha1")).and_then(Value::as_str).map(|hash| Checksum::Sha1(hash.to_string())));
    Some((str_of(file, "url"), str_of(file, "filename"), checksum))
}

pub fn required_dependencies(version: &Value) -> Vec<Dependency> {
    version
        .get("dependencies")
        .and_then(Value::as_array)
        .map(|dependencies| {
            dependencies
                .iter()
                .filter(|dependency| dependency.get("dependency_type").and_then(Value::as_str) == Some("required"))
                .filter_map(|dependency| {
                    let project_id = dependency.get("project_id").and_then(Value::as_str).map(str::to_string);
                    let version_id = dependency.get("version_id").and_then(Value::as_str).map(str::to_string);
                    Some(Dependency { source: ContentSourceKind::Modrinth, project_id: project_id.or_else(|| version_id.clone())?, version_id })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Current version (by SHA-1) of each installed file that Modrinth knows.
pub async fn identify(hashes: &[String]) -> PanelResult<HashMap<String, Value>> {
    if hashes.is_empty() {
        return Ok(HashMap::new());
    }
    let value: Value = crate::net::http()
        .post(format!("{API}/version_files"))
        .json(&json!({"hashes": hashes, "algorithm": "sha1"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(value.as_object().map(|map| map.iter().map(|(hash, version)| (hash.clone(), version.clone())).collect()).unwrap_or_default())
}

/// Newest compatible version for each installed file, keyed by SHA-1.
pub async fn latest_for(hashes: &[String], target: &Target) -> PanelResult<HashMap<String, Value>> {
    if hashes.is_empty() {
        return Ok(HashMap::new());
    }
    let mut body = json!({"hashes": hashes, "algorithm": "sha1", "loaders": target.loaders});
    if let Some(version) = &target.game_version {
        body["game_versions"] = json!([version]);
    }
    let value: Value = crate::net::http().post(format!("{API}/version_files/update")).json(&body).send().await?.error_for_status()?.json().await?;
    Ok(value.as_object().map(|map| map.iter().map(|(hash, version)| (hash.clone(), version.clone())).collect()).unwrap_or_default())
}

impl ContentSource for Modrinth {
    fn kind(&self) -> ContentSourceKind {
        ContentSourceKind::Modrinth
    }

    fn supports(&self, _target: &Target) -> bool {
        true
    }

    fn search<'a>(&'a self, target: &'a Target, query: &'a str, page: u32) -> BoxFuture<'a, PanelResult<ContentPage>> {
        Box::pin(async move {
            let project_type = match target.kind {
                AddonKind::Plugin => "plugin",
                AddonKind::Mod => "mod",
            };
            let loaders: Vec<String> = target.loaders.iter().map(|loader| format!("\"categories:{loader}\"")).collect();
            let mut facets = vec![format!("[{}]", loaders.join(","))];
            if let Some(version) = &target.game_version {
                facets.push(format!("[\"versions:{version}\"]"));
            }
            search_type(project_type, facets, query, page).await
        })
    }

    fn versions<'a>(&'a self, target: &'a Target, project_id: &'a str) -> BoxFuture<'a, PanelResult<Vec<ContentVersion>>> {
        Box::pin(async move {
            let mut versions: Vec<ContentVersion> = project_versions(project_id).await?.iter().map(|value| version_from_json(Some(target), value)).collect();
            super::sort_versions(&mut versions);
            Ok(versions)
        })
    }

    fn download<'a>(&'a self, target: &'a Target, project_id: &'a str, version_id: &'a str) -> BoxFuture<'a, PanelResult<Download>> {
        Box::pin(async move {
            let version_id = if version_id.is_empty() { super::pick_version(&self.versions(target, project_id).await?, "")? } else { version_id.to_string() };
            let value = version(&version_id).await?;
            let (url, file_name, checksum) = primary_file(&value).ok_or_else(|| PanelError::not_found("La versione non ha file scaricabili"))?;
            Ok(Download { url, file_name, checksum, dependencies: required_dependencies(&value), manual: Vec::new() })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::ProviderKind;

    #[test]
    fn reads_versions_files_and_dependencies() {
        let target = Target { kind: AddonKind::Mod, provider: ProviderKind::Fabric, loaders: vec!["fabric"], game_version: Some("1.21.1".into()) };
        let value = json!({
            "id": "v1", "version_number": "1.2.0", "name": "1.2.0", "version_type": "release", "date_published": "2026-01-01",
            "game_versions": ["1.21.1"], "loaders": ["fabric"],
            "files": [{"url": "https://cdn/x.jar", "filename": "x.jar", "primary": true, "hashes": {"sha1": "aa", "sha512": "bb"}}],
            "dependencies": [
                {"project_id": "P7dR8mSH", "version_id": null, "dependency_type": "required"},
                {"project_id": "opt", "dependency_type": "optional"}
            ]
        });
        let parsed = version_from_json(Some(&target), &value);
        assert!(parsed.compatible && parsed.stable);
        let (url, file, checksum) = primary_file(&value).unwrap();
        assert_eq!((url.as_str(), file.as_str()), ("https://cdn/x.jar", "x.jar"));
        assert!(matches!(checksum, Some(Checksum::Sha512(hash)) if hash == "bb"));
        let dependencies = required_dependencies(&value);
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].project_id, "P7dR8mSH");
    }
}
