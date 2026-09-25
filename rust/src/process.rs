// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::broadcast;

use sysinfo::{Pid, ProcessesToUpdate, System};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
}

pub struct LogHub {
    lines: VecDeque<String>,
    tx: broadcast::Sender<String>,
}

impl LogHub {
    pub fn new() -> Arc<Mutex<Self>> {
        let (tx, _) = broadcast::channel(512);
        Arc::new(Mutex::new(Self {
            lines: VecDeque::new(),
            tx,
        }))
    }

    pub fn push(this: &Mutex<Self>, line: String) {
        let mut hub = this.lock().unwrap_or_else(|error| error.into_inner());
        if hub.lines.len() >= 2000 {
            hub.lines.pop_front();
        }
        hub.lines.push_back(line.clone());
        let _ = hub.tx.send(line);
    }

    pub fn snapshot(&self) -> (Vec<String>, broadcast::Receiver<String>) {
        (self.lines.iter().cloned().collect(), self.tx.subscribe())
    }
}

struct ProcSlot {
    stdin: Option<ChildStdin>,
    pid: u32,
}

struct Supervisor {
    procs: HashMap<String, ProcSlot>,
    status: HashMap<String, RunStatus>,
    logs: HashMap<String, Arc<Mutex<LogHub>>>,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self {
            procs: HashMap::new(),
            status: HashMap::new(),
            logs: HashMap::new(),
        }
    }
}

fn supervisor() -> &'static Mutex<Supervisor> {
    static SUPERVISOR: OnceLock<Mutex<Supervisor>> = OnceLock::new();
    SUPERVISOR.get_or_init(|| Mutex::new(Supervisor::default()))
}

fn lock() -> std::sync::MutexGuard<'static, Supervisor> {
    supervisor().lock().unwrap_or_else(|error| error.into_inner())
}

pub fn init_job() {
    let _ = global_job();
}

pub fn status_of(id: &str) -> RunStatus {
    lock().status.get(id).copied().unwrap_or(RunStatus::Stopped)
}

pub fn pid_of(id: &str) -> Option<u32> {
    lock().procs.get(id).map(|slot| slot.pid)
}

pub fn any_running() -> bool {
    lock().procs.values().next().is_some()
}

pub fn ensure_logs(id: &str) -> Arc<Mutex<LogHub>> {
    let mut state = lock();
    state
        .logs
        .entry(id.to_string())
        .or_insert_with(LogHub::new)
        .clone()
}

pub fn console_history(id: &str) -> Vec<String> {
    let logs = ensure_logs(id);
    let guard = logs.lock().unwrap_or_else(|error| error.into_inner());
    guard.snapshot().0
}

pub fn online_players(id: &str) -> u32 {
    if !matches!(status_of(id), RunStatus::Running | RunStatus::Starting) {
        return 0;
    }
    let mut online: i32 = 0;
    for line in console_history(id) {
        let plain = strip_ansi(&line);
        if plain.contains(" joined the game") {
            online += 1;
        } else if plain.contains(" left the game") {
            online = (online - 1).max(0);
        }
    }
    online as u32
}

fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(char) = chars.next() {
        if char == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(char);
    }
    out
}

pub fn subscribe(id: &str) -> (Vec<String>, broadcast::Receiver<String>) {
    let logs = ensure_logs(id);
    let guard = logs.lock().unwrap_or_else(|error| error.into_inner());
    guard.snapshot()
}

pub fn running_ids() -> Vec<String> {
    lock().procs.keys().cloned().collect()
}

pub async fn start(
    id: &str,
    program: &Path,
    args: &[String],
    cwd: &Path,
) -> Result<(), String> {
    if lock().procs.contains_key(id) {
        return Err("Il server è già in esecuzione".into());
    }
    lock().status.insert(id.to_string(), RunStatus::Starting);
    let logs = ensure_logs(id);
    let spawned = match spawn_captured(program, args, cwd, logs.clone(), {
        let id = id.to_string();
        move |pid| mark_exited(&id, pid)
    })
    .await
    {
        Ok(spawned) => spawned,
        Err(error) => {
            lock().status.insert(id.to_string(), RunStatus::Stopped);
            return Err(error);
        }
    };
    let mut state = lock();
    state.procs.insert(
        id.to_string(),
        ProcSlot {
            stdin: Some(spawned.stdin),
            pid: spawned.pid,
        },
    );
    state.status.insert(id.to_string(), RunStatus::Running);
    LogHub::push(&logs, format!("Processo avviato, PID {}.", spawned.pid));
    Ok(())
}

pub fn mark_exited(id: &str, pid: u32) {
    let mut state = lock();
    let matches = state.procs.get(id).map(|slot| slot.pid) == Some(pid);
    if matches {
        state.procs.remove(id);
        state.status.insert(id.to_string(), RunStatus::Stopped);
        if let Some(logs) = state.logs.get(id).cloned() {
            drop(state);
            LogHub::push(&logs, format!("Processo {pid} terminato."));
        }
    }
}

