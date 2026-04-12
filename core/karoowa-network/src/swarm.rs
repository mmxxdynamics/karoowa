//! The `Network` struct — owns the libp2p Swarm and drives the event loop.
//!
//! Users interact with the network via the [`NetworkHandle`] returned by
//! [`Network::start`]. The handle is cheaply cloneable and safe to share
//! across tasks.

use karoowa_core::{Block, Transaction};
use libp2p::futures::StreamExt;
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::identify;
use libp2p::kad;
use libp2p::request_response::{self, OutboundRequestId};
use libp2p::swarm::SwarmEvent;
use libp2p::{identity::Keypair, Multiaddr, PeerId, Swarm, SwarmBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, info, warn};

use crate::behaviour::{self, BehaviourEvent, KaroowaBehaviour, TOPIC_BLOCKS, TOPIC_TRANSACTIONS};
use crate::bridge::{BridgeProtocolProvider, BridgeRequest, BridgeResponse};
use crate::config::NetworkConfig;
use crate::error::NetworkError;
use crate::light_client::{LightClientProvider, LightClientRequest, LightClientResponse};
use crate::state_sync::{SnapshotProvider, SnapshotRequest, SnapshotResponse};

/// Commands sent from the handle to the swarm event loop.
#[allow(clippy::large_enum_variant)]
enum Command {
    Publish {
        topic: String,
        data: Vec<u8>,
        reply: oneshot::Sender<Result<(), NetworkError>>,
    },
    Dial {
        addr: Multiaddr,
        reply: oneshot::Sender<Result<(), NetworkError>>,
    },
    PeerCount {
        reply: oneshot::Sender<usize>,
    },
    ConnectedPeers {
        reply: oneshot::Sender<Vec<PeerId>>,
    },
    ListenAddress {
        reply: oneshot::Sender<Vec<Multiaddr>>,
    },
    SendSnapshotRequest {
        peer: PeerId,
        request: SnapshotRequest,
        reply: oneshot::Sender<Result<SnapshotResponse, NetworkError>>,
    },
    SetSnapshotProvider {
        provider: Arc<dyn SnapshotProvider>,
        reply: oneshot::Sender<()>,
    },
    SendLightRequest {
        peer: PeerId,
        request: LightClientRequest,
        reply: oneshot::Sender<Result<LightClientResponse, NetworkError>>,
    },
    SetLightProvider {
        provider: Arc<dyn LightClientProvider>,
        reply: oneshot::Sender<()>,
    },
    SendBridgeRequest {
        peer: PeerId,
        request: BridgeRequest,
        reply: oneshot::Sender<Result<BridgeResponse, NetworkError>>,
    },
    SetBridgeProvider {
        provider: Arc<dyn BridgeProtocolProvider>,
        reply: oneshot::Sender<()>,
    },
}

/// Handle to the running network. Cheaply cloneable.
#[derive(Clone)]
pub struct NetworkHandle {
    cmd_tx: mpsc::Sender<Command>,
    block_tx: broadcast::Sender<Block>,
    tx_tx: broadcast::Sender<Transaction>,
    peer_count: Arc<AtomicUsize>,
    local_peer_id: PeerId,
}

impl NetworkHandle {
    /// This node's PeerId.
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Current connected peer count (lock-free read).
    pub fn peer_count(&self) -> usize {
        self.peer_count.load(Ordering::Relaxed)
    }

