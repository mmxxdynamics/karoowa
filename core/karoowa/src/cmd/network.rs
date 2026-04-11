//! `karoowa network` — network peer inspection utilities.

use clap::{Args, Subcommand};
use karoowa_sdk::NodeClient;

#[derive(Args)]
pub struct NetworkArgs {
    /// RPC endpoint URL
    #[arg(long, default_value = "http://localhost:8545")]
    rpc: String,

    #[command(subcommand)]
    command: NetworkCommand,
}

#[derive(Subcommand)]
enum NetworkCommand {
    /// Show connected peers and node info
    Peers,
}

pub async fn run(args: NetworkArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = NodeClient::new(&args.rpc);

    match args.command {
        NetworkCommand::Peers => {
            let info = client.node_info().await?;
            let peer_count = client.peer_count().await?;

            println!("Node: {}", info["peer_id"].as_str().unwrap_or("unknown"));
            println!("Peers: {peer_count}");
        }
    }

    Ok(())
}
