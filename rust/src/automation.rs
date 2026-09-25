// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Background work: scheduled tasks, automatic restarts, autostart and live metrics.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use chrono::Local;

use crate::api::types::{ScheduleKind, ScheduledTask};
use crate::paths::Layout;
use crate::process::{self, RunStatus};
use crate::{PanelError, PanelResult};

const SCHEDULER_TICK: Duration = Duration::from_secs(20);
const METRICS_TICK: Duration = Duration::from_secs(10);
const DISK_TICK: Duration = Duration::from_secs(60);
/// Waits before each automatic restart of a crash streak; the streak ends after the last one.
const RESTART_BACKOFF: &[u64] = &[5, 15, 60, 180, 600];
/// A server that stayed up this long starts a new streak on its next crash.
const STABLE_AFTER: Duration = Duration::from_secs(10 * 60);

pub fn start_background_tasks() {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(scheduler_loop());
    tokio::spawn(metrics_loop());
    tokio::spawn(disk_loop());
}

fn streaks() -> &'static Mutex<HashMap<String, u32>> {
    static STREAKS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
    STREAKS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Next delay of the crash streak, or `None` when the attempts are exhausted.
pub fn next_backoff(attempts: u32) -> Option<Duration> {
    RESTART_BACKOFF.get(attempts as usize).map(|seconds| Duration::from_secs(*seconds))
}

/// `uptime_secs` is how long the server ran before crashing.
pub async fn on_crash(id: &str, uptime_secs: Option<i64>) {
    let Ok(record) = crate::catalog::get(&Layout::app(), id) else { return };
    if !record.auto_restart {
        return;
    }
    let attempts = {
        let mut streaks = streaks().lock().unwrap_or_else(|error| error.into_inner());
        let attempts = streaks.entry(id.to_string()).or_insert(0);
        if uptime_secs.is_some_and(|uptime| uptime as u64 >= STABLE_AFTER.as_secs()) {
            *attempts = 0;
        }
        *attempts
    };
    let logs = process::ensure_logs(id);
    let Some(delay) = next_backoff(attempts) else {
        process::LogHub::push(&logs, "Troppi crash consecutivi: riavvio automatico sospeso.".into());
        tracing::warn!(server = id, "riavvio automatico sospeso dopo {attempts} tentativi");
        return;
    };
    process::LogHub::push(&logs, format!("Riavvio automatico tra {} secondi (tentativo {}).", delay.as_secs(), attempts + 1));
    tokio::time::sleep(delay).await;
    if process::is_running(id) {
        return;
    }
    *streaks().lock().unwrap_or_else(|error| error.into_inner()).entry(id.to_string()).or_insert(0) += 1;
    process::set_restart_attempts(id, attempts + 1);
    tracing::info!(server = id, attempt = attempts + 1, "riavvio automatico dopo un crash");
    if let Err(error) = crate::install::launch(&Layout::app(), id).await {
        process::LogHub::push(&logs, format!("Riavvio automatico non riuscito: {}", error.message));
    }
}

