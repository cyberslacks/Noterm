use anyhow::Result;
use ratatui_textarea::TextArea;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

use std::sync::{Arc, Mutex};

use crate::{
    config::Config,
    git::{GitCommit, GitStatus},
    import::meetily::MeetilyMeeting,
    kazam::{kb_browser::KazamPage, mcp_client::KazamMcpClient},
    llm::ChatMessage,
    notes::{
        annotations::Annotation,
        freshness::FreshnessEntry,
        FileNode, Note, SearchResult, VectorSearchResult,
    },
    tasks::KanbanState,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Edit,
    Search,
    VectorSearch,
    Chat,
    Kanban,
    Git,
    Help,
    NewNote,        // inline prompt for new note name
    GitCommitInput, // inline commit message prompt
    ConfirmDelete,  // confirmation overlay before deleting a note
    MeetilyImport,  // Meetily meeting browser overlay
    Settings,       // LLM / provider settings panel
    Summarize,         // streaming AI summary overlay
    FreshnessView,     // Kazam-style staleness dashboard overlay
    AnnotationPanel,   // sidecar annotation viewer/composer
    KazamKbBrowser,    // Kazam KB page browser/importer
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsMode {
    Navigating,
    EditingText,
    PickingModel,
    EditingLongText, // full-screen textarea for multi-line fields (e.g. system prompt)
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug)]
pub enum AppEvent {
    FileTreeRefresh(Vec<FileNode>),
    SearchResults(Vec<SearchResult>),
    VectorSearchResults(Vec<VectorSearchResult>),
    GitStatusResult(Result<GitStatus, String>),
    GitLogResult(Result<Vec<GitCommit>, String>),
    GitOpComplete(Result<(), String>),
    ChatChunk(String),
    ChatDone,
    ChatError(String),
    EmbedRequest {
        note_id: String,
        note_path: String,
        content_hash: String,
        content: String,
    },
    EmbeddingDone(PathBuf),
    IndexingComplete,
    NoteImported(PathBuf),   // a note was auto-imported (watch folder or API)
    NoteDeleted(PathBuf),    // a note was deleted from disk
    MeetilyMeetingsLoaded(Vec<MeetilyMeeting>),
    MeetilyImportDone { path: PathBuf, meetily_id: String },
    FreshnessListLoaded(Vec<FreshnessEntry>),
    AnnotationsLoaded { slug: String, entries: Vec<Annotation> },
    AnnotationSaved(String), // slug — triggers reload
    KazamItemsLoaded(Vec<KazamPage>),
    KazamImportDone(PathBuf),
    KazamExportDone(PathBuf),
    KazamExportError(String),
    KazamMcpConnected(Arc<Mutex<KazamMcpClient>>),
    KazamMcpError(String),
    KazamKbContextLoaded(Vec<String>),
    ModelsLoaded { ollama: Vec<String>, openai: Vec<String> },
    ForceReembed,
    SummaryChunk(String),
    SummaryDone,
    SummaryError(String),
    UpdateAvailable(String),
    Error(String),
}

pub struct MeetilyPanelState {
    pub meetings: Vec<MeetilyMeeting>,
    pub cursor: usize,
    pub loading: bool,
    /// meetily_ids that have already been imported this session
    pub imported_ids: std::collections::HashSet<String>,
}

impl Default for MeetilyPanelState {
    fn default() -> Self {
        Self {
            meetings: Vec::new(),
            cursor: 0,
            loading: false,
            imported_ids: std::collections::HashSet::new(),
        }
    }
}

pub struct FreshnessPanelState {
    pub entries: Vec<FreshnessEntry>,
    pub cursor: usize,
    pub loading: bool,
}

impl Default for FreshnessPanelState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            loading: false,
        }
    }
}

pub struct AnnotationPanelState {
    pub entries: Vec<Annotation>,
    pub cursor: usize,
    pub input: String,
    pub composing: bool,
    pub section_hint: String,
    pub slug: String,
}

impl Default for AnnotationPanelState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            input: String::new(),
            composing: false,
            section_hint: String::new(),
            slug: String::new(),
        }
    }
}

pub struct KazamKbState {
    pub entries: Vec<KazamPage>,
    pub cursor: usize,
    pub loading: bool,
    pub filter: String,
}

impl Default for KazamKbState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            loading: false,
            filter: String::new(),
        }
    }
}

pub struct AppState {
    pub mode: Mode,
    pub previous_mode: Mode,
    pub config: Config,

    // File tree
    pub notes_dir: PathBuf,
    pub file_tree: Vec<FileNode>,
    pub selected_file_idx: usize,

