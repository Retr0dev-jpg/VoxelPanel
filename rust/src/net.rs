// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use crate::Progress;
use std::path::Path;
use std::time::Duration;

pub fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("VoxelPanel/1.0")
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(600))
        .build()
        .expect("client http")
}

pub async fn download(
    url: &str,
    destination: &Path,
    stage: &str,
    progress: &crate::ProgressTx,
) -> crate::PanelResult<()> {
    if let Some(parent) = destination.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let client = http();
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Download fallito: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Download fallito: {error}"))?;
    let total = response.content_length();
    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|error| format!("Impossibile creare il file: {error}"))?;
    let mut downloaded = 0u64;
    let mut last_percent = None;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Download interrotto: {error}"))?
    {
        downloaded += chunk.len() as u64;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|error| format!("Scrittura file fallita: {error}"))?;
        let fraction = total.map(|total| downloaded as f64 / total.max(1) as f64);
        let percent = fraction.map(|value| (value * 100.0) as u8);
        if percent != last_percent {
            last_percent = percent;
            let message = match total {
                Some(total) => format!(
                    "{} / {} MB",
                    downloaded / (1024 * 1024),
                    total / (1024 * 1024)
                ),
                None => format!("{} MB", downloaded / (1024 * 1024)),
            };
            progress.send(Progress {
                stage: stage.to_string(),
                message,
                fraction,
            });
        }
    }
    Ok(())
}

/// Extracts `.zip` or `.tar.gz` archives (Adoptium ships zip on Windows, tar.gz elsewhere).
pub fn extract_archive(archive: &Path, destination: &Path) -> crate::PanelResult<()> {
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz(archive, destination)
    } else {
        extract_zip(archive, destination)
    }
}

pub fn extract_tar_gz(archive: &Path, destination: &Path) -> crate::PanelResult<()> {
    crate::paths::ensure_dir(destination)?;
    let file = std::fs::File::open(archive)
        .map_err(|error| crate::PanelError::io(format!("Archivio illeggibile: {error}")))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    tar.set_preserve_permissions(true);
    // `unpack` refuses entries that escape the destination folder.
    tar.unpack(destination)
        .map_err(|error| crate::PanelError::io(format!("Estrazione non riuscita: {error}")))
}

pub fn extract_zip(archive: &Path, destination: &Path) -> crate::PanelResult<()> {
    crate::paths::ensure_dir(destination)?;
    let file = std::fs::File::open(archive)
        .map_err(|error| crate::PanelError::io(format!("Archivio illeggibile: {error}")))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| crate::PanelError::invalid(format!("Zip non valido: {error}")))?;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| crate::PanelError::invalid(format!("Voce zip illeggibile: {error}")))?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let out = destination.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut output)?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_tar_gz_archives() {
        let root = std::env::temp_dir().join(format!("voxel-tar-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let archive = root.join("jdk.tar.gz");
        {
            let file = std::fs::File::create(&archive).unwrap();
            let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            let body = b"JAVA_VERSION=\"21.0.2\"\n";
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, "jdk-21/release", &body[..]).unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        let out = root.join("out");
        extract_archive(&archive, &out).unwrap();
        assert!(out.join("jdk-21").join("release").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
