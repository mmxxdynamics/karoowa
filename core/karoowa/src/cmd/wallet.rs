//! `karoowa wallet` — key management.

use clap::{Args, Subcommand};
use karoowa_crypto::Keypair;
use std::path::PathBuf;

#[derive(Args)]
pub struct WalletArgs {
    #[command(subcommand)]
    command: WalletCommand,
}

#[derive(Subcommand)]
enum WalletCommand {
    /// Generate a new validator keypair
    New {
        /// Output file for the private key (hex-encoded)
        #[arg(short, long, default_value = "validator.key")]
        output: PathBuf,
    },
    /// Display the address for a key file
    Address {
        /// Path to the private key file
        key_file: PathBuf,
    },
    /// Sign an arbitrary message with a key
    Sign {
        /// Path to the private key file
        key_file: PathBuf,
        /// Message to sign (hex-encoded)
        message: String,
    },
}

pub fn run(args: WalletArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        WalletCommand::New { output } => {
            let kp = Keypair::generate();
            let key_hex = kp.private_key_hex();
            crate::cmd::write_secret_file(&output, &key_hex)?;
            println!("Address:  {}", kp.address());
            println!("Key file: {}", output.display());
            Ok(())
        }
        WalletCommand::Address { key_file } => {
            let kp = load_keypair(&key_file)?;
            println!("{}", kp.address());
            Ok(())
        }
        WalletCommand::Sign { key_file, message } => {
            let kp = load_keypair(&key_file)?;
            let msg_bytes = hex::decode(message.strip_prefix("0x").unwrap_or(&message))?;
            let sig = kp.sign(&msg_bytes);
            println!("{}", hex::encode(sig.to_bytes()));
            Ok(())
        }
    }
}

fn load_keypair(path: &PathBuf) -> Result<Keypair, Box<dyn std::error::Error>> {
    let hex_str = std::fs::read_to_string(path)?.trim().to_string();
    let bytes = hex::decode(hex_str.strip_prefix("0x").unwrap_or(&hex_str))?;
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes, got {}", bytes.len()).into());
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    Ok(Keypair::from_seed(&seed))
}
