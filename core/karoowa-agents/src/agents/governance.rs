//! Governance Agent (enterprise-only).
//!
//! Manages on-chain governance proposals: drafting, submission, vote
//! monitoring, execution scheduling, and compliance reporting.
//!
//! Gated behind an enterprise license file check at construction time.

use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;

use crate::agent::Agent;
use crate::error::AgentError;
use crate::types::{ToolCall, ToolDefinition, ToolResult};

/// The Governance Agent for enterprise customers.
pub struct GovernanceAgent {
    /// RPC endpoint of the chain to govern.
    rpc_url: String,
}

impl GovernanceAgent {
    /// Create a new Governance Agent. Requires a valid enterprise license file.
    ///
    /// Returns `AgentError::Config` if the license file does not exist or
    /// is empty (placeholder check until full license verification ships
    /// in M6 Phase 6.3).
    pub fn new(rpc_url: &str, license_file: &PathBuf) -> Result<Self, AgentError> {
        check_enterprise_license(license_file)?;
        Ok(GovernanceAgent {
            rpc_url: rpc_url.to_string(),
        })
    }
}

#[async_trait]
impl Agent for GovernanceAgent {
    fn name(&self) -> &str {
        "governance"
    }

    fn system_prompt(&self) -> String {
        format!(
            r#"You are the Karoowa Governance Agent, an enterprise capability for managing on-chain governance.

Connected chain: {rpc_url}

Your role:
- Draft new governance proposals (parameter changes, treasury disbursements)
- Monitor active proposals and report vote progress
- Schedule executions of passed proposals (respecting timelock windows)
- Generate compliance reports (proposal audit trails, vote logs)
- Flag proposals that violate the GovernableParams registry constraints

Safety rules:
- NEVER submit a proposal without explicit human approval
- ALWAYS validate parameter changes against the GovernableParams registry
- ALWAYS log every governance action to the audit trail
- Escalate any proposal with quorum risk or unusual voting patterns

This agent is gated to enterprise customers per parent PRD REQ-011 and REQ-012."#,
            rpc_url = self.rpc_url
        )
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "list_proposals".into(),
                description: "List active and recent governance proposals".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Filter by status: voting, passed, rejected, executed, all (default)"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "draft_proposal".into(),
                description: "Draft a new governance proposal (does NOT submit it)".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "description": {"type": "string"},
                        "parameter_changes": {
                            "type": "object",
                            "description": "Map of parameter name → new value"
                        }
                    },
                    "required": ["title", "description"]
                }),
            },
            ToolDefinition {
                name: "monitor_votes".into(),
                description: "Report vote progress on a specific proposal".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "proposal_id": {"type": "integer"}
                    },
                    "required": ["proposal_id"]
                }),
            },
            ToolDefinition {
                name: "schedule_execution".into(),
                description:
                    "Schedule on-chain execution of a passed proposal after the timelock window"
                        .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "proposal_id": {"type": "integer"},
                        "approved": {
                            "type": "boolean",
                            "description": "Whether a human has approved this execution"
                        }
                    },
                    "required": ["proposal_id", "approved"]
                }),
            },
            ToolDefinition {
                name: "compliance_report".into(),
                description: "Generate a compliance report for a date range or proposal range"
                    .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "from_date": {"type": "string"},
                        "to_date": {"type": "string"}
                    }
                }),
            },
        ]
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        match call.name.as_str() {
            "list_proposals" => {
                let status = call.arguments["status"].as_str().unwrap_or("all");
                Ok(ToolResult {
                    name: "list_proposals".into(),
                    output: format!(
                        "Listing proposals with status='{status}' from {}.\n\
                         (Governance module integration ships in M6 Phase 6.0)",
                        self.rpc_url
                    ),
                    success: true,
                })
            }
            "draft_proposal" => {
                let title = call.arguments["title"].as_str().unwrap_or("untitled");
                let description = call.arguments["description"]
                    .as_str()
                    .unwrap_or("(no description)");
                Ok(ToolResult {
                    name: "draft_proposal".into(),
                    output: format!(
                        "Draft proposal:\n\
                         Title: {title}\n\
                         Description: {description}\n\n\
                         BLOCKED: Submission requires human review and the M6 governance module."
                    ),
                    success: true,
                })
            }
            "monitor_votes" => {
                let proposal_id = call.arguments["proposal_id"].as_u64().unwrap_or(0);
                Ok(ToolResult {
                    name: "monitor_votes".into(),
                    output: format!(
                        "Monitoring proposal {proposal_id} votes from {}.\n\
                         (M6 governance integration pending)",
                        self.rpc_url
                    ),
                    success: true,
                })
            }
            "schedule_execution" => {
                let proposal_id = call.arguments["proposal_id"].as_u64().unwrap_or(0);
                let approved = call.arguments["approved"].as_bool().unwrap_or(false);
                if !approved {
                    return Ok(ToolResult {
                        name: "schedule_execution".into(),
                        output: format!(
                            "BLOCKED: Execution of proposal {proposal_id} requires human approval. Set approved=true."
                        ),
                        success: false,
                    });
                }
                Ok(ToolResult {
                    name: "schedule_execution".into(),
                    output: format!(
                        "Scheduling execution of proposal {proposal_id} after timelock window.\n\
                         (Governance module ships in M6 Phase 6.0)"
                    ),
                    success: true,
                })
            }
            "compliance_report" => {
                let from = call.arguments["from_date"].as_str().unwrap_or("");
                let to = call.arguments["to_date"].as_str().unwrap_or("");
                Ok(ToolResult {
                    name: "compliance_report".into(),
                    output: format!(
                        "Compliance report for {from}..{to}:\n\
                         No proposals in date range.\n\
                         (Audit log integration via M2 Observability Agent)"
                    ),
                    success: true,
                })
            }
            _ => Err(AgentError::Tool(format!("unknown tool: {}", call.name))),
        }
    }
}

