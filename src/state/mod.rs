pub mod filter;
pub mod task;

use anyhow::Result;
use std::collections::HashMap;

use crate::wal::{
    event::{Column, TagAction, WalEntry, WalEvent},
    read_all,
};

use self::task::{Board, Task};

pub fn replay_from(entries: impl IntoIterator<Item = WalEntry>) -> Board {
    let mut tasks: HashMap<String, Task> = HashMap::new();

    for entry in entries {
        match entry.event {
            WalEvent::Add {
                id,
                title,
                tags,
                day,
            } => {
                tasks.insert(
                    id.clone(),
                    Task {
                        id,
                        title,
                        tags,
                        column: Column::Todo,
                        notes: vec![],
                        created_at: entry.ts,
                        started_at: None,
                        done_at: None,
                        created_day: day,
                    },
                );
            }
            WalEvent::Move { id, to } => {
                if let Some(task) = tasks.get_mut(&id) {
                    if to == Column::Doing && task.started_at.is_none() {
                        task.started_at = Some(entry.ts);
                    }
                    if to == Column::Done {
                        task.done_at = Some(entry.ts);
                    } else {
                        task.done_at = None;
                    }
                    task.column = to;
                }
            }
            WalEvent::Edit { id, title } => {
                if let Some(task) = tasks.get_mut(&id) {
                    task.title = title;
                }
            }
            WalEvent::Delete { id } => {
                tasks.remove(&id);
            }
            WalEvent::Note { id, text } => {
                if let Some(task) = tasks.get_mut(&id) {
                    task.notes.push(text);
                }
            }
            WalEvent::Tag { id, action, tag } => {
                if let Some(task) = tasks.get_mut(&id) {
                    match action {
                        TagAction::Add => task.tags.push(tag),
                        TagAction::Remove => task.tags.retain(|t| t != &tag),
                    }
                }
            }
        }
    }

    let mut board = Board::default();
    for task in tasks.into_values() {
        match task.column {
            Column::Todo => board.todo.push(task),
            Column::Doing => board.doing.push(task),
            Column::Done => board.done.push(task),
        }
    }

    board.todo.sort_by_key(|t| t.created_at);
    board.doing.sort_by_key(|t| t.started_at.unwrap_or(t.created_at));
    board.done.sort_by_key(|t| t.done_at);

    board
}

pub fn replay() -> Result<Board> {
    let entries = read_all()?;
    Ok(replay_from(entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::filter::{apply_daily_view, DailyViewMode};
    use crate::wal::event::{Column, WalEntry, WalEvent};
    use chrono::{TimeZone, Utc};

    #[test]
    fn replay_add_move_done() {
        let id = "01TESTTESTTESTTESTTEST".to_string();
        let entries = vec![
            WalEntry {
                ts: Utc.with_ymd_and_hms(2026, 4, 1, 10, 0, 0).unwrap(),
                event: WalEvent::Add {
                    id: id.clone(),
                    title: "t".to_string(),
                    tags: vec![],
                    day: "2026-04-01".to_string(),
                },
            },
            WalEntry {
                ts: Utc.with_ymd_and_hms(2026, 4, 2, 10, 0, 0).unwrap(),
                event: WalEvent::Move {
                    id: id.clone(),
                    to: Column::Doing,
                },
            },
            WalEntry {
                ts: Utc.with_ymd_and_hms(2026, 4, 3, 10, 0, 0).unwrap(),
                event: WalEvent::Move {
                    id: id.clone(),
                    to: Column::Done,
                },
            },
        ];
        let board = replay_from(entries);
        assert_eq!(board.done.len(), 1);
        assert_eq!(board.done[0].id, id);
    }

    #[test]
    fn daily_view_filters_done_only() {
        use chrono::Local;

        let id = "01TESTTESTTESTTESTTEST".to_string();
        let entries = vec![
            WalEntry {
                ts: Utc.with_ymd_and_hms(2026, 4, 1, 10, 0, 0).unwrap(),
                event: WalEvent::Add {
                    id: id.clone(),
                    title: "t".to_string(),
                    tags: vec![],
                    day: "2026-04-01".to_string(),
                },
            },
            WalEntry {
                ts: Utc.with_ymd_and_hms(2026, 4, 2, 10, 0, 0).unwrap(),
                event: WalEvent::Move {
                    id: id.clone(),
                    to: Column::Done,
                },
            },
        ];
        let board = replay_from(entries);
        let done_day = board.done[0]
            .done_at
            .unwrap()
            .with_timezone(&Local)
            .date_naive();
        let wrong_day = done_day.pred_opt().unwrap_or(done_day);
        let view = apply_daily_view(&board, DailyViewMode::Day(wrong_day));
        assert_eq!(view.done.len(), 0);
        let view2 = apply_daily_view(&board, DailyViewMode::Day(done_day));
        assert_eq!(view2.done.len(), 1);
        assert_eq!(view2.todo.len(), 0);
    }
}
