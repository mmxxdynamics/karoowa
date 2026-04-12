//! Light client request-response protocol over libp2p.
//!
//! Defines the wire format for `/karoowa/light/1`:
//! - `LightClientRequest::GetHeader { height }` → `Header(Option<BlockHeader>)`
//! - `LightClientRequest::GetHeaderRange { from, to }` → `Headers(Vec<BlockHeader>)`
//! - `LightClientRequest::GetStateProof { key, height }` → `StateProof(Option<MerkleProof>)`
//!
//! Mirrors the design of [`crate::state_sync`] — uses the same length-prefixed
//! bincode codec and a `LightClientProvider` trait so the network layer
//! doesn't depend on a concrete backend.

use async_trait::async_trait;
use futures::prelude::*;
use karoowa_core::BlockHeader;
use karoowa_trie::MerkleProof;
use libp2p::request_response::{self, Codec};
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::io;

/// The protocol name used by libp2p for stream negotiation.
pub const PROTOCOL_NAME: &str = "/karoowa/light/1";

/// A request sent over the light client protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LightClientRequest {
    /// Fetch a single header at the given height.
    GetHeader { height: u64 },
    /// Fetch a contiguous range of headers `[from, to]` (inclusive).
    GetHeaderRange { from: u64, to: u64 },
    /// Fetch a Merkle proof for an account/storage key at a height.
    GetStateProof { key: Vec<u8>, height: u64 },
}

/// A response to a light client request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LightClientResponse {
    /// A single header (or `None` if not found).
    Header(Option<BlockHeader>),
    /// A range of headers.
    Headers(Vec<BlockHeader>),
    /// A Merkle proof (or `None` if the height is unknown).
    StateProof(Option<MerkleProof>),
    /// The responder rejected the request.
    Error(String),
}

/// Trait that the responder side implements to serve light client data.
///
/// Wires the network layer to whatever header/state backend the node uses.
#[async_trait]
pub trait LightClientProvider: Send + Sync + 'static {
    async fn get_header(&self, height: u64) -> Option<BlockHeader>;
    async fn get_header_range(&self, from: u64, to: u64) -> Vec<BlockHeader>;
    async fn get_state_proof(&self, key: &[u8], height: u64) -> Option<MerkleProof>;
}

/// Bincode-based codec implementing libp2p's `Codec` trait.
#[derive(Debug, Clone, Default)]
pub struct LightClientCodec;

/// Maximum size of a request/response message in bytes.
/// Headers are small but a header range could carry hundreds.
const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024; // 4 MB

#[async_trait]
impl Codec for LightClientCodec {
    type Protocol = StreamProtocol;
    type Request = LightClientRequest;
    type Response = LightClientResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
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

/// Construct the libp2p `request_response::Behaviour` for the light client protocol.
pub fn build_behaviour() -> request_response::Behaviour<LightClientCodec> {
    let protocol =
        StreamProtocol::try_from_owned(PROTOCOL_NAME.to_string()).expect("valid protocol name");
    let cfg = request_response::Config::default();
    request_response::Behaviour::new(
        std::iter::once((protocol, request_response::ProtocolSupport::Full)),
        cfg,
    )
}

/// A re-export of the request-response event type for convenience.
pub type LightClientEvent = request_response::Event<LightClientRequest, LightClientResponse>;
