---
name: taskwal-project-docs
description: Maintains TaskWAL user-facing documentation in English and Turkish and TUI screenshots when the CLI, board, or on-disk layout changes. Use when editing src/, adding or changing tw subcommands or TUI behavior, updating README or docs, or when the user asks to sync or refresh project documentation.
---

# TaskWAL project documentation

## Files to keep aligned

| Asset | Role |
| ----- | ---- |
| [README.md](../../../README.md) | Short overview, quick reference, links to full guides |
| [docs/USAGE.md](../../../docs/USAGE.md) | Full English user guide |
| [docs/KULLANIM.md](../../../docs/KULLANIM.md) | Full Turkish user guide (mirror of USAGE) |
| [docs/images/](../../../docs/images/) | `board.png`, `stats.png` for README and guides |

## When to update

1. **CLI change** (`src/main.rs`, `src/commands.rs`): reflect new/changed subcommands, flags, or id rules in README (table or examples), then **both** USAGE and KULLANIM.
2. **TUI change** (`src/ui/`): update keyboard tables, command-line (`:`) behavior, and **refresh screenshots** if the visible layout or colors change materially.
3. **Data paths / env**: keep README, USAGE, and KULLANIM consistent (`TASKWAL_DIR`, WAL path).
4. **Analytics text** (`tw stats` output vs stats screen): keep USAGE/KULLANIM descriptions in sync with `src/` and [src/ui/stats.rs](../../../src/ui/stats.rs).

## Screenshots

- Regenerate stylized PNGs after meaningful UI changes:

  ```bash
  pip install --target .docgen_pillow pillow   # once; .docgen_pillow is gitignored
  PYTHONPATH=.docgen_pillow python3 scripts/generate_doc_screenshots.py
  ```

- Commit updated files under `docs/images/`. The script draws an approximation of the TUI; replace with real terminal captures if you need pixel-perfect fidelity.

## Checklist

- [ ] README links to USAGE and KULLANIM
- [ ] English and Turkish guides describe the same commands and keys
- [ ] Image paths in markdown use `images/...` from `docs/*.md` and `docs/images/...` from README root
- [ ] No drive-by edits to unrelated markdown
