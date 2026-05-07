# noterm — Architecture & Implementation Plan

A terminal-native Markdown note-taking app built in Rust.

---

## Vision

Notes live as plain `.md` files with YAML frontmatter. The app provides a full TUI with:
- A **markdown editor** (ratatui-textarea)
- **File explorer** for navigating notes
- **Full-text search** (Tantivy)
- **Semantic/vector search** (Ollama embeddings → SQLite cosine similarity)
- **LLM chat** over your notes (Ollama local + Claude + OpenAI)
- **Git integration** for sync and backup (git2)
- **Kanban board** driven by task YAML frontmatter

---

## Tech Stack

| Concern | Crate | Notes |
|---------|-------|-------|
| TUI framework | `ratatui 0.30` | Industry standard Rust TUI |
| Text editor widget | `ratatui-textarea 0.9` | Full editor widget for ratatui |
| Terminal backend | `crossterm 0.29` | Cross-platform raw mode |
| Async runtime | `tokio 1` | All IO/LLM calls off main thread |
| HTTP client | `reqwest 0.12` | rustls by default, no openssl-dev needed |
| Git | `git2 0.19` | libgit2 bindings |
| Full-text search | `tantivy 0.22` | Pure-Rust Lucene-style engine |
| Embedding + task DB | `rusqlite 0.31` (bundled) | SQLite, statically linked |
| Markdown parsing | `pulldown-cmark 0.11` | CommonMark compliant |
| Frontmatter parsing | `gray_matter 0.2` | YAML/TOML/JSON frontmatter |
| YAML serialization | `serde_yaml 0.9` | Frontmatter round-trips |
| Error handling | `anyhow 1` | Simple propagation |
| Logging | `tracing` + `tracing-appender` | File-only (stdout breaks TUI) |

---

## Source Layout

```
src/
├── main.rs                  # tokio::main, select! event loop, terminal init/teardown
├── app.rs                   # AppState, Mode enum, AppEvent, handle_app_event()
├── config.rs                # Config structs, load/write ~/.config/noterm/config.toml
├── error.rs                 # App error types
│
├── tui/
│   ├── mod.rs               # init(), restore(), panic hook (CRITICAL — must be first)
│   ├── layout.rs            # compute_layout() → LayoutChunks
│   ├── renderer.rs          # render() dispatches to all widget renderers
│   ├── keys.rs              # handle_event() maps keys → mode actions
│   └── widgets/
│       ├── file_tree.rs     # Left pane: navigable file/dir list
│       ├── editor.rs        # ratatui-textarea wrapper (Edit mode)
│       ├── viewer.rs        # Styled markdown viewer (Normal mode)
│       ├── status_bar.rs    # Bottom bar: mode, path, git branch, indicators
│       ├── search_overlay.rs   # Full-text search popup
│       ├── vector_search.rs    # Semantic search popup with scores
│       ├── chat_panel.rs    # LLM chat sidebar
│       ├── kanban.rs        # Four-column kanban board
│       ├── git_panel.rs     # Git popup: Status | Log | Diff tabs
│       └── help_popup.rs    # Key binding reference
│
├── notes/
│   ├── mod.rs               # Note struct, NoteManager, Note::from_path()
│   ├── frontmatter.rs       # Parse/write YAML frontmatter
│   ├── markdown.rs          # pulldown-cmark → ratatui Spans
│   └── watcher.rs           # Directory scan → Vec<FileNode> (spawn_blocking)
│
├── search/
│   ├── mod.rs               # Routes to FTS or vector search
│   ├── fulltext.rs          # Tantivy: index_note(), search(), delete_note()
│   └── vector.rs            # SQLite embedding store + cosine similarity
│
├── tasks/
│   ├── mod.rs               # Task, TaskStatus; parse from Note frontmatter
│   └── kanban.rs            # KanbanState, column logic, frontmatter write-back
│
├── git/
│   ├── mod.rs               # GitManager: status, log, stage, commit, push, pull
│   └── operations.rs        # All ops via spawn_blocking (git2 is not Send)
│
├── llm/
│   ├── mod.rs               # LlmClient trait + make_client() factory
│   ├── ollama.rs            # /api/chat (NDJSON streaming) + /api/embeddings
│   ├── claude.rs            # Anthropic Messages API (SSE streaming)
│   ├── openai.rs            # OpenAI-compatible API (SSE streaming)
│   └── context.rs           # Build system prompt with relevant notes
│
└── db/
    ├── mod.rs               # Connection, WAL mode, singleton Arc<Mutex<Connection>>
    └── migrations.rs        # Schema creation: embeddings + task_cache tables
```

