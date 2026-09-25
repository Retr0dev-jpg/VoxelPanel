// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

use crate::api::types::{LogFileInfo, LogFileKind};
use crate::{PanelError, PanelResult};

/// Only the end of huge logs is returned; that is where problems are.
pub const MAX_LOG_BYTES: usize = 4 * 1024 * 1024;

pub fn list(root: &Path) -> Vec<LogFileInfo> {
    let mut files = Vec::new();
    let mut push = |directory: &str, kind_of: &dyn Fn(&str) -> Option<LogFileKind>| {
        let Ok(entries) = std::fs::read_dir(root.join(directory)) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(kind) = kind_of(&name) else { continue };
            let Ok(meta) = entry.metadata() else { continue };
            files.push(LogFileInfo {
                relative: format!("{directory}/{name}"),
                size_bytes: meta.len() as i64,
                modified_ms: meta
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|value| value.as_millis() as i64)
                    .unwrap_or(0),
                name,
                kind,
            });
        }
    };
    push("logs", &|name| match name {
        "latest.log" => Some(LogFileKind::Latest),
        name if name.ends_with(".log.gz") => Some(LogFileKind::Archive),
        name if name.ends_with(".log") => Some(LogFileKind::Archive),
        _ => None,
    });
    push("crash-reports", &|name| name.ends_with(".txt").then_some(LogFileKind::Crash));
    files.sort_by(|left, right| (left.kind != LogFileKind::Latest).cmp(&(right.kind != LogFileKind::Latest)).then(right.modified_ms.cmp(&left.modified_ms)));
    files
}

/// Reads a log (decompressing `.gz`), keeping only the last [`MAX_LOG_BYTES`].
pub fn read(root: &Path, relative: &str) -> PanelResult<String> {
    if !(relative.starts_with("logs/") || relative.starts_with("crash-reports/")) {
        return Err(PanelError::invalid("Non è un file di log"));
    }
    let path = crate::server_files::resolve(root, relative)?;
    let file = std::fs::File::open(&path)?;
    let mut bytes = Vec::new();
    if relative.ends_with(".gz") {
        flate2::read::GzDecoder::new(file).read_to_end(&mut bytes)?;
    } else {
        std::io::BufReader::new(file).read_to_end(&mut bytes)?;
    }
    if bytes.len() > MAX_LOG_BYTES {
        let start = bytes.len() - MAX_LOG_BYTES;
        let start = bytes[start..].iter().position(|byte| *byte == b'\n').map(|offset| start + offset + 1).unwrap_or(start);
        bytes.drain(..start);
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn lists_and_reads_logs() {
        let root = std::env::temp_dir().join(format!("voxel-logs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("logs")).unwrap();
        std::fs::create_dir_all(root.join("crash-reports")).unwrap();
        std::fs::write(root.join("logs/latest.log"), "ciao\n").unwrap();
        std::fs::write(root.join("crash-reports/crash-2026.txt"), "boom\n").unwrap();
        let mut encoder = flate2::write::GzEncoder::new(std::fs::File::create(root.join("logs/2026-09-24-1.log.gz")).unwrap(), flate2::Compression::fast());
        encoder.write_all(b"archiviato\n").unwrap();
        encoder.finish().unwrap();
        let files = list(&root);
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].kind, LogFileKind::Latest);
        assert_eq!(read(&root, "logs/2026-09-24-1.log.gz").unwrap(), "archiviato\n");
        assert!(read(&root, "server.properties").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
