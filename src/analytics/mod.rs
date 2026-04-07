use std::collections::HashMap;

use chrono::{Duration, Local, Utc};

use crate::state::task::Task;

pub struct DailyStats {
    pub date: String,
    pub completed: usize,
}

pub struct Analytics {
    pub total_done: usize,
    /// Cycle time: first start (or creation) → done
    pub avg_cycle_days: f64,
    /// Lead time: created → done
    pub avg_lead_days: f64,
    pub avg_daily_done: f64,
    pub current_streak: usize,
    pub daily_breakdown: Vec<DailyStats>,
}

pub fn compute(done_tasks: &[Task]) -> Analytics {
    let total_done = done_tasks.len();

    let avg_cycle_days = if total_done == 0 {
        0.0
    } else {
        let sum: f64 = done_tasks
            .iter()
            .filter_map(|t| {
                let done = t.done_at?;
                let start = t.started_at.unwrap_or(t.created_at);
                Some((done - start).num_seconds() as f64 / 86_400.0)
            })
            .sum();
        sum / total_done as f64
    };

    let avg_lead_days = if total_done == 0 {
        0.0
    } else {
        let sum: f64 = done_tasks
            .iter()
            .filter_map(|t| {
                let done = t.done_at?;
                Some((done - t.created_at).num_seconds() as f64 / 86_400.0)
            })
            .sum();
        sum / total_done as f64
    };

    let mut by_day: HashMap<String, usize> = HashMap::new();
    for task in done_tasks {
        if let Some(done_at) = task.done_at {
            let day = done_at.with_timezone(&Local).format("%Y-%m-%d").to_string();
            *by_day.entry(day).or_default() += 1;
        }
    }

    let active_days = by_day.len();
    let avg_daily_done = if active_days == 0 {
        0.0
    } else {
        total_done as f64 / active_days as f64
    };

    let mut streak = 0usize;
    let mut check = Utc::now().with_timezone(&Local).date_naive();
    loop {
        let key = check.format("%Y-%m-%d").to_string();
        if by_day.contains_key(&key) {
            streak += 1;
            check -= Duration::days(1);
        } else {
            break;
        }
    }

    let mut daily_breakdown: Vec<DailyStats> = by_day
        .into_iter()
        .map(|(date, completed)| DailyStats { date, completed })
        .collect();
    daily_breakdown.sort_by(|a, b| b.date.cmp(&a.date));

    Analytics {
        total_done,
        avg_cycle_days,
        avg_lead_days,
        avg_daily_done,
        current_streak: streak,
        daily_breakdown,
    }
}
