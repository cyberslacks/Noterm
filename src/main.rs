mod app;
mod config;
mod db;
mod error;
mod git;
mod import;
mod llm;
mod notes;
mod search;
mod tasks;
mod tui;

use anyhow::Result;
use crossterm::event::EventStream;
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

use app::{AppEvent, AppState, Mode};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config first (creates default if missing)
    let config = Config::load()?;

    // Ensure notes directory exists
    std::fs::create_dir_all(&config.notes_dir)?;

    // Open SQLite database
    let db_path = Config::db_path();
    let db = db::open(&db_path)?;

    // Set up async event channel
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Create app state
    let mut app = AppState::new(config, tx.clone());

    // Initial file tree scan
    {
        let notes_dir = app.notes_dir.clone();
        let show_hidden = app.config.ui.show_hidden;
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let nodes = tokio::task::spawn_blocking(move || {
                notes::watcher::scan_dir(&notes_dir, show_hidden)
            })
            .await
            .unwrap_or_default();
            tx2.send(AppEvent::FileTreeRefresh(nodes)).ok();
        });
    }

    // Background index all existing notes on startup
    {
        let notes_dir = app.notes_dir.clone();
        let index_dir = app.config.index_dir();
        let show_hidden = app.config.ui.show_hidden;
        let tx2 = tx.clone();
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                let idx = crate::search::fulltext::FtsIndex::open_or_create(&index_dir)?;
                let nodes = notes::watcher::scan_dir(&notes_dir, show_hidden);
                for node in nodes.iter().filter(|n| !n.is_dir) {
                    if let Ok(note) = notes::Note::from_path(&node.path, &notes_dir) {
                        let title = note.frontmatter.title.clone().unwrap_or_default();
                        let tags = note.frontmatter.tags.clone().unwrap_or_default();
                        idx.index_note(&note.relative_path, &title, &note.body, &tags).ok();
                    }
                }
                anyhow::Ok(())
            })
            .await
            .ok();
            tx2.send(AppEvent::IndexingComplete).ok();
        });
    }

    // Start inbox folder watcher (runs forever in background)
    {
        let inbox_dir = app.config.import.resolved_watch_dir();
        let notes_dir = app.notes_dir.clone();
        let interval = app.config.import.watch_interval_secs;
        let show_hidden = app.config.ui.show_hidden;
        let tx2 = tx.clone();
        tokio::spawn(import::watcher::run_inbox_watcher(
            inbox_dir, notes_dir, interval, show_hidden, tx2,
        ));
    }

    // Start local REST API if enabled in config
    if app.config.import.api_enabled {
        let api_state = import::api::ApiState {
            notes_dir: app.notes_dir.clone(),
            show_hidden: app.config.ui.show_hidden,
            tx: tx.clone(),
        };
        let host = app.config.import.api_host.clone();
        let port = app.config.import.api_port;
        tokio::spawn(async move {
            if let Err(e) = import::api::start(&host, port, api_state).await {
                eprintln!("Import API error: {e}");
            }
        });
    }

    // Initialize TUI
    let mut terminal = tui::init()?;

    // Event stream from crossterm
    let mut event_stream = EventStream::new();

    // 100ms tick for animations / periodic tasks
    let mut tick = time::interval(Duration::from_millis(100));

    // Search debounce tracking
    let mut last_search_query = String::new();
    let mut search_debounce: Option<time::Instant> = None;

    // Track whether we've already spawned a chat task for current loading state
    let mut chat_task_spawned = false;

    loop {
        terminal.draw(|f| tui::renderer::render(f, &mut app))?;

        tokio::select! {
            // Keyboard / mouse events from crossterm
            Some(Ok(event)) = event_stream.next() => {
                match tui::keys::handle_event(&mut app, event).await? {
                    tui::keys::Action::Quit => break,
                    tui::keys::Action::Continue => {}
                }

                // Trigger search when query changes in Search mode
                if app.mode == Mode::Search && app.search_query != last_search_query {
                    last_search_query = app.search_query.clone();
                    search_debounce = Some(time::Instant::now());
                }

                // Trigger LLM chat send when loading flag set
                if app.chat_loading && !chat_task_spawned {
                    if let Some(last) = app.chat_messages.last() {
                        if last.role == llm::ChatRole::User {
                            chat_task_spawned = true;
                            let messages = app.chat_messages.clone();
                            let config = app.config.llm.clone();
                            let current_note = app.current_note.clone();
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                send_chat(messages, config, current_note, tx2).await;
                            });
                        }
                    }
                }
                if !app.chat_loading {
                    chat_task_spawned = false;
                }

                // Trigger vector search
                if app.mode == Mode::VectorSearch && app.vsearch_loading {
                    let query = app.vsearch_query.clone();
                    let config = app.config.llm.clone();
                    let db2 = db.clone();
                    let tx2 = tx.clone();
                    app.vsearch_loading = false; // prevent re-trigger
                    tokio::spawn(async move {
                        run_vector_search(query, config, db2, tx2).await;
                    });
                }
            }

            // Events from background tasks (LLM chunks, git results, etc.)
            Some(event) = rx.recv() => {
                // Reset chat_task_spawned when done
                if matches!(event, AppEvent::ChatDone | AppEvent::ChatError(_)) {
                    chat_task_spawned = false;
                }
                app.handle_app_event(event);
            }

            // Periodic tick
            _ = tick.tick() => {
                // Fire debounced search
                if let Some(t) = search_debounce {
                    if t.elapsed() >= Duration::from_millis(300) {
                        search_debounce = None;
                        let query = app.search_query.clone();
                        if !query.is_empty() {
                            let index_dir = app.config.index_dir();
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                run_fts_search(query, index_dir, tx2).await;
                            });
                        }
                    }
                }
            }
        }
    }

    tui::restore()?;
    Ok(())
}

