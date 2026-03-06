//! # CTM-C — Chain-of-Thought Moderne-Compressed
//!
//! Compression layer for chain-of-thought reasoning, optimized for edge deployment.
//!
//! ## Why
//!
//! Full chain-of-thought is verbose and expensive. CTM-C compresses reasoning
//! chains into minimal, information-dense representations that preserve the
//! critical reasoning path while discarding redundant elaboration.
//!
//! ## Method
//!
//! 1. **Extract**: Identify key reasoning nodes (premises, deductions, conclusions)
//! 2. **Prune**: Remove redundant steps, verbose explanations, and filler
//! 3. **Compress**: Encode the essential chain in minimal tokens
//! 4. **Verify**: Ensure the compressed chain still reaches the correct conclusion
//!
//! This enables running complex reasoning on resource-constrained devices
//! by reducing the token budget required per reasoning task.

use crate::backend::{GenerationParams, InferenceBackend, InferenceRequest};
use crate::{InferenceError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// CTM-C Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtmcConfig {
    /// Target compression ratio (0.0 - 1.0, lower = more compression)
    pub target_ratio: f64,
    /// Maximum tokens for compression step
    pub max_compress_tokens: usize,
    /// Whether to verify the compressed chain preserves the conclusion
    pub verify_compression: bool,
    /// Minimum reasoning nodes to preserve
    pub min_nodes: usize,
}

impl Default for CtmcConfig {
    fn default() -> Self {
        Self {
            target_ratio: 0.3, // Compress to 30% of original
            max_compress_tokens: 1024,
            verify_compression: true,
            min_nodes: 3,
        }
    }
}

/// A compressed chain-of-thought
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedThought {
    /// The compressed reasoning chain
    pub compressed: String,
    /// Key reasoning nodes extracted
    pub nodes: Vec<ReasoningNode>,
    /// The final conclusion (preserved)
    pub conclusion: String,
    /// Compression statistics
    pub stats: CompressionStats,
    /// Content hash for integrity verification
    pub hash: String,
}

/// A key reasoning node in the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningNode {
    pub index: usize,
    pub node_type: NodeType,
    pub content: String,
    /// Importance score (0.0 - 1.0)
    pub importance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Premise,
    Deduction,
    Intermediate,
    Conclusion,
}

/// Compression statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub compression_ratio: f64,
    pub nodes_extracted: usize,
    pub nodes_preserved: usize,
    pub verification_passed: Option<bool>,
}

/// CTM-C Compressor
pub struct CtmcCompressor {
    config: CtmcConfig,
}

impl CtmcCompressor {
    pub fn new(config: CtmcConfig) -> Self {
        Self { config }
    }

    /// Compress a chain-of-thought
    pub fn compress(
        &self,
        backend: &dyn InferenceBackend,
        thought_chain: &str,
    ) -> Result<CompressedThought> {
        let original_tokens = backend.estimate_tokens(thought_chain);

        // Phase 1: Extract reasoning nodes
        let nodes = self.extract_nodes(backend, thought_chain)?;

        // Phase 2: Prune low-importance nodes
        let preserved = self.prune_nodes(&nodes);

        // Phase 3: Compress into minimal representation
        let (compressed, conclusion) = self.compress_nodes(backend, thought_chain, &preserved)?;

        let compressed_tokens = backend.estimate_tokens(&compressed);

        // Phase 4: Verify if enabled
        let verification_passed = if self.config.verify_compression {
            Some(self.verify_compression(backend, thought_chain, &compressed)?)
        } else {
            None
        };

        let ratio = if original_tokens > 0 {
            compressed_tokens as f64 / original_tokens as f64
        } else {
            1.0
        };

        // Content hash
        let mut hasher = Sha256::new();
        hasher.update(compressed.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let nodes_extracted = nodes.len();
        let nodes_preserved_count = preserved.len();
        Ok(CompressedThought {
            compressed,
            nodes: preserved,
            conclusion,
            stats: CompressionStats {
                original_tokens,
                compressed_tokens,
                compression_ratio: ratio,
                nodes_extracted,
                nodes_preserved: nodes_preserved_count,
                verification_passed,
            },
            hash,
        })
    }

    /// Extract reasoning nodes from the chain
    fn extract_nodes(
        &self,
        backend: &dyn InferenceBackend,
        thought_chain: &str,
    ) -> Result<Vec<ReasoningNode>> {
        let prompt = format!(
            "Extract the key reasoning nodes from this chain of thought.\n\
             For each node, identify:\n\
             - Type: PREMISE / DEDUCTION / INTERMEDIATE / CONCLUSION\n\
             - Content: the key insight (one sentence)\n\
             - Importance: 0.0 to 1.0\n\n\
             Chain of thought:\n{}\n\n\
             Nodes:",
            thought_chain
        );

        let request = InferenceRequest {
            prompt,
            params: GenerationParams::deterministic(),
            system_prompt: Some(
                "You are a reasoning chain analyzer. Extract only the essential \
                 reasoning nodes. Be concise."
                    .to_string(),
            ),
        };

        let response = backend
            .generate(&request)
            .map_err(|e| InferenceError::CompressionError(format!("Node extraction failed: {}", e)))?;

        // Parse nodes from response
        let nodes = parse_nodes(&response.text, thought_chain);

        // Ensure we have at least a conclusion
        if nodes.is_empty() {
            Ok(vec![ReasoningNode {
                index: 0,
                node_type: NodeType::Conclusion,
                content: extract_conclusion(thought_chain),
                importance: 1.0,
            }])
        } else {
            Ok(nodes)
        }
    }

    /// Prune low-importance nodes while preserving minimum count
    fn prune_nodes(&self, nodes: &[ReasoningNode]) -> Vec<ReasoningNode> {
        let mut sorted: Vec<_> = nodes.to_vec();
        sorted.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));

