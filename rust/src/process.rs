// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::broadcast;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::{PanelError, PanelResult};

pub const DEFAULT_STOP_TIMEOUT: Duration = Duration::from_secs(30);
const LOG_CAPACITY: usize = 5000;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub id: String,
    pub status: RunStatus,
    pub pid: Option<u32>,
    pub started_unix: Option<i64>,
    pub players: Vec<String>,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub last_exit_code: Option<i32>,
    pub crashed: bool,
}

pub struct LogHub {
    lines: VecDeque<String>,
    tx: broadcast::Sender<String>,
}

impl LogHub {
    pub fn new() -> Arc<Mutex<Self>> {
        let (tx, _) = broadcast::channel(1024);
        Arc::new(Mutex::new(Self {
            lines: VecDeque::new(),
            tx,
        }))
    }

    pub fn push(this: &Mutex<Self>, line: String) {
        let mut hub = this.lock().unwrap_or_else(|error| error.into_inner());
        if hub.lines.len() >= LOG_CAPACITY {
            hub.lines.pop_front();
        }
        hub.lines.push_back(line.clone());
        let _ = hub.tx.send(line);
    }

    pub fn snapshot(&self) -> (Vec<String>, broadcast::Receiver<String>) {
        (self.lines.iter().cloned().collect(), self.tx.subscribe())
    }
}

#[derive(Default)]
struct RuntimeState {
    stdin: Option<ChildStdin>,
    pid: Option<u32>,
    status: Option<RunStatus>,
    started_unix: Option<i64>,
    players: BTreeSet<String>,
    cpu_percent: f64,
    memory_bytes: u64,
    stop_requested: bool,
    last_exit_code: Option<i32>,
    crashed: bool,
}

#[derive(Default)]
struct Supervisor {
    servers: HashMap<String, RuntimeState>,
    logs: HashMap<String, Arc<Mutex<LogHub>>>,
}

fn supervisor() -> &'static Mutex<Supervisor> {
    static SUPERVISOR: OnceLock<Mutex<Supervisor>> = OnceLock::new();
    SUPERVISOR.get_or_init(|| Mutex::new(Supervisor::default()))
}

fn lock() -> std::sync::MutexGuard<'static, Supervisor> {
    supervisor().lock().unwrap_or_else(|error| error.into_inner())
}

fn events() -> &'static broadcast::Sender<RuntimeSnapshot> {
    static EVENTS: OnceLock<broadcast::Sender<RuntimeSnapshot>> = OnceLock::new();
    EVENTS.get_or_init(|| broadcast::channel(256).0)
}

pub fn subscribe_events() -> broadcast::Receiver<RuntimeSnapshot> {
    ensure_sampler();
    events().subscribe()
}

fn snapshot_of(id: &str, runtime: Option<&RuntimeState>) -> RuntimeSnapshot {
    match runtime {
        Some(runtime) => RuntimeSnapshot {
            id: id.to_string(),
            status: runtime.status.unwrap_or(RunStatus::Stopped),
            pid: runtime.pid,
            started_unix: runtime.started_unix,
            players: runtime.players.iter().cloned().collect(),
            cpu_percent: runtime.cpu_percent,
            memory_bytes: runtime.memory_bytes,
            last_exit_code: runtime.last_exit_code,
            crashed: runtime.crashed,
        },
        None => RuntimeSnapshot {
            id: id.to_string(),
            status: RunStatus::Stopped,
            pid: None,
            started_unix: None,
            players: Vec::new(),
            cpu_percent: 0.0,
            memory_bytes: 0,
            last_exit_code: None,
            crashed: false,
        },
    }
}

pub fn snapshot(id: &str) -> RuntimeSnapshot {
    let state = lock();
    snapshot_of(id, state.servers.get(id))
}

pub fn all_snapshots() -> Vec<RuntimeSnapshot> {
    let state = lock();
    state
        .servers
        .iter()
        .map(|(id, runtime)| snapshot_of(id, Some(runtime)))
        .collect()
}

