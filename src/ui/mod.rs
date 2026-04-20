pub mod board;
pub mod stats;

use anyhow::Result;
use chrono::{Local, Utc};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::commands::execute_board_line;
use crate::state::filter::{apply_daily_view, DailyViewMode};
use crate::state::replay;
use crate::state::task::{Board, Task};
use crate::wal::event::{Column, WalEntry, WalEvent};
use crate::wal::append;

pub enum ActiveScreen {
    Board,
    Stats,
}

pub struct App {
    pub full_board: Board,
    pub display: Board,
    pub view_mode: DailyViewMode,
    pub selected_col: usize,
    pub selected_row: usize,
    pub screen: ActiveScreen,
    /// Typed command line when `command_focused` is true.
    pub command_buffer: String,
    /// Byte index in `command_buffer` for insert/delete/cursor (UTF-8 boundary).
    pub command_cursor: usize,
    pub command_focused: bool,
    /// Last command result or error (cleared on next navigation key).
    pub status_line: Option<String>,
}

impl App {
    fn refresh_display(&mut self) {
        self.display = apply_daily_view(&self.full_board, self.view_mode);
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let col_len = self.current_column_len();
        if col_len == 0 {
            self.selected_row = 0;
        } else {
            self.selected_row = self.selected_row.min(col_len - 1);
        }
    }

    fn current_column_len(&self) -> usize {
        match self.selected_col {
            0 => self.display.todo.len(),
            1 => self.display.doing.len(),
            _ => self.display.done.len(),
        }
    }

    fn selected_task_id(&self) -> Option<String> {
        let tasks = match self.selected_col {
            0 => &self.display.todo,
            1 => &self.display.doing,
            _ => &self.display.done,
        };
        tasks.get(self.selected_row).map(|t| t.id.clone())
    }

    fn reload(&mut self) -> Result<()> {
        self.full_board = replay()?;
        self.refresh_display();
        Ok(())
    }

    /// Move selection to the row that contains `id` in the current display board (after a move).
    fn focus_task_by_id(&mut self, id: &str) {
        if let Some((col, row)) = task_position_in_board(&self.display, id) {
            self.selected_col = col;
            self.selected_row = row;
        } else {
            self.clamp_selection();
        }
    }

    fn clear_status(&mut self) {
        self.status_line = None;
    }

    fn run_command_line(&mut self) -> Result<()> {
        let line = self.command_buffer.trim();
        if line.is_empty() {
            self.command_buffer.clear();
            self.command_cursor = 0;
            self.command_focused = false;
            self.clear_status();
            return Ok(());
        }
        let now = Utc::now();
        match execute_board_line(line, now, &self.full_board) {
            Ok(msg) => {
                self.reload()?;
                self.status_line = Some(msg);
            }
            Err(e) => {
                self.status_line = Some(format!("{}", e));
            }
        }
        self.command_buffer.clear();
        self.command_cursor = 0;
        self.command_focused = false;
        Ok(())
    }

    /// Selected task in the current column/row, if any (empty column → `None`).
    pub fn selected_task(&self) -> Option<&Task> {
        let tasks = match self.selected_col {
            0 => &self.display.todo,
            1 => &self.display.doing,
            _ => &self.display.done,
        };
        tasks.get(self.selected_row)
    }

    /// Footer middle: command result/error if set, otherwise a one-line preview of the selected task.
    pub fn footer_status_text(&self) -> String {
        if let Some(ref s) = self.status_line {
            return s.clone();
        }
        self.selected_task()
            .map(format_selected_task_preview)
            .unwrap_or_default()
    }
}

fn task_position_in_board(board: &Board, id: &str) -> Option<(usize, usize)> {
    if let Some(i) = board.todo.iter().position(|t| t.id == id) {
        return Some((0, i));
    }
    if let Some(i) = board.doing.iter().position(|t| t.id == id) {
        return Some((1, i));
    }
    if let Some(i) = board.done.iter().position(|t| t.id == id) {
        return Some((2, i));
    }
    None
}

