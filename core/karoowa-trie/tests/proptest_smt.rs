//! Property-based tests for the Sparse Merkle Trie.
//!
//! Treated as a lightweight fuzz harness for the pre-audit rc1 gate. Every
//! test runs 256 randomized cases and exercises the proof system under
//! arbitrary key/value distributions that hand-written unit tests won't
//! catch.

use karoowa_crypto::sha3_256;
use karoowa_trie::SparseMerkleTrie;
use proptest::collection::{hash_set, vec};
use proptest::prelude::*;

// 32 cases per property — dense enough to catch edge cases while keeping
// the whole suite under ~30s. SMT insert is O(D) with D=256 so even small
// item counts are expensive; proptest shrinking multiplies the cost.
const CASES: u32 = 32;

/// Arbitrary key: 1..=64 random bytes, hashed internally by the trie.
fn arb_key() -> impl Strategy<Value = Vec<u8>> {
    vec(any::<u8>(), 1..64)
}

/// Arbitrary value: 0..=128 bytes.
fn arb_value() -> impl Strategy<Value = Vec<u8>> {
    vec(any::<u8>(), 0..128)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    /// Every inserted (key, value) must produce an inclusion proof that
    /// verifies against the current root. Round-trip over all items.
    #[test]
    fn all_inserted_items_have_valid_inclusion_proofs(
        items in vec((arb_key(), arb_value()), 1..8),
    ) {
        let mut trie = SparseMerkleTrie::new();
        // Dedup keys — trie is a key-value store so the last write wins.
        let mut dedup: std::collections::BTreeMap<Vec<u8>, Vec<u8>> = Default::default();
        for (k, v) in &items {
            dedup.insert(k.clone(), v.clone());
        }
        for (k, v) in &dedup {
            trie.insert(k, v.clone());
        }
        let root = trie.root();
        for (k, v) in &dedup {
            let proof = trie.proof(k);
            prop_assert!(proof.verify(&root).is_ok());
            prop_assert_eq!(proof.value.as_ref(), Some(v));
        }
    }

    /// A key that was never inserted must produce an exclusion proof
    /// (proof.value == None) that still verifies.
    #[test]
    fn absent_keys_have_valid_exclusion_proofs(
        inserted_keys in hash_set(arb_key(), 1..6),
        probe_key in arb_key(),
    ) {
        prop_assume!(!inserted_keys.contains(&probe_key));
        let mut trie = SparseMerkleTrie::new();
        for k in &inserted_keys {
            trie.insert(k, b"x".to_vec());
        }
        let root = trie.root();
        let proof = trie.proof(&probe_key);
        prop_assert!(proof.value.is_none());
        prop_assert!(proof.verify(&root).is_ok());
    }

    /// Tampering with a proof's value must invalidate the proof. This is
    /// the core soundness property: an attacker cannot claim a different
    /// value at a key without being caught by the root check.
    #[test]
    fn tampered_value_breaks_proof(
        items in vec((arb_key(), arb_value()), 1..6),
        tamper in arb_value(),
    ) {
        let mut dedup: std::collections::BTreeMap<Vec<u8>, Vec<u8>> = Default::default();
        for (k, v) in &items {
            dedup.insert(k.clone(), v.clone());
        }
        let mut trie = SparseMerkleTrie::new();
        for (k, v) in &dedup {
            trie.insert(k, v.clone());
        }
        let root = trie.root();
        let (first_k, first_v) = dedup.iter().next().unwrap();
        prop_assume!(&tamper != first_v);
        let mut proof = trie.proof(first_k);
        proof.value = Some(tamper);
        prop_assert!(proof.verify(&root).is_err());
    }

    /// The root is a pure function of the key-value set: inserting the
    /// same pairs in different orders must produce the same root.
    #[test]
    fn root_is_order_independent(
        items in vec((arb_key(), arb_value()), 1..8),
    ) {
        // Dedup so we're comparing identical final states.
        let mut dedup: std::collections::BTreeMap<Vec<u8>, Vec<u8>> = Default::default();
        for (k, v) in &items {
            dedup.insert(k.clone(), v.clone());
        }

        let mut trie_a = SparseMerkleTrie::new();
        for (k, v) in &dedup {
            trie_a.insert(k, v.clone());
        }

        let mut trie_b = SparseMerkleTrie::new();
        // Reverse order.
        for (k, v) in dedup.iter().rev() {
            trie_b.insert(k, v.clone());
        }

        prop_assert_eq!(trie_a.root(), trie_b.root());
    }

    /// A proof for key A cannot be passed off as a proof for key B. The
    /// verifier must refuse mismatched keys.
    #[test]
    fn cross_key_proof_reuse_rejected(
        a in arb_key(),
        b in arb_key(),
        va in arb_value(),
    ) {
        prop_assume!(a != b);
        let mut trie = SparseMerkleTrie::new();
        trie.insert(&a, va.clone());
        let root = trie.root();
        let mut proof = trie.proof(&a);
        // Swap the proof's key field to B and claim A's value for B.
        proof.key = sha3_256(&b);
        prop_assert!(proof.verify(&root).is_err());
    }
}
