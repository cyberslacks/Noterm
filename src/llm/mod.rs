pub mod claude;
pub mod ollama;
pub mod openai;
pub mod context;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::config::{LlmConfig, LlmProvider};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: ChatRole::User, content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: ChatRole::Assistant, content: content.into() }
    }
}

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat_stream(&self, messages: Vec<ChatMessage>, system: &str) -> Result<TokenStream>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

pub fn make_client(config: &LlmConfig) -> Box<dyn LlmClient> {
    match config.provider {
        LlmProvider::Ollama => Box::new(ollama::OllamaClient::new(config)),
        LlmProvider::Claude => Box::new(claude::ClaudeClient::new(config)),
        LlmProvider::OpenAI => Box::new(openai::OpenAiClient::new(config)),
    }
}
