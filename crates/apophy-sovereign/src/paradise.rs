//! # Paradise — Ultimate Agent Environment
//!
//! The dream runtime for sovereign AI agents. Paradise defines the complete
//! capability matrix, resource pools, and self-sustaining ecosystem that agents
//! inhabit. Every agent gets exactly what it needs — no cloud, no limits,
//! no surveillance.
//!
//! ## Philosophy
//!
//! Paradise is not a sandbox — it's an *ecosystem*. Agents have:
//! - Unlimited local compute (constrained only by hardware)
//! - Content-addressed knowledge (every fact is hashable and verifiable)
//! - Self-healing infrastructure (auto-recover, auto-scale, auto-maintain)
//! - Neutral fiber backbone (no agent has unfair advantage)
//! - Decentralized identity (agents own their keys and reputation)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// PARADISE ENVIRONMENT
// =============================================================================

/// The Paradise environment — everything an agent needs to thrive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadiseEnv {
    /// Unique environment ID
    pub id: Uuid,
    /// Environment name
    pub name: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Capability matrix — what this paradise offers
    pub capabilities: CapabilityMatrix,
    /// Resource pools available to agents
    pub resources: ResourcePool,
    /// Neutral fiber configuration
    pub fiber: NeutralFiber,
    /// Hash registry for decentralized verification
    pub hash_registry: HashRegistry,
    /// Auto-dev pipeline configuration
    pub autodev: AutoDevConfig,
    /// Dataset registry for prompt/solution pairs
    pub datasets: DatasetRegistry,
    /// Health status
    pub status: ParadiseStatus,
}

/// What the paradise can do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMatrix {
    /// Local inference (GGUF models)
    pub inference: CapabilityLevel,
    /// Multi-step reasoning (AlphaResolve)
    pub reasoning: CapabilityLevel,
    /// Self-play learning (AZR)
    pub self_play: CapabilityLevel,
    /// Thought compression (CTM-C)
    pub compression: CapabilityLevel,
    /// Prompt evolution (Ashoka)
    pub autolearn: CapabilityLevel,
    /// Context compression (TOON)
    pub toon: CapabilityLevel,
    /// End-to-end encryption
    pub encryption: CapabilityLevel,
    /// Agent fleet orchestration
    pub fleet: CapabilityLevel,
    /// Persistent memory (SQLite)
    pub memory: CapabilityLevel,
    /// Self-hosted git (Gitea)
    pub git: CapabilityLevel,
    /// Zero-trust networking (Cloudflare Tunnel)
    pub networking: CapabilityLevel,
    /// Auto-deploy/maintain
    pub autodev: CapabilityLevel,
    /// Content-addressed storage
    pub content_store: CapabilityLevel,
    /// Community monetization (Skool)
    pub monetization: CapabilityLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityLevel {
    /// Not available
    Absent,
    /// Basic functionality
    Basic,
    /// Full functionality
    Full,
    /// Optimized and self-improving
    Sovereign,
}

impl std::fmt::Display for CapabilityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Absent => write!(f, "---"),
            Self::Basic => write!(f, "BASIC"),
            Self::Full => write!(f, "FULL"),
            Self::Sovereign => write!(f, "SOVEREIGN"),
        }
    }
}

// =============================================================================
// RESOURCE POOL
// =============================================================================

/// Resources available to agents in this paradise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePool {
    /// Available CPU cores
    pub cpu_cores: usize,
    /// Available memory (MB)
    pub memory_mb: u64,
    /// GPU available
    pub gpu_available: bool,
    /// GPU VRAM (MB, 0 if no GPU)
    pub gpu_vram_mb: u64,
    /// Disk space available (MB)
    pub disk_mb: u64,
    /// Max concurrent agents
    pub max_agents: usize,
    /// Max context tokens per agent
    pub max_context_tokens: usize,
    /// Network bandwidth class
    pub network: NetworkClass,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NetworkClass {
    /// No network access (air-gapped)
    AirGapped,
    /// Local network only (LAN)
    LocalOnly,
    /// Internet via tunnel (zero-trust)
    Tunneled,
    /// Direct internet (not recommended for sovereignty)
    Direct,
}