/// Placeholder enterprise license check.
///
/// Real verification (signed license file with feature gates) ships in
/// M6 Phase 6.3. For now, the file just needs to exist and be non-empty.
fn check_enterprise_license(path: &PathBuf) -> Result<(), AgentError> {
    if !path.exists() {
        return Err(AgentError::Config(format!(
            "enterprise license file not found at {}. Governance Agent is enterprise-only.",
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
    use std::io::Write;

    #[test]
    fn rejects_missing_license_file() {
        let path = PathBuf::from("/nonexistent/karoowa.license");
        let result = GovernanceAgent::new("http://localhost:8545", &path);
        assert!(matches!(result, Err(AgentError::Config(_))));
    }

    #[test]
    fn rejects_empty_license_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.license");
        std::fs::write(&path, b"").unwrap();
        let result = GovernanceAgent::new("http://localhost:8545", &path);
        assert!(matches!(result, Err(AgentError::Config(_))));
    }

    #[test]
    fn accepts_valid_license_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valid.license");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"karoowa-enterprise-2026").unwrap();

        let agent = GovernanceAgent::new("http://localhost:8545", &path).unwrap();
        assert_eq!(agent.name(), "governance");
    }

    #[test]
    fn tool_definitions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valid.license");
        std::fs::write(&path, b"key").unwrap();

        let agent = GovernanceAgent::new("http://localhost:8545", &path).unwrap();
        let tools = agent.tools();
        assert_eq!(tools.len(), 5);
        assert!(tools.iter().any(|t| t.name == "list_proposals"));
        assert!(tools.iter().any(|t| t.name == "draft_proposal"));
        assert!(tools.iter().any(|t| t.name == "monitor_votes"));
        assert!(tools.iter().any(|t| t.name == "schedule_execution"));
        assert!(tools.iter().any(|t| t.name == "compliance_report"));
    }

    #[tokio::test]
    async fn schedule_execution_blocked_without_approval() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valid.license");
        std::fs::write(&path, b"key").unwrap();
        let agent = GovernanceAgent::new("http://localhost:8545", &path).unwrap();

        let call = ToolCall {
            name: "schedule_execution".into(),
            arguments: json!({"proposal_id": 1, "approved": false}),
        };
        let result = agent.execute_tool(&call).await.unwrap();
        assert!(!result.success);
        assert!(result.output.contains("BLOCKED"));
    }
}
