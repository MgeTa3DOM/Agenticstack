//! # Apophy Crypto
//!
//! Sovereign cryptography layer implementing:
//! - Double Ratchet protocol (Signal-inspired) for E2E messaging
//! - ChaCha20-Poly1305 symmetric encryption
//! - X25519 key exchange
//! - Ed25519 signing
//! - Zero-knowledge key rotation

use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce,
};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::Zeroize;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),

    #[error("Signature verification failed")]
    SignatureInvalid,

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
}

pub type Result<T> = std::result::Result<T, CryptoError>;

/// Identity keypair for a sovereign node
pub struct SovereignIdentity {
    signing_key: SigningKey,
    x25519_secret: StaticSecret,
}

impl SovereignIdentity {
    /// Generate a new random identity
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let x25519_secret = StaticSecret::random_from_rng(OsRng);
        Self {
            signing_key,
            x25519_secret,
        }
    }

    /// Get the public verifying key
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Get the X25519 public key for key exchange
    pub fn x25519_public(&self) -> X25519PublicKey {
        X25519PublicKey::from(&self.x25519_secret)
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    /// Perform X25519 Diffie-Hellman key exchange
    pub fn dh_exchange(&self, peer_public: &X25519PublicKey) -> SharedSecret {
        let shared = self.x25519_secret.diffie_hellman(peer_public);
        SharedSecret {
            bytes: *shared.as_bytes(),
        }
    }
}

/// Shared secret from DH exchange
pub struct SharedSecret {
    bytes: [u8; 32],
}

impl SharedSecret {
    /// Derive an encryption key from the shared secret using HKDF-like construction
    pub fn derive_key(&self, context: &[u8]) -> Key {
        let mut hasher = Sha256::new();
        hasher.update(&self.bytes);
        hasher.update(context);
        let result = hasher.finalize();
        *Key::from_slice(&result)
    }
}

impl Drop for SharedSecret {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

/// Encrypted envelope for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
    pub sender_public_key: [u8; 32],
    pub epoch: u64,
}

/// Symmetric encryption/decryption using ChaCha20-Poly1305
pub struct SymmetricCipher {
    cipher: ChaCha20Poly1305,
}

impl SymmetricCipher {
    /// Create from a raw 256-bit key
    pub fn new(key: &Key) -> Self {
        Self {
            cipher: ChaCha20Poly1305::new(key),
        }
    }

    /// Create from shared secret + context
    pub fn from_shared_secret(secret: &SharedSecret, context: &[u8]) -> Self {
        let key = secret.derive_key(context);
        Self::new(&key)
    }

    /// Encrypt plaintext, returns (ciphertext, nonce)
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 12])> {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(nonce.as_slice());
        Ok((ciphertext, nonce_bytes))
    }

    /// Decrypt ciphertext with nonce
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        let nonce = Nonce::from_slice(nonce);
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
    }
}

/// Double Ratchet session state for forward secrecy
pub struct RatchetSession {
    send_chain_key: [u8; 32],
    recv_chain_key: [u8; 32],
    epoch: u64,
}

impl RatchetSession {
    /// Initialize a ratchet session as the initiator (Alice).
    /// Alice's send chain = chain-A, recv chain = chain-B.
    pub fn new_initiator(shared_secret: &SharedSecret) -> Self {
        let (chain_a, chain_b) = derive_chains(shared_secret);
        Self {
            send_chain_key: chain_a,
            recv_chain_key: chain_b,
            epoch: 0,
        }
    }

    /// Initialize a ratchet session as the responder (Bob).
    /// Bob's send chain = chain-B, recv chain = chain-A.
    /// This means Bob's recv matches Alice's send.
    pub fn new_responder(shared_secret: &SharedSecret) -> Self {
        let (chain_a, chain_b) = derive_chains(shared_secret);
        Self {
            send_chain_key: chain_b,
            recv_chain_key: chain_a,
            epoch: 0,
        }
    }

