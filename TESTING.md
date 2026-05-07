# noterm — Feature Testing Guide

## Prerequisites

```bash
# Build the binary
cargo build

# Create a test notes directory with some sample notes
mkdir -p ~/notes/daily ~/notes/projects
echo '---
title: "Sprint Planning"
id: "test-001"
created: "2026-05-01T10:00:00Z"
modified: "2026-05-01T10:00:00Z"
tags: ["work", "planning"]
tasks:
  - id: "task-001"
    title: "Design API endpoints"
    status: "todo"
    priority: 1
  - id: "task-002"
    title: "Write unit tests"
    status: "in_progress"
    priority: 2
  - id: "task-003"
    title: "Deploy to staging"
    status: "done"
    priority: 1
---

# Sprint Planning

## Goals

Ship v1 by end of quarter.

## Notes

- Team sync every Tuesday
- Blockers go to Slack #blockers
' > ~/notes/projects/sprint.md

echo '---
title: "Daily 2026-05-07"
id: "test-002"
created: "2026-05-07T09:00:00Z"
modified: "2026-05-07T09:00:00Z"
tags: ["daily"]
---

# Daily 2026-05-07

Standup notes for today.

## Done yesterday

- Reviewed PR #42
- Fixed login bug

## Today

- Finish the vector search integration
- Code review for team
' > ~/notes/daily/2026-05-07.md

echo '---
title: "Architecture Overview"
id: "test-003"
created: "2026-05-06T14:00:00Z"
modified: "2026-05-06T14:00:00Z"
tags: ["tech", "architecture"]
---

# Architecture Overview

The system uses a microservices approach with an API gateway.

## Components

- **Auth service** — handles login, tokens
- **Data service** — reads/writes to Postgres
- **Notification service** — sends emails and Slack messages

## Deployment

All services run in Kubernetes. Helm charts live in `infra/`.
' > ~/notes/projects/architecture.md

# (Optional) Init a git repo in notes dir for git features
cd ~/notes && git init && git add . && git commit -m "initial notes"
```

---

## 1. Launch & Basic Navigation

```bash
cargo run
```

**Expected:** TUI opens with file tree on the left, blank viewer on the right.

| Key | Expected behaviour |
|-----|--------------------|
| `j` / `↓` | Cursor moves down the file tree |
| `k` / `↑` | Cursor moves up the file tree |
| `Enter` | Opens the selected note in the viewer |
| `PageDown` / `PageUp` | Scrolls the viewer |
| `q` | Quits cleanly — terminal restored to normal |
| `Ctrl+q` | Quits from any mode |

**Check:** Status bar at the bottom shows `NORMAL` in green with the current note path.

---

## 2. Note Viewer (Markdown Rendering)

Open `projects/sprint.md`.

**Expected:** The note renders with styled markdown:
- `# Sprint Planning` appears in **Cyan/Bold**
- `## Goals` in Blue/Bold
- `**Ship v1**` in Bold
- Bullet list items rendered correctly

Open `projects/architecture.md`.

**Expected:** Inline code `` `infra/` `` renders in a distinct colour.

---

## 3. Create a New Note

| Step | Action | Expected |
|------|--------|----------|
| 1 | Press `n` | Popup appears: "New Note Name" |
| 2 | Type `test-note` | Prompt shows typed text |
| 3 | Press `Enter` | File created, editor opens in `INSERT` mode |
| 4 | Type some content | Text appears in the editor |
| 5 | Press `Esc` | Returns to `NORMAL`, note saved to `~/notes/test-note.md` |

**Verify:** `cat ~/notes/test-note.md` — should have YAML frontmatter with `id`, `created`, `modified`.

---

## 4. Edit a Note

Open any note, then:

| Step | Action | Expected |
|------|--------|----------|
| 1 | Press `e` | Status bar switches to `INSERT` |
| 2 | Edit text | Changes visible in editor |
| 3 | Press `Ctrl+s` | Status bar briefly shows "saved" / mode stays INSERT |
| 4 | Press `Esc` | Auto-saves and returns to `NORMAL` |

**Verify:** `cat ~/notes/projects/sprint.md` — `modified` timestamp updated.

---

## 5. Delete a Note

