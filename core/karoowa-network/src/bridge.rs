//! Bridge request-response protocol over libp2p.
//!
//! Defines the wire format for `/karoowa/bridge/1`:
//! - `BridgeRequest::SubmitPacket { packet, proof, source_state_root }` →
//!   `BridgeResponse::Acknowledgement(Acknowledgement)`
//! - `BridgeRequest::GetPacketProof { packet_hash }` →
//!   `BridgeResponse::PacketProof(Option<PacketProof>)`
//! - `BridgeRequest::GetAcknowledgement { packet_hash }` →
//!   `BridgeResponse::Acknowledgement(...)` or `BridgeResponse::Error`
//!
//! Mirrors the design of [`crate::state_sync`] and [`crate::light_client`] —
//! same length-prefixed bincode codec, same `BridgeProtocolProvider` trait
//! abstraction so the network layer doesn't import bridge logic directly.

use async_trait::async_trait;
use futures::prelude::*;
use karoowa_bridge::{Acknowledgement, BridgePacket, PacketProof};
use karoowa_crypto::Hash;
use libp2p::request_response::{self, Codec};
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::io;

/// The protocol name used by libp2p for stream negotiation.
pub const PROTOCOL_NAME: &str = "/karoowa/bridge/1";

/// A request sent over the bridge protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum BridgeRequest {
    /// Submit a packet (with its proof) to the destination chain for processing.
    SubmitPacket {
        packet: BridgePacket,
        proof: PacketProof,
        source_state_root: Hash,
    },
    /// Fetch the commitment proof for a previously sent packet.
    GetPacketProof { packet_hash: Hash },
    /// Fetch the acknowledgement for a packet that was already processed.
    GetAcknowledgement { packet_hash: Hash },
}

/// A response to a bridge request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeResponse {
    /// Acknowledgement returned by the destination after processing a packet.
    Acknowledgement(Acknowledgement),
    /// A commitment proof, or `None` if the packet is unknown.
    PacketProof(Option<PacketProof>),
    /// The responder rejected the request.
    Error(String),
}

/// Trait that the responder side implements to serve bridge protocol requests.
#[async_trait]
pub trait BridgeProtocolProvider: Send + Sync + 'static {
    /// Process a submitted packet (verify proof, mint wrapped tokens, ack).
    async fn submit_packet(
        &self,
        packet: BridgePacket,
        proof: PacketProof,
        source_state_root: Hash,
    ) -> Acknowledgement;

    /// Look up the proof for a previously sent packet.
    async fn get_packet_proof(&self, packet_hash: &Hash) -> Option<PacketProof>;

    /// Look up the acknowledgement for a previously processed packet.
    async fn get_acknowledgement(&self, packet_hash: &Hash) -> Option<Acknowledgement>;
}

/// Bincode-based codec implementing libp2p's `Codec` trait.
#[derive(Debug, Clone, Default)]
pub struct BridgeCodec;

/// Maximum size of a request/response message in bytes.
/// Bridge packets are small (<1KB) but proofs can be ~16KB for SMTs.
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MB

#[async_trait]
impl Codec for BridgeCodec {
    type Protocol = StreamProtocol;
    type Request = BridgeRequest;
    type Response = BridgeResponse;

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

/// Construct the libp2p `request_response::Behaviour` for the bridge protocol.
pub fn build_behaviour() -> request_response::Behaviour<BridgeCodec> {
    let protocol =
        StreamProtocol::try_from_owned(PROTOCOL_NAME.to_string()).expect("valid protocol name");
    let cfg = request_response::Config::default();
    request_response::Behaviour::new(
        std::iter::once((protocol, request_response::ProtocolSupport::Full)),
        cfg,
    )
}

/// A re-export of the request-response event type for convenience.
pub type BridgeProtocolEvent = request_response::Event<BridgeRequest, BridgeResponse>;