fn publish(id: &str) {
    let snapshot = snapshot(id);
    let _ = events().send(snapshot);
}

fn update(id: &str, change: impl FnOnce(&mut RuntimeState)) {
    {
        let mut state = lock();
        change(state.servers.entry(id.to_string()).or_default());
    }
    publish(id);
}

pub fn init_job() {
    let _ = global_job();
}

pub fn status_of(id: &str) -> RunStatus {
    lock()
        .servers
        .get(id)
        .and_then(|runtime| runtime.status)
        .unwrap_or(RunStatus::Stopped)
}

pub fn pid_of(id: &str) -> Option<u32> {
    lock().servers.get(id).and_then(|runtime| runtime.pid)
}

pub fn is_running(id: &str) -> bool {
    pid_of(id).is_some()
}

pub fn ensure_stopped(id: &str) -> PanelResult<()> {
    if is_running(id) {
        Err(PanelError::running())
    } else {
        Ok(())
    }
}

pub fn any_running() -> bool {
    lock().servers.values().any(|runtime| runtime.pid.is_some())
}

pub fn running_ids() -> Vec<String> {
    lock()
        .servers
        .iter()
        .filter(|(_, runtime)| runtime.pid.is_some())
        .map(|(id, _)| id.clone())
        .collect()
}

pub fn ensure_logs(id: &str) -> Arc<Mutex<LogHub>> {
    let mut state = lock();
    state
        .logs
        .entry(id.to_string())
        .or_insert_with(LogHub::new)
        .clone()
}

pub fn subscribe(id: &str) -> (Vec<String>, broadcast::Receiver<String>) {
    let logs = ensure_logs(id);
    let guard = logs.lock().unwrap_or_else(|error| error.into_inner());
    guard.snapshot()
}

