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
