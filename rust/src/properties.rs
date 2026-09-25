// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub motd: String,
    pub port: u32,
    pub max_players: u32,
    pub online_mode: bool,
    pub difficulty: String,
    pub gamemode: String,
    pub view_distance: u32,
    pub simulation_distance: u32,
    pub white_list: bool,
    pub pvp: bool,
    pub spawn_protection: u32,
    pub level_name: String,
    pub level_seed: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            motd: "A Minecraft Server".into(),
            port: 25565,
            max_players: 20,
            online_mode: true,
            difficulty: "easy".into(),
            gamemode: "survival".into(),
            view_distance: 10,
            simulation_distance: 10,
            white_list: false,
            pvp: true,
            spawn_protection: 16,
            level_name: "world".into(),
            level_seed: String::new(),
        }
    }
}

impl Settings {
    pub fn to_map(&self) -> HashMap<String, String> {
        HashMap::from([
            ("motd".into(), self.motd.clone()),
            ("server-port".into(), self.port.to_string()),
            ("max-players".into(), self.max_players.to_string()),
            ("online-mode".into(), bool_text(self.online_mode)),
            ("difficulty".into(), self.difficulty.clone()),
            ("gamemode".into(), self.gamemode.clone()),
            ("view-distance".into(), self.view_distance.to_string()),
            (
                "simulation-distance".into(),
                self.simulation_distance.to_string(),
            ),
            ("white-list".into(), bool_text(self.white_list)),
            ("pvp".into(), bool_text(self.pvp)),
            ("spawn-protection".into(), self.spawn_protection.to_string()),
            ("level-name".into(), self.level_name.clone()),
            ("level-seed".into(), self.level_seed.clone()),
        ])
    }
}

fn bool_text(value: bool) -> String {
    if value { "true" } else { "false" }.to_string()
}

pub fn parse_map(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        map.insert(key.trim().to_string(), value.trim().to_string());
    }
    map
}

pub fn settings_from_map(map: &HashMap<String, String>) -> Settings {
    let defaults = Settings::default();
    Settings {
        motd: map.get("motd").cloned().unwrap_or(defaults.motd),
        port: map
            .get("server-port")
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.port),
        max_players: map
            .get("max-players")
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.max_players),
        online_mode: map
            .get("online-mode")
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(defaults.online_mode),
        difficulty: map.get("difficulty").cloned().unwrap_or(defaults.difficulty),
        gamemode: map.get("gamemode").cloned().unwrap_or(defaults.gamemode),
        view_distance: map
            .get("view-distance")
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.view_distance),
        simulation_distance: map
            .get("simulation-distance")
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.simulation_distance),
        white_list: map
            .get("white-list")
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(defaults.white_list),
        pvp: map
            .get("pvp")
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(defaults.pvp),
        spawn_protection: map
            .get("spawn-protection")
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.spawn_protection),
        level_name: map.get("level-name").cloned().unwrap_or(defaults.level_name),
        level_seed: map.get("level-seed").cloned().unwrap_or(defaults.level_seed),
    }
}

pub fn apply_updates(content: &str, updates: &HashMap<String, String>) -> String {
    let mut seen = HashSet::new();
    let mut out = String::new();
    for line in content.split_inclusive(['\n']) {
        let raw = line.trim_end_matches(['\r', '\n']);
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            out.push_str(raw);
            out.push_str("\r\n");
            continue;
        }
        if let Some((key, _)) = trimmed.split_once('=') {
            let key = key.trim();
            if let Some(value) = updates.get(key) {
                out.push_str(key);
                out.push('=');
                out.push_str(value);
                out.push_str("\r\n");
                seen.insert(key.to_string());
                continue;
            }
        }
        out.push_str(raw);
        out.push_str("\r\n");
    }
    for (key, value) in updates {
        if !seen.contains(key) {
            out.push_str(key);
            out.push('=');
            out.push_str(value);
            out.push_str("\r\n");
        }
    }
    out
}

pub fn read_entries(content: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        entries.push((key.trim().to_string(), value.trim().to_string()));
    }
    entries
}

pub fn load_entries(root: &Path) -> crate::PanelResult<Vec<(String, String)>> {
    let path = root.join("server.properties");
    if !path.is_file() {
        return Err("server.properties non trovato".into());
    }
    let content = std::fs::read_to_string(&path).map_err(|error| crate::PanelError::from(error.to_string()))?;
    Ok(read_entries(&content))
}

pub fn write_entries(root: &Path, entries: &[(String, String)]) -> crate::PanelResult<()> {
    let mut seen = HashSet::new();
    let mut updates = HashMap::new();
    for (key, value) in entries {
        validate_entry(key, value)?;
        if !seen.insert(key.clone()) {
            return Err(crate::PanelError::from(format!("Chiave duplicata: {key}")));
        }
        updates.insert(key.clone(), value.clone());
    }
    let path = root.join("server.properties");
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let next = apply_updates(&current, &updates);
    std::fs::write(path, next).map_err(|error| crate::PanelError::from(error.to_string()))
}

