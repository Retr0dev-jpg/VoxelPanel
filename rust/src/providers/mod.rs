// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Server software providers: where versions come from and how a server is installed.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use futures::future::BoxFuture;

use crate::api::types::{BuildEntry, ProviderCategory, ProviderInfo, ProviderKind, VersionEntry};
use crate::{LaunchSpec, PanelError, PanelResult, ProgressTx};

pub mod fill;
pub mod leaf;
pub mod mojang;
pub mod pufferfish;
pub mod purpur;
pub mod spigot;

pub struct InstallRequest<'a> {
    pub root: &'a Path,
    pub mc_version: &'a str,
    /// `None` installs the latest build.
    pub build: Option<&'a str>,
    /// Java executable for providers that run an installer.
    pub java: &'a Path,
    pub progress: &'a ProgressTx,
}

#[derive(Debug, Clone)]
pub struct Installed {
    pub launch: LaunchSpec,
    pub mc_version: String,
    pub build: Option<String>,
}

pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    fn versions(&self, snapshots: bool) -> BoxFuture<'_, PanelResult<Vec<VersionEntry>>>;

    fn builds<'a>(&'a self, _version: &'a str) -> BoxFuture<'a, PanelResult<Vec<BuildEntry>>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn required_java<'a>(&'a self, version: &'a str) -> BoxFuture<'a, PanelResult<u32>> {
        Box::pin(mojang::required_java(version))
    }

    fn install<'a>(&'a self, request: InstallRequest<'a>) -> BoxFuture<'a, PanelResult<Installed>>;
}

/// Implemented providers, in the order shown by the wizard.
pub fn all() -> Vec<&'static dyn Provider> {
    vec![
        &mojang::VANILLA,
        &fill::PAPER,
        &purpur::PURPUR,
        &fill::FOLIA,
        &pufferfish::PUFFERFISH,
        &leaf::LEAF,
        &spigot::SPIGOT,
        &fill::VELOCITY,
    ]
}

pub fn get(kind: ProviderKind) -> PanelResult<&'static dyn Provider> {
    all()
        .into_iter()
        .find(|provider| provider.kind() == kind)
        .ok_or_else(|| PanelError::invalid(format!("{} non è ancora installabile da VoxelPanel.", meta(kind).name)))
}

pub fn id_of(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Vanilla => "vanilla",
        ProviderKind::Paper => "paper",
        ProviderKind::Folia => "folia",
        ProviderKind::Purpur => "purpur",
        ProviderKind::Pufferfish => "pufferfish",
        ProviderKind::Leaf => "leaf",
        ProviderKind::Spigot => "spigot",
        ProviderKind::Fabric => "fabric",
        ProviderKind::Quilt => "quilt",
        ProviderKind::Forge => "forge",
        ProviderKind::NeoForge => "neoforge",
        ProviderKind::Velocity => "velocity",
        ProviderKind::BungeeCord => "bungeecord",
        ProviderKind::Waterfall => "waterfall",
        ProviderKind::Mohist => "mohist",
        ProviderKind::Arclight => "arclight",
        ProviderKind::SpongeVanilla => "spongevanilla",
        ProviderKind::SpongeForge => "spongeforge",
        ProviderKind::Custom => "custom",
    }
}

#[cfg(test)]
pub fn from_id(id: &str) -> Option<ProviderKind> {
    ALL_KINDS.iter().copied().find(|kind| id_of(*kind) == id)
}

#[cfg(test)]
pub const ALL_KINDS: &[ProviderKind] = &[
    ProviderKind::Vanilla,
    ProviderKind::Paper,
    ProviderKind::Folia,
    ProviderKind::Purpur,
    ProviderKind::Pufferfish,
    ProviderKind::Leaf,
    ProviderKind::Spigot,
    ProviderKind::Fabric,
    ProviderKind::Quilt,
    ProviderKind::Forge,
    ProviderKind::NeoForge,
    ProviderKind::Velocity,
    ProviderKind::BungeeCord,
    ProviderKind::Waterfall,
    ProviderKind::Mohist,
    ProviderKind::Arclight,
    ProviderKind::SpongeVanilla,
    ProviderKind::SpongeForge,
    ProviderKind::Custom,
];

