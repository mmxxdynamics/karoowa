//! State sync request-response protocol over libp2p.
//!
//! Defines the wire format for `/karoowa/state-sync/1`:
//! - `SnapshotRequest::ListSnapshots` → `SnapshotResponse::Manifests(Vec<...>)`
//! - `SnapshotRequest::GetManifest { height }` → `SnapshotResponse::Manifest(...)`
//! - `SnapshotRequest::GetChunk { height, index }` → `SnapshotResponse::Chunk(...)`
//!
//! The protocol uses libp2p's `request_response::Behaviour` with a custom
//! bincode codec. A `SnapshotProvider` trait abstracts the responder side
//! so the network layer doesn't depend on a concrete storage backend.

use async_trait::async_trait;
use futures::prelude::*;
use karoowa_storage::{SnapshotChunk, SnapshotManifest};
use libp2p::request_response::{self, Codec};
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::io;

/// The protocol name used by libp2p for stream negotiation.
pub const PROTOCOL_NAME: &str = "/karoowa/state-sync/1";

/// A request sent over the state-sync protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotRequest {
    /// List all snapshot manifests the responder has.
    ListSnapshots,
    /// Fetch the manifest for a specific snapshot height.
    GetManifest { height: u64 },
    /// Fetch a specific chunk from a snapshot.
    GetChunk { height: u64, index: u32 },
}

/// A response to a state-sync request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotResponse {
    /// List of available snapshot manifests (response to `ListSnapshots`).
    Manifests(Vec<SnapshotManifest>),
    /// A specific snapshot manifest, or `None` if not found.
    Manifest(Option<SnapshotManifest>),
    /// A specific chunk, or `None` if not found.
    Chunk(Option<SnapshotChunk>),
    /// The responder rejected the request (e.g. malformed, unauthorized).
    Error(String),
}

/// Trait that the responder side implements to serve snapshot data.
///
/// Wires the network layer to whatever `SnapshotStore` the node uses.
#[async_trait]
pub trait SnapshotProvider: Send + Sync + 'static {
    async fn list_snapshots(&self) -> Vec<SnapshotManifest>;
    async fn get_manifest(&self, height: u64) -> Option<SnapshotManifest>;
    async fn get_chunk(&self, height: u64, index: u32) -> Option<SnapshotChunk>;
}

/// Bincode-based codec implementing libp2p's `Codec` trait.
#[derive(Debug, Clone, Default)]
pub struct StateSyncCodec;

/// Maximum size of a request/response message in bytes.
/// Chunks are ~4 MB compressed; allow some overhead for the manifest list.
const MAX_MESSAGE_SIZE: usize = 8 * 1024 * 1024; // 8 MB

#[async_trait]
impl Codec for StateSyncCodec {
    type Protocol = StreamProtocol;
    type Request = SnapshotRequest;
    type Response = SnapshotResponse;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let bytes = read_length_prefixed(io, MAX_MESSAGE_SIZE).await?;
        bincode::deserialize(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let bytes = read_length_prefixed(io, MAX_MESSAGE_SIZE).await?;
        bincode::deserialize(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = bincode::serialize(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        write_length_prefixed(io, &bytes).await
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        resp: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = bincode::serialize(&resp)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        write_length_prefixed(io, &bytes).await
    }
}

/// Read a length-prefixed (u32 LE) message from a stream.
async fn read_length_prefixed<T>(io: &mut T, max_size: usize) -> io::Result<Vec<u8>>
where
    T: AsyncRead + Unpin + Send,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > max_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("message too large: {len} > {max_size}"),
        ));
    }
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Write a length-prefixed (u32 LE) message to a stream.
async fn write_length_prefixed<T>(io: &mut T, data: &[u8]) -> io::Result<()>
where
    T: AsyncWrite + Unpin + Send,
{
    let len = data.len() as u32;
    io.write_all(&len.to_le_bytes()).await?;
    io.write_all(data).await?;
    io.flush().await?;
    Ok(())
}

/// Construct the libp2p `request_response::Behaviour` for state sync.
pub fn build_behaviour() -> request_response::Behaviour<StateSyncCodec> {
    let protocol = StreamProtocol::try_from_owned(PROTOCOL_NAME.to_string())
        .expect("valid protocol name");
    let cfg = request_response::Config::default();
    request_response::Behaviour::new(
        std::iter::once((protocol, request_response::ProtocolSupport::Full)),
        cfg,
    )
}

/// A re-export of the request-response event type for convenience.
pub type StateSyncEvent = request_response::Event<SnapshotRequest, SnapshotResponse>;
