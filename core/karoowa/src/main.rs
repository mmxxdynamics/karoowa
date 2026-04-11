//! Karoowa CLI — the single binary for all Karoowa operations.

use clap::{Parser, Subcommand};

mod cmd;

#[derive(Parser)]
#[command(
    name = "karoowa",
    version,
    about = "Karoowa — agent-native blockchain framework",
    long_about = "Light enough to launch anything.\n\n\
                  Karoowa is a Rust-based blockchain framework with pluggable consensus,\n\
                  built-in AI agents, and a batteries-included developer experience."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a Karoowa node
    Node(cmd::node::NodeArgs),

    /// Key management — generate, inspect, and sign
    Wallet(cmd::wallet::WalletArgs),

    /// Generate and validate genesis configurations
    Genesis(cmd::genesis::GenesisArgs),

    /// Quick one-shot RPC queries
    Client(cmd::client::ClientArgs),

    /// Local devnet management
    Devnet(cmd::devnet::DevnetArgs),

    /// Network peer inspection
    Network(cmd::network::NetworkArgs),

    /// Run an AI agent (onboard, monitor, dev)
    Agent(cmd::agent::AgentArgs),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,karoowa=debug".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Node(args) => cmd::node::run(args).await,
        Commands::Wallet(args) => cmd::wallet::run(args),
        Commands::Genesis(args) => cmd::genesis::run(args),
        Commands::Client(args) => cmd::client::run(args).await,
        Commands::Devnet(args) => cmd::devnet::run(args),
        Commands::Network(args) => cmd::network::run(args).await,
        Commands::Agent(args) => cmd::agent::run(args).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