    /// Get the exact peer count by querying the swarm (async, goes through
    /// the event loop).
    pub async fn peer_count_exact(&self) -> Result<usize, NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::PeerCount { reply })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)
    }

    /// Get the list of connected peer IDs.
    pub async fn connected_peers(&self) -> Result<Vec<PeerId>, NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::ConnectedPeers { reply })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)
    }

    /// Get the addresses this node is listening on.
    pub async fn listen_addresses(&self) -> Result<Vec<Multiaddr>, NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::ListenAddress { reply })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)
    }

    /// Broadcast a block to all connected peers.
    pub async fn broadcast_block(&self, block: &Block) -> Result<(), NetworkError> {
        let data = bincode::serialize(block)?;
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Publish {
                topic: TOPIC_BLOCKS.to_string(),
                data,
                reply,
            })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?
    }

    /// Broadcast a transaction to all connected peers.
    pub async fn broadcast_transaction(&self, tx: &Transaction) -> Result<(), NetworkError> {
        let data = bincode::serialize(tx)?;
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Publish {
                topic: TOPIC_TRANSACTIONS.to_string(),
                data,
                reply,
            })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?
    }

    /// Subscribe to incoming blocks from the network.
    pub fn subscribe_blocks(&self) -> broadcast::Receiver<Block> {
        self.block_tx.subscribe()
    }

    /// Subscribe to incoming transactions from the network.
    pub fn subscribe_transactions(&self) -> broadcast::Receiver<Transaction> {
        self.tx_tx.subscribe()
    }

    /// Dial a remote peer by multiaddress.
    pub async fn dial(&self, addr: Multiaddr) -> Result<(), NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Dial { addr, reply })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?
    }

    /// Send a snapshot request to a specific peer and await the response.
    pub async fn request_snapshot(
        &self,
        peer: PeerId,
        request: SnapshotRequest,
    ) -> Result<SnapshotResponse, NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::SendSnapshotRequest {
                peer,
                request,
                reply,
            })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?
    }

    /// Install a snapshot provider that will respond to incoming state-sync
    /// requests from other peers. Pass `None` to operate as a request-only node.
    pub async fn set_snapshot_provider(
        &self,
        provider: Arc<dyn SnapshotProvider>,
    ) -> Result<(), NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::SetSnapshotProvider { provider, reply })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?;
        Ok(())
    }

    /// Send a light client request to a specific peer and await the response.
    pub async fn request_light(
        &self,
        peer: PeerId,
        request: LightClientRequest,
    ) -> Result<LightClientResponse, NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::SendLightRequest {
                peer,
                request,
                reply,
            })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?
    }

    /// Install a light client provider for serving incoming light client requests.
    pub async fn set_light_provider(
        &self,
        provider: Arc<dyn LightClientProvider>,
    ) -> Result<(), NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::SetLightProvider { provider, reply })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?;
        Ok(())
    }

    /// Send a bridge request to a specific peer and await the response.
    pub async fn request_bridge(
        &self,
        peer: PeerId,
        request: BridgeRequest,
    ) -> Result<BridgeResponse, NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::SendBridgeRequest {
                peer,
                request,
                reply,
            })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?
    }

    /// Install a bridge provider for serving incoming bridge protocol requests.
    pub async fn set_bridge_provider(
        &self,
        provider: Arc<dyn BridgeProtocolProvider>,
    ) -> Result<(), NetworkError> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::SetBridgeProvider { provider, reply })
            .await
            .map_err(|_| NetworkError::NotRunning)?;
        rx.await.map_err(|_| NetworkError::NotRunning)?;
        Ok(())
    }
}

/// The network node. Call [`Network::start`] to spawn the event loop and
/// get a [`NetworkHandle`].
pub struct Network;