/// Starts the servers flagged for autostart, one after the other.
pub async fn autostart() -> PanelResult<Vec<String>> {
    if !crate::launcher_settings::current().general.autostart_servers {
        return Ok(Vec::new());
    }
    let mut started = Vec::new();
    for record in crate::catalog::list(&Layout::app())?.into_iter().filter(|record| record.autostart) {
        if process::is_running(&record.id) {
            continue;
        }
        match crate::install::launch(&Layout::app(), &record.id).await {
            Ok(()) => started.push(record.name.clone()),
            Err(error) => tracing::warn!(server = record.id, "avvio automatico non riuscito: {}", error.message),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Ok(started)
}

async fn say(id: &str, message: &str) {
    let _ = process::send_command(id, &format!("say {message}")).await;
}

async fn restart_with_warnings(id: &str, warn: bool) -> PanelResult<()> {
    if process::status_of(id) != RunStatus::Running {
        return crate::install::launch(&Layout::app(), id).await;
    }
    if warn {
        say(id, "Riavvio programmato del server tra 5 minuti.").await;
        tokio::time::sleep(Duration::from_secs(240)).await;
        say(id, "Riavvio programmato del server tra 1 minuto.").await;
        tokio::time::sleep(Duration::from_secs(50)).await;
        say(id, "Riavvio programmato del server tra 10 secondi.").await;
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    process::stop_server(id).await?;
    crate::install::launch(&Layout::app(), id).await
}

/// Backup of a running server: world saving is paused while the archive is written.
pub async fn backup_now(id: &str, prefix: &str) -> PanelResult<String> {
    let layout = Layout::app();
    let record = crate::catalog::get(&layout, id)?;
    let running = process::status_of(id) == RunStatus::Running;
    if running {
        let _ = run_quiet(id, "save-off").await;
        let _ = run_quiet(id, "save-all flush").await;
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    let result = crate::backup::safety_backup(&layout, &record, prefix).await;
    if running {
        let _ = run_quiet(id, "save-on").await;
    }
    let file = result?;
    crate::backup::prune(&layout.backups(id), crate::launcher_settings::current().backup.retention)?;
    Ok(file)
}

/// RCON when available (no console noise), stdin otherwise.
async fn run_quiet(id: &str, command: &str) -> PanelResult<String> {
    match crate::rcon::execute_for(id, command).await {
        Ok(response) => Ok(response),
        Err(_) => process::send_command(id, command).await.map(|_| String::new()),
    }
}

pub async fn run_task(id: &str, task: &ScheduledTask) -> PanelResult<()> {
    let logs = process::ensure_logs(id);
    process::LogHub::push(&logs, format!("Attività pianificata: {:?} ({})", task.kind, task.cron));
    match task.kind {
        ScheduleKind::Restart => restart_with_warnings(id, task.warn_players).await,
        ScheduleKind::Start if !process::is_running(id) => crate::install::launch(&Layout::app(), id).await,
        ScheduleKind::Start => Ok(()),
        ScheduleKind::Stop if process::is_running(id) => process::stop_server(id).await,
        ScheduleKind::Stop => Ok(()),
        ScheduleKind::Command if process::is_running(id) => run_quiet(id, task.command.trim()).await.map(|_| ()),
        ScheduleKind::Command => Err(PanelError::stopped()),
        ScheduleKind::Backup => backup_now(id, "scheduled").await.map(|file| {
            process::LogHub::push(&logs, format!("Backup pianificato creato: {file}"));
        }),
    }
}

fn mark_run(id: &str, task_id: &str, when: i64) {
    let layout = Layout::app();
    if let Ok(mut record) = crate::catalog::get(&layout, id) {
        if let Some(task) = record.schedules.iter_mut().find(|task| task.id == task_id) {
            task.last_run_unix = Some(when);
            let _ = crate::catalog::save(&layout, &record);
        }
    }
}

async fn scheduler_loop() {
    let mut previous = Local::now();
    let mut ticker = tokio::time::interval(SCHEDULER_TICK);
    loop {
        ticker.tick().await;
        let now = Local::now();
        let Ok(records) = crate::catalog::list(&Layout::app()) else { continue };
        for record in records {
            for task in record.schedules.iter().filter(|task| task.enabled) {
                let Ok(schedule) = crate::scheduler::parse(&task.cron) else { continue };
                if !crate::scheduler::is_due(&schedule, previous, now) {
                    continue;
                }
                mark_run(&record.id, &task.id, now.timestamp());
                let (id, task) = (record.id.clone(), task.clone());
                tokio::spawn(async move {
                    if let Err(error) = run_task(&id, &task).await {
                        tracing::warn!(server = id, "attività pianificata non riuscita: {}", error.message);
                        process::LogHub::push(&process::ensure_logs(&id), format!("Attività pianificata non riuscita: {}", error.message));
                    }
                });
            }
        }
        previous = now;
    }
}

/// Paper and forks answer `tps` / `mspt`; Vanilla 1.20.3+ answers `tick query`.
pub fn parse_tps(response: &str) -> Option<f64> {
    let plain = crate::rcon::strip_formatting(response);
    let after = plain.split_once(':').map(|(_, rest)| rest).unwrap_or(&plain);
    after
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .map(|part| part.trim_start_matches('*').trim())
        .find_map(|part| part.parse::<f64>().ok())
        .map(|tps| tps.min(20.0))
}

pub fn parse_mspt(response: &str) -> Option<f64> {
    let plain = crate::rcon::strip_formatting(response);
    if let Some(index) = plain.find("Average time per tick:") {
        let rest = &plain[index + "Average time per tick:".len()..];
        return rest.split_whitespace().next().and_then(|value| value.trim_end_matches("ms").parse().ok());
    }
    // `mspt`: "... from last 5s, 10s, 1m:\n◴ 1.2/0.5/3.4, ..." (average of the last 5 s first).
    let (_, rest) = plain.split_once(":\n").or_else(|| plain.rsplit_once(": "))?;
    rest.split(|ch: char| ch == '/' || ch == ',' || ch.is_whitespace()).find_map(|part| part.trim_start_matches('◴').parse::<f64>().ok())
}

pub fn tick_query_tps(response: &str) -> Option<f64> {
    let plain = crate::rcon::strip_formatting(response);
    let mspt = parse_mspt(&plain)?;
    let target = plain
        .split("Target tick rate:")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(20.0);
    Some(if mspt <= 0.0 { target } else { (1000.0 / mspt).min(target) })
}

async fn measure(id: &str) -> (Option<f64>, Option<f64>) {
    if let Ok(response) = crate::rcon::execute_for(id, "tps").await {
        if let Some(tps) = parse_tps(&response).filter(|_| !response.to_lowercase().contains("unknown")) {
            let mspt = crate::rcon::execute_for(id, "mspt").await.ok().and_then(|response| parse_mspt(&response));
            return (Some(tps), mspt);
        }
    }
    match crate::rcon::execute_for(id, "tick query").await {
        Ok(response) => (tick_query_tps(&response), parse_mspt(&response)),
        Err(_) => (None, None),
    }
}

async fn metrics_loop() {
    let mut ticker = tokio::time::interval(METRICS_TICK);
    loop {
        ticker.tick().await;
        let Ok(records) = crate::catalog::list(&Layout::app()) else { continue };
        for record in records {
            if record.rcon.is_none() || crate::providers::is_proxy(record.provider) || process::status_of(&record.id) != RunStatus::Running {
                continue;
            }
            let (tps, mspt) = measure(&record.id).await;
            process::set_metrics(&record.id, tps, mspt);
        }
    }
}

async fn disk_loop() {
    let mut ticker = tokio::time::interval(DISK_TICK);
    loop {
        ticker.tick().await;
        for id in process::running_ids() {
            let Ok(record) = crate::catalog::get(&Layout::app(), &id) else { continue };
            let root = record.root.clone();
            if let Ok(bytes) = tokio::task::spawn_blocking(move || crate::worlds::dir_size(&root)).await {
                process::set_disk(&id, bytes);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paper_tps_and_mspt() {
        assert_eq!(parse_tps("§6TPS from last 1m, 5m, 15m: §a*20.0, §a19.87, §a19.9"), Some(20.0));
        assert_eq!(parse_tps("TPS from last 1m, 5m, 15m: 18.5, 19.0, 19.5"), Some(18.5));
        let mspt = "§6Server tick times §e(§7avg§e/§7min§e/§7max§e)§6 from last 5s§7,§6 10s§7,§6 1m§e:\n§6◴ §a1.2§7/§a0.5§7/§a3.4§e, §a1.1§7/§a0.4§7/§a3.0";
        assert_eq!(parse_mspt(mspt), Some(1.2));
    }

    #[test]
    fn parses_vanilla_tick_query() {
        let response = "The game is running normally\nTarget tick rate: 20.0 per second.\nAverage time per tick: 25.0ms (Target: 50.0ms)";
        assert_eq!(parse_mspt(response), Some(25.0));
        assert_eq!(tick_query_tps(response), Some(20.0));
        let slow = "Target tick rate: 20.0 per second.\nAverage time per tick: 80.0ms (Target: 50.0ms)";
        assert_eq!(tick_query_tps(slow), Some(12.5));
    }

    #[test]
    fn backs_off_and_gives_up() {
        assert_eq!(next_backoff(0), Some(Duration::from_secs(5)));
        assert_eq!(next_backoff(4), Some(Duration::from_secs(600)));
        assert_eq!(next_backoff(5), None);
    }
}

/// Full cycle on a real Paper server. Needs `VOXELPANEL_TEST_JAVA_HOME` (a Java able to run the
/// newest Paper) and should run with `VOXELPANEL_DATA_DIR` pointing to a scratch folder.
#[cfg(test)]
mod live {
    use super::*;
    use crate::api::types::{CreateServerRequest, ProviderKind};

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn live_runs_paper_with_rcon_and_metrics() {
        let Ok(java_home) = std::env::var("VOXELPANEL_TEST_JAVA_HOME") else { return };
        let layout = Layout::app();
        let version = crate::providers::get(ProviderKind::Paper).unwrap().versions(false).await.unwrap()[0].id.clone();
        let request = CreateServerRequest {
            name: "Live RCON".into(),
            root: String::new(),
            provider: ProviderKind::Paper,
            mc_version: version,
            build: String::new(),
            jar_path: String::new(),
            java_home,
            ram_min: "1G".into(),
            ram_max: "2G".into(),
            jvm_flags: crate::jvm::preset_flags(crate::api::settings::JvmPreset::Aikar),
            port: 0,
            motd: "live".into(),
            max_players: 5,
            gamemode: "survival".into(),
            difficulty: "easy".into(),
            online_mode: false,
            level_seed: String::new(),
            accept_eula: true,
        };
        let id = crate::install::create_server(&layout, request, &crate::ProgressTx::silent()).await.unwrap();
        crate::install::launch(&layout, &id).await.unwrap();
        let _ = process::subscribe_events();
        let mut ready = false;
        for _ in 0..240 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if process::status_of(&id) == RunStatus::Running {
                ready = true;
                break;
            }
        }
        assert!(ready, "server did not reach Running: {:?}", process::console_tail(&id));
        let record = crate::catalog::get(&layout, &id).unwrap();
        assert!(record.rcon.is_some(), "RCON not configured");
        let list = crate::rcon::execute_for(&id, "list").await.unwrap();
        println!("list -> {list}");
        let (tps, mspt) = measure(&id).await;
        println!("tps {tps:?} mspt {mspt:?}");
        assert!(tps.is_some());
        let backup = backup_now(&id, "live").await.unwrap();
        println!("backup {backup}");
        process::stop_server(&id).await.unwrap();
        assert_eq!(process::status_of(&id), RunStatus::Stopped);
        assert!(!process::snapshot(&id).crashed);
        crate::install::delete_server(&layout, &id, true, true).await.unwrap();
    }
}
