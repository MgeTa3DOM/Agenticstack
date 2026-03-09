//! # Memory Vault — ChaCha20-Poly1305 Encrypted RAM
//!
//! The core principle: **encrypt BEFORE storing in memory**.
//! The application never holds plaintext at rest.
//!
//! Every piece of data that persists beyond immediate use
//! (tab DOM, history, bookmarks, session state) passes through
//! the vault. The vault is the only component that holds the key.

use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: corrupted or tampered data")]
    DecryptionFailed,
    #[error("Key derivation failed")]
    KeyDerivationFailed,
    #[error("Vault is sealed")]
    Sealed,
    #[error("Integrity check failed: expected {expected}, got {actual}")]
    IntegrityFailed { expected: String, actual: String },
}

pub type Result<T> = std::result::Result<T, VaultError>;

/// A sealed (encrypted) blob with its nonce and integrity hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedBlob {
    /// The encrypted ciphertext
    pub ciphertext: Vec<u8>,
    /// 12-byte nonce used for encryption (unique per seal operation)
    pub nonce: [u8; 12],
    /// SHA-256 hash of plaintext (for integrity verification after unseal)
    pub integrity_hash: String,
    /// Timestamp of sealing
    pub sealed_at: chrono::DateTime<chrono::Utc>,
    /// Size of original plaintext (for metrics)
    pub original_size: usize,
}

/// The Memory Vault — all data passes through here
///
/// Holds a 256-bit master key (zeroized on drop) and provides
/// seal/unseal operations with per-operation random nonces.
pub struct MemoryVault {
    cipher: ChaCha20Poly1305,
    /// Master key (zeroized on drop for security)
    #[allow(dead_code)]
    master_key: MasterKey,
    /// Whether the vault is currently unlocked
    unlocked: bool,
    /// Total bytes sealed (for metrics)
    bytes_sealed: u64,
    /// Total bytes unsealed (for metrics)
    bytes_unsealed: u64,
    /// Total seal operations
    seal_count: u64,
}

/// A master key that zeroizes itself on drop
#[derive(Zeroize, ZeroizeOnDrop)]
struct MasterKey {
    bytes: [u8; 32],
}

impl MemoryVault {
    /// Create a new vault with a random master key
    pub fn generate() -> Self {
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        Self::from_key(key_bytes)
    }

    /// Create a vault from an existing 256-bit key
    pub fn from_key(key_bytes: [u8; 32]) -> Self {
        let cipher = ChaCha20Poly1305::new_from_slice(&key_bytes)
            .expect("32-byte key is always valid for ChaCha20Poly1305");
        Self {
            cipher,
            master_key: MasterKey { bytes: key_bytes },
            unlocked: true,
            bytes_sealed: 0,
            bytes_unsealed: 0,
            seal_count: 0,
        }
    }

