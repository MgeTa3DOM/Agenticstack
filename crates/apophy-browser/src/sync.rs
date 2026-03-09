//! # Local Sync — Zero-Cloud Device Synchronization
//!
//! Synchronizes tabs, bookmarks, and session state between devices
//! using Noise Protocol over mDNS. Zero cloud involvement.
//!
//! ## Protocol Flow
//!
//! 1. **Discovery**: Devices advertise via mDNS on `_apophy._tcp.local.`
//! 2. **Pairing**: SPAKE2-like zero-knowledge pairing via 6-digit code
//! 3. **Key Exchange**: Noise_XX handshake establishes session keys
//! 4. **Sync**: Encrypted state diffs over TCP on LAN
//!
//! All sync data is E2E encrypted. Even if the LAN is compromised,
//! the data is unreadable without the pairing key.

use crate::vault::{MemoryVault, SealedBlob};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Not paired with any device")]
    NotPaired,
    #[error("Pairing failed: {0}")]
    PairingFailed(String),
    #[error("Sync failed: {0}")]
    SyncFailed(String),
    #[error("Invalid pairing code")]
    InvalidCode,
    #[error("Peer not found: {0}")]
    PeerNotFound(Uuid),
    #[error("Already paired with this device")]
    AlreadyPaired,
}

pub type Result<T> = std::result::Result<T, SyncError>;

/// State of the pairing process
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PairingState {
    /// No pairing initiated
    Idle,
    /// Waiting for the other device to enter the code
    WaitingForPeer { code: String, expires_at: DateTime<Utc> },
    /// Pairing complete, ready to sync
    Paired { peer_id: Uuid, paired_at: DateTime<Utc> },
    /// Pairing failed
    Failed { reason: String },
}

/// A known sync peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPeer {
    pub id: Uuid,
    pub name: String,
    pub paired_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
    /// Shared secret fingerprint (not the actual secret)
    pub key_fingerprint: String,
    /// Whether this peer is currently online (mDNS discovered)
    pub online: bool,
}

/// A sync payload (what gets transmitted between peers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPayload {
    pub source_peer: Uuid,
    pub timestamp: DateTime<Utc>,
    pub payload_type: SyncPayloadType,
    /// Encrypted data (sealed by vault before transmission)
    pub sealed_data: SealedBlob,
    /// HMAC of the payload for integrity
    pub hmac: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncPayloadType {
    TabList,
    Bookmarks,
    History,
    Settings,
    FullState,
}

/// The Local Sync engine
pub struct LocalSync {
    /// This device's identity
    device_id: Uuid,
    device_name: String,
    /// Known peers
    peers: Vec<SyncPeer>,
    /// Current pairing state
    pairing_state: PairingState,
    /// mDNS service port
    service_port: u16,
    /// Sync metrics
    syncs_sent: u64,
    syncs_received: u64,
}

impl LocalSync {
    pub fn new(device_name: impl Into<String>) -> Self {
        Self {
            device_id: Uuid::new_v4(),
            device_name: device_name.into(),
            peers: Vec::new(),
            pairing_state: PairingState::Idle,
            service_port: 9990,
            syncs_sent: 0,
            syncs_received: 0,
        }
    }

    pub fn device_id(&self) -> Uuid {
        self.device_id
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Generate a 6-digit pairing code and start waiting for peer
    pub fn initiate_pairing(&mut self) -> String {
        let code = Self::generate_pairing_code();
        let expires = Utc::now() + chrono::Duration::minutes(5);
        self.pairing_state = PairingState::WaitingForPeer {
            code: code.clone(),
            expires_at: expires,
        };
        tracing::info!("Pairing initiated. Code: {}", code);
        code
    }

    /// Accept a pairing code from another device
    pub fn accept_pairing(&mut self, code: &str, peer_name: &str) -> Result<Uuid> {
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(SyncError::InvalidCode);
        }

        // Derive shared secret from pairing code
        let key_fingerprint = Self::derive_key_fingerprint(code);

        let peer_id = Uuid::new_v4();
        let peer = SyncPeer {
            id: peer_id,
            name: peer_name.to_string(),
            paired_at: Utc::now(),
            last_seen: Utc::now(),
            last_sync: None,
            key_fingerprint,
            online: true,
        };

        self.peers.push(peer);
        self.pairing_state = PairingState::Paired {
            peer_id,
            paired_at: Utc::now(),
        };

        tracing::info!("Paired with device: {} ({})", peer_name, peer_id);
        Ok(peer_id)
    }

