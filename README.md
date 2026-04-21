# TaskWAL (`tw`)

**TaskWAL** is a **local-first**, Kanban-style task tracker for the terminal. Every change is appended to a **write-ahead log** (JSON lines on disk); the tool **replays** that log to build your **Todo**, **Doing**, and **Done** columns—no server, no account, no cloud sync.

![TaskWAL board (TUI)](docs/images/board.png)

The default data file is `wal.log` under `~/.taskwal/` on Unix and `%USERPROFILE%\.taskwal\` on Windows.

![TaskWAL stats (text output)](docs/images/stats.png)

## Why TaskWAL?

- **Offline and yours:** Tasks live on your machine. There is no hosted backend or login—just a directory and a log file.
- **Transparent history:** The WAL is **append-only**. Deletes and edits are **events** in the log, not silent overwrites. Inspect or script against the stream with `tw log`.
- **Terminal-native:** Small CLI for day-to-day moves, plus an optional full-screen **board** (`tw board`) with `ratatui` for keyboard-driven review.
- **Daily focus:** The **Done** column defaults to what you finished **today** (local calendar date), so `tw ls` and `tw board` stay oriented around “what I closed this day” unless you widen the view (`--all`, `--date`).

## Use cases

- **Personal inbox + WIP limits:** Capture work in Todo, pull a few items into Doing, close them to Done when finished.
- **Developer workflow:** Stay in the shell next to git and builds; no context switch to a web app.
- **Travel or locked-down networks:** Full functionality without connectivity.
- **Automation and backup:** JSONL lines are easy to grep, pipe, or archive; replay semantics stay predictable.

## Features at a glance

- Stable **ULID** task identifiers (prefix matching when unique)
- **Tags**, **notes**, rename, column moves, and **append-only** removal
- **`tw ls`** (columns in the terminal) and **`tw board`** (interactive TUI) with the same view flags
- **`tw stats`** for aggregate counts and **`tw log`** for raw WAL output
- Optional **`TASKWAL_DIR`** to relocate the data directory

For every subcommand, flag, and TUI key, see the full guides: **[docs/USAGE.md](docs/USAGE.md)** (English) and **[docs/KULLANIM.md](docs/KULLANIM.md)** (Türkçe).

## Installation

### Prebuilt binaries

You do **not** need Rust or Cargo to run TaskWAL. Prebuilt `tw` binaries are attached to [GitHub Releases](https://github.com/enesify/taskwal/releases) for:

- Linux x86_64 (`x86_64-unknown-linux-gnu`, typical glibc-based distros)
- macOS Intel and Apple Silicon (`x86_64-apple-darwin`, `aarch64-apple-darwin`)
- Windows x86_64 (`x86_64-pc-windows-msvc`)

**Linux / macOS — install script** (default install dir: `~/.local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/enesify/taskwal/master/scripts/install-tw.sh | bash
```

System-wide install (e.g. `/usr/local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/enesify/taskwal/master/scripts/install-tw.sh | TASKWAL_PREFIX=/usr/local bash
```

Optional: `TASKWAL_REPO=owner/repo`, `TASKWAL_VERSION=v0.1.0` to pin a release.

**Windows — manual install:** download `taskwal-<tag>-x86_64-pc-windows-msvc.zip` from Releases, verify `SHA256SUMS`, extract `tw.exe`, and place it on your `PATH`.

**Manual (any platform):** download the matching `taskwal-<tag>-<target>.tar.gz` or `.zip`, verify checksums in `SHA256SUMS`, extract the `tw` binary, and move it to a directory on your `PATH`.

**Maintainers:** create a release by pushing a git tag `vX.Y.Z` that matches the `version` in [`Cargo.toml`](Cargo.toml) (for example tag `v0.1.0` for `version = "0.1.0"`). The release workflow builds and uploads archives plus `SHA256SUMS`.

### Build from source

Requires [Rust](https://rustup.rs/) 1.70+.

**Build** (binary at `target/release/tw`):

```bash
cargo build --release
```

**Install** into Cargo’s bin directory (e.g. `~/.cargo/bin/tw` on Unix):

```bash
cargo install --path . --force
```

Re-run `cargo install` after pulling updates. If `cargo` is not on your `PATH`, see [docs/USAGE.md](docs/USAGE.md) for shell setup hints.

## Cross-platform

- **macOS / Linux / Windows:** Same codebase; `crossterm` + `ratatui` work on common terminals.
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

- **`TASKWAL_DIR`:** Override the data directory (tests and custom locations). WAL file: `$TASKWAL_DIR/wal.log` on Unix; `%TASKWAL_DIR%\wal.log` on Windows.

## License

MIT OR Apache-2.0