fn insert_char_at_cursor(buf: &mut String, cursor: &mut usize, c: char) {
    buf.insert(*cursor, c);
    *cursor += c.len_utf8();
}

fn backspace_at_cursor(buf: &mut String, cursor: &mut usize) {
    if *cursor == 0 || *cursor > buf.len() {
        return;
    }
    let prev = buf[..*cursor]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0);
    buf.replace_range(prev..*cursor, "");
    *cursor = prev;
}

fn cursor_step_left(buf: &str, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    *cursor = buf[..*cursor]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0);
}

fn cursor_step_right(buf: &str, cursor: &mut usize) {
    if *cursor >= buf.len() {
        return;
    }
    let ch = buf[*cursor..].chars().next().unwrap();
    *cursor += ch.len_utf8();
}

fn format_selected_task_preview(task: &Task) -> String {
    let mut out = format!("{}  {}", task.id, task.title);
    if !task.tags.is_empty() {
        out.push_str(&format!("  [{}]", task.tags.join(", ")));
    }
    if !task.notes.is_empty() {
        out.push_str("  |  ");
        out.push_str(&task.notes.join("; "));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::event::Column;
    use chrono::Utc;

    #[test]
    fn format_preview_includes_id_title_tags_notes() {
        let task = Task {
            id: "01HZTESTTESTTEST".to_string(),
            title: "Fix the board".to_string(),
            tags: vec!["bug".to_string(), "ui".to_string()],
            column: Column::Todo,
            notes: vec!["see BAT-9".to_string(), "wrap long lines".to_string()],
            created_at: Utc::now(),
            started_at: None,
            done_at: None,
            created_day: "2026-04-20".to_string(),
        };
        let s = format_selected_task_preview(&task);
        assert!(s.contains("01HZTESTTESTTEST"));
        assert!(s.contains("Fix the board"));
        assert!(s.contains("[bug, ui]"));
        assert!(s.contains("see BAT-9; wrap long lines"));
    }

    #[test]
    fn task_position_finds_column_and_row() {
        let mut board = Board::default();
        let t = Task {
            id: "01HZFINDME000000".to_string(),
            title: "x".to_string(),
            tags: vec![],
            column: Column::Doing,
            notes: vec![],
            created_at: Utc::now(),
            started_at: None,
            done_at: None,
            created_day: "2026-04-20".to_string(),
        };
        board.doing.push(t);
        assert_eq!(
            task_position_in_board(&board, "01HZFINDME000000"),
            Some((1, 0))
        );
    }

    #[test]
    fn command_cursor_inserts_and_moves() {
        let mut buf = String::new();
        let mut cur = 0usize;
        insert_char_at_cursor(&mut buf, &mut cur, 'a');
        insert_char_at_cursor(&mut buf, &mut cur, 'b');
        assert_eq!(buf, "ab");
        assert_eq!(cur, 2);
        cursor_step_left(&buf, &mut cur);
        assert_eq!(cur, 1);
        insert_char_at_cursor(&mut buf, &mut cur, 'X');
        assert_eq!(buf, "aXb");
        backspace_at_cursor(&mut buf, &mut cur);
        assert_eq!(buf, "ab");
        assert_eq!(cur, 1);
    }
}

pub fn run(initial_mode: DailyViewMode) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let full_board = replay()?;
    let display = apply_daily_view(&full_board, initial_mode);
    let mut app = App {
        full_board,
        display,
        view_mode: initial_mode,
        selected_col: 0,
        selected_row: 0,
        screen: ActiveScreen::Board,
        command_buffer: String::new(),
        command_cursor: 0,
        command_focused: false,
        status_line: None,
    };
    app.clamp_selection();

    loop {
        terminal.draw(|f| match app.screen {
            ActiveScreen::Board => board::draw(f, &app),
            ActiveScreen::Stats => stats::draw(f, &app),
        })?;

        let size = terminal.size()?;
        match (&app.screen, app.command_focused) {
            (ActiveScreen::Board, true) => {
                if let Some((col, row)) = board::command_cursor_position(size, &app) {
                    execute!(terminal.backend_mut(), MoveTo(col, row), Show)?;
                } else {
                    execute!(terminal.backend_mut(), Hide)?;
                }
            }
            _ => {
                execute!(terminal.backend_mut(), Hide)?;
            }
        }

        if let Event::Key(key) = event::read()? {
            match app.screen {
                ActiveScreen::Stats => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Esc | KeyCode::Char('g') | KeyCode::Char('G') => {
                        app.screen = ActiveScreen::Board;
                    }
                    _ => {}
                },
                ActiveScreen::Board => {
                    if app.command_focused {
                        match key.code {
                            KeyCode::Esc => {
                                app.command_buffer.clear();
                                app.command_cursor = 0;
                                app.command_focused = false;
                                app.clear_status();
                            }
                            KeyCode::Enter => {
                                app.run_command_line()?;
                            }
                            KeyCode::Backspace => {
                                backspace_at_cursor(&mut app.command_buffer, &mut app.command_cursor);
                            }
                            KeyCode::Left => {
                                cursor_step_left(&app.command_buffer, &mut app.command_cursor);
                            }
                            KeyCode::Right => {
                                cursor_step_right(&app.command_buffer, &mut app.command_cursor);
                            }
                            KeyCode::Char(c) => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    continue;
                                }
                                insert_char_at_cursor(&mut app.command_buffer, &mut app.command_cursor, c);
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Navigation mode
                    match key.code {
                        KeyCode::Char(':') => {
                            app.clear_status();
                            app.command_focused = true;
                            app.command_cursor = app.command_buffer.len();
                        }
                        KeyCode::Char('q') => break,
                        KeyCode::Tab => {
                            app.clear_status();
                            app.selected_col = (app.selected_col + 1) % 3;
                            app.selected_row = 0;
                            app.clamp_selection();
                        }
                        KeyCode::Char('g') | KeyCode::Char('G') => {
                            app.clear_status();
                            app.screen = ActiveScreen::Stats;
                        }
                        KeyCode::Char('s') => {
                            app.clear_status();
                            if let Some(id) = app.selected_task_id() {
                                let now = chrono::Utc::now();
                                append(&WalEntry {
                                    ts: now,
                                    event: WalEvent::Move {
                                        id: id.clone(),
                                        to: Column::Doing,
                                    },
                                })?;
                                app.reload()?;
                                app.focus_task_by_id(&id);
                            }
                        }
                        KeyCode::Char('d') => {
                            app.clear_status();
                            if let Some(id) = app.selected_task_id() {
                                let now = chrono::Utc::now();
                                append(&WalEntry {
                                    ts: now,
                                    event: WalEvent::Move {
                                        id: id.clone(),
                                        to: Column::Done,
                                    },
                                })?;
                                app.reload()?;
                                app.focus_task_by_id(&id);
                            }
                        }
                        KeyCode::Char('b') => {
                            app.clear_status();
                            if let Some(id) = app.selected_task_id() {
                                if let Some(task) = app.full_board.find_task(&id) {
                                    if let Some(to) = task.column.back_from() {
                                        let now = chrono::Utc::now();
                                        append(&WalEntry {
                                            ts: now,
                                            event: WalEvent::Move {
                                                id: id.clone(),
                                                to,
                                            },
                                        })?;
                                        app.reload()?;
                                        app.focus_task_by_id(&id);
                                    }
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            app.clear_status();
                            app.view_mode = match app.view_mode {
                                DailyViewMode::Day(_) => DailyViewMode::AllDone,
                                DailyViewMode::AllDone => {
                                    DailyViewMode::Day(Local::now().date_naive())
                                }
                            };
                            app.refresh_display();
                        }
                        KeyCode::Up => {
                            app.clear_status();
                            if app.selected_row > 0 {
                                app.selected_row -= 1;
                            }
                        }
                        KeyCode::Down => {
                            app.clear_status();
                            let max = app.current_column_len();
                            if max > 0 && app.selected_row + 1 < max {
                                app.selected_row += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), Show)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