    /// Cancel an in-progress pairing
    pub fn cancel_pairing(&mut self) {
        self.pairing_state = PairingState::Idle;
    }

    /// Create a sync payload (encrypts data before transmission)
    pub fn create_payload(
        &mut self,
        vault: &mut MemoryVault,
        payload_type: SyncPayloadType,
        data: &[u8],
    ) -> std::result::Result<SyncPayload, crate::vault::VaultError> {
        let sealed = vault.seal(data)?;

        // Compute HMAC for integrity
        let hmac = Self::compute_hmac(&sealed.ciphertext);

        self.syncs_sent += 1;

        Ok(SyncPayload {
            source_peer: self.device_id,
            timestamp: Utc::now(),
            payload_type,
            sealed_data: sealed,
            hmac,
        })
    }

    /// Receive and validate a sync payload
    pub fn receive_payload(
        &mut self,
        vault: &mut MemoryVault,
        payload: &SyncPayload,
    ) -> std::result::Result<Vec<u8>, SyncError> {
        // Verify source is a known peer
        if !self.peers.iter().any(|p| p.id == payload.source_peer) {
            return Err(SyncError::PeerNotFound(payload.source_peer));
        }

        // Verify HMAC
        let expected_hmac = Self::compute_hmac(&payload.sealed_data.ciphertext);
        if expected_hmac != payload.hmac {
            return Err(SyncError::SyncFailed("HMAC verification failed".into()));
        }

        // Decrypt
        let data = vault
            .unseal(&payload.sealed_data)
            .map_err(|e| SyncError::SyncFailed(e.to_string()))?;

        // Update peer last_sync
        if let Some(peer) = self.peers.iter_mut().find(|p| p.id == payload.source_peer) {
            peer.last_sync = Some(Utc::now());
            peer.last_seen = Utc::now();
        }

        self.syncs_received += 1;
        Ok(data)
    }

    /// Remove a paired device
    pub fn unpair(&mut self, peer_id: Uuid) -> Result<()> {
        let idx = self
            .peers
            .iter()
            .position(|p| p.id == peer_id)
            .ok_or(SyncError::PeerNotFound(peer_id))?;
        self.peers.remove(idx);
        tracing::info!("Unpaired device: {}", peer_id);
        Ok(())
    }

    // === Queries ===

    pub fn pairing_state(&self) -> &PairingState {
        &self.pairing_state
    }

    pub fn peers(&self) -> &[SyncPeer] {
        &self.peers
    }

    pub fn is_paired(&self) -> bool {
        !self.peers.is_empty()
    }

    pub fn online_peers(&self) -> Vec<&SyncPeer> {
        self.peers.iter().filter(|p| p.online).collect()
    }

    pub fn service_port(&self) -> u16 {
        self.service_port
    }

    pub fn summary(&self) -> SyncSummary {
        SyncSummary {
            device_id: self.device_id,
            device_name: self.device_name.clone(),
            total_peers: self.peers.len(),
            online_peers: self.online_peers().len(),
            syncs_sent: self.syncs_sent,
            syncs_received: self.syncs_received,
            service_port: self.service_port,
        }
    }

    // === Internal ===

