//! Escrow store for the lock-and-mint bridge model.
//!
//! On the source chain, native tokens being bridged are locked in an
//! escrow account. On the destination chain, wrapped tokens are minted
//! against the locked supply. When wrapped tokens are bridged back, they
//! are burned on the destination chain and the original tokens are
//! released from escrow on the source chain.

use karoowa_crypto::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::BridgeError;

/// A balance entry tracking native or wrapped token amounts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BalanceEntry {
    pub amount: u64,
}

/// Trait for the escrow + wrapped balance store. Each chain has its own
/// instance, tracking what's locked (source side) and what's minted
/// (destination side) for each (denom, recipient) pair.
pub trait EscrowStore: Send + Sync {
    /// Lock `amount` of `denom` from `from` into the bridge escrow.
    /// Used on the source chain when a transfer is initiated.
    fn lock(&self, from: &Address, denom: &str, amount: u64) -> Result<(), BridgeError>;

    /// Release `amount` of `denom` from escrow to `to`.
    /// Used on the source chain when a return packet is received.
    fn release(&self, to: &Address, denom: &str, amount: u64) -> Result<(), BridgeError>;

    /// Mint `amount` of wrapped `denom` to `to` on the destination chain.
    fn mint(&self, to: &Address, denom: &str, amount: u64) -> Result<(), BridgeError>;

    /// Burn `amount` of wrapped `denom` from `from` on the destination chain.
    fn burn(&self, from: &Address, denom: &str, amount: u64) -> Result<(), BridgeError>;

    /// Get the wrapped balance for an account.
    fn balance_of(&self, addr: &Address, denom: &str) -> u64;

    /// Get the native balance for an account.
    fn native_balance_of(&self, addr: &Address, denom: &str) -> u64;

    /// Get the total amount locked in escrow for a denom.
    fn escrowed(&self, denom: &str) -> u64;
}

/// In-memory escrow store for testing and the MVP relayer.
pub struct InMemoryEscrow {
    inner: Mutex<EscrowState>,
}

#[derive(Default)]
struct EscrowState {
    /// Native balances: (denom, address) → amount.
    native: HashMap<(String, Address), u64>,
    /// Wrapped balances: (denom, address) → amount.
    wrapped: HashMap<(String, Address), u64>,
    /// Total escrowed per denom.
    escrowed: HashMap<String, u64>,
}

impl InMemoryEscrow {
    pub fn new() -> Self {
        InMemoryEscrow {
            inner: Mutex::new(EscrowState::default()),
        }
    }

    /// Pre-fund a native balance (for testing setup).
    pub fn fund_native(&self, addr: Address, denom: &str, amount: u64) {
        let mut state = self.inner.lock().unwrap();
        *state.native.entry((denom.to_string(), addr)).or_insert(0) += amount;
    }
}

impl Default for InMemoryEscrow {
    fn default() -> Self {
        Self::new()
    }
}

impl EscrowStore for InMemoryEscrow {
    fn lock(&self, from: &Address, denom: &str, amount: u64) -> Result<(), BridgeError> {
        let mut state = self.inner.lock().unwrap();
        let key = (denom.to_string(), *from);
        let balance = state.native.get(&key).copied().unwrap_or(0);
        if balance < amount {
            return Err(BridgeError::InsufficientBalance {
                needed: amount,
                available: balance,
            });
        }
        state.native.insert(key, balance - amount);
        *state.escrowed.entry(denom.to_string()).or_insert(0) += amount;
        Ok(())
    }

    fn release(&self, to: &Address, denom: &str, amount: u64) -> Result<(), BridgeError> {
        let mut state = self.inner.lock().unwrap();
        let escrowed = state.escrowed.get(denom).copied().unwrap_or(0);
        if escrowed < amount {
            return Err(BridgeError::InsufficientBalance {
                needed: amount,
                available: escrowed,
            });
        }
        state.escrowed.insert(denom.to_string(), escrowed - amount);
        *state.native.entry((denom.to_string(), *to)).or_insert(0) += amount;
        Ok(())
    }

    fn mint(&self, to: &Address, denom: &str, amount: u64) -> Result<(), BridgeError> {
        let mut state = self.inner.lock().unwrap();
        *state.wrapped.entry((denom.to_string(), *to)).or_insert(0) += amount;
        Ok(())
    }

