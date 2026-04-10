use anyhow::Result;
use chrono::Utc;
use clap::{Args, Parser, Subcommand};

use taskwal::commands::{
    add_task, back_task, done_task, edit_task, note_task, parse_view_mode, rm_task, short_id,
    start_task,
};
use taskwal::state::filter::{apply_daily_view, DailyViewMode};
use taskwal::state::replay;
use taskwal::wal::read_all;
use taskwal::{analytics, ui};

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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let now = Utc::now();

    match cli.command {
        Commands::Add { title, tag } => {
            println!("{}", add_task(now, title, tag)?);
        }

        Commands::Start { id } => {
            let board = replay()?;
            println!("{}", start_task(now, &board, &id)?);
        }

        Commands::Done { id } => {
            let board = replay()?;
            println!("{}", done_task(now, &board, &id)?);
        }

        Commands::Back { id } => {
            let board = replay()?;
            println!("{}", back_task(now, &board, &id)?);
        }

        Commands::Edit { id, title } => {
            let board = replay()?;
            println!("{}", edit_task(now, &board, &id, title)?);
        }

        Commands::Note { id, text } => {
            let board = replay()?;
            println!("{}", note_task(now, &board, &id, text)?);
        }

        Commands::Rm { id } => {
            let board = replay()?;
            println!("{}", rm_task(now, &board, &id)?);
        }

        Commands::Ls { view } => {
            let mode = parse_view_mode(view.all, view.date.as_deref())?;
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
            let stats = analytics::compute(&board.done);
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
            let mode = parse_view_mode(view.all, view.date.as_deref())?;
            ui::run(mode)?;
        }
    }

    Ok(())
}
