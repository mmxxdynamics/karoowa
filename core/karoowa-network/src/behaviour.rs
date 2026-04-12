//! Combined network behaviour: Gossipsub + Kademlia + Identify.
//!
//! The `KaroowaBehaviour` struct composes the three sub-behaviours using
//! libp2p's derive macro. The Identify protocol is included so that peers
//! exchange their listen addresses, which Kademlia needs for routing.

use libp2p::gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId, ValidationMode};
use libp2p::identify;
use libp2p::identity::Keypair;
use libp2p::kad;
use libp2p::kad::store::MemoryStore;
use libp2p::request_response;
use libp2p::swarm::NetworkBehaviour;
use libp2p::StreamProtocol;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use crate::bridge::{self, BridgeCodec};
use crate::light_client::{self, LightClientCodec};
use crate::state_sync::{self, StateSyncCodec};

/// Gossipsub topic names.
pub const TOPIC_BLOCKS: &str = "/karoowa/blocks/1";
pub const TOPIC_TRANSACTIONS: &str = "/karoowa/transactions/1";

/// The composed network behaviour.
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "BehaviourEvent")]
pub struct KaroowaBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub identify: identify::Behaviour,
    pub state_sync: request_response::Behaviour<StateSyncCodec>,
    pub light_client: request_response::Behaviour<LightClientCodec>,
    pub bridge: request_response::Behaviour<BridgeCodec>,
}

/// Events emitted by the composed behaviour.
#[derive(Debug)]
pub enum BehaviourEvent {
    Gossipsub(gossipsub::Event),
    Kademlia(kad::Event),
    Identify(Box<identify::Event>),
    StateSync(Box<state_sync::StateSyncEvent>),
    LightClient(Box<light_client::LightClientEvent>),
    Bridge(Box<bridge::BridgeProtocolEvent>),
}

impl From<gossipsub::Event> for BehaviourEvent {
    fn from(e: gossipsub::Event) -> Self {
        BehaviourEvent::Gossipsub(e)
    }
}

impl From<kad::Event> for BehaviourEvent {
    fn from(e: kad::Event) -> Self {
        BehaviourEvent::Kademlia(e)
    }
}

impl From<identify::Event> for BehaviourEvent {
    fn from(e: identify::Event) -> Self {
        BehaviourEvent::Identify(Box::new(e))
    }
}

impl From<state_sync::StateSyncEvent> for BehaviourEvent {
    fn from(e: state_sync::StateSyncEvent) -> Self {
        BehaviourEvent::StateSync(Box::new(e))
    }
}

impl From<light_client::LightClientEvent> for BehaviourEvent {
    fn from(e: light_client::LightClientEvent) -> Self {
        BehaviourEvent::LightClient(Box::new(e))
    }
}

impl From<bridge::BridgeProtocolEvent> for BehaviourEvent {
    fn from(e: bridge::BridgeProtocolEvent) -> Self {
        BehaviourEvent::Bridge(Box::new(e))
    }
}

/// Build the composed Karoowa network behaviour.
pub fn build_behaviour(
    keypair: &Keypair,
    heartbeat: Duration,
    mesh_n: usize,
    mesh_n_low: usize,
    mesh_n_high: usize,
) -> KaroowaBehaviour {
    // -- Gossipsub --
    let message_id_fn = |message: &gossipsub::Message| {
        let mut hasher = DefaultHasher::new();
        message.data.hash(&mut hasher);
        message.topic.hash(&mut hasher);
        MessageId::from(hasher.finish().to_be_bytes().to_vec())
    };

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(heartbeat)
        .validation_mode(ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .mesh_n(mesh_n)
        .mesh_n_low(mesh_n_low)
        .mesh_n_high(mesh_n_high)
        .build()
        .expect("valid gossipsub config");

    let mut gossipsub = gossipsub::Behaviour::new(
        MessageAuthenticity::Signed(keypair.clone()),
        gossipsub_config,
    )
    .expect("valid gossipsub behaviour");

    // Subscribe to both topics.
    let blocks_topic = IdentTopic::new(TOPIC_BLOCKS);
    let txs_topic = IdentTopic::new(TOPIC_TRANSACTIONS);
    gossipsub
        .subscribe(&blocks_topic)
        .expect("subscribe blocks");
    gossipsub.subscribe(&txs_topic).expect("subscribe txs");

    // -- Kademlia --
    let peer_id = keypair.public().to_peer_id();
    let store = MemoryStore::new(peer_id);
    let mut kademlia = kad::Behaviour::new(peer_id, store);
    kademlia.set_mode(Some(kad::Mode::Server));

    // Use a custom protocol name so Karoowa nodes don't pollute other
    // libp2p networks' DHTs.
    let kad_protocol =
        StreamProtocol::try_from_owned("/karoowa/kad/1".to_string()).expect("valid protocol name");
    let mut kad_config = kad::Config::new(kad_protocol);
    kad_config.set_record_ttl(Some(Duration::from_secs(3600)));
    // Apply config — rebuild kademlia with custom config
    kademlia = kad::Behaviour::with_config(peer_id, MemoryStore::new(peer_id), kad_config);
    kademlia.set_mode(Some(kad::Mode::Server));

    // -- Identify --
    let identify = identify::Behaviour::new(identify::Config::new(
        "/karoowa/id/1".to_string(),
        keypair.public(),
    ));

    // -- State sync request-response --
    let state_sync = state_sync::build_behaviour();

    // -- Light client request-response --
    let light_client = light_client::build_behaviour();

    // -- Bridge request-response --
    let bridge = bridge::build_behaviour();

    KaroowaBehaviour {
        gossipsub,
        kademlia,
        identify,
        state_sync,
        light_client,
        bridge,
    }
}
