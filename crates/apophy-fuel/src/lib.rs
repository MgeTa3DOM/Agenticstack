//! # Apophy Fuel
//!
//! Sovereign E2E encrypted communication protocol.
//! Replaces WhatsApp/Slack/Teams with zero third-party dependencies.
//!
//! Features:
//! - End-to-end encryption (Double Ratchet / ChaCha20-Poly1305)
//! - Peer-to-peer message routing
//! - Group messaging (up to 10,000 members)
//! - Agent command channel
//! - Auto-destructing messages
//! - Zero central server requirement

use apophy_crypto::{
    CryptoError, EncryptedEnvelope, RatchetSession, SovereignIdentity,
    SymmetricCipher,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};

#[derive(Error, Debug)]
pub enum FuelError {
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Group error: {0}")]
    GroupError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, FuelError>;

/// Unique identifier for a peer node
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message types supported by Fuel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    File {
        filename: String,
        size: u64,
        mime_type: String,
    },
    AgentCommand {
        action: String,
        payload: serde_json::Value,
    },
    GroupInvite {
        group_id: String,
        group_name: String,
    },
    Acknowledgment {
        message_id: String,
    },
}

/// A decrypted Fuel message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelMessage {
    pub id: String,
    pub sender: PeerId,
    pub recipient: PeerId,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: u64,
    pub auto_destroy_secs: Option<u64>,
}

impl FuelMessage {
    pub fn text(sender: PeerId, recipient: PeerId, content: String) -> Self {
        Self {
            id: generate_message_id(),
            sender,
            recipient,
            content,
            message_type: MessageType::Text,
            timestamp: current_timestamp(),
            auto_destroy_secs: None,
        }
    }

    pub fn agent_command(
        sender: PeerId,
        recipient: PeerId,
        action: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: generate_message_id(),
            sender,
            recipient,
            content: String::new(),
            message_type: MessageType::AgentCommand { action, payload },
            timestamp: current_timestamp(),
            auto_destroy_secs: None,
        }
    }

    pub fn with_auto_destroy(mut self, secs: u64) -> Self {
        self.auto_destroy_secs = Some(secs);
        self
    }
}

/// Group chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelGroup {
    pub id: String,
    pub name: String,
    pub members: Vec<PeerId>,
    pub admins: Vec<PeerId>,
    pub created_at: u64,
    pub max_members: usize,
}

impl FuelGroup {
    pub fn new(name: String, creator: PeerId) -> Self {
        Self {
            id: generate_message_id(),
            name,
            members: vec![creator.clone()],
            admins: vec![creator],
            created_at: current_timestamp(),
            max_members: 10_000,
        }
    }

    pub fn add_member(&mut self, peer: PeerId) -> Result<()> {
        if self.members.len() >= self.max_members {
            return Err(FuelError::GroupError(format!(
                "Group full ({} members max)",
                self.max_members
            )));
        }
        if !self.members.contains(&peer) {
            self.members.push(peer);
        }
        Ok(())
    }

    pub fn remove_member(&mut self, peer: &PeerId) -> Result<()> {
        self.members.retain(|m| m != peer);
        Ok(())
    }

    pub fn is_admin(&self, peer: &PeerId) -> bool {
        self.admins.contains(peer)
    }
}

/// Fuel client - sovereign communication node
pub struct FuelClient {
    identity: SovereignIdentity,
    peer_id: PeerId,
    sessions: Arc<RwLock<HashMap<PeerId, SessionState>>>,
    groups: Arc<RwLock<HashMap<String, FuelGroup>>>,
    inbox: mpsc::Receiver<FuelMessage>,
    inbox_sender: mpsc::Sender<FuelMessage>,
    outbox: mpsc::Sender<EncryptedEnvelope>,
    outbox_receiver: Option<mpsc::Receiver<EncryptedEnvelope>>,
}

struct SessionState {
    ratchet: RatchetSession,
    peer_x25519_public: [u8; 32],
}

impl FuelClient {
    /// Create a new Fuel client with a fresh identity
    pub fn new() -> Self {
        let identity = SovereignIdentity::generate();
        let public_bytes = identity.x25519_public().as_bytes().to_owned();
        let peer_id = PeerId(hex::encode(&public_bytes[..16]));

        let (inbox_tx, inbox_rx) = mpsc::channel(10_000);
        let (outbox_tx, outbox_rx) = mpsc::channel(10_000);

        tracing::info!("Fuel client created: {}", peer_id);

        Self {
            identity,
            peer_id,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            inbox: inbox_rx,
            inbox_sender: inbox_tx,
            outbox: outbox_tx,
            outbox_receiver: Some(outbox_rx),
        }
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    /// Establish an encrypted session with a peer.
    /// `is_initiator` determines the ratchet role: the initiator's send
    /// chain matches the responder's recv chain, enabling E2E decryption.
    pub async fn establish_session(
        &self,
        peer_id: PeerId,
        peer_x25519_public: [u8; 32],
        is_initiator: bool,
    ) -> Result<()> {
        let peer_public = x25519_dalek::PublicKey::from(peer_x25519_public);
        let shared_secret = self.identity.dh_exchange(&peer_public);
        let ratchet = if is_initiator {
            RatchetSession::new_initiator(&shared_secret)
        } else {
            RatchetSession::new_responder(&shared_secret)
        };

        let state = SessionState {
            ratchet,
            peer_x25519_public,
        };

        self.sessions.write().await.insert(peer_id.clone(), state);
        tracing::info!("Session established with {}", peer_id);
        Ok(())
    }

    /// Send an encrypted message to a peer
    pub async fn send_message(&self, message: FuelMessage) -> Result<EncryptedEnvelope> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&message.recipient)
            .ok_or_else(|| FuelError::PeerNotFound(message.recipient.to_string()))?;

