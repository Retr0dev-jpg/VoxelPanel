// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use crate::content::{self, Target};
use crate::frb_generated::StreamSink;
use crate::paths::Layout;
use crate::{process, PanelResult};

use super::progress::report;
use super::types::*;

fn target(id: &str, kind: AddonKind) -> PanelResult<(crate::ServerRecord, Target)> {
    let record = crate::catalog::get(&Layout::app(), id)?;
    let target = Target::of(&record, kind);
    Ok((record, target))
}

/// Catalogues that offer plugins or mods for this server.
pub async fn content_sources(id: String, kind: AddonKind) -> PanelResult<Vec<ContentSourceKind>> {
    Ok(content::available(&target(&id, kind)?.1))
}

pub async fn search_content(id: String, kind: AddonKind, source: ContentSourceKind, query: String, page: u32) -> PanelResult<ContentPage> {
    let (_, target) = target(&id, kind)?;
    content::get(source).search(&target, query.trim(), page).await
}

pub async fn content_versions(id: String, kind: AddonKind, source: ContentSourceKind, project_id: String) -> PanelResult<Vec<ContentVersion>> {
    let (_, target) = target(&id, kind)?;
    content::get(source).versions(&target, &project_id).await
}

/// Installs a plugin or mod with its required dependencies. `version_id` empty: newest compatible.
pub async fn install_content(id: String, kind: AddonKind, source: ContentSourceKind, project_id: String, version_id: String, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let (record, _) = target(&id, kind)?;
    report(
        sink,
        "Contenuti",
        move |files: &Vec<String>| {
            let message = if files.is_empty() { "Nessun file nuovo da installare.".to_string() } else { format!("Installati: {}", files.join(", ")) };
            (message, Some(id))
        },
        |tx| async move { content::install(&record, kind, source, &project_id, &version_id, &tx).await },
    )
    .await
}

pub async fn check_addon_updates(id: String, kind: AddonKind) -> PanelResult<Vec<AddonUpdate>> {
    let (record, _) = target(&id, kind)?;
    content::check_updates(&record, kind).await
}

pub async fn update_addons(id: String, kind: AddonKind, updates: Vec<AddonUpdate>, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    process::ensure_stopped(&id)?;
    let (record, _) = target(&id, kind)?;
    report(
        sink,
        "Aggiornamenti",
        move |count: &u32| (format!("{count} file aggiornati."), Some(id)),
        |tx| async move { content::apply_updates(&record, kind, &updates, &tx).await },
    )
    .await
}

#[flutter_rust_bridge::frb(sync)]
pub fn modpack_sources() -> Vec<ContentSourceKind> {
    let mut sources = vec![ContentSourceKind::Modrinth];
    if content::curseforge::api_key().is_some() {
        sources.push(ContentSourceKind::CurseForge);
    }
    sources
}

pub async fn search_modpacks(source: ContentSourceKind, query: String, page: u32) -> PanelResult<ContentPage> {
    match source {
        ContentSourceKind::CurseForge => content::curseforge::search_class(content::curseforge::CLASS_MODPACKS, None, query.trim(), page).await,
        _ => content::modrinth::search_type("modpack", Vec::new(), query.trim(), page).await,
    }
}

pub async fn modpack_versions(source: ContentSourceKind, project_id: String) -> PanelResult<Vec<ContentVersion>> {
    let mut versions: Vec<ContentVersion> = match source {
        ContentSourceKind::CurseForge => {
            let value = content::curseforge::get(&format!("/mods/{project_id}/files"), &[("pageSize", "50".into())]).await?;
            value
                .get("data")
                .and_then(serde_json::Value::as_array)
                .map(|files| files.iter().map(|file| content::curseforge::version_from_file(None, file)).collect())
                .unwrap_or_default()
        }
        _ => content::modrinth::project_versions(&project_id).await?.iter().map(|version| content::modrinth::version_from_json(None, version)).collect(),
    };
    content::sort_versions(&mut versions);
    Ok(versions)
}

/// Creates a server from a modpack: installs the right loader, downloads the server-side files
/// and copies the overrides.
pub async fn create_from_modpack(request: ModpackRequest, sink: StreamSink<ProgressEvent>) -> PanelResult<()> {
    report(
        sink,
        "Fatto",
        |id: &String| ("Server creato dal modpack.".into(), Some(id.clone())),
        |tx| async move { content::modpack::create(&Layout::app(), request, &tx).await },
    )
    .await
}
