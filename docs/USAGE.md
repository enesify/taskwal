# TaskWAL (`tw`) — User guide

This document describes the current CLI and TUI for this project.

## What is TaskWAL?

TaskWAL is a **local-first** task tracker. Every change, including deletes, is appended to a **write-ahead log** (WAL) file. The app **replays** that log to build the current board (Todo / Doing / Done).

## Data location

| Platform    | Default directory        | WAL file  |
| ----------- | ------------------------ | --------- |
| macOS/Linux | `~/.taskwal/`            | `wal.log` |
| Windows     | `%USERPROFILE%\.taskwal\` | `wal.log` |

Override with environment variable:

- **`TASKWAL_DIR`:** data directory. WAL path: `$TASKWAL_DIR/wal.log` (on Windows, `%TASKWAL_DIR%\wal.log`).

## Install and the `tw` command

After building the project:

```bash
cargo install --path /path/to/taskwal --force
```

This usually installs `tw` to `~/.cargo/bin/tw` (Windows: `%USERPROFILE%\.cargo\bin\tw.exe`). If `cargo` is not on your PATH:

```bash
. "$HOME/.cargo/env"   # macOS/Linux
```

Re-run `cargo install --path ... --force` after updates.

Help:

```bash
tw --help
tw <command> --help
```

---

## Daily view rules (`ls` and `board`)

**Todo** and **Doing:** list **all** incomplete tasks, regardless of creation day. Open work carries over to the next day.

**Done** column:

- **Default:** tasks completed on the selected **local calendar day**. If you do not pass a date, that day is **today**.
- **`--all`:** all completed tasks (history included); Todo/Doing still show all open work.
- **`--date YYYY-MM-DD`:** Done column filtered to completions on that local day.

`--all` and **`--date`** cannot be used together.

---

## Commands

### `tw add <title>`

Creates a task (Todo by default). Creation day is the current local date.

```bash
tw add "Draft API"
tw add "Review" --tag backend,urgent
```

- **`--tag` / `-t`:** comma-separated tags.

### `tw start <id>`

Moves the task to **Doing**. The first move to Doing records a **start** time.

### `tw done <id>`

Moves the task to **Done** and records completion time.

### `tw back <id>`

Moves one step backward: **Done → Doing → Todo** (when allowed).

### `tw edit <id> <new title>`

Renames the task.

### `tw note <id> <text>`

Appends a note (previous notes are kept).

### `tw rm <id>`

Removes the task from the board (a delete event is appended to the WAL).

### Task id (`id`)

Full ULID or a **unique prefix** (e.g. first 8 characters). Ambiguous or missing matches are errors. An empty prefix is not allowed.

### `tw ls`

Text view of Todo / Doing / Done for the current view.

```bash
tw ls
tw ls --all
tw ls --date 2026-04-01
```

### `tw board`

Full-screen **Kanban** TUI (Ratatui). Same view options as `ls`:

```bash
tw board
tw board --all
tw board --date 2026-04-01
```

![Board TUI](images/board.png)

The **focused column** highlights its border. The **selected row** uses the **same accent color as that column** (yellow / blue / green) as its background, with bold text (black on yellow and green, white on blue) so the cursor stays visible.

Above the **command** box, a one-line **status** strip shows the **selected task** (id, title, tags, and notes) when you are not viewing command output, so you can read fields that are clipped in the column list; very long lines are truncated to the terminal width. After you run a `:` command, that strip shows the result or error until you move with **Tab**, arrows, or another navigation key.

**Board keys:**

| Key       | Action |
| --------- | ------ |
| `Tab`     | Next column (Todo → Doing → Done) |
| `↑` / `↓` | Move selection in the current column |
| `s`       | Move selected task to Doing |
| `d`       | Move selected task to Done |
| `b`       | Move selected task one column back |
| `a`       | Toggle Done column: today vs all completed |
| `g`       | Open stats screen |
| `q`       | Quit |
| `:`       | Command line (see below) |
| `Esc`     | Cancel command line |

After **`s`**, **`d`**, or **`b`**, the same task stays selected and highlighted in its new column (if it is still visible in the current view).

**Command line (`:`):** type a line such as `add My task` or `start 01ABC123` (optional `tw` prefix). Press **Enter** to run, **Esc** to cancel. Use **←** / **→** to move the cursor while editing.

**Stats screen:**

| Key        | Action        |
| ---------- | ------------- |
| `g`, `Esc` | Back to board |
| `q`        | Quit app      |

New tasks can also be added from the shell with `tw add "…"`.

### `tw stats` (text output)

Prints **all-time** aggregates from completed tasks:

- Total completed count  
- Average **cycle** time (days): first start or creation → completion  
- Average **lead** time (days): creation → completion  
- Average completed tasks per **active** day  
- Local **streak** (consecutive days)  
- Short recent-day breakdown  

The full-screen stats view inside `tw board` looks like this:

![Stats screen](images/stats.png)

### `tw log`

Prints raw WAL lines as JSON (backup or debugging).

---

## Example session

```bash
tw add "Morning standup"
tw add "PR review" --tag code

tw ls
tw start 01ABC123
tw done 01ABC123

tw stats
tw board
```

---

## Windows notes

- **Build:** `cargo build --release` produces `target\release\tw.exe` for distribution.
- **Terminal:** Windows Terminal or another modern console is recommended; the TUI works in most environments.

---

## License

As stated in `Cargo.toml` (e.g. MIT OR Apache-2.0).
