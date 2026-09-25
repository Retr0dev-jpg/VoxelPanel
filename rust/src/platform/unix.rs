// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::{PanelError, PanelResult};

pub fn init() {}

/// Each server gets its own process group so the whole tree can be signalled.
pub fn configure_command(command: &mut std::process::Command) {
    command.process_group(0);
    #[cfg(target_os = "linux")]
    unsafe {
        command.pre_exec(|| {
            // The server dies with VoxelPanel even if VoxelPanel crashes.
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

pub fn attach_child(_child: &std::process::Child) -> bool {
    true
}

fn alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

pub async fn kill_tree(pid: u32) {
    let pid = pid as i32;
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && alive(pid) {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    if alive(pid) {
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

pub fn open_directory(path: &Path) -> PanelResult<()> {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    std::process::Command::new(opener)
        .arg(path)
        .spawn()
        .map_err(|error| PanelError::io(format!("Impossibile aprire la cartella: {error}")))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn autostart_file() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|home| home.join("Library/LaunchAgents/dev.voxelpanel.VoxelPanel.plist"))
}

#[cfg(not(target_os = "macos"))]
fn autostart_file() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|config| config.join("autostart/voxelpanel.desktop"))
}

fn autostart_content(exe: &Path) -> String {
    let exe = exe.display();
    if cfg!(target_os = "macos") {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key>\n  <string>dev.voxelpanel.VoxelPanel</string>\n  <key>ProgramArguments</key>\n  <array>\n    <string>{exe}</string>\n  </array>\n  <key>RunAtLoad</key>\n  <true/>\n</dict>\n</plist>\n"
        )
    } else {
        format!("[Desktop Entry]\nType=Application\nName=VoxelPanel\nExec=\"{exe}\"\nX-GNOME-Autostart-enabled=true\n")
    }
}

/// XDG autostart entry on Linux, LaunchAgent on macOS.
pub fn set_launch_at_startup(enabled: bool) -> PanelResult<()> {
    let file = autostart_file().ok_or_else(|| PanelError::not_found("Cartella di configurazione non trovata"))?;
    if !enabled {
        if file.exists() {
            std::fs::remove_file(&file)?;
        }
        return Ok(());
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&file, autostart_content(&std::env::current_exe()?))?;
    Ok(())
}
