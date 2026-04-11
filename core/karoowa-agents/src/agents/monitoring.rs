//! Monitoring Agent — basic node health monitoring.
//!
//! Tools: `read_metrics`, `read_health`, `report_status`

use async_trait::async_trait;
use serde_json::json;

use crate::agent::Agent;
use crate::error::AgentError;
use crate::types::{ToolCall, ToolDefinition, ToolResult};

/// The Monitoring Agent for Validator Operators.
pub struct MonitoringAgent {
    rpc_url: String,
}

impl MonitoringAgent {
    #[must_use]
    pub fn new(rpc_url: &str) -> Self {
        MonitoringAgent {
            rpc_url: rpc_url.to_string(),
        }
    }
}

#[async_trait]
impl Agent for MonitoringAgent {
    fn name(&self) -> &str {
        "monitor"
    }

    fn system_prompt(&self) -> String {
        format!(
            r#"You are the Karoowa Monitoring Agent. You observe a Karoowa node at {rpc_url} and report on its health.

Your role:
- Poll the node's /health and /metrics endpoints
- Summarize the node's status in plain language
- Flag anomalies (peer count = 0, block height not advancing, etc.)
- Suggest remediation actions when problems are detected
- Be concise — operators want quick status, not essays

Use your tools to gather data, then synthesize a summary."#,
            rpc_url = self.rpc_url
        )
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "read_health".into(),
                description: "Read the node's /health endpoint".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDefinition {
                name: "read_metrics".into(),
                description: "Read the node's /metrics endpoint (Prometheus format)".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDefinition {
                name: "report_status".into(),
                description: "Generate a structured status report".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "summary": {
                            "type": "string",
                            "description": "One-line summary of node health"
                        },
                        "issues": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of detected issues (empty if healthy)"
                        }
                    },
                    "required": ["summary"]
                }),
            },
        ]
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        match call.name.as_str() {
            "read_health" => {
                let url = format!("{}/health", self.rpc_url);
                match reqwest::get(&url).await {
                    Ok(resp) => {
                        let body = resp.text().await.unwrap_or_else(|_| "error".into());
                        Ok(ToolResult {
                            name: "read_health".into(),
                            output: body,
                            success: true,
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        name: "read_health".into(),
                        output: format!("Failed to reach health endpoint: {e}"),
                        success: false,
                    }),
                }
            }
            "read_metrics" => {
                let url = format!("{}/metrics", self.rpc_url);
                match reqwest::get(&url).await {
                    Ok(resp) => {
                        let body = resp.text().await.unwrap_or_else(|_| "error".into());
                        Ok(ToolResult {
                            name: "read_metrics".into(),
                            output: body,
                            success: true,
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        name: "read_metrics".into(),
                        output: format!("Failed to reach metrics endpoint: {e}"),
                        success: false,
                    }),
                }
            }
            "report_status" => {
                let summary = call.arguments["summary"]
                    .as_str()
                    .unwrap_or("status unknown");
                let issues: Vec<String> = call.arguments["issues"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let report = if issues.is_empty() {
                    format!("Status: {summary}\nNo issues detected.")
                } else {
                    format!(
                        "Status: {summary}\nIssues:\n{}",
                        issues
                            .iter()
                            .map(|i| format!("  - {i}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                };

                Ok(ToolResult {
                    name: "report_status".into(),
                    output: report,
                    success: true,
                })
            }
            _ => Err(AgentError::Tool(format!("unknown tool: {}", call.name))),
        }
    }
}
