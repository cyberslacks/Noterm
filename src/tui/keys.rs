use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{AppEvent, AppState, Mode, SettingsMode};
use crate::tui::widgets::settings_panel as sp;

pub enum Action {
    Continue,
    Quit,
}

pub async fn handle_event(state: &mut AppState, event: Event) -> Result<Action> {
    if let Event::Key(key) = event {
        return handle_key(state, key).await;
    }
    Ok(Action::Continue)
}

async fn handle_key(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    // Global: Ctrl+q always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
        return Ok(Action::Quit);
    }

    match state.mode.clone() {
        Mode::Normal => handle_normal(state, key).await,
        Mode::Edit => handle_edit(state, key),
        Mode::Search => handle_search(state, key),
        Mode::VectorSearch => handle_vsearch(state, key),
        Mode::Chat => handle_chat(state, key),
        Mode::Kanban => handle_kanban(state, key),
        Mode::Git => handle_git(state, key),
        Mode::Help => {
            state.mode = Mode::Normal;
            Ok(Action::Continue)
        }
        Mode::NewNote => handle_new_note(state, key),
        Mode::GitCommitInput => handle_git_commit_input(state, key),
        Mode::ConfirmDelete => handle_confirm_delete(state, key).await,
        Mode::MeetilyImport => handle_meetily(state, key).await,
        Mode::Settings => handle_settings(state, key),
        Mode::Summarize => handle_summarize(state, key),
    }
}