impl Network {
    /// Build the libp2p Swarm and identity from config.
    fn build_swarm(
        config: &NetworkConfig,
    ) -> Result<(Swarm<KaroowaBehaviour>, PeerId), NetworkError> {
        let keypair = match config.keypair_seed {
            Some(seed) => {
                let mut key_bytes = seed.to_vec();
                // ed25519 secret key is 32 bytes; libp2p expects it via
                // ed25519::Keypair which we build from the seed.
                let secret =
                    libp2p::identity::ed25519::SecretKey::try_from_bytes(&mut key_bytes)
                        .map_err(|e| NetworkError::Transport(format!("bad keypair seed: {e}")))?;
                let ed_keypair = libp2p::identity::ed25519::Keypair::from(secret);
                Keypair::from(ed_keypair)
            }
            None => Keypair::generate_ed25519(),
        };

        let peer_id = keypair.public().to_peer_id();

        let behaviour = behaviour::build_behaviour(
            &keypair,
            config.gossipsub_heartbeat,
            config.mesh_n,
            config.mesh_n_low,
            config.mesh_n_high,
        );

        let swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default().nodelay(true),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| NetworkError::Transport(format!("tcp transport: {e}")))?
            .with_dns()
            .map_err(|e| NetworkError::Transport(format!("dns: {e}")))?
            .with_behaviour(|_| behaviour)
            .map_err(|e| NetworkError::Transport(format!("behaviour: {e}")))?
            .build();

        Ok((swarm, peer_id))
    }

    /// Start the network: builds the swarm, begins listening, connects to
    /// bootnodes, and spawns the event loop. Returns a handle for interaction.
    pub async fn start(config: NetworkConfig) -> Result<NetworkHandle, NetworkError> {
        let (mut swarm, peer_id) = Self::build_swarm(&config)?;

        // Start listening.
        swarm
            .listen_on(config.listen_addr.clone())
            .map_err(|e| NetworkError::Listen(e.to_string()))?;

        info!(
            peer_id = %peer_id,
            listen = %config.listen_addr,
            bootnodes = config.bootnodes.len(),
            "starting P2P network"
        );

        // Add bootnodes to Kademlia and dial them.
        for addr in &config.bootnodes {
            if let Err(e) = swarm.dial(addr.clone()) {
                warn!(addr = %addr, error = %e, "failed to dial bootnode");
            }
        }

        let (cmd_tx, cmd_rx) = mpsc::channel(256);
        let (block_tx, _) = broadcast::channel(256);
        let (tx_tx, _) = broadcast::channel(1024);
        let peer_count = Arc::new(AtomicUsize::new(0));

        let handle = NetworkHandle {
            cmd_tx,
            block_tx: block_tx.clone(),
            tx_tx: tx_tx.clone(),
            peer_count: Arc::clone(&peer_count),
            local_peer_id: peer_id,
        };

        // Spawn the event loop.
        tokio::spawn(event_loop(swarm, cmd_rx, block_tx, tx_tx, peer_count));

        Ok(handle)
    }
}

