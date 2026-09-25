// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PluginEntry {
    pub file_name: String,
    pub enabled: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ModrinthHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: i64,
}

pub fn list(root: &Path) -> Vec<PluginEntry> {
    let plugins = root.join("plugins");
    let mut items = Vec::new();
    let Ok(entries) = std::fs::read_dir(plugins) else {
        return items;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        let enabled = lower.ends_with(".jar");
        let disabled = lower.ends_with(".jar.disabled");
        if !enabled && !disabled {
            continue;
        }
        let size_bytes = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        items.push(PluginEntry {
            file_name: name,
            enabled,
            size_bytes,
        });
    }
    items.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    items
}

pub fn set_enabled(root: &Path, file_name: &str, enabled: bool) -> crate::PanelResult<()> {
    let current = plugin_path(root, file_name)?;
    let lower = file_name.to_lowercase();
    let target_name = if enabled {
        file_name.trim_end_matches(".disabled").to_string()
    } else if lower.ends_with(".jar.disabled") {
        file_name.to_string()
    } else {
        format!("{file_name}.disabled")
    };
    if target_name == file_name {
        return Ok(());
    }
    let target = root.join("plugins").join(&target_name);
    if target.exists() {
        return Err("Esiste già un file con questo nome".into());
    }
    std::fs::rename(current, target).map_err(|error| crate::PanelError::from(error.to_string()))
}

pub fn delete(root: &Path, file_name: &str) -> crate::PanelResult<()> {
    let path = plugin_path(root, file_name)?;
    std::fs::remove_file(path).map_err(|error| crate::PanelError::from(error.to_string()))
}

pub fn install_file(root: &Path, source: &Path) -> crate::PanelResult<()> {
    if !source.is_file() {
        return Err("Jar plugin non trovato".into());
    }
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Nome plugin non valido")?;
    if !name.to_lowercase().ends_with(".jar") || name.contains(['\\', '/', ':']) {
        return Err("Il file deve essere un .jar".into());
    }
    let plugins = root.join("plugins");
    crate::paths::ensure_dir(&plugins)?;
    let destination = plugins.join(name);
    if source != destination {
        std::fs::copy(source, &destination).map_err(|error| crate::PanelError::from(error.to_string()))?;
    }
    Ok(())
}

fn plugin_path(root: &Path, file_name: &str) -> crate::PanelResult<PathBuf> {
    if file_name.contains(['\\', '/', ':']) || file_name.contains("..") {
        return Err("Nome plugin non valido".into());
    }
    let path = root.join("plugins").join(file_name);
    if !path.is_file() {
        return Err("Plugin non trovato".into());
    }
    Ok(path)
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    downloads: i64,
}

#[derive(Debug, Deserialize)]
struct ModrinthVersion {
    game_versions: Vec<String>,
    loaders: Vec<String>,
    date_published: String,
    files: Vec<ModrinthFile>,
}

#[derive(Debug, Deserialize)]
struct ModrinthFile {
    url: String,
    filename: String,
    primary: bool,
}

pub async fn search(query: &str) -> crate::PanelResult<Vec<ModrinthHit>> {
    let response: SearchResponse = crate::net::http()
        .get("https://api.modrinth.com/v2/search")
        .query(&[
            ("query", query),
            ("limit", "20"),
            (
                "facets",
                r#"[["project_type:plugin"],["categories:paper"]]"#,
            ),
        ])
        .send()
        .await
        .map_err(|error| format!("Modrinth non raggiungibile: {error}"))?
        .error_for_status()
        .map_err(|error| crate::PanelError::from(error.to_string()))?
        .json()
        .await
        .map_err(|error| crate::PanelError::from(error.to_string()))?;
    Ok(response
        .hits
        .into_iter()
        .map(|hit| ModrinthHit {
            project_id: hit.project_id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            downloads: hit.downloads,
        })
        .collect())
}

pub async fn install_project(
    root: &Path,
    project_id: &str,
    minecraft_version: &str,
    progress: &crate::ProgressTx,
) -> crate::PanelResult<()> {
    if project_id.is_empty() || project_id.contains(['/', '\\', ' ']) {
        return Err("Progetto Modrinth non valido".into());
    }
    let versions: Vec<ModrinthVersion> = crate::net::http()
        .get(format!("https://api.modrinth.com/v2/project/{project_id}/version"))
        .send()
        .await
        .map_err(|error| format!("Versioni Modrinth non raggiungibili: {error}"))?
        .error_for_status()
        .map_err(|error| crate::PanelError::from(error.to_string()))?
        .json()
        .await
        .map_err(|error| crate::PanelError::from(error.to_string()))?;
    let mut compatible: Vec<ModrinthVersion> = versions
        .into_iter()
        .filter(|version| version_compatible(minecraft_version, &version.game_versions))
        .filter(|version| {
            version.loaders.iter().any(|loader| {
                matches!(
                    loader.as_str(),
                    "paper" | "purpur" | "spigot" | "bukkit" | "folia"
                )
            })
        })
        .collect();
    compatible.sort_by(|left, right| right.date_published.cmp(&left.date_published));
    let version = compatible
        .into_iter()
        .next()
        .ok_or("Nessuna versione del plugin è compatibile con questo server Paper")?;
    let file = version
        .files
        .iter()
        .find(|file| file.primary && file.filename.to_lowercase().ends_with(".jar"))
        .or_else(|| {
            version
                .files
                .iter()
                .find(|file| file.filename.to_lowercase().ends_with(".jar"))
        })
        .ok_or("Il plugin non ha un file jar")?;
    let plugins = root.join("plugins");
    crate::paths::ensure_dir(&plugins)?;
    let destination = plugins.join(&file.filename);
    progress.send(crate::Progress {
        stage: "Plugin".into(),
        message: format!("Download {}...", file.filename),
        fraction: Some(0.0),
    });
    crate::net::download(&file.url, &destination, "Plugin", progress).await
}

fn version_compatible(server: &str, supported: &[String]) -> bool {
    if supported.iter().any(|version| version == server) {
        return true;
    }
    match server.rsplit_once('.') {
        Some((prefix, _)) => supported.iter().any(|version| version == prefix),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_enabled_and_disabled_jars() {
        let root = std::env::temp_dir().join(format!("voxel-plugins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("plugins")).unwrap();
        std::fs::write(root.join("plugins").join("alpha.jar"), b"a").unwrap();
        std::fs::write(root.join("plugins").join("beta.jar.disabled"), b"b").unwrap();
        std::fs::write(root.join("plugins").join("note.txt"), b"c").unwrap();
        let plugins = list(&root);
        assert_eq!(plugins.len(), 2);
        assert!(plugins.iter().any(|plugin| plugin.enabled && plugin.file_name == "alpha.jar"));
        assert!(plugins.iter().any(|plugin| !plugin.enabled));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn matches_minor_game_version() {
        let supported = vec!["1.21".into()];
        assert!(version_compatible("1.21.11", &supported));
        assert!(!version_compatible("1.20.4", &supported));
    }
}
