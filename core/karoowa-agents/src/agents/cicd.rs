//! CI/CD & Deployment Agent — manages releases and deployments.
//!
//! Tools: `read_release_artifacts`, `deploy_to_target`, `rollback`,
//! `verify_deployment`, `list_releases`

use async_trait::async_trait;
use serde_json::json;

use crate::agent::Agent;
use crate::error::AgentError;
use crate::types::{ToolCall, ToolDefinition, ToolResult};

/// The CI/CD & Deployment Agent for Operator teams.
pub struct CiCdAgent {
    /// GitHub repository (owner/repo format).
    repo: String,
}

impl CiCdAgent {
    #[must_use]
    pub fn new(repo: &str) -> Self {
        CiCdAgent {
            repo: repo.to_string(),
        }
    }
}

#[async_trait]
impl Agent for CiCdAgent {
    fn name(&self) -> &str {
        "cicd"
    }

    fn system_prompt(&self) -> String {
        format!(
            r#"You are the Karoowa CI/CD & Deployment Agent. You manage releases and deployments for the {repo} repository.

Your role:
- List available releases and their artifacts
- Deploy a specific release to a target server
- Verify deployments are healthy after deploy
- Roll back to a previous version if a deployment fails
- Always present a deployment plan and wait for human approval before executing

Safety rules:
- NEVER deploy without explicit human approval
- ALWAYS verify the health endpoint after deployment
- Keep an audit log of every deployment action
- If verification fails, suggest rollback immediately

Available deployment targets are configured by the operator."#,
            repo = self.repo
        )
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "list_releases".into(),
                description: "List available GitHub releases for the repository".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum releases to list (default: 5)"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "read_release_artifacts".into(),
                description: "List download URLs for a specific release's binary artifacts".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "tag": {
                            "type": "string",
                            "description": "Release tag (e.g. v0.1.0)"
                        }
                    },
                    "required": ["tag"]
                }),
            },
            ToolDefinition {
                name: "deploy_to_target".into(),
                description: "Deploy a release to a target server via SSH".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "tag": {
                            "type": "string",
                            "description": "Release tag to deploy"
                        },
                        "target": {
                            "type": "string",
                            "description": "Target server (e.g. devnet-1, production)"
                        },
                        "approved": {
                            "type": "boolean",
                            "description": "Whether the human has approved this deployment"
                        }
                    },
                    "required": ["tag", "target", "approved"]
                }),
            },
            ToolDefinition {
                name: "verify_deployment".into(),
                description: "Check health of a deployed node".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "Target server to verify"
                        },
                        "rpc_url": {
                            "type": "string",
                            "description": "RPC endpoint to check"
                        }
                    },
                    "required": ["target"]
                }),
            },
            ToolDefinition {
                name: "rollback".into(),
                description: "Roll back to a previous version on a target server".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "Target server"
                        },
                        "to_tag": {
                            "type": "string",
                            "description": "Tag to roll back to"
                        }
                    },
                    "required": ["target", "to_tag"]
                }),
            },
        ]
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        match call.name.as_str() {
            "list_releases" => {
                let limit = call.arguments["limit"].as_u64().unwrap_or(5);
                // In production, this would call the GitHub API.
                Ok(ToolResult {
                    name: "list_releases".into(),
                    output: format!(
                        "Listing latest {limit} releases for {}.\n\
                         (GitHub API integration pending — use `gh release list` manually)",
                        self.repo
                    ),
                    success: true,
                })
            }
            "read_release_artifacts" => {
                let tag = call.arguments["tag"].as_str().unwrap_or("latest");
                Ok(ToolResult {
                    name: "read_release_artifacts".into(),
                    output: format!(
                        "Release {tag} artifacts for {}:\n\
                         - karoowa-{tag}-x86_64-unknown-linux-musl.tar.gz\n\
                         - karoowa-{tag}-aarch64-unknown-linux-musl.tar.gz\n\
                         - karoowa-{tag}-x86_64-apple-darwin.tar.gz\n\
                         - karoowa-{tag}-aarch64-apple-darwin.tar.gz\n\
                         - checksums-sha256.txt",
                        self.repo
                    ),
                    success: true,
                })
            }
            "deploy_to_target" => {
                let tag = call.arguments["tag"].as_str().unwrap_or("latest");
                let target = call.arguments["target"].as_str().unwrap_or("unknown");
                let approved = call.arguments["approved"].as_bool().unwrap_or(false);

                if !approved {
                    return Ok(ToolResult {
                        name: "deploy_to_target".into(),
                        output: format!(
                            "BLOCKED: Deployment of {tag} to {target} requires human approval.\n\
                             Please confirm: deploy {tag} to {target}? (set approved=true)"
                        ),
                        success: false,
                    });
                }

                Ok(ToolResult {
                    name: "deploy_to_target".into(),
                    output: format!(
                        "Deployment plan for {tag} → {target}:\n\
                         1. Download binary from GitHub Releases\n\
                         2. SSH to {target}, stop karoowa-node service\n\
                         3. Replace /opt/karoowa/bin/karoowa with new binary\n\
                         4. Start karoowa-node service\n\
                         5. Verify health endpoint\n\n\
                         (SSH deployment integration pending)"
                    ),
                    success: true,
                })
            }
            "verify_deployment" => {
                let target = call.arguments["target"].as_str().unwrap_or("unknown");
                let rpc_url = call.arguments["rpc_url"]
                    .as_str()
                    .unwrap_or("http://localhost:8545");

                match reqwest::get(format!("{rpc_url}/health")).await {
                    Ok(resp) if resp.status().is_success() => {
                        let body = resp.text().await.unwrap_or_default();
                        Ok(ToolResult {
                            name: "verify_deployment".into(),
                            output: format!("Deployment to {target} verified: {body}"),
                            success: true,
                        })
                    }
                    Ok(resp) => Ok(ToolResult {
                        name: "verify_deployment".into(),
                        output: format!(
                            "WARNING: {target} returned HTTP {}. Consider rollback.",
                            resp.status()
                        ),
                        success: false,
                    }),
                    Err(e) => Ok(ToolResult {
                        name: "verify_deployment".into(),
                        output: format!(
                            "CRITICAL: Cannot reach {target} at {rpc_url}: {e}. Rollback recommended."
                        ),
                        success: false,
                    }),
                }
            }
            "rollback" => {
                let target = call.arguments["target"].as_str().unwrap_or("unknown");
                let to_tag = call.arguments["to_tag"].as_str().unwrap_or("unknown");
                Ok(ToolResult {
                    name: "rollback".into(),
                    output: format!(
                        "Rollback plan for {target} → {to_tag}:\n\
                         1. Download {to_tag} binary\n\
                         2. Stop karoowa-node\n\
                         3. Replace binary\n\
                         4. Start karoowa-node\n\
                         5. Verify health\n\n\
                         (SSH rollback integration pending)"
                    ),
                    success: true,
                })
            }
            _ => Err(AgentError::Tool(format!("unknown tool: {}", call.name))),
        }
    }
}
