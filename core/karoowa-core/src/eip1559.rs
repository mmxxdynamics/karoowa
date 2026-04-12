//! EIP-1559 / EIP-2718 / EIP-2930 transaction support.
//!
//! Adds typed transaction envelopes alongside the existing legacy
//! [`Transaction`] type, plus the EIP-1559 fee market machinery
//! (priority fee, base fee, effective gas price). The legacy `Transaction`
//! type is unchanged — it becomes the `Legacy` variant of the envelope.
//!
//! See <https://eips.ethereum.org/EIPS/eip-1559>,
//! <https://eips.ethereum.org/EIPS/eip-2718>,
//! <https://eips.ethereum.org/EIPS/eip-2930>.
//!
//! # BlockHeader.base_fee
//!
//! For backwards compatibility with existing tests, the `base_fee_per_gas`
//! field has not yet been added to `BlockHeader`. The base fee is currently
//! passed in at the call site (e.g. by the consensus engine). Phase 4.3.b
//! will integrate it into the header once consensus engines opt in.

use karoowa_crypto::{sha3_256, Address, Hash, Keypair, SignatureError};
use serde::{Deserialize, Serialize};

use crate::transaction::Transaction;

/// EIP-2930 access list: a per-tx hint declaring which addresses and
/// storage slots the transaction will touch. Used by some chains for
/// gas accounting; Karoowa stores it but does not currently use it for
/// gas pricing.
pub type AccessList = Vec<(Address, Vec<Hash>)>;

/// An EIP-1559 typed transaction.
///
/// Replaces the single `gas_price` field with a fee market:
/// - `max_fee_per_gas`: the highest the sender will pay per unit of gas
/// - `max_priority_fee_per_gas`: the tip given to the proposer above the base fee
///
/// The actual gas price is `min(max_fee_per_gas, base_fee + max_priority_fee_per_gas)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Eip1559Transaction {
    pub from: Address,
    pub to: Option<Address>,
    pub value: u64,
    pub nonce: u64,
    /// Maximum total fee per gas the sender is willing to pay.
    pub max_fee_per_gas: u64,
    /// Tip per gas paid to the block proposer (above base fee).
    pub max_priority_fee_per_gas: u64,
    pub gas_limit: u64,
    pub data: Vec<u8>,
    pub chain_id: u64,
    /// EIP-2930 access list.
    pub access_list: AccessList,
    /// Signature bytes.
    pub signature: Vec<u8>,
    /// Signer's public key bytes.
    pub signer_pubkey: Vec<u8>,
}

/// Fields signed by EIP-1559 (everything except signature + pubkey).
#[derive(Serialize)]
struct Eip1559SignablePayload<'a> {
    /// Transaction type byte (EIP-2718): 0x02 for EIP-1559.
    type_byte: u8,
    chain_id: u64,
    nonce: u64,
    max_priority_fee_per_gas: u64,
    max_fee_per_gas: u64,
    gas_limit: u64,
    to: &'a Option<Address>,
    value: u64,
    data: &'a [u8],
    access_list: &'a AccessList,
    from: &'a Address,
}

impl Eip1559Transaction {
    /// EIP-2718 type byte for EIP-1559 transactions.
    pub const TYPE_BYTE: u8 = 0x02;