        // Always keep conclusions
        let mut preserved: Vec<ReasoningNode> = sorted
            .iter()
            .filter(|n| n.node_type == NodeType::Conclusion)
            .cloned()
            .collect();

        // Add most important non-conclusion nodes up to minimum
        for node in &sorted {
            if preserved.len() >= self.config.min_nodes {
                break;
            }
            if node.node_type != NodeType::Conclusion {
                preserved.push(node.clone());
            }
        }

        // Sort by original index
        preserved.sort_by_key(|n| n.index);
        preserved
    }

    /// Compress preserved nodes into minimal text
    fn compress_nodes(
        &self,
        backend: &dyn InferenceBackend,
        original: &str,
        nodes: &[ReasoningNode],
    ) -> Result<(String, String)> {
        let node_summary: String = nodes
            .iter()
            .map(|n| format!("[{:?}] {}", n.node_type, n.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Compress this reasoning chain into a minimal representation.\n\
             Preserve the logical flow and conclusion.\n\
             Target: {} of the original length.\n\n\
             Original ({} chars):\n{}\n\n\
             Key nodes:\n{}\n\n\
             Compressed chain:",
            format!("{:.0}%", self.config.target_ratio * 100.0),
            original.len(),
            original,
            node_summary,
        );

        let request = InferenceRequest {
            prompt,
            params: GenerationParams {
                max_tokens: self.config.max_compress_tokens,
                ..GenerationParams::deterministic()
            },
            system_prompt: Some(
                "You are a compression engine. Output only the compressed reasoning chain. \
                 No preamble. No explanation. Just the compressed chain."
                    .to_string(),
            ),
        };

        let response = backend
            .generate(&request)
            .map_err(|e| InferenceError::CompressionError(format!("Compression failed: {}", e)))?;

        let conclusion = nodes
            .iter()
            .find(|n| n.node_type == NodeType::Conclusion)
            .map(|n| n.content.clone())
            .unwrap_or_else(|| extract_conclusion(&response.text));

        Ok((response.text, conclusion))
    }

    /// Verify that compression preserves the correct conclusion
    fn verify_compression(
        &self,
        backend: &dyn InferenceBackend,
        original: &str,
        compressed: &str,
    ) -> Result<bool> {
        let prompt = format!(
            "Does this compressed reasoning reach the same conclusion as the original?\n\n\
             Original:\n{}\n\n\
             Compressed:\n{}\n\n\
             Answer SAME or DIFFERENT (one word only):",
            original, compressed
        );

        let request = InferenceRequest {
            prompt,
            params: GenerationParams::deterministic(),
            system_prompt: None,
        };

        let response = backend
            .generate(&request)
            .map_err(|e| InferenceError::CompressionError(format!("Verification failed: {}", e)))?;

        Ok(response.text.to_lowercase().contains("same")
            || response.text.to_lowercase().contains("verified")
            || response.text.to_lowercase().contains("correct"))
    }
}

/// Parse reasoning nodes from LLM output
fn parse_nodes(text: &str, original: &str) -> Vec<ReasoningNode> {
    let mut nodes = Vec::new();
    let mut index = 0;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let node_type = if line.to_lowercase().contains("premise") {
            NodeType::Premise
        } else if line.to_lowercase().contains("conclusion") || line.to_lowercase().contains("answer") {
            NodeType::Conclusion
        } else if line.to_lowercase().contains("deduction") || line.to_lowercase().contains("therefore") {
            NodeType::Deduction
        } else {
            NodeType::Intermediate
        };

        let importance = if node_type == NodeType::Conclusion {
            1.0
        } else if node_type == NodeType::Premise {
            0.8
        } else if node_type == NodeType::Deduction {
            0.7
        } else {
            0.5
        };

        // Extract content (remove type prefixes)
        let content = line
            .trim_start_matches(|c: char| c == '[' || c == ']' || c.is_ascii_uppercase() || c == ' ')
            .trim()
            .to_string();

        if !content.is_empty() {
            nodes.push(ReasoningNode {
                index,
                node_type,
                content,
                importance,
            });
            index += 1;
        }
    }

    // If no nodes were parsed, create from original text
    if nodes.is_empty() {
        let sentences: Vec<&str> = original.split('.').filter(|s| s.trim().len() > 5).collect();
        if let Some(first) = sentences.first() {
            nodes.push(ReasoningNode {
                index: 0,
                node_type: NodeType::Premise,
                content: first.trim().to_string(),
                importance: 0.8,
            });
        }
        if let Some(last) = sentences.last() {
            nodes.push(ReasoningNode {
                index: 1,
                node_type: NodeType::Conclusion,
                content: last.trim().to_string(),
                importance: 1.0,
            });
        }
    }

    nodes
}

