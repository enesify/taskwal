//! Shared CLI / board command execution (WAL mutations and helpers).

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Local, NaiveDate, Utc};
use ulid::Ulid;

use crate::state::task::Board;
use crate::state::filter::DailyViewMode;
use crate::wal::append;
use crate::wal::event::{Column, WalEntry, WalEvent};

pub fn short_id(id: &str) -> &str {
    if id.len() >= 8 {
        &id[..8]
    } else {
        id
    }
}

pub fn resolve_task_id(prefix: &str, board: &Board) -> Result<String> {
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

pub fn parse_view_mode(all: bool, date: Option<&str>) -> Result<DailyViewMode> {
    if all && date.is_some() {
        bail!("--all and --date cannot be used together");
    }
    if all {
        return Ok(DailyViewMode::AllDone);
    }
    if let Some(d) = date {
        let day = NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .with_context(|| format!("invalid date {:?}", d))?;
        return Ok(DailyViewMode::Day(day));
    }
    Ok(DailyViewMode::Day(Local::now().date_naive()))
}

pub fn add_task(now: DateTime<Utc>, title: String, tags: Vec<String>) -> Result<String> {
    let id = Ulid::new().to_string();
    let entry = WalEntry {
        ts: now,
        event: WalEvent::Add {
            id: id.clone(),
            title: title.clone(),
            tags,
            day: Local::now().format("%Y-%m-%d").to_string(),
        },
    };
    append(&entry)?;
    Ok(format!("added [{}] {}", short_id(&id), title))
}

pub fn start_task(now: DateTime<Utc>, board: &Board, id_prefix: &str) -> Result<String> {
    let id = resolve_task_id(id_prefix, board)?;
    let entry = WalEntry {
        ts: now,
        event: WalEvent::Move {
            id: id.clone(),
            to: Column::Doing,
        },
    };
    append(&entry)?;
    Ok(format!("doing {}", short_id(&id)))
}

pub fn done_task(now: DateTime<Utc>, board: &Board, id_prefix: &str) -> Result<String> {
    let id = resolve_task_id(id_prefix, board)?;
    let entry = WalEntry {
        ts: now,
        event: WalEvent::Move {
            id: id.clone(),
            to: Column::Done,
        },
    };
    append(&entry)?;
    Ok(format!("done {}", short_id(&id)))
}

pub fn back_task(now: DateTime<Utc>, board: &Board, id_prefix: &str) -> Result<String> {
    let id = resolve_task_id(id_prefix, board)?;
    let task = board
        .find_task(&id)
        .context("task not found after resolve")?;
    let to = task
        .column
        .back_from()
        .context("task is already at Todo (nowhere to go back)")?;
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
    Ok(format!("back -> {} {}", label, short_id(&id)))
}

pub fn edit_task(now: DateTime<Utc>, board: &Board, id_prefix: &str, title: String) -> Result<String> {
    let id = resolve_task_id(id_prefix, board)?;
    let entry = WalEntry {
        ts: now,
        event: WalEvent::Edit { id: id.clone(), title: title.clone() },
    };
    append(&entry)?;
    Ok(format!("updated [{}] {}", short_id(&id), title))
}

pub fn note_task(now: DateTime<Utc>, board: &Board, id_prefix: &str, text: String) -> Result<String> {
    let id = resolve_task_id(id_prefix, board)?;
    let entry = WalEntry {
        ts: now,
        event: WalEvent::Note {
            id: id.clone(),
            text,
        },
    };
    append(&entry)?;
    Ok(format!("note {}", short_id(&id)))
}

pub fn rm_task(now: DateTime<Utc>, board: &Board, id_prefix: &str) -> Result<String> {
    let id = resolve_task_id(id_prefix, board)?;
    let entry = WalEntry {
        ts: now,
        event: WalEvent::Delete { id: id.clone() },
    };
    append(&entry)?;
    Ok(format!("removed {}", short_id(&id)))
}

/// Parsed board command line (same semantics as `tw` subcommands, without the `tw` prefix).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardLineCmd {
    Add { title: String, tags: Vec<String> },
    Start { id: String },
    Done { id: String },
    Back { id: String },
    Edit { id: String, title: String },
    Note { id: String, text: String },
    Rm { id: String },
}

