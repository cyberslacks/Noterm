# Feature Backlog

## F-1 · Folder/Root-Wide Task Aggregation in Kanban

**Goal:** The Kanban board shows tasks aggregated from all notes in the current folder (or all notes at the vault root), not just the open note. Moving a card writes the status change back to the source note's frontmatter.

### Current state
- `Task` struct (src/tasks/mod.rs) has a `note_path` field.
- `NoteFrontmatter` has `tasks: Option<Vec<Task>>` — tasks are already stored per-note.
- Kanban columns are populated at load time from whatever the current note exposes.
- No mechanism exists to scan multiple notes and merge their task lists.

### Tasks

#### F-1-1  Folder task loader
- Add `load_tasks_from_folder(folder: &Path) -> Vec<Task>` in `src/tasks/mod.rs` (or a new `src/tasks/loader.rs`).
- Walk the folder tree with `walkdir` (already in the tree for `watcher.rs`) and read each `.md` frontmatter.
- Skip notes whose frontmatter has no `tasks` key.
- Return all tasks with `note_path` set to the source file.

#### F-1-2  Inline checkbox parsing
- Parse `- [ ] title` and `- [x] title` lines from note *bodies* (not just frontmatter).
- Map `- [ ]` → `TaskStatus::Todo`, `- [x]` → `TaskStatus::Done`.
- Tag parsed tasks with `tags: ["inline"]` so they are distinguishable from frontmatter tasks.
- Merge with frontmatter tasks; deduplicate by title + note_path.

#### F-1-3  Kanban scope selector
- Add `KanbanScope` enum to `AppState`: `CurrentNote`, `CurrentFolder`, `Vault`.
- Expose a toggle key in Kanban mode (e.g., `s`) to cycle through scopes.
- Display active scope in the Kanban status bar.

#### F-1-4  Async task loading
- Fire `AppEvent::LoadKanbanTasks(scope)` from the scope toggle and on Kanban mode entry.
- Handle in `main.rs` with `spawn_blocking` (reads frontmatter of N files).
- On completion send `AppEvent::KanbanTasksLoaded(Vec<Task>)` back and rebuild `KanbanState`.

#### F-1-5  Write-back on card move
- When a card is moved between columns, identify the source note by `task.note_path`.
- Read the note, update the matching task's `status` in frontmatter, write back atomically.
- Fire `AppEvent::NoteModified` so the file watcher sees the change.
- For *inline* tasks, rewrite the checkbox line in the body (`- [ ]` ↔ `- [x]`).

#### F-1-6  Card source attribution
- Show note filename (basename without extension) as a sub-label on each Kanban card.
- Keep it dimmed so it doesn't compete with the title.

### Technical notes
- Folder-scope load should be debounced / cached — re-run only when the file watcher reports a `.md` change inside the folder.
- Inline task IDs are ephemeral (generated each load); use `title + note_path` as the stable key for write-back matching.

---

## F-2 · Summarizer Always Produces Tasks

**Goal:** Every AI summary — whether of a meeting transcript or a regular work note — ends with a `## Tasks` section containing both explicit and inferred tasks. Extracted tasks are saved to the note's frontmatter automatically.

### Current state
- `src/llm/summarizer.rs` has `DEFAULT_SYSTEM_PROMPT` that assumes *meeting transcripts* and outputs a `## Next Steps` section.
- `extract_tasks_from_summary()` in `src/app.rs` already parses `- [ ]` lines from `## Next Steps`.
- Extracted tasks are tagged `["summary"]` and stored in the note's frontmatter via `insert_summary_into_note`.

### Tasks

#### F-2-1  Generalise the system prompt
Replace the meeting-transcript framing with a general-purpose note summarizer.  
Exact replacement for `DEFAULT_SYSTEM_PROMPT` in `src/llm/summarizer.rs`:

```
You are a knowledge assistant that summarizes notes. Your job is to read a note
and produce a clean, structured Markdown summary that captures essential
information, surfaces actionable tasks, and highlights open questions.

## Output Format

Always return exactly this structure, in this order:

---

# Summary
A concise 2–4 sentence overview. What is this note about? What is the main
outcome, decision, or finding? Was anything resolved?

---

## Key Points
Bulleted list of the most important facts, decisions, or conclusions.

- Key point one
- Key point two

---

## Ideas
Bulleted list of ideas, proposals, or open questions worth preserving — even
half-formed ones.

- Idea one

---

## Tasks

### Stated
Explicit action items, commitments, or to-dos that appear directly in the note.
Include owner and due date where mentioned.

- [ ] Explicit task — Owner: [Name or TBD], Due: [Date or TBD]

### Suggested
Tasks that logically follow from the content but are not explicitly stated.
Use judgment to surface what will clearly need to happen.

- [ ] Implied next step — Owner: TBD

---

## Rules
- The **Tasks** section is MANDATORY. Both sub-sections must always be present.
  If there are no stated tasks write "- None identified." Do the same for
  Suggested. Never omit either sub-section.
- For Suggested tasks, reason carefully about what the note makes necessary
  even if it was never written down.
- Do not invent facts. Only summarize what is in the note.
- Keep language clear, direct, and professional.
- Use plain Markdown only. No HTML, no tables unless the data requires it.
- If anything in the note looks inaccurate or contradictory, flag it with a
  brief "⚠ Accuracy note:" line at the bottom of the relevant section.
```