| Step | Action | Expected |
|------|--------|----------|
| 1 | Select `test-note.md` in the tree | Note highlighted |
| 2 | Press `d` | Red popup: "DELETE?" with filename |
| 3 | Press `n` or `Esc` | Popup closes, nothing deleted |
| 4 | Press `d` again | Popup appears again |
| 5 | Press `y` | File deleted, tree refreshes, status shows "Deleted: test-note.md" |

**Verify:** `ls ~/notes/test-note.md` — file should not exist.

---

## 6. Full-Text Search

Index must be built first (auto-happens on first open or save). If results are empty, open and re-save a note to trigger indexing.

| Step | Action | Expected |
|------|--------|----------|
| 1 | Press `/` | Search overlay opens, status shows `SEARCH` |
| 2 | Type `kubernetes` | Results appear: "Architecture Overview" |
| 3 | Press `↑`/`↓` | Cursor moves through results |
| 4 | Press `Enter` | Note opens in viewer |
| 5 | Press `Esc` | Returns to Normal |

**Also test:** Search for a word that appears in multiple notes (e.g. `api`) — expect multiple results.

---

## 7. Vector / Semantic Search

Requires Ollama running with `nomic-embed-text` model:

```bash
ollama pull nomic-embed-text
ollama serve   # in a separate terminal
```

| Step | Action | Expected |
|------|--------|----------|
| 1 | Open and save a note (triggers embedding) | Status briefly shows background activity |
| 2 | Press `v` | Vector search overlay, status shows `VSEARCH` |
| 3 | Type a semantic query: `deployment infrastructure` | |
| 4 | Press `Enter` | Results with similarity scores, `architecture.md` near top |
| 5 | Press `Enter` on a result | Note opens |
| 6 | Press `Esc` | Returns to Normal |

---

## 8. LLM Chat

### With Ollama (default)

```bash
ollama pull llama3.2
ollama serve
```

| Step | Action | Expected |
|------|--------|----------|
| 1 | Open `sprint.md` | Note visible in viewer |
| 2 | Press `c` | Chat panel opens on the right, status shows `CHAT` |
| 3 | Type: `What are the action items in this note?` | Text appears in chat input |
| 4 | Press `Enter` | Message sent, loading indicator, LLM streams a response |
| 5 | Press `Ctrl+l` | Chat history cleared |
| 6 | Press `Esc` | Chat panel closes |

### With Claude (if configured)

Edit `~/.config/noterm/config.toml`:
```toml
[llm]
provider = "claude"
claude_api_key = "sk-ant-..."
```

Repeat chat steps above — response should come from Claude.

---

## 9. Kanban Board

| Step | Action | Expected |
|------|--------|----------|
| 1 | Press `K` | Kanban view replaces main pane, status shows `KANBAN` |
| 2 | View columns | `Todo`, `In Progress`, `Done`, `Blocked` columns |
| 3 | Check cards | Tasks from `sprint.md` frontmatter appear in the correct columns |
| 4 | Press `h`/`l` | Focus moves between columns |
| 5 | Press `j`/`k` | Cursor moves between cards within a column |
| 6 | Press `m` | Focused card moves to the next column |
| 7 | Press `Esc` | Returns to Normal |

**Verify:** After moving a card, open `sprint.md` — the `status` field in the task's frontmatter should be updated.

---

## 10. Git Panel

Requires `~/notes` to be a git repository (see Prerequisites).

| Step | Action | Expected |
|------|--------|----------|
| 1 | Modify a note, save it | File is dirty |
| 2 | Press `G` | Git panel opens, status shows `GIT`, Status tab loads |
| 3 | View Status tab | Modified file listed as unstaged |
| 4 | Press `Tab` | Switches to Log tab, shows recent commits |
| 5 | Press `s` | Stages all changes |
| 6 | Press `c` | Commit message prompt appears |
| 7 | Type a message, press `Enter` | Commit created, status shows "Git operation complete" |
| 8 | Press `p` | Pushes to remote (if configured) |
| 9 | Press `Esc` | Returns to Normal |

---

## 11. Help Popup

| Step | Action | Expected |
|------|--------|----------|
| 1 | Press `?` from any mode | Help popup with key bindings table |
| 2 | Verify `I` key listed | "Meetily import panel" entry present |
| 3 | Press `Esc` or `?` | Returns to previous mode |

