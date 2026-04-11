//! `NodeClient` — async HTTP client wrapping the Karoowa JSON-RPC surface.
//!
//! Provides typed methods for all 14 `kw_*` JSON-RPC methods so dApp
//! developers don't have to hand-roll requests.

use karoowa_core::{Block, Receipt};
use karoowa_crypto::{Address, Hash};
use reqwest::Client;
use serde_json::{json, Value};

use crate::error::SdkError;

/// Async client for a Karoowa node's JSON-RPC endpoint.
#[derive(Debug, Clone)]
pub struct NodeClient {
    rpc_url: String,
    http: Client,
}

impl NodeClient {
    /// Create a new client pointing at the given RPC endpoint.
    ///
    /// # Example
    /// ```no_run
    /// use karoowa_sdk::NodeClient;
    /// let client = NodeClient::new("http://localhost:8545");
    /// ```
    pub fn new(rpc_url: &str) -> Self {
        NodeClient {
            rpc_url: format!("{}/rpc", rpc_url.trim_end_matches('/')),
            http: Client::new(),
        }
    }

    /// Send a JSON-RPC request and return the `result` field.
    async fn call(&self, method: &str, params: Value) -> Result<Value, SdkError> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp: Value = self
            .http
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if let Some(err) = resp.get("error") {
            let code = err["code"].as_i64().unwrap_or(-1);
            let message = err["message"].as_str().unwrap_or("unknown").to_string();
            return Err(SdkError::Rpc { code, message });
        }

        Ok(resp["result"].clone())
    }

    // -- Read methods -------------------------------------------------------

    /// Returns the chain ID.
    pub async fn chain_id(&self) -> Result<u64, SdkError> {
        let result = self.call("kw_chainId", json!([])).await?;
        result
            .as_u64()
            .ok_or_else(|| SdkError::Parse("chain_id not a u64".into()))
    }

    /// Returns the current block height.
    pub async fn block_number(&self) -> Result<u64, SdkError> {
        let result = self.call("kw_blockNumber", json!([])).await?;
        result
            .as_u64()
            .ok_or_else(|| SdkError::Parse("block_number not a u64".into()))
    }

    /// Returns the block at the given height, or `None`.
    pub async fn get_block_by_number(&self, height: u64) -> Result<Option<Block>, SdkError> {
        let result = self.call("kw_getBlockByNumber", json!([height])).await?;
        if result.is_null() {
            return Ok(None);
        }
        serde_json::from_value(result)
            .map(Some)
            .map_err(|e| SdkError::Parse(format!("block: {e}")))
    }

    /// Returns the block with the given hash, or `None`.
    pub async fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>, SdkError> {
        let result = self
            .call("kw_getBlockByHash", json!([hash.to_string()]))
            .await?;
        if result.is_null() {
            return Ok(None);
        }
        serde_json::from_value(result)
            .map(Some)
            .map_err(|e| SdkError::Parse(format!("block: {e}")))
    }

    /// Returns transaction info by hash, or `None`.
    pub async fn get_transaction_by_hash(&self, hash: &Hash) -> Result<Option<Value>, SdkError> {
        let result = self
            .call("kw_getTransactionByHash", json!([hash.to_string()]))
            .await?;
        if result.is_null() {
            return Ok(None);
        }
        Ok(Some(result))
    }

    /// Returns the receipt for the given transaction hash, or `None`.
    pub async fn get_transaction_receipt(&self, hash: &Hash) -> Result<Option<Receipt>, SdkError> {
        let result = self
            .call("kw_getTransactionReceipt", json!([hash.to_string()]))
            .await?;
        if result.is_null() {
            return Ok(None);
        }
        serde_json::from_value(result)
            .map(Some)
            .map_err(|e| SdkError::Parse(format!("receipt: {e}")))
    }

    /// Returns the balance for the given address.
    pub async fn get_balance(&self, address: &Address) -> Result<u64, SdkError> {
        let result = self
            .call("kw_getBalance", json!([address.to_string()]))
            .await?;
        result
            .as_u64()
            .ok_or_else(|| SdkError::Parse("balance not a u64".into()))
    }

    /// Returns the nonce (transaction count) for the given address.
    pub async fn get_transaction_count(&self, address: &Address) -> Result<u64, SdkError> {
        let result = self
            .call("kw_getTransactionCount", json!([address.to_string()]))
            .await?;
        result
            .as_u64()
            .ok_or_else(|| SdkError::Parse("nonce not a u64".into()))
    }

    /// Returns the code hash for the given address, or `None` for EOAs.
    pub async fn get_code(&self, address: &Address) -> Result<Option<String>, SdkError> {
        let result = self
            .call("kw_getCode", json!([address.to_string()]))
            .await?;
        if result.is_null() {
            return Ok(None);
        }
        Ok(result.as_str().map(|s| s.to_string()))
    }

    /// Returns whether the node is syncing.
    pub async fn syncing(&self) -> Result<bool, SdkError> {
        let result = self.call("kw_syncing", json!([])).await?;
        Ok(result.as_bool().unwrap_or(false))
    }

    /// Returns the number of connected peers.
    pub async fn peer_count(&self) -> Result<u64, SdkError> {
        let result = self.call("kw_peerCount", json!([])).await?;
        result
            .as_u64()
            .ok_or_else(|| SdkError::Parse("peer_count not a u64".into()))
    }

    /// Returns node info (peer_id, chain_id, version).
    pub async fn node_info(&self) -> Result<Value, SdkError> {
        self.call("kw_nodeInfo", json!([])).await
    }

    // -- Write methods ------------------------------------------------------

    /// Submit a raw signed transaction (hex-encoded).
    /// Returns the transaction hash.
    pub async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, SdkError> {
        let result = self.call("kw_sendRawTransaction", json!([tx_hex])).await?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| SdkError::Parse("tx hash not a string".into()))
    }

    /// Returns the hashes of pending transactions in the mempool.
    pub async fn pending_transactions(&self) -> Result<Vec<String>, SdkError> {
        let result = self.call("kw_pendingTransactions", json!([])).await?;
        serde_json::from_value(result).map_err(|e| SdkError::Parse(format!("pending: {e}")))
    }
}
