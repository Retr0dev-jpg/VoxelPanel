// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::Path;
use std::time::Duration;

use futures::future::BoxFuture;
use tokio::io::{AsyncBufReadExt, BufReader};

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{ProviderKind, VersionEntry};
use crate::{LaunchSpec, PanelError, PanelResult};

const VERSIONS: &str = "https://hub.spigotmc.org/versions/";
const BUILD_TOOLS: &str = "https://hub.spigotmc.org/jenkins/job/BuildTools/lastSuccessfulBuild/artifact/target/BuildTools.jar";

/// Spigot cannot be redistributed: it is compiled locally with BuildTools.
pub struct Spigot;
pub static SPIGOT: Spigot = Spigot;

/// Reads `1.21.4.json`-style entries from the directory listing of the versions folder.
pub fn versions_from_index(html: &str) -> Vec<VersionEntry> {
    let mut entries: Vec<VersionEntry> = html
        .split("href=\"")
        .skip(1)
        .filter_map(|chunk| chunk.split('"').next())
        .filter_map(|href| href.strip_suffix(".json"))
        .filter(|name| name.contains('.') && name.chars().all(|ch| ch.is_ascii_digit() || ch == '.'))
        .map(|name| VersionEntry { id: name.to_string(), stable: true })
        .collect();
    entries.dedup_by(|left, right| left.id == right.id);
    super::sort_versions_desc(&mut entries);
    entries
}

async fn fetch_index() -> PanelResult<String> {
    let cache = crate::paths::Layout::app().cache().join("api").join("spigot-versions.html");
    let fresh = std::fs::metadata(&cache)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age < crate::cache::LONG);
    if fresh {
        if let Ok(text) = std::fs::read_to_string(&cache) {
            return Ok(text);
        }
    }
    match crate::net::http().get(VERSIONS).send().await.and_then(|response| response.error_for_status()) {
        Ok(response) => {
            let text = response.text().await?;
            if let Some(parent) = cache.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&cache, &text);
            Ok(text)
        }
        Err(error) => std::fs::read_to_string(&cache).map_err(|_| PanelError::network(format!("SpigotMC non raggiungibile: {error}"))),
    }
}

fn git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

async fn ensure_build_tools(directory: &Path, progress: &crate::ProgressTx) -> PanelResult<std::path::PathBuf> {
    let jar = directory.join("BuildTools.jar");
    let stale = std::fs::metadata(&jar)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_none_or(|age| age > Duration::from_secs(24 * 60 * 60));
    if stale {
        progress.emit("Spigot", "Download BuildTools...", Some(0.0));
        crate::net::download(BUILD_TOOLS, &jar, "Spigot", progress).await?;
    }
    Ok(jar)
}

impl Provider for Spigot {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Spigot
    }

    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move { Ok(versions_from_index(&fetch_index().await?)) })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            // On Windows BuildTools downloads a portable Git by itself.
            if !cfg!(windows) && !git_available() {
                return Err(PanelError::not_found("BuildTools richiede Git: installalo e riprova."));
            }
            let work = crate::paths::Layout::app().cache().join("buildtools");
            crate::paths::ensure_dir(&work)?;
            let build_tools = ensure_build_tools(&work, request.progress).await?;
            request.progress.emit("Spigot", format!("Compilazione di Spigot {} (può richiedere diversi minuti)...", request.mc_version), None);
            let mut command = tokio::process::Command::new(request.java);
            command
                .arg("-jar")
                .arg(&build_tools)
                .args(["--rev", request.mc_version, "--compile", "spigot", "--nogui", "--output-dir"])
                .arg(request.root)
                .current_dir(&work)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true);
            let mut child = command.spawn().map_err(|error| PanelError::io(format!("BuildTools non avviabile: {error}")))?;
            if let Some(stdout) = child.stdout.take() {
                let progress = request.progress.clone();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if !line.trim().is_empty() {
                            progress.emit("Spigot", line, None);
                        }
                    }
                });
            }
            let status = child.wait().await?;
            if !status.success() {
                return Err(PanelError::from(format!(
                    "BuildTools è terminato con errore (codice {}). Controlla i messaggi sopra.",
                    status.code().unwrap_or(-1)
                )));
            }
            let jar = super::jar_destination(request.root, &format!("spigot-{}.jar", request.mc_version))?;
            if !jar.is_file() {
                return Err(PanelError::not_found(format!("BuildTools non ha prodotto {}", jar.display())));
            }
            Ok(Installed {
                launch: LaunchSpec::Jar { path: jar },
                mc_version: request.mc_version.to_string(),
                build: None,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_version_index() {
        let html = r#"<a href="../">../</a><a href="1.10.2.json">1.10.2.json</a><a href="latest.json">latest.json</a><a href="1.21.4.json">1.21.4.json</a><a href="3000.json">x</a>"#;
        let ids: Vec<String> = versions_from_index(html).into_iter().map(|entry| entry.id).collect();
        assert_eq!(ids, vec!["1.21.4", "1.10.2"]);
    }
}