    /// Derive a vault key from a passphrase using SHA-256
    /// (In production, use Argon2id instead)
    pub fn from_passphrase(passphrase: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(passphrase.as_bytes());
        hasher.update(b"apophy-sovereign-vault-salt-2026");
        let hash = hasher.finalize();
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&hash);
        Self::from_key(key_bytes)
    }

    /// Seal (encrypt) plaintext data
    ///
    /// Returns a SealedBlob containing:
    /// - Ciphertext (ChaCha20-Poly1305 AEAD)
    /// - Random nonce (unique per operation)
    /// - Integrity hash (SHA-256 of plaintext)
    pub fn seal(&mut self, plaintext: &[u8]) -> Result<SealedBlob> {
        if !self.unlocked {
            return Err(VaultError::Sealed);
        }

        // Generate random nonce (12 bytes)
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Compute integrity hash before encryption
        let integrity_hash = Self::compute_hash(plaintext);

        // Encrypt
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| VaultError::EncryptionFailed(e.to_string()))?;

        // Update metrics
        self.bytes_sealed += plaintext.len() as u64;
        self.seal_count += 1;

        Ok(SealedBlob {
            ciphertext,
            nonce: nonce_bytes,
            integrity_hash,
            sealed_at: chrono::Utc::now(),
            original_size: plaintext.len(),
        })
    }

    /// Unseal (decrypt) a sealed blob
    ///
    /// Verifies integrity hash after decryption to detect tampering.
    pub fn unseal(&mut self, blob: &SealedBlob) -> Result<Vec<u8>> {
        if !self.unlocked {
            return Err(VaultError::Sealed);
        }

        let nonce = Nonce::from_slice(&blob.nonce);

        // Decrypt
        let plaintext = self
            .cipher
            .decrypt(nonce, blob.ciphertext.as_ref())
            .map_err(|_| VaultError::DecryptionFailed)?;

        // Verify integrity
        let actual_hash = Self::compute_hash(&plaintext);
        if actual_hash != blob.integrity_hash {
            return Err(VaultError::IntegrityFailed {
                expected: blob.integrity_hash.clone(),
                actual: actual_hash,
            });
        }

        // Update metrics
        self.bytes_unsealed += plaintext.len() as u64;

        Ok(plaintext)
    }

    /// Seal a serializable value (JSON -> encrypt)
    pub fn seal_value<T: Serialize>(&mut self, value: &T) -> Result<SealedBlob> {
        let json = serde_json::to_vec(value)
            .map_err(|e| VaultError::EncryptionFailed(e.to_string()))?;
        self.seal(&json)
    }

    /// Unseal a serializable value (decrypt -> JSON parse)
    pub fn unseal_value<T: for<'de> Deserialize<'de>>(&mut self, blob: &SealedBlob) -> Result<T> {
        let plaintext = self.unseal(blob)?;
        serde_json::from_slice(&plaintext)
            .map_err(|e| VaultError::EncryptionFailed(e.to_string()))
    }

    /// Lock the vault (no operations possible until re-unlocked)
    pub fn lock(&mut self) {
        self.unlocked = false;
    }

    /// Unlock the vault
    pub fn unlock(&mut self) {
        self.unlocked = true;
    }

    pub fn is_unlocked(&self) -> bool {
        self.unlocked
    }

    /// Get vault metrics
    pub fn metrics(&self) -> VaultMetrics {
        VaultMetrics {
            bytes_sealed: self.bytes_sealed,
            bytes_unsealed: self.bytes_unsealed,
            seal_count: self.seal_count,
            unlocked: self.unlocked,
        }
    }

    fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMetrics {
    pub bytes_sealed: u64,
    pub bytes_unsealed: u64,
    pub seal_count: u64,
    pub unlocked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seal_unseal_roundtrip() {
        let mut vault = MemoryVault::generate();
        let plaintext = b"sovereign data that must be protected";

        let blob = vault.seal(plaintext).unwrap();
        assert_ne!(blob.ciphertext, plaintext);
        assert_eq!(blob.original_size, plaintext.len());

        let recovered = vault.unseal(&blob).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn test_different_nonces_per_seal() {
        let mut vault = MemoryVault::generate();
        let data = b"same data twice";

        let blob1 = vault.seal(data).unwrap();
        let blob2 = vault.seal(data).unwrap();

        // Same plaintext, different nonces → different ciphertext
        assert_ne!(blob1.nonce, blob2.nonce);
        assert_ne!(blob1.ciphertext, blob2.ciphertext);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let mut vault = MemoryVault::generate();
        let blob = vault.seal(b"original").unwrap();

        let mut tampered = blob.clone();
        if !tampered.ciphertext.is_empty() {
            tampered.ciphertext[0] ^= 0xFF;
        }

        assert!(vault.unseal(&tampered).is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let mut vault1 = MemoryVault::generate();
        let mut vault2 = MemoryVault::generate();

        let blob = vault1.seal(b"secret").unwrap();
        assert!(vault2.unseal(&blob).is_err());
    }

    #[test]
    fn test_sealed_vault_rejects_operations() {
        let mut vault = MemoryVault::generate();
        vault.lock();

        assert!(vault.seal(b"data").is_err());

        let blob = SealedBlob {
            ciphertext: vec![],
            nonce: [0u8; 12],
            integrity_hash: String::new(),
            sealed_at: chrono::Utc::now(),
            original_size: 0,
        };
        assert!(vault.unseal(&blob).is_err());

        vault.unlock();
        assert!(vault.is_unlocked());
    }

    #[test]
    fn test_seal_unseal_json_value() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct SecretConfig {
            api_key: String,
            tokens: Vec<String>,
        }

        let mut vault = MemoryVault::generate();
        let config = SecretConfig {
            api_key: "sk-sovereign-12345".into(),
            tokens: vec!["token1".into(), "token2".into()],
        };

        let blob = vault.seal_value(&config).unwrap();
        let recovered: SecretConfig = vault.unseal_value(&blob).unwrap();

        assert_eq!(recovered, config);
    }

    #[test]
    fn test_passphrase_derivation() {
        let mut vault1 = MemoryVault::from_passphrase("my-sovereign-passphrase");
        let mut vault2 = MemoryVault::from_passphrase("my-sovereign-passphrase");

        let blob = vault1.seal(b"data").unwrap();
        let recovered = vault2.unseal(&blob).unwrap();
        assert_eq!(recovered, b"data");
    }

    #[test]
    fn test_metrics_tracking() {
        let mut vault = MemoryVault::generate();

        vault.seal(b"hello").unwrap();
        vault.seal(b"world!").unwrap();

        let metrics = vault.metrics();
        assert_eq!(metrics.seal_count, 2);
        assert_eq!(metrics.bytes_sealed, 11); // 5 + 6
        assert!(metrics.unlocked);
    }

    #[test]
    fn test_large_data_seal() {
        let mut vault = MemoryVault::generate();
        let large_data = vec![0xABu8; 1_000_000]; // 1MB

        let blob = vault.seal(&large_data).unwrap();
        assert_eq!(blob.original_size, 1_000_000);

        let recovered = vault.unseal(&blob).unwrap();
        assert_eq!(recovered, large_data);
    }

    #[test]
    fn test_empty_data_seal() {
        let mut vault = MemoryVault::generate();
        let blob = vault.seal(b"").unwrap();
        let recovered = vault.unseal(&blob).unwrap();
        assert!(recovered.is_empty());
    }

    #[test]
    fn test_blob_serialization() {
        let mut vault = MemoryVault::generate();
        let blob = vault.seal(b"serializable").unwrap();

        let json = serde_json::to_string(&blob).unwrap();
        let restored: SealedBlob = serde_json::from_str(&json).unwrap();

        let recovered = vault.unseal(&restored).unwrap();
        assert_eq!(recovered, b"serializable");
    }
}