fn tokenize_line(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut chars = input.trim().chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == '"' {
            chars.next();
            let mut s = String::new();
            let mut closed = false;
            for ch in chars.by_ref() {
                if ch == '"' {
                    closed = true;
                    break;
                }
                s.push(ch);
            }
            if !closed {
                bail!("unclosed quote in command");
            }
            tokens.push(s);
        } else {
            let mut s = String::new();
            for ch in chars.by_ref() {
                if ch.is_whitespace() {
                    break;
                }
                s.push(ch);
            }
            tokens.push(s);
        }
    }
    Ok(tokens)
}

fn parse_tag_values(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Parse a line typed in the board command row (`tw` prefix optional).
pub fn parse_board_line(line: &str) -> Result<BoardLineCmd> {
    let tokens = tokenize_line(line)?;
    if tokens.is_empty() {
        bail!("empty command");
    }
    let mut i = 0;
    if tokens[0] == "tw" {
        i += 1;
        if i >= tokens.len() {
            bail!("expected subcommand after tw");
        }
    }
    let cmd = tokens[i].as_str();
    i += 1;
    match cmd {
        "ls" | "stats" | "log" | "board" => {
            bail!("use shell: tw {}", cmd);
        }
        "add" => {
            let mut title_parts: Vec<String> = Vec::new();
            while i < tokens.len() && tokens[i] != "-t" && tokens[i] != "--tag" {
                let t = &tokens[i];
                if t.starts_with('-') {
                    bail!("unexpected flag {:?}", t);
                }
                title_parts.push(t.clone());
                i += 1;
            }
            if title_parts.is_empty() {
                bail!("add requires a title");
            }
            let title = title_parts.join(" ");
            let mut tags: Vec<String> = Vec::new();
            while i < tokens.len() {
                let t = &tokens[i];
                if t == "-t" || t == "--tag" {
                    i += 1;
                    let Some(val) = tokens.get(i) else {
                        bail!("missing value for {}", tokens[i - 1]);
                    };
                    tags.extend(parse_tag_values(val));
                    i += 1;
                } else {
                    bail!("unexpected argument {:?}", t);
                }
            }
            Ok(BoardLineCmd::Add { title, tags })
        }
        "start" => {
            let id = tokens
                .get(i)
                .ok_or_else(|| anyhow!("start requires a task id prefix"))?
                .clone();
            if tokens.len() > i + 1 {
                bail!("too many arguments for start");
            }
            Ok(BoardLineCmd::Start { id })
        }
        "done" => {
            let id = tokens
                .get(i)
                .ok_or_else(|| anyhow!("done requires a task id prefix"))?
                .clone();
            if tokens.len() > i + 1 {
                bail!("too many arguments for done");
            }
            Ok(BoardLineCmd::Done { id })
        }
        "back" => {
            let id = tokens
                .get(i)
                .ok_or_else(|| anyhow!("back requires a task id prefix"))?
                .clone();
            if tokens.len() > i + 1 {
                bail!("too many arguments for back");
            }
            Ok(BoardLineCmd::Back { id })
        }
        "rm" => {
            let id = tokens
                .get(i)
                .ok_or_else(|| anyhow!("rm requires a task id prefix"))?
                .clone();
            if tokens.len() > i + 1 {
                bail!("too many arguments for rm");
            }
            Ok(BoardLineCmd::Rm { id })
        }
        "edit" => {
            let id = tokens
                .get(i)
                .ok_or_else(|| anyhow!("edit requires a task id prefix and title"))?
                .clone();
            i += 1;
            if i >= tokens.len() {
                bail!("edit requires a new title");
            }
            let title = tokens[i..].join(" ");
            Ok(BoardLineCmd::Edit { id, title })
        }
        "note" => {
            let id = tokens
                .get(i)
                .ok_or_else(|| anyhow!("note requires a task id prefix and text"))?
                .clone();
            i += 1;
            if i >= tokens.len() {
                bail!("note requires text");
            }
            let text = tokens[i..].join(" ");
            Ok(BoardLineCmd::Note { id, text })
        }
        _ => bail!("unknown command {:?}", cmd),
    }
}

/// Run a parsed board command against the current WAL (reload board before/after by caller).
pub fn run_board_line_cmd(
    cmd: BoardLineCmd,
    now: DateTime<Utc>,
    board: &Board,
) -> Result<String> {
    match cmd {
        BoardLineCmd::Add { title, tags } => add_task(now, title, tags),
        BoardLineCmd::Start { id } => start_task(now, board, &id),
        BoardLineCmd::Done { id } => done_task(now, board, &id),
        BoardLineCmd::Back { id } => back_task(now, board, &id),
        BoardLineCmd::Edit { id, title } => edit_task(now, board, &id, title),
        BoardLineCmd::Note { id, text } => note_task(now, board, &id, text),
        BoardLineCmd::Rm { id } => rm_task(now, board, &id),
    }
}

/// Parse and execute a board command line. Uses `board` for id resolution; caller should `replay()` first if needed.
pub fn execute_board_line(line: &str, now: DateTime<Utc>, board: &Board) -> Result<String> {
    let cmd = parse_board_line(line)?;
    run_board_line_cmd(cmd, now, board)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_add_quoted_and_tags() {
        let c = parse_board_line(r#"add "foo bar" -t a,b"#).unwrap();
        assert_eq!(
            c,
            BoardLineCmd::Add {
                title: "foo bar".to_string(),
                tags: vec!["a".to_string(), "b".to_string()],
            }
        );
    }

    #[test]
    fn parse_add_multiword_title() {
        let c = parse_board_line("add hello world").unwrap();
        assert_eq!(
            c,
            BoardLineCmd::Add {
                title: "hello world".to_string(),
                tags: vec![],
            }
        );
    }

    #[test]
    fn parse_tw_prefix() {
        let c = parse_board_line("tw start abcdef12").unwrap();
        assert_eq!(
            c,
            BoardLineCmd::Start {
                id: "abcdef12".to_string()
            }
        );
    }

    #[test]
    fn parse_edit_joins_title() {
        let c = parse_board_line("edit 01ABC my new title").unwrap();
        assert_eq!(
            c,
            BoardLineCmd::Edit {
                id: "01ABC".to_string(),
                title: "my new title".to_string(),
            }
        );
    }

    #[test]
    fn parse_note_joins_text() {
        let c = parse_board_line("note 01ABC hello there").unwrap();
        assert_eq!(
            c,
            BoardLineCmd::Note {
                id: "01ABC".to_string(),
                text: "hello there".to_string(),
            }
        );
    }

    #[test]
    fn read_only_rejected() {
        let e = parse_board_line("ls").unwrap_err();
        assert!(e.to_string().contains("use shell"));
    }

    #[test]
    fn resolve_task_id_ambiguous() {
        use crate::state::task::{Board, Task};
        use crate::wal::event::Column;
        use chrono::Utc;

        let t1 = Task {
            id: "0111111111111111".to_string(),
            title: "a".to_string(),
            tags: vec![],
            column: Column::Todo,
            notes: vec![],
            created_at: Utc::now(),
            started_at: None,
            done_at: None,
            created_day: "2026-04-10".to_string(),
        };
        let t2 = Task {
            id: "0111111122222222".to_string(),
            title: "b".to_string(),
            tags: vec![],
            column: Column::Todo,
            notes: vec![],
            created_at: Utc::now(),
            started_at: None,
            done_at: None,
            created_day: "2026-04-10".to_string(),
        };
        let board = Board {
            todo: vec![t1, t2],
            doing: vec![],
            done: vec![],
        };
        let err = resolve_task_id("01111111", &board).unwrap_err();
        assert!(err.to_string().contains("ambiguous"));
    }
}
