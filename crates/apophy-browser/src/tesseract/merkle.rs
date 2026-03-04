//! # Merkle Timeline — Holographic Fractal Proof
//!
//! "Chaque partie contient le tout."
//!
//! A Merkle tree over the timeline states. The root hash is the
//! fingerprint of ALL states combined. You can prove any single
//! state's inclusion without revealing the rest — zero-knowledge
//! proof of browsing history.
//!
//! ## Properties
//!
//! - **Holographic**: root = f(all states). Break the chain, each piece still proves itself.
//! - **Fractal**: any subtree is a valid Merkle tree of its own states.
//! - **O(log n) proofs**: prove inclusion of 1 state in a tree of N states with log2(N) hashes.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A node in the Merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
    /// Index of the leaf state (only for leaf nodes)
    pub leaf_index: Option<usize>,
}

/// A Merkle proof — path from leaf to root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// The leaf hash being proved
    pub leaf_hash: [u8; 32],
    /// Index of the leaf in the original list
    pub leaf_index: usize,
    /// Sibling hashes along the path to root (with position: left or right)
    pub path: Vec<ProofStep>,
    /// The expected root hash
    pub root: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    pub hash: [u8; 32],
    /// true = sibling is on the right, false = sibling is on the left
    pub is_right: bool,
}

/// The Merkle Timeline — tree over tab state hashes
pub struct MerkleTimeline {
    /// Root of the Merkle tree
    root: MerkleNode,
    /// Original leaf hashes (state hashes in order)
    leaves: Vec<[u8; 32]>,
}

impl MerkleTimeline {
    /// Build a Merkle tree from a list of state hashes.
    ///
    /// If the number of leaves is odd, the last leaf is duplicated.
    pub fn build(state_hashes: &[[u8; 32]]) -> Option<Self> {
        if state_hashes.is_empty() {
            return None;
        }

        let leaves = state_hashes.to_vec();
        let root = Self::build_tree(&leaves);

        Some(Self { root, leaves })
    }

    /// Get the root hash — the fingerprint of ALL states
    pub fn root_hash(&self) -> [u8; 32] {
        self.root.hash
    }

    /// Short root hash for display
    pub fn short_root(&self) -> String {
        self.root.hash.iter().take(4).map(|b| format!("{:02x}", b)).collect()
    }

    /// Number of leaves (states) in the tree
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    /// Generate a Merkle proof for a specific leaf index.
    ///
    /// Returns a proof that can verify inclusion of the leaf hash in the tree
    /// without revealing any other leaf.
    pub fn prove(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaves.len() {
            return None;
        }

        let path = Self::generate_proof_path(&self.leaves, index);

        Some(MerkleProof {
            leaf_hash: self.leaves[index],
            leaf_index: index,
            path,
            root: self.root.hash,
        })
    }

    /// Verify a Merkle proof: does this leaf belong to the tree with this root?
    pub fn verify_proof(proof: &MerkleProof) -> bool {
        let mut current = proof.leaf_hash;

        for step in &proof.path {
            let mut hasher = Sha256::new();
            if step.is_right {
                hasher.update(current);
                hasher.update(step.hash);
            } else {
                hasher.update(step.hash);
                hasher.update(current);
            }
            current = hasher.finalize().into();
        }

        current == proof.root
    }

    // === Internal tree construction ===

    fn build_tree(leaves: &[[u8; 32]]) -> MerkleNode {
        if leaves.len() == 1 {
            return MerkleNode {
                hash: leaves[0],
                left: None,
                right: None,
                leaf_index: Some(0),
            };
        }

        // Bottom-up: pair leaves, hash pairs, repeat
        let mut level: Vec<MerkleNode> = leaves
            .iter()
            .enumerate()
            .map(|(i, h)| MerkleNode {
                hash: *h,
                left: None,
                right: None,
                leaf_index: Some(i),
            })
            .collect();

        while level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in level.chunks(2) {
                let left = chunk[0].clone();
                let right = if chunk.len() > 1 {
                    chunk[1].clone()
                } else {
                    left.clone() // Duplicate last if odd
                };

                let mut hasher = Sha256::new();
                hasher.update(left.hash);
                hasher.update(right.hash);
                let hash: [u8; 32] = hasher.finalize().into();

                next_level.push(MerkleNode {
                    hash,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                    leaf_index: None,
                });
            }

            level = next_level;
        }

