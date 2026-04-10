mod analytics;
mod state;
mod ui;
mod wal;

use anyhow::{bail, Context, Result};
use chrono::{Local, NaiveDate, Utc};
use clap::{Args, Parser, Subcommand};
use ulid::Ulid;

use crate::state::filter::{apply_daily_view, DailyViewMode};
use crate::state::replay;
use crate::state::task::Board;
use crate::wal::event::{Column, WalEntry, WalEvent};
use crate::wal::{append, read_all};

#[derive(Parser)]
#[command(name = "tw", about = "TaskWAL — local WAL-backed task tracker")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Clone, Default)]
struct ViewOpts {
    /// Show all completed tasks in the Done column (Todo/Doing still show all open work).
    #[arg(short = 'a', long)]
    all: bool,
    /// Calendar day for the Done column (local timezone). Default: today.
    #[arg(long, value_name = "YYYY-MM-DD")]
    date: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a task for today
    Add {
        title: String,
        #[arg(short, long, value_delimiter = ',')]
        tag: Vec<String>,
    },
    /// Move a task to Doing
    Start {
        id: String,
    },
    /// Move a task to Done
    Done {
        id: String,
    },
    /// Move one step back (Done → Doing, Doing → Todo)
    Back {
        id: String,
    },
    /// Rename a task
    Edit {
        id: String,
        title: String,
    },
    /// Append a note to a task
    Note {
        id: String,
        text: String,
    },
    /// Remove a task
    Rm {
        id: String,
    },
    /// Open the TUI board
    Board {
        #[command(flatten)]
        view: ViewOpts,
    },
    /// List tasks for the current view
    Ls {
        #[command(flatten)]
        view: ViewOpts,
    },
    /// Print aggregate statistics (all-time)
    Stats,
    /// Dump the raw WAL as JSON lines
    Log,
}

fn parse_view_mode(opts: &ViewOpts) -> Result<DailyViewMode> {
    if opts.all && opts.date.is_some() {
        bail!("--all and --date cannot be used together");
    }
    if opts.all {
        return Ok(DailyViewMode::AllDone);
    }
    if let Some(d) = &opts.date {
        let day = NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .with_context(|| format!("invalid date {:?}", d))?;
        return Ok(DailyViewMode::Day(day));
    }
    Ok(DailyViewMode::Day(Local::now().date_naive()))
}

fn resolve_task_id(prefix: &str, board: &Board) -> Result<String> {
    if prefix.is_empty() {
        bail!("task id prefix cannot be empty");
    }
    let mut matches: Vec<&str> = board
        .todo
        .iter()
        .chain(board.doing.iter())
        .chain(board.done.iter())
        .map(|t| t.id.as_str())
        .filter(|id| id.starts_with(prefix))
        .collect();
    matches.sort_unstable();
    matches.dedup();
    match matches.len() {
        0 => bail!("no task id matches prefix {:?}", prefix),
        1 => Ok(matches[0].to_string()),
        _ => bail!("ambiguous id prefix {:?}", prefix),
    }
}

fn short_id(id: &str) -> &str {
    if id.len() >= 8 {
        &id[..8]
    } else {
        id
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let now = Utc::now();

    match cli.command {
        Commands::Add { title, tag } => {
            let id = Ulid::new().to_string();
            let entry = WalEntry {
                ts: now,
                event: WalEvent::Add {
                    id: id.clone(),
                    title: title.clone(),
                    tags: tag.clone(),
                    day: Local::now().format("%Y-%m-%d").to_string(),
                },
            };
            append(&entry)?;
            println!("added [{}] {}", short_id(&id), title);
        }

        Commands::Start { id } => {
            let board = replay()?;
            let id = resolve_task_id(&id, &board)?;
            let entry = WalEntry {
                ts: now,
                event: WalEvent::Move {
                    id: id.clone(),
                    to: Column::Doing,
                },
            };
            append(&entry)?;
            println!("doing {}", short_id(&id));
        }

        Commands::Done { id } => {
            let board = replay()?;
            let id = resolve_task_id(&id, &board)?;
            let entry = WalEntry {
                ts: now,
                event: WalEvent::Move {
                    id: id.clone(),
                    to: Column::Done,
                },
            };
            append(&entry)?;
            println!("done {}", short_id(&id));
        }

        Commands::Back { id } => {
            let board = replay()?;
            let id = resolve_task_id(&id, &board)?;
            let task = board
                .find_task(&id)
                .context("task not found after resolve")?;
            let to = task.column.back_from().context(
                "task is already at Todo (nowhere to go back)",
            )?;
            let entry = WalEntry {
                ts: now,
                event: WalEvent::Move {
                    id: id.clone(),
                    to,
                },
            };
            append(&entry)?;
            let label = match to {
                Column::Doing => "doing",
                Column::Todo => "todo",
                Column::Done => unreachable!("back_from never returns Done"),
            };
            println!("back -> {} {}", label, short_id(&id));
        }

        Commands::Edit { id, title } => {
            let board = replay()?;
            let id = resolve_task_id(&id, &board)?;
            let entry = WalEntry {
                ts: now,
                event: WalEvent::Edit { id: id.clone(), title: title.clone() },
            };
            append(&entry)?;
            println!("updated [{}] {}", short_id(&id), title);
        }

        Commands::Note { id, text } => {
            let board = replay()?;
            let id = resolve_task_id(&id, &board)?;
            let entry = WalEntry {
                ts: now,
                event: WalEvent::Note {
                    id: id.clone(),
                    text: text.clone(),
                },
            };
            append(&entry)?;
            println!("note {}", short_id(&id));
        }

        Commands::Rm { id } => {
            let board = replay()?;
            let id = resolve_task_id(&id, &board)?;
            let entry = WalEntry {
                ts: now,
                event: WalEvent::Delete { id: id.clone() },
            };
            append(&entry)?;
            println!("removed {}", short_id(&id));
        }

        Commands::Ls { view } => {
            let mode = parse_view_mode(&view)?;
            let board = replay()?;
            let view_board = apply_daily_view(&board, mode);
            let day_label = match mode {
                DailyViewMode::Day(d) => d.format("%Y-%m-%d").to_string(),
                DailyViewMode::AllDone => "all (Done column)".to_string(),
            };
            println!("\nTaskWAL — {}\n", day_label);
            println!("TODO ({}):", view_board.todo.len());
            for t in &view_board.todo {
                println!("  - [{}] {}", short_id(&t.id), t.title);
            }
            println!("\nDOING ({}):", view_board.doing.len());
            for t in &view_board.doing {
                println!("  * [{}] {}", short_id(&t.id), t.title);
            }
            println!("\nDONE ({}):", view_board.done.len());
            for t in &view_board.done {
                println!("  x [{}] {}", short_id(&t.id), t.title);
            }
        }

        Commands::Stats => {
            let board = replay()?;
            let stats = crate::analytics::compute(&board.done);
            println!("\nStats (all completed tasks)");
            println!("  Total completed      : {}", stats.total_done);
            println!("  Avg cycle time     : {:.1} d", stats.avg_cycle_days);
            println!("  Avg lead time      : {:.1} d", stats.avg_lead_days);
            println!("  Avg / active day   : {:.1}", stats.avg_daily_done);
            println!("  Streak (local days): {}", stats.current_streak);
        }

        Commands::Log => {
            let entries = read_all()?;
            for e in entries {
                println!("{}", serde_json::to_string(&e)?);
            }
        }

        Commands::Board { view } => {
            let mode = parse_view_mode(&view)?;
            ui::run(mode)?;
        }
    }

    Ok(())
}
