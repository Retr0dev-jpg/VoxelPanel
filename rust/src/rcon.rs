// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Minimal Source RCON client, used to query servers without writing to their console.
//!
//! One authenticated connection is kept per server: servers log every RCON connection, so opening
//! one per command floods the console.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
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

struct Session {
    config: RconConfig,
    stream: TcpStream,
    last_id: i32,
}

impl Session {
    async fn open(config: &RconConfig) -> PanelResult<Self> {
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
        Ok(Self { config: config.clone(), stream, last_id: 1 })
    }

    async fn run(&mut self, command: &str) -> PanelResult<String> {
        self.last_id = if self.last_id >= i32::MAX - 1 { 2 } else { self.last_id + 1 };
        let request = self.last_id;
        self.stream.write_all(&encode(request, COMMAND, command)).await?;
        loop {
            // Leftover fragments of earlier long responses carry older ids.
            let (id, _, body) = read_packet(&mut self.stream).await?;
            if id == request {
                return Ok(body);
            }
        }
    }
}

type Slot = Arc<tokio::sync::Mutex<Option<Session>>>;

fn sessions() -> &'static Mutex<HashMap<String, Slot>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, Slot>>> = OnceLock::new();
    SESSIONS.get_or_init(Default::default)
}

/// Drops the connection kept for a server, e.g. when its process exits.
pub fn disconnect(key: &str) {
    sessions().lock().unwrap_or_else(PoisonError::into_inner).remove(key);
}

async fn run_pooled(session: &mut Option<Session>, config: &RconConfig, command: &str) -> PanelResult<String> {
    if let Some(open) = session.as_mut().filter(|open| open.config == *config) {
        if let Ok(body) = open.run(command).await {
            return Ok(body);
        }
    }
    // Stale connection (server restarted, settings changed): reconnect once.
    *session = None;
    let mut fresh = Session::open(config).await?;
    let body = fresh.run(command).await?;
    *session = Some(fresh);
    Ok(body)
}

/// Runs one command on the connection kept for `key`, opening it when needed.
pub async fn execute_pooled(key: &str, config: &RconConfig, command: &str) -> PanelResult<String> {
    let slot = sessions().lock().unwrap_or_else(PoisonError::into_inner).entry(key.to_string()).or_default().clone();
    let mut session = slot.lock().await;
    let outcome = tokio::time::timeout(TIMEOUT, run_pooled(&mut session, config, command)).await;
    match outcome {
        Ok(Ok(body)) => Ok(body),
        // A half-read response would desynchronise the next command.
        Ok(Err(error)) => {
            *session = None;
            Err(error)
        }
        Err(_) => {
            *session = None;
            Err(PanelError::network("RCON non ha risposto in tempo"))
        }
    }
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
    execute_pooled(id, &config, command).await
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

    /// Accepts `connections` clients; each authenticates and gets `commands` answers echoing the
    /// command, preceded by a stray packet with an old id. Returns the port and an accept counter.
    async fn fake_server(connections: usize, commands: usize) -> (u32, Arc<std::sync::atomic::AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port() as u32;
        let accepted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = accepted.clone();
        tokio::spawn(async move {
            for _ in 0..connections {
                let (mut socket, _) = listener.accept().await.unwrap();
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let (id, kind, password) = read_packet_raw(&mut socket).await;
                assert_eq!(kind, AUTH);
                socket.write_all(&encode(if password == "secret" { id } else { -1 }, 2, "")).await.unwrap();
                for _ in 0..commands {
                    let (id, _, body) = read_packet_raw(&mut socket).await;
                    socket.write_all(&encode(id - 1, 0, "stale fragment")).await.unwrap();
                    socket.write_all(&encode(id, 0, &format!("ok {body}"))).await.unwrap();
                }
            }
        });
        (port, accepted)
    }

    #[tokio::test]
    async fn reuses_one_connection_for_many_commands() {
        let (port, accepted) = fake_server(1, 3).await;
        let config = RconConfig { port, password: "secret".into() };
        for command in ["tps", "mspt", "list"] {
            assert_eq!(execute_pooled("reuse", &config, command).await.unwrap(), format!("ok {command}"));
        }
        assert_eq!(accepted.load(std::sync::atomic::Ordering::SeqCst), 1);
        disconnect("reuse");
    }

    #[tokio::test]
    async fn reconnects_when_the_server_closes_the_connection() {
        let (port, accepted) = fake_server(2, 1).await;
        let config = RconConfig { port, password: "secret".into() };
        assert_eq!(execute_pooled("reconnect", &config, "tps").await.unwrap(), "ok tps");
        assert_eq!(execute_pooled("reconnect", &config, "list").await.unwrap(), "ok list");
        assert_eq!(accepted.load(std::sync::atomic::Ordering::SeqCst), 2);
        disconnect("reconnect");
    }

    #[tokio::test]
    async fn reports_a_wrong_password() {
        let (port, _) = fake_server(2, 0).await;
        let error = execute_pooled("password", &RconConfig { port, password: "wrong".into() }, "list").await.unwrap_err();
        assert_eq!(error.message, "Password RCON rifiutata");
        disconnect("password");
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
