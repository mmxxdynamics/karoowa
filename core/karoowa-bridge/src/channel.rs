//! Bridge channel state machine.
//!
//! A channel is a bidirectional pipe between two chains. Channels go
//! through a four-step handshake before they can carry packets:
//!
//! `Init` → `TryOpen` → `Open` → (optional `Closed`)

use serde::{Deserialize, Serialize};

/// State of a bridge channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelState {
    /// Channel created on the source side; waiting for the counterparty.
    Init,
    /// Counterparty has acknowledged; waiting for source confirmation.
    TryOpen,
    /// Channel is open and can carry packets.
    Open,
    /// Channel is closed; no new packets accepted.
    Closed,
}

impl std::fmt::Display for ChannelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelState::Init => write!(f, "Init"),
            ChannelState::TryOpen => write!(f, "TryOpen"),
            ChannelState::Open => write!(f, "Open"),
            ChannelState::Closed => write!(f, "Closed"),
        }
    }
}

/// A bridge channel between two chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeChannel {
    /// Channel identifier (e.g. "channel-0").
    pub id: String,
    /// Source chain ID.
    pub source_chain_id: String,
    /// Destination chain ID.
    pub dest_chain_id: String,
    /// Current state.
    pub state: ChannelState,
    /// Next outbound packet sequence number.
    pub next_sequence_send: u64,
    /// Next inbound packet sequence number expected.
    pub next_sequence_recv: u64,
}

impl BridgeChannel {
    /// Create a new channel in `Init` state.
    pub fn new(
        id: impl Into<String>,
        source_chain_id: impl Into<String>,
        dest_chain_id: impl Into<String>,
    ) -> Self {
        BridgeChannel {
            id: id.into(),
            source_chain_id: source_chain_id.into(),
            dest_chain_id: dest_chain_id.into(),
            state: ChannelState::Init,
            next_sequence_send: 1,
            next_sequence_recv: 1,
        }
    }

    /// Transition to `TryOpen`. Only valid from `Init`.
    pub fn try_open(&mut self) -> Result<(), String> {
        if self.state != ChannelState::Init {
            return Err(format!("cannot transition to TryOpen from {}", self.state));
        }
        self.state = ChannelState::TryOpen;
        Ok(())
    }

    /// Transition to `Open`. Valid from `Init` or `TryOpen`.
    pub fn open(&mut self) -> Result<(), String> {
        match self.state {
            ChannelState::Init | ChannelState::TryOpen => {
                self.state = ChannelState::Open;
                Ok(())
            }
            other => Err(format!("cannot open channel from state {other}")),
        }
    }

    /// Close the channel. Valid from `Open`.
    pub fn close(&mut self) -> Result<(), String> {
        if self.state != ChannelState::Open {
            return Err(format!("cannot close channel from state {}", self.state));
        }
        self.state = ChannelState::Closed;
        Ok(())
    }

    /// Whether the channel can currently carry packets.
    pub fn is_open(&self) -> bool {
        self.state == ChannelState::Open
    }

    /// Get and increment the next outbound sequence.
    pub fn next_send_sequence(&mut self) -> u64 {
        let seq = self.next_sequence_send;
        self.next_sequence_send += 1;
        seq
    }

    /// Verify and increment the next inbound sequence.
    /// Returns an error if `incoming_seq` doesn't match the expected value.
    pub fn accept_recv_sequence(&mut self, incoming_seq: u64) -> Result<(), String> {
        if incoming_seq != self.next_sequence_recv {
            return Err(format!(
                "out-of-order packet: expected seq {}, got {}",
                self.next_sequence_recv, incoming_seq
            ));
        }
        self.next_sequence_recv += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_channel() -> BridgeChannel {
        BridgeChannel::new("channel-0", "karoowa-a", "karoowa-b")
    }

    #[test]
    fn new_channel_starts_in_init() {
        let ch = new_channel();
        assert_eq!(ch.state, ChannelState::Init);
        assert!(!ch.is_open());
    }

    #[test]
    fn handshake_to_open() {
        let mut ch = new_channel();
        ch.try_open().unwrap();
        assert_eq!(ch.state, ChannelState::TryOpen);
        ch.open().unwrap();
        assert_eq!(ch.state, ChannelState::Open);
        assert!(ch.is_open());
    }

    #[test]
    fn cannot_skip_to_open_from_random() {
        let mut ch = new_channel();
        ch.open().unwrap(); // valid from Init
                            // can't open again from Open
        assert!(ch.open().is_err());
    }

    #[test]
    fn cannot_close_from_init() {
        let mut ch = new_channel();
        assert!(ch.close().is_err());
    }

    #[test]
    fn open_then_close() {
        let mut ch = new_channel();
        ch.open().unwrap();
        ch.close().unwrap();
        assert_eq!(ch.state, ChannelState::Closed);
        assert!(!ch.is_open());
    }

    #[test]
    fn send_sequence_increments() {
        let mut ch = new_channel();
        assert_eq!(ch.next_send_sequence(), 1);
        assert_eq!(ch.next_send_sequence(), 2);
        assert_eq!(ch.next_send_sequence(), 3);
    }

    #[test]
    fn recv_sequence_must_be_in_order() {
        let mut ch = new_channel();
        ch.accept_recv_sequence(1).unwrap();
        ch.accept_recv_sequence(2).unwrap();
        // out-of-order rejected
        assert!(ch.accept_recv_sequence(5).is_err());
        // duplicate rejected
        assert!(ch.accept_recv_sequence(2).is_err());
        // next valid
        ch.accept_recv_sequence(3).unwrap();
    }
}