---

## Application Modes

```
Normal ──e──────→ Edit      (focus editor)
Normal ──/──────→ Search    (full-text search overlay)
Normal ──v──────→ VecSearch (semantic search overlay)
Normal ──c──────→ Chat      (toggle chat panel, stays Normal for nav)
Normal ──K──────→ Kanban    (kanban board replaces main pane)
Normal ──G──────→ Git       (git panel overlay)
Normal ──?──────→ Help      (help popup)
Any    ──Esc────→ Normal    (auto-save if in Edit, close overlay otherwise)
Edit   ──Ctrl+s─→ (save, stay in Edit)
```

---

## TUI Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ NOTERM │ ~/notes/                                    [git:main ●] │  title bar (1 line)
├─────────────────┬────────────────────────────────────────────────┤
│ File Explorer   │ Editor (Edit) / Viewer (Normal)                 │
│ (20% width)     │                                                  │
│                 │ [when chat open: editor 65% | chat sidebar 35%] │
│ notes/          │                                                  │
│ ├ daily/        │ # My Note Title                                  │
│ │ └ 2026-05-06  │                                                  │
│ ├ projects/     │ Some **bold** content and *italic* text          │
│ └ README.md     │                                                  │
├─────────────────┴────────────────────────────────────────────────┤
│ [NORMAL] ~/notes/README.md    FTS● VEC● GIT●    14:32            │  status bar (1 line)
└──────────────────────────────────────────────────────────────────┘
```

Overlays rendered on top via `tui-popup`:
- `/` → full-text search popup
- `v` → vector search popup
- `G` → git panel (Status | Log tabs)
- `?` → help key binding table

---

## Key Bindings

| Key | Mode | Action |
|-----|------|--------|
| `q` / `Ctrl+q` | Any | Quit (checks unsaved changes) |
| `j`/`k` `↑`/`↓` | Normal | Navigate file tree |
| `Enter` | Normal | Open file / expand directory |
| `e` | Normal | Focus editor → Edit mode |
| `n` | Normal | New note (inline name prompt) |
| `/` | Normal | Full-text search |
| `v` | Normal | Vector/semantic search |
| `c` | Normal | Toggle LLM chat panel |
| `K` | Normal | Kanban board view |
| `G` | Normal | Git panel |
| `?` | Any | Help popup |
| `Esc` | Edit | Auto-save → Normal |
| `Ctrl+s` | Edit | Explicit save |
| `Enter` | Chat input | Send message |
| `h`/`l` | Kanban | Move between columns |
| `m` | Kanban | Move card to next column |
| `s`/`c`/`p`/`P` | Git | Stage / Commit / Push / Pull |

---

## Note Format (Frontmatter Schema)

```yaml
---
title: "Sprint 3 Planning"
id: "550e8400-e29b-41d4-a716-446655440000"
created: "2026-05-06T14:00:00Z"
modified: "2026-05-06T14:32:00Z"
tags: ["work", "planning"]
tasks:
  - id: "a1b2c3d4-..."
    title: "Design API endpoints"
    status: "done"          # todo | in_progress | done | blocked
    priority: 1             # 1 = highest
    due: "2026-05-10T00:00:00Z"
  - id: "e5f6g7h8-..."
    title: "Write tests"
    status: "in_progress"
    priority: 2
---

# Sprint 3 Planning

Note body in standard Markdown...
```

---

## SQLite Schema

```sql
-- Vector embeddings for semantic search
CREATE TABLE embeddings (
    note_id         TEXT PRIMARY KEY,
    note_path       TEXT NOT NULL,
    content_hash    TEXT NOT NULL,      -- SHA-256; skip re-embed if unchanged
    embedding       BLOB NOT NULL,      -- f32 LE bytes (768 dims = nomic-embed-text)
    embedding_model TEXT NOT NULL,
    dimension       INTEGER NOT NULL,
    indexed_at      INTEGER NOT NULL
);

