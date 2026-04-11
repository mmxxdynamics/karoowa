//! Core types for the agent framework.

use serde::{Deserialize, Serialize};

/// A message in a conversation with an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A prompt sent to the LLM provider.
#[derive(Debug, Clone)]
pub struct Prompt {
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// Available tools the model can call.
    pub tools: Vec<ToolDefinition>,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
}

/// Definition of a tool the agent can use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (e.g. "generate_wallet").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub parameters: serde_json::Value,
}

/// The LLM's response.
#[derive(Debug, Clone)]
pub struct Completion {
    /// Text content of the response (may be empty if a tool was called).
    pub content: String,
    /// Tool calls requested by the model.
    pub tool_calls: Vec<ToolCall>,
    /// Token usage stats.
    pub usage: Option<Usage>,
}

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Name of the tool to invoke.
    pub name: String,
    /// JSON arguments for the tool.
    pub arguments: serde_json::Value,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Result of executing a tool.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Name of the tool that was called.
    pub name: String,
    /// Output of the tool execution.
    pub output: String,
    /// Whether the tool execution succeeded.
    pub success: bool,
}
