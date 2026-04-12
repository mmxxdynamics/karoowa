//! Property-based tests for bridge packet parsing + hashing.
//!
//! Packets arrive over the wire from relayers — adversarial input by
//! definition. These properties assert that deserialization is injective
//! with serialization (round-trip), that hashing is a pure function of
//! content, and that junk bytes are rejected rather than panicking the
//! destination node.

use karoowa_bridge::BridgePacket;
use karoowa_crypto::Address;
use proptest::prelude::*;

const CASES: u32 = 64;

fn arb_address() -> impl Strategy<Value = Address> {
    any::<[u8; 32]>().prop_map(|seed| Address::from_public_key(&seed))
}

fn arb_packet() -> impl Strategy<Value = BridgePacket> {
    (
        "[a-z0-9-]{1,32}",
        "[a-z0-9-]{1,32}",
        any::<u64>(),
        arb_address(),
        arb_address(),
        any::<u64>(),
        "[a-z0-9/]{1,16}",
        any::<u64>(),
    )
        .prop_map(
            |(source_chain, dest_chain, sequence, sender, recipient, amount, denom, timeout_height)| {
                BridgePacket {
                    source_chain,
                    dest_chain,
                    sequence,
                    sender,
                    recipient,
                    amount,
                    denom,
                    timeout_height,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    /// bincode round-trip: encode(decode(x)) == x for every valid packet.
    #[test]
    fn bincode_round_trip(packet in arb_packet()) {
        let bytes = bincode::serialize(&packet).unwrap();
        let decoded: BridgePacket = bincode::deserialize(&bytes).unwrap();
        prop_assert_eq!(decoded, packet);
    }

    /// Hash is a pure function of content: two identical packets must
    /// produce identical hashes; two differing packets must not collide
    /// on the fields we vary (sequence is the primary demux axis).
    #[test]
    fn hash_is_content_addressed(packet in arb_packet()) {
        let h1 = packet.hash();
        let h2 = packet.hash();
        prop_assert_eq!(h1, h2);

        let mut mutated = packet.clone();
        mutated.sequence = mutated.sequence.wrapping_add(1);
        prop_assert_ne!(packet.hash(), mutated.hash());
    }

    /// Commitment key and receipt key are distinct for every packet —
    /// if they ever collided, a relayer could forge a receipt by
    /// submitting a commitment proof, or vice versa.
    #[test]
    fn commitment_and_receipt_keys_are_distinct(packet in arb_packet()) {
        prop_assert_ne!(packet.commitment_key(), packet.receipt_key());
    }

    /// Arbitrary junk bytes must not panic the deserializer — they should
    /// return a decode error cleanly. A panic here would be a DoS vector
    /// on the bridge protocol handler.
    #[test]
    fn junk_bytes_do_not_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let _ = bincode::deserialize::<BridgePacket>(&bytes);
    }
}