fn validate_entry(key: &str, value: &str) -> crate::PanelResult<()> {
    if key.is_empty() || key.contains(['\n', '\r', '=']) || value.contains(['\n', '\r']) {
        return Err(crate::PanelError::from(format!("Proprietà non valida: {key}")));
    }
    match key {
        "motd" if value.chars().count() > 256 => Err("MOTD non valido".into()),
        "server-port" => parse_range(value, 1, 65535, "Porta non valida"),
        "max-players" => parse_range(value, 1, 100_000, "Numero massimo di giocatori non valido"),
        "view-distance" | "simulation-distance" => parse_range(value, 2, 64, "Distanza di visualizzazione non valida"),
        "spawn-protection" => parse_range(value, 0, 999, "Spawn protection non valida"),
        "level-name" if !valid_level_name(value) => Err("Nome mondo non valido".into()),
        "level-seed" if value.chars().count() > 128 => Err("Seed non valido".into()),
        _ => Ok(()),
    }
}

fn parse_range(value: &str, min: u32, max: u32, message: &str) -> crate::PanelResult<()> {
    let parsed: u32 = value.parse().map_err(|_| message.to_string())?;
    if (min..=max).contains(&parsed) {
        Ok(())
    } else {
        Err(message.into())
    }
}

pub fn read_settings(root: &Path) -> Settings {
    let path = root.join("server.properties");
    let Ok(content) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    settings_from_map(&parse_map(&content))
}

pub fn write_settings(root: &Path, settings: &Settings) -> crate::PanelResult<()> {
    validate(settings)?;
    let path = root.join("server.properties");
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let base = if current.trim().is_empty() {
        "# Configurazione generata da VoxelPanel\r\n".to_string()
    } else {
        current
    };
    let next = apply_updates(&base, &settings.to_map());
    std::fs::write(path, next).map_err(|error| crate::PanelError::from(error.to_string()))
}

pub fn validate(settings: &Settings) -> crate::PanelResult<()> {
    if settings.motd.chars().count() > 256 || settings.motd.contains(['\n', '\r']) {
        return Err("MOTD non valido".into());
    }
    if !(1..=65535).contains(&settings.port) {
        return Err("Porta non valida".into());
    }
    if !(1..=100_000).contains(&settings.max_players) {
        return Err("Numero massimo di giocatori non valido".into());
    }
    if !matches!(settings.difficulty.as_str(), "peaceful" | "easy" | "normal" | "hard") {
        return Err("Difficoltà non valida".into());
    }
    if !matches!(
        settings.gamemode.as_str(),
        "survival" | "creative" | "adventure" | "spectator"
    ) {
        return Err("Gamemode non valido".into());
    }
    if !(2..=64).contains(&settings.view_distance) || !(2..=64).contains(&settings.simulation_distance)
    {
        return Err("Distanza di visualizzazione non valida".into());
    }
    if settings.spawn_protection > 999 {
        return Err("Spawn protection non valida".into());
    }
    if !valid_level_name(&settings.level_name) {
        return Err("Nome mondo non valido".into());
    }
    if settings.level_seed.chars().count() > 128 || settings.level_seed.contains(['\n', '\r']) {
        return Err("Seed non valido".into());
    }
    Ok(())
}

pub fn valid_level_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        && name != "."
        && name != ".."
        && !name.contains('\n')
}

pub fn read_port(root: &Path) -> Option<u32> {
    if !root.join("server.properties").exists() {
        return None;
    }
    Some(read_settings(root).port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_keys_and_keeps_comments() {
        let original = "# keep me\r\nmotd=Old\r\nserver-port=25565\r\ncustom-flag=yes\r\n";
        let mut updates = HashMap::new();
        updates.insert("motd".into(), "Nuovo".into());
        updates.insert("level-seed".into(), "abc".into());
        let next = apply_updates(original, &updates);
        assert!(next.contains("# keep me"));
        assert!(next.contains("motd=Nuovo"));
        assert!(next.contains("custom-flag=yes"));
        assert!(next.contains("level-seed=abc"));
        assert!(next.contains("server-port=25565"));
    }

    #[test]
    fn reads_unknown_keys_in_file_order() {
        let content = "motd=Ciao\n# nota\nenable-command-block=true\ncustom-flag=yes\n";
        let entries = read_entries(content);
        assert_eq!(
            entries,
            vec![
                ("motd".into(), "Ciao".into()),
                ("enable-command-block".into(), "true".into()),
                ("custom-flag".into(), "yes".into()),
            ]
        );
    }
}