-- Denormalized task cache for fast kanban queries
CREATE TABLE task_cache (
    task_id     TEXT PRIMARY KEY,
    note_path   TEXT NOT NULL,
    title       TEXT NOT NULL,
    status      TEXT NOT NULL,          -- todo | in_progress | done | blocked
    priority    INTEGER,
    due_at      INTEGER,                -- Unix timestamp
    tags        TEXT,                   -- JSON array
    created_at  INTEGER NOT NULL,
    modified_at INTEGER NOT NULL
);

PRAGMA journal_mode=WAL;               -- concurrent read+write without contention
```

Embeddings stored as raw `f32` little-endian bytes. Load all into memory for cosine similarity at query time (768-dim × 10K notes ≈ 30MB — acceptable).

---

## Async Architecture

The TUI event loop never blocks. All IO runs in spawned tasks that send `AppEvent` back via `mpsc::unbounded_channel`:

```
Main thread (tokio::select!):
  ├── crossterm events  → key handling → may spawn async tasks
  ├── AppEvent channel  → update AppState (chat chunks, search results, git status)
  └── tick (100ms)      → animation frames

Spawned tasks:
  ├── LLM streaming     → ChatChunk / ChatDone events per token
  ├── Git operations    → spawn_blocking (git2 is not Send, reconstruct Repo each call)
  ├── Embeddings        → spawn_blocking for Ollama HTTP + SQLite write
  ├── Directory scan    → spawn_blocking for large trees
  └── Tantivy indexing  → spawn_blocking for writes; reads inline after 150ms debounce
```

Critical: **all stdout/stderr goes to a log file** (`~/.local/share/noterm/noterm.log`) — any terminal output breaks the TUI.

---

## Mermaid Charts

Detect ` ```mermaid ` fenced blocks during markdown parse. Show `[Mermaid — press M to preview]` in status bar. `M` key writes a temp HTML file using the Mermaid CDN JS renderer and opens it via `xdg-open` (Linux) or `open` (macOS).

---

## Config File (`~/.config/noterm/config.toml`)

```toml
notes_dir = "~/notes"

[editor]
tab_width = 2
wrap_lines = true
auto_save = true

[git]
enabled = true
auto_commit = false
remote = "origin"

[llm]
provider = "ollama"          # ollama | claude | openai
ollama_base_url = "http://localhost:11434"
ollama_chat_model = "llama3.2"
ollama_embed_model = "nomic-embed-text"
# claude_api_key = "sk-ant-..."
# openai_api_key = "sk-..."
max_context_notes = 5

[search]
auto_index = true
embed_on_save = true

[ui]
file_tree_width_pct = 20
chat_width_pct = 35
```

---

## Implementation Phases

| # | Phase | Milestone |
|---|-------|-----------|
| 1 | Cargo setup, TUI init/teardown, event loop, Config | Binary starts, `q` exits cleanly |
| 2 | File tree scan, two-pane layout, Note loading, raw viewer | Navigate and open notes |
| 3 | ratatui-textarea editor, Edit mode, auto-save, new note | Create and edit notes |
| 4 | Tantivy FTS index, SQLite DB, search overlay | `/` full-text search works |
| 5 | pulldown-cmark → styled ratatui spans | Notes render with markdown formatting |
| 6 | git2 wrapper, git panel, stage/commit/push/pull | `G` panel, can commit and push |
| 7 | Task frontmatter, task cache, Kanban widget | `K` kanban board from notes |
| 8 | Ollama embeddings, SQLite store, vector search overlay | `v` semantic search works |
| 9 | LlmClient trait, Ollama/Claude/OpenAI streaming, chat panel | `c` chat panel with streaming |
| 10 | Help popup, theming, error display, `--notes-dir` CLI arg | Release build |

---

## Known Gotchas

1. **Panic hook** — restore terminal on panic or raw mode persists. Set in `tui/mod.rs` before anything else.
2. **`TextArea<'static>`** — load file into owned `String`, pass lines to `TextArea::new()`. Save with `textarea.lines().join("\n")`.
3. **git2 is not `Send`** — reconstruct `Repository::open()` inside every `spawn_blocking` closure.
4. **Tantivy `IndexWriter` is not `Send`** — wrap in `Arc<Mutex<Option<IndexWriter>>>`.
5. **Embedding model change** — detect via `embedding_model` column mismatch at startup; warn and re-embed all notes.
6. **Frontmatter round-trip** — serialize frontmatter + `\n---\n` + raw body string; never re-parse body when updating task status.
7. **Logs to file only** — `tracing-appender` to `~/.local/share/noterm/noterm.log`.
