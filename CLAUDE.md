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

**Event-driven async loop** — `main.rs` is the orchestrator. It runs a `tokio::select!` loop over three sources: crossterm keyboard events, a `tokio::sync::mpsc` channel (`AppEvent`) for background task results, and a 100ms tick. All blocking work (file I/O, search, git, SQLite) is offloaded to `tokio::task::spawn_blocking`; async work (LLM HTTP streams) runs in spawned tasks. Results always come back over `AppEvent`.

**`AppState` (`src/app.rs`)** — single struct holding all UI state. `Mode` enum drives which widget handles input:
`Normal`, `Edit`, `Search`, `VectorSearch`, `Chat`, `Kanban`, `Git`, `Help`, `NewNote`, `GitCommitInput`, `ConfirmDelete`, `MeetilyImport`, `Settings`, `Summarize`, `FreshnessView`, `AnnotationPanel`, `KazamKbBrowser`.
Each overlay mode has a corresponding state struct (e.g. `MeetilyPanelState`, `FreshnessPanelState`, `AnnotationPanelState`, `KazamKbState`) held directly on `AppState`.

**TUI (`src/tui/`)** — `renderer.rs` dispatches to per-mode widgets in `widgets/`. `keys.rs` dispatches keyboard events to per-mode handler functions. Layout is computed in `layout.rs`.

**Notes (`src/notes/`)** — notes are plain `.md` files with YAML frontmatter.
- `frontmatter.rs` — parses/serializes `NoteFrontmatter`. Unknown keys are captured by `#[serde(flatten)] extra: HashMap<String, serde_yaml::Value>` so round-trips never drop custom fields.
- `freshness.rs` — staleness computation ported from Kazam. Hand-rolled JDN date math (no chrono). Reads `review_every` / `owner` / `expires` frontmatter fields.
- `annotations.rs` — Kazam-compatible sidecar YAML annotations stored at `.annotations/<slug>/` inside the notes directory. `note_slug()`, `load_annotations()`, `save_annotation()`, `count_pending()`.
- `markdown.rs` — renders body for the viewer pane.
- `watcher.rs` — recursive directory scan returning `Vec<FileNode>`.

**Kazam (`src/kazam/`)** — Kazam integration modules (Track A + B):
- `kb_browser.rs` — reads Kazam KB YAML files directly from `kazam.kb_path` (no subprocess). `scan_kb()` returns `Vec<KazamPage>`; `import_page()` writes to `notes_dir/<import_folder>/`.
- `mcp_client.rs` — blocking JSON-RPC 2.0 client wrapping a `kazam mcp --kb <path>` subprocess. Always call from `spawn_blocking`. Stored as `Option<Arc<Mutex<KazamMcpClient>>>` in `AppState`.

**Export (`src/export/`)** — `kazam.rs` converts a noterm note to Kazam-compatible YAML and writes it to `kazam.kb_path/<slug>.yaml`. Called via `E` key in Normal mode.

**Search (`src/search/`)** — `fulltext.rs` wraps Tantivy (FTS index at `~/.local/share/noterm/fts_index/`). `vector.rs` stores embeddings in SQLite (cosine similarity via dot product). Embeddings are generated through the LLM layer.

**LLM (`src/llm/`)** — `mod.rs` defines `LlmClient` and `EmbedClient` traits. Backends: `ollama.rs`, `claude.rs`, `openai.rs`. Chat uses streaming (`chat_stream` → `Pin<Box<dyn Stream<Item = Result<String>>>>`). `context.rs` builds the system prompt from open notes. `summarizer.rs` handles the `X`-key AI summary feature. Claude does **not** support embeddings; use Ollama or OpenAI for `embed_provider`.

**Database (`src/db/`)** — SQLite at `~/.local/share/noterm/noterm.db` for embeddings and import metadata. `migrations.rs` runs schema migrations on open.

**Import (`src/import/`)** — `watcher.rs` polls an inbox folder; `api.rs` runs an optional local Axum HTTP server (default port 7373); `meetily.rs` reads directly from Meetily's SQLite database.

**Git (`src/git/`)** — wraps `git2` (libgit2). All operations are blocking and run in `spawn_blocking`.

**Config (`src/config.rs`)** — TOML at `~/.config/noterm/config.toml`, auto-created on first run with defaults. Each feature area has its own config struct (`LlmConfig`, `MeetilyConfig`, `FreshnessConfig`, `KazamConfig`, etc.) added as `#[serde(default)]` fields on the root `Config`.

`KazamConfig` fields: `kb_path` (path to Kazam KB directory), `import_folder` (default: `"kazam"`), `binary_path` (default: `"kazam"` for MCP subprocess).

## How to add a new mode

This pattern has been established for every overlay (Meetily, Summarize, Freshness, etc.):

