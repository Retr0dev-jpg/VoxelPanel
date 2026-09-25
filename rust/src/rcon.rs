// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Minimal Source RCON client, used to query servers without writing to their console.

use std::collections::HashMap;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::paths::Layout;
use crate::{PanelError, PanelResult, RconConfig, ServerRecord};

const AUTH: i32 = 3;
const COMMAND: i32 = 2;
const TIMEOUT: Duration = Duration::from_secs(4);
const FIRST_PORT: u32 = 25575;

pub fn encode(id: i32, kind: i32, body: &str) -> Vec<u8> {
    let length = 4 + 4 + body.len() + 2;
    let mut packet = Vec::with_capacity(length + 4);
    packet.extend_from_slice(&(length as i32).to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&kind.to_le_bytes());
    packet.extend_from_slice(body.as_bytes());
    packet.extend_from_slice(&[0, 0]);
    packet
}

async fn read_packet(stream: &mut TcpStream) -> PanelResult<(i32, i32, String)> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length).await?;
    let length = i32::from_le_bytes(length);
    if !(10..=1_048_576).contains(&length) {
        return Err(PanelError::invalid("Risposta RCON non valida"));
    }
    let mut payload = vec![0u8; length as usize];
    stream.read_exact(&mut payload).await?;
    let id = i32::from_le_bytes(payload[0..4].try_into().unwrap_or_default());
    let kind = i32::from_le_bytes(payload[4..8].try_into().unwrap_or_default());
    let body = String::from_utf8_lossy(&payload[8..payload.len().saturating_sub(2)]).to_string();
    Ok((id, kind, body))
}

/// Connects, authenticates and runs one command, returning its text output.
pub async fn execute(config: &RconConfig, command: &str) -> PanelResult<String> {
    tokio::time::timeout(TIMEOUT, async {
        let mut stream = TcpStream::connect(("127.0.0.1", config.port as u16))
            .await
            .map_err(|error| PanelError::network(format!("RCON non raggiungibile: {error}")))?;
        stream.write_all(&encode(1, AUTH, &config.password)).await?;
        loop {
            let (id, kind, _) = read_packet(&mut stream).await?;
            // Some servers send an empty response value before the auth response.
            if kind == 2 {
                if id == -1 {
                    return Err(PanelError::invalid("Password RCON rifiutata"));
                }
                break;
            }
        }
        stream.write_all(&encode(2, COMMAND, command)).await?;
        let (_, _, body) = read_packet(&mut stream).await?;
        Ok(body)
    })
    .await
    .map_err(|_| PanelError::network("RCON non ha risposto in tempo"))?
}

pub fn strip_formatting(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '§' {
            chars.next();
            continue;
        }
        out.push(ch);
    }
    crate::process::strip_ansi(&out)
}

/// Parses `There are 2 of a max of 20 players online: Steve, Alex`.
pub fn parse_list(response: &str) -> Vec<String> {
    let plain = strip_formatting(response);
    let Some((_, names)) = plain.split_once(':') else {
        return Vec::new();
    };
    names
        .split(',')
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty() && !name.contains(' '))
        .collect()
}

fn random_password() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Makes sure RCON is usable before launch. User-provided settings are kept; otherwise RCON is
/// enabled on a free port with a random password. Proxies and opted-out servers are left alone.
pub fn ensure_configured(layout: &Layout, record: &mut ServerRecord) -> PanelResult<()> {
    if crate::providers::is_proxy(record.provider) || !record.manage_rcon {
        return Ok(());
    }
    let entries: HashMap<String, String> = crate::properties::load_entries(&record.root).unwrap_or_default().into_iter().collect();
    let enabled = entries.get("enable-rcon").is_some_and(|value| value == "true");
    let password = entries.get("rcon.password").cloned().unwrap_or_default();
    let port = entries.get("rcon.port").and_then(|value| value.parse::<u32>().ok());
    if enabled && !password.is_empty() {
        let config = RconConfig { port: port.unwrap_or(FIRST_PORT), password };
        if record.rcon.as_ref() != Some(&config) {
            record.rcon = Some(config);
            crate::catalog::save(layout, record)?;
        }
        return Ok(());
    }
    let used: Vec<u32> = crate::catalog::list(layout)?
        .into_iter()
        .filter(|other| other.id != record.id)
        .filter_map(|other| other.rcon.map(|rcon| rcon.port))
        .chain(crate::catalog::used_ports(layout, &record.id))
        .collect();
    let mut port = FIRST_PORT;
    while used.contains(&port) {
        port += 1;
    }
    let config = RconConfig { port, password: random_password() };
    crate::properties::write_entries(
        &record.root,
        &[
            ("enable-rcon".into(), "true".into()),
            ("rcon.port".into(), port.to_string()),
            ("rcon.password".into(), config.password.clone()),
            ("broadcast-rcon-to-ops".into(), "false".into()),
        ],
    )?;
    record.rcon = Some(config);
    crate::catalog::save(layout, record)
}

pub async fn execute_for(id: &str, command: &str) -> PanelResult<String> {
    let record = crate::catalog::get(&Layout::app(), id)?;
    let config = record.rcon.ok_or_else(|| PanelError::invalid("RCON non configurato per questo server"))?;
    execute(&config, command).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_packets() {
        let packet = encode(7, COMMAND, "list");
        assert_eq!(i32::from_le_bytes(packet[0..4].try_into().unwrap()), 14);
        assert_eq!(&packet[12..16], b"list");
        assert_eq!(&packet[16..], &[0, 0]);
    }

    #[test]
    fn parses_player_lists() {
        assert_eq!(parse_list("There are 2 of a max of 20 players online: Steve, Alex_01"), vec!["Steve", "Alex_01"]);
        assert!(parse_list("There are 0 of a max of 20 players online: ").is_empty());
        assert_eq!(parse_list("§6There are §c1§6 out of maximum §c20§6 players online.\n§6default§r: §fSteve"), vec!["Steve"]);
    }

    #[tokio::test]
    async fn talks_to_a_fake_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port() as u32;
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (id, kind, body) = read_packet_raw(&mut socket).await;
            assert_eq!((kind, body.as_str()), (AUTH, "secret"));
            socket.write_all(&encode(id, 2, "")).await.unwrap();
            let (id, _, body) = read_packet_raw(&mut socket).await;
            assert_eq!(body, "list");
            socket.write_all(&encode(id, 0, "There are 1 of a max of 20 players online: Steve")).await.unwrap();
        });
        let response = execute(&RconConfig { port, password: "secret".into() }, "list").await.unwrap();
        assert_eq!(parse_list(&response), vec!["Steve"]);
    }

    async fn read_packet_raw(socket: &mut TcpStream) -> (i32, i32, String) {
        let mut length = [0u8; 4];
        socket.read_exact(&mut length).await.unwrap();
        let mut payload = vec![0u8; i32::from_le_bytes(length) as usize];
        socket.read_exact(&mut payload).await.unwrap();
        (
            i32::from_le_bytes(payload[0..4].try_into().unwrap()),
            i32::from_le_bytes(payload[4..8].try_into().unwrap()),
            String::from_utf8_lossy(&payload[8..payload.len() - 2]).to_string(),
        )
    }
}