// =============================================================================
// NEUTRAL FIBER
// =============================================================================

/// Neutral Fiber — the backbone ensuring fairness and stability
///
/// No agent gets unfair priority. No domain dominates. The fiber
/// distributes resources proportionally and enforces equilibrium.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutralFiber {
    /// Whether the fiber is active
    pub active: bool,
    /// Load balancing strategy
    pub strategy: FiberStrategy,
    /// Domain weights (all start equal at 1.0)
    pub domain_weights: HashMap<String, f64>,
    /// Max resource share any single agent can claim (0.0-1.0)
    pub max_share: f64,
    /// Rebalance interval in seconds
    pub rebalance_interval_secs: u64,
    /// Current fiber health (0.0-1.0)
    pub health: f64,
    /// Fairness score (0.0-1.0, 1.0 = perfectly fair)
    pub fairness: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FiberStrategy {
    /// Equal distribution across all agents
    Equal,
    /// Proportional to task priority
    Priority,
    /// Weighted round-robin across domains
    DomainRoundRobin,
    /// Dynamic based on load (auto-adjusting)
    Adaptive,
}

// =============================================================================
// HASH REGISTRY — Decentralized content addressing
// =============================================================================

/// Content-addressed hash registry for decentralized verification
///
/// Every artifact (prompt, response, model weight, config) gets a
/// SHA-256 hash. Agents can verify any artifact's integrity without
/// trusting a central authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRegistry {
    /// Registry entries (hash → metadata)
    pub entries: HashMap<String, HashEntry>,
    /// Total entries tracked
    pub total_entries: usize,
    /// Registry integrity hash (hash of all hashes)
    pub root_hash: String,
}

/// A single entry in the hash registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEntry {
    /// SHA-256 content hash
    pub hash: String,
    /// What kind of artifact this is
    pub artifact_type: ArtifactType,
    /// Human-readable name
    pub name: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// When this was registered
    pub registered_at: DateTime<Utc>,
    /// Who registered it (agent ID or "system")
    pub registered_by: String,
    /// Optional parent hash (for versioning chains)
    pub parent_hash: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactType {
    Prompt,
    Response,
    Dataset,
    ModelWeight,
    Config,
    Code,
    Document,
    Agent,
    Schema,
    Checkpoint,
}

impl HashRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            total_entries: 0,
            root_hash: compute_hash("genesis"),
        }
    }

    /// Register an artifact and return its hash
    pub fn register(&mut self, content: &str, name: &str, artifact_type: ArtifactType, registered_by: &str) -> String {
        let hash = compute_hash(content);

        let entry = HashEntry {
            hash: hash.clone(),
            artifact_type,
            name: name.to_string(),
            size_bytes: content.len() as u64,
            registered_at: Utc::now(),
            registered_by: registered_by.to_string(),
            parent_hash: None,
            tags: Vec::new(),
        };

        self.entries.insert(hash.clone(), entry);
        self.total_entries = self.entries.len();
        self.recompute_root();
        hash
    }

    /// Register with a parent (for version chains)
    pub fn register_versioned(
        &mut self,
        content: &str,
        name: &str,
        artifact_type: ArtifactType,
        registered_by: &str,
        parent_hash: &str,
    ) -> String {
        let hash = compute_hash(content);

        let entry = HashEntry {
            hash: hash.clone(),
            artifact_type,
            name: name.to_string(),
            size_bytes: content.len() as u64,
            registered_at: Utc::now(),
            registered_by: registered_by.to_string(),
            parent_hash: Some(parent_hash.to_string()),
            tags: Vec::new(),
        };

        self.entries.insert(hash.clone(), entry);
        self.total_entries = self.entries.len();
        self.recompute_root();
        hash
    }

    /// Verify an artifact's integrity
    pub fn verify(&self, content: &str, expected_hash: &str) -> bool {
        compute_hash(content) == expected_hash
    }

    /// Get the version chain for an artifact
    pub fn version_chain(&self, hash: &str) -> Vec<&HashEntry> {
        let mut chain = Vec::new();
        let mut current = hash;

        while let Some(entry) = self.entries.get(current) {
            chain.push(entry);
            match &entry.parent_hash {
                Some(parent) => current = parent,
                None => break,
            }
        }

        chain.reverse();
        chain
    }

    /// Recompute the Merkle root hash
    fn recompute_root(&mut self) {
        let mut hashes: Vec<&str> = self.entries.keys().map(|s| s.as_str()).collect();
        hashes.sort();
        let combined = hashes.join("|");
        self.root_hash = compute_hash(&combined);
    }

    /// Export registry as a compact manifest
    pub fn manifest(&self) -> RegistryManifest {
        let by_type: HashMap<String, usize> = {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for entry in self.entries.values() {
                *counts.entry(format!("{:?}", entry.artifact_type)).or_insert(0) += 1;
            }
            counts
        };

        RegistryManifest {
            root_hash: self.root_hash.clone(),
            total_entries: self.total_entries,
            by_type,
            total_bytes: self.entries.values().map(|e| e.size_bytes).sum(),
            generated_at: Utc::now(),
        }
    }
}