async fn handle_normal(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Char('q') => return Ok(Action::Quit),
        KeyCode::Char('?') => state.enter_mode(Mode::Help),
        KeyCode::Char('j') | KeyCode::Down => state.nav_tree_down(),
        KeyCode::Char('k') | KeyCode::Up => state.nav_tree_up(),
        KeyCode::Enter => open_selected(state)?,
        KeyCode::Char('e') => {
            if state.current_note.is_some() {
                state.enter_mode(Mode::Edit);
            }
        }
        KeyCode::Char('n') => {
            state.prompt_input.clear();
            state.enter_mode(Mode::NewNote);
        }
        KeyCode::Char('d') => {
            // Only delete files, not directories
            if state.selected_file_node().map(|n| !n.is_dir).unwrap_or(false) {
                state.enter_mode(Mode::ConfirmDelete);
            }
        }
        KeyCode::Char('/') => {
            state.search_query.clear();
            state.search_results.clear();
            state.enter_mode(Mode::Search);
        }
        KeyCode::Char('v') => {
            state.vsearch_query.clear();
            state.vsearch_results.clear();
            state.enter_mode(Mode::VectorSearch);
        }
        KeyCode::Char('c') => {
            state.enter_mode(Mode::Chat);
        }
        KeyCode::Char('K') => {
            // Load tasks from the current note's frontmatter
            let tasks = state
                .current_note
                .as_ref()
                .and_then(|n| n.frontmatter.tasks.clone())
                .unwrap_or_default()
                .into_iter()
                .map(|mut t| {
                    if let Some(note) = &state.current_note {
                        t.note_path = note.relative_path.clone();
                    }
                    t
                })
                .collect();
            state.kanban = crate::tasks::KanbanState::from_tasks(tasks);
            state.enter_mode(Mode::Kanban);
        }
        KeyCode::Char('G') => {
            state.git_loading = true;
            state.enter_mode(Mode::Git);
            let tx = state.tx.clone();
            let notes_dir = state.notes_dir.clone();
            let notes_dir2 = notes_dir.clone();
            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    crate::git::operations::get_status(&notes_dir)
                })
                .await;
                let mapped = match result {
                    Ok(Ok(s)) => Ok(s),
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                tx.send(AppEvent::GitStatusResult(mapped)).ok();

                let log_result = tokio::task::spawn_blocking(move || {
                    crate::git::operations::get_log(&notes_dir2, 50)
                })
                .await;
                if let Ok(Ok(log)) = log_result {
                    tx.send(AppEvent::GitLogResult(Ok(log))).ok();
                }
            });
        }
        KeyCode::Char('S') => {
            state.settings_cursor = 0;
            state.settings_mode = SettingsMode::Navigating;
            state.settings_edit_buf.clear();
            state.settings_model_cursor = 0;
            state.enter_mode(Mode::Settings);
            // Fetch available models in background
            let ollama_url = state.config.llm.ollama_base_url.clone();
            let openai_url = state.config.llm.openai_base_url.clone();
            let openai_key = state.config.llm.openai_api_key.clone().unwrap_or_default();
            let tx = state.tx.clone();
            tokio::spawn(async move {
                let ollama = crate::llm::list_ollama_models(&ollama_url).await.unwrap_or_default();
                let openai = crate::llm::list_openai_models(&openai_url, &openai_key).await.unwrap_or_default();
                tx.send(AppEvent::ModelsLoaded { ollama, openai }).ok();
            });
        }
        KeyCode::Char('I') => {
            state.enter_mode(Mode::MeetilyImport);
            if state.meetily.meetings.is_empty() && !state.meetily.loading {
                state.meetily.loading = true;
                let tx = state.tx.clone();
                let meetily_cfg = state.config.meetily.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        let db_path = crate::import::meetily::find_db(&meetily_cfg)
                            .ok_or_else(|| anyhow::anyhow!("Meetily DB not found"))?;
                        crate::import::meetily::load_meetings(&db_path)
                    })
                    .await;
                    match result {
                        Ok(Ok(meetings)) => { tx.send(AppEvent::MeetilyMeetingsLoaded(meetings)).ok(); }
                        Ok(Err(e)) => { tx.send(AppEvent::Error(format!("Meetily: {e}"))).ok(); }
                        Err(e) => { tx.send(AppEvent::Error(format!("Meetily task: {e}"))).ok(); }
                    }
                });
            }
        }
        KeyCode::Char('X') => {
            if state.current_note.is_some() {
                state.summarize_loading = true;
                state.summarize_buf.clear();
                state.summarize_scroll = 0;
                state.enter_mode(Mode::Summarize);
                // The main event loop detects summarize_loading and spawns the task.
            } else {
                state.set_status("Open a note first".into(), crate::app::StatusLevel::Warning);
            }
        }
        KeyCode::PageDown => {
            state.viewer_scroll = state.viewer_scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            state.viewer_scroll = state.viewer_scroll.saturating_sub(10);
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_summarize(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            // Close the overlay; if generation already finished the summary was
            // already inserted on SummaryDone. If still loading, abandon it.
            state.summarize_loading = false;
            state.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.summarize_scroll = state.summarize_scroll.saturating_add(3);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.summarize_scroll = state.summarize_scroll.saturating_sub(3);
        }
        KeyCode::PageDown => {
            state.summarize_scroll = state.summarize_scroll.saturating_add(20);
        }
        KeyCode::PageUp => {
            state.summarize_scroll = state.summarize_scroll.saturating_sub(20);
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn open_selected(state: &mut AppState) -> Result<()> {
    if let Some(node) = state.selected_file_node().cloned() {
        if !node.is_dir {
            let note = crate::notes::Note::from_path(&node.path, &state.notes_dir)?;
            state.open_note(note);
        }
    }
    Ok(())
}

fn handle_edit(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            if state.config.editor.auto_save {
                state.save_current_note()?;
                index_current_note(state);
            }
            state.mode = Mode::Normal;
        }
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                state.save_current_note()?;
                index_current_note(state);
            } else {
                state.is_modified = state.editor.input(crossterm::event::Event::Key(key));
            }
        }
    }
    Ok(Action::Continue)
}

fn index_current_note(state: &AppState) {
    if let Some(note) = &state.current_note {
        let rel_path = note.relative_path.clone();
        let title = note.frontmatter.title.clone().unwrap_or_default();
        let body = note.body.clone();
        let tags = note.frontmatter.tags.clone().unwrap_or_default();
        let index_dir = state.config.index_dir();
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                if let Ok(idx) = crate::search::fulltext::FtsIndex::open_or_create(&index_dir) {
                    idx.index_note(&rel_path, &title, &body, &tags).ok();
                }
            })
            .await
            .ok();
        });

        if state.config.search.embed_on_save {
            let note_id = note.frontmatter.id.clone()
                .unwrap_or_else(|| note.relative_path.clone());
            let content = format!("{}\n\n{}", note.frontmatter.title.clone().unwrap_or_default(), note.body);
            state.tx.send(crate::app::AppEvent::EmbedRequest {
                note_id,
                note_path: note.relative_path.clone(),
                content_hash: note.content_hash.clone(),
                content,
            }).ok();
        }
    }
}

