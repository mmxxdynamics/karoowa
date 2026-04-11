//! Karoowa transaction type.
//!
//! A [`Transaction`] represents a signed, broadcastable state transition on a
//! Karoowa chain. Transactions are serialized with `bincode` for hashing and
//! network transfer.

use karoowa_crypto::{sha3_256, Address, Hash, Keypair, SignatureError};
use serde::{Deserialize, Serialize};

/// A signed transaction ready for inclusion in a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Sender address (derived from the signer's public key).
    pub from: Address,
    /// Recipient address. `None` for contract creation (future — M3).
    pub to: Option<Address>,
    /// Value transferred in the smallest unit.
    pub value: u64,
    /// Sender's nonce (monotonically increasing per account).
    pub nonce: u64,
    /// Gas price the sender is willing to pay per unit of gas.
    pub gas_price: u64,
    /// Maximum gas units the sender is willing to consume.
    pub gas_limit: u64,
    /// Arbitrary data payload (contract call data in M3).
    pub data: Vec<u8>,
    /// The chain ID this transaction targets (replay protection).
    pub chain_id: u64,
    /// Signature bytes (64 bytes, stored as Vec for serde compatibility).
    pub signature: Vec<u8>,
    /// Signer's public key bytes (32 bytes, stored as Vec for serde compatibility).
    pub signer_pubkey: Vec<u8>,
}

/// The fields of a transaction that are signed (everything except signature
/// and signer_pubkey).
#[derive(Serialize)]
struct SignablePayload<'a> {
    from: &'a Address,
    to: &'a Option<Address>,
    value: u64,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    data: &'a [u8],
    chain_id: u64,
}

impl Transaction {
    /// Compute the transaction hash (SHA3-256 of the bincode-serialized tx).
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(self).expect("transaction serialization cannot fail");
        sha3_256(&bytes)
    }

    /// Build and sign a transfer transaction.
    pub fn sign_transfer(
        keypair: &Keypair,
        to: Address,
        value: u64,
        nonce: u64,
        gas_price: u64,
        gas_limit: u64,
        chain_id: u64,
    ) -> Self {
        Self::sign_raw(
            keypair,
            Some(to),
            value,
            nonce,
            gas_price,
            gas_limit,
            Vec::new(),
            chain_id,
        )
    }

    /// Build and sign a transaction with arbitrary data.
    #[allow(clippy::too_many_arguments)]
    pub fn sign_raw(
        keypair: &Keypair,
        to: Option<Address>,
        value: u64,
        nonce: u64,
        gas_price: u64,
        gas_limit: u64,
        data: Vec<u8>,
        chain_id: u64,
    ) -> Self {
        let from = keypair.address();

        let payload = SignablePayload {
            from: &from,
            to: &to,
            value,
            nonce,
            gas_price,
            gas_limit,
            data: &data,
            chain_id,
        };
        let payload_bytes =
            bincode::serialize(&payload).expect("signable payload serialization cannot fail");

        let sig = keypair.sign(&payload_bytes);

        Transaction {
            from,
            to,
            value,
            nonce,
            gas_price,
            gas_limit,
            data,
            chain_id,
            signature: sig.to_bytes().to_vec(),
            signer_pubkey: sig.signer_public_key().to_vec(),
        }
    }

    /// Verify the transaction signature.
    ///
    /// Returns `Ok(())` if the signature is valid and the `from` address
    /// matches the signer's public key.
    pub fn verify_signature(&self) -> Result<(), SignatureError> {
        let payload = SignablePayload {
            from: &self.from,
            to: &self.to,
            value: self.value,
            nonce: self.nonce,
            gas_price: self.gas_price,
            gas_limit: self.gas_limit,
            data: &self.data,
            chain_id: self.chain_id,
        };
        let payload_bytes =
            bincode::serialize(&payload).expect("signable payload serialization cannot fail");

        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| SignatureError::Invalid)?;
        let pk_bytes: [u8; 32] = self
            .signer_pubkey
            .as_slice()
            .try_into()
            .map_err(|_| SignatureError::Invalid)?;

        let sig = karoowa_crypto::Signature::from_parts(&sig_bytes, &pk_bytes)?;
        sig.verify(&payload_bytes)?;

        // Verify that `from` matches the signer's public key.
        let derived_addr = Address::from_public_key(&pk_bytes);
        if derived_addr != self.from {
            return Err(SignatureError::Invalid);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keypair() -> Keypair {
        Keypair::from_seed(&[1u8; 32])
    }

    #[test]
    fn sign_and_verify_transfer() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = Transaction::sign_transfer(&kp, to, 1000, 0, 1, 21000, 1);
        assert!(tx.verify_signature().is_ok());
    }

    #[test]
    fn from_matches_signer_address() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = Transaction::sign_transfer(&kp, to, 1000, 0, 1, 21000, 1);
        assert_eq!(tx.from, kp.address());
    }

    #[test]
    fn tampered_value_fails_verification() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let mut tx = Transaction::sign_transfer(&kp, to, 1000, 0, 1, 21000, 1);
        tx.value = 9999; // tamper
        assert!(tx.verify_signature().is_err());
    }

    #[test]
    fn tampered_from_fails_verification() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let mut tx = Transaction::sign_transfer(&kp, to, 1000, 0, 1, 21000, 1);
        tx.from = Address::ZERO; // tamper
        assert!(tx.verify_signature().is_err());
    }

    #[test]
    fn hash_is_deterministic() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let tx1 = Transaction::sign_transfer(&kp, to, 1000, 0, 1, 21000, 1);
        let tx2 = Transaction::sign_transfer(&kp, to, 1000, 0, 1, 21000, 1);
        // Note: two separate sign calls will produce different signatures
        // because ed25519 signing is deterministic for the same key+message,
        // but the message includes the same payload, so hashes should differ
        // only if signatures differ. In ed25519-dalek, signing IS deterministic.
        assert_eq!(tx1.hash(), tx2.hash());
    }

    #[test]
    fn different_txs_have_different_hashes() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let tx1 = Transaction::sign_transfer(&kp, to, 1000, 0, 1, 21000, 1);
        let tx2 = Transaction::sign_transfer(&kp, to, 2000, 0, 1, 21000, 1);
        assert_ne!(tx1.hash(), tx2.hash());
    }

    #[test]
    fn serde_roundtrip() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = Transaction::sign_transfer(&kp, to, 1000, 0, 1, 21000, 1);
        let json = serde_json::to_string(&tx).unwrap();
        let deserialized: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(tx.hash(), deserialized.hash());
        assert!(deserialized.verify_signature().is_ok());
    }

    #[test]
    fn sign_raw_with_data() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let data = vec![0xde, 0xad, 0xbe, 0xef];
        let tx = Transaction::sign_raw(&kp, Some(to), 0, 0, 1, 50000, data.clone(), 1);
        assert!(tx.verify_signature().is_ok());
        assert_eq!(tx.data, data);
    }

    #[test]
    fn contract_creation_tx() {
        let kp = test_keypair();
        let data = vec![0x60, 0x80]; // placeholder bytecode
        let tx = Transaction::sign_raw(&kp, None, 0, 0, 1, 100000, data, 1);
        assert!(tx.verify_signature().is_ok());
        assert!(tx.to.is_none());
    }
}
