// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Forge and NeoForge: the official installer is run with `--installServer`.

use std::path::{Path, PathBuf};

use futures::future::BoxFuture;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};

use super::{InstallRequest, Installed, Provider};
use crate::api::types::{BuildEntry, ProviderKind, VersionEntry};
use crate::{cache, LaunchSpec, PanelError, PanelResult, ProgressTx};

const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const FORGE_PROMOTIONS: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

pub struct Forge;
pub static FORGE: Forge = Forge;
pub struct NeoForge;
pub static NEOFORGE: NeoForge = NeoForge;

/// Runs an installer jar and streams its output; the process is killed if VoxelPanel gives up.
pub async fn run_installer(java: &Path, jar: &Path, args: &[&str], cwd: &Path, stage: &str, progress: &ProgressTx) -> PanelResult<()> {
    let mut command = tokio::process::Command::new(java);
    command
        .arg("-jar")
        .arg(jar)
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command.spawn().map_err(|error| PanelError::io(format!("Installer non avviabile: {error}")))?;
    let mut tail: Vec<String> = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            progress.emit(stage, line.clone(), None);
            tail.push(line);
            if tail.len() > 20 {
                tail.remove(0);
            }
        }
    }
    let status = child.wait().await?;
    if !status.success() {
        return Err(PanelError::from(format!(
            "L'installer è terminato con errore (codice {}): {}",
            status.code().unwrap_or(-1),
            tail.last().cloned().unwrap_or_default()
        )));
    }
    Ok(())
}

/// Modern installers write `libraries/.../<os>_args.txt`; older ones leave a runnable jar.
pub fn launch_after_install(root: &Path, library_dir: &Path, jar_prefix: &str) -> PanelResult<LaunchSpec> {
    let args = library_dir.join(if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" });
    if args.is_file() {
        let relative = args.strip_prefix(root).map(Path::to_path_buf).unwrap_or(args);
        return Ok(LaunchSpec::ArgsFile { path: root.join(relative) });
    }
    let mut jars: Vec<PathBuf> = std::fs::read_dir(root)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_lowercase();
            name.starts_with(jar_prefix) && name.ends_with(".jar") && !name.contains("installer")
        })
        .collect();
    // `-shim.jar` (1.20.4+) and `-universal.jar` (old versions) are the launchable ones.
    jars.sort_by_key(|path| {
        let name = path.to_string_lossy().to_lowercase();
        (!name.contains("shim"), !name.contains("universal"))
    });
    jars.into_iter()
        .next()
        .map(|path| LaunchSpec::Jar { path })
        .ok_or_else(|| PanelError::not_found("L'installer non ha prodotto un file di avvio riconoscibile"))
}

fn cleanup_installer(root: &Path, jar: &Path) {
    let _ = std::fs::remove_file(jar);
    let _ = std::fs::remove_file(jar.with_extension("jar.log"));
    let _ = std::fs::remove_file(root.join("installer.log"));
}

pub fn maven_versions(xml: &str) -> Vec<String> {
    xml.split("<version>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</version>").next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

async fn fetch_text(key: &str, url: &str) -> PanelResult<String> {
    let path = crate::paths::Layout::app().cache().join("api").join(format!("{key}.txt"));
    let fresh = std::fs::metadata(&path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age < cache::SHORT);
    if fresh {
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Ok(text);
        }
    }
    match crate::net::http().get(url).send().await.and_then(|response| response.error_for_status()) {
        Ok(response) => {
            let text = response.text().await?;
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&path, &text);
            Ok(text)
        }
        Err(error) => std::fs::read_to_string(&path).map_err(|_| PanelError::network(format!("{url}: {error}"))),
    }
}

/// Minecraft versions that have a Forge promotion, newest first.
pub fn forge_versions_from_promotions(value: &Value) -> Vec<VersionEntry> {
    let mut versions: Vec<VersionEntry> = value
        .get("promos")
        .and_then(Value::as_object)
        .map(|promos| {
            promos
                .keys()
                .filter_map(|key| key.strip_suffix("-latest"))
                .map(|mc| VersionEntry { id: mc.to_string(), stable: !mc.contains("pre") })
                .collect()
        })
        .unwrap_or_default();
    super::sort_versions_desc(&mut versions);
    versions
}