1. Add variant to `Mode` enum in `src/app.rs`
2. Add a `*PanelState` struct + `Default` impl, and a field on `AppState`
3. Add `AppEvent` variant(s) for async results; handle them in `AppState::handle_app_event`
4. Add `(mode_str, color)` arm in `status_bar.rs`
5. Add overlay render call in `renderer.rs` match
6. Add `Mode::NewMode => handle_new_mode(state, key)` in `keys.rs` dispatch
7. Create `src/tui/widgets/new_mode_panel.rs`; export from `widgets/mod.rs`
8. Add the trigger key in `handle_normal` in `keys.rs`, spawning the background task via `tokio::spawn` + `spawn_blocking` + `tx.send(AppEvent::...)`

## Kazam integration

### Track A — Tier 1: Freshness (complete)

Press `F` (Shift+F) in Normal mode to open the freshness dashboard. It scans all notes in the current file tree for `review_every` metadata and displays them sorted by staleness (Expired → Overdue → Due Soon → Fresh). Press Enter on any entry to open it; Esc closes the panel.

Staleness states: `Fresh`, `DueSoon { days_until_due }` (only for cadences > 30 days), `Overdue { days_overdue }`, `Expired { days_past_expiry }`. The status bar shows a live color-coded badge for the open note when it has `review_every` set.

Supported frontmatter fields (Kazam-compatible):
```yaml
review_every: 30d     # Nd · Nw · Nm · Ny · weekly · monthly · quarterly · yearly
owner: jordan
expires: 2026-12-31   # hard expiry date
sources_of_truth:
  - label: Notion doc
    href: https://notion.so/...
```

The computation logic lives entirely in `src/notes/freshness.rs` (`compute()`, `scan_paths()`). The `FreshnessConfig` in `config.toml` holds `default_owner` and `default_review_every`.

### Track A — Tier 2: Sidecar Annotations (complete)

Press `A` in Normal mode (with a note open) to open the annotations panel. Annotations are Kazam-compatible YAML files stored at `<notes_dir>/.annotations/<slug>/<id>.yaml`. Statuses: `pending`, `incorporated`, `ignored`, `stale`. Keys: `n`=new, `i`=incorporate, `d`=ignore, `j/k`=navigate, `Esc`=close.

The status bar shows an `A:<n>` badge when the open note has pending annotations. Count is cached in `state.annotation_pending_count` and updated by `open_note()` (blocking but fast). Logic is in `src/notes/annotations.rs`.

### Track A — Tier 3: Kazam KB Browser (complete)

Press `B` in Normal mode to open the Kazam KB browser (requires `kazam.kb_path` set in config). Reads Kazam YAML files directly — no subprocess. Type to filter; Enter imports a page as `notes_dir/kazam/<slug>.md`; Enter on an already-imported page opens it. Logic in `src/kazam/kb_browser.rs`.

### Track B — Tier 4: Side-by-side Kazam integration (complete)

- **`E`** — Export the open note to `<kazam.kb_path>/<slug>.yaml` in Kazam page format (requires `kazam.kb_path`). Kazam is responsible for rebuilding its static site from the YAML.
- **`M`** — Toggle Kazam MCP connection. Spawns `kazam mcp --kb <kb_path>` as a subprocess and performs the JSON-RPC initialize handshake (requires `kazam.binary_path`). The live `KazamMcpClient` is stored as `Option<Arc<Mutex<KazamMcpClient>>>` in `AppState`.
- **`Tab` in Chat** — Toggle Kazam KB context. When on, scans `kazam.kb_path` directly and injects page content into the chat system prompt via `llm::context::build_system_prompt()`. `context.rs` signature: `build_system_prompt(config, context_notes, kazam_pages)`.

## Summarizer feature

Press `X` (Shift+X) in Normal mode with a note open to generate an AI summary. The summary streams into a full-screen overlay. When generation completes, the content is automatically inserted into the note's `## Summary` section (or prepended as a new one if absent). The note is saved to disk immediately.

Settings are configured in the S panel under **Summarizer (X key)**: URL, API key, and model. These map to `[summarizer]` in `config.toml` (`SummarizerConfig`). The summarizer uses any OpenAI-compatible endpoint.

The summary insertion logic lives in `AppState::insert_summary_into_note` (`src/app.rs`) and the free function `inject_summary_into_body` in the same file. It locates the first `## Summary` heading (case-insensitive, must be at start of line) and replaces content up to the next `## ` sibling heading or EOF.

## Critical constraints

- **Never write to stdout** — it breaks the ratatui TUI. All logging goes to `~/.local/share/noterm/noterm.log` via `tracing-appender`. Use `tracing::debug!` / `tracing::error!`, never `println!` or `eprintln!`.
- `AppEvent::EmbedRequest` and `AppEvent::ForceReembed` are handled in `main.rs` before `app.handle_app_event` because they require direct `db` access not held by `AppState`.
- Search debounce (300ms) is managed in `main.rs`; the `Mode::Search` query change comparison lives in the event loop, not in `keys.rs`.
- `NoteFrontmatter` uses `#[serde(flatten)] extra` to absorb unknown YAML keys — always add new frontmatter fields as explicit typed fields on the struct, never rely on `extra` for reading them.
