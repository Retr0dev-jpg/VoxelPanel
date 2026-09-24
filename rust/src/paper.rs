// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use serde::Deserialize;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const PAPER_PROJECT: &str = "https://fill.papermc.io/v3/projects/paper";
const MOJANG_MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone)]
pub struct PaperDownload {
    pub version: String,
    pub build: u32,
    pub file_name: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    versions: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    id: String,
    url: String,
}

pub fn versions_from_project_json(value: &Value) -> Result<Vec<String>, String> {
    let versions = value
        .get("versions")
        .ok_or("Campo versions mancante nella risposta Paper")?;
    let mut found = Vec::new();
    match versions {
        Value::Array(items) => push_strings(&mut found, items),
        Value::Object(map) => {
            for value in map.values() {
                match value {
                    Value::String(text) => found.push(text.clone()),
                    Value::Array(items) => push_strings(&mut found, items),
                    _ => {}
                }
            }
        }
        _ => return Err("Formato versioni Paper non riconosciuto".into()),
    }
    found.retain(|version| !version.contains('-'));
    found.sort_by(|left, right| compare_versions(right, left));
    found.dedup();
    Ok(found)
}

fn push_strings(found: &mut Vec<String>, items: &[Value]) {
    for item in items {
        if let Some(text) = item.as_str() {
            found.push(text.to_string());
        }
    }
}

pub fn keep_known_minecraft(versions: Vec<String>, mojang_ids: &HashSet<String>) -> Vec<String> {
    let filtered: Vec<String> = versions
        .iter()
        .filter(|version| mojang_ids.contains(*version))
        .cloned()
        .collect();
    if filtered.is_empty() { versions } else { filtered }
}

pub fn compare_versions(left: &str, right: &str) -> Ordering {
    version_parts(left)
        .cmp(&version_parts(right))
        .then_with(|| left.cmp(right))
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

pub fn java_major_from_meta(value: &Value) -> u32 {
    value
        .get("javaVersion")
        .and_then(|entry| entry.get("majorVersion"))
        .and_then(|entry| entry.as_u64())
        .unwrap_or(8) as u32
}

pub fn parse_paper_version(file_name: &str) -> Option<String> {
    let lower = file_name.to_lowercase();
    if !lower.starts_with("paper-") {
        return None;
    }
    file_name
        .split('-')
        .nth(1)
        .map(|value| value.trim_end_matches(".jar").to_string())
        .filter(|value| !value.is_empty())
}

pub async fn fetch_versions() -> Result<Vec<String>, String> {
    let client = crate::net::http();
    let project: Value = client
        .get(PAPER_PROJECT)
        .send()
        .await
        .map_err(|error| format!("PaperMC non raggiungibile: {error}"))?
        .error_for_status()
        .map_err(|error| format!("PaperMC ha risposto con errore: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Risposta Paper non valida: {error}"))?;
    let manifest: Manifest = client
        .get(MOJANG_MANIFEST)
        .send()
        .await
        .map_err(|error| format!("Manifest Mojang non raggiungibile: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Manifest Mojang ha risposto con errore: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Manifest Mojang non valido: {error}"))?;
    let ids = manifest.versions.into_iter().map(|entry| entry.id).collect();
    let versions = versions_from_project_json(&project)?;
    Ok(keep_known_minecraft(versions, &ids))
}

pub async fn required_java(version: &str) -> Result<u32, String> {
    let client = crate::net::http();
    let manifest: Manifest = client
        .get(MOJANG_MANIFEST)
        .send()
        .await
        .map_err(|error| format!("Manifest Mojang non raggiungibile: {error}"))?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let entry = manifest
        .versions
        .into_iter()
        .find(|entry| entry.id == version)
        .ok_or_else(|| format!("Versione Minecraft {version} non trovata nel manifest Mojang"))?;
    let meta: Value = client
        .get(entry.url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    Ok(java_major_from_meta(&meta))
}

pub async fn latest_download(version: &str) -> Result<PaperDownload, String> {
    let client = crate::net::http();
    let url = format!("{PAPER_PROJECT}/versions/{version}/builds/latest");
    let value: Value = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Build Paper non raggiungibile: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Build Paper non trovata: {error}"))?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let build = value
        .get("id")
        .and_then(|entry| entry.as_u64())
        .ok_or("Build Paper senza id")? as u32;
    let download = value
        .get("downloads")
        .and_then(|entry| entry.get("server:default"))
        .ok_or("Download server:default mancante")?;
    let file_name = download
        .get("name")
        .and_then(|entry| entry.as_str())
        .ok_or("Nome jar Paper mancante")?
        .to_string();
    let url = download
        .get("url")
        .and_then(|entry| entry.as_str())
        .ok_or("URL jar Paper mancante")?
        .to_string();
    Ok(PaperDownload {
        version: version.to_string(),
        build,
        file_name,
        url,
    })
}

pub async fn download_paper(
    version: &str,
    root: &Path,
    progress: &std::sync::mpsc::Sender<crate::Progress>,
) -> Result<PathBuf, String> {
    let download = latest_download(version).await?;
    let destination = root.join(&download.file_name);
    if destination.exists() {
        let _ = progress.send(crate::Progress {
            stage: "Paper".into(),
            message: format!("Paper {version} già presente."),
            fraction: Some(1.0),
        });
        return Ok(destination);
    }
let _ = progress.send(crate::Progress {
            stage: "Paper".into(),
            message: format!(
                "Download Paper {} build {}...",
                download.version, download.build
            ),
            fraction: Some(0.0),
        });
    crate::net::download(&download.url, &destination, "Paper", progress).await?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flattens_and_sorts_stable_versions() {
        let value = json!({
            "versions": {
                "1.21": ["1.21.1", "1.21.1-rc1", "1.20.6", "1.21"],
                "1.20": ["1.20.4"]
            }
        });
        let versions = versions_from_project_json(&value).unwrap();
        assert_eq!(versions, vec!["1.21.1", "1.21", "1.20.6", "1.20.4"]);
    }

    #[test]
    fn keeps_mojang_ids_when_present() {
        let versions = vec!["1.21.1".into(), "99.0".into()];
        let ids = HashSet::from(["1.21.1".to_string()]);
        assert_eq!(keep_known_minecraft(versions, &ids), vec!["1.21.1"]);
    }

    #[test]
    fn reads_java_major() {
        let value = json!({"javaVersion": {"majorVersion": 21}});
        assert_eq!(java_major_from_meta(&value), 21);
        assert_eq!(java_major_from_meta(&json!({})), 8);
    }

    #[test]
    fn reads_paper_jar_version() {
        assert_eq!(
            parse_paper_version("paper-1.21.11-132.jar").as_deref(),
            Some("1.21.11")
        );
        assert!(parse_paper_version("server.jar").is_none());
    }
}