pub async fn send_command(id: &str, command: &str) -> Result<(), String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("Comando vuoto".into());
    }
    if command.contains(['\n', '\r']) {
        return Err("Comando non valido".into());
    }
    let mut stdin = {
        let mut state = lock();
        let slot = state
            .procs
            .get_mut(id)
            .ok_or("Il server non è in esecuzione")?;
        slot.stdin.take().ok_or("Console non disponibile")?
    };
    let write = async {
        stdin.write_all(format!("{command}\n").as_bytes()).await?;
        stdin.flush().await?;
        Ok::<(), std::io::Error>(())
    };
    let result = write.await;
    {
        let mut state = lock();
        if let Some(slot) = state.procs.get_mut(id) {
            slot.stdin = Some(stdin);
        }
    }
    result.map_err(|error| format!("Invio comando fallito: {error}"))?;
    LogHub::push(&ensure_logs(id), format!("> {command}"));
    Ok(())
}

pub async fn stop(id: &str) -> Result<(), String> {
    let pid = {
        let mut state = lock();
        if !state.procs.contains_key(id) {
            return Err("Il server non è in esecuzione".into());
        }
        state.status.insert(id.to_string(), RunStatus::Stopping);
        state.procs.get(id).map(|slot| slot.pid).unwrap_or(0)
    };
    let _ = send_command(id, "stop").await;
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if !lock().procs.contains_key(id) {
            lock().status.insert(id.to_string(), RunStatus::Stopped);
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    kill_pid(pid).await;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !lock().procs.contains_key(id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    if lock().procs.get(id).map(|slot| slot.pid) == Some(pid) {
        mark_exited(id, pid);
    }
    lock().status.insert(id.to_string(), RunStatus::Stopped);
    Ok(())
}

pub async fn shutdown_all() {
    let ids = running_ids();
    let stops = ids.into_iter().map(|id| async move {
        let _ = stop(&id).await;
    });
    futures::future::join_all(stops).await;
}

pub fn process_usage(pid: u32) -> Option<(f64, i64)> {
    let mut system = System::new();
    let pid = Pid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let process = system.process(pid)?;
    Some((f64::from(process.cpu_usage()), process.memory() as i64))
}

struct Spawned {
    pid: u32,
    stdin: ChildStdin,
}

async fn spawn_captured<F>(
    program: &Path,
    args: &[String],
    cwd: &Path,
    logs: Arc<Mutex<LogHub>>,
    on_exit: F,
) -> Result<Spawned, String>
where
    F: FnOnce(u32) + Send + 'static,
{
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(false);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command
        .spawn()
        .map_err(|error| format!("Avvio fallito: {error}"))?;
    let pid = child.id().ok_or("PID del processo non disponibile")?;
    let job_assigned = assign_job(&child);
    if !job_assigned {
        LogHub::push(
            &logs,
            "Job di sistema non assegnato: lo stop userà la chiusura del processo.".into(),
        );
    }
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take().ok_or("Stdin non disponibile")?;
    if let Some(stdout) = stdout {
        spawn_reader(stdout, logs.clone());
    }
    if let Some(stderr) = stderr {
        spawn_reader(stderr, logs.clone());
    }
    tokio::spawn(async move {
        let _ = child.wait().await;
        on_exit(pid);
    });
    Ok(Spawned { pid, stdin })
}

fn spawn_reader<R>(pipe: R, logs: Arc<Mutex<LogHub>>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(pipe).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            LogHub::push(&logs, line);
        }
    });
}

async fn kill_pid(pid: u32) {
    let mut command = tokio::process::Command::new("taskkill");
    command.args(["/PID", &pid.to_string(), "/T", "/F"]);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let _ = command.status().await;
}

#[cfg(windows)]
fn assign_job(child: &Child) -> bool {
    use std::os::windows::io::AsRawHandle;
    let Some(job) = global_job() else {
        return false;
    };
    let Some(handle) = child.raw_handle() else {
        return false;
    };
    unsafe {
        windows::Win32::System::JobObjects::AssignProcessToJobObject(
            windows::Win32::Foundation::HANDLE(job.as_raw_handle()),
            windows::Win32::Foundation::HANDLE(handle),
        )
        .is_ok()
    }
}

#[cfg(not(windows))]
fn assign_job(_child: &Child) -> bool {
    false
}

#[cfg(windows)]
fn global_job() -> Option<&'static std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::{FromRawHandle, OwnedHandle};
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

#[cfg(not(windows))]
fn global_job() -> Option<&'static ()> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn captures_command_output() {
        let logs = LogHub::new();
        let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into());
        let _spawned = spawn_captured(
            Path::new(&comspec),
            &["/c".into(), "echo voxel-panel".into()],
            &std::env::temp_dir(),
            logs.clone(),
            |_| {},
        )
        .await
        .unwrap();
        let mut seen = false;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let history = logs.lock().unwrap().snapshot().0;
            if history.iter().any(|line| line.contains("voxel-panel")) {
                seen = true;
                break;
            }
        }
        assert!(seen);
    }
}
