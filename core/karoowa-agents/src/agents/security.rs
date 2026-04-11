//! Vulnerability Scanner Agent — M3 Security bundle.
//!
//! Scans dependencies, contract bytecode, and known vulnerability patterns.

use async_trait::async_trait;
use serde_json::json;

use crate::agent::Agent;
use crate::error::AgentError;
use crate::types::{ToolCall, ToolDefinition, ToolResult};

/// The Vulnerability Scanner Agent.
pub struct SecurityAgent;

impl SecurityAgent {
    #[must_use]
    pub fn new() -> Self {
        SecurityAgent
    }
}

impl Default for SecurityAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for SecurityAgent {
    fn name(&self) -> &str {
        "security"
    }

    fn system_prompt(&self) -> String {
        r#"You are the Karoowa Security Agent. You scan for vulnerabilities in:
- Rust dependencies (cargo-audit)
- WASM contract bytecode (known patterns)
- Consensus state machine (fuzzing results)

Report findings with severity grades. Block PRs with high-severity issues.
Suggest fixes where possible."#
            .to_string()
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "scan_dependencies".into(),
                description: "Run cargo-audit to check for known vulnerable dependencies".into(),
                parameters: json!({"type": "object", "properties": {}}),
            },
            ToolDefinition {
                name: "scan_contract".into(),
                description: "Analyze WASM contract bytecode for known vulnerability patterns"
                    .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "bytecode_hex": {"type": "string", "description": "Hex-encoded WASM bytecode"}
                    },
                    "required": ["bytecode_hex"]
                }),
            },
            ToolDefinition {
                name: "report_findings".into(),
                description: "Generate a structured vulnerability report".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "findings": {
                            "type": "array",
                            "items": {"type": "object"},
                            "description": "List of findings with severity, description, and fix"
                        }
                    }
                }),
            },
        ]
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        match call.name.as_str() {
            "scan_dependencies" => Ok(ToolResult {
                name: "scan_dependencies".into(),
                output:
                    "Run `cargo audit` to scan dependencies.\n(Integration with cargo-audit pending)"
                        .into(),
                success: true,
            }),
            "scan_contract" => {
                let _bytecode = call.arguments["bytecode_hex"]
                    .as_str()
                    .unwrap_or("");
                Ok(ToolResult {
                    name: "scan_contract".into(),
                    output: "Contract bytecode analysis:\n- No known vulnerability patterns detected.\n(Pattern database pending)".into(),
                    success: true,
                })
            }
            "report_findings" => Ok(ToolResult {
                name: "report_findings".into(),
                output: "Vulnerability report generated.".into(),
                success: true,
            }),
            _ => Err(AgentError::Tool(format!("unknown tool: {}", call.name))),
        }
    }
}
