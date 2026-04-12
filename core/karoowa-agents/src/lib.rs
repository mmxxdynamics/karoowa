//! Karoowa AI agent runtime.
//!
//! Provides the [`LlmProvider`] trait (pluggable LLM backends), the [`Agent`]
//! trait (pluggable agent implementations), a [`MemoryStore`] for agent RAG,
//! and the three M1 agents:
//!
//! - **Onboarding Agent** — guides hobbyists through first-time setup
//! - **Monitoring Agent** — summarizes node health from metrics
//! - **CLI/Dev Agent** — translates natural language to CLI commands
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌──────────────┐     ┌─────────────┐
//! │   Agent      │────▶│  LlmProvider  │────▶│  Anthropic   │
//! │ (Onboarding) │     │  (trait)      │     │  or Ollama   │
//! └──────┬───────┘     └──────────────┘     └─────────────┘
//!        │
//!        ▼
//! ┌──────────────┐
//! │ MemoryStore   │
//! │ (in-memory /  │
//! │  LanceDB)     │
//! └──────────────┘
//! ```

pub mod agent;
pub mod agents;
pub mod error;
pub mod memory;
pub mod provider;
pub mod providers;
pub mod sidecar;
pub mod types;

pub use agent::{run_agent_flow, run_agent_step, Agent};
pub use agents::cicd::CiCdAgent;
pub use agents::cli_dev::CliDevAgent;
pub use agents::governance::GovernanceAgent;
pub use agents::monitoring::MonitoringAgent;
pub use agents::observability::ObservabilityAgent;
pub use agents::onboarding::OnboardingAgent;
pub use agents::optimizer::OptimizerAgent;
pub use agents::security::SecurityAgent;
pub use agents::treasury::TreasuryAgent;
pub use error::AgentError;
pub use memory::{InMemoryStore, MemoryEntry, MemoryStore};
pub use provider::{create_provider, LlmProvider, ProviderConfig};
pub use sidecar::{RuntimeMode, SidecarConfig};
pub use types::*;