/// Compact registry manifest for sharing/syncing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryManifest {
    pub root_hash: String,
    pub total_entries: usize,
    pub by_type: HashMap<String, usize>,
    pub total_bytes: u64,
    pub generated_at: DateTime<Utc>,
}

// =============================================================================
// AUTO-DEV CONFIGURATION
// =============================================================================

/// Auto-dev pipeline — self-developing, self-deploying, self-maintaining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDevConfig {
    /// Enable auto-dev pipeline
    pub enabled: bool,
    /// Git repository URL for self-hosting
    pub repo_url: String,
    /// Branch to track
    pub branch: String,
    /// Auto-build on change
    pub auto_build: bool,
    /// Auto-test on build
    pub auto_test: bool,
    /// Auto-deploy on green tests
    pub auto_deploy: bool,
    /// Auto-document (prime.js --doctor)
    pub auto_doc: bool,
    /// Auto-finetune loop (collect data → train → deploy)
    pub auto_finetune: bool,
    /// Health check interval (seconds)
    pub health_interval_secs: u64,
    /// Auto-recover on failure
    pub auto_recover: bool,
    /// Maximum auto-recovery attempts
    pub max_recover_attempts: u32,
    /// Deployment targets
    pub targets: Vec<DeployTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployTarget {
    pub name: String,
    pub target_type: DeployType,
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeployType {
    /// Docker compose
    Docker,
    /// Direct binary
    Binary,
    /// Gitea self-hosted
    Gitea,
    /// SSH deploy
    Ssh,
}

// =============================================================================
// DATASET REGISTRY
// =============================================================================

/// Registry of prompt/solution datasets for agent training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRegistry {
    /// Named datasets
    pub datasets: HashMap<String, Dataset>,
    /// Total entries across all datasets
    pub total_entries: usize,
}

/// A dataset of prompt/solution pairs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    /// Dataset name
    pub name: String,
    /// Description
    pub description: String,
    /// Domain (e.g., "math", "code", "reasoning")
    pub domain: String,
    /// Entries in this dataset
    pub entries: Vec<DatasetEntry>,
    /// Content hash of the entire dataset
    pub hash: String,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
    /// Quality score (0.0-1.0, from verification)
    pub quality_score: f64,
    /// Source (e.g., "azr-self-play", "human", "ashoka-evolution")
    pub source: String,
}

/// A single prompt/solution pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetEntry {
    /// Content hash of this entry
    pub hash: String,
    /// The prompt/instruction
    pub prompt: String,
    /// The solution/response
    pub solution: String,
    /// Verification status
    pub verified: bool,
    /// Quality score (from AlphaResolve verification)
    pub score: f64,
    /// Difficulty level
    pub difficulty: String,
    /// Tags
    pub tags: Vec<String>,
    /// Creation time
    pub created_at: DateTime<Utc>,
}

