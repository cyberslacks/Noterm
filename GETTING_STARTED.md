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
[notes]
notes_dir = "~/Documents/notes"
```
