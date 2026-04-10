pub mod board;
pub mod stats;

use anyhow::Result;
use chrono::{Local, Utc};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::commands::execute_board_line;
use crate::state::filter::{apply_daily_view, DailyViewMode};
use crate::state::replay;
use crate::state::task::Board;
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

    fn clear_status(&mut self) {
        self.status_line = None;
    }

    fn run_command_line(&mut self) -> Result<()> {
        let line = self.command_buffer.trim();
        if line.is_empty() {
            self.command_buffer.clear();
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
        self.command_focused = false;
        Ok(())
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
        command_focused: false,
        status_line: None,
    };
    app.clamp_selection();

    loop {
        terminal.draw(|f| match app.screen {
            ActiveScreen::Board => board::draw(f, &app),
            ActiveScreen::Stats => stats::draw(f, &app),
        })?;

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
                                app.command_focused = false;
                                app.clear_status();
                            }
                            KeyCode::Enter => {
                                app.run_command_line()?;
                            }
                            KeyCode::Backspace => {
                                app.command_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    continue;
                                }
                                app.command_buffer.push(c);
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
                                        id,
                                        to: Column::Doing,
                                    },
                                })?;
                                app.reload()?;
                            }
                        }
                        KeyCode::Char('d') => {
                            app.clear_status();
                            if let Some(id) = app.selected_task_id() {
                                let now = chrono::Utc::now();
                                append(&WalEntry {
                                    ts: now,
                                    event: WalEvent::Move {
                                        id,
                                        to: Column::Done,
                                    },
                                })?;
                                app.reload()?;
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
                                            event: WalEvent::Move { id, to },
                                        })?;
                                        app.reload()?;
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