async fn run_fts_search(
    query: String,
    index_dir: std::path::PathBuf,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let result = tokio::task::spawn_blocking(move || {
        let index = search::fulltext::FtsIndex::open_or_create(&index_dir)?;
        index.search(&query, 20)
    })
    .await;

    match result {
        Ok(Ok(results)) => { tx.send(AppEvent::SearchResults(results)).ok(); }
        Ok(Err(e)) => { tx.send(AppEvent::Error(format!("Search error: {e}"))).ok(); }
        Err(e) => { tx.send(AppEvent::Error(format!("Search task error: {e}"))).ok(); }
    }
}

async fn run_vector_search(
    query: String,
    llm_config: config::LlmConfig,
    db: db::Db,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let client = llm::make_client(&llm_config);
    match client.embed(&query).await {
        Ok(embedding) => {
            let model = llm_config.ollama_embed_model.clone();
            let result = tokio::task::spawn_blocking(move || {
                search::vector::top_k_similar(&db, &embedding, &model, 10)
            })
            .await;

            match result {
                Ok(Ok(results)) => { tx.send(AppEvent::VectorSearchResults(results)).ok(); }
                Ok(Err(e)) => { tx.send(AppEvent::Error(format!("Vector search: {e}"))).ok(); }
                Err(e) => { tx.send(AppEvent::Error(format!("Vector task: {e}"))).ok(); }
            }
        }
        Err(e) => {
            tx.send(AppEvent::Error(format!("Embed error: {e}"))).ok();
            tx.send(AppEvent::VectorSearchResults(vec![])).ok();
        }
    }
}

async fn send_chat(
    messages: Vec<llm::ChatMessage>,
    llm_config: config::LlmConfig,
    current_note: Option<notes::Note>,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let context_notes: Vec<notes::Note> = current_note.into_iter().collect();
    let system = llm::context::build_system_prompt(&llm_config, &context_notes);
    let client = llm::make_client(&llm_config);

    match client.chat_stream(messages, &system).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(token) => { tx.send(AppEvent::ChatChunk(token)).ok(); }
                    Err(e) => { tx.send(AppEvent::ChatError(e.to_string())).ok(); return; }
                }
            }
            tx.send(AppEvent::ChatDone).ok();
        }
        Err(e) => {
            tx.send(AppEvent::ChatError(e.to_string())).ok();
        }
    }
}
