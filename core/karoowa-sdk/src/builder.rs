//! Transaction builder helpers.

use karoowa_core::Transaction;
use karoowa_crypto::Address;

use crate::wallet::Wallet;

/// Builder for value transfer transactions.
pub struct TransferBuilder {
    to: Option<Address>,
    value: u64,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
}

impl TransferBuilder {
    /// Create a new transfer builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        TransferBuilder {
            to: None,
            value: 0,
            nonce: 0,
            gas_price: 1,
            gas_limit: 21_000,
        }
    }

    /// Set the recipient address.
    #[must_use]
    pub fn to(mut self, addr: Address) -> Self {
        self.to = Some(addr);
        self
    }

    /// Set the transfer value.
    #[must_use]
    pub fn value(mut self, v: u64) -> Self {
        self.value = v;
        self
    }

    /// Set the sender nonce.
    #[must_use]
    pub fn nonce(mut self, n: u64) -> Self {
        self.nonce = n;
        self
    }

    /// Set the gas price.
    #[must_use]
    pub fn gas_price(mut self, p: u64) -> Self {
        self.gas_price = p;
        self
    }

    /// Set the gas limit.
    #[must_use]
    pub fn gas_limit(mut self, l: u64) -> Self {
        self.gas_limit = l;
        self
    }

    /// Sign the transfer with the given wallet.
    ///
    /// # Panics
    ///
    /// Panics if `to` was not set.
    pub fn sign(self, wallet: &Wallet) -> Transaction {
        let to = self.to.expect("TransferBuilder: `to` address is required");
        wallet.sign_transfer(to, self.value, self.nonce, self.gas_price, self.gas_limit)
    }
}

impl Default for TransferBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Placeholder builder for contract calls (real implementation in M3).
pub struct ContractCallBuilder {
    to: Option<Address>,
    value: u64,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    data: Vec<u8>,
}

impl ContractCallBuilder {
    /// Create a new contract call builder.
    #[must_use]
    pub fn new() -> Self {
        ContractCallBuilder {
            to: None,
            value: 0,
            nonce: 0,
            gas_price: 1,
            gas_limit: 100_000,
            data: Vec::new(),
        }
    }

    /// Set the contract address.
    #[must_use]
    pub fn to(mut self, addr: Address) -> Self {
        self.to = Some(addr);
        self
    }

    /// Set the value to send with the call.
    #[must_use]
    pub fn value(mut self, v: u64) -> Self {
        self.value = v;
        self
    }

    /// Set the sender nonce.
    #[must_use]
    pub fn nonce(mut self, n: u64) -> Self {
        self.nonce = n;
        self
    }

    /// Set the gas price.
    #[must_use]
    pub fn gas_price(mut self, p: u64) -> Self {
        self.gas_price = p;
        self
    }

    /// Set the gas limit.
    #[must_use]
    pub fn gas_limit(mut self, l: u64) -> Self {
        self.gas_limit = l;
        self
    }

    /// Set the call data (ABI-encoded function call).
    #[must_use]
    pub fn data(mut self, d: Vec<u8>) -> Self {
        self.data = d;
        self
    }

    /// Sign the contract call with the given wallet.
    pub fn sign(self, wallet: &Wallet) -> Transaction {
        wallet.sign_with_data(
            self.to,
            self.value,
            self.nonce,
            self.gas_price,
            self.gas_limit,
            self.data,
        )
    }
}

impl Default for ContractCallBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_builder_signs() {
        let wallet = Wallet::from_seed(&[1u8; 32], 42);
        let to = Address::from_public_key(&[2u8; 32]);

        let tx = TransferBuilder::new()
            .to(to)
            .value(500)
            .nonce(3)
            .gas_price(2)
            .gas_limit(21_000)
            .sign(&wallet);

        assert_ne!(tx.hash(), karoowa_crypto::Hash::ZERO);
    }

    #[test]
    fn contract_call_builder_signs() {
        let wallet = Wallet::from_seed(&[1u8; 32], 42);
        let to = Address::from_public_key(&[3u8; 32]);

        let tx = ContractCallBuilder::new()
            .to(to)
            .data(vec![0xde, 0xad, 0xbe, 0xef])
            .nonce(0)
            .sign(&wallet);

        assert_ne!(tx.hash(), karoowa_crypto::Hash::ZERO);
    }
}
