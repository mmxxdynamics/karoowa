//! `karoowa agent` — run AI agents.

use clap::{Args, Subcommand};
use karoowa_agents::{
    create_provider, run_agent_step, Agent, CiCdAgent, CliDevAgent, MonitoringAgent,
    ObservabilityAgent, OnboardingAgent, ProviderConfig, RuntimeMode,
};
use std::io::{self, BufRead, Write};
use tracing::info;

#[derive(Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    command: AgentCommand,

    /// LLM provider (anthropic, ollama)
    #[arg(long, default_value = "anthropic")]
    provider: String,

    /// Model name
    #[arg(long)]
    model: Option<String>,

    /// Ollama base URL (for ollama provider)
    #[arg(long)]
    ollama_url: Option<String>,

    /// Runtime mode: in-process (default), sidecar, cloud-hosted
    #[arg(long, default_value = "in-process")]
    mode: String,
}

#[derive(Subcommand)]
enum AgentCommand {
    /// Run the Onboarding Agent — guides first-time setup
    Onboard,
    /// Run the Monitoring Agent — basic node health (M1)
    Monitor {
        /// RPC endpoint of the node to monitor
        #[arg(long, default_value = "http://localhost:8545")]
        rpc: String,
    },
    /// Run the CLI/Dev Agent — translates natural language to CLI commands
    Dev,
    /// Run the CI/CD Agent — manages releases and deployments (M2)
    Cicd {
        /// GitHub repository (owner/repo)
        #[arg(long, default_value = "mmxxdynamics/karoowa")]
        repo: String,
    },
    /// Run the Observability Agent — production monitoring + remediation (M2)
    Observe {
        /// RPC endpoint of the node to monitor
        #[arg(long, default_value = "http://localhost:8545")]
        rpc: String,
    },
}

pub async fn run(args: AgentArgs) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_mode: RuntimeMode = args
        .mode
        .parse()
        .map_err(|e: String| -> Box<dyn std::error::Error> { e.into() })?;

    if runtime_mode == RuntimeMode::Sidecar {
        println!("Sidecar mode: agent will communicate via loopback proxy.");
        println!("Proxy address: 127.0.0.1:9100 (configure with --proxy-addr)");
        println!("(Full sidecar proxy implementation pending — running in-process for now)");
        println!();
    }

    if runtime_mode == RuntimeMode::CloudHosted {
        return Err("Cloud-hosted mode is an enterprise feature (M4+).".into());
    }

    let default_model = match args.provider.as_str() {
        "anthropic" => "claude-sonnet-4-20250514",
        "ollama" => "gemma4:e2b",
        _ => "claude-sonnet-4-20250514",
    };

    let config = ProviderConfig {
        provider: args.provider.clone(),
        model: args.model.unwrap_or_else(|| default_model.to_string()),
        api_key: None,
        base_url: args.ollama_url,
        max_tokens: 1024,
    };

    let provider = create_provider(&config)?;

    let agent: Box<dyn Agent> = match &args.command {
        AgentCommand::Onboard => Box::new(OnboardingAgent::new()),
        AgentCommand::Monitor { rpc } => Box::new(MonitoringAgent::new(rpc)),
        AgentCommand::Dev => Box::new(CliDevAgent::new()),
        AgentCommand::Cicd { repo } => Box::new(CiCdAgent::new(repo)),
        AgentCommand::Observe { rpc } => Box::new(ObservabilityAgent::new(rpc)),
    };

    info!(
        agent = agent.name(),
        provider = provider.name(),
        model = config.model,
        mode = %runtime_mode,
        "starting agent"
    );

    println!("Karoowa {} Agent", agent.name());
    println!("Provider: {} ({})", provider.name(), config.model);
    println!("Mode: {runtime_mode}");
    println!("Type your request (Ctrl+D to exit):\n");

    let mut history = Vec::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let input = line?;
        if input.trim().is_empty() {
            continue;
        }

        match run_agent_step(agent.as_ref(), provider.as_ref(), &input, &mut history).await {
            Ok(response) => {
                println!("\n{response}\n");
            }
            Err(e) => {
                eprintln!("\nAgent error: {e}\n");
            }
        }

        print!("> ");
        stdout.flush()?;
    }

    Ok(())
}
