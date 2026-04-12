//! Validator set view used by the light client.
//!
//! The light client tracks the active validator set externally rather than
//! reading it from the block header (which would require a breaking change).
//! When the validator set changes on-chain, the light client must be told
//! about it via [`LightClient::update_validator_set`].

use karoowa_crypto::Address;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// An immutable snapshot of the active validator set at a given height.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorSetView {
    /// Set of active validator addresses.
    validators: BTreeSet<Address>,
    /// Height at which this set became active.
    pub effective_from: u64,
}

impl ValidatorSetView {
    /// Create a new validator set view from a list of addresses.
    pub fn new(validators: Vec<Address>, effective_from: u64) -> Self {
        ValidatorSetView {
            validators: validators.into_iter().collect(),
            effective_from,
        }
    }

    /// Check if an address is an active validator in this view.
    pub fn contains(&self, address: &Address) -> bool {
        self.validators.contains(address)
    }

    /// Number of active validators.
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }

    /// Iterate active validators.
    pub fn iter(&self) -> impl Iterator<Item = &Address> {
        self.validators.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(seed: u8) -> Address {
        Address::from_public_key(&[seed; 32])
    }

    #[test]
    fn new_view_dedupes_and_sorts() {
        let view = ValidatorSetView::new(vec![addr(2), addr(1), addr(2), addr(3)], 0);
        assert_eq!(view.len(), 3);
        assert!(view.contains(&addr(1)));
        assert!(view.contains(&addr(2)));
        assert!(view.contains(&addr(3)));
        assert!(!view.contains(&addr(4)));
    }

    #[test]
    fn empty_view() {
        let view = ValidatorSetView::new(vec![], 0);
        assert!(view.is_empty());
        assert_eq!(view.len(), 0);
    }

    #[test]
    fn effective_from() {
        let view = ValidatorSetView::new(vec![addr(1)], 100);
        assert_eq!(view.effective_from, 100);
    }
}
