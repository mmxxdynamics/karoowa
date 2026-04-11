//! `karoowa genesis` — generate and validate genesis configurations.

use clap::{Args, Subcommand};
use karoowa_core::GenesisConfig;
use karoowa_crypto::Keypair;
use std::path::PathBuf;

#[derive(Args)]
pub struct GenesisArgs {
    #[command(subcommand)]
    command: GenesisCommand,
}

#[derive(Subcommand)]
enum GenesisCommand {
    /// Generate a genesis configuration
    Generate {
        /// Number of validators (generates fresh keys)
        #[arg(long, default_value = "1")]
        validators: usize,

        /// Output file
        #[arg(short, long, default_value = "genesis.toml")]
        output: PathBuf,
    },
    /// Validate a genesis configuration file
    Validate {
        /// Path to the genesis file (TOML or JSON)
        file: PathBuf,
    },
}

pub fn run(args: GenesisArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        GenesisCommand::Generate { validators, output } => {
            let mut validator_keys = Vec::with_capacity(validators);
            for i in 0..validators {
                let kp = Keypair::generate();
                println!("Validator {i}: {}", kp.address());
                let key_path = output.with_extension(format!("validator{i}.key"));
                std::fs::write(&key_path, kp.private_key_hex())?;
                println!("  Key file: {}", key_path.display());
                validator_keys.push(kp);
            }

            let genesis =
                GenesisConfig::devnet(validator_keys.iter().map(|kp| kp.address()).collect());
            genesis.validate()?;

            let toml_str = toml::to_string_pretty(&genesis)?;
            std::fs::write(&output, &toml_str)?;
            println!("\nGenesis written to: {}", output.display());
            Ok(())
        }
        GenesisCommand::Validate { file } => {
            let content = std::fs::read_to_string(&file)?;
            let genesis: GenesisConfig = if file.extension().is_some_and(|e| e == "json") {
                serde_json::from_str(&content)?
            } else {
                toml::from_str(&content)?
            };
            genesis.validate()?;
            println!("Valid genesis configuration.");
            println!("  Chain ID:   {}", genesis.chain_config.chain_id);
            println!("  Validators: {}", genesis.validators.len());
            println!("  Accounts:   {}", genesis.alloc.len());
            Ok(())
        }
    }
}