    fn burn(&self, from: &Address, denom: &str, amount: u64) -> Result<(), BridgeError> {
        let mut state = self.inner.lock().unwrap();
        let key = (denom.to_string(), *from);
        let balance = state.wrapped.get(&key).copied().unwrap_or(0);
        if balance < amount {
            return Err(BridgeError::InsufficientBalance {
                needed: amount,
                available: balance,
            });
        }
        state.wrapped.insert(key, balance - amount);
        Ok(())
    }

    fn balance_of(&self, addr: &Address, denom: &str) -> u64 {
        let state = self.inner.lock().unwrap();
        state
            .wrapped
            .get(&(denom.to_string(), *addr))
            .copied()
            .unwrap_or(0)
    }

    fn native_balance_of(&self, addr: &Address, denom: &str) -> u64 {
        let state = self.inner.lock().unwrap();
        state
            .native
            .get(&(denom.to_string(), *addr))
            .copied()
            .unwrap_or(0)
    }

    fn escrowed(&self, denom: &str) -> u64 {
        let state = self.inner.lock().unwrap();
        state.escrowed.get(denom).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(seed: u8) -> Address {
        Address::from_public_key(&[seed; 32])
    }

    #[test]
    fn lock_reduces_native_increases_escrow() {
        let store = InMemoryEscrow::new();
        store.fund_native(addr(1), "kar", 1000);
        store.lock(&addr(1), "kar", 300).unwrap();

        assert_eq!(store.native_balance_of(&addr(1), "kar"), 700);
        assert_eq!(store.escrowed("kar"), 300);
    }

    #[test]
    fn lock_insufficient_balance() {
        let store = InMemoryEscrow::new();
        store.fund_native(addr(1), "kar", 100);
        let result = store.lock(&addr(1), "kar", 200);
        assert!(matches!(
            result,
            Err(BridgeError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn release_increases_native_decreases_escrow() {
        let store = InMemoryEscrow::new();
        store.fund_native(addr(1), "kar", 1000);
        store.lock(&addr(1), "kar", 500).unwrap();
        store.release(&addr(2), "kar", 200).unwrap();

        assert_eq!(store.escrowed("kar"), 300);
        assert_eq!(store.native_balance_of(&addr(2), "kar"), 200);
    }

    #[test]
    fn release_more_than_escrowed_fails() {
        let store = InMemoryEscrow::new();
        let result = store.release(&addr(1), "kar", 100);
        assert!(matches!(
            result,
            Err(BridgeError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn mint_increases_wrapped_balance() {
        let store = InMemoryEscrow::new();
        store.mint(&addr(1), "ibc/kar", 500).unwrap();
        assert_eq!(store.balance_of(&addr(1), "ibc/kar"), 500);
        store.mint(&addr(1), "ibc/kar", 300).unwrap();
        assert_eq!(store.balance_of(&addr(1), "ibc/kar"), 800);
    }

    #[test]
    fn burn_reduces_wrapped_balance() {
        let store = InMemoryEscrow::new();
        store.mint(&addr(1), "ibc/kar", 1000).unwrap();
        store.burn(&addr(1), "ibc/kar", 400).unwrap();
        assert_eq!(store.balance_of(&addr(1), "ibc/kar"), 600);
    }

    #[test]
    fn burn_more_than_held_fails() {
        let store = InMemoryEscrow::new();
        store.mint(&addr(1), "ibc/kar", 100).unwrap();
        let result = store.burn(&addr(1), "ibc/kar", 200);
        assert!(matches!(
            result,
            Err(BridgeError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn separate_denoms_independent() {
        let store = InMemoryEscrow::new();
        store.fund_native(addr(1), "kar", 1000);
        store.fund_native(addr(1), "usd", 500);

        store.lock(&addr(1), "kar", 200).unwrap();
        assert_eq!(store.escrowed("kar"), 200);
        assert_eq!(store.escrowed("usd"), 0);
        assert_eq!(store.native_balance_of(&addr(1), "kar"), 800);
        assert_eq!(store.native_balance_of(&addr(1), "usd"), 500);
    }
}