pub fn forge_builds(versions: &[String], promotions: &Value, mc: &str) -> Vec<BuildEntry> {
    let promo = |kind: &str| promotions.get("promos").and_then(|promos| promos.get(format!("{mc}-{kind}"))).and_then(Value::as_str).map(str::to_string);
    let recommended = promo("recommended");
    let prefix = format!("{mc}-");
    let mut builds: Vec<BuildEntry> = versions
        .iter()
        .filter_map(|version| version.strip_prefix(&prefix))
        .filter(|build| build.chars().next().is_some_and(|ch| ch.is_ascii_digit()) && !build.contains('-'))
        .map(|build| BuildEntry {
            id: build.to_string(),
            stable: recommended.as_deref() == Some(build),
            label: if recommended.as_deref() == Some(build) { format!("{build} · consigliata") } else { build.to_string() },
        })
        .collect();
    builds.sort_by(|left, right| super::compare_versions(&right.id, &left.id));
    builds
}

/// NeoForge `21.1.77` targets Minecraft 1.21.1, `21.0.x` targets 1.21 and the
/// year-based `26.3.0.16` targets 26.3 (a non-zero third part is a hotfix: 26.1.2).
pub fn neoforge_minecraft(version: &str) -> Option<String> {
    let core = version.split('-').next()?;
    let parts: Vec<u32> = core.split('.').map(|part| part.parse().ok()).collect::<Option<Vec<_>>>()?;
    match parts.as_slice() {
        [year, drop, hotfix, _build] if *year >= 25 => Some(if *hotfix == 0 { format!("{year}.{drop}") } else { format!("{year}.{drop}.{hotfix}") }),
        [major, 0, _] => Some(format!("1.{major}")),
        [major, minor, _] => Some(format!("1.{major}.{minor}")),
        _ => None,
    }
}

/// Every Minecraft version with at least one build; those with only beta builds are marked unstable.
pub fn neoforge_versions(builds: &[String]) -> Vec<VersionEntry> {
    let mut versions: Vec<VersionEntry> = Vec::new();
    for build in builds {
        let Some(mc) = neoforge_minecraft(build) else { continue };
        let stable = !build.contains("beta") && !build.contains("alpha");
        match versions.iter_mut().find(|entry| entry.id == mc) {
            Some(entry) => entry.stable |= stable,
            None => versions.push(VersionEntry { id: mc, stable }),
        }
    }
    super::sort_versions_desc(&mut versions);
    versions
}

pub fn neoforge_builds(builds: &[String], mc: &str) -> Vec<BuildEntry> {
    let mut entries: Vec<BuildEntry> = builds
        .iter()
        .filter(|build| neoforge_minecraft(build).as_deref() == Some(mc))
        .map(|build| BuildEntry {
            id: build.clone(),
            stable: !build.contains("beta") && !build.contains("alpha"),
            label: build.clone(),
        })
        .collect();
    entries.sort_by(|left, right| super::compare_versions(&right.id, &left.id));
    entries
}

fn pick_build(builds: &[BuildEntry], wanted: Option<&str>) -> Option<String> {
    match wanted {
        Some(wanted) => builds.iter().find(|build| build.id == wanted).map(|build| build.id.clone()),
        None => builds.iter().find(|build| build.stable).or_else(|| builds.first()).map(|build| build.id.clone()),
    }
}

impl Forge {
    async fn metadata() -> PanelResult<(Vec<String>, Value)> {
        let xml = fetch_text("forge-maven", &format!("{FORGE_MAVEN}/maven-metadata.xml")).await?;
        let promotions = cache::json("forge-promotions", FORGE_PROMOTIONS, cache::SHORT).await?;
        Ok((maven_versions(&xml), promotions))
    }

    /// Installs Forge `mc-build` into `root`; shared with SpongeForge.
    pub async fn install_version(root: &Path, mc: &str, build: &str, java: &Path, progress: &ProgressTx) -> PanelResult<LaunchSpec> {
        let full = format!("{mc}-{build}");
        let jar = root.join(format!("forge-{full}-installer.jar"));
        progress.emit("Forge", format!("Download dell'installer Forge {full}..."), Some(0.0));
        crate::net::download(&format!("{FORGE_MAVEN}/{full}/forge-{full}-installer.jar"), &jar, "Forge", progress).await?;
        progress.emit("Forge", "Installazione di Forge (scarica le librerie, può richiedere qualche minuto)...", None);
        run_installer(java, &jar, &["--installServer"], root, "Forge", progress).await?;
        cleanup_installer(root, &jar);
        launch_after_install(root, &root.join("libraries/net/minecraftforge/forge").join(&full), "forge-")
    }
}

impl Provider for Forge {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Forge
    }

    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move {
            let promotions = cache::json("forge-promotions", FORGE_PROMOTIONS, cache::SHORT).await?;
            Ok(forge_versions_from_promotions(&promotions))
        })
    }

    fn builds<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move {
            let (versions, promotions) = Self::metadata().await?;
            Ok(forge_builds(&versions, &promotions, version))
        })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let (versions, promotions) = Self::metadata().await?;
            let builds = forge_builds(&versions, &promotions, request.mc_version);
            let build = pick_build(&builds, request.build).ok_or_else(|| PanelError::not_found(format!("Nessuna build di Forge per {}", request.mc_version)))?;
            let launch = Self::install_version(request.root, request.mc_version, &build, request.java, request.progress).await?;
            Ok(Installed { launch, mc_version: request.mc_version.to_string(), build: Some(build) })
        })
    }
}

