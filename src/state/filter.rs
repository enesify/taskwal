use chrono::{DateTime, Local, NaiveDate, Utc};

use crate::state::task::Board;

#[derive(Clone, Copy)]
pub enum DailyViewMode {
    /// Done column: tasks completed on this calendar day (local date of `done_at`).
    Day(NaiveDate),
    /// Done column: all completed tasks; Todo/Doing unchanged.
    AllDone,
}

fn done_local_day(done_at: DateTime<Utc>) -> NaiveDate {
    done_at.with_timezone(&Local).date_naive()
}

pub fn apply_daily_view(board: &Board, mode: DailyViewMode) -> Board {
    let mut out = Board::default();
    out.todo = board.todo.clone();
    out.doing = board.doing.clone();

    match mode {
        DailyViewMode::Day(day) => {
            out.done = board
                .done
                .iter()
                .filter(|t| {
                    t.done_at
                        .map(done_local_day) == Some(day)
                })
                .cloned()
                .collect();
            out.done.sort_by_key(|t| t.done_at);
        }
        DailyViewMode::AllDone => {
            out.done = board.done.clone();
            out.done.sort_by_key(|t| t.done_at);
        }
    }

    out
}
