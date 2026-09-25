// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use crate::paths::Layout;
use crate::{process, PanelError, PanelResult};

use super::types::*;

pub async fn player_lists(id: String) -> PanelResult<PlayerLists> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let root = &record.root;
    Ok(PlayerLists {
        whitelist: crate::players::players(root, PlayerListKind::Whitelist),
        operators: crate::players::players(root, PlayerListKind::Operators),
        banned_players: crate::players::players(root, PlayerListKind::BannedPlayers),
        banned_ips: crate::players::ip_bans(root),
        whitelist_enabled: crate::properties::read_settings(root).white_list,
    })
}

/// Online players: asked through RCON when available, otherwise taken from the console log.
pub async fn online_players(id: String) -> PanelResult<Vec<String>> {
    if !process::is_running(&id) {
        return Ok(Vec::new());
    }
    match crate::rcon::execute_for(&id, "list").await {
        Ok(response) => Ok(crate::rcon::parse_list(&response)),
        Err(_) => Ok(process::snapshot(&id).players),
    }
}

/// Runs a console command, through RCON when possible so that its output can be returned.
pub async fn run_server_command(id: String, command: String) -> PanelResult<String> {
    if !process::is_running(&id) {
        return Err(PanelError::stopped());
    }
    let command = command.trim().trim_start_matches('/').to_string();
    if command.is_empty() || command.contains(['\n', '\r']) {
        return Err(PanelError::invalid("Comando non valido"));
    }
    match crate::rcon::execute_for(&id, &command).await {
        Ok(response) => Ok(crate::rcon::strip_formatting(&response)),
        Err(_) => {
            process::send_command(&id, &command).await?;
            Ok(String::new())
        }
    }
}

/// Adds or removes a player (or an IP) from a list. Running servers get the equivalent command.
pub async fn modify_player_list(id: String, list: PlayerListKind, add: bool, target: String, reason: String) -> PanelResult<()> {
    let target = target.trim().to_string();
    let valid = if list == PlayerListKind::BannedIps { crate::players::valid_ip(&target) } else { crate::players::valid_name(&target) };
    if !valid {
        return Err(PanelError::invalid(if list == PlayerListKind::BannedIps { "Indirizzo IP non valido" } else { "Nome giocatore non valido" }));
    }
    if reason.contains(['\n', '\r']) {
        return Err(PanelError::invalid("Motivo non valido"));
    }
    if process::is_running(&id) {
        let command = crate::players::command_for(list, add, &target, &reason);
        return run_server_command(id, command).await.map(|_| ());
    }
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let (uuid, name) = if list == PlayerListKind::BannedIps || !add {
        (String::new(), target)
    } else if crate::properties::read_settings(&record.root).online_mode {
        crate::players::lookup(&target).await?
    } else {
        (crate::players::offline_uuid(&target), target)
    };
    crate::players::edit_file(&record.root, list, add, &uuid, &name, &reason)
}
