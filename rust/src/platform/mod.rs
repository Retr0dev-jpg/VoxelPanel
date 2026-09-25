// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Everything that differs between Windows, Linux and macOS lives here.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{attach_child, configure_command, init, kill_tree, open_directory, set_launch_at_startup};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{attach_child, configure_command, init, kill_tree, open_directory, set_launch_at_startup};

pub const JAVA_BINARY: &str = if cfg!(windows) { "java.exe" } else { "java" };

pub fn java_executable(home: &Path) -> PathBuf {
    home.join("bin").join(JAVA_BINARY)
}

/// Returns the JDK home inside `dir`; macOS archives nest it in `Contents/Home`.
pub fn resolve_java_home(dir: &Path) -> Option<PathBuf> {
    [dir.to_path_buf(), dir.join("Contents").join("Home")]
        .into_iter()
        .find(|candidate| java_executable(candidate).is_file())
}

pub fn app_data_root() -> PathBuf {
    if let Some(custom) = std::env::var_os("VOXELPANEL_DATA_DIR") {
        return PathBuf::from(custom);
    }
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VoxelPanel")
}

/// Compares paths the way the current file system does (case-insensitive on Windows).
pub fn same_path(left: &Path, right: &Path) -> bool {
    normalize(left) == normalize(right)
}

pub fn normalize(path: &Path) -> String {
    let text = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

/// Paths read from scripts written on another OS use the wrong separator.
pub fn native_path(text: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(text.replace('/', "\\"))
    } else {
        PathBuf::from(text.replace('\\', "/"))
    }
}

pub fn adoptium_os() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    }
}

pub fn adoptium_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    }
}

pub fn start_script_name() -> &'static str {
    if cfg!(windows) {
        "start.bat"
    } else {
        "start.sh"
    }
}

pub struct ScriptSpec<'a> {
    /// Program and arguments, already relative to the server folder when possible.
    pub program: &'a str,
    pub args: &'a [String],
}

pub fn render_start_script(spec: &ScriptSpec) -> Result<String, String> {
    if cfg!(windows) {
        let mut line = quote_bat(spec.program)?;
        for arg in spec.args {
            line.push(' ');
            line.push_str(&quote_bat(arg)?);
        }
        Ok(format!(
            "@echo off\r\ncd /d \"%~dp0\"\r\nif not exist \"eula.txt\" (\r\n    (echo eula=true)> \"eula.txt\"\r\n)\r\n{line}\r\nset \"SERVER_EXIT=%ERRORLEVEL%\"\r\nif not \"%SERVER_EXIT%\"==\"0\" (\r\n    echo Server terminato con errore: %SERVER_EXIT%\r\n    pause\r\n    exit /b %SERVER_EXIT%\r\n)\r\nexit /b 0\r\n"
        ))
    } else {
        let mut line = quote_sh(spec.program)?;
        for arg in spec.args {
            line.push(' ');
            line.push_str(&quote_sh(arg)?);
        }
        Ok(format!(
            "#!/usr/bin/env sh\ncd \"$(dirname \"$0\")\" || exit 1\n[ -f eula.txt ] || echo \"eula=true\" > eula.txt\nexec {line}\n"
        ))
    }
}

fn quote_bat(value: &str) -> Result<String, String> {
    if value.contains(['"', '\n', '\r', '%']) {
        return Err(format!("Valore non valido per start.bat: {value}"));
    }
    if value.contains([' ', '&', '(', ')', '^', '|', '<', '>']) || value.contains('\\') {
        Ok(format!("\"{value}\""))
    } else {
        Ok(value.to_string())
    }
}

fn quote_sh(value: &str) -> Result<String, String> {
    if value.contains(['\n', '\r', '\'']) {
        return Err(format!("Valore non valido per start.sh: {value}"));
    }
    if value.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_=.:/+@,".contains(ch)) {
        Ok(value.to_string())
    } else {
        Ok(format!("'{value}'"))
    }
}

pub fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

type SpawnReply = mpsc::Sender<std::io::Result<std::process::Child>>;

/// Children are spawned from one long-lived thread: on Linux the parent-death signal
/// is tied to the spawning thread, and short-lived runtime threads would kill servers.
pub fn spawn(command: std::process::Command) -> std::io::Result<std::process::Child> {
    static SPAWNER: OnceLock<Mutex<mpsc::Sender<(std::process::Command, SpawnReply)>>> = OnceLock::new();
    let sender = SPAWNER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<(std::process::Command, SpawnReply)>();
        std::thread::Builder::new()
            .name("voxel-spawner".into())
            .spawn(move || {
                for (mut command, reply) in rx {
                    let _ = reply.send(command.spawn());
                }
            })
            .expect("spawner thread");
        Mutex::new(tx)
    });
    let (reply, response) = mpsc::channel();
    sender
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .send((command, reply))
        .map_err(|_| std::io::Error::other("spawner non disponibile"))?;
    response
        .recv()
        .map_err(|_| std::io::Error::other("spawner non disponibile"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_path_uses_bin() {
        let home = PathBuf::from("runtime").join("jdk-21.0.2");
        assert_eq!(java_executable(&home), home.join("bin").join(JAVA_BINARY));
    }

    #[test]
    fn finds_nested_macos_home() {
        let root = std::env::temp_dir().join(format!("voxel-jdk-{}", uuid::Uuid::new_v4()));
        let nested = root.join("Contents").join("Home");
        std::fs::create_dir_all(nested.join("bin")).unwrap();
        std::fs::write(java_executable(&nested), b"").unwrap();
        assert_eq!(resolve_java_home(&root), Some(nested));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn renders_a_start_script() {
        let args = vec!["-Xmx4G".to_string(), "-jar".to_string(), "paper 1.jar".to_string(), "nogui".to_string()];
        let script = render_start_script(&ScriptSpec { program: "java", args: &args }).unwrap();
        assert!(script.contains("-Xmx4G -jar"));
        if cfg!(windows) {
            assert!(script.contains("\"paper 1.jar\""));
        } else {
            assert!(script.starts_with("#!/usr/bin/env sh"));
            assert!(script.contains("'paper 1.jar'"));
        }
    }
}
