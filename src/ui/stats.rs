use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::analytics::compute;
use crate::ui::App;

pub fn draw(f: &mut Frame, app: &App) {
    let stats = compute(&app.full_board.done);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stats (all-time) | g / Esc back | q quit ")
        .style(Style::default().fg(Color::Cyan));

    let recent = stats
        .daily_breakdown
        .iter()
        .take(7)
        .map(|d| format!("    {} -> {} done", d.date, d.completed))
        .collect::<Vec<_>>()
        .join("\n");

    let text = format!(
        "\n  Total completed   : {}\n\n  Avg cycle time    : {:.1} d\n\n  Avg lead time     : {:.1} d\n\n  Avg per active day: {:.1} tasks\n\n  Streak (local)    : {} d\n\n  Recent days:\n{}\n",
        stats.total_done,
        stats.avg_cycle_days,
        stats.avg_lead_days,
        stats.avg_daily_done,
        stats.current_streak,
        recent
    );

    let para = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(para, f.size());
}
