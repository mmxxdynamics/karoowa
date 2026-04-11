//! Anthropic Claude provider.
//!
//! Talks to the Anthropic Messages API over HTTPS. Supports tool use.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::debug;

use crate::error::AgentError;
use crate::provider::{LlmProvider, ProviderConfig};
use crate::types::{Completion, Prompt, Role, ToolCall, Usage};

const DEFAULT_API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    api_url: String,
}

impl AnthropicProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, AgentError> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| {
                AgentError::Config(
                    "Anthropic API key required. Set ANTHROPIC_API_KEY env var or provide api_key in config.".into(),
                )
            })?;

        let api_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_API_URL.to_string());

        Ok(AnthropicProvider {
            client: Client::new(),
            api_key,
            model: config.model.clone(),
            api_url,
        })
    }

    fn build_request_body(&self, prompt: &Prompt) -> Value {
        // Separate system message from conversation.
        let system_msg: Option<String> = prompt
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());

        let messages: Vec<Value> = prompt
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::User | Role::Tool => "user",
                        Role::Assistant => "assistant",
                        Role::System => unreachable!(),
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = json!({
            "model": self.model,
            "max_tokens": prompt.max_tokens,
            "messages": messages,
        });

        if let Some(sys) = system_msg {
            body["system"] = json!(sys);
        }

        // Add tools if any are defined.
        if !prompt.tools.is_empty() {
            let tools: Vec<Value> = prompt
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            body["tools"] = json!(tools);
        }

        body
    }

    fn parse_response(&self, body: &Value) -> Result<Completion, AgentError> {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        if let Some(blocks) = body["content"].as_array() {
            for block in blocks {
                match block["type"].as_str() {
                    Some("text") => {
                        if let Some(text) = block["text"].as_str() {
                            content.push_str(text);
                        }
                    }
                    Some("tool_use") => {
                        if let (Some(name), Some(input)) =
                            (block["name"].as_str(), block.get("input"))
                        {
                            tool_calls.push(ToolCall {
                                name: name.to_string(),
                                arguments: input.clone(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        let usage = body.get("usage").map(|u| Usage {
            input_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(Completion {
            content,
            tool_calls,
            usage,
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn complete(&self, prompt: &Prompt) -> Result<Completion, AgentError> {
        let body = self.build_request_body(prompt);

        debug!(model = %self.model, "sending Anthropic request");

        let resp = self
            .client
            .post(&self.api_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let resp_body: Value = resp.json().await?;

        if !status.is_success() {
            let error_msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("unknown error");
            return Err(AgentError::Provider(format!(
                "Anthropic API error ({status}): {error_msg}"
            )));
        }

        self.parse_response(&resp_body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    #[test]
    fn build_request_body_basic() {
        let config = ProviderConfig {
            api_key: Some("test-key".into()),
            model: "claude-sonnet-4-20250514".into(),
            ..Default::default()
        };
        let provider = AnthropicProvider::new(&config).unwrap();

        let prompt = Prompt {
            messages: vec![
                Message {
                    role: Role::System,
                    content: "You are a helpful assistant.".into(),
                },
                Message {
                    role: Role::User,
                    content: "Hello".into(),
                },
            ],
            tools: vec![],
            max_tokens: 100,
        };

        let body = provider.build_request_body(&prompt);
        assert_eq!(body["model"], "claude-sonnet-4-20250514");
        assert_eq!(body["system"], "You are a helpful assistant.");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn parse_text_response() {
        let config = ProviderConfig {
            api_key: Some("test-key".into()),
            ..Default::default()
        };
        let provider = AnthropicProvider::new(&config).unwrap();

        let body = json!({
            "content": [
                {"type": "text", "text": "Hello!"}
            ],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });

        let completion = provider.parse_response(&body).unwrap();
        assert_eq!(completion.content, "Hello!");
        assert!(completion.tool_calls.is_empty());
        assert_eq!(completion.usage.unwrap().output_tokens, 5);
    }

    #[test]
    fn parse_tool_use_response() {
        let config = ProviderConfig {
            api_key: Some("test-key".into()),
            ..Default::default()
        };
        let provider = AnthropicProvider::new(&config).unwrap();

        let body = json!({
            "content": [
                {
                    "type": "tool_use",
                    "name": "generate_wallet",
                    "input": {"output_path": "test.key"}
                }
            ]
        });

        let completion = provider.parse_response(&body).unwrap();
        assert_eq!(completion.tool_calls.len(), 1);
        assert_eq!(completion.tool_calls[0].name, "generate_wallet");
    }
}
