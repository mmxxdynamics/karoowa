//! `Wallet` — key management and transaction signing.

use karoowa_core::Transaction;
use karoowa_crypto::{Address, Keypair};

use crate::error::SdkError;

/// A wallet wrapping a [`Keypair`] with helpers for signing transactions.
pub struct Wallet {
    keypair: Keypair,
    chain_id: u64,
}

impl Wallet {
    /// Generate a new wallet with a random keypair.
    #[must_use]
    pub fn generate(chain_id: u64) -> Self {
        Wallet {
            keypair: Keypair::generate(),
            chain_id,
        }
    }

    /// Create a wallet from an existing keypair.
    #[must_use]
    pub fn from_keypair(keypair: Keypair, chain_id: u64) -> Self {
        Wallet { keypair, chain_id }
    }

    /// Create a wallet from a 32-byte seed (deterministic).
    #[must_use]
    pub fn from_seed(seed: &[u8; 32], chain_id: u64) -> Self {
        Wallet {
            keypair: Keypair::from_seed(seed),
            chain_id,
        }
    }

    /// The wallet's address.
    #[must_use]
    pub fn address(&self) -> Address {
        self.keypair.address()
    }

    /// The chain ID this wallet is bound to.
    #[must_use]
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Sign a value transfer transaction.
    pub fn sign_transfer(
        &self,
        to: Address,
        value: u64,
        nonce: u64,
        gas_price: u64,
        gas_limit: u64,
    ) -> Transaction {
        Transaction::sign_transfer(
            &self.keypair,
            to,
            value,
            nonce,
            gas_price,
            gas_limit,
            self.chain_id,
        )
    }

    /// Sign a transaction with arbitrary data (e.g. contract call).
    /// The `to` field is `None` for contract creation.
    pub fn sign_with_data(
        &self,
        to: Option<Address>,
        value: u64,
        nonce: u64,
        gas_price: u64,
        gas_limit: u64,
        data: Vec<u8>,
    ) -> Transaction {
        Transaction::sign_raw(
            &self.keypair,
            to,
            value,
            nonce,
            gas_price,
            gas_limit,
            data,
            self.chain_id,
        )
    }

    /// Encode a signed transaction as a hex string (with `0x` prefix)
    /// suitable for `kw_sendRawTransaction`.
    pub fn encode_transaction(tx: &Transaction) -> Result<String, SdkError> {
        let bytes =
            bincode::serialize(tx).map_err(|e| SdkError::Transaction(format!("serialize: {e}")))?;
        Ok(format!("0x{}", hex::encode(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_crypto::Hash;

    #[test]
    fn generate_produces_valid_wallet() {
        let wallet = Wallet::generate(42);
        assert_eq!(wallet.chain_id(), 42);
        // Address is 20 bytes, displayed as 0x + 40 hex chars.
        let addr_str = wallet.address().to_string();
        assert!(addr_str.starts_with("0x"));
        assert_eq!(addr_str.len(), 42);
    }

    #[test]
    fn from_seed_is_deterministic() {
        let w1 = Wallet::from_seed(&[1u8; 32], 1);
        let w2 = Wallet::from_seed(&[1u8; 32], 1);
        assert_eq!(w1.address(), w2.address());
    }

    #[test]
    fn sign_transfer_produces_valid_tx() {
        let wallet = Wallet::from_seed(&[1u8; 32], 42);
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = wallet.sign_transfer(to, 100, 0, 1, 21000);
        assert_ne!(tx.hash(), Hash::ZERO);
    }

    #[test]
    fn encode_transaction_roundtrips() {
        let wallet = Wallet::from_seed(&[1u8; 32], 42);
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = wallet.sign_transfer(to, 100, 0, 1, 21000);

        let hex_str = Wallet::encode_transaction(&tx).unwrap();
        assert!(hex_str.starts_with("0x"));

        // Decode back.
        let bytes = hex::decode(hex_str.strip_prefix("0x").unwrap()).unwrap();
        let decoded: Transaction = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded.hash(), tx.hash());
    }
}