        level.into_iter().next().unwrap()
    }

    fn generate_proof_path(leaves: &[[u8; 32]], target_index: usize) -> Vec<ProofStep> {
        if leaves.len() <= 1 {
            return vec![];
        }

        let mut path = Vec::new();
        let mut current_level = leaves.to_vec();
        let mut idx = target_index;

        while current_level.len() > 1 {
            // Pad odd level
            if current_level.len() % 2 != 0 {
                let last = *current_level.last().unwrap();
                current_level.push(last);
            }

            // Find sibling
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            let is_right = idx % 2 == 0; // sibling is on the right if we're even

            path.push(ProofStep {
                hash: current_level[sibling_idx],
                is_right,
            });

            // Build next level
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(chunk[0]);
                hasher.update(chunk[1]);
                next_level.push(hasher.finalize().into());
            }

            current_level = next_level;
            idx /= 2;
        }

        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tesseract::state::TabState;
    use uuid::Uuid;

    fn make_hashes(n: usize) -> Vec<[u8; 32]> {
        let tab_id = Uuid::new_v4();
        let mut states = vec![TabState::genesis(tab_id)];
        for i in 1..n {
            let prev = states.last().unwrap();
            states.push(TabState::new(
                tab_id,
                format!("https://page{}.com", i),
                format!("content-{}", i).as_bytes(),
                format!("Page {}", i),
                0,
                prev.hash,
            ));
        }
        states.iter().map(|s| s.hash).collect()
    }

    #[test]
    fn test_build_single_leaf() {
        let hashes = make_hashes(1);
        let tree = MerkleTimeline::build(&hashes).unwrap();
        assert_eq!(tree.leaf_count(), 1);
        assert_eq!(tree.root_hash(), hashes[0]); // Single leaf = root
    }

    #[test]
    fn test_build_two_leaves() {
        let hashes = make_hashes(2);
        let tree = MerkleTimeline::build(&hashes).unwrap();
        assert_eq!(tree.leaf_count(), 2);
        // Root should be hash(leaf0 || leaf1)
        let mut expected = Sha256::new();
        expected.update(hashes[0]);
        expected.update(hashes[1]);
        let root: [u8; 32] = expected.finalize().into();
        assert_eq!(tree.root_hash(), root);
    }

    #[test]
    fn test_build_four_leaves() {
        let hashes = make_hashes(4);
        let tree = MerkleTimeline::build(&hashes).unwrap();
        assert_eq!(tree.leaf_count(), 4);
        assert_eq!(tree.short_root().len(), 8);
    }

    #[test]
    fn test_build_odd_leaves() {
        let hashes = make_hashes(5);
        let tree = MerkleTimeline::build(&hashes).unwrap();
        assert_eq!(tree.leaf_count(), 5);
    }

    #[test]
    fn test_empty_returns_none() {
        assert!(MerkleTimeline::build(&[]).is_none());
    }

    #[test]
    fn test_proof_and_verify_two_leaves() {
        let hashes = make_hashes(2);
        let tree = MerkleTimeline::build(&hashes).unwrap();

        let proof0 = tree.prove(0).unwrap();
        assert!(MerkleTimeline::verify_proof(&proof0));
        assert_eq!(proof0.leaf_hash, hashes[0]);

        let proof1 = tree.prove(1).unwrap();
        assert!(MerkleTimeline::verify_proof(&proof1));
        assert_eq!(proof1.leaf_hash, hashes[1]);
    }

    #[test]
    fn test_proof_and_verify_eight_leaves() {
        let hashes = make_hashes(8);
        let tree = MerkleTimeline::build(&hashes).unwrap();

        for i in 0..8 {
            let proof = tree.prove(i).unwrap();
            assert!(
                MerkleTimeline::verify_proof(&proof),
                "Proof failed for leaf {}",
                i
            );
            assert_eq!(proof.leaf_hash, hashes[i]);
            assert_eq!(proof.root, tree.root_hash());
            // Proof path should be log2(8) = 3 steps
            assert_eq!(proof.path.len(), 3, "Wrong path length for leaf {}", i);
        }
    }

    #[test]
    fn test_proof_out_of_bounds() {
        let hashes = make_hashes(4);
        let tree = MerkleTimeline::build(&hashes).unwrap();
        assert!(tree.prove(4).is_none());
        assert!(tree.prove(100).is_none());
    }

    #[test]
    fn test_tampered_proof_fails() {
        let hashes = make_hashes(4);
        let tree = MerkleTimeline::build(&hashes).unwrap();

        let mut proof = tree.prove(0).unwrap();
        // Tamper with the leaf hash
        proof.leaf_hash[0] ^= 0xFF;
        assert!(!MerkleTimeline::verify_proof(&proof));
    }

    #[test]
    fn test_wrong_root_fails() {
        let hashes = make_hashes(4);
        let tree = MerkleTimeline::build(&hashes).unwrap();

        let mut proof = tree.prove(0).unwrap();
        proof.root[0] ^= 0xFF;
        assert!(!MerkleTimeline::verify_proof(&proof));
    }

    #[test]
    fn test_different_trees_different_roots() {
        let hashes_a = make_hashes(4);
        let hashes_b = make_hashes(4); // Different UUIDs = different hashes

        let tree_a = MerkleTimeline::build(&hashes_a).unwrap();
        let tree_b = MerkleTimeline::build(&hashes_b).unwrap();

        assert_ne!(tree_a.root_hash(), tree_b.root_hash());
    }

    #[test]
    fn test_same_data_same_root() {
        let hashes = make_hashes(4);
        let tree1 = MerkleTimeline::build(&hashes).unwrap();
        let tree2 = MerkleTimeline::build(&hashes).unwrap();
        assert_eq!(tree1.root_hash(), tree2.root_hash());
    }

    #[test]
    fn test_large_tree() {
        let hashes = make_hashes(1000);
        let tree = MerkleTimeline::build(&hashes).unwrap();

        assert_eq!(tree.leaf_count(), 1000);

        // Verify random proofs
        for i in [0, 1, 499, 500, 998, 999] {
            let proof = tree.prove(i).unwrap();
            assert!(
                MerkleTimeline::verify_proof(&proof),
                "Proof failed for leaf {} in 1000-leaf tree",
                i
            );
        }
    }

    #[test]
    fn test_proof_serialization() {
        let hashes = make_hashes(4);
        let tree = MerkleTimeline::build(&hashes).unwrap();
        let proof = tree.prove(2).unwrap();

        let json = serde_json::to_string(&proof).unwrap();
        let restored: MerkleProof = serde_json::from_str(&json).unwrap();
        assert!(MerkleTimeline::verify_proof(&restored));
    }
}
