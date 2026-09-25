// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::{PanelError, PanelResult};

fn file_for(key: &str) -> PathBuf {
    let safe: String = key
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' { ch } else { '_' })
        .collect();
    crate::paths::Layout::app().cache().join("api").join(format!("{safe}.json"))
}

fn age(path: &PathBuf) -> Option<Duration> {
    let modified = std::fs::metadata(path).and_then(|meta| meta.modified()).ok()?;
    SystemTime::now().duration_since(modified).ok()
}

/// GET `url` as JSON, reusing a copy on disk younger than `ttl`. When the network fails a
/// stale copy is returned instead, so the wizard keeps working offline.
pub async fn json(key: &str, url: &str, ttl: Duration) -> PanelResult<Value> {
    let path = file_for(key);
    if age(&path).is_some_and(|age| age < ttl) {
        if let Some(value) = read(&path) {
            return Ok(value);
        }
    }
    match fetch(url).await {
        Ok(value) => {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&path, value.to_string());
            Ok(value)
        }
        Err(error) => {
            if let Some(value) = read(&path) {
                tracing::warn!("{url} non raggiungibile, uso la copia in cache: {}", error.message);
                return Ok(value);
            }
            Err(error)
        }
    }
}

fn read(path: &PathBuf) -> Option<Value> {
    std::fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok())
}

pub async fn fetch(url: &str) -> PanelResult<Value> {
    let response = crate::net::http()
        .get(url)
        .send()
        .await
        .map_err(|error| PanelError::network(format!("{url}: {error}")))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(PanelError::not_found(format!("Risorsa non trovata: {url}")));
    }
    let response = response
        .error_for_status()
        .map_err(|error| PanelError::network(format!("{url}: {error}")))?;
    Ok(response.json().await?)
}

pub const SHORT: Duration = Duration::from_secs(10 * 60);
pub const LONG: Duration = Duration::from_secs(6 * 60 * 60);