pub fn is_proxy(kind: ProviderKind) -> bool {
    matches!(kind, ProviderKind::Velocity | ProviderKind::BungeeCord | ProviderKind::Waterfall)
}

/// Static description of every provider, implemented or not (records may reference any of them).
pub fn meta(kind: ProviderKind) -> ProviderInfo {
    use ProviderCategory as C;
    let bukkit = ["server.properties", "bukkit.yml", "spigot.yml", "commands.yml", "permissions.yml"];
    let paper = ["config/paper-global.yml", "config/paper-world-defaults.yml"];
    let (name, category, description, plugins, mods, snapshots, builds, deprecated, note, mut files): (&str, C, &str, bool, bool, bool, bool, bool, &str, Vec<&str>) = match kind {
        ProviderKind::Vanilla => ("Vanilla", C::Vanilla, "Server ufficiale Mojang, senza modifiche.", false, false, true, false, false, "", vec!["server.properties"]),
        ProviderKind::Paper => ("Paper", C::Plugins, "Il server a plugin più diffuso: veloce, stabile, compatibile con Bukkit e Spigot.", true, false, true, true, false, "", [&bukkit[..], &paper[..]].concat()),
        ProviderKind::Folia => ("Folia", C::Plugins, "Fork di Paper multithread per server con molti giocatori. Molti plugin non sono compatibili.", true, false, false, true, false, "", [&bukkit[..], &paper[..]].concat()),
        ProviderKind::Purpur => ("Purpur", C::Plugins, "Fork di Paper con centinaia di opzioni di gameplay configurabili.", true, false, false, true, false, "", [&bukkit[..], &paper[..], &["purpur.yml"][..]].concat()),
        ProviderKind::Pufferfish => ("Pufferfish", C::Plugins, "Fork di Paper orientato alle prestazioni per server grandi.", true, false, false, false, false, "", [&bukkit[..], &paper[..], &["pufferfish.yml"][..]].concat()),
        ProviderKind::Leaf => ("Leaf", C::Plugins, "Fork di Paper e Gale con ottimizzazioni aggressive.", true, false, false, true, false, "", [&bukkit[..], &paper[..], &["config/leaf-global.yml"][..]].concat()),
        ProviderKind::Spigot => ("Spigot", C::Plugins, "Il server a plugin storico, compilato in locale con BuildTools.", true, false, false, false, false, "Richiede Git e alcuni minuti di compilazione.", bukkit.to_vec()),
        ProviderKind::Fabric => ("Fabric", C::Modded, "Loader di mod leggero e aggiornato rapidamente.", false, true, true, true, false, "", vec!["server.properties"]),
        ProviderKind::Quilt => ("Quilt", C::Modded, "Fork di Fabric compatibile con la maggior parte delle sue mod.", false, true, true, true, false, "", vec!["server.properties"]),
        ProviderKind::Forge => ("Forge", C::Modded, "Il loader di mod storico, con il catalogo più ampio.", false, true, false, true, false, "", vec!["server.properties"]),
        ProviderKind::NeoForge => ("NeoForge", C::Modded, "Il successore moderno di Forge.", false, true, false, true, false, "", vec!["server.properties"]),
        ProviderKind::Velocity => ("Velocity", C::Proxy, "Proxy moderno e sicuro per collegare più server.", true, false, true, true, false, "", vec!["velocity.toml"]),
        ProviderKind::BungeeCord => ("BungeeCord", C::Proxy, "Il proxy storico di SpigotMC.", true, false, false, false, false, "", vec!["config.yml"]),
        ProviderKind::Waterfall => ("Waterfall", C::Proxy, "Fork di BungeeCord di PaperMC, non più sviluppato.", true, false, false, true, true, "Progetto archiviato: preferisci Velocity.", vec!["config.yml", "waterfall.yml"]),
        ProviderKind::Mohist => ("Mohist", C::Hybrid, "Forge o NeoForge con supporto ai plugin Bukkit.", true, true, false, true, false, "", [&bukkit[..], &["mohist-config/mohist.yml"][..]].concat()),
        ProviderKind::Arclight => ("Arclight", C::Hybrid, "Plugin Bukkit su Forge, NeoForge o Fabric.", true, true, false, true, false, "", [&bukkit[..], &["arclight.conf"][..]].concat()),
        ProviderKind::SpongeVanilla => ("SpongeVanilla", C::Hybrid, "Piattaforma di plugin Sponge su server Vanilla.", true, false, false, true, false, "", vec!["server.properties", "config/sponge/global.conf"]),
        ProviderKind::SpongeForge => ("SpongeForge", C::Hybrid, "Plugin Sponge insieme alle mod Forge.", true, true, false, true, false, "", vec!["server.properties", "config/sponge/global.conf"]),
        ProviderKind::Custom => ("Jar personalizzato", C::Other, "Un jar scelto da te: VoxelPanel lo avvia ma non lo aggiorna.", true, true, false, false, false, "", vec!["server.properties"]),
    };
    if is_proxy(kind) {
        files.retain(|file| *file != "server.properties");
    }
    ProviderInfo {
        kind,
        id: id_of(kind).to_string(),
        name: name.to_string(),
        category,
        description: description.to_string(),
        supports_plugins: plugins,
        supports_mods: mods,
        has_worlds: !is_proxy(kind),
        is_proxy: is_proxy(kind),
        has_snapshots: snapshots,
        has_builds: builds,
        deprecated,
        note: note.to_string(),
        config_files: files.into_iter().map(str::to_string).collect(),
    }
}