    // Editor/Viewer
    pub current_note: Option<Note>,
    pub editor: TextArea<'static>,
    pub is_modified: bool,
    pub viewer_scroll: usize,

    // Full-text search
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_cursor: usize,

    // Vector search
    pub vsearch_query: String,
    pub vsearch_results: Vec<VectorSearchResult>,
    pub vsearch_cursor: usize,
    pub vsearch_loading: bool,

    // Chat
    pub chat_messages: Vec<ChatMessage>,
    pub chat_input: String,
    pub chat_loading: bool,
    pub chat_streaming_buf: String,

    // Kanban
    pub kanban: KanbanState,

    // Git
    pub git_status: Option<GitStatus>,
    pub git_log: Vec<GitCommit>,
    pub git_loading: bool,
    pub git_selected_tab: usize, // 0=Status, 1=Log
    pub git_commit_msg: String,

    // Inline prompts
    pub prompt_input: String,

    // Meetily import panel
    pub meetily: MeetilyPanelState,

    // Freshness / staleness dashboard
    pub freshness: FreshnessPanelState,

    // Sidecar annotations panel
    pub annotation: AnnotationPanelState,
    pub annotation_pending_count: usize,

    // Kazam KB browser
    pub kazam_kb: KazamKbState,

    // Kazam MCP client (Track B — optional)
    pub kazam_mcp: Option<Arc<Mutex<KazamMcpClient>>>,
    pub kazam_mcp_connected: bool,

    // Chat KB context toggle (Track B)
    pub chat_kazam_context: bool,
    pub kazam_kb_pages: Vec<String>,

    // Settings panel
    pub settings_mode: SettingsMode,
    pub settings_cursor: usize,
    pub settings_edit_buf: String,
    pub settings_model_cursor: usize,
    pub available_ollama_models: Vec<String>,
    pub available_openai_models: Vec<String>,
    pub settings_prompt_editor: TextArea<'static>,

    // Summarize panel
    pub summarize_loading: bool,
    pub summarize_buf: String,
    pub summarize_scroll: usize,

    // Help panel
    pub help_scroll: usize,

    // Status bar notification
    pub status_message: Option<(String, StatusLevel)>,

    // In-app update notification
    pub update_available: Option<String>,

    // Async event channel (sender cloned into spawned tasks)
    pub tx: UnboundedSender<AppEvent>,
}

impl AppState {
    pub fn new(config: Config, tx: UnboundedSender<AppEvent>) -> Self {
        let notes_dir = config.notes_dir.clone();
        Self {
            mode: Mode::Normal,
            previous_mode: Mode::Normal,
            notes_dir,
            file_tree: Vec::new(),
            selected_file_idx: 0,
            current_note: None,
            editor: TextArea::default(),
            is_modified: false,
            viewer_scroll: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_cursor: 0,
            vsearch_query: String::new(),
            vsearch_results: Vec::new(),
            vsearch_cursor: 0,
            vsearch_loading: false,
            chat_messages: Vec::new(),
            chat_input: String::new(),
            chat_loading: false,
            chat_streaming_buf: String::new(),
            kanban: KanbanState::default(),
            git_status: None,
            git_log: Vec::new(),
            git_loading: false,
            git_selected_tab: 0,
            git_commit_msg: String::new(),
            prompt_input: String::new(),
            meetily: MeetilyPanelState::default(),
            freshness: FreshnessPanelState::default(),
            annotation: AnnotationPanelState::default(),
            annotation_pending_count: 0,
            kazam_kb: KazamKbState::default(),
            kazam_mcp: None,
            kazam_mcp_connected: false,
            chat_kazam_context: false,
            kazam_kb_pages: Vec::new(),
            settings_mode: SettingsMode::Navigating,
            settings_cursor: 0,
            settings_edit_buf: String::new(),
            settings_model_cursor: 0,
            available_ollama_models: Vec::new(),
            available_openai_models: Vec::new(),
            settings_prompt_editor: TextArea::default(),
            summarize_loading: false,
            summarize_buf: String::new(),
            summarize_scroll: 0,
            help_scroll: 0,
            status_message: None,
            update_available: None,
            config,
            tx,
        }
    }

    pub fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::FileTreeRefresh(nodes) => {
                self.file_tree = nodes;
                // Clamp selection
                if self.selected_file_idx >= self.file_tree.len() {
                    self.selected_file_idx = self.file_tree.len().saturating_sub(1);
                }
            }

            AppEvent::SearchResults(results) => {
                self.search_results = results;
                self.search_cursor = 0;
            }

            AppEvent::VectorSearchResults(results) => {
                self.vsearch_results = results;
                self.vsearch_cursor = 0;
                self.vsearch_loading = false;
            }