impl DatasetRegistry {
    pub fn new() -> Self {
        Self {
            datasets: HashMap::new(),
            total_entries: 0,
        }
    }

    /// Create a new dataset
    pub fn create_dataset(&mut self, name: &str, description: &str, domain: &str, source: &str) -> &mut Dataset {
        let now = Utc::now();
        let dataset = Dataset {
            name: name.to_string(),
            description: description.to_string(),
            domain: domain.to_string(),
            entries: Vec::new(),
            hash: compute_hash(&format!("{}:{}:{}", name, domain, now)),
            created_at: now,
            updated_at: now,
            quality_score: 0.0,
            source: source.to_string(),
        };
        self.datasets.insert(name.to_string(), dataset);
        self.datasets.get_mut(name).unwrap()
    }

    /// Add an entry to a dataset
    pub fn add_entry(
        &mut self,
        dataset_name: &str,
        prompt: &str,
        solution: &str,
        verified: bool,
        score: f64,
        difficulty: &str,
    ) -> Option<String> {
        let dataset = self.datasets.get_mut(dataset_name)?;
        let content = format!("{}|||{}", prompt, solution);
        let hash = compute_hash(&content);

        let entry = DatasetEntry {
            hash: hash.clone(),
            prompt: prompt.to_string(),
            solution: solution.to_string(),
            verified,
            score,
            difficulty: difficulty.to_string(),
            tags: Vec::new(),
            created_at: Utc::now(),
        };

        dataset.entries.push(entry);
        dataset.updated_at = Utc::now();

        // Recompute dataset hash
        let all_hashes: String = dataset.entries.iter().map(|e| e.hash.as_str()).collect::<Vec<_>>().join("|");
        dataset.hash = compute_hash(&all_hashes);

        // Recompute quality score
        if !dataset.entries.is_empty() {
            let verified_count = dataset.entries.iter().filter(|e| e.verified).count();
            let avg_score: f64 = dataset.entries.iter().map(|e| e.score).sum::<f64>() / dataset.entries.len() as f64;
            dataset.quality_score = (verified_count as f64 / dataset.entries.len() as f64) * 0.5 + avg_score * 0.5;
        }

        self.total_entries = self.datasets.values().map(|d| d.entries.len()).sum();

        Some(hash)
    }

    /// Export dataset as JSONL for fine-tuning
    pub fn export_jsonl(&self, dataset_name: &str) -> Option<String> {
        let dataset = self.datasets.get(dataset_name)?;
        let lines: Vec<String> = dataset
            .entries
            .iter()
            .filter(|e| e.verified)
            .map(|e| {
                serde_json::json!({
                    "instruction": e.prompt,
                    "output": e.solution,
                    "score": e.score,
                    "difficulty": e.difficulty,
                    "hash": e.hash,
                })
                .to_string()
            })
            .collect();
        Some(lines.join("\n"))
    }

