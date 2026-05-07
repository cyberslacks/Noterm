use anyhow::Result;
use ratatui_textarea::TextArea;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    config::Config,
    git::{GitCommit, GitStatus},
    import::meetily::MeetilyMeeting,
    llm::ChatMessage,
    notes::{FileNode, Note, SearchResult, VectorSearchResult},
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
    EmbeddingDone(PathBuf),
    IndexingComplete,
    NoteImported(PathBuf),   // a note was auto-imported (watch folder or API)
    NoteDeleted(PathBuf),    // a note was deleted from disk
    MeetilyMeetingsLoaded(Vec<MeetilyMeeting>),
    MeetilyImportDone { path: PathBuf, meetily_id: String },
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

    // Status bar notification
    pub status_message: Option<(String, StatusLevel)>,

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
            status_message: None,
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