            AppEvent::GitStatusResult(result) => {
                self.git_loading = false;
                match result {
                    Ok(status) => self.git_status = Some(status),
                    Err(e) => self.set_status(format!("Git error: {e}"), StatusLevel::Error),
                }
            }

            AppEvent::GitLogResult(result) => {
                match result {
                    Ok(log) => self.git_log = log,
                    Err(e) => self.set_status(format!("Git log error: {e}"), StatusLevel::Error),
                }
            }

            AppEvent::GitOpComplete(result) => {
                self.git_loading = false;
                match result {
                    Ok(()) => self.set_status("Git operation complete".into(), StatusLevel::Success),
                    Err(e) => self.set_status(format!("Git error: {e}"), StatusLevel::Error),
                }
            }

            AppEvent::ChatChunk(token) => {
                self.chat_streaming_buf.push_str(&token);
                // Update last assistant message or append new one
                if let Some(last) = self.chat_messages.last_mut() {
                    if last.role == crate::llm::ChatRole::Assistant {
                        last.content = self.chat_streaming_buf.clone();
                        return;
                    }
                }
                self.chat_messages.push(ChatMessage::assistant(self.chat_streaming_buf.clone()));
            }

            AppEvent::ChatDone => {
                self.chat_loading = false;
                self.chat_streaming_buf.clear();
            }

            AppEvent::ChatError(e) => {
                self.chat_loading = false;
                self.chat_streaming_buf.clear();
                self.set_status(format!("LLM error: {e}"), StatusLevel::Error);
            }

            AppEvent::EmbedRequest { .. } => {
                // handled in main event loop (requires db access)
            }

            AppEvent::EmbeddingDone(path) => {
                tracing::debug!("Embedding done: {}", path.display());
            }

            AppEvent::IndexingComplete => {
                self.set_status("Index updated".into(), StatusLevel::Info);
            }

            AppEvent::NoteImported(path) => {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                self.set_status(format!("Imported: {name}"), StatusLevel::Success);
                // File tree will be refreshed by a follow-up FileTreeRefresh event
            }

            AppEvent::NoteDeleted(path) => {
                // If we deleted the currently open note, clear the editor
                if let Some(ref note) = self.current_note {
                    if note.path == path {
                        self.current_note = None;
                        self.editor = ratatui_textarea::TextArea::default();
                        self.is_modified = false;
                    }
                }
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                self.set_status(format!("Deleted: {name}"), StatusLevel::Success);
            }

            AppEvent::MeetilyMeetingsLoaded(meetings) => {
                self.meetily.meetings = meetings;
                self.meetily.cursor = 0;
                self.meetily.loading = false;
            }

            AppEvent::MeetilyImportDone { path, meetily_id } => {
                self.meetily.imported_ids.insert(meetily_id);
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                self.set_status(format!("Imported meeting: {name}"), StatusLevel::Success);
                // Trigger file tree refresh
                let tx = self.tx.clone();
                let notes_dir = self.notes_dir.clone();
                let show_hidden = self.config.ui.show_hidden;
                tokio::spawn(async move {
                    let nodes = tokio::task::spawn_blocking(move || {
                        crate::notes::watcher::scan_dir(&notes_dir, show_hidden)
                    })
                    .await
                    .unwrap_or_default();
                    tx.send(AppEvent::FileTreeRefresh(nodes)).ok();
                });
            }

            AppEvent::FreshnessListLoaded(entries) => {
                self.freshness.entries = entries;
                self.freshness.cursor = 0;
                self.freshness.loading = false;
            }

            AppEvent::AnnotationsLoaded { slug, entries } => {
                if self.annotation.slug == slug {
                    let pending = entries
                        .iter()
                        .filter(|a| a.status == crate::notes::annotations::AnnotationStatus::Pending)
                        .count();
                    self.annotation.entries = entries;
                    self.annotation.cursor = 0;
                    self.annotation.composing = false;
                    self.annotation.input.clear();
                    self.annotation_pending_count = pending;
                }
            }

            AppEvent::AnnotationSaved(slug) => {
                let tx = self.tx.clone();
                let notes_dir = self.notes_dir.clone();
                let slug2 = slug.clone();
                tokio::spawn(async move {
                    let entries = tokio::task::spawn_blocking(move || {
                        crate::notes::annotations::load_annotations(&notes_dir, &slug2)
                    })
                    .await
                    .unwrap_or_default();
                    tx.send(AppEvent::AnnotationsLoaded { slug, entries }).ok();
                });
            }

