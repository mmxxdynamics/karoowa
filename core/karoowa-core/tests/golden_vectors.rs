//! Golden vectors for consensus-critical encodings.
//!
//! # Why this file exists
//!
//! Block hashes, transaction hashes and Merkle roots are derived from exact
//! byte encodings. Every parent link in the chain follows from them. A change
//! to any of these values is a hard fork: chains persisted before it stop
//! validating after it.
//!
//! The rest of the suite cannot catch such a change. Those tests compute a
//! root or a hash and then verify against the value they just computed, so
//! they stay green no matter what the encoding does. That is not hypothetical:
//! adding RFC 6962 leaf/node domain separation to `MerkleTree` changed every
//! `tx_root` in the codebase and **not one existing test failed**.
//!
//! These vectors pin known inputs to known outputs. Their job is to fail, and
//! a failure here is not automatically a bug:
//!
//! - If you did not intend to change an encoding, you have introduced a
//!   consensus break. Revert it.
//! - If you did intend to (a deliberate hard fork, a serializer migration per
//!   issue #40), update the constants **in the same commit**, and say so in
//!   the message. The point is that the break is loud and deliberate rather
//!   than silent.
//!
//! Keep every input here fully deterministic: fixed key seeds, fixed
//! timestamps, no clock, no randomness. Ed25519 signing is deterministic
//! (RFC 8032), so a signature over fixed bytes from a fixed seed is itself a
//! fixed value and can be pinned.

use karoowa_core::{BlockBuilder, Transaction};
use karoowa_crypto::{Address, Hash, Keypair, MerkleTree};

/// Deterministic keypair. Never use a seed like this outside tests.
fn kp(seed: u8) -> Keypair {
    Keypair::from_seed(&[seed; 32])
}

fn addr(seed: u8) -> Address {
    Address::from_public_key(&[seed; 32])
}

/// A fixed, fully deterministic transaction.
fn golden_tx(nonce: u64) -> Transaction {
    Transaction::sign_transfer(&kp(1), addr(2), 1_000, nonce, 7, 21_000, 1)
}

// ---------------------------------------------------------------------------
// Address derivation
// ---------------------------------------------------------------------------

#[test]
fn golden_address_from_public_key() {
    assert_eq!(
        addr(2).to_string(),
        "0x2020f5e43894a3298c77203a853beb9460211cd5",
        "Address derivation changed: every account identity in the chain moves"
    );
}

// ---------------------------------------------------------------------------
// Transaction hashing and signing
// ---------------------------------------------------------------------------

#[test]
fn golden_transaction_hash() {
    assert_eq!(
        golden_tx(0).hash().to_string(),
        "0x73ba9216aafcdca3264a9f563e7bc099b6ef46d2c19514cf1da974a41dc328ab",
        "Transaction encoding changed: every tx hash and tx_root moves"
    );
}

#[test]
fn golden_transaction_signature_is_deterministic() {
    // Ed25519 is deterministic, so the signature bytes are themselves a
    // golden value. A change here means the signing domain or the signable
    // payload layout moved, which silently invalidates every existing
    // signature.
    let tx = golden_tx(0);
    assert_eq!(
        hex_of(&tx.signature),
        "7366f4123aefb10188d7857296d249467a7d705fab99710227ed6ead43152644e83d0cef4e0e1e0034e14eaa4e0c2d42133c3c1caed5333f3c4554fe3c995b00",
        "Signing domain or signable payload changed: existing signatures break"
    );
    assert!(tx.verify_signature().is_ok(), "golden tx must verify");
}

// ---------------------------------------------------------------------------
// Merkle tree
// ---------------------------------------------------------------------------

#[test]
fn golden_merkle_roots() {
    // Pins RFC 6962 leaf/node domain separation. Without the tags these
    // values differ, and the single-leaf root would equal its own leaf.
    let leaves: Vec<Hash> = (0..4u8).map(|i| Hash::from_bytes([i + 1; 32])).collect();

    assert_eq!(
        MerkleTree::from_leaves(&leaves[..1]).root().to_string(),
        "0x29f8f87d926a90ecc02e336bbadc2e512c7b155497a6ca8b86a574593d2ea58d",
        "single-leaf Merkle root changed"
    );
    assert_eq!(
        MerkleTree::from_leaves(&leaves[..2]).root().to_string(),
        "0xbb7eaf44188f6bd1394e085b8e3d03ffafd19f84554f1747e7ebbe4098cca851",
        "two-leaf Merkle root changed"
    );
    assert_eq!(
        MerkleTree::from_leaves(&leaves[..3]).root().to_string(),
        "0x8686c1d9d97cdc561d70aedd42a94fa807c3810e8fb6bf7bbf388bf5fdc8c763",
        "odd-leaf (duplicate-last) Merkle root changed"
    );
    assert_eq!(
        MerkleTree::from_leaves(&leaves).root().to_string(),
        "0xc584e8fb5a4a00d899006d9242f38e97d650d079c4cd49a5ef7ab9ebff3d7bac",
        "four-leaf Merkle root changed"
    );

    // The single-leaf root must never equal its raw leaf, or an empty proof
    // validates anything. See the second-preimage tests in merkle.rs.
    assert_ne!(
        MerkleTree::from_leaves(&leaves[..1]).root(),
        leaves[0],
        "single-leaf root equals its raw leaf: domain separation lost"
    );
}

// ---------------------------------------------------------------------------
// Block header hashing
// ---------------------------------------------------------------------------

#[test]
fn golden_unsigned_block_hash() {
    let block = BlockBuilder::new(Hash::ZERO, 1, 1_700_000_000, addr(9))
        .transactions(vec![golden_tx(0), golden_tx(1)])
        .build();

    assert_eq!(
        block.header.tx_root.to_string(),
        "0x4e8b6065e078bbcbb2f824068bf2ac0cf6a7aad7c669801fe48ca2fd937aec94",
        "tx_root changed: block bodies no longer commit the same way"
    );
    assert_eq!(
        block.hash().to_string(),
        "0x40939b5812acf5289cba614632e0f3103acab69e2254a424f4b6f4ea1c012b39",
        "Block header encoding changed: every parent link in the chain moves"
    );
}

#[test]
fn golden_signed_block_hash() {
    // Signing writes the attestation into consensus_data, which is part of
    // the header hash. Pinning the signed hash therefore pins the
    // attestation encoding too, which is what the canonical-encoding check
    // in verify_proposer_signature depends on.
    let signer = kp(7);
    let mut block = BlockBuilder::new(Hash::ZERO, 1, 1_700_000_000, signer.address())
        .transactions(vec![golden_tx(0)])
        .build();
    block.header.sign_as_proposer(&signer);

    assert_eq!(
        hex_of(&block.header.consensus_data),
        "2000000000000000ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c400000000000000045a67a07c4a04492159544003d51b7594b54aa79a1cf1bc8b3ed858a6c1b4e7638be3dc224a00dc0d9c7609093a288d0a0913e56b1407a86756fa559a47c0507",
        "ProposerAttestation encoding changed"
    );
    assert_eq!(
        block.hash().to_string(),
        "0x8af1d60dc59a26c1d6090d547ce30f0528d7ce25dcf15a08f2fac9f3c36bb29a",
        "Signed block header hash changed"
    );
    assert!(
        block.header.verify_proposer_signature().is_ok(),
        "golden signed header must verify"
    );
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
