//! `LlmProvider` trait — the pluggable interface for LLM backends.
//!
//! Every LLM provider (Anthropic, OpenAI, Ollama/Gemma, etc.) implements
//! this trait. Agents interact with the LLM exclusively through this
//! abstraction, making the provider swappable via configuration.

use async_trait::async_trait;

use crate::error::AgentError;
use crate::types::{Completion, Prompt};

/// Configuration for selecting and configuring an LLM provider.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderConfig {
    /// Provider name: "anthropic", "ollama", "openai".
    pub provider: String,
    /// Model name (e.g. "claude-sonnet-4-20250514", "gemma4:e2b").
    pub model: String,
    /// API key (for hosted providers). Read from env var if not set.
    pub api_key: Option<String>,
    /// Base URL (for local providers like Ollama).
    pub base_url: Option<String>,
    /// Maximum tokens to generate per request.
    pub max_tokens: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        ProviderConfig {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            api_key: None,
            base_url: None,
            max_tokens: 1024,
        }
    }
}

/// The pluggable LLM provider interface.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// Send a prompt and return the model's completion.
    async fn complete(&self, prompt: &Prompt) -> Result<Completion, AgentError>;
}

/// Create an LLM provider from configuration.
pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn LlmProvider>, AgentError> {
    match config.provider.as_str() {
        "anthropic" => {
            let provider = super::providers::anthropic::AnthropicProvider::new(config)?;
            Ok(Box::new(provider))
        }
        "ollama" => {
            let provider = super::providers::ollama::OllamaProvider::new(config)?;
            Ok(Box::new(provider))
        }
        other => Err(AgentError::Config(format!(
            "unknown provider: {other}. Available: anthropic, ollama"
        ))),
    }
}