        let msg_key = session.ratchet.next_send_key();
        let cipher = SymmetricCipher::new(&msg_key);

        let plaintext =
            serde_json::to_vec(&message).map_err(|e| FuelError::SerializationError(e.to_string()))?;

        let (ciphertext, nonce) = cipher.encrypt(&plaintext)?;

        let envelope = EncryptedEnvelope {
            ciphertext,
            nonce,
            sender_public_key: *self.identity.x25519_public().as_bytes(),
            epoch: session.ratchet.epoch(),
        };

        self.outbox
            .send(envelope.clone())
            .await
            .map_err(|_| FuelError::ChannelClosed)?;

        tracing::info!("Message sent to {}", message.recipient);
        Ok(envelope)
    }

    /// Decrypt a received envelope
    pub async fn decrypt_envelope(
        &self,
        sender: &PeerId,
        envelope: &EncryptedEnvelope,
    ) -> Result<FuelMessage> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(sender)
            .ok_or_else(|| FuelError::PeerNotFound(sender.to_string()))?;

        let msg_key = session.ratchet.next_recv_key();
        let cipher = SymmetricCipher::new(&msg_key);

        let plaintext = cipher.decrypt(&envelope.ciphertext, &envelope.nonce)?;
        let message: FuelMessage =
            serde_json::from_slice(&plaintext).map_err(|e| FuelError::SerializationError(e.to_string()))?;

        Ok(message)
    }

    /// Create a new group
    pub async fn create_group(&self, name: String) -> Result<FuelGroup> {
        let group = FuelGroup::new(name, self.peer_id.clone());
        let group_id = group.id.clone();
        self.groups.write().await.insert(group_id, group.clone());
        tracing::info!("Group created: {}", group.name);
        Ok(group)
    }

    /// Take ownership of the outbox receiver (for network transport layer)
    pub fn take_outbox(&mut self) -> Option<mpsc::Receiver<EncryptedEnvelope>> {
        self.outbox_receiver.take()
    }

    /// Get the inbox sender (for network transport layer to deliver messages)
    pub fn inbox_sender(&self) -> mpsc::Sender<FuelMessage> {
        self.inbox_sender.clone()
    }
}

// hex encoding (no extra dependency)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

fn generate_message_id() -> String {
    let bytes: [u8; 16] = rand::random();
    hex::encode(&bytes)
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fuel_client_creation() {
        let client = FuelClient::new();
        assert!(!client.peer_id().0.is_empty());
    }

    #[tokio::test]
    async fn test_session_establishment() {
        let alice = FuelClient::new();
        let bob = FuelClient::new();

        // Exchange public keys
        let alice_pub = *alice.identity.x25519_public().as_bytes();
        let bob_pub = *bob.identity.x25519_public().as_bytes();

        alice
            .establish_session(bob.peer_id().clone(), bob_pub, true)
            .await
            .unwrap();
        bob.establish_session(alice.peer_id().clone(), alice_pub, false)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_send_receive_message() {
        let alice = FuelClient::new();
        let bob = FuelClient::new();

        let alice_pub = *alice.identity.x25519_public().as_bytes();
        let bob_pub = *bob.identity.x25519_public().as_bytes();

        alice
            .establish_session(bob.peer_id().clone(), bob_pub, true)
            .await
            .unwrap();
        bob.establish_session(alice.peer_id().clone(), alice_pub, false)
            .await
            .unwrap();

        let msg = FuelMessage::text(
            alice.peer_id().clone(),
            bob.peer_id().clone(),
            "Souveraineté totale!".to_string(),
        );

        let envelope = alice.send_message(msg).await.unwrap();
        let decrypted = bob
            .decrypt_envelope(alice.peer_id(), &envelope)
            .await
            .unwrap();

        assert_eq!(decrypted.content, "Souveraineté totale!");
    }

    #[tokio::test]
    async fn test_group_creation() {
        let client = FuelClient::new();
        let group = client.create_group("Team Sovereign".to_string()).await.unwrap();
        assert_eq!(group.name, "Team Sovereign");
        assert_eq!(group.members.len(), 1);
        assert!(group.is_admin(client.peer_id()));
    }

    #[tokio::test]
    async fn test_auto_destroy_message() {
        let msg = FuelMessage::text(
            PeerId("alice".into()),
            PeerId("bob".into()),
            "Secret".into(),
        )
        .with_auto_destroy(60);

        assert_eq!(msg.auto_destroy_secs, Some(60));
    }
}
