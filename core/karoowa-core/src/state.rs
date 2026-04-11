//! Account state and state diff types.
//!
//! An [`Account`] represents the on-chain state of a single address: balance,
//! nonce, code hash (for contracts), and storage root. [`StateDiff`] captures
//! the set of changes produced by executing one block's transactions.

use karoowa_crypto::{Address, Hash};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// On-chain state of a single account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    /// Account nonce (incremented with each outgoing transaction).
    pub nonce: u64,
    /// Account balance in the chain's smallest unit.
    pub balance: u64,
    /// Hash of the contract bytecode. `Hash::ZERO` for externally-owned accounts.
    pub code_hash: Hash,
    /// Root of the account's storage trie. `Hash::ZERO` for accounts with no storage.
    pub storage_root: Hash,
}

impl Account {
    /// A default externally-owned account with zero balance.
    pub fn new_eoa() -> Self {
        Account {
            nonce: 0,
            balance: 0,
            code_hash: Hash::ZERO,
            storage_root: Hash::ZERO,
        }
    }

    /// Whether this account has contract code deployed.
    pub fn is_contract(&self) -> bool {
        self.code_hash != Hash::ZERO
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::new_eoa()
    }
}

/// A change to a single account produced by block execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountChange {
    /// The account was created or modified. Contains the new state.
    Modified(Account),
    /// The account was deleted (e.g. self-destruct, future use).
    Deleted,
}

/// The set of state changes produced by executing one block's transactions.
///
/// Keyed by address, ordered deterministically via `BTreeMap` so that the
/// resulting state root is reproducible.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateDiff {
    /// Account-level changes.
    pub accounts: BTreeMap<Address, AccountChange>,
    /// Storage slot changes per contract address.
    /// Outer key = contract address, inner key = storage slot hash.
    pub storage: BTreeMap<Address, BTreeMap<Hash, Vec<u8>>>,
}

impl StateDiff {
    /// Create an empty diff.
    pub fn new() -> Self {
        StateDiff::default()
    }

    /// Record a balance transfer: debit `from`, credit `to`.
    ///
    /// If the accounts don't exist in the diff yet, they are initialized
    /// from `get_account` (or defaults if not found). This is a convenience
    /// helper for the block executor.
    pub fn apply_transfer(
        &mut self,
        from: Address,
        to: Address,
        amount: u64,
        from_account: &Account,
        to_account: &Account,
    ) {
        // Debit sender
        let mut sender = from_account.clone();
        sender.balance = sender.balance.saturating_sub(amount);
        sender.nonce += 1;
        self.accounts.insert(from, AccountChange::Modified(sender));

        // Credit receiver
        let mut receiver = to_account.clone();
        receiver.balance = receiver.balance.saturating_add(amount);
        self.accounts.insert(to, AccountChange::Modified(receiver));
    }

    /// Number of accounts affected.
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Whether this diff is empty (no changes).
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty() && self.storage.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_eoa_defaults() {
        let acct = Account::new_eoa();
        assert_eq!(acct.nonce, 0);
        assert_eq!(acct.balance, 0);
        assert!(!acct.is_contract());
    }

    #[test]
    fn contract_account() {
        let acct = Account {
            code_hash: karoowa_crypto::sha3_256(b"code"),
            ..Account::new_eoa()
        };
        assert!(acct.is_contract());
    }

    #[test]
    fn apply_transfer() {
        let from = Address::from_public_key(&[1u8; 32]);
        let to = Address::from_public_key(&[2u8; 32]);
        let mut from_acct = Account::new_eoa();
        from_acct.balance = 1000;
        let to_acct = Account::new_eoa();

        let mut diff = StateDiff::new();
        diff.apply_transfer(from, to, 300, &from_acct, &to_acct);

        assert_eq!(diff.account_count(), 2);

        if let AccountChange::Modified(sender) = &diff.accounts[&from] {
            assert_eq!(sender.balance, 700);
            assert_eq!(sender.nonce, 1);
        } else {
            panic!("expected Modified");
        }

        if let AccountChange::Modified(receiver) = &diff.accounts[&to] {
            assert_eq!(receiver.balance, 300);
        } else {
            panic!("expected Modified");
        }
    }

    #[test]
    fn empty_diff() {
        let diff = StateDiff::new();
        assert!(diff.is_empty());
        assert_eq!(diff.account_count(), 0);
    }

    #[test]
    fn serde_roundtrip() {
        let mut diff = StateDiff::new();
        let addr = Address::from_public_key(&[1u8; 32]);
        diff.accounts.insert(
            addr,
            AccountChange::Modified(Account {
                balance: 500,
                ..Account::new_eoa()
            }),
        );
        let json = serde_json::to_string(&diff).unwrap();
        let deserialized: StateDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(diff.account_count(), deserialized.account_count());
    }
}
