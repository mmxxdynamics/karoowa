//! `karoowa devnet` — local devnet management utilities.

use clap::{Args, Subcommand};

#[derive(Args)]
pub struct DevnetArgs {
    #[command(subcommand)]
    command: DevnetCommand,
}

#[derive(Subcommand)]
enum DevnetCommand {
    /// Print a sample docker-compose invocation for a local devnet
    Info,
}

pub fn run(args: DevnetArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        DevnetCommand::Info => {
            println!("Karoowa Local Devnet");
            println!("====================");
            println!();
            println!("Quick start:");
            println!("  1. Generate genesis:  karoowa genesis generate --validators 4");
            println!("  2. Start single node: karoowa node --validator-key validator0.key --consensus poa");
            println!("  3. Docker devnet:     docker compose -f docker/devnet.yml up -d");
            println!();
            println!("Docker devnet setup ships in Phase 1.9.");
            println!("See specs/development/dev_plan.md for current status.");
            Ok(())
        }
    }
}
