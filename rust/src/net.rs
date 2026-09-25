// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::path::Path;
use std::time::Duration;

fn client_cache() -> &'static std::sync::Mutex<Option<reqwest::Client>> {
    static CLIENT: std::sync::OnceLock<std::sync::Mutex<Option<reqwest::Client>>> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| std::sync::Mutex::new(None))
}

/// Shared client honouring the proxy and timeout from the launcher settings.
pub fn http() -> reqwest::Client {
    let mut cached = client_cache().lock().unwrap_or_else(|error| error.into_inner());
    if let Some(client) = cached.as_ref() {
        return client.clone();
    }
    let network = crate::launcher_settings::current().network;
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("VoxelPanel/", env!("CARGO_PKG_VERSION"), " (+https://github.com/Retr0dev-jpg/VoxelPanel)"))
        .connect_timeout(Duration::from_secs(u64::from(network.timeout_secs)))
        .read_timeout(Duration::from_secs(u64::from(network.timeout_secs) * 4));
    if let Ok(proxy) = reqwest::Proxy::all(network.proxy.trim()) {
        builder = builder.proxy(proxy);
    }
    let client = builder.build().unwrap_or_default();
    *cached = Some(client.clone());
    client
}

/// Called when the network settings change.
pub fn reset_client() {
    *client_cache().lock().unwrap_or_else(|error| error.into_inner()) = None;
}

#[derive(Debug, Clone)]
pub enum Checksum {
    Sha512(String),
    Sha256(String),
    Sha1(String),
    Md5(String),
}

impl Checksum {
    fn hasher(&self) -> Box<dyn sha2::digest::DynDigest + Send> {
        match self {
            Checksum::Sha512(_) => Box::new(sha2::Sha512::default()),
            Checksum::Sha256(_) => Box::new(sha2::Sha256::default()),
            Checksum::Sha1(_) => Box::new(sha1::Sha1::default()),
            Checksum::Md5(_) => Box::new(md5::Md5::default()),
        }
    }

    fn expected(&self) -> &str {
        match self {
            Checksum::Sha512(value) | Checksum::Sha256(value) | Checksum::Sha1(value) | Checksum::Md5(value) => value,
        }
    }
}

pub async fn download(url: &str, destination: &Path, stage: &str, progress: &crate::ProgressTx) -> crate::PanelResult<()> {
    download_checked(url, destination, stage, progress, None).await
}

/// Streams `url` into `destination` through a `.part` file, verifying the checksum when given,
/// so an interrupted or corrupted download never leaves a broken jar behind.
pub async fn download_checked(
    url: &str,
    destination: &Path,
    stage: &str,
    progress: &crate::ProgressTx,
    checksum: Option<Checksum>,
) -> crate::PanelResult<()> {
    if let Some(parent) = destination.parent() {
        crate::paths::ensure_dir(parent)?;
    }
    let mut response = http()
        .get(url)
        .send()
        .await
        .map_err(|error| crate::PanelError::network(format!("Download fallito: {error}")))?
        .error_for_status()
        .map_err(|error| crate::PanelError::network(format!("Download fallito: {error}")))?;
    let total = response.content_length();
    let temp = destination.with_extension("part");
    let mut file = tokio::fs::File::create(&temp)
        .await
        .map_err(|error| crate::PanelError::io(format!("Impossibile creare il file: {error}")))?;
    let mut hasher = checksum.as_ref().map(Checksum::hasher);
    let mut downloaded = 0u64;
    let mut last_percent = None;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| crate::PanelError::network(format!("Download interrotto: {error}")))?
    {
        downloaded += chunk.len() as u64;
        if let Some(hasher) = hasher.as_mut() {
            hasher.update(&chunk);
        }
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|error| crate::PanelError::io(format!("Scrittura file fallita: {error}")))?;
        let fraction = total.map(|total| downloaded as f64 / total.max(1) as f64);
        let percent = fraction.map(|value| (value * 100.0) as u8);
        if percent != last_percent {
            last_percent = percent;
            let message = match total {
                Some(total) => format!("{} / {} MB", downloaded / (1024 * 1024), total / (1024 * 1024)),
                None => format!("{} MB", downloaded / (1024 * 1024)),
            };
            progress.emit(stage, message, fraction);
        }
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    drop(file);
    if let (Some(checksum), Some(hasher)) = (checksum.as_ref(), hasher) {
        let actual: String = hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect();
        if !actual.eq_ignore_ascii_case(checksum.expected()) {
            let _ = std::fs::remove_file(&temp);
            return Err(crate::PanelError::invalid(format!("Checksum non valido per {url}: il file scaricato è corrotto.")));
        }
    }
    std::fs::rename(&temp, destination)?;
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
