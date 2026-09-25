// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::sync::OnceLock;

use crate::{PanelError, PanelResult};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn init() {
    let _ = global_job();
}

pub fn configure_command(command: &mut std::process::Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

/// Puts the child in a job object that is closed (and its processes killed) with VoxelPanel.
pub fn attach_child(child: &std::process::Child) -> bool {
    let Some(job) = global_job() else {
        return false;
    };
    unsafe {
        windows::Win32::System::JobObjects::AssignProcessToJobObject(
            windows::Win32::Foundation::HANDLE(job.as_raw_handle()),
            windows::Win32::Foundation::HANDLE(child.as_raw_handle()),
        )
        .is_ok()
    }
}

pub async fn kill_tree(pid: u32) {
    let mut command = tokio::process::Command::new("taskkill");
    command.args(["/PID", &pid.to_string(), "/T", "/F"]);
    command.creation_flags(CREATE_NO_WINDOW);
    let _ = command.status().await;
}

fn global_job() -> Option<&'static OwnedHandle> {
    use windows::Win32::System::JobObjects::{
        CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    static JOB: OnceLock<Option<OwnedHandle>> = OnceLock::new();
    JOB.get_or_init(|| {
        let raw = unsafe { CreateJobObjectW(None, None) }.ok()?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured.is_err() {
            return None;
        }
        Some(unsafe { OwnedHandle::from_raw_handle(raw.0) })
    })
    .as_ref()
}

/// explorer.exe parses its own command line: a quoted path with spaces makes it open
/// Documents. ShellExecuteW takes the path as a wide string and skips that parser.
pub fn open_directory(path: &Path) -> PanelResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let mut file: Vec<u16> = path.as_os_str().encode_wide().collect();
    file.push(0);
    let operation: Vec<u16> = "explore".encode_utf16().chain(std::iter::once(0)).collect();
    let code = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // Values <= 32 are Win32 error codes, not an instance handle.
    if (code.0 as isize) <= 32 {
        return Err(PanelError::io(format!(
            "Impossibile aprire la cartella (codice {})",
            code.0 as isize
        )));
    }
    Ok(())
}