/// Extract conclusion from text (last meaningful sentence)
fn extract_conclusion(text: &str) -> String {
    text.lines()
        .rev()
        .find(|l| l.trim().len() > 5)
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| text.chars().take(100).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::StubBackend;

    #[test]
    fn test_ctmc_basic_compression() {
        let config = CtmcConfig {
            target_ratio: 0.3,
            verify_compression: false,
            min_nodes: 2,
            ..Default::default()
        };
        let compressor = CtmcCompressor::new(config);
        let backend = StubBackend::new("test");

        let chain = "Step 1: Read the problem carefully. \
                     Step 2: Identify the key variables x and y. \
                     Step 3: Apply the quadratic formula. \
                     Step 4: Calculate the discriminant. \
                     Step 5: The answer is x = 3 and y = 7.";

        let result = compressor.compress(&backend, chain).unwrap();

        assert!(!result.compressed.is_empty());
        assert!(!result.nodes.is_empty());
        assert!(!result.conclusion.is_empty());
        assert!(result.stats.nodes_extracted > 0);
    }

    #[test]
    fn test_ctmc_with_verification() {
        let config = CtmcConfig {
            target_ratio: 0.5,
            verify_compression: true,
            min_nodes: 2,
            ..Default::default()
        };
        let compressor = CtmcCompressor::new(config);
        let backend = StubBackend::new("test");

        let chain = "Premise: All men are mortal. Socrates is a man. \
                     Deduction: Therefore Socrates is mortal. \
                     Conclusion: Socrates is mortal.";

        let result = compressor.compress(&backend, chain).unwrap();
        assert!(result.stats.verification_passed.is_some());
    }

    #[test]
    fn test_ctmc_preserves_conclusion() {
        let config = CtmcConfig {
            min_nodes: 1,
            verify_compression: false,
            ..Default::default()
        };
        let compressor = CtmcCompressor::new(config);
        let backend = StubBackend::new("test");

        let chain = "Analysis of the problem. Various intermediate steps. The answer is 42.";
        let result = compressor.compress(&backend, chain).unwrap();

        // Should have at least one node
        assert!(!result.nodes.is_empty());
    }

    #[test]
    fn test_ctmc_stats() {
        let config = CtmcConfig {
            verify_compression: false,
            ..Default::default()
        };
        let compressor = CtmcCompressor::new(config);
        let backend = StubBackend::new("test");

        let chain = "A long chain of reasoning with many steps and intermediate calculations.";
        let result = compressor.compress(&backend, chain).unwrap();

        assert!(result.stats.original_tokens > 0);
        assert!(result.stats.compressed_tokens > 0);
        assert!(result.stats.compression_ratio > 0.0);
        assert!(!result.hash.is_empty());
    }

    #[test]
    fn test_node_parsing() {
        let text = "[PREMISE] All cats are animals\n[DEDUCTION] Therefore my cat is an animal\n[CONCLUSION] Answer: yes";
        let nodes = parse_nodes(text, "");
        assert!(nodes.len() >= 2);
    }

    #[test]
    fn test_extract_conclusion() {
        let result = extract_conclusion("Step 1\nStep 2\nThe answer is 42");
        assert_eq!(result, "The answer is 42");
    }

    #[test]
    fn test_prune_preserves_min_nodes() {
        let config = CtmcConfig {
            min_nodes: 3,
            ..Default::default()
        };
        let compressor = CtmcCompressor::new(config);

        let nodes = vec![
            ReasoningNode { index: 0, node_type: NodeType::Premise, content: "P1".into(), importance: 0.8 },
            ReasoningNode { index: 1, node_type: NodeType::Intermediate, content: "I1".into(), importance: 0.3 },
            ReasoningNode { index: 2, node_type: NodeType::Intermediate, content: "I2".into(), importance: 0.2 },
            ReasoningNode { index: 3, node_type: NodeType::Conclusion, content: "C1".into(), importance: 1.0 },
        ];

        let preserved = compressor.prune_nodes(&nodes);
        assert!(preserved.len() >= 3);
        // Conclusion must be preserved
        assert!(preserved.iter().any(|n| n.node_type == NodeType::Conclusion));
    }
}
