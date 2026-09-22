use crate::Progress;
use std::path::Path;
use std::sync::mpsc::Sender;
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
    progress: &Sender<Progress>,
) -> Result<(), String> {
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
            let _ = progress.send(Progress {
                stage: stage.to_string(),
                message,
                fraction,
            });
        }
    }
    Ok(())
}

pub fn extract_zip(archive: &Path, destination: &Path) -> Result<(), String> {
    crate::paths::ensure_dir(destination)?;
    let file = std::fs::File::open(archive).map_err(|error| format!("Archivio illeggibile: {error}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| format!("Zip non valido: {error}"))?;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| format!("Voce zip illeggibile: {error}"))?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let out = destination.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|error| error.to_string())?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut output = std::fs::File::create(&out).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
