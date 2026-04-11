//! Ollama provider — local LLM via Ollama's HTTP API.
//!
//! Supports any model Ollama can serve, including:
//! - **Gemma 4 E2B (5B)** — hobbyist no-key fallback
//! - **Gemma 4 E4B (8B)** — for hosts with more RAM
//! - Any GGUF-compatible model

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::debug;

use crate::error::AgentError;
use crate::provider::{LlmProvider, ProviderConfig};
use crate::types::{Completion, Prompt, Role, ToolCall, Usage};

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

pub struct OllamaProvider {
    client: Client,
    model: String,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, AgentError> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string());

        Ok(OllamaProvider {
            client: Client::new(),
            model: config.model.clone(),
            base_url,
        })
    }

    fn build_messages(&self, prompt: &Prompt) -> Vec<Value> {
        prompt
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "user",
                    },
                    "content": m.content,
                })
            })
            .collect()
    }

    fn build_tools(&self, prompt: &Prompt) -> Vec<Value> {
        prompt
            .tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect()
    }

    fn parse_response(&self, body: &Value) -> Result<Completion, AgentError> {
        let message = &body["message"];
        let content = message["content"].as_str().unwrap_or("").to_string();

        let mut tool_calls = Vec::new();
        if let Some(calls) = message["tool_calls"].as_array() {
            for call in calls {
                if let Some(func) = call.get("function") {
                    let name = func["name"].as_str().unwrap_or("").to_string();
                    let arguments = func.get("arguments").cloned().unwrap_or(json!({}));
                    tool_calls.push(ToolCall { name, arguments });
                }
            }
        }

        let usage = body.get("eval_count").map(|ec| Usage {
            input_tokens: body["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
            output_tokens: ec.as_u64().unwrap_or(0) as u32,
        });

        Ok(Completion {
            content,
            tool_calls,
            usage,
        })
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn complete(&self, prompt: &Prompt) -> Result<Completion, AgentError> {
        let url = format!("{}/api/chat", self.base_url);

        let messages = self.build_messages(prompt);
        let tools = self.build_tools(prompt);

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });

        if !tools.is_empty() {
            body["tools"] = json!(tools);
        }

        debug!(model = %self.model, url = %url, "sending Ollama request");

        let resp = self.client.post(&url).json(&body).send().await?;

        let status = resp.status();
        let resp_body: Value = resp.json().await?;

        if !status.is_success() {
            let error_msg = resp_body["error"].as_str().unwrap_or("unknown error");
            return Err(AgentError::Provider(format!(
                "Ollama API error ({status}): {error_msg}"
            )));
        }

        self.parse_response(&resp_body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_defaults() {
        let config = ProviderConfig {
            provider: "ollama".into(),
            model: "gemma4:e2b".into(),
            ..Default::default()
        };
        let provider = OllamaProvider::new(&config).unwrap();
        assert_eq!(provider.model, "gemma4:e2b");
        assert_eq!(provider.base_url, DEFAULT_OLLAMA_URL);
    }

    #[test]
    fn parse_text_response() {
        let config = ProviderConfig {
            provider: "ollama".into(),
            model: "test".into(),
            ..Default::default()
        };
        let provider = OllamaProvider::new(&config).unwrap();

        let body = json!({
            "message": {
                "role": "assistant",
                "content": "Hello from Gemma!"
            },
            "eval_count": 10,
            "prompt_eval_count": 5,
        });

        let completion = provider.parse_response(&body).unwrap();
        assert_eq!(completion.content, "Hello from Gemma!");
        assert!(completion.tool_calls.is_empty());
        assert_eq!(completion.usage.unwrap().output_tokens, 10);
    }

    #[test]
    fn parse_tool_call_response() {
        let config = ProviderConfig {
            provider: "ollama".into(),
            model: "test".into(),
            ..Default::default()
        };
        let provider = OllamaProvider::new(&config).unwrap();

        let body = json!({
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "function": {
                            "name": "read_metrics",
                            "arguments": {"endpoint": "/metrics"}
                        }
                    }
                ]
            }
        });

        let completion = provider.parse_response(&body).unwrap();
        assert_eq!(completion.tool_calls.len(), 1);
        assert_eq!(completion.tool_calls[0].name, "read_metrics");
    }
}