#### F-2-2  Update task extractor for new heading
- Change `extract_tasks_from_summary` in `src/app.rs` to search for `## Tasks` as the section header (in addition to the existing `## Next Steps` fallback for legacy summaries).
- Parse both `### Stated` and `### Suggested` sub-sections.
- Tag stated tasks `["summary", "stated"]` and suggested tasks `["summary", "suggested"]`.

#### F-2-3  Surface task count in summary overlay
- After streaming completes, append a one-line status to the overlay footer:
  `N task(s) extracted and saved to frontmatter.`
- Use `0` when no tasks were found; warn if the Tasks section is absent.

#### F-2-4  Settings toggle: auto-extract tasks
- Add `summarizer.extract_tasks: bool` (default `true`) to `SummarizerConfig` in `src/config.rs`.
- Expose in the Settings panel (S key) so users can disable auto-extraction.

---

## F-3 · Weekly Summary

**Goal:** A single command scans all notes created or modified in the past 7 days, reviews task movements and completions, and streams a consolidated "week in review" summary to an overlay. The summary can optionally be saved as a new note.

### Current state
- Notes have `created` and `modified` timestamps in frontmatter.
- No task history/changelog exists — only current task status is stored.
- No weekly/multi-note summary feature exists.
- The LLM streaming infrastructure (`chat_stream`) and the summary overlay are proven patterns reusable here.

### Tasks

#### F-3-1  Task history log
- Add a `task_events` table to the SQLite DB (`src/db/`):
  ```sql
  CREATE TABLE task_events (
      id          TEXT PRIMARY KEY,
      task_id     TEXT NOT NULL,
      note_path   TEXT NOT NULL,
      title       TEXT NOT NULL,
      from_status TEXT,
      to_status   TEXT NOT NULL,
      changed_at  TEXT NOT NULL   -- ISO-8601
  );
  ```
- Write a migration in `src/db/migrations.rs`.
- Log an event every time a task's status changes (Kanban card move, inline checkbox toggle, or summary extraction that creates a task).

#### F-3-2  Weekly note scanner
- Add `collect_weekly_notes(since: DateTime<Utc>) -> Vec<WeeklyNoteEntry>` in `src/notes/` or a new `src/weekly/`.
- `WeeklyNoteEntry`:
  ```rust
  struct WeeklyNoteEntry {
      path: PathBuf,
      title: String,
      modified: DateTime<Utc>,
      summary_excerpt: Option<String>,  // first paragraph of existing ## Summary
      task_events: Vec<TaskEvent>,      // from DB for this note
  }
  ```
- Filter notes whose `modified` frontmatter timestamp is within the window.

#### F-3-3  Weekly summary prompt builder
- Build a structured prompt from `Vec<WeeklyNoteEntry>`:
  - List of notes touched (title, modified date, summary excerpt if present).
  - List of task events grouped by note: `task title: todo → done`.
- System prompt for the weekly LLM call:
  ```
  You are a productivity assistant. Given a list of notes the user worked on
  this week and the task status changes that occurred, produce a concise
  "Week in Review" report.

  ## Output Format
  # Week in Review: [date range]

  ## What Got Done
  Bulleted summary of completed work, grouped thematically (not by note).

  ## In Progress
  Work that advanced but isn't finished.

  ## Completed Tasks
  - [x] Task title (source note)

  ## Carried Over / Stalled
  Tasks that did not move or were not touched.

  ## Observations
  1–3 patterns, blockers, or insights worth noting.

  ## Suggested Focus for Next Week
  - [ ] Suggested priority one
  - [ ] Suggested priority two

  Rules: be concise; do not pad; attribute to specific notes only when it adds clarity.
  ```

#### F-3-4  Weekly summary trigger and overlay
- Add `Mode::WeeklySummary` to the `Mode` enum in `src/app.rs`.
- Bind `W` (Shift+W) in Normal mode to fire `AppEvent::StartWeeklySummary`.
- Reuse the existing summary streaming overlay; add a header line showing the date range scanned.
- Add a "Save as note" action (`Enter` when streaming is done) that creates a new `.md` file named `week-YYYY-WNN.md` with the streamed content and today's date in frontmatter.

#### F-3-5  Date-range picker (stretch)
- Default window is `now - 7 days`.
- Allow the user to type a number of days in a small input prompt before the scan starts (e.g., `14` for a two-week review).
- Store last-used window in config as `weekly_summary.lookback_days: u32` (default `7`).

#### F-3-6  Settings panel entry
- Add **Weekly Summary (W key)** section in the Settings panel.
- Expose `lookback_days` and the LLM config (URL, key, model) — can reuse `SummarizerConfig` or add a parallel `WeeklySummaryConfig`.

### Technical notes
- The weekly scan reads potentially many files; run entirely in `spawn_blocking`.
- Task history is append-only in the DB; no backfill for tasks moved before F-3-1 ships (document this).
- If no notes or no task events are found in the window, show a friendly "Nothing to summarize for this period." message instead of calling the LLM.

---

## Dependencies / Recommended Ship Order

```
F-1-1  →  F-1-2  →  F-1-3  →  F-1-4  →  F-1-5  →  F-1-6
F-2-1  →  F-2-2  →  F-2-3  →  F-2-4
F-3-1  →  F-3-2  →  F-3-3  →  F-3-4  →  F-3-5 (stretch)

F-1-5 (write-back) should land before F-3-1 (history log),
so the first logged events come from real card moves.

F-2-2 (updated extractor) must land with or after F-2-1 (new prompt)
to avoid the old heading lookup breaking on new-format summaries.
```
