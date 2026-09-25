// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

//! Cron expressions for scheduled tasks (standard 5 fields, local time).

use std::str::FromStr;

use chrono::{DateTime, Local};
use cron::Schedule;

use crate::api::types::{ScheduleKind, ScheduledTask};
use crate::{PanelError, PanelResult};

/// Accepts the usual `minute hour day month weekday` form (the `cron` crate wants seconds too).
pub fn parse(expression: &str) -> PanelResult<Schedule> {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    let full = match fields.len() {
        5 => format!("0 {}", fields.join(" ")),
        6 | 7 => fields.join(" "),
        _ => return Err(PanelError::invalid("Espressione cron non valida: servono 5 campi (minuto ora giorno mese giorno-settimana)")),
    };
    Schedule::from_str(&full).map_err(|error| PanelError::invalid(format!("Espressione cron non valida: {error}")))
}

pub fn next_runs(expression: &str, count: usize) -> PanelResult<Vec<i64>> {
    Ok(parse(expression)?.upcoming(Local).take(count).map(|time| time.timestamp()).collect())
}

/// True when the task should have run in `(previous, now]`.
pub fn is_due(schedule: &Schedule, previous: DateTime<Local>, now: DateTime<Local>) -> bool {
    schedule.after(&previous).next().is_some_and(|next| next <= now)
}

pub fn validate(task: &ScheduledTask) -> PanelResult<()> {
    parse(&task.cron)?;
    if task.kind == ScheduleKind::Command {
        let command = task.command.trim();
        if command.is_empty() || command.contains(['\n', '\r']) {
            return Err(PanelError::invalid("Comando dell'attività non valido"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parses_five_field_expressions() {
        assert!(parse("0 4 * * *").is_ok());
        assert!(parse("*/15 * * * 1-5").is_ok());
        assert!(parse("61 * * * *").is_err());
        assert!(parse("ogni giorno").is_err());
        let runs = next_runs("0 4 * * *", 3).unwrap();
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[1] - runs[0], 24 * 3600);
    }

    #[test]
    fn detects_due_tasks() {
        let schedule = parse("30 12 * * *").unwrap();
        let before = Local.with_ymd_and_hms(2026, 9, 25, 12, 29, 0).unwrap();
        let after = Local.with_ymd_and_hms(2026, 9, 25, 12, 31, 0).unwrap();
        assert!(is_due(&schedule, before, after));
        assert!(!is_due(&schedule, after, after + chrono::Duration::minutes(10)));
    }
}