            AppEvent::KazamItemsLoaded(pages) => {
                self.kazam_kb.entries = pages;
                self.kazam_kb.cursor = 0;
                self.kazam_kb.loading = false;
            }

            AppEvent::KazamImportDone(path) => {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                self.set_status(format!("Imported from Kazam: {name}"), StatusLevel::Success);
                // Refresh file tree and open the imported note
                let tx = self.tx.clone();
                let notes_dir = self.notes_dir.clone();
                let show_hidden = self.config.ui.show_hidden;
                tokio::spawn(async move {
                    let nodes = tokio::task::spawn_blocking(move || {
                        crate::notes::watcher::scan_dir(&notes_dir, show_hidden)
                    })
                    .await
                    .unwrap_or_default();
                    tx.send(AppEvent::FileTreeRefresh(nodes)).ok();
                });
            }

            AppEvent::KazamExportDone(path) => {
                self.set_status(
                    format!("Exported → {}", path.display()),
                    StatusLevel::Success,
                );
            }

            AppEvent::KazamExportError(e) => {
                self.set_status(format!("Export failed: {e}"), StatusLevel::Error);
            }

            AppEvent::KazamMcpConnected(client) => {
                self.kazam_mcp = Some(client);
                self.kazam_mcp_connected = true;
                self.set_status("Kazam MCP connected".into(), StatusLevel::Success);
            }

            AppEvent::KazamMcpError(e) => {
                self.kazam_mcp = None;
                self.kazam_mcp_connected = false;
                self.set_status(format!("Kazam MCP: {e}"), StatusLevel::Error);
            }

            AppEvent::KazamKbContextLoaded(pages) => {
                self.kazam_kb_pages = pages;
                self.set_status(
                    format!("KB context loaded ({} pages)", self.kazam_kb_pages.len()),
                    StatusLevel::Info,
                );
            }

            AppEvent::ModelsLoaded { ollama, openai } => {
                self.available_ollama_models = ollama;
                self.available_openai_models = openai;
            }

            AppEvent::ForceReembed => {
                // handled in main event loop before reaching here
            }

            AppEvent::SummaryChunk(token) => {
                self.summarize_buf.push_str(&token);
            }

            AppEvent::SummaryDone => {
                self.summarize_loading = false;
                match self.insert_summary_into_note() {
                    Ok(true) => self.set_status("Summary inserted into note".into(), StatusLevel::Success),
                    Ok(false) => {}
                    Err(e) => self.set_status(format!("Summary insert failed: {e}"), StatusLevel::Error),
                }
            }

            AppEvent::SummaryError(e) => {
                self.summarize_loading = false;
                self.set_status(format!("Summarizer: {e}"), StatusLevel::Error);
                if self.mode == Mode::Summarize {
                    self.mode = Mode::Normal;
                }
            }

            AppEvent::UpdateAvailable(version) => {
                self.update_available = Some(version);
            }

            AppEvent::Error(e) => {
                self.set_status(e, StatusLevel::Error);
            }
        }
    }

    pub fn set_status(&mut self, msg: String, level: StatusLevel) {
        self.status_message = Some((msg, level));
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Insert `summarize_buf` into the note's `## Summary` section (or prepend one).
    /// Returns Ok(true) if a note was open and the write succeeded.
    pub fn insert_summary_into_note(&mut self) -> anyhow::Result<bool> {
        let summary = self.summarize_buf.clone();
        if summary.is_empty() {
            return Ok(false);
        }
        let note = match self.current_note.as_mut() {
            Some(n) => n,
            None => return Ok(false),
        };

        let new_body = inject_summary_into_body(&note.body, &summary);

        // Extract action items from the summary and merge into frontmatter tasks.
        let new_tasks = extract_tasks_from_summary(&summary);
        if !new_tasks.is_empty() {
            let relative_path = note.relative_path.clone();
            let existing = note.frontmatter.tasks.get_or_insert_with(Vec::new);
            for mut task in new_tasks {
                task.note_path = relative_path.clone();
                if !existing.iter().any(|e| e.title == task.title) {
                    existing.push(task);
                }
            }
        }

        note.body = new_body.clone();
        note.raw = crate::notes::frontmatter::serialize(&note.frontmatter, &new_body);
        std::fs::write(&note.path, &note.raw)?;
        self.is_modified = false;

        // Keep the editor in sync with the updated body
        let lines: Vec<String> = note.body.lines().map(String::from).collect();
        self.editor = ratatui_textarea::TextArea::new(lines);

        Ok(true)
    }

    pub fn selected_file_node(&self) -> Option<&FileNode> {
        self.file_tree.get(self.selected_file_idx)
    }

    pub fn git_branch(&self) -> String {
        self.git_status
            .as_ref()
            .map(|s| s.branch.clone())
            .unwrap_or_else(|| "?".into())
    }

    pub fn save_current_note(&mut self) -> Result<()> {
        if let Some(note) = &mut self.current_note {
            let content = self.editor.lines().join("\n");
            note.body = content.clone();
            note.raw = crate::notes::frontmatter::serialize(&note.frontmatter, &content);
            std::fs::write(&note.path, &note.raw)?;
            self.is_modified = false;
        }
        Ok(())
    }

    pub fn open_note(&mut self, note: Note) {
        let slug = crate::notes::annotations::note_slug(&note.path);
        self.annotation_pending_count =
            crate::notes::annotations::count_pending(&self.notes_dir, &slug);
        let lines: Vec<String> = note.body.lines().map(String::from).collect();
        self.editor = TextArea::new(lines);
        self.viewer_scroll = 0;
        self.is_modified = false;
        self.current_note = Some(note);
    }

    pub fn nav_tree_down(&mut self) {
        if self.selected_file_idx + 1 < self.file_tree.len() {
            self.selected_file_idx += 1;
        }
    }

    pub fn nav_tree_up(&mut self) {
        self.selected_file_idx = self.selected_file_idx.saturating_sub(1);
    }

    pub fn enter_mode(&mut self, mode: Mode) {
        self.previous_mode = self.mode.clone();
        self.mode = mode;
    }

    pub fn return_to_previous(&mut self) {
        let prev = self.previous_mode.clone();
        self.mode = prev;
    }
}