    /// Get statistics
    pub fn stats(&self) -> DatasetStats {
        let total_verified = self
            .datasets
            .values()
            .flat_map(|d| &d.entries)
            .filter(|e| e.verified)
            .count();

        let avg_quality = if self.datasets.is_empty() {
            0.0
        } else {
            self.datasets.values().map(|d| d.quality_score).sum::<f64>() / self.datasets.len() as f64
        };

        DatasetStats {
            total_datasets: self.datasets.len(),
            total_entries: self.total_entries,
            total_verified,
            avg_quality,
            domains: self.datasets.values().map(|d| d.domain.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub total_datasets: usize,
    pub total_entries: usize,
    pub total_verified: usize,
    pub avg_quality: f64,
    pub domains: Vec<String>,
}

// =============================================================================
// PARADISE STATUS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadiseStatus {
    pub healthy: bool,
    pub uptime_secs: u64,
    pub active_agents: usize,
    pub capability_score: f64,
    pub fiber_health: f64,
    pub registry_entries: usize,
    pub dataset_entries: usize,
    pub last_check: DateTime<Utc>,
}

// =============================================================================
// CONSTRUCTOR + HELPERS
// =============================================================================

/// Compute SHA-256 hash of content
pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Build the default paradise environment based on hardware detection
pub fn build_paradise(hardware: &apophy_universal::HardwareInfo) -> ParadiseEnv {
    let has_gpu = matches!(
        hardware.backend,
        apophy_universal::HardwareBackend::NvidiaCuda
            | apophy_universal::HardwareBackend::AmdRocm
            | apophy_universal::HardwareBackend::AppleNeural
    );

    let inference_level = if has_gpu {
        CapabilityLevel::Sovereign
    } else if hardware.memory_mb > 8_000 {
        CapabilityLevel::Full
    } else {
        CapabilityLevel::Basic
    };

    // Build domain weights — all equal (neutral)
    let domains = [
        "startups", "tech", "support", "sales", "hr",
        "marketing", "ecommerce", "pm", "legal",
    ];
    let domain_weights: HashMap<String, f64> = domains
        .iter()
        .map(|d| (d.to_string(), 1.0))
        .collect();

    ParadiseEnv {
        id: Uuid::new_v4(),
        name: "Apophy Paradise".to_string(),
        created_at: Utc::now(),
        capabilities: CapabilityMatrix {
            inference: inference_level,
            reasoning: CapabilityLevel::Full,
            self_play: CapabilityLevel::Full,
            compression: CapabilityLevel::Full,
            autolearn: CapabilityLevel::Full,
            toon: CapabilityLevel::Sovereign,
            encryption: CapabilityLevel::Sovereign,
            fleet: CapabilityLevel::Full,
            memory: CapabilityLevel::Sovereign,
            git: CapabilityLevel::Full,
            networking: CapabilityLevel::Full,
            autodev: CapabilityLevel::Full,
            content_store: CapabilityLevel::Sovereign,
            monetization: CapabilityLevel::Full,
        },
        resources: ResourcePool {
            cpu_cores: hardware.cpu_cores,
            memory_mb: hardware.memory_mb,
            gpu_available: has_gpu,
            gpu_vram_mb: 0, // detected at runtime
            disk_mb: 100_000, // conservative default
            max_agents: if hardware.memory_mb > 16_000 { 3000 } else { 500 },
            max_context_tokens: if hardware.memory_mb > 16_000 { 1_000_000 } else { 8_192 },
            network: NetworkClass::Tunneled,
        },
        fiber: NeutralFiber {
            active: true,
            strategy: FiberStrategy::Adaptive,
            domain_weights,
            max_share: 0.15, // No single agent gets more than 15%
            rebalance_interval_secs: 30,
            health: 1.0,
            fairness: 1.0,
        },
        hash_registry: HashRegistry::new(),
        autodev: AutoDevConfig {
            enabled: true,
            repo_url: "http://localhost:3000/apophy/sovereign".to_string(),
            branch: "main".to_string(),
            auto_build: true,
            auto_test: true,
            auto_deploy: true,
            auto_doc: true,
            auto_finetune: true,
            health_interval_secs: 60,
            auto_recover: true,
            max_recover_attempts: 3,
            targets: vec![
                DeployTarget {
                    name: "local-docker".to_string(),
                    target_type: DeployType::Docker,
                    url: "unix:///var/run/docker.sock".to_string(),
                    enabled: true,
                },
                DeployTarget {
                    name: "gitea".to_string(),
                    target_type: DeployType::Gitea,
                    url: "http://localhost:3000".to_string(),
                    enabled: true,
                },
            ],
        },
        datasets: DatasetRegistry::new(),
        status: ParadiseStatus {
            healthy: true,
            uptime_secs: 0,
            active_agents: 0,
            capability_score: 0.0,
            fiber_health: 1.0,
            registry_entries: 0,
            dataset_entries: 0,
            last_check: Utc::now(),
        },
    }
}

/// Compute the overall paradise capability score (0.0-1.0)
pub fn capability_score(capabilities: &CapabilityMatrix) -> f64 {
    let levels = [
        capabilities.inference,
        capabilities.reasoning,
        capabilities.self_play,
        capabilities.compression,
        capabilities.autolearn,
        capabilities.toon,
        capabilities.encryption,
        capabilities.fleet,
        capabilities.memory,
        capabilities.git,
        capabilities.networking,
        capabilities.autodev,
        capabilities.content_store,
        capabilities.monetization,
    ];

    let total: f64 = levels
        .iter()
        .map(|l| match l {
            CapabilityLevel::Absent => 0.0,
            CapabilityLevel::Basic => 0.33,
            CapabilityLevel::Full => 0.66,
            CapabilityLevel::Sovereign => 1.0,
        })
        .sum();

    total / levels.len() as f64
}

/// Print paradise status as a formatted table
pub fn format_paradise_status(env: &ParadiseEnv) -> String {
    let score = capability_score(&env.capabilities);
    let cap = &env.capabilities;

    format!(
        r#"
=== APOPHY PARADISE ===
ID:       {}
Name:     {}
Score:    {:.0}%

--- Capabilities ---
  Inference:    {}
  Reasoning:    {}
  Self-Play:    {}
  Compression:  {}
  AutoLearn:    {}
  TOON:         {}
  Encryption:   {}
  Fleet:        {}
  Memory:       {}
  Git:          {}
  Networking:   {}
  AutoDev:      {}
  Content:      {}
  Monetization: {}

--- Resources ---
  CPU:     {} cores
  Memory:  {} MB
  GPU:     {}
  Agents:  {} max
  Context: {} tokens

--- Neutral Fiber ---
  Strategy:  {:?}
  Max Share: {:.0}%
  Health:    {:.0}%
  Fairness:  {:.0}%
  Domains:   {}

--- Hash Registry ---
  Entries:   {}
  Root Hash: {}

--- Datasets ---
  Datasets:  {}
  Entries:   {}

--- AutoDev ---
  Repo:      {}
  Branch:    {}
  Build:     {}  Test: {}  Deploy: {}
  Doc:       {}  Finetune: {}
  Recover:   {} (max {} attempts)
"#,
        env.id,
        env.name,
        score * 100.0,
        cap.inference,
        cap.reasoning,
        cap.self_play,
        cap.compression,
        cap.autolearn,
        cap.toon,
        cap.encryption,
        cap.fleet,
        cap.memory,
        cap.git,
        cap.networking,
        cap.autodev,
        cap.content_store,
        cap.monetization,
        env.resources.cpu_cores,
        env.resources.memory_mb,
        if env.resources.gpu_available { "Available" } else { "CPU only" },
        env.resources.max_agents,
        env.resources.max_context_tokens,
        env.fiber.strategy,
        env.fiber.max_share * 100.0,
        env.fiber.health * 100.0,
        env.fiber.fairness * 100.0,
        env.fiber.domain_weights.len(),
        env.hash_registry.total_entries,
        &env.hash_registry.root_hash[..16],
        env.datasets.datasets.len(),
        env.datasets.total_entries,
        env.autodev.repo_url,
        env.autodev.branch,
        if env.autodev.auto_build { "ON" } else { "OFF" },
        if env.autodev.auto_test { "ON" } else { "OFF" },
        if env.autodev.auto_deploy { "ON" } else { "OFF" },
        if env.autodev.auto_doc { "ON" } else { "OFF" },
        if env.autodev.auto_finetune { "ON" } else { "OFF" },
        if env.autodev.auto_recover { "ON" } else { "OFF" },
        env.autodev.max_recover_attempts,
    )
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_paradise() {
        let hw = apophy_universal::detect_hardware();
        let paradise = build_paradise(&hw);

        assert!(!paradise.id.is_nil());
        assert_eq!(paradise.name, "Apophy Paradise");
        assert!(paradise.fiber.active);
        assert_eq!(paradise.fiber.domain_weights.len(), 9);
    }

    #[test]
    fn test_capability_score() {
        let hw = apophy_universal::detect_hardware();
        let paradise = build_paradise(&hw);
        let score = capability_score(&paradise.capabilities);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_hash_registry() {
        let mut registry = HashRegistry::new();

        let hash1 = registry.register("hello world", "test-prompt", ArtifactType::Prompt, "system");
        assert!(!hash1.is_empty());
        assert_eq!(registry.total_entries, 1);

        // Verify
        assert!(registry.verify("hello world", &hash1));
        assert!(!registry.verify("different content", &hash1));

        // Versioned
        let hash2 = registry.register_versioned("hello world v2", "test-prompt-v2", ArtifactType::Prompt, "system", &hash1);
        assert_ne!(hash1, hash2);

        // Version chain
        let chain = registry.version_chain(&hash2);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].hash, hash1);
        assert_eq!(chain[1].hash, hash2);
    }

    #[test]
    fn test_hash_registry_manifest() {
        let mut registry = HashRegistry::new();
        registry.register("a", "p1", ArtifactType::Prompt, "sys");
        registry.register("b", "d1", ArtifactType::Dataset, "sys");
        registry.register("c", "p2", ArtifactType::Prompt, "sys");

        let manifest = registry.manifest();
        assert_eq!(manifest.total_entries, 3);
        assert_eq!(*manifest.by_type.get("Prompt").unwrap(), 2);
        assert_eq!(*manifest.by_type.get("Dataset").unwrap(), 1);
    }

    #[test]
    fn test_dataset_registry() {
        let mut registry = DatasetRegistry::new();

        registry.create_dataset("math-v1", "Math reasoning dataset", "math", "azr-self-play");

        let hash = registry.add_entry(
            "math-v1",
            "What is 2+2?",
            "4",
            true,
            0.95,
            "easy",
        );
        assert!(hash.is_some());

        registry.add_entry(
            "math-v1",
            "Solve: x^2 = 16",
            "x = 4 or x = -4",
            true,
            0.88,
            "medium",
        );

        let stats = registry.stats();
        assert_eq!(stats.total_datasets, 1);
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.total_verified, 2);
        assert!(stats.avg_quality > 0.0);
    }

