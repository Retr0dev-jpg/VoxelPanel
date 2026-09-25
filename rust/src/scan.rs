// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::{Path, PathBuf};

use crate::api::types::ProviderKind;
use crate::java_runtime::{self, JavaRuntime};
use crate::script::{self, ParsedStart};
use crate::LaunchSpec;

#[derive(Debug, Clone, Default)]
pub struct DetectedServer {
    pub provider: ProviderKind,
    pub mc_version: Option<String>,
    pub java_major: Option<u32>,
    pub java_home: Option<PathBuf>,
    pub launch: LaunchSpec,
    pub ram_min: Option<String>,
    pub ram_max: Option<String>,
    pub jvm_flags: Vec<String>,
    pub plugin_count: u32,
    pub world_count: u32,
    pub has_eula: bool,
}

pub fn derive_java_version(name: &str) -> Option<u32> {
    let lower = name.to_lowercase();
    if let Some(rest) = lower.strip_prefix("jdk-") {
        return rest
            .split(['-', '.', '+', '_'])
            .next()
            .and_then(|value| value.parse().ok());
    }
    if let Some(rest) = lower.strip_prefix("jdk") {
        return rest
            .split(['u', '-', '.', '+', '_'])
            .next()
            .and_then(|value| value.parse().ok());
    }
    None
}

pub fn detect(root: &Path) -> DetectedServer {
    let runtimes = java_runtime::scan_runtime_dir(&root.join("runtime"));
    let jars = scan_jars(root);
    let start = script::parse_start_script(root).unwrap_or_default();
    let jar_path = start
        .jar_path
        .clone()
        .filter(|path| path.exists())
        .or_else(|| jars.first().cloned());
    let (provider, mc_version) = crate::providers::detect(root, jar_path.as_deref());
    let java_home = java_home_from_start(root, &start, &runtimes);
    let java_major = java_home.as_ref().and_then(|path| {
        java_runtime::derive_from_home(path).or_else(|| {
            runtimes
                .iter()
                .find(|runtime| crate::platform::same_path(&runtime.home, path))
                .and_then(|runtime| runtime.major)
        })
    });
    DetectedServer {
        provider,
        mc_version,
        java_major,
        java_home,
        launch: jar_path.map(|path| LaunchSpec::Jar { path }).unwrap_or_default(),
        ram_min: start.ram_min,
        ram_max: start.ram_max,
        jvm_flags: start.jvm_flags,
        plugin_count: crate::plugins::list(root).len() as u32,
        world_count: crate::worlds::list(root, None).len() as u32,
        has_eula: root.join("eula.txt").exists(),
    }
}

fn java_home_from_start(root: &Path, start: &ParsedStart, runtimes: &[JavaRuntime]) -> Option<PathBuf> {
    if let Some(java) = &start.java_path {
        if java.exists() {
            return java.parent().and_then(|bin| bin.parent()).map(Path::to_path_buf);
        }
    }
    runtimes.first().map(|runtime| runtime.home.clone()).or_else(|| {
        let _ = root;
        None
    })
}

fn scan_jars(root: &Path) -> Vec<PathBuf> {
    let mut jars = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return jars;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("jar"))
        {
            jars.push(path);
        }
    }
    // Known server jars first, so installers or unrelated jars are not picked.
    let known = |path: &PathBuf| crate::providers::detect(root, Some(path)).0 != ProviderKind::Custom;
    jars.sort_by(|left, right| known(right).cmp(&known(left)).then_with(|| left.file_name().cmp(&right.file_name())));
    jars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_jdk_folder_versions() {
        assert_eq!(derive_java_version("jdk-21.0.2+13"), Some(21));
        assert_eq!(derive_java_version("jdk-17.0.9"), Some(17));
        assert_eq!(derive_java_version("not-java"), None);
    }
}