/// Recognises the server software from the jar name and characteristic files.
pub fn detect(root: &Path, jar: Option<&Path>) -> (ProviderKind, Option<String>) {
    let file_name = jar
        .and_then(|jar| jar.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let version = version_in_name(&file_name);
    let by_name = [
        ("purpur", ProviderKind::Purpur),
        ("folia", ProviderKind::Folia),
        ("pufferfish", ProviderKind::Pufferfish),
        ("leaf", ProviderKind::Leaf),
        ("paper", ProviderKind::Paper),
        ("spigot", ProviderKind::Spigot),
        ("velocity", ProviderKind::Velocity),
        ("waterfall", ProviderKind::Waterfall),
        ("bungeecord", ProviderKind::BungeeCord),
        ("mohist", ProviderKind::Mohist),
        ("arclight", ProviderKind::Arclight),
        ("spongevanilla", ProviderKind::SpongeVanilla),
        ("spongeforge", ProviderKind::SpongeForge),
        ("quilt-server", ProviderKind::Quilt),
        ("fabric-server", ProviderKind::Fabric),
        ("neoforge", ProviderKind::NeoForge),
        ("forge", ProviderKind::Forge),
        ("minecraft_server", ProviderKind::Vanilla),
    ];
    if let Some((_, kind)) = by_name.iter().find(|(needle, _)| file_name.contains(needle)) {
        return (*kind, version);
    }
    let exists = |relative: &str| root.join(relative).exists();
    let kind = if exists("libraries/net/neoforged") {
        ProviderKind::NeoForge
    } else if exists("libraries/net/minecraftforge") {
        ProviderKind::Forge
    } else if exists(".fabric") {
        ProviderKind::Fabric
    } else if exists(".quilt") {
        ProviderKind::Quilt
    } else if exists("velocity.toml") {
        ProviderKind::Velocity
    } else if exists("purpur.yml") {
        ProviderKind::Purpur
    } else if exists("config/paper-global.yml") || exists("paper.yml") {
        ProviderKind::Paper
    } else if exists("spigot.yml") {
        ProviderKind::Spigot
    } else if file_name == "server.jar" {
        ProviderKind::Vanilla
    } else {
        ProviderKind::Custom
    };
    (kind, version)
}

/// Minecraft version inside a jar name like `paper-1.21.11-132.jar` or `leaf-26.2-4.jar`.
pub fn version_in_name(file_name: &str) -> Option<String> {
    let mut best: Option<String> = None;
    for part in file_name.trim_end_matches(".jar").split(['-', '_']) {
        let dots = part.matches('.').count();
        let numeric = !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit() || ch == '.');
        if numeric && dots >= 1 && !part.starts_with('.') && !part.ends_with('.') {
            best.get_or_insert_with(|| part.to_string());
        }
    }
    best
}

