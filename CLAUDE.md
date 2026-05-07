# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                        # debug build
cargo build --release              # release build (opt-level 3, thin LTO, stripped)
cargo run                          # run in debug mode
cargo test                         # run all tests
cargo test <test_name>             # run a single test
cargo clippy                       # lint
cargo fmt                          # format

# Linux static binary (requires musl-tools)
cargo build --release --target x86_64-unknown-linux-musl
```

Releases are published by pushing a version tag (`git tag v0.1.0 && git push --tags`). CI builds Linux (musl static), macOS arm64, macOS x86_64, and Windows x86_64, then creates a universal macOS binary via `lipo`.

No system packages are required — libgit2, libsqlite3, and TLS are all compiled from source via Cargo features (`vendored-libgit2`, `vendored-openssl`, `bundled`).

## Architecture

**Event-driven async loop** — `main.rs` is the orchestrator. It runs a `tokio::select!` loop that handles three sources: crossterm keyboard events, a `tokio::sync::mpsc` channel (`AppEvent`) for results from background tasks, and a 100ms tick. All blocking work (file I/O, search, git, SQLite) is offloaded to `tokio::task::spawn_blocking`; async work (LLM HTTP streams, git ops) runs directly in spawned tasks. Results come back over the `AppEvent` channel.

**`AppState` (`src/app.rs`)** — single struct holding all UI state. `Mode` enum drives which widget handles input: `Normal`, `Edit`, `Search`, `VectorSearch`, `Chat`, `Kanban`, `Git`, `Help`, `NewNote`, `GitCommitInput`, `ConfirmDelete`, `MeetilyImport`, `Settings`.

**TUI (`src/tui/`)** — `renderer.rs` dispatches to per-mode widgets in `widgets/`. `keys.rs` dispatches keyboard events to per-mode handlers. Layout is computed in `layout.rs`.

**Notes (`src/notes/`)** — notes are plain `.md` files with YAML frontmatter. `frontmatter.rs` parses/serializes; `markdown.rs` renders for the viewer; `watcher.rs` scans the filesystem for the file tree.

**Search (`src/search/`)** — `fulltext.rs` wraps Tantivy (FTS index in `~/.local/share/noterm/fts_index/`). `vector.rs` stores embeddings in SQLite (cosine similarity via dot product). Embeddings are generated through the LLM layer.

**LLM (`src/llm/`)** — `mod.rs` defines `LlmClient` and `EmbedClient` traits. Backends: `ollama.rs`, `claude.rs`, `openai.rs`. Chat uses streaming (`chat_stream` → `Pin<Box<dyn Stream<Item = Result<String>>>>`). `context.rs` builds the system prompt from open notes. Claude does **not** support embeddings; use Ollama or OpenAI for `embed_provider`.

**Database (`src/db/`)** — SQLite at `~/.local/share/noterm/noterm.db` for embeddings and import metadata. `migrations.rs` runs schema migrations on open.

**Import (`src/import/`)** — `watcher.rs` polls an inbox folder; `api.rs` runs an optional local Axum HTTP server (default port 7373); `meetily.rs` reads directly from Meetily's SQLite database.

**Git (`src/git/`)** — wraps `git2` (libgit2). All operations are blocking and run in `spawn_blocking`.

**Config (`src/config.rs`)** — TOML at `~/.config/noterm/config.toml`, auto-created on first run with defaults.

## Summarizer feature

Press `X` (Shift+X) in Normal mode with a note open to generate an AI summary. The summary streams into a full-screen overlay. When generation completes, the content is automatically inserted into the note's `## Summary` section (or prepended as a new one if absent). The note is saved to disk immediately.

Settings are configured in the S panel under **Summarizer (X key)**: URL, API key, and model. These map to `[summarizer]` in `config.toml` (`SummarizerConfig`). The summarizer uses any OpenAI-compatible endpoint (OpenWebUI, Ollama, etc.) and a hardcoded system prompt in `src/llm/summarizer.rs`.

The summary insertion logic lives in `AppState::insert_summary_into_note` (`src/app.rs`) and the free function `inject_summary_into_body` in the same file. It locates the first `## Summary` heading (case-insensitive, must be at start of line) and replaces content up to the next `## ` sibling heading or EOF.

## Critical constraints

- **Never write to stdout** — it breaks the ratatui TUI. All logging goes to `~/.local/share/noterm/noterm.log` via `tracing-appender`. Use `tracing::debug!` / `tracing::error!`, never `println!` or `eprintln!`.
- The `AppEvent::EmbedRequest` and `AppEvent::ForceReembed` variants are handled specially in `main.rs` (before `app.handle_app_event`) because they require direct `db` access not held by `AppState`.
- Search debounce (300ms) is managed in `main.rs`; the `Mode::Search` query change comparison lives in the event loop, not in `keys.rs`.
