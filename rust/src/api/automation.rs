// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use crate::paths::Layout;
use crate::{PanelError, PanelResult};

use super::types::*;

pub async fn list_schedules(id: String) -> PanelResult<Vec<ScheduledTask>> {
    Ok(crate::catalog::get(&Layout::app(), &id)?.schedules)
}

/// Replaces the scheduled tasks of a server; every task is validated first.
pub async fn save_schedules(id: String, tasks: Vec<ScheduledTask>) -> PanelResult<()> {
    let layout = Layout::app();
    let mut record = crate::catalog::get(&layout, &id)?;
    let mut clean = Vec::with_capacity(tasks.len());
    for mut task in tasks {
        crate::scheduler::validate(&task)?;
        if task.id.trim().is_empty() {
            task.id = uuid::Uuid::new_v4().simple().to_string();
        }
        task.cron = task.cron.split_whitespace().collect::<Vec<_>>().join(" ");
        task.command = task.command.trim().to_string();
        clean.push(task);
    }
    record.schedules = clean;
    crate::catalog::save(&layout, &record)
}

/// Next run times (unix seconds) of a cron expression, for previews while editing.
#[flutter_rust_bridge::frb(sync)]
pub fn next_schedule_runs(cron: String, count: u32) -> PanelResult<Vec<i64>> {
    crate::scheduler::next_runs(&cron, count.min(10) as usize)
}

pub async fn run_schedule_now(id: String, task_id: String) -> PanelResult<()> {
    let record = crate::catalog::get(&Layout::app(), &id)?;
    let task = record.schedules.into_iter().find(|task| task.id == task_id).ok_or_else(|| PanelError::not_found("Attività non trovata"))?;
    crate::automation::run_task(&id, &task).await
}

/// Starts the servers marked for autostart (when enabled in the launcher settings); returns their names.
pub async fn autostart_servers() -> PanelResult<Vec<String>> {
    crate::automation::autostart().await
}

/// Size of the server folder in bytes.
pub async fn server_disk_usage(id: String) -> PanelResult<i64> {
    let root = crate::catalog::get(&Layout::app(), &id)?.root;
    Ok(tokio::task::spawn_blocking(move || crate::worlds::dir_size(&root)).await? as i64)
}