pub fn strip_ansi(line: &str) -> String {
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

/// Lines that mean the server accepts players (Vanilla/Paper/Velocity print "Done (", Bungee "Listening on").
pub fn is_ready_line(plain: &str) -> bool {
    (plain.contains("Done (") && plain.contains(")!")) || plain.contains("Listening on /")
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    Joined(String),
    Left(String),
}

pub fn parse_player_event(plain: &str) -> Option<PlayerEvent> {
    let message = plain.rsplit_once("]: ").map(|(_, rest)| rest).unwrap_or(plain).trim();
    let name_of = |rest: &str| {
        let name = rest.trim();
        let valid = !name.is_empty()
            && name.len() <= 16
            && name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.');
        valid.then(|| name.to_string())
    };
    if let Some(rest) = message.strip_suffix(" joined the game") {
        return name_of(rest).map(PlayerEvent::Joined);
    }
    if let Some(rest) = message.strip_suffix(" left the game") {
        return name_of(rest).map(PlayerEvent::Left);
    }
    None
}

fn observe_line(id: &str, line: &str) {
    let plain = strip_ansi(line);
    let ready = status_of(id) == RunStatus::Starting && is_ready_line(&plain);
    let player = parse_player_event(&plain);
    if !ready && player.is_none() {
        return;
    }
    update(id, |runtime| {
        if ready {
            runtime.status = Some(RunStatus::Running);
        }
        match player {
            Some(PlayerEvent::Joined(name)) => {
                runtime.players.insert(name);
            }
            Some(PlayerEvent::Left(name)) => {
                runtime.players.remove(&name);
            }
            None => {}
        }
    });
}

pub async fn start(id: &str, program: &Path, args: &[String], cwd: &Path) -> PanelResult<()> {
    {
        let mut state = lock();
        let runtime = state.servers.entry(id.to_string()).or_default();
        if runtime.pid.is_some() || runtime.status == Some(RunStatus::Starting) {
            return Err(PanelError::running());
        }
        *runtime = RuntimeState {
            status: Some(RunStatus::Starting),
            ..RuntimeState::default()
        };
    }
    publish(id);
    let logs = ensure_logs(id);
    let spawned = spawn_captured(program, args, cwd, logs.clone(), id.to_string()).await;
    let spawned = match spawned {
        Ok(spawned) => spawned,
        Err(error) => {
            update(id, |runtime| runtime.status = Some(RunStatus::Stopped));
            return Err(error);
        }
    };
    update(id, |runtime| {
        runtime.stdin = Some(spawned.stdin);
        runtime.pid = Some(spawned.pid);
        runtime.started_unix = Some(crate::paths::unix_now());
    });
    LogHub::push(&logs, format!("Processo avviato, PID {}.", spawned.pid));
    Ok(())
}

fn mark_exited(id: &str, pid: u32, code: Option<i32>) {
    let mut matched = false;
    update(id, |runtime| {
        if runtime.pid != Some(pid) {
            return;
        }
        matched = true;
        runtime.crashed = !runtime.stop_requested && code != Some(0);
        runtime.last_exit_code = code;
        runtime.pid = None;
        runtime.stdin = None;
        runtime.status = Some(RunStatus::Stopped);
        runtime.started_unix = None;
        runtime.players.clear();
        runtime.cpu_percent = 0.0;
        runtime.memory_bytes = 0;
    });
    if matched {
        let text = match code {
            Some(code) => format!("Processo {pid} terminato (codice {code})."),
            None => format!("Processo {pid} terminato."),
        };
        LogHub::push(&ensure_logs(id), text);
    }
}

pub async fn send_command(id: &str, command: &str) -> PanelResult<()> {
    let command = command.trim();
    if command.is_empty() {
        return Err(PanelError::invalid("Comando vuoto"));
    }
    if command.contains(['\n', '\r']) {
        return Err(PanelError::invalid("Comando non valido"));
    }
    let mut stdin = {
        let mut state = lock();
        let runtime = state
            .servers
            .get_mut(id)
            .filter(|runtime| runtime.pid.is_some())
            .ok_or_else(PanelError::stopped)?;
        runtime
            .stdin
            .take()
            .ok_or_else(|| PanelError::from("Console occupata, riprova"))?
    };
    let write = async {
        stdin.write_all(format!("{command}\n").as_bytes()).await?;
        stdin.flush().await?;
        Ok::<(), std::io::Error>(())
    };
    let result = write.await;
    {
        let mut state = lock();
        if let Some(runtime) = state.servers.get_mut(id) {
            if runtime.pid.is_some() {
                runtime.stdin = Some(stdin);
            }
        }
    }
    result.map_err(|error| PanelError::io(format!("Invio comando fallito: {error}")))?;
    LogHub::push(&ensure_logs(id), format!("> {command}"));
    Ok(())
}

pub async fn stop(id: &str, timeout: Duration) -> PanelResult<()> {
    let pid = pid_of(id).ok_or_else(PanelError::stopped)?;
    update(id, |runtime| {
        runtime.status = Some(RunStatus::Stopping);
        runtime.stop_requested = true;
    });
    let _ = send_command(id, "stop").await;
    if wait_exit(id, pid, timeout).await {
        return Ok(());
    }
    LogHub::push(&ensure_logs(id), "Arresto oltre il timeout: chiusura forzata.".into());
    kill_pid(pid).await;
    if !wait_exit(id, pid, Duration::from_secs(5)).await {
        mark_exited(id, pid, None);
    }
    Ok(())
}

async fn wait_exit(id: &str, pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if pid_of(id) != Some(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    pid_of(id) != Some(pid)
}

pub async fn shutdown_all(timeout: Duration) {
    let stops = running_ids().into_iter().map(|id| async move {
        let _ = stop(&id, timeout).await;
    });
    futures::future::join_all(stops).await;
}

fn system() -> &'static Mutex<System> {
    static SYSTEM: OnceLock<Mutex<System>> = OnceLock::new();
    SYSTEM.get_or_init(|| Mutex::new(System::new()))
}

/// CPU usage needs two refreshes of the same `System`, so a single instance is kept alive.
fn sample_stats() {
    let running: Vec<(String, u32)> = {
        let state = lock();
        state
            .servers
            .iter()
            .filter_map(|(id, runtime)| runtime.pid.map(|pid| (id.clone(), pid)))
            .collect()
    };
    if running.is_empty() {
        return;
    }
    let pids: Vec<Pid> = running.iter().map(|(_, pid)| Pid::from_u32(*pid)).collect();
    let cpus = std::thread::available_parallelism().map(|value| value.get()).unwrap_or(1) as f64;
    let samples: Vec<(String, f64, u64)> = {
        let mut system = system().lock().unwrap_or_else(|error| error.into_inner());
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&pids),
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
        running
            .iter()
            .filter_map(|(id, pid)| {
                system.process(Pid::from_u32(*pid)).map(|process| {
                    let cpu = f64::from(process.cpu_usage()) / cpus;
                    (id.clone(), cpu.clamp(0.0, 100.0), process.memory())
                })
            })
            .collect()
    };
    for (id, cpu, memory) in samples {
        update(&id, |runtime| {
            runtime.cpu_percent = cpu;
            runtime.memory_bytes = memory;
        });
    }
}

