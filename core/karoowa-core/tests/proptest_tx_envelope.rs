//! Property-based tests for TransactionEnvelope serialization.
//!
//! Transactions arrive over RPC, mempool gossip, and block bodies — every
//! one of those paths is adversarial. The deserializer must never panic
//! on junk bytes and must be bit-exact invertible with the serializer.

use karoowa_core::{AccessList, Eip1559Transaction, Transaction, TransactionEnvelope};
use karoowa_crypto::Address;
use proptest::prelude::*;

const CASES: u32 = 64;

fn arb_address() -> impl Strategy<Value = Address> {
    any::<[u8; 32]>().prop_map(|seed| Address::from_public_key(&seed))
}

fn arb_bytes(max: usize) -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(any::<u8>(), 0..max)
}

fn arb_legacy_tx() -> impl Strategy<Value = Transaction> {
    (
        arb_address(),
        proptest::option::of(arb_address()),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        arb_bytes(64),
        any::<u64>(),
        arb_bytes(64),
        arb_bytes(32),
    )
        .prop_map(
            |(
                from,
                to,
                value,
                nonce,
                gas_price,
                gas_limit,
                data,
                chain_id,
                signature,
                signer_pubkey,
            )| Transaction {
                from,
                to,
                value,
                nonce,
                gas_price,
                gas_limit,
                data,
                chain_id,
                signature,
                signer_pubkey,
            },
        )
}

fn arb_access_list() -> impl Strategy<Value = AccessList> {
    proptest::collection::vec(
        (
            arb_address(),
            proptest::collection::vec(
                any::<[u8; 32]>().prop_map(karoowa_crypto::Hash::from_bytes),
                0..4,
            ),
        ),
        0..4,
    )
}

fn arb_eip1559_tx() -> impl Strategy<Value = Eip1559Transaction> {
    (
        arb_address(),
        proptest::option::of(arb_address()),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        arb_bytes(64),
        any::<u64>(),
        arb_access_list(),
        arb_bytes(64),
        arb_bytes(32),
    )
        .prop_map(
            |(
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
                signature,
                signer_pubkey,
            )| Eip1559Transaction {
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
                signature,
                signer_pubkey,
            },
        )
}

fn arb_envelope() -> impl Strategy<Value = TransactionEnvelope> {
    prop_oneof![
        arb_legacy_tx().prop_map(TransactionEnvelope::Legacy),
        arb_eip1559_tx().prop_map(TransactionEnvelope::Eip1559),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    /// bincode round-trip for both envelope variants. decode(encode(x))
    /// must produce a value whose wire image is byte-identical to x.
    #[test]
    fn envelope_bincode_round_trip(env in arb_envelope()) {
        let bytes = bincode::serialize(&env).unwrap();
        let decoded: TransactionEnvelope = bincode::deserialize(&bytes).unwrap();
        // Envelope doesn't derive PartialEq; compare re-encoded bytes
        // which is the load-bearing property for network agreement.
        let reencoded = bincode::serialize(&decoded).unwrap();
        prop_assert_eq!(bytes, reencoded);
    }

    /// Hash is a pure function of content: recomputing it on the same
    /// envelope must yield the same 32-byte digest.
    #[test]
    fn envelope_hash_is_deterministic(env in arb_envelope()) {
        prop_assert_eq!(env.hash(), env.hash());
    }

    /// type_byte() must match EIP-2718 encoding: 0x00 for legacy, 0x02
    /// for EIP-1559. A wrong byte here would cause downstream encoders
    /// to misroute transactions on the wire.
    #[test]
    fn envelope_type_byte_matches_variant(env in arb_envelope()) {
        match &env {
            TransactionEnvelope::Legacy(_) => prop_assert_eq!(env.type_byte(), 0x00),
            TransactionEnvelope::Eip1559(_) => prop_assert_eq!(env.type_byte(), 0x02),
        }
    }

    /// Arbitrary junk bytes must not panic the deserializer — the
    /// mempool admits tx bytes from the network before any validation,
    /// so a panic here is a crash-the-node DoS.
    #[test]
    fn junk_bytes_do_not_panic_envelope(bytes in arb_bytes(256)) {
        let _ = bincode::deserialize::<TransactionEnvelope>(&bytes);
    }

    #[test]
    fn junk_bytes_do_not_panic_legacy(bytes in arb_bytes(256)) {
        let _ = bincode::deserialize::<Transaction>(&bytes);
    }

    #[test]
    fn junk_bytes_do_not_panic_eip1559(bytes in arb_bytes(256)) {
        let _ = bincode::deserialize::<Eip1559Transaction>(&bytes);
    }
}
