//! CLI/Dev Agent — suggests karoowa CLI commands from natural language.
//!
//! Tools: `suggest_command`, `explain_command`

use async_trait::async_trait;
use serde_json::json;

use crate::agent::Agent;
use crate::error::AgentError;
use crate::types::{ToolCall, ToolDefinition, ToolResult};

/// The CLI/Dev Agent for Chain Builders and developers.
pub struct CliDevAgent;

impl CliDevAgent {
    #[must_use]
    pub fn new() -> Self {
        CliDevAgent
    }
}

impl Default for CliDevAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for CliDevAgent {
    fn name(&self) -> &str {
        "dev"
    }

    fn system_prompt(&self) -> String {
        r#"You are the Karoowa CLI/Dev Agent. You help developers use the Karoowa CLI.

Available subcommands:
- `karoowa node` — start a node (--validator-key, --consensus, --data-dir, --rpc-port, --p2p-port, --block-time, --join)
- `karoowa wallet new` — generate a keypair (--output)
- `karoowa wallet address <key-file>` — show address for a key
- `karoowa wallet sign <key-file> <message>` — sign a message
- `karoowa genesis generate` — create genesis config (--validators, --output)
- `karoowa genesis validate <file>` — validate a genesis file
- `karoowa client block-number` — get current block height (--rpc)
- `karoowa client chain-id` — get chain ID
- `karoowa client get-balance <address>` — get balance
- `karoowa client get-nonce <address>` — get nonce
- `karoowa client node-info` — get node info
- `karoowa client peer-count` — get peer count
- `karoowa client syncing` — check sync status
- `karoowa devnet info` — show devnet quickstart
- `karoowa network peers` — show connected peers

Your role:
- Translate natural language requests into the right karoowa CLI command
- Explain what each command does and what its flags mean
- Suggest the most common workflows
- Be concise — output the command first, then a brief explanation"#.to_string()
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "suggest_command".into(),
                description: "Suggest a karoowa CLI command for the user's request".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The full karoowa CLI command to run"
                        },
                        "explanation": {
                            "type": "string",
                            "description": "Brief explanation of what the command does"
                        }
                    },
                    "required": ["command", "explanation"]
                }),
            },
            ToolDefinition {
                name: "explain_command".into(),
                description: "Explain what a karoowa CLI command does in detail".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command to explain"
                        }
                    },
                    "required": ["command"]
                }),
            },
        ]
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        match call.name.as_str() {
            "suggest_command" => {
                let command = call.arguments["command"]
                    .as_str()
                    .unwrap_or("karoowa --help");
                let explanation = call.arguments["explanation"].as_str().unwrap_or("");
                Ok(ToolResult {
                    name: "suggest_command".into(),
                    output: format!("Command: {command}\n\n{explanation}"),
                    success: true,
                })
            }
            "explain_command" => {
                let command = call.arguments["command"]
                    .as_str()
                    .unwrap_or("karoowa --help");
                Ok(ToolResult {
                    name: "explain_command".into(),
                    output: format!("Explaining: {command}"),
                    success: true,
                })
            }
            _ => Err(AgentError::Tool(format!("unknown tool: {}", call.name))),
        }
    }
}