pub fn compare_versions(left: &str, right: &str) -> Ordering {
    version_parts(left).cmp(&version_parts(right)).then_with(|| left.cmp(right))
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

pub fn sort_versions_desc(entries: &mut [VersionEntry]) {
    entries.sort_by(|left, right| compare_versions(&right.id, &left.id));
}

pub fn jar_destination(root: &Path, file_name: &str) -> PanelResult<PathBuf> {
    if file_name.is_empty() || file_name.contains(['/', '\\']) || file_name.contains("..") {
        return Err(PanelError::invalid(format!("Nome file non valido: {file_name}")));
    }
    Ok(root.join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_providers_from_jar_names() {
        let root = Path::new("missing-folder");
        let detect_name = |name: &str| detect(root, Some(Path::new(name)));
        assert_eq!(detect_name("paper-1.21.11-132.jar"), (ProviderKind::Paper, Some("1.21.11".into())));
        assert_eq!(detect_name("purpur-1.21.4-2400.jar").0, ProviderKind::Purpur);
        assert_eq!(detect_name("leaf-26.2-4.jar"), (ProviderKind::Leaf, Some("26.2".into())));
        assert_eq!(detect_name("velocity-3.4.0-SNAPSHOT-500.jar").0, ProviderKind::Velocity);
        assert_eq!(detect_name("minecraft_server.1.21.1.jar").0, ProviderKind::Vanilla);
        assert_eq!(detect_name("mystery.jar").0, ProviderKind::Custom);
    }

    #[test]
    fn detects_providers_from_files() {
        let root = std::env::temp_dir().join(format!("voxel-detect-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("libraries/net/neoforged")).unwrap();
        assert_eq!(detect(&root, None).0, ProviderKind::NeoForge);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn every_kind_has_metadata() {
        for kind in ALL_KINDS {
            let info = meta(*kind);
            assert!(!info.name.is_empty());
            assert_eq!(from_id(&info.id), Some(*kind));
        }
        assert!(!meta(ProviderKind::Velocity).has_worlds);
    }

    #[test]
    fn sorts_year_based_versions() {
        let mut entries: Vec<VersionEntry> = ["1.21.11", "26.1.2", "1.8.9", "26.3"]
            .iter()
            .map(|id| VersionEntry { id: id.to_string(), stable: true })
            .collect();
        sort_versions_desc(&mut entries);
        let ids: Vec<&str> = entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, vec!["26.3", "26.1.2", "1.21.11", "1.8.9"]);
    }
}

/// Talks to the real APIs: `cargo test live_ -- --ignored`.
#[cfg(test)]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn live_lists_versions_and_builds() {
        for provider in all() {
            let versions = provider.versions(false).await.unwrap_or_else(|error| panic!("{:?}: {}", provider.kind(), error.message));
            assert!(!versions.is_empty(), "{:?} has no versions", provider.kind());
            let newest = &versions[0].id;
            let builds = provider.builds(newest).await.unwrap();
            let java = provider.required_java(newest).await.unwrap_or(0);
            println!("{:?}: {} versions, newest {newest}, {} builds, java {java}", provider.kind(), versions.len(), builds.len());
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_installs_verified_jars() {
        let root = std::env::temp_dir().join(format!("voxel-live-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let progress = ProgressTx::silent();
        for kind in [ProviderKind::Vanilla, ProviderKind::Paper, ProviderKind::Purpur, ProviderKind::Leaf, ProviderKind::Velocity, ProviderKind::Pufferfish] {
            let provider = get(kind).unwrap();
            let version = provider.versions(false).await.unwrap()[0].id.clone();
            let installed = provider
                .install(InstallRequest { root: &root, mc_version: &version, build: None, java: Path::new("java"), progress: &progress })
                .await
                .unwrap_or_else(|error| panic!("{kind:?}: {}", error.message));
            let jar = installed.launch.jar().unwrap();
            assert!(std::fs::metadata(jar).unwrap().len() > 100_000, "{kind:?} jar too small");
            println!("{kind:?} {version} -> {}", jar.display());
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