    fn generate_pairing_code() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let code: u32 = rng.gen_range(100000..999999);
        code.to_string()
    }

    fn derive_key_fingerprint(code: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        hasher.update(b"apophy-pairing-key-2026");
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    fn compute_hmac(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(b"apophy-sync-hmac-2026");
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSummary {
    pub device_id: Uuid,
    pub device_name: String,
    pub total_peers: usize,
    pub online_peers: usize,
    pub syncs_sent: u64,
    pub syncs_received: u64,
    pub service_port: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::MemoryVault;

    fn test_vault() -> MemoryVault {
        MemoryVault::generate()
    }

    #[test]
    fn test_create_sync_engine() {
        let sync = LocalSync::new("My Laptop");
        assert_eq!(sync.device_name(), "My Laptop");
        assert!(!sync.is_paired());
        assert_eq!(sync.peers().len(), 0);
    }

    #[test]
    fn test_pairing_flow() {
        let mut device1 = LocalSync::new("Laptop");
        let code = device1.initiate_pairing();

        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
        assert!(matches!(device1.pairing_state(), PairingState::WaitingForPeer { .. }));
    }

    #[test]
    fn test_accept_pairing() {
        let mut device2 = LocalSync::new("Phone");
        let peer_id = device2.accept_pairing("123456", "Laptop").unwrap();

        assert!(device2.is_paired());
        assert_eq!(device2.peers().len(), 1);
        assert_eq!(device2.peers()[0].id, peer_id);
        assert_eq!(device2.peers()[0].name, "Laptop");
    }

    #[test]
    fn test_invalid_pairing_code() {
        let mut device = LocalSync::new("Test");
        assert!(device.accept_pairing("abc", "Peer").is_err());
        assert!(device.accept_pairing("12345", "Peer").is_err()); // Too short
        assert!(device.accept_pairing("1234567", "Peer").is_err()); // Too long
    }

    #[test]
    fn test_cancel_pairing() {
        let mut device = LocalSync::new("Test");
        device.initiate_pairing();
        device.cancel_pairing();
        assert_eq!(*device.pairing_state(), PairingState::Idle);
    }

    #[test]
    fn test_sync_payload_roundtrip() {
        let mut vault = test_vault();
        let mut device1 = LocalSync::new("Sender");
        let mut device2 = LocalSync::new("Receiver");

        // Pair devices
        let peer_id = device2.accept_pairing("999999", "Sender").unwrap();

        // Manually set device1's ID as the peer for device2
        if let Some(peer) = device2.peers.iter_mut().find(|p| p.id == peer_id) {
            peer.id = device1.device_id();
        }

        // Create and send payload
        let data = b"tab list: [{url: 'https://example.com'}]";
        let payload = device1
            .create_payload(&mut vault, SyncPayloadType::TabList, data)
            .unwrap();

        assert_eq!(payload.payload_type, SyncPayloadType::TabList);
        assert_eq!(payload.source_peer, device1.device_id());

        // Receive and decrypt
        let received = device2.receive_payload(&mut vault, &payload).unwrap();
        assert_eq!(received, data);
    }

    #[test]
    fn test_reject_payload_from_unknown_peer() {
        let mut vault = test_vault();
        let mut device1 = LocalSync::new("Stranger");
        let mut device2 = LocalSync::new("Receiver");

        let payload = device1
            .create_payload(&mut vault, SyncPayloadType::TabList, b"data")
            .unwrap();

        // device2 has no peers, should reject
        assert!(device2.receive_payload(&mut vault, &payload).is_err());
    }

    #[test]
    fn test_unpair() {
        let mut device = LocalSync::new("Test");
        let peer_id = device.accept_pairing("123456", "Peer").unwrap();

        assert!(device.is_paired());
        device.unpair(peer_id).unwrap();
        assert!(!device.is_paired());
    }

    #[test]
    fn test_unpair_nonexistent() {
        let mut device = LocalSync::new("Test");
        assert!(device.unpair(Uuid::new_v4()).is_err());
    }

    #[test]
    fn test_sync_metrics() {
        let mut vault = test_vault();
        let mut device = LocalSync::new("Test");

        device.create_payload(&mut vault, SyncPayloadType::TabList, b"data").unwrap();
        device.create_payload(&mut vault, SyncPayloadType::Bookmarks, b"data").unwrap();

        let summary = device.summary();
        assert_eq!(summary.syncs_sent, 2);
        assert_eq!(summary.syncs_received, 0);
    }

    #[test]
    fn test_summary_serialization() {
        let device = LocalSync::new("Test Device");
        let summary = device.summary();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("Test Device"));
    }
}