fn handle_search(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            state.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Some(result) = state.search_results.get(state.search_cursor).cloned() {
                // Open the selected result
                let path = state.notes_dir.join(&result.relative_path);
                if let Ok(note) = crate::notes::Note::from_path(&path, &state.notes_dir) {
                    state.open_note(note);
                    state.mode = Mode::Normal;
                }
            } else if !state.search_query.is_empty() {
                // No results yet — fire the search immediately (bypass debounce)
                let query = state.search_query.clone();
                let index_dir = state.config.index_dir();
                let tx = state.tx.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        let idx = crate::search::fulltext::FtsIndex::open_or_create(&index_dir)?;
                        idx.search(&query, 20)
                    })
                    .await;
                    match result {
                        Ok(Ok(results)) => { tx.send(AppEvent::SearchResults(results)).ok(); }
                        Ok(Err(e)) => { tx.send(AppEvent::Error(format!("Search error: {e}"))).ok(); }
                        Err(e) => { tx.send(AppEvent::Error(format!("Search task: {e}"))).ok(); }
                    }
                });
            }
        }
        KeyCode::Up => {
            state.search_cursor = state.search_cursor.saturating_sub(1);
        }
        KeyCode::Down => {
            if state.search_cursor + 1 < state.search_results.len() {
                state.search_cursor += 1;
            }
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
            // Trigger search (will be debounced in practice via the search module)
            // For now, signal via channel — caller will handle
        }
        KeyCode::Backspace => {
            state.search_query.pop();
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_vsearch(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => state.mode = Mode::Normal,
        KeyCode::Enter => {
            if !state.vsearch_results.is_empty() {
                // Open selected result
                if let Some(result) = state.vsearch_results.get(state.vsearch_cursor).cloned() {
                    let path = state.notes_dir.join(&result.relative_path);
                    if let Ok(note) = crate::notes::Note::from_path(&path, &state.notes_dir) {
                        state.open_note(note);
                        state.mode = Mode::Normal;
                    }
                }
            } else if !state.vsearch_query.is_empty() && !state.vsearch_loading {
                state.vsearch_loading = true;
            }
        }
        KeyCode::Up => {
            state.vsearch_cursor = state.vsearch_cursor.saturating_sub(1);
        }
        KeyCode::Down => {
            if state.vsearch_cursor + 1 < state.vsearch_results.len() {
                state.vsearch_cursor += 1;
            }
        }
        KeyCode::Char(c) => {
            state.vsearch_query.push(c);
            state.vsearch_results.clear();
            state.vsearch_cursor = 0;
        }
        KeyCode::Backspace => {
            state.vsearch_query.pop();
            state.vsearch_results.clear();
            state.vsearch_cursor = 0;
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_chat(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => state.mode = Mode::Normal,
        KeyCode::Enter => {
            if !state.chat_input.is_empty() && !state.chat_loading {
                let msg = state.chat_input.drain(..).collect::<String>();
                state.chat_messages.push(crate::llm::ChatMessage::user(msg));
                state.chat_loading = true;
                state.chat_streaming_buf.clear();
                // Chat send is triggered by the main loop watching chat_loading state
            }
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.chat_messages.clear();
        }
        KeyCode::Char(c) => state.chat_input.push(c),
        KeyCode::Backspace => { state.chat_input.pop(); }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_kanban(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            save_kanban_to_note(state)?;
            state.mode = Mode::Normal;
        }
        KeyCode::Char('h') | KeyCode::Left => state.kanban.nav_col_left(),
        KeyCode::Char('l') | KeyCode::Right => state.kanban.nav_col_right(),
        KeyCode::Char('j') | KeyCode::Down => state.kanban.nav_down(),
        KeyCode::Char('k') | KeyCode::Up => state.kanban.nav_up(),
        KeyCode::Char('m') => state.kanban.move_focused_right(),
        KeyCode::Char('M') => state.kanban.move_focused_left(),
        _ => {}
    }
    Ok(Action::Continue)
}

fn save_kanban_to_note(state: &mut AppState) -> Result<()> {
    if let Some(note) = &mut state.current_note {
        let tasks: Vec<crate::tasks::Task> = state
            .kanban
            .columns
            .iter()
            .flat_map(|col| col.cards.iter().cloned())
            .collect();

        note.frontmatter.tasks = if tasks.is_empty() { None } else { Some(tasks) };
        let body = note.body.clone();
        note.raw = crate::notes::frontmatter::serialize(&note.frontmatter, &body);
        std::fs::write(&note.path, &note.raw)?;
    }
    Ok(())
}

fn handle_git(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => state.mode = Mode::Normal,
        KeyCode::Tab => {
            state.git_selected_tab = (state.git_selected_tab + 1) % 2;
        }
        KeyCode::Char('s') => {
            // Stage all
            let tx = state.tx.clone();
            let notes_dir = state.notes_dir.clone();
            state.git_loading = true;
            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    crate::git::operations::stage_all(&notes_dir)
                })
                .await;
                let mapped = match result {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                tx.send(AppEvent::GitOpComplete(mapped)).ok();
            });
        }
        KeyCode::Char('c') => {
            state.prompt_input.clear();
            state.enter_mode(Mode::GitCommitInput);
        }
        KeyCode::Char('p') => {
            let tx = state.tx.clone();
            let notes_dir = state.notes_dir.clone();
            let remote = state.config.git.remote.clone().unwrap_or_else(|| "origin".into());
            let git_username = state.config.git.git_username.clone();
            let git_token = state.config.git.git_token.clone();
            state.git_loading = true;
            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    let repo = git2::Repository::open(&notes_dir)?;
                    let branch = repo.head()?.shorthand().unwrap_or("main").to_string();
                    drop(repo);
                    crate::git::operations::push(&notes_dir, &remote, &branch, git_username, git_token)
                })
                .await;
                let mapped = match result {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                tx.send(AppEvent::GitOpComplete(mapped)).ok();
            });
        }
        KeyCode::Char('P') => {
            let tx = state.tx.clone();
            let notes_dir = state.notes_dir.clone();
            let remote = state.config.git.remote.clone().unwrap_or_else(|| "origin".into());
            let git_username = state.config.git.git_username.clone();
            let git_token = state.config.git.git_token.clone();
            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    let repo = git2::Repository::open(&notes_dir)?;
                    let branch = repo.head()?.shorthand().unwrap_or("main").to_string();
                    drop(repo);
                    crate::git::operations::pull(&notes_dir, &remote, &branch, git_username, git_token)
                })
                .await;
                let mapped = match result {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                tx.send(AppEvent::GitOpComplete(mapped)).ok();
            });
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_new_note(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            state.prompt_input.clear();
            state.return_to_previous();
        }
        KeyCode::Enter => {
            let name = state.prompt_input.drain(..).collect::<String>().trim().to_string();
            if !name.is_empty() {
                let filename = if name.ends_with(".md") {
                    name
                } else {
                    format!("{name}.md")
                };
                // Create in the selected folder, or the parent of the selected file
                let target_dir = state
                    .selected_file_node()
                    .map(|n| {
                        if n.is_dir {
                            n.path.clone()
                        } else {
                            n.path.parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or_else(|| state.notes_dir.clone())
                        }
                    })
                    .unwrap_or_else(|| state.notes_dir.clone());
                let path = target_dir.join(&filename);
                if !path.exists() {
                    let id = uuid::Uuid::new_v4().to_string();
                    let now = chrono::Utc::now().to_rfc3339();
                    let content = format!(
                        "---\ntitle: \"{}\"\nid: \"{}\"\ncreated: \"{}\"\nmodified: \"{}\"\n---\n\n",
                        filename.trim_end_matches(".md"),
                        id,
                        now,
                        now
                    );
                    std::fs::write(&path, &content)?;

                    // Refresh file tree
                    let tx = state.tx.clone();
                    let notes_dir = state.notes_dir.clone();
                    let show_hidden = state.config.ui.show_hidden;
                    tokio::spawn(async move {
                        let nodes = tokio::task::spawn_blocking(move || {
                            crate::notes::watcher::scan_dir(&notes_dir, show_hidden)
                        })
                        .await
                        .unwrap_or_default();
                        tx.send(AppEvent::FileTreeRefresh(nodes)).ok();
                    });

                    // Open the new note
                    let note = crate::notes::Note::from_path(&path, &state.notes_dir)?;
                    state.open_note(note);
                    state.mode = Mode::Edit;
                }
            }
        }
        KeyCode::Char(c) => state.prompt_input.push(c),
        KeyCode::Backspace => { state.prompt_input.pop(); }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_git_commit_input(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            state.prompt_input.clear();
            state.mode = Mode::Git;
        }
        KeyCode::Enter => {
            let msg = state.prompt_input.drain(..).collect::<String>().trim().to_string();
            if !msg.is_empty() {
                let tx = state.tx.clone();
                let notes_dir = state.notes_dir.clone();
                state.git_loading = true;
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        crate::git::operations::commit(&notes_dir, &msg)
                    })
                    .await;
                    let mapped = match result {
                        Ok(Ok(())) => Ok(()),
                        Ok(Err(e)) => Err(e.to_string()),
                        Err(e) => Err(e.to_string()),
                    };
                    tx.send(AppEvent::GitOpComplete(mapped)).ok();
                });
                state.mode = Mode::Git;
            }
        }
        KeyCode::Char(c) => state.prompt_input.push(c),
        KeyCode::Backspace => { state.prompt_input.pop(); }
        _ => {}
    }
    Ok(Action::Continue)
}

