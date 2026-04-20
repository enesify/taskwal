use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::state::task::Task;
use crate::ui::App;

fn split_main(area: Rect) -> (Rect, Rect, Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(7),
        ])
        .split(area);
    (rows[0], rows[1], rows[2])
}

/// Footer: single-line status (top), command row (middle), keys hint (bottom).
fn split_footer(footer: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(footer);
    (chunks[0], chunks[1], chunks[2])
}

/// Truncate to one terminal row (ellipsis when shortened).
fn fit_status_line(s: &str, max_cols: u16) -> String {
    let max = max_cols as usize;
    if max == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    let ell = "…";
    let ell_w = UnicodeWidthStr::width(ell);
    if max <= ell_w {
        return ell.chars().take(max).collect();
    }
    let budget = max - ell_w;
    let mut acc = 0usize;
    let mut end_byte = 0usize;
    for (i, ch) in s.char_indices() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if acc + w > budget {
            break;
        }
        acc += w;
        end_byte = i + ch.len_utf8();
    }
    format!("{}{}", &s[..end_byte], ell)
}

fn input_block_title(app: &App) -> &'static str {
    if app.command_focused {
        " command "
    } else {
        " command (press :) "
    }
}

/// Terminal cursor position `(column, row)` after `> ` + buffer (insert point), using default cursor style.
pub fn command_cursor_position(term_area: Rect, app: &App) -> Option<(u16, u16)> {
    if !app.command_focused {
        return None;
    }
    let (_, _, footer) = split_main(term_area);
    let (_, input_outer, _) = split_footer(footer);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(input_block_title(app));
    let inner = block.inner(input_outer);
    let prefix = "> ";
    let mut c = app.command_cursor.min(app.command_buffer.len());
    while c > 0 && !app.command_buffer.is_char_boundary(c) {
        c -= 1;
    }
    let before = &app.command_buffer[..c];
    let w = UnicodeWidthStr::width(prefix).saturating_add(UnicodeWidthStr::width(before));
    let w_u16 = u16::try_from(w).unwrap_or(u16::MAX);
    let col = inner.x.saturating_add(w_u16);
    let row = inner.y;
    Some((col, row))
}

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.size();

    let (header_area, cols_area, footer_area) = split_main(size);

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
    f.render_widget(header, header_area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(cols_area);

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

    let (status_area, input_area, hint_area) = split_footer(footer_area);

    let status_raw = app.footer_status_text();
    let status_w = status_area.width;
    let status_text = fit_status_line(&status_raw, status_w);
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(status, status_area);

    let input_label = input_block_title(app);
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
    f.render_widget(input, input_area);

    let hint = Paragraph::new(
        " s start | d done | b back | a Done view | Tab | g stats | q | : command | Esc | ←/→ ",
    )
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default().borders(Borders::ALL).title(" keymap "));
    f.render_widget(hint, hint_area);
}

fn selection_style(column_accent: Color) -> Style {
    let fg = match column_accent {
        Color::Blue => Color::White,
        _ => Color::Black,
    };
    Style::default()
        .fg(fg)
        .bg(column_accent)
        .add_modifier(Modifier::BOLD)
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
    let selected_row_style = selection_style(color);

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
                selected_row_style
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
