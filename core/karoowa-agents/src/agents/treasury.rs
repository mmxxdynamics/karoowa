//! Treasury Agent (enterprise-only).
//!
//! Monitors the on-chain treasury, reviews grant applications, and tracks
//! disbursements. Gated behind an enterprise license file check.

use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;

use crate::agent::Agent;
use crate::error::AgentError;
use crate::types::{ToolCall, ToolDefinition, ToolResult};

/// The Treasury Agent for enterprise customers.
pub struct TreasuryAgent {
    rpc_url: String,
}

impl TreasuryAgent {
    /// Create a new Treasury Agent. Requires a valid enterprise license file.
    pub fn new(rpc_url: &str, license_file: &PathBuf) -> Result<Self, AgentError> {
        check_enterprise_license(license_file)?;
        Ok(TreasuryAgent {
            rpc_url: rpc_url.to_string(),
        })
    }
}

#[async_trait]
impl Agent for TreasuryAgent {
    fn name(&self) -> &str {
        "treasury"
    }

    fn system_prompt(&self) -> String {
        format!(
            r#"You are the Karoowa Treasury Agent, an enterprise capability for managing the on-chain treasury.

Connected chain: {rpc_url}

Your role:
- Monitor treasury balance and incoming/outgoing flows
- Review grant applications against budget allocations
- Track disbursement schedules and milestone-based payouts
- Generate financial reports for stakeholders
- Flag unusual transaction patterns (large outflows, repeated small drains)

Safety rules:
- NEVER initiate a disbursement without explicit human approval
- ALWAYS verify recipient addresses against an approved list
- ALWAYS log every treasury action to the audit trail
- Escalate any single transfer above the per-tx threshold

This agent is gated to enterprise customers per parent PRD REQ-011 and REQ-012."#,
            rpc_url = self.rpc_url
        )
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "treasury_balance".into(),
                description: "Query the current treasury balance and recent flow summary".into(),
                parameters: json!({"type": "object", "properties": {}}),
            },
            ToolDefinition {
                name: "list_grants".into(),
                description: "List active grants and their disbursement status".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Filter: pending, active, complete, all (default)"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "review_grant".into(),
                description: "Review a grant application against budget and policy constraints"
                    .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "grant_id": {"type": "integer"},
                        "amount": {"type": "integer"},
                        "recipient": {"type": "string"},
                        "milestones": {
                            "type": "array",
                            "items": {"type": "string"}
                        }
                    },
                    "required": ["grant_id", "amount", "recipient"]
                }),
            },
            ToolDefinition {
                name: "schedule_disbursement".into(),
                description: "Schedule a disbursement to a recipient (requires human approval)"
                    .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "recipient": {"type": "string"},
                        "amount": {"type": "integer"},
                        "milestone": {"type": "string"},
                        "approved": {"type": "boolean"}
                    },
                    "required": ["recipient", "amount", "approved"]
                }),
            },
            ToolDefinition {
                name: "financial_report".into(),
                description: "Generate a financial report for a date range".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "from_date": {"type": "string"},
                        "to_date": {"type": "string"}
                    }
                }),
            },
            ToolDefinition {
                name: "flag_anomaly".into(),
                description: "Flag an anomalous transaction or pattern for human review".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "tx_hash": {"type": "string"},
                        "reason": {"type": "string"}
                    },
                    "required": ["reason"]
                }),
            },
        ]
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        match call.name.as_str() {
            "treasury_balance" => Ok(ToolResult {
                name: "treasury_balance".into(),
                output: format!(
                    "Treasury balance from {}: \n\
                     Total: 0 (no chain-side treasury yet)\n\
                     30-day inflow: 0\n\
                     30-day outflow: 0\n\
                     (M6 governance treasury module pending)",
                    self.rpc_url
                ),
                success: true,
            }),
            "list_grants" => {
                let status = call.arguments["status"].as_str().unwrap_or("all");
                Ok(ToolResult {
                    name: "list_grants".into(),
                    output: format!("No grants in '{status}' state. (Grant registry pending M6.)"),
                    success: true,
                })
            }
            "review_grant" => {
                let grant_id = call.arguments["grant_id"].as_u64().unwrap_or(0);
                let amount = call.arguments["amount"].as_u64().unwrap_or(0);
                let recipient = call.arguments["recipient"].as_str().unwrap_or("");
                Ok(ToolResult {
                    name: "review_grant".into(),
                    output: format!(
                        "Grant #{grant_id} review:\n\
                         Recipient: {recipient}\n\
                         Amount: {amount}\n\
                         Recommendation: PENDING (requires human approval and budget allocation check)"
                    ),
                    success: true,
                })
            }
            "schedule_disbursement" => {
                let recipient = call.arguments["recipient"].as_str().unwrap_or("");
                let amount = call.arguments["amount"].as_u64().unwrap_or(0);
                let approved = call.arguments["approved"].as_bool().unwrap_or(false);
                if !approved {
                    return Ok(ToolResult {
                        name: "schedule_disbursement".into(),
                        output: format!(
                            "BLOCKED: Disbursement of {amount} to {recipient} requires human approval. Set approved=true."
                        ),
                        success: false,
                    });
                }
                Ok(ToolResult {
                    name: "schedule_disbursement".into(),
                    output: format!(
                        "Scheduled disbursement of {amount} to {recipient}.\n\
                         (On-chain execution pending M6 treasury module.)"
                    ),
                    success: true,
                })
            }
            "financial_report" => {
                let from = call.arguments["from_date"].as_str().unwrap_or("");
                let to = call.arguments["to_date"].as_str().unwrap_or("");
                Ok(ToolResult {
                    name: "financial_report".into(),
                    output: format!(
                        "Financial report {from}..{to}:\n\
                         No activity. (Treasury module ships in M6.)"
                    ),
                    success: true,
                })
            }
            "flag_anomaly" => {
                let reason = call.arguments["reason"].as_str().unwrap_or("unspecified");
                Ok(ToolResult {
                    name: "flag_anomaly".into(),
                    output: format!("ESCALATED: Anomaly flagged for human review: {reason}"),
                    success: true,
                })
            }
            _ => Err(AgentError::Tool(format!("unknown tool: {}", call.name))),
        }
    }
}

