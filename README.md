# TaskWAL (`tw`)

Local-first task tracker using an append-only JSONL write-ahead log (`~/.taskwal/wal.log` on Unix, `%USERPROFILE%\.taskwal\wal.log` on Windows).

![TaskWAL board (TUI)](docs/images/board.png)

## Build

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
cargo build --release
# binary: target/release/tw
```

## Cross-platform

- **macOS / Linux / Windows:** same codebase; `crossterm` + `ratatui` work on common terminals.
- Build on each OS you ship for, or use cross-compilation (e.g. `cargo build --release --target x86_64-pc-windows-gnu` from a configured toolchain).

## Documentation

- **English (full guide):** [docs/USAGE.md](docs/USAGE.md)
- **Türkçe (tam kılavuz):** [docs/KULLANIM.md](docs/KULLANIM.md)

## Usage (quick reference)

```bash
tw add "Write API" --tag backend
tw start 01HX…       # prefix match if unique
tw done 01HX…
tw back 01HX…       # one step back: Done→Doing or Doing→Todo
tw edit 01HX… "New title"
tw note 01HX… "Blocked on API"
tw rm 01HX…

tw ls               # Todo/Doing: all open tasks; Done: today (local) by default
tw ls --all         # Done column: all completed tasks
tw ls --date 2026-04-01

tw board            # interactive Kanban TUI (same view flags as ls)
tw stats            # aggregate stats (all-time), text output
tw log              # raw WAL JSON lines
```

### Commands

| Command | Purpose |
|--------|---------|
| `add <title>` | New task (Todo); `--tag` / `-t` comma-separated tags |
| `start <id>` | Move to Doing |
| `done <id>` | Move to Done |
| `back <id>` | Move one column backward |
| `edit <id> <title>` | Rename task |
| `note <id> <text>` | Append a note |
| `rm <id>` | Remove task (append-only delete event in WAL) |
| `ls` | Print board columns for the current view (`--all`, `--date`) |
| `board` | Full-screen TUI board (`--all`, `--date`) |
| `stats` | Print statistics |
| `log` | Dump WAL as JSON lines |

### Daily view rules

- **Todo / Doing:** every open task is listed (carry-over across days).
- **Done:** by default, tasks completed on **today’s local calendar date**. Use `--all` or `--date` to widen.

## Environment

- **`TASKWAL_DIR`:** override the data directory (used by tests and for custom locations). WAL file: `$TASKWAL_DIR/wal.log`.

## License

MIT OR Apache-2.0
