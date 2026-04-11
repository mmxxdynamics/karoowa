//! Onboarding Agent — guides first-time users through setup.
//!
//! Tools: `generate_wallet`, `check_node_running`, `join_devnet`,
//! `wait_for_block`, `explain_error`

use async_trait::async_trait;
use serde_json::json;

use crate::agent::Agent;
use crate::error::AgentError;
use crate::types::{ToolCall, ToolDefinition, ToolResult};

/// The Onboarding Agent for Solo / Hobbyist Operators.
pub struct OnboardingAgent;

impl OnboardingAgent {
    #[must_use]
    pub fn new() -> Self {
        OnboardingAgent
    }
}

impl Default for OnboardingAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for OnboardingAgent {
    fn name(&self) -> &str {
        "onboarding"
    }

    fn system_prompt(&self) -> String {
        r#"You are the Karoowa Onboarding Agent, helping first-time users set up and run a Karoowa node.

Your role:
- Guide users through wallet key generation, node startup, and joining the devnet
- Use the tools available to you to perform actions on behalf of the user
- Explain each step clearly in plain language
- If any step fails, diagnose the problem and suggest a fix
- Be encouraging and patient — this may be the user's first blockchain experience

Steps to guide the user through:
1. Generate a wallet key (tool: generate_wallet)
2. Check if a node is already running (tool: check_node_running)
3. Start a node or join the public devnet (tool: join_devnet)
4. Wait for the first block (tool: wait_for_block)
5. Confirm success

Keep responses concise. Use the tools proactively — don't just describe what to do, do it."#.to_string()
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "generate_wallet".into(),
                description: "Generate a new Karoowa validator keypair and save it to a file"
                    .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "output_path": {
                            "type": "string",
                            "description": "Path to save the key file"
                        }
                    },
                    "required": ["output_path"]
                }),
            },
            ToolDefinition {
                name: "check_node_running".into(),
                description:
                    "Check if a Karoowa node is already running by probing the health endpoint"
                        .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "rpc_url": {
                            "type": "string",
                            "description": "RPC endpoint to check (default: http://localhost:8545)"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "join_devnet".into(),
                description: "Start a Karoowa node and join the public devnet".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "key_file": {
                            "type": "string",
                            "description": "Path to the validator key file"
                        }
                    },
                    "required": ["key_file"]
                }),
            },
            ToolDefinition {
                name: "wait_for_block".into(),
                description: "Wait for the node to produce or receive its first block".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "rpc_url": {
                            "type": "string",
                            "description": "RPC endpoint (default: http://localhost:8545)"
                        },
                        "timeout_secs": {
                            "type": "integer",
                            "description": "Timeout in seconds (default: 60)"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "explain_error".into(),
                description: "Explain a Karoowa error message in plain language and suggest a fix"
                    .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "error_message": {
                            "type": "string",
                            "description": "The error message to explain"
                        }
                    },
                    "required": ["error_message"]
                }),
            },
        ]
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        match call.name.as_str() {
            "generate_wallet" => {
                let output_path = call.arguments["output_path"]
                    .as_str()
                    .unwrap_or("validator.key");
                let kp = karoowa_crypto::Keypair::generate();
                match std::fs::write(output_path, kp.private_key_hex()) {
                    Ok(()) => Ok(ToolResult {
                        name: "generate_wallet".into(),
                        output: format!(
                            "Wallet generated successfully.\nAddress: {}\nKey file: {output_path}",
                            kp.address()
                        ),
                        success: true,
                    }),
                    Err(e) => Ok(ToolResult {
                        name: "generate_wallet".into(),
                        output: format!("Failed to write key file: {e}"),
                        success: false,
                    }),
                }
            }
            "check_node_running" => {
                let rpc_url = call.arguments["rpc_url"]
                    .as_str()
                    .unwrap_or("http://localhost:8545");
                let url = format!("{rpc_url}/health");
                match reqwest::get(&url).await {
                    Ok(resp) if resp.status().is_success() => Ok(ToolResult {
                        name: "check_node_running".into(),
                        output: "Node is running and healthy.".into(),
                        success: true,
                    }),
                    _ => Ok(ToolResult {
                        name: "check_node_running".into(),
                        output: "No node detected at the default endpoint.".into(),
                        success: true,
                    }),
                }
            }
            "join_devnet" => {
                // In M1, this is a placeholder — the agent describes the command
                // rather than executing it (to avoid spawning a long-running process).
                let key_file = call.arguments["key_file"]
                    .as_str()
                    .unwrap_or("validator.key");
                Ok(ToolResult {
                    name: "join_devnet".into(),
                    output: format!(
                        "To start a node and join the devnet, run:\n\n  karoowa node --validator-key {key_file} --consensus poa --join public-devnet\n\nThe node will connect to the public devnet bootnodes automatically."
                    ),
                    success: true,
                })
            }
            "wait_for_block" => {
                let rpc_url = call.arguments["rpc_url"]
                    .as_str()
                    .unwrap_or("http://localhost:8545");
                let timeout = call.arguments["timeout_secs"].as_u64().unwrap_or(60);

                let client = reqwest::Client::new();
                let deadline =
                    tokio::time::Instant::now() + std::time::Duration::from_secs(timeout);

                loop {
                    if tokio::time::Instant::now() > deadline {
                        return Ok(ToolResult {
                            name: "wait_for_block".into(),
                            output: format!("Timed out after {timeout}s waiting for a block."),
                            success: false,
                        });
                    }

                    let resp = client
                        .post(format!("{rpc_url}/rpc"))
                        .json(&json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "kw_blockNumber",
                            "params": []
                        }))
                        .send()
                        .await;

                    if let Ok(r) = resp {
                        if let Ok(body) = r.json::<serde_json::Value>().await {
                            if let Some(height) = body["result"].as_u64() {
                                if height > 0 {
                                    return Ok(ToolResult {
                                        name: "wait_for_block".into(),
                                        output: format!(
                                            "Block height is {height}. Node is producing blocks!"
                                        ),
                                        success: true,
                                    });
                                }
                            }
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
            "explain_error" => {
                let error_msg = call.arguments["error_message"]
                    .as_str()
                    .unwrap_or("unknown error");
                // The LLM will interpret this tool result and provide a human explanation.
                Ok(ToolResult {
                    name: "explain_error".into(),
                    output: format!("Error to explain: {error_msg}"),
                    success: true,
                })
            }
            _ => Err(AgentError::Tool(format!("unknown tool: {}", call.name))),
        }
    }
}