/// The swarm event loop — processes libp2p events and commands from handles.
async fn event_loop(
    mut swarm: Swarm<KaroowaBehaviour>,
    mut cmd_rx: mpsc::Receiver<Command>,
    block_tx: broadcast::Sender<Block>,
    tx_tx: broadcast::Sender<Transaction>,
    peer_count: Arc<AtomicUsize>,
) {
    let mut connected_peers: HashSet<PeerId> = HashSet::new();
    // Pending snapshot requests we sent: maps libp2p request id → caller's reply channel.
    let mut pending_requests: HashMap<
        OutboundRequestId,
        oneshot::Sender<Result<SnapshotResponse, NetworkError>>,
    > = HashMap::new();
    // Pending light client requests we sent.
    let mut pending_light_requests: HashMap<
        OutboundRequestId,
        oneshot::Sender<Result<LightClientResponse, NetworkError>>,
    > = HashMap::new();
    // Pending bridge requests we sent.
    let mut pending_bridge_requests: HashMap<
        OutboundRequestId,
        oneshot::Sender<Result<BridgeResponse, NetworkError>>,
    > = HashMap::new();
    // Optional providers for serving incoming requests.
    let mut snapshot_provider: Option<Arc<dyn SnapshotProvider>> = None;
    let mut light_provider: Option<Arc<dyn LightClientProvider>> = None;
    let mut bridge_provider: Option<Arc<dyn BridgeProtocolProvider>> = None;

    loop {
        tokio::select! {
            // Handle commands from NetworkHandle.
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(Command::Publish { topic, data, reply }) => {
                        let topic = IdentTopic::new(topic);
                        let result = swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(topic, data)
                            .map(|_| ())
                            .map_err(|e| NetworkError::Publish(e.to_string()));
                        let _ = reply.send(result);
                    }
                    Some(Command::Dial { addr, reply }) => {
                        let result = swarm
                            .dial(addr)
                            .map_err(|e| NetworkError::Dial(e.to_string()));
                        let _ = reply.send(result);
                    }
                    Some(Command::PeerCount { reply }) => {
                        let _ = reply.send(connected_peers.len());
                    }
                    Some(Command::ConnectedPeers { reply }) => {
                        let peers: Vec<PeerId> = connected_peers.iter().copied().collect();
                        let _ = reply.send(peers);
                    }
                    Some(Command::ListenAddress { reply }) => {
                        let addrs: Vec<Multiaddr> = swarm.listeners().cloned().collect();
                        let _ = reply.send(addrs);
                    }
                    Some(Command::SendSnapshotRequest { peer, request, reply }) => {
                        let req_id = swarm
                            .behaviour_mut()
                            .state_sync
                            .send_request(&peer, request);
                        pending_requests.insert(req_id, reply);
                    }
                    Some(Command::SetSnapshotProvider { provider, reply }) => {
                        snapshot_provider = Some(provider);
                        let _ = reply.send(());
                    }
                    Some(Command::SendLightRequest { peer, request, reply }) => {
                        let req_id = swarm
                            .behaviour_mut()
                            .light_client
                            .send_request(&peer, request);
                        pending_light_requests.insert(req_id, reply);
                    }
                    Some(Command::SetLightProvider { provider, reply }) => {
                        light_provider = Some(provider);
                        let _ = reply.send(());
                    }
                    Some(Command::SendBridgeRequest { peer, request, reply }) => {
                        let req_id = swarm
                            .behaviour_mut()
                            .bridge
                            .send_request(&peer, request);
                        pending_bridge_requests.insert(req_id, reply);
                    }
                    Some(Command::SetBridgeProvider { provider, reply }) => {
                        bridge_provider = Some(provider);
                        let _ = reply.send(());
                    }
                    None => {
                        debug!("all handles dropped, stopping event loop");
                        return;
                    }
                }
            }

            // Handle swarm events.
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(addr = %address, "listening on");
                    }

                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        connected_peers.insert(peer_id);
                        peer_count.store(connected_peers.len(), Ordering::Relaxed);
                        debug!(peer = %peer_id, count = connected_peers.len(), "peer connected");

                        // Add to Kademlia routing table.
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, Multiaddr::empty());
                    }

                    SwarmEvent::ConnectionClosed { peer_id, num_established, .. } => {
                        if num_established == 0 {
                            connected_peers.remove(&peer_id);
                            peer_count.store(connected_peers.len(), Ordering::Relaxed);
                            debug!(peer = %peer_id, count = connected_peers.len(), "peer disconnected");
                        }
                    }

                    SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, .. },
                    )) => {
                        let topic_str = message.topic.as_str();
                        match topic_str {
                            t if t == TOPIC_BLOCKS => {
                                match bincode::deserialize::<Block>(&message.data) {
                                    Ok(block) => {
                                        debug!(
                                            height = block.height(),
                                            hash = %block.hash(),
                                            "received block from network"
                                        );
                                        // Best-effort broadcast to subscribers; drop if no receivers.
                                        let _ = block_tx.send(block);
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "failed to deserialize block from gossip");
                                    }
                                }
                            }
                            t if t == TOPIC_TRANSACTIONS => {
                                match bincode::deserialize::<Transaction>(&message.data) {
                                    Ok(tx) => {
                                        debug!(
                                            hash = %tx.hash(),
                                            "received transaction from network"
                                        );
                                        let _ = tx_tx.send(tx);
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "failed to deserialize tx from gossip");
                                    }
                                }
                            }
                            _ => {
                                debug!(topic = %topic_str, "message on unknown topic");
                            }
                        }
                    }

                    SwarmEvent::Behaviour(BehaviourEvent::Identify(event)) => {
                        if let identify::Event::Received { peer_id, info, .. } = *event {
                        // When we learn a peer's listen addresses via Identify,
                        // feed them into Kademlia so the DHT knows how to route.
                        for addr in &info.listen_addrs {
                            swarm
                                .behaviour_mut()
                                .kademlia
                                .add_address(&peer_id, addr.clone());
                        }
                        debug!(peer = %peer_id, addrs = info.listen_addrs.len(), "identify received");
                        }
                    }

                    SwarmEvent::Behaviour(BehaviourEvent::Kademlia(
                        kad::Event::RoutingUpdated { peer, .. },
                    )) => {
                        debug!(peer = %peer, "kademlia routing updated");
                    }

                    SwarmEvent::Behaviour(BehaviourEvent::StateSync(event)) => {
                        match *event {
                            request_response::Event::Message { message, .. } => {
                                match message {
                                    request_response::Message::Request {
                                        request,
                                        channel,
                                        ..
                                    } => {
                                        // Compute response from the provider (if any).
                                        let response = match &snapshot_provider {
                                            Some(provider) => handle_snapshot_request(
                                                provider.as_ref(),
                                                request,
                                            )
                                            .await,
                                            None => SnapshotResponse::Error(
                                                "no snapshot provider configured".into(),
                                            ),
                                        };
                                        let _ = swarm
                                            .behaviour_mut()
                                            .state_sync
                                            .send_response(channel, response);
                                    }
                                    request_response::Message::Response {
                                        request_id,
                                        response,
                                    } => {
                                        if let Some(reply) =
                                            pending_requests.remove(&request_id)
                                        {
                                            let _ = reply.send(Ok(response));
                                        }
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure {
                                request_id,
                                error,
                                ..
                            } => {
                                if let Some(reply) = pending_requests.remove(&request_id) {
                                    let _ = reply.send(Err(NetworkError::Transport(format!(
                                        "outbound state-sync failure: {error}"
                                    ))));
                                }
                            }
                            request_response::Event::InboundFailure { error, .. } => {
                                warn!(error = %error, "inbound state-sync failure");
                            }
                            request_response::Event::ResponseSent { .. } => {}
                        }
                    }

                    SwarmEvent::Behaviour(BehaviourEvent::LightClient(event)) => {
                        match *event {
                            request_response::Event::Message { message, .. } => {
                                match message {
                                    request_response::Message::Request {
                                        request,
                                        channel,
                                        ..
                                    } => {
                                        let response = match &light_provider {
                                            Some(provider) => handle_light_request(
                                                provider.as_ref(),
                                                request,
                                            )
                                            .await,
                                            None => LightClientResponse::Error(
                                                "no light client provider configured".into(),
                                            ),
                                        };
                                        let _ = swarm
                                            .behaviour_mut()
                                            .light_client
                                            .send_response(channel, response);
                                    }
                                    request_response::Message::Response {
                                        request_id,
                                        response,
                                    } => {
                                        if let Some(reply) =
                                            pending_light_requests.remove(&request_id)
                                        {
                                            let _ = reply.send(Ok(response));
                                        }
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure {
                                request_id,
                                error,
                                ..
                            } => {
                                if let Some(reply) =
                                    pending_light_requests.remove(&request_id)
                                {
                                    let _ = reply.send(Err(NetworkError::Transport(format!(
                                        "outbound light client failure: {error}"
                                    ))));
                                }
                            }
                            request_response::Event::InboundFailure { error, .. } => {
                                warn!(error = %error, "inbound light client failure");
                            }
                            request_response::Event::ResponseSent { .. } => {}
                        }
                    }

                    SwarmEvent::Behaviour(BehaviourEvent::Bridge(event)) => {
                        match *event {
                            request_response::Event::Message { message, .. } => {
                                match message {
                                    request_response::Message::Request {
                                        request,
                                        channel,
                                        ..
                                    } => {
                                        let response = match &bridge_provider {
                                            Some(provider) => handle_bridge_request(
                                                provider.as_ref(),
                                                request,
                                            )
                                            .await,
                                            None => BridgeResponse::Error(
                                                "no bridge provider configured".into(),
                                            ),
                                        };
                                        let _ = swarm
                                            .behaviour_mut()
                                            .bridge
                                            .send_response(channel, response);
                                    }
                                    request_response::Message::Response {
                                        request_id,
                                        response,
                                    } => {
                                        if let Some(reply) =
                                            pending_bridge_requests.remove(&request_id)
                                        {
                                            let _ = reply.send(Ok(response));
                                        }
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure {
                                request_id,
                                error,
                                ..
                            } => {
                                if let Some(reply) =
                                    pending_bridge_requests.remove(&request_id)
                                {
                                    let _ = reply.send(Err(NetworkError::Transport(format!(
                                        "outbound bridge failure: {error}"
                                    ))));
                                }
                            }
                            request_response::Event::InboundFailure { error, .. } => {
                                warn!(error = %error, "inbound bridge failure");
                            }
                            request_response::Event::ResponseSent { .. } => {}
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}

/// Handle an incoming snapshot request by querying the local provider.
async fn handle_snapshot_request(
    provider: &dyn SnapshotProvider,
    request: SnapshotRequest,
) -> SnapshotResponse {
    match request {
        SnapshotRequest::ListSnapshots => {
            let manifests = provider.list_snapshots().await;
            SnapshotResponse::Manifests(manifests)
        }
        SnapshotRequest::GetManifest { height } => {
            let manifest = provider.get_manifest(height).await;
            SnapshotResponse::Manifest(manifest)
        }
        SnapshotRequest::GetChunk { height, index } => {
            let chunk = provider.get_chunk(height, index).await;
            SnapshotResponse::Chunk(chunk)
        }
    }
}

/// Handle an incoming light client request by querying the local provider.
async fn handle_light_request(
    provider: &dyn LightClientProvider,
    request: LightClientRequest,
) -> LightClientResponse {
    match request {
        LightClientRequest::GetHeader { height } => {
            let header = provider.get_header(height).await;
            LightClientResponse::Header(header)
        }
        LightClientRequest::GetHeaderRange { from, to } => {
            let headers = provider.get_header_range(from, to).await;
            LightClientResponse::Headers(headers)
        }
        LightClientRequest::GetStateProof { key, height } => {
            let proof = provider.get_state_proof(&key, height).await;
            LightClientResponse::StateProof(proof)
        }
    }
}

/// Handle an incoming bridge request by querying the local provider.
async fn handle_bridge_request(
    provider: &dyn BridgeProtocolProvider,
    request: BridgeRequest,
) -> BridgeResponse {
    match request {
        BridgeRequest::SubmitPacket {
            packet,
            proof,
            source_state_root,
        } => {
            let ack = provider
                .submit_packet(packet, proof, source_state_root)
                .await;
            BridgeResponse::Acknowledgement(ack)
        }
        BridgeRequest::GetPacketProof { packet_hash } => {
            let proof = provider.get_packet_proof(&packet_hash).await;
            BridgeResponse::PacketProof(proof)
        }
        BridgeRequest::GetAcknowledgement { packet_hash } => {
            match provider.get_acknowledgement(&packet_hash).await {
                Some(ack) => BridgeResponse::Acknowledgement(ack),
                None => {
                    BridgeResponse::Error(format!("no acknowledgement for packet {packet_hash}"))
                }
            }
        }
    }
}
