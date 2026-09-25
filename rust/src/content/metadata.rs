// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::io::Read;
use std::path::Path;

use serde_json::Value;
use sha1::{Digest, Sha1};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct JarMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
}

fn read_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut entry = zip.by_name(name).ok()?;
    let mut text = String::new();
    entry.by_ref().take(256 * 1024).read_to_string(&mut text).ok()?;
    Some(text)
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches(['"', '\'']).trim().to_string()
}

/// Top-level `key: value` pairs of a simple YAML file (plugin.yml does not need more).
pub fn parse_plugin_yml(text: &str) -> JarMetadata {
    let mut meta = JarMetadata::default();
    let mut in_authors = false;
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
            if in_authors {
                if let Some(author) = line.trim().strip_prefix('-') {
                    meta.authors.push(unquote(author));
                }
            }
            continue;
        }
        in_authors = false;
        let Some((key, value)) = line.split_once(':') else { continue };
        match key.trim() {
            "name" => meta.name = unquote(value),
            "version" => meta.version = unquote(value),
            "description" => meta.description = unquote(value),
            "author" => meta.authors.push(unquote(value)),
            "authors" => {
                let value = value.trim();
                if value.is_empty() {
                    in_authors = true;
                } else {
                    meta.authors.extend(value.trim_matches(['[', ']']).split(',').map(unquote).filter(|author| !author.is_empty()));
                }
            }
            _ => {}
        }
    }
    meta
}

fn authors_from_json(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string).or_else(|| item.get("name").and_then(Value::as_str).map(str::to_string))).collect())
        .unwrap_or_default()
}

pub fn parse_fabric_json(text: &str) -> Option<JarMetadata> {
    let value: Value = serde_json::from_str(text).ok()?;
    let field = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    Some(JarMetadata { name: if field("name").is_empty() { field("id") } else { field("name") }, version: field("version"), description: field("description"), authors: authors_from_json(value.get("authors")) })
}

pub fn parse_quilt_json(text: &str) -> Option<JarMetadata> {
    let value: Value = serde_json::from_str(text).ok()?;
    let loader = value.get("quilt_loader")?;
    let metadata = loader.get("metadata");
    let field = |source: Option<&Value>, key: &str| source.and_then(|source| source.get(key)).and_then(Value::as_str).unwrap_or_default().to_string();
    let contributors = metadata.and_then(|metadata| metadata.get("contributors")).and_then(Value::as_object).map(|map| map.keys().cloned().collect()).unwrap_or_default();
    Some(JarMetadata { name: field(metadata, "name"), version: field(Some(loader), "version"), description: field(metadata, "description"), authors: contributors })
}

pub fn parse_velocity_json(text: &str) -> Option<JarMetadata> {
    let value: Value = serde_json::from_str(text).ok()?;
    let field = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    Some(JarMetadata { name: if field("name").is_empty() { field("id") } else { field("name") }, version: field("version"), description: field("description"), authors: authors_from_json(value.get("authors")) })
}

/// First `[[mods]]` block of a Forge/NeoForge `mods.toml`.
pub fn parse_mods_toml(text: &str, manifest_version: Option<&str>) -> JarMetadata {
    let mut meta = JarMetadata::default();
    let mut in_mods = false;
    let mut multiline_description = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if multiline_description {
            if let Some(end) = trimmed.strip_suffix("'''") {
                meta.description.push_str(end);
                multiline_description = false;
            } else {
                meta.description.push_str(trimmed);
                meta.description.push(' ');
            }
            continue;
        }
        if trimmed.starts_with("[[") {
            if in_mods {
                break;
            }
            in_mods = trimmed == "[[mods]]";
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else { continue };
        let value = value.trim();
        match (in_mods, key.trim()) {
            (true, "displayName") => meta.name = unquote(value),
            (true, "version") => meta.version = unquote(value),
            (true, "description") => {
                if let Some(rest) = value.strip_prefix("'''") {
                    match rest.strip_suffix("'''") {
                        Some(single) => meta.description = single.trim().to_string(),
                        None => {
                            meta.description = format!("{} ", rest.trim());
                            multiline_description = true;
                        }
                    }
                } else {
                    meta.description = unquote(value);
                }
            }
            (_, "authors") => meta.authors = unquote(value).split(',').map(|author| author.trim().to_string()).filter(|author| !author.is_empty()).collect(),
            _ => {}
        }
    }
    if meta.version.contains("${") {
        meta.version = manifest_version.unwrap_or_default().to_string();
    }
    meta.description = meta.description.trim().to_string();
    meta
}