fn ensure_sampler() {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(async {
        let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
        loop {
            ticker.tick().await;
            let _ = tokio::task::spawn_blocking(sample_stats).await;
        }
    });
}

struct Spawned {
    pid: u32,
    stdin: ChildStdin,
}

async fn spawn_captured(
    program: &Path,
    args: &[String],
    cwd: &Path,
    logs: Arc<Mutex<LogHub>>,
    id: String,
) -> PanelResult<Spawned> {
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
        .map_err(|error| PanelError::io(format!("Avvio fallito: {error}")))?;
    let pid = child.id().ok_or("PID del processo non disponibile")?;
    if !assign_job(&child) {
        LogHub::push(
            &logs,
            "Job di sistema non assegnato: lo stop userà la chiusura del processo.".into(),
        );
    }
    let stdin = child.stdin.take().ok_or("Stdin non disponibile")?;
    if let Some(stdout) = child.stdout.take() {
        spawn_reader(stdout, logs.clone(), id.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_reader(stderr, logs.clone(), id.clone());
    }
    tokio::spawn(async move {
        let code = child.wait().await.ok().and_then(|status| status.code());
        mark_exited(&id, pid, code);
    });
    Ok(Spawned { pid, stdin })
}

fn spawn_reader<R>(pipe: R, logs: Arc<Mutex<LogHub>>, id: String)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(pipe).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            observe_line(&id, &line);
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

    #[test]
    fn detects_ready_lines() {
        assert!(is_ready_line("[12:00:00 INFO]: Done (3.210s)! For help, type \"help\""));
        assert!(is_ready_line("[INFO] Listening on /0.0.0.0:25577"));
        assert!(!is_ready_line("[12:00:00 INFO]: Preparing spawn area"));
    }

    #[test]
    fn parses_player_events() {
        assert_eq!(
            parse_player_event("[12:00:00 INFO]: Steve joined the game"),
            Some(PlayerEvent::Joined("Steve".into()))
        );
        assert_eq!(
            parse_player_event("[12:00:00 INFO]: Alex_01 left the game"),
            Some(PlayerEvent::Left("Alex_01".into()))
        );
        assert_eq!(parse_player_event("[12:00:00 INFO]: <Steve> I joined the game"), None);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn captures_command_output() {
        let logs = LogHub::new();
        let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into());
        let _spawned = spawn_captured(
            Path::new(&comspec),
            &["/c".into(), "echo voxel-panel".into()],
            &std::env::temp_dir(),
            logs.clone(),
            "test".into(),
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