    /// Compute the transaction hash.
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(self).expect("eip1559 tx serialization cannot fail");
        sha3_256(&bytes)
    }

    /// Build and sign an EIP-1559 transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        keypair: &Keypair,
        to: Option<Address>,
        value: u64,
        nonce: u64,
        max_priority_fee_per_gas: u64,
        max_fee_per_gas: u64,
        gas_limit: u64,
        data: Vec<u8>,
        chain_id: u64,
        access_list: AccessList,
    ) -> Self {
        assert!(
            max_fee_per_gas >= max_priority_fee_per_gas,
            "max_fee_per_gas must be >= max_priority_fee_per_gas"
        );

        let from = keypair.address();

        let payload = Eip1559SignablePayload {
            type_byte: Self::TYPE_BYTE,
            chain_id,
            nonce,
            max_priority_fee_per_gas,
            max_fee_per_gas,
            gas_limit,
            to: &to,
            value,
            data: &data,
            access_list: &access_list,
            from: &from,
        };
        let payload_bytes =
            bincode::serialize(&payload).expect("signable payload serialization cannot fail");

        let sig = keypair.sign(&payload_bytes);

        Eip1559Transaction {
            from,
            to,
            value,
            nonce,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            gas_limit,
            data,
            chain_id,
            access_list,
            signature: sig.to_bytes().to_vec(),
            signer_pubkey: sig.signer_public_key().to_vec(),
        }
    }

    /// Verify the transaction signature.
    pub fn verify_signature(&self) -> Result<(), SignatureError> {
        let payload = Eip1559SignablePayload {
            type_byte: Self::TYPE_BYTE,
            chain_id: self.chain_id,
            nonce: self.nonce,
            max_priority_fee_per_gas: self.max_priority_fee_per_gas,
            max_fee_per_gas: self.max_fee_per_gas,
            gas_limit: self.gas_limit,
            to: &self.to,
            value: self.value,
            data: &self.data,
            access_list: &self.access_list,
            from: &self.from,
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

        let derived_addr = Address::from_public_key(&pk_bytes);
        if derived_addr != self.from {
            return Err(SignatureError::Invalid);
        }
        Ok(())
    }

    /// Calculate the effective gas price for a given base fee.
    ///
    /// `effective_gas_price = min(max_fee_per_gas, base_fee + max_priority_fee_per_gas)`
    pub fn effective_gas_price(&self, base_fee: u64) -> u64 {
        let with_tip = base_fee.saturating_add(self.max_priority_fee_per_gas);
        self.max_fee_per_gas.min(with_tip)
    }

    /// Calculate the priority fee (tip) actually paid to the proposer.
    ///
    /// This is `effective_gas_price - base_fee`, clamped to non-negative.
    pub fn priority_fee_paid(&self, base_fee: u64) -> u64 {
        self.effective_gas_price(base_fee).saturating_sub(base_fee)
    }
}

/// EIP-2718 typed transaction envelope.
///
/// Wraps both legacy and EIP-1559 transactions in a single type so they
/// can be processed uniformly. The variant tag corresponds to the
/// EIP-2718 type byte: 0x00 for Legacy, 0x02 for Eip1559.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionEnvelope {
    /// Pre-EIP-1559 transaction with a flat `gas_price`.
    Legacy(Transaction),
    /// EIP-1559 transaction with `max_fee_per_gas` + `max_priority_fee_per_gas`.
    Eip1559(Eip1559Transaction),
}

impl TransactionEnvelope {
    /// EIP-2718 type byte for this transaction.
    pub fn type_byte(&self) -> u8 {
        match self {
            TransactionEnvelope::Legacy(_) => 0x00,
            TransactionEnvelope::Eip1559(_) => Eip1559Transaction::TYPE_BYTE,
        }
    }

    /// Compute the transaction hash.
    pub fn hash(&self) -> Hash {
        match self {
            TransactionEnvelope::Legacy(tx) => tx.hash(),
            TransactionEnvelope::Eip1559(tx) => tx.hash(),
        }
    }

    /// Sender address.
    pub fn from(&self) -> Address {
        match self {
            TransactionEnvelope::Legacy(tx) => tx.from,
            TransactionEnvelope::Eip1559(tx) => tx.from,
        }
    }

    /// Recipient address.
    pub fn to(&self) -> Option<Address> {
        match self {
            TransactionEnvelope::Legacy(tx) => tx.to,
            TransactionEnvelope::Eip1559(tx) => tx.to,
        }
    }

    /// Transaction nonce.
    pub fn nonce(&self) -> u64 {
        match self {
            TransactionEnvelope::Legacy(tx) => tx.nonce,
            TransactionEnvelope::Eip1559(tx) => tx.nonce,
        }
    }

    /// Gas limit.
    pub fn gas_limit(&self) -> u64 {
        match self {
            TransactionEnvelope::Legacy(tx) => tx.gas_limit,
            TransactionEnvelope::Eip1559(tx) => tx.gas_limit,
        }
    }