fn manifest_version(zip: &mut zip::ZipArchive<std::fs::File>) -> Option<String> {
    read_entry(zip, "META-INF/MANIFEST.MF")?
        .lines()
        .find_map(|line| line.strip_prefix("Implementation-Version:"))
        .map(|value| value.trim().to_string())
}

/// Metadata of a plugin or mod jar; `None` when the jar has no known descriptor.
pub fn read(path: &Path) -> Option<JarMetadata> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    for name in ["paper-plugin.yml", "plugin.yml", "bungee.yml"] {
        if let Some(text) = read_entry(&mut zip, name) {
            return Some(parse_plugin_yml(&text));
        }
    }
    if let Some(text) = read_entry(&mut zip, "velocity-plugin.json") {
        return parse_velocity_json(&text);
    }
    if let Some(text) = read_entry(&mut zip, "quilt.mod.json") {
        return parse_quilt_json(&text);
    }
    if let Some(text) = read_entry(&mut zip, "fabric.mod.json") {
        return parse_fabric_json(&text);
    }
    for name in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
        if let Some(text) = read_entry(&mut zip, name) {
            let version = manifest_version(&mut zip);
            return Some(parse_mods_toml(&text, version.as_deref()));
        }
    }
    None
}

pub fn sha1_of(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plugin_yml() {
        let meta = parse_plugin_yml("name: LuckPerms\nversion: '5.4.1'\nmain: me.lucko.Main\nauthors: [Luck, Other]\ndescription: \"Permissions\"\ncommands:\n  lp:\n    description: nested\n");
        assert_eq!(meta, JarMetadata { name: "LuckPerms".into(), version: "5.4.1".into(), description: "Permissions".into(), authors: vec!["Luck".into(), "Other".into()] });
        let list = parse_plugin_yml("name: X\nauthors:\n  - A\n  - B\nversion: 1\n");
        assert_eq!(list.authors, vec!["A", "B"]);
    }

    #[test]
    fn parses_mod_descriptors() {
        let fabric = parse_fabric_json(r#"{"id": "sodium", "name": "Sodium", "version": "0.6.0", "authors": ["jellysquid", {"name": "IMS"}]}"#).unwrap();
        assert_eq!(fabric.authors, vec!["jellysquid", "IMS"]);
        let toml = "modLoader=\"javafml\"\n[[mods]]\nmodId=\"create\"\nversion=\"${file.jarVersion}\"\ndisplayName=\"Create\"\ndescription='''\nBuilding tools\nand more'''\n[[dependencies.create]]\nmodId=\"forge\"\n";
        let meta = parse_mods_toml(toml, Some("6.0.1"));
        assert_eq!((meta.name.as_str(), meta.version.as_str(), meta.description.as_str()), ("Create", "6.0.1", "Building tools and more"));
    }

    #[test]
    fn reads_jars() {
        let path = std::env::temp_dir().join(format!("voxel-meta-{}.jar", uuid::Uuid::new_v4()));
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            zip.start_file("plugin.yml", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut zip, b"name: Demo\nversion: 2.0\n").unwrap();
            zip.finish().unwrap();
        }
        assert_eq!(read(&path).unwrap().name, "Demo");
        assert_eq!(sha1_of(&path).unwrap().len(), 40);
        let _ = std::fs::remove_file(path);
    }
}