async fn handle_meetily(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            state.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let len = state.meetily.meetings.len();
            if len > 0 && state.meetily.cursor + 1 < len {
                state.meetily.cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.meetily.cursor = state.meetily.cursor.saturating_sub(1);
        }
        KeyCode::Enter => {
            if let Some(meeting) = state.meetily.meetings.get(state.meetily.cursor).cloned() {
                let notes_dir = state.notes_dir.clone();
                let folder = state.config.meetily.import_folder.clone();
                let tags = state.config.meetily.tags.clone();
                let tx = state.tx.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        crate::import::meetily::write_note(&meeting, &notes_dir, &folder, &tags)
                            .map(|p| (p, meeting.id.clone()))
                    })
                    .await;
                    match result {
                        Ok(Ok((path, meetily_id))) => {
                            tx.send(AppEvent::MeetilyImportDone { path, meetily_id }).ok();
                        }
                        Ok(Err(e)) => { tx.send(AppEvent::Error(format!("Import failed: {e}"))).ok(); }
                        Err(e) => { tx.send(AppEvent::Error(format!("Import task: {e}"))).ok(); }
                    }
                });
            }
        }
        _ => {}
    }
    Ok(Action::Continue)
}

async fn handle_confirm_delete(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(node) = state.selected_file_node().cloned() {
                if !node.is_dir {
                    let path = node.path.clone();
                    let notes_dir = state.notes_dir.clone();
                    let tx = state.tx.clone();
                    let show_hidden = state.config.ui.show_hidden;
                    let index_dir = state.config.index_dir();

                    tokio::spawn(async move {
                        if let Err(e) = std::fs::remove_file(&path) {
                            tx.send(AppEvent::Error(format!("Delete failed: {e}"))).ok();
                            return;
                        }

                        tx.send(AppEvent::NoteDeleted(path.clone())).ok();

                        // Remove from FTS index
                        let rel = path.strip_prefix(&notes_dir)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();
                        tokio::task::spawn_blocking(move || {
                            if let Ok(idx) = crate::search::fulltext::FtsIndex::open_or_create(&index_dir) {
                                idx.delete_note(&rel).ok();
                            }
                        })
                        .await
                        .ok();

                        // Refresh file tree
                        let nodes = tokio::task::spawn_blocking(move || {
                            crate::notes::watcher::scan_dir(&notes_dir, show_hidden)
                        })
                        .await
                        .unwrap_or_default();
                        tx.send(AppEvent::FileTreeRefresh(nodes)).ok();
                    });
                }
            }
            state.mode = Mode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            state.mode = Mode::Normal;
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_settings(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match &state.settings_mode.clone() {
        SettingsMode::Navigating => handle_settings_nav(state, key),
        SettingsMode::EditingText => handle_settings_edit(state, key),
        SettingsMode::PickingModel => handle_settings_pick(state, key),
        SettingsMode::EditingLongText => handle_settings_edit_long(state, key),
    }
}

fn handle_settings_nav(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            state.config.write().ok();
            state.set_status("Settings saved".into(), crate::app::StatusLevel::Success);
            state.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if state.settings_cursor + 1 < sp::TOTAL_FIELDS {
                state.settings_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.settings_cursor = state.settings_cursor.saturating_sub(1);
        }
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
            match state.settings_cursor {
                sp::FIELD_CHAT_PROVIDER => sp::cycle_chat_provider(state),
                sp::FIELD_EMBED_PROVIDER => sp::cycle_embed_provider(state),
                _ => {}
            }
        }
        KeyCode::Enter => {
            if sp::is_provider_field(state.settings_cursor) {
                match state.settings_cursor {
                    sp::FIELD_CHAT_PROVIDER => sp::cycle_chat_provider(state),
                    sp::FIELD_EMBED_PROVIDER => sp::cycle_embed_provider(state),
                    _ => {}
                }
            } else if sp::is_model_field(state.settings_cursor) {
                state.settings_model_cursor = 0;
                state.settings_mode = SettingsMode::PickingModel;
            } else if state.settings_cursor == sp::FIELD_SUMMARIZER_PROMPT {
                // Open full textarea editor for the system prompt.
                let lines: Vec<String> = state
                    .config
                    .summarizer
                    .system_prompt
                    .lines()
                    .map(String::from)
                    .collect();
                state.settings_prompt_editor = ratatui_textarea::TextArea::new(lines);
                state.settings_mode = SettingsMode::EditingLongText;
            } else {
                // Text edit mode: populate edit buf with current raw value
                state.settings_edit_buf = sp::get_field_raw(state, state.settings_cursor);
                state.settings_mode = SettingsMode::EditingText;
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            state.config.write().ok();
            state.set_status("Settings saved".into(), crate::app::StatusLevel::Success);
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            state.tx.send(AppEvent::ForceReembed).ok();
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_settings_edit(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            state.settings_edit_buf.clear();
            state.settings_mode = SettingsMode::Navigating;
        }
        KeyCode::Enter => {
            let value = state.settings_edit_buf.drain(..).collect::<String>();
            let field = state.settings_cursor;
            sp::apply_field_value(state, field, value);
            state.settings_mode = SettingsMode::Navigating;
        }
        KeyCode::Char(c) => {
            state.settings_edit_buf.push(c);
        }
        KeyCode::Backspace => {
            state.settings_edit_buf.pop();
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_settings_pick(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    let model_count = if sp::uses_ollama_models(state.settings_cursor) {
        state.available_ollama_models.len()
    } else {
        state.available_openai_models.len()
    };

    match key.code {
        KeyCode::Esc => {
            state.settings_mode = SettingsMode::Navigating;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if model_count > 0 && state.settings_model_cursor + 1 < model_count {
                state.settings_model_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.settings_model_cursor = state.settings_model_cursor.saturating_sub(1);
        }
        KeyCode::Enter => {
            let selected = if sp::uses_ollama_models(state.settings_cursor) {
                state.available_ollama_models.get(state.settings_model_cursor).cloned()
            } else {
                state.available_openai_models.get(state.settings_model_cursor).cloned()
            };
            if let Some(model) = selected {
                sp::apply_field_value(state, state.settings_cursor, model);
            }
            state.settings_mode = SettingsMode::Navigating;
        }
        _ => {}
    }
    Ok(Action::Continue)
}

fn handle_settings_edit_long(state: &mut AppState, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            // Save prompt content back to config and return to settings nav.
            let prompt = state.settings_prompt_editor.lines().join("\n");
            state.config.summarizer.system_prompt = prompt;
            state.settings_mode = SettingsMode::Navigating;
        }
        _ => {
            // Ctrl+s also saves without closing.
            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('s')
            {
                let prompt = state.settings_prompt_editor.lines().join("\n");
                state.config.summarizer.system_prompt = prompt;
                state.config.write().ok();
                state.set_status("Prompt saved".into(), crate::app::StatusLevel::Success);
            } else {
                state
                    .settings_prompt_editor
                    .input(crossterm::event::Event::Key(key));
            }
        }
    }
    Ok(Action::Continue)
}
