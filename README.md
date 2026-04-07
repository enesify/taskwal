# TaskWAL (`tw`)

Local-first task tracker using an append-only JSONL write-ahead log (`~/.taskwal/wal.log` on Unix, `%USERPROFILE%\.taskwal\wal.log` on Windows).

## Build

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
cargo build --release
# binary: target/release/tw
```

## Cross-platform

- **macOS / Linux / Windows**: the same codebase; `crossterm` + `ratatui` work on common terminals.
- Build on each OS you ship for, or use cross-compilation (e.g. `cargo build --release --target x86_64-pc-windows-gnu` from a configured toolchain).

## Usage

Türkçe ayrıntılı kılavuz: [docs/KULLANIM.md](docs/KULLANIM.md).

```bash
tw add "Write API" --tag backend
tw start 01HX…      # prefix match if unique
tw done 01HX…
tw ls               # Todo/Doing: all open tasks; Done: today (local) by default
tw ls --all         # Done column: all completed tasks
tw ls --date 2026-04-01
tw stats
tw board
tw log              # raw WAL JSON lines
```

### Daily view rules

- **Todo / Doing:** every open task is listed (carry-over across days; nothing unfinished disappears from the default view).
- **Done:** by default, tasks completed on **today’s local calendar date**. Use `--all` or `--date` to widen.

## Environment

- **`TASKWAL_DIR`**: override the data directory (used by tests and for custom locations). WAL file: `$TASKWAL_DIR/wal.log`.

## License

MIT OR Apache-2.0
