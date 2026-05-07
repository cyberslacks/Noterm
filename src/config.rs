use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_notes_dir")]
    pub notes_dir: PathBuf,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub import: ImportConfig,
    #[serde(default)]
    pub meetily: MeetilyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetilyConfig {
    /// Path to meeting_minutes.db. Auto-detected if not set.
    pub db_path: Option<PathBuf>,
    /// Subfolder within notes_dir to store imported call notes. Default: "calls"
    #[serde(default = "default_calls_folder")]
    pub import_folder: String,
    /// Poll Meetily DB every N seconds for new meetings (0 = disabled). Default: 0
    #[serde(default)]
    pub auto_sync_secs: u64,
    /// Tags to add to every imported call note
    #[serde(default = "default_call_tags")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    /// Inbox folder to poll for files to auto-import. Default: ~/notes/inbox/
    pub watch_dir: Option<PathBuf>,
    /// How often (seconds) to scan the inbox folder. Default: 5
    #[serde(default = "default_watch_interval")]
    pub watch_interval_secs: u64,
    /// Enable the local REST API for programmatic note creation. Default: false
    #[serde(default)]
    pub api_enabled: bool,
    /// Port the import API listens on. Default: 7373
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    /// Host the API binds to. Default: 127.0.0.1 (localhost only)
    #[serde(default = "default_api_host")]
    pub api_host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_tab_width")]
    pub tab_width: u8,
    #[serde(default = "default_true")]
    pub wrap_lines: bool,
    #[serde(default = "default_true")]
    pub auto_save: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub auto_commit: bool,
    #[serde(default = "default_remote")]
    pub auto_commit_msg: String,
    pub remote: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Ollama,
    Claude,
    OpenAI,
}

/// Which backend to use for generating embeddings (vector search).
/// Claude does not support embeddings, so this is separate from the chat provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EmbedProvider {
    /// Use Ollama (default). Points at `ollama_base_url`.
    Ollama,
    /// Use any OpenAI-compatible API — includes OpenWebUI, LocalAI, etc.
    /// Points at `openai_base_url` with `openai_api_key`.
    OpenAI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider used for chat completions.
    #[serde(default)]
    pub provider: LlmProvider,

    // --- Ollama ---
    #[serde(default = "default_ollama_url")]
    pub ollama_base_url: String,
    #[serde(default = "default_chat_model")]
    pub ollama_chat_model: String,
    #[serde(default = "default_embed_model")]
    pub ollama_embed_model: String,

    // --- Claude ---
    pub claude_api_key: Option<String>,
    #[serde(default = "default_claude_model")]
    pub claude_model: String,

    // --- OpenAI-compatible (also used for OpenWebUI) ---
    pub openai_api_key: Option<String>,
    /// Base URL for the OpenAI-compatible API. Set to your OpenWebUI URL, e.g.
    /// "http://localhost:3000/api" to use OpenWebUI instead of OpenAI.
    #[serde(default = "default_openai_url")]
    pub openai_base_url: String,
    /// Model used for chat when provider = "openai".
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    /// Model used for embeddings when embed_provider = "openai".
    #[serde(default = "default_openai_embed_model")]
    pub openai_embed_model: String,

    // --- Embedding ---
    /// Which provider to use for embeddings (independent of chat provider).
    #[serde(default)]
    pub embed_provider: EmbedProvider,

    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_context_notes")]
    pub max_context_notes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub index_dir: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub auto_index: bool,
    #[serde(default = "default_true")]
    pub embed_on_save: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_tree_width")]
    pub file_tree_width_pct: u16,
    #[serde(default = "default_chat_width")]
    pub chat_width_pct: u16,
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default = "default_date_format")]
    pub date_format: String,
}

// --- defaults ---

fn default_watch_interval() -> u64 { 5 }
fn default_api_port() -> u16 { 7373 }
fn default_api_host() -> String { "127.0.0.1".into() }
fn default_calls_folder() -> String { "calls".into() }
fn default_call_tags() -> Vec<String> { vec!["call".into(), "meeting".into(), "meetily".into()] }

fn default_notes_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("notes")
}

