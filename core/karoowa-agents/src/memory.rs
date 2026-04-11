//! Agent memory store.
//!
//! Provides a simple in-memory store for M1. The full LanceDB integration
//! requires the `lancedb` crate which has heavy native dependencies (Arrow,
//! etc.) that are best added incrementally. This module provides the trait
//! surface and an in-memory implementation so agents can use memory
//! immediately; the LanceDB backend slots in behind the same trait.
//!
//! The `MemoryStore` trait is designed so that swapping to LanceDB (or
//! Qdrant, or any vector store) requires no changes to agent code.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::AgentError;

/// A single memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier.
    pub id: String,
    /// The text content of the memory.
    pub content: String,
    /// Metadata tags for filtering.
    pub tags: HashMap<String, String>,
}

/// Trait for agent memory storage.
///
/// M1 ships an in-memory implementation. LanceDB (L3 in the database
/// strategy) slots in behind this trait in a future update.
#[async_trait::async_trait]
pub trait MemoryStore: Send + Sync {
    /// Insert a memory entry.
    async fn insert(&self, entry: MemoryEntry) -> Result<(), AgentError>;

    /// Query for relevant memories. Returns up to `top_k` entries
    /// matching the query string (by keyword for in-memory, by semantic
    /// similarity for vector stores).
    async fn query(&self, query: &str, top_k: usize) -> Result<Vec<MemoryEntry>, AgentError>;

    /// Delete a memory entry by ID.
    async fn delete(&self, id: &str) -> Result<(), AgentError>;
}

/// Simple in-memory store backed by a `Vec`. Keyword matching for queries.
///
/// Suitable for development and testing. Production deployments should
/// use the LanceDB backend (when available) for semantic search.
#[derive(Clone)]
pub struct InMemoryStore {
    entries: Arc<Mutex<Vec<MemoryEntry>>>,
}

impl InMemoryStore {
    #[must_use]
    pub fn new() -> Self {
        InMemoryStore {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MemoryStore for InMemoryStore {
    async fn insert(&self, entry: MemoryEntry) -> Result<(), AgentError> {
        self.entries.lock().await.push(entry);
        Ok(())
    }

    async fn query(&self, query: &str, top_k: usize) -> Result<Vec<MemoryEntry>, AgentError> {
        let entries = self.entries.lock().await;
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(usize, &MemoryEntry)> = entries
            .iter()
            .map(|e| {
                let content_lower = e.content.to_lowercase();
                let score = query_words
                    .iter()
                    .filter(|w| content_lower.contains(*w))
                    .count();
                (score, e)
            })
            .filter(|(score, _)| *score > 0)
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(scored
            .into_iter()
            .take(top_k)
            .map(|(_, e)| e.clone())
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<(), AgentError> {
        let mut entries = self.entries.lock().await;
        entries.retain(|e| e.id != id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_query() {
        let store = InMemoryStore::new();

        store
            .insert(MemoryEntry {
                id: "1".into(),
                content: "The devnet bootnode IP is 10.0.0.1".into(),
                tags: HashMap::new(),
            })
            .await
            .unwrap();

        store
            .insert(MemoryEntry {
                id: "2".into(),
                content: "The chain ID is 42".into(),
                tags: HashMap::new(),
            })
            .await
            .unwrap();

        let results = store.query("bootnode IP address", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
    }

    #[tokio::test]
    async fn delete_entry() {
        let store = InMemoryStore::new();

        store
            .insert(MemoryEntry {
                id: "1".into(),
                content: "test entry".into(),
                tags: HashMap::new(),
            })
            .await
            .unwrap();

        store.delete("1").await.unwrap();
        let results = store.query("test", 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn query_returns_top_k() {
        let store = InMemoryStore::new();

        for i in 0..10 {
            store
                .insert(MemoryEntry {
                    id: format!("{i}"),
                    content: format!("entry {i} about karoowa blockchain"),
                    tags: HashMap::new(),
                })
                .await
                .unwrap();
        }

        let results = store.query("karoowa blockchain", 3).await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