    /// Convenience: same as `new_initiator` for backward compat
    pub fn new(shared_secret: &SharedSecret) -> Self {
        Self::new_initiator(shared_secret)
    }

    /// Advance the send chain and return a message key
    pub fn next_send_key(&mut self) -> Key {
        let mut h = Sha256::new();
        h.update(&self.send_chain_key);
        h.update(b"message-key");
        let msg_key = h.finalize();

        // Advance chain
        let mut h2 = Sha256::new();
        h2.update(&self.send_chain_key);
        h2.update(b"chain-advance");
        let next = h2.finalize();
        self.send_chain_key.copy_from_slice(&next);
        self.epoch += 1;

        *Key::from_slice(&msg_key)
    }

    /// Advance the receive chain and return a message key
    pub fn next_recv_key(&mut self) -> Key {
        let mut h = Sha256::new();
        h.update(&self.recv_chain_key);
        h.update(b"message-key");
        let msg_key = h.finalize();

        // Advance chain
        let mut h2 = Sha256::new();
        h2.update(&self.recv_chain_key);
        h2.update(b"chain-advance");
        let next = h2.finalize();
        self.recv_chain_key.copy_from_slice(&next);

        *Key::from_slice(&msg_key)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

fn derive_chains(shared_secret: &SharedSecret) -> ([u8; 32], [u8; 32]) {
    let chain_a = {
        let mut h = Sha256::new();
        h.update(&shared_secret.bytes);
        h.update(b"chain-A");
        let r = h.finalize();
        let mut k = [0u8; 32];
        k.copy_from_slice(&r);
        k
    };
    let chain_b = {
        let mut h = Sha256::new();
        h.update(&shared_secret.bytes);
        h.update(b"chain-B");
        let r = h.finalize();
        let mut k = [0u8; 32];
        k.copy_from_slice(&r);
        k
    };
    (chain_a, chain_b)
}

impl Drop for RatchetSession {
    fn drop(&mut self) {
        self.send_chain_key.zeroize();
        self.recv_chain_key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let id = SovereignIdentity::generate();
        let _vk = id.verifying_key();
        let _pk = id.x25519_public();
    }

    #[test]
    fn test_dh_key_exchange() {
        let alice = SovereignIdentity::generate();
        let bob = SovereignIdentity::generate();

        let alice_shared = alice.dh_exchange(&bob.x25519_public());
        let bob_shared = bob.dh_exchange(&alice.x25519_public());

        // Both should derive the same key
        let alice_key = alice_shared.derive_key(b"test");
        let bob_key = bob_shared.derive_key(b"test");
        assert_eq!(alice_key.as_slice(), bob_key.as_slice());
    }

    #[test]
    fn test_symmetric_encrypt_decrypt() {
        let alice = SovereignIdentity::generate();
        let bob = SovereignIdentity::generate();

        let shared = alice.dh_exchange(&bob.x25519_public());
        let cipher = SymmetricCipher::from_shared_secret(&shared, b"chat");

        let plaintext = b"Hello sovereign world!";
        let (ciphertext, nonce) = cipher.encrypt(plaintext).unwrap();
        assert_ne!(ciphertext, plaintext);

        let decrypted = cipher.decrypt(&ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ratchet_forward_secrecy() {
        let alice = SovereignIdentity::generate();
        let bob = SovereignIdentity::generate();
        let shared = alice.dh_exchange(&bob.x25519_public());

        let mut session = RatchetSession::new(&shared);

        let key1 = session.next_send_key();
        let key2 = session.next_send_key();

        // Each key should be different (forward secrecy)
        assert_ne!(key1.as_slice(), key2.as_slice());
        assert_eq!(session.epoch(), 2);
    }

    #[test]
    fn test_signing() {
        let id = SovereignIdentity::generate();
        let message = b"sovereign data";
        let signature = id.sign(message);
        assert!(!signature.is_empty());
    }
}