impl NeoForge {
    async fn all_builds() -> PanelResult<Vec<String>> {
        let xml = fetch_text("neoforge-maven", &format!("{NEOFORGE_MAVEN}/maven-metadata.xml")).await?;
        Ok(maven_versions(&xml))
    }
}

impl Provider for NeoForge {
    fn kind(&self) -> ProviderKind {
        ProviderKind::NeoForge
    }

    fn versions(&self, _snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>> {
        Box::pin(async move { Ok(neoforge_versions(&Self::all_builds().await?)) })
    }

    fn builds<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async move { Ok(neoforge_builds(&Self::all_builds().await?, version)) })
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>> {
        Box::pin(async move {
            let builds = neoforge_builds(&Self::all_builds().await?, request.mc_version);
            let build = pick_build(&builds, request.build).ok_or_else(|| PanelError::not_found(format!("Nessuna build di NeoForge per {}", request.mc_version)))?;
            let jar = request.root.join(format!("neoforge-{build}-installer.jar"));
            request.progress.emit("NeoForge", format!("Download dell'installer NeoForge {build}..."), Some(0.0));
            crate::net::download(&format!("{NEOFORGE_MAVEN}/{build}/neoforge-{build}-installer.jar"), &jar, "NeoForge", request.progress).await?;
            request.progress.emit("NeoForge", "Installazione di NeoForge (scarica le librerie, può richiedere qualche minuto)...", None);
            run_installer(request.java, &jar, &["--installServer"], request.root, "NeoForge", request.progress).await?;
            cleanup_installer(request.root, &jar);
            let launch = launch_after_install(request.root, &request.root.join("libraries/net/neoforged/neoforge").join(&build), "neoforge-")?;
            Ok(Installed { launch, mc_version: request.mc_version.to_string(), build: Some(build) })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_neoforge_versions_to_minecraft() {
        assert_eq!(neoforge_minecraft("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(neoforge_minecraft("21.0.167").as_deref(), Some("1.21"));
        assert_eq!(neoforge_minecraft("20.2.12-beta").as_deref(), Some("1.20.2"));
        assert_eq!(neoforge_minecraft("26.3.0.16-beta").as_deref(), Some("26.3"));
        assert_eq!(neoforge_minecraft("26.1.2.5").as_deref(), Some("26.1.2"));
        let builds: Vec<String> = ["21.1.76", "21.1.77", "26.3.0.16-beta"].iter().map(|value| value.to_string()).collect();
        let versions = neoforge_versions(&builds);
        assert_eq!(versions[0].id, "26.3");
        assert!(!versions[0].stable);
        assert_eq!(neoforge_builds(&builds, "1.21.1")[0].id, "21.1.77");
    }

    #[test]
    fn reads_forge_metadata() {
        let xml = "<versions><version>1.21.1-52.1.0</version><version>1.21.1-52.0.9</version><version>1.7.10-10.13.4.1614-1.7.10</version></versions>";
        let versions = maven_versions(xml);
        let promotions = json!({"promos": {"1.21.1-latest": "52.1.0", "1.21.1-recommended": "52.0.9", "26.3-latest": "66.0.3"}});
        let builds = forge_builds(&versions, &promotions, "1.21.1");
        assert_eq!(builds[0].id, "52.1.0");
        assert!(builds[1].stable);
        assert_eq!(pick_build(&builds, None).as_deref(), Some("52.0.9"));
        assert_eq!(forge_versions_from_promotions(&promotions)[0].id, "26.3");
    }

    #[test]
    fn prefers_args_files_then_shim_jars() {
        let root = std::env::temp_dir().join(format!("voxel-forge-{}", uuid::Uuid::new_v4()));
        let library = root.join("libraries/net/minecraftforge/forge/1.21.1-52.1.0");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::write(root.join("forge-1.21.1-52.1.0-shim.jar"), b"x").unwrap();
        assert_eq!(launch_after_install(&root, &library, "forge-").unwrap(), LaunchSpec::Jar { path: root.join("forge-1.21.1-52.1.0-shim.jar") });
        let args = library.join(if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" });
        std::fs::write(&args, b"--launchTarget forgeserver").unwrap();
        assert_eq!(launch_after_install(&root, &library, "forge-").unwrap(), LaunchSpec::ArgsFile { path: args });
        let _ = std::fs::remove_dir_all(&root);
    }
}
