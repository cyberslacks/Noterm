# Getting Started with noterm

noterm is a single binary with no system dependencies. Download, make executable, and run.

---

## macOS

### Download

Go to the [Releases page](https://github.com/cyberslacks/Noterm/releases/latest) and download:

- **`noterm-macos-universal.tar.gz`** — recommended, works on both Apple Silicon (M1/M2/M3/M4) and Intel Macs

### Install

```bash
tar -xzf noterm-macos-universal.tar.gz
chmod +x noterm-macos-universal
sudo mv noterm-macos-universal /usr/local/bin/noterm
```

### Clear the quarantine flag

macOS will block an unsigned binary the first time you run it. Clear the flag before moving it:

```bash
xattr -dr com.apple.quarantine noterm-macos-universal
```

Or after moving it:

```bash
xattr -dr com.apple.quarantine /usr/local/bin/noterm
```

Alternatively, right-click the file in Finder → **Open** → **Open Anyway** the first time.

### Run

```bash
noterm
```

noterm creates `~/notes/` and `~/.config/noterm/config.toml` on first launch.

---

## Linux

### Download

Go to the [Releases page](https://github.com/cyberslacks/Noterm/releases/latest) and download:

- **`noterm-linux-x86_64.tar.gz`** — fully static binary, runs on any x86_64 Linux distro with no dependencies

### Install

```bash
tar -xzf noterm-linux-x86_64.tar.gz
chmod +x noterm-linux-x86_64
sudo mv noterm-linux-x86_64 /usr/local/bin/noterm
```

### Run

```bash
noterm
```

---

## Windows

### Download

Go to the [Releases page](https://github.com/cyberslacks/Noterm/releases/latest) and download:

- **`noterm-windows-x86_64.exe.zip`**

### Install

Extract the zip and place `noterm-windows-x86_64.exe` wherever you like (e.g. `C:\Tools\noterm.exe`). Add that folder to your `PATH` via **System Properties → Environment Variables**.

### Run

Open **Windows Terminal** (recommended) or PowerShell and run:

```powershell
noterm
```

> noterm is a TUI application — it requires a proper terminal emulator. The old `cmd.exe` works but Windows Terminal gives a much better experience.

---

## First Launch

On first run noterm will:

1. Create `~/notes/` (your notes directory)
2. Write a default config at `~/.config/noterm/config.toml`

The TUI opens with a file tree on the left and a note viewer/editor on the right.

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate the file tree |
| `Enter` | Open a note |
| `n` | Create a new note |
| `e` | Edit the open note |
| `Esc` | Save and return to Normal |
| `q` | Quit |
| `?` | Full key binding reference |

---

## LLM / AI features (optional)

The chat panel (`c`), vector search (`v`), and semantic indexing are all optional and only activate when an LLM provider is configured.

### Local — Ollama (no API key needed)

```bash
ollama pull llama3.2           # chat
ollama pull nomic-embed-text   # embeddings (for vector search)
ollama serve
```

```toml
# ~/.config/noterm/config.toml
[llm]
provider = "ollama"
ollama_chat_model = "llama3.2"
ollama_embed_model = "nomic-embed-text"
```

### Claude

```toml
[llm]
provider = "claude"
claude_api_key = "sk-ant-..."
claude_model = "claude-sonnet-4-5"
```

### OpenAI

```toml
[llm]
provider = "openai"
openai_api_key = "sk-..."
openai_model = "gpt-4o"
```

---

## AI Summarizer (optional)

Press `X` (Shift+X) with a note open to generate an AI summary. The summary streams in a full-screen overlay and is automatically inserted into the note's `## Summary` section when complete.

Configure in Settings (`S` key) under **Summarizer**, or directly in `config.toml`:

```toml
[summarizer]
base_url = "http://localhost:3000/api"   # any OpenAI-compatible endpoint
model = "llama3.2"
api_key = ""                             # leave empty for local endpoints
```

---

## Freshness tracking (optional)

Track when notes need review by adding a `review_every` field to a note's YAML frontmatter:

```yaml
---
title: "Architecture decisions"
review_every: 30d        # Nd · Nw · Nm · Ny · monthly · quarterly · yearly
owner: jordan
expires: 2026-12-31      # optional hard expiry date
---
```

The status bar shows a live badge (`FRESH` / `DUE IN Nd` / `OVERDUE Nd` / `EXPIRED Nd`) for the open note. Press `F` (Shift+F) to open the freshness dashboard — a sorted list of all notes with staleness metadata, worst-first.

Default owner and cadence can be set in `config.toml`:

```toml
[freshness]
default_owner = "jordan"
default_review_every = "30d"
```

---

## Sidecar annotations (optional)

Add non-destructive annotations to any note without editing its content. Annotations are Kazam-compatible YAML files stored alongside your notes in `.annotations/<note-slug>/`.

Press `A` in Normal mode (with a note open) to open the annotations panel:

| Key | Action |
|-----|--------|
| `n` | Write a new annotation |
| `i` | Mark focused annotation as incorporated |
| `d` | Mark focused annotation as ignored |
| `j` / `k` | Navigate |
| `Esc` | Close panel |

The status bar shows `A:<n>` when the open note has pending annotations.

---

## Kazam integration (optional)

noterm integrates with [Kazam](https://github.com/tdiderich/kazam), a local AI-native knowledge base engine. All Kazam features require `kazam.kb_path` to be set in `config.toml`:

```toml
[kazam]
kb_path = "~/Documents/my-kb"    # path to your Kazam KB directory
import_folder = "kazam"          # subfolder in notes_dir for imported pages
binary_path = "kazam"            # path to the kazam binary (for MCP)
```

### KB Browser (`B`)

Browse and import Kazam KB pages directly into noterm — no Kazam install required, reads YAML files directly.

Press `B` to open the browser. Type to filter by title, `Enter` to import a page as a markdown note (or open it if already imported), `Esc` to close.

### Export to Kazam (`E`)

Press `E` to export the open note as a Kazam-compatible YAML page written to `<kb_path>/<slug>.yaml`. Kazam picks it up automatically when it rebuilds its static site.

### MCP connection (`M`)

Press `M` to toggle a live connection to `kazam mcp --kb <kb_path>`. When connected, noterm can use Kazam's MCP tools for agent operations. Requires the `kazam` binary on your PATH (or set `binary_path`).

### Chat KB context (`Tab` in chat)

While in the chat panel, press `Tab` to inject all Kazam KB pages into the chat system prompt. The panel border turns cyan and shows `[KB:ON]`. Press `Tab` again to clear the context.

---

## Git sync (optional)

To sync notes across machines, point `notes_dir` at a git repository and noterm will show live status in the title bar. Use `G` to open the git panel and stage, commit, and push without leaving the TUI.

```bash
cd ~/notes
git init
git remote add origin https://github.com/yourname/notes.git
```

---

## Change the notes directory

Edit `~/.config/noterm/config.toml`:

```toml
notes_dir = "~/Documents/notes"
```
