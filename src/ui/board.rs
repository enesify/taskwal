use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::state::task::Task;
use crate::ui::App;

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.size();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(7),
        ])
        .split(size);

    let today = Local::now().format("%Y-%m-%d").to_string();
    let mode_label = match app.view_mode {
        crate::state::filter::DailyViewMode::Day(d) => {
            format!("Done: {} (local) ", d.format("%Y-%m-%d"))
        }
        crate::state::filter::DailyViewMode::AllDone => "Done: all time ".to_string(),
    };
    let header = Paragraph::new(format!(" TaskWAL  —  today {}  |  {}", today, mode_label))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(rows[1]);

    render_column(
        f,
        cols[0],
        "TODO",
        &app.display.todo,
        app.selected_col == 0,
        app.selected_row,
        Color::Yellow,
    );
    render_column(
        f,
        cols[1],
        "DOING",
        &app.display.doing,
        app.selected_col == 1,
        app.selected_row,
        Color::Blue,
    );
    render_column(
        f,
        cols[2],
        "DONE",
        &app.display.done,
        app.selected_col == 2,
        app.selected_row,
        Color::Green,
    );

    let footer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(rows[2]);

    let hint = Paragraph::new(
        " s start | d done | b back | a Done view | Tab | g stats | q | : command ",
    )
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(hint, footer[0]);

    let status_text = app
        .status_line
        .as_deref()
        .unwrap_or("");
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(status, footer[1]);

    let input_label = if app.command_focused {
        " command "
    } else {
        " command (press :) "
    };
    let prompt = if app.command_focused {
        format!("> {}", app.command_buffer)
    } else {
        "> ".to_string()
    };
    let input = Paragraph::new(prompt)
        .style(if app.command_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if app.command_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                })
                .title(input_label),
        );
    f.render_widget(input, footer[2]);
}

fn render_column(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    tasks: &[Task],
    is_selected: bool,
    selected_row: usize,
    color: Color,
) {
    let border_style = if is_selected {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let prefix = match title {
                "TODO" => "o",
                "DOING" => "*",
                _ => "x",
            };
            let tags = if task.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", task.tags.join(", "))
            };
            let short = if task.id.len() >= 8 {
                &task.id[..8]
            } else {
                task.id.as_str()
            };
            let label = format!(" {} [{}] {}{}", prefix, short, task.title, tags);
            let style = if is_selected && i == selected_row {
                Style::default().fg(Color::White).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ({}) ", title, tasks.len())),
    );

    f.render_widget(list, area);
}