---

## 12. Import — Inbox Folder Watcher

```bash
mkdir -p ~/notes/inbox
```

The watcher polls every 5 seconds by default.

```bash
# Drop a markdown file into the inbox while noterm is running
echo '# Meeting Notes

Quick notes from the call.' > ~/notes/inbox/call-notes.md
```

**Expected within ~5 seconds:**
- Status bar shows "Imported: call-notes.md"
- File tree refreshes, note appears in `~/notes/`
- Source file removed from inbox
- New note has proper YAML frontmatter

---

## 13. Import — REST API

Enable the API in `~/.config/noterm/config.toml`:
```toml
[import]
api_enabled = true
api_port = 7373
```

Restart noterm, then in another terminal:

```bash
# Health check
curl http://localhost:7373/api/health
# Expected: {"status":"ok"}

# Create a note via API
curl -X POST http://localhost:7373/api/notes \
  -H "Content-Type: application/json" \
  -d '{
    "title": "API Test Note",
    "body": "This note was created via the REST API.\n\n- Item 1\n- Item 2",
    "tags": ["api", "test"],
    "subfolder": "inbox"
  }'
# Expected: {"path":"...","status":"created"}

# Import a raw markdown string
curl -X POST http://localhost:7373/api/import \
  -H "Content-Type: application/json" \
  -d '{
    "filename": "imported.md",
    "content": "---\ntitle: \"Imported\"\n---\n\n# Imported Note\n\nHello from the API."
  }'
```

**Expected in noterm:** Status shows "Imported: …", file tree refreshes.

---

## 14. Meetily Import

Requires Meetily installed with at least one recorded meeting.

| Step | Action | Expected |
|------|--------|----------|
| 1 | Press `I` (uppercase) | Meetily panel opens over the viewer |
| 2 | Panel loads | List of meetings with date, duration, title |
| 3 | If "No meetings found" | Check DB path: `~/.local/share/com.meetily.ai/meeting_minutes.sqlite` |
| 4 | Press `j`/`k` | Navigates through meeting list |
| 5 | Press `Enter` | Selected meeting imported to `~/notes/calls/` |
| 6 | Status bar | Shows "Imported meeting: <filename>" |
| 7 | Meeting row | Gets `✓ imported` badge |
| 8 | Press `Esc` | Returns to Normal |

**Verify the imported note structure:**
```bash
ls ~/notes/calls/
cat ~/notes/calls/<meeting-title>.md
```

Expected sections in order:
1. YAML frontmatter with `source: "meetily"`, `meetily_id`, `tags: ["call","meeting","meetily"]`
2. `# Title` heading with `> Call Note · date · duration` blockquote
3. `## Summary` — AI-generated summary
4. `### Key Points` — bullet list (if available)
5. `### Action Items` — `- [ ]` checkboxes (if available)
6. `## Notes` — user notes written during the meeting (if any)
7. `## Transcript` — timestamped segments with `**[MM:SS]**` markers, `**Mic**`/`**System**` labels (if available)

**Custom DB path** (if Meetily is in a non-standard location):
```toml
# ~/.config/noterm/config.toml
[meetily]
db_path = "/path/to/meeting_minutes.sqlite"
import_folder = "calls"
tags = ["call", "meeting", "meetily"]
```

---

## 15. Config File

```bash
cat ~/.config/noterm/config.toml
```

Verify all sections are present: `[editor]`, `[git]`, `[llm]`, `[search]`, `[ui]`, `[import]`, `[meetily]`.

Test a config change:
```toml
[ui]
show_hidden = true
```

Restart noterm — dotfiles/directories should appear in the file tree.

---

## Quick Smoke Test (all features in ~5 minutes)

```
cargo run
  → j/k to navigate
  → Enter to open sprint.md
  → e → type something → Esc (edit + save)
  → n → "smoke-test" → Enter → type → Esc (new note)
  → d → y (delete smoke-test)
  → / → "kubernetes" → Enter (FTS search)
  → K → m → Esc (kanban move)
  → G → Tab → Esc (git panel)
  → ? → Esc (help)
  → I → j/k → Enter (meetily import, if available)
  → q (quit)
```