/// Parse `- [ ] ...` lines from the `## Next Steps` section of a generated summary.
/// Each line becomes a `Task` with status `Todo`. Lines like "None identified." are skipped.
fn extract_tasks_from_summary(summary: &str) -> Vec<crate::tasks::Task> {
    let mut tasks = Vec::new();
    let mut in_next_steps = false;

    for line in summary.lines() {
        let trimmed = line.trim();

        if trimmed.eq_ignore_ascii_case("## next steps") || trimmed.starts_with("## Next Steps") {
            in_next_steps = true;
            continue;
        }

        // Any `## ` sibling (not `### ` sub-section) closes the section.
        if in_next_steps && trimmed.starts_with("## ") && !trimmed.starts_with("### ") {
            break;
        }

        if in_next_steps {
            if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
                // Strip the ` — Owner: ...` suffix generated by the system prompt.
                let title = rest
                    .splitn(2, " — Owner:")
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .to_string();

                if title.is_empty() || title.eq_ignore_ascii_case("none identified.") {
                    continue;
                }

                tasks.push(crate::tasks::Task {
                    id: uuid::Uuid::new_v4().to_string(),
                    title,
                    description: None,
                    status: crate::tasks::TaskStatus::Todo,
                    priority: None,
                    due: None,
                    tags: Some(vec!["summary".into()]),
                    note_path: String::new(), // filled in by caller
                });
            }
        }
    }

    tasks
}

/// Replace the `## Summary` section of `body` with `summary_text`, or prepend one.
fn inject_summary_into_body(body: &str, summary_text: &str) -> String {
    // Locate a "## Summary" (case-insensitive) at the start of a line.
    let lower = body.to_lowercase();
    let search = "## summary";

    let section_start = lower.find(search).filter(|&pos| {
        // Must be at the start of a line (pos == 0 or preceded by '\n')
        pos == 0 || body.as_bytes().get(pos - 1) == Some(&b'\n')
    });

    if let Some(start) = section_start {
        // Find where this section ends: next "## " at the same depth, or EOF.
        let after_heading = start + search.len();
        let rest = &body[after_heading..];
        let section_end = rest.find("\n## ")
            .map(|p| after_heading + p)     // keep the '\n' so next heading stays on its own line
            .unwrap_or(body.len());

        let before = body[..start].trim_end_matches('\n');
        let after = body[section_end..].trim_start_matches('\n');

        if before.is_empty() && after.is_empty() {
            format!("## Summary\n\n{summary_text}\n")
        } else if before.is_empty() {
            format!("## Summary\n\n{summary_text}\n\n{after}")
        } else if after.is_empty() {
            format!("{before}\n\n## Summary\n\n{summary_text}\n")
        } else {
            format!("{before}\n\n## Summary\n\n{summary_text}\n\n{after}")
        }
    } else {
        // No Summary section — prepend one before the body content.
        if body.trim().is_empty() {
            format!("## Summary\n\n{summary_text}\n")
        } else {
            format!("## Summary\n\n{summary_text}\n\n---\n\n{}", body.trim_start())
        }
    }
}