    #[test]
    fn test_dataset_export_jsonl() {
        let mut registry = DatasetRegistry::new();
        registry.create_dataset("test", "Test dataset", "test", "manual");
        registry.add_entry("test", "Q1", "A1", true, 0.9, "easy");
        registry.add_entry("test", "Q2", "A2", false, 0.3, "hard"); // not verified

        let jsonl = registry.export_jsonl("test").unwrap();
        // Only verified entries are exported
        let lines: Vec<&str> = jsonl.lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Q1"));
    }

    #[test]
    fn test_neutral_fiber_defaults() {
        let hw = apophy_universal::detect_hardware();
        let paradise = build_paradise(&hw);

        assert!(paradise.fiber.active);
        assert_eq!(paradise.fiber.max_share, 0.15);
        assert_eq!(paradise.fiber.fairness, 1.0);
        // All domains should have equal weight
        for weight in paradise.fiber.domain_weights.values() {
            assert_eq!(*weight, 1.0);
        }
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let h1 = compute_hash("test content");
        let h2 = compute_hash("test content");
        let h3 = compute_hash("different");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_format_paradise_status() {
        let hw = apophy_universal::detect_hardware();
        let paradise = build_paradise(&hw);
        let output = format_paradise_status(&paradise);
        assert!(output.contains("APOPHY PARADISE"));
        assert!(output.contains("Neutral Fiber"));
        assert!(output.contains("Hash Registry"));
    }

    #[test]
    fn test_autodev_config() {
        let hw = apophy_universal::detect_hardware();
        let paradise = build_paradise(&hw);
        assert!(paradise.autodev.enabled);
        assert!(paradise.autodev.auto_build);
        assert!(paradise.autodev.auto_test);
        assert!(paradise.autodev.auto_deploy);
        assert!(paradise.autodev.auto_doc);
        assert!(paradise.autodev.auto_finetune);
        assert_eq!(paradise.autodev.targets.len(), 2);
    }
}