fn default_tab_width() -> u8 { 2 }
fn default_true() -> bool { true }
fn default_remote() -> String { "noterm: auto-save {timestamp}".into() }
fn default_ollama_url() -> String { "http://localhost:11434".into() }
fn default_chat_model() -> String { "llama3.2".into() }
fn default_embed_model() -> String { "nomic-embed-text".into() }
fn default_claude_model() -> String { "claude-sonnet-4-5".into() }
fn default_openai_url() -> String { "https://api.openai.com/v1".into() }
fn default_openai_model() -> String { "gpt-4o".into() }
fn default_openai_embed_model() -> String { "text-embedding-3-small".into() }
fn default_context_notes() -> usize { 5 }
fn default_tree_width() -> u16 { 20 }
fn default_chat_width() -> u16 { 35 }
fn default_date_format() -> String { "%Y-%m-%d %H:%M".into() }
fn default_system_prompt() -> String {
    "You are a helpful assistant with access to the user's notes. \
     Answer questions based on the provided note context. \
     Be concise and reference specific notes when relevant."
        .into()
}

impl Default for LlmProvider {
    fn default() -> Self { LlmProvider::Ollama }
}

impl Default for EmbedProvider {
    fn default() -> Self { EmbedProvider::Ollama }
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::Ollama => write!(f, "Ollama"),
            LlmProvider::Claude => write!(f, "Claude"),
            LlmProvider::OpenAI => write!(f, "OpenAI / WebUI"),
        }
    }
}

impl std::fmt::Display for EmbedProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedProvider::Ollama => write!(f, "Ollama"),
            EmbedProvider::OpenAI => write!(f, "OpenAI / WebUI"),
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: default_tab_width(),
            wrap_lines: true,
            auto_save: true,
        }
    }
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_commit: false,
            auto_commit_msg: default_remote(),
            remote: Some("origin".into()),
            branch: None,
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Ollama,
            ollama_base_url: default_ollama_url(),
            ollama_chat_model: default_chat_model(),
            ollama_embed_model: default_embed_model(),
            claude_api_key: None,
            claude_model: default_claude_model(),
            openai_api_key: None,
            openai_base_url: default_openai_url(),
            openai_model: default_openai_model(),
            openai_embed_model: default_openai_embed_model(),
            embed_provider: EmbedProvider::Ollama,
            system_prompt: default_system_prompt(),
            max_context_notes: default_context_notes(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            index_dir: None,
            auto_index: true,
            embed_on_save: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            file_tree_width_pct: default_tree_width(),
            chat_width_pct: default_chat_width(),
            show_hidden: false,
            date_format: default_date_format(),
        }
    }
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            watch_dir: None,
            watch_interval_secs: default_watch_interval(),
            api_enabled: false,
            api_port: default_api_port(),
            api_host: default_api_host(),
        }
    }
}

impl ImportConfig {
    pub fn resolved_watch_dir(&self) -> PathBuf {
        self.watch_dir.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("notes")
                .join("inbox")
        })
    }
}

impl Default for MeetilyConfig {
    fn default() -> Self {
        Self {
            db_path: None,
            import_folder: default_calls_folder(),
            auto_sync_secs: 0,
            tags: default_call_tags(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notes_dir: default_notes_dir(),
            editor: EditorConfig::default(),
            git: GitConfig::default(),
            llm: LlmConfig::default(),
            search: SearchConfig::default(),
            ui: UiConfig::default(),
            import: ImportConfig::default(),
            meetily: MeetilyConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            let config = Config::default();
            config.write()?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading config from {}", config_path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| "parsing config.toml")?;

        Ok(config)
    }

    pub fn write(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("noterm")
            .join("config.toml")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("noterm")
    }

    pub fn index_dir(&self) -> PathBuf {
        self.search
            .index_dir
            .clone()
            .unwrap_or_else(|| Self::data_dir().join("fts_index"))
    }

    pub fn db_path() -> PathBuf {
        Self::data_dir().join("noterm.db")
    }

    pub fn log_path() -> PathBuf {
        Self::data_dir().join("noterm.log")
    }
}