    /// Effective gas price for the given base fee.
    ///
    /// - **Legacy:** the flat `gas_price` (base fee is ignored).
    /// - **EIP-1559:** `min(max_fee, base_fee + priority_fee)`.
    pub fn effective_gas_price(&self, base_fee: u64) -> u64 {
        match self {
            TransactionEnvelope::Legacy(tx) => tx.gas_price,
            TransactionEnvelope::Eip1559(tx) => tx.effective_gas_price(base_fee),
        }
    }

    /// Verify the transaction signature.
    pub fn verify_signature(&self) -> Result<(), SignatureError> {
        match self {
            TransactionEnvelope::Legacy(tx) => tx.verify_signature(),
            TransactionEnvelope::Eip1559(tx) => tx.verify_signature(),
        }
    }
}

/// Compute the next block's base fee per the EIP-1559 algorithm.
///
/// The base fee adjusts based on whether the parent block was below or
/// above its target gas usage:
/// - **At target:** base fee unchanged.
/// - **Above target:** base fee increases proportionally (max +12.5%).
/// - **Below target:** base fee decreases proportionally (max -12.5%).
///
/// The denominator (`8` per EIP-1559) caps single-block changes at 12.5%.
pub fn compute_base_fee(parent_base_fee: u64, gas_used: u64, gas_target: u64) -> u64 {
    if gas_target == 0 {
        return parent_base_fee;
    }

    if gas_used == gas_target {
        return parent_base_fee;
    }

    if gas_used > gas_target {
        // Increase: delta = parent_base_fee * (gas_used - target) / target / 8, min 1.
        let delta = parent_base_fee
            .saturating_mul(gas_used - gas_target)
            .saturating_div(gas_target)
            .saturating_div(8)
            .max(1);
        parent_base_fee.saturating_add(delta)
    } else {
        // Decrease: delta = parent_base_fee * (target - gas_used) / target / 8.
        let delta = parent_base_fee
            .saturating_mul(gas_target - gas_used)
            .saturating_div(gas_target)
            .saturating_div(8);
        parent_base_fee.saturating_sub(delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keypair() -> Keypair {
        Keypair::from_seed(&[7u8; 32])
    }

    #[test]
    fn type_bytes() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);

        let legacy =
            TransactionEnvelope::Legacy(Transaction::sign_transfer(&kp, to, 100, 0, 5, 21000, 1));
        assert_eq!(legacy.type_byte(), 0x00);

        let eip1559 = TransactionEnvelope::Eip1559(Eip1559Transaction::sign(
            &kp,
            Some(to),
            100,
            0,
            2,
            10,
            21000,
            vec![],
            1,
            vec![],
        ));
        assert_eq!(eip1559.type_byte(), 0x02);
    }

    #[test]
    fn eip1559_sign_and_verify() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = Eip1559Transaction::sign(
            &kp,
            Some(to),
            1000,
            0,
            2,  // priority fee
            10, // max fee
            21000,
            vec![],
            1,
            vec![],
        );
        assert!(tx.verify_signature().is_ok());
    }

    #[test]
    fn eip1559_tampered_value_fails_verification() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let mut tx =
            Eip1559Transaction::sign(&kp, Some(to), 1000, 0, 2, 10, 21000, vec![], 1, vec![]);
        tx.value = 9999;
        assert!(tx.verify_signature().is_err());
    }

    #[test]
    fn eip1559_with_access_list() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let access_list = vec![(
            Address::from_public_key(&[3u8; 32]),
            vec![sha3_256(b"slot-0"), sha3_256(b"slot-1")],
        )];
        let tx = Eip1559Transaction::sign(
            &kp,
            Some(to),
            0,
            0,
            1,
            5,
            50000,
            vec![0xab, 0xcd],
            42,
            access_list.clone(),
        );
        assert!(tx.verify_signature().is_ok());
        assert_eq!(tx.access_list, access_list);
    }

    #[test]
    fn effective_gas_price_below_max_fee() {
        // base_fee + priority_fee < max_fee → use base_fee + priority_fee.
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = Eip1559Transaction::sign(
            &kp,
            Some(to),
            0,
            0,
            2,  // priority
            20, // max
            21000,
            vec![],
            1,
            vec![],
        );
        let base_fee = 5;
        // base_fee(5) + priority(2) = 7, which is < max(20), so effective = 7.
        assert_eq!(tx.effective_gas_price(base_fee), 7);
        assert_eq!(tx.priority_fee_paid(base_fee), 2);
    }

    #[test]
    fn effective_gas_price_capped_at_max_fee() {
        // base_fee + priority_fee > max_fee → cap at max_fee.
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let tx = Eip1559Transaction::sign(
            &kp,
            Some(to),
            0,
            0,
            5,  // priority
            10, // max
            21000,
            vec![],
            1,
            vec![],
        );
        let base_fee = 100; // way above max_fee
                            // min(10, 100+5) = 10.
        assert_eq!(tx.effective_gas_price(base_fee), 10);
        // priority paid = 10 - 100, saturating = 0.
        assert_eq!(tx.priority_fee_paid(base_fee), 0);
    }

    #[test]
    fn effective_gas_price_legacy_unchanged() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let envelope =
            TransactionEnvelope::Legacy(Transaction::sign_transfer(&kp, to, 100, 0, 7, 21000, 1));
        // Legacy ignores base_fee.
        assert_eq!(envelope.effective_gas_price(0), 7);
        assert_eq!(envelope.effective_gas_price(1000), 7);
    }

    #[test]
    fn envelope_accessors() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);
        let envelope = TransactionEnvelope::Eip1559(Eip1559Transaction::sign(
            &kp,
            Some(to),
            500,
            42,
            1,
            10,
            21000,
            vec![],
            1,
            vec![],
        ));

        assert_eq!(envelope.from(), kp.address());
        assert_eq!(envelope.to(), Some(to));
        assert_eq!(envelope.nonce(), 42);
        assert_eq!(envelope.gas_limit(), 21000);
        assert!(envelope.verify_signature().is_ok());
    }

    #[test]
    fn base_fee_unchanged_at_target() {
        let new = compute_base_fee(1000, 100, 100);
        assert_eq!(new, 1000);
    }

    #[test]
    fn base_fee_increases_above_target() {
        // 50% over target → ~6.25% increase.
        let new = compute_base_fee(1000, 150, 100);
        assert!(new > 1000);
        // Cap at +12.5% (single-block max).
        assert!(new <= 1125);
    }

    #[test]
    fn base_fee_decreases_below_target() {
        let new = compute_base_fee(1000, 50, 100);
        assert!(new < 1000);
        // Cap at -12.5%.
        assert!(new >= 875);
    }

    #[test]
    fn base_fee_max_increase_when_full() {
        // gas_used = 2 * gas_target → max +12.5%.
        let new = compute_base_fee(1000, 200, 100);
        // delta = 1000 * 100 / 100 / 8 = 125
        assert_eq!(new, 1125);
    }

    #[test]
    fn base_fee_max_decrease_when_empty() {
        // gas_used = 0 → max -12.5%.
        let new = compute_base_fee(1000, 0, 100);
        // delta = 1000 * 100 / 100 / 8 = 125
        assert_eq!(new, 875);
    }

    #[test]
    fn base_fee_safe_with_zero_target() {
        // Defensive: zero target shouldn't crash.
        let new = compute_base_fee(1000, 100, 0);
        assert_eq!(new, 1000);
    }

    #[test]
    fn base_fee_minimum_increase_one() {
        // Tiny overshoot still increases by at least 1.
        let new = compute_base_fee(100, 101, 100);
        assert!(new > 100);
    }

    #[test]
    fn envelope_serde_roundtrip() {
        let kp = test_keypair();
        let to = Address::from_public_key(&[2u8; 32]);

        let legacy =
            TransactionEnvelope::Legacy(Transaction::sign_transfer(&kp, to, 100, 0, 5, 21000, 1));
        let bytes = bincode::serialize(&legacy).unwrap();
        let decoded: TransactionEnvelope = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded.type_byte(), 0x00);
        assert_eq!(decoded.hash(), legacy.hash());

        let eip1559 = TransactionEnvelope::Eip1559(Eip1559Transaction::sign(
            &kp,
            Some(to),
            100,
            0,
            2,
            10,
            21000,
            vec![],
            1,
            vec![],
        ));
        let bytes = bincode::serialize(&eip1559).unwrap();
        let decoded: TransactionEnvelope = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded.type_byte(), 0x02);
        assert_eq!(decoded.hash(), eip1559.hash());
    }
}
