// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Whitelist, operators and bans. Running servers are changed through commands (they would
//! overwrite the JSON files); stopped servers get their files edited directly.

use std::path::Path;

use md5::{Digest, Md5};
use serde_json::{json, Value};

use crate::api::types::{IpBan, PlayerEntry, PlayerListKind};
use crate::{PanelError, PanelResult};

pub fn file_of(kind: PlayerListKind) -> &'static str {
    match kind {
        PlayerListKind::Whitelist => "whitelist.json",
        PlayerListKind::Operators => "ops.json",
        PlayerListKind::BannedPlayers => "banned-players.json",
        PlayerListKind::BannedIps => "banned-ips.json",
    }
}

fn read_array(root: &Path, kind: PlayerListKind) -> Vec<Value> {
    std::fs::read_to_string(root.join(file_of(kind)))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
}

fn write_array(root: &Path, kind: PlayerListKind, items: &[Value]) -> PanelResult<()> {
    let text = serde_json::to_string_pretty(items)?;
    std::fs::write(root.join(file_of(kind)), text)?;
    Ok(())
}

pub fn players(root: &Path, kind: PlayerListKind) -> Vec<PlayerEntry> {
    read_array(root, kind)
        .iter()
        .filter_map(|item| {
            Some(PlayerEntry {
                name: item.get("name")?.as_str()?.to_string(),
                uuid: item.get("uuid").and_then(Value::as_str).unwrap_or_default().to_string(),
                detail: item
                    .get("reason")
                    .or_else(|| item.get("level"))
                    .map(|value| value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string()))
                    .unwrap_or_default(),
            })
        })
        .collect()
}

pub fn ip_bans(root: &Path) -> Vec<IpBan> {
    read_array(root, PlayerListKind::BannedIps)
        .iter()
        .filter_map(|item| {
            Some(IpBan {
                ip: item.get("ip")?.as_str()?.to_string(),
                reason: item.get("reason").and_then(Value::as_str).unwrap_or_default().to_string(),
            })
        })
        .collect()
}

pub fn valid_name(name: &str) -> bool {
    (1..=16).contains(&name.len()) && name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// UUID used by servers in offline mode: a v3 UUID of `OfflinePlayer:<name>`.
pub fn offline_uuid(name: &str) -> String {
    let mut hash: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    dashed(&hash.iter().map(|byte| format!("{byte:02x}")).collect::<String>())
}

fn dashed(hex: &str) -> String {
    if hex.len() != 32 {
        return hex.to_string();
    }
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

/// Online UUID from the Mojang API, with the correctly cased name.
pub async fn lookup(name: &str) -> PanelResult<(String, String)> {
    let response = crate::net::http()
        .get(format!("https://api.mojang.com/users/profiles/minecraft/{name}"))
        .send()
        .await
        .map_err(|error| PanelError::network(format!("API Mojang non raggiungibile: {error}")))?;
    if matches!(response.status().as_u16(), 204 | 404) {
        return Err(PanelError::not_found(format!("Nessun account Minecraft si chiama {name}")));
    }
    let value: Value = response.error_for_status()?.json().await?;
    let id = value.get("id").and_then(Value::as_str).ok_or("Risposta Mojang senza id")?;
    let real_name = value.get("name").and_then(Value::as_str).unwrap_or(name);
    Ok((dashed(id), real_name.to_string()))
}

/// Command a running server understands for this change.
pub fn command_for(kind: PlayerListKind, add: bool, target: &str, reason: &str) -> String {
    let reason = reason.trim();
    let with_reason = |base: String| if reason.is_empty() { base } else { format!("{base} {reason}") };
    match (kind, add) {
        (PlayerListKind::Whitelist, true) => format!("whitelist add {target}"),
        (PlayerListKind::Whitelist, false) => format!("whitelist remove {target}"),
        (PlayerListKind::Operators, true) => format!("op {target}"),
        (PlayerListKind::Operators, false) => format!("deop {target}"),
        (PlayerListKind::BannedPlayers, true) => with_reason(format!("ban {target}")),
        (PlayerListKind::BannedPlayers, false) => format!("pardon {target}"),
        (PlayerListKind::BannedIps, true) => with_reason(format!("ban-ip {target}")),
        (PlayerListKind::BannedIps, false) => format!("pardon-ip {target}"),
    }
}

fn now_text() -> String {
    // Same shape Minecraft writes ("2026-09-25 10:00:00 +0000").
    let secs = crate::paths::unix_now();
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} +0000", rem / 3600, (rem % 3600) / 60, rem % 60)
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Edits the JSON file of a stopped server.
pub fn edit_file(root: &Path, kind: PlayerListKind, add: bool, uuid: &str, name: &str, reason: &str) -> PanelResult<()> {
    let mut items = read_array(root, kind);
    let key = if kind == PlayerListKind::BannedIps { "ip" } else { "name" };
    items.retain(|item| !item.get(key).and_then(Value::as_str).is_some_and(|value| value.eq_ignore_ascii_case(name)));
    if add {
        let reason = if reason.trim().is_empty() { "Banned by an operator." } else { reason.trim() };
        items.push(match kind {
            PlayerListKind::Whitelist => json!({"uuid": uuid, "name": name}),
            PlayerListKind::Operators => json!({"uuid": uuid, "name": name, "level": 4, "bypassesPlayerLimit": false}),
            PlayerListKind::BannedPlayers => json!({"uuid": uuid, "name": name, "created": now_text(), "source": "VoxelPanel", "expires": "forever", "reason": reason}),
            PlayerListKind::BannedIps => json!({"ip": name, "created": now_text(), "source": "VoxelPanel", "expires": "forever", "reason": reason}),
        });
    }
    write_array(root, kind, &items)
}

pub fn valid_ip(value: &str) -> bool {
    value.parse::<std::net::IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_offline_uuids() {
        // Known value used by offline-mode servers.
        assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
    }

    #[test]
    fn edits_lists_of_stopped_servers() {
        let root = std::env::temp_dir().join(format!("voxel-players-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        edit_file(&root, PlayerListKind::Operators, true, "u-1", "Steve", "").unwrap();
        edit_file(&root, PlayerListKind::BannedPlayers, true, "u-2", "Griefer", "spam").unwrap();
        edit_file(&root, PlayerListKind::BannedIps, true, "", "10.0.0.1", "").unwrap();
        assert_eq!(players(&root, PlayerListKind::Operators)[0].name, "Steve");
        assert_eq!(players(&root, PlayerListKind::BannedPlayers)[0].detail, "spam");
        assert_eq!(ip_bans(&root)[0].ip, "10.0.0.1");
        edit_file(&root, PlayerListKind::Operators, false, "", "steve", "").unwrap();
        assert!(players(&root, PlayerListKind::Operators).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn builds_commands_and_dates() {
        assert_eq!(command_for(PlayerListKind::BannedPlayers, true, "Griefer", "spam"), "ban Griefer spam");
        assert_eq!(command_for(PlayerListKind::Whitelist, false, "Steve", ""), "whitelist remove Steve");
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_721), (2026, 9, 25));
        assert!(valid_name("Alex_01") && !valid_name("bad name"));
        assert!(valid_ip("192.168.1.2") && !valid_ip("nope"));
    }
}