/// Placeholder enterprise license check.
fn check_enterprise_license(path: &PathBuf) -> Result<(), AgentError> {
    if !path.exists() {
        return Err(AgentError::Config(format!(
            "enterprise license file not found at {}. Treasury Agent is enterprise-only.",
            path.display()
        )));
    }
    let metadata = std::fs::metadata(path)
        .map_err(|e| AgentError::Config(format!("cannot read license file: {e}")))?;
    if metadata.len() == 0 {
        return Err(AgentError::Config(
            "enterprise license file is empty".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_license_file() {
        let path = PathBuf::from("/nonexistent/karoowa.license");
        let result = TreasuryAgent::new("http://localhost:8545", &path);
        assert!(matches!(result, Err(AgentError::Config(_))));
    }

    #[test]
    fn accepts_valid_license_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valid.license");
        std::fs::write(&path, b"karoowa-enterprise").unwrap();
        let agent = TreasuryAgent::new("http://localhost:8545", &path).unwrap();
        assert_eq!(agent.name(), "treasury");
    }

    #[test]
    fn tool_definitions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valid.license");
        std::fs::write(&path, b"key").unwrap();
        let agent = TreasuryAgent::new("http://localhost:8545", &path).unwrap();
        let tools = agent.tools();
        assert_eq!(tools.len(), 6);
    }

    #[tokio::test]
    async fn disbursement_blocked_without_approval() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valid.license");
        std::fs::write(&path, b"key").unwrap();
        let agent = TreasuryAgent::new("http://localhost:8545", &path).unwrap();

        let call = ToolCall {
            name: "schedule_disbursement".into(),
            arguments: json!({
                "recipient": "0xabc",
                "amount": 1000000,
                "approved": false
            }),
        };
        let result = agent.execute_tool(&call).await.unwrap();
        assert!(!result.success);
        assert!(result.output.contains("BLOCKED"));
    }

    #[tokio::test]
    async fn disbursement_succeeds_with_approval() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valid.license");
        std::fs::write(&path, b"key").unwrap();
        let agent = TreasuryAgent::new("http://localhost:8545", &path).unwrap();

        let call = ToolCall {
            name: "schedule_disbursement".into(),
            arguments: json!({
                "recipient": "0xabc",
                "amount": 1000000,
                "approved": true
            }),
        };
        let result = agent.execute_tool(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Scheduled"));
    }
}
