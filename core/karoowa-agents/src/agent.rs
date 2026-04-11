//! `Agent` trait — the interface for all Karoowa AI agents.
//!
//! Each agent (Onboarding, Monitoring, CLI/Dev) implements this trait.
//! The runtime drives the agent loop: prompt -> LLM -> tool calls -> repeat.

use async_trait::async_trait;

use crate::error::AgentError;
use crate::provider::LlmProvider;
use crate::types::{Message, Prompt, Role, ToolCall, ToolDefinition, ToolResult};

/// The agent interface. Each M1 agent implements this.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Human-readable agent name (e.g. "onboarding", "monitor", "dev").
    fn name(&self) -> &str;

    /// The system prompt that defines the agent's personality and role.
    fn system_prompt(&self) -> String;

    /// Tool definitions available to this agent.
    fn tools(&self) -> Vec<ToolDefinition>;

    /// Execute a tool call. Returns the tool's output.
    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult, AgentError>;
}

/// Run one agent step: send messages to the LLM, execute any tool calls,
/// return the final response.
pub async fn run_agent_step(
    agent: &dyn Agent,
    provider: &dyn LlmProvider,
    user_input: &str,
    history: &mut Vec<Message>,
) -> Result<String, AgentError> {
    // Build the prompt.
    let mut messages = vec![Message {
        role: Role::System,
        content: agent.system_prompt(),
    }];
    messages.extend(history.iter().cloned());
    messages.push(Message {
        role: Role::User,
        content: user_input.to_string(),
    });

    let tools = agent.tools();
    let prompt = Prompt {
        messages: messages.clone(),
        tools: tools.clone(),
        max_tokens: 1024,
    };

    let completion = provider.complete(&prompt).await?;

    // If the model called tools, execute them and feed results back.
    if !completion.tool_calls.is_empty() {
        // Record the assistant's tool-calling response.
        history.push(Message {
            role: Role::User,
            content: user_input.to_string(),
        });
        history.push(Message {
            role: Role::Assistant,
            content: format!(
                "[tool calls: {}]",
                completion
                    .tool_calls
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });

        let mut tool_outputs = Vec::new();
        for call in &completion.tool_calls {
            let result = agent.execute_tool(call).await?;
            tool_outputs.push(format!(
                "Tool '{}': {}{}",
                result.name,
                if result.success { "" } else { "[FAILED] " },
                result.output
            ));
        }

        // Feed tool results back to the LLM for a final response.
        let tool_summary = tool_outputs.join("\n\n");
        history.push(Message {
            role: Role::Tool,
            content: tool_summary.clone(),
        });

        let follow_up = Prompt {
            messages: {
                let mut m = vec![Message {
                    role: Role::System,
                    content: agent.system_prompt(),
                }];
                m.extend(history.iter().cloned());
                m
            },
            tools,
            max_tokens: 1024,
        };

        let final_completion = provider.complete(&follow_up).await?;
        let response = final_completion.content;
        history.push(Message {
            role: Role::Assistant,
            content: response.clone(),
        });
        Ok(response)
    } else {
        // No tool calls — the model responded directly.
        history.push(Message {
            role: Role::User,
            content: user_input.to_string(),
        });
        history.push(Message {
            role: Role::Assistant,
            content: completion.content.clone(),
        });
        Ok(completion.content)
    }
}

/// Run an agent in a non-interactive, scripted flow. Executes steps
/// sequentially, using the agent's tools, until the agent says it's done.
pub async fn run_agent_flow(
    agent: &dyn Agent,
    provider: &dyn LlmProvider,
    initial_input: &str,
    max_steps: usize,
) -> Result<Vec<String>, AgentError> {
    let mut history = Vec::new();
    let mut outputs = Vec::new();

    let response = run_agent_step(agent, provider, initial_input, &mut history).await?;
    outputs.push(response);

    // Continue if the agent has more to do (indicated by tool calls).
    for _ in 1..max_steps {
        let last = history.last().map(|m| m.content.as_str()).unwrap_or("");
        if last.contains("complete") || last.contains("done") || last.contains("finished") {
            break;
        }
        let response = run_agent_step(
            agent,
            provider,
            "Continue with the next step.",
            &mut history,
        )
        .await?;
        outputs.push(response);
    }

    Ok(outputs)
}
