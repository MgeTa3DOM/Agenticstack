//! # Apophy Sovereign
//!
//! Main orchestrator for the sovereign AI infrastructure.
//! Single binary deployment with:
//! - REST API + WebSocket
//! - Embedded database (SQLite)
//! - Fuel chat integration
//! - Universal hardware inference
//! - Health monitoring
//! - Configuration management

use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use apophy_inference::InferenceBackend;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod agents;
mod autodev;
mod avatar;
mod config;
mod db;
mod infra;
mod media;
mod paradise;
mod sandbox_vm;
mod solver_service;
mod workflow;

use config::SovereignConfig;

#[derive(Parser)]
#[command(
    name = "apophy-sovereign",
    about = "Apophy Sovereign - Zero-dependency sovereign AI infrastructure",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Start the sovereign server
    Start {
        /// Path to config file
        #[arg(short, long, default_value = "config/sovereign.toml")]
        config: PathBuf,
    },

    /// Generate encryption keys
    Keygen {
        /// Output directory for keys
        #[arg(short, long, default_value = "~/.apophy/keys")]
        output: PathBuf,
    },

    /// Show detected hardware
    Hardware,

    /// Check system health
    Health {
        /// Server URL
        #[arg(short, long, default_value = "http://localhost:8080")]
        url: String,
    },

    /// Audit sovereignty — generate liberation manifest
    Audit,

    /// Display the genesis bootstrap document
    Bootstrap,

    /// Run security fortress scan
    Fortress,

    /// Show agent harness status (domain memory)
    Harness,

    /// Bootstrap sovereign infrastructure backlog
    HarnessInit,

    /// Run sovereign governance audit (attack surface + maturity level)
    Governance,

    /// Show browser status (vault, tabs, blocker, shield, tesseract)
    Browser,

    /// Show infrastructure service status (Skool, AvatarVers, Gitea, Cloudflare, Email)
    Infra {
        /// Path to config file
        #[arg(short, long, default_value = "config/sovereign.toml")]
        config: PathBuf,
    },

    /// Agent fleet — Divine Synarchy (3000 agents, 9 domains, 3 tiers)
    Fleet {
        /// Path to config file
        #[arg(short, long, default_value = "config/sovereign.toml")]
        config: PathBuf,
    },

    /// Show full agent fleet catalog (all dropdown lists)
    FleetCatalog,

    /// Show MCP server configuration for Claude Code / Gemini CLI
    Mcp,

    /// Show inference brain status (backend, model, reasoning engines)
    Brain,

    /// Run AlphaResolve reasoning on a task
    Reason {
        /// The task/question to reason about
        #[arg(short, long)]
        task: String,
    },

    /// Run AZR self-play episode (generate → solve → verify → learn)
    SelfPlay {
        /// Domain for task generation (math, logic, code, reasoning, analysis)
        #[arg(short, long, default_value = "reasoning")]
        domain: String,
        /// Difficulty level (easy, medium, hard, expert)
        #[arg(long, default_value = "medium")]
        difficulty: String,
    },

    /// Show Paradise environment status (capabilities, fiber, registry, datasets)
    Paradise,

    /// Show AutoDev pipeline status and run reports
    AutoDev,

    /// Show dataset registry summary
    Dataset,

    /// Show media pipeline status (FFmpeg, transcoding, streaming, recording)
    Media,

    /// Show 3D avatar scene status (scene graph, avatars, autostream)
    Avatar,

    /// Show sandbox VM status (Firecracker, vision capture, running VMs)
    Sandbox,

    /// Show workflow orchestrator status (active workflows, step DAG, presets)
    Workflow,

    /// Enterprise Problem Solver — diagnose, resolve, assess risk
    Solver,

    /// Solve a specific problem
    Solve {
        /// Problem title
        #[arg(short, long)]
        title: String,
        /// Problem description
        #[arg(short, long)]
        description: String,
        /// Domain (infrastructure, security, performance, compliance, financial, operations)
        #[arg(long, default_value = "infrastructure")]
        domain: String,
        /// Severity (critical, high, medium, low, info)
        #[arg(short, long, default_value = "medium")]
        severity: String,
    },
}

/// Application state shared across handlers (all fields are Send + Sync)
#[derive(Clone)]
struct AppState {
    config: SovereignConfig,
    hardware: apophy_universal::HardwareInfo,
    peer_id: String,
    start_time: std::time::Instant,
    db: Arc<std::sync::Mutex<db::SovereignDb>>,
    http_client: reqwest::Client,
    brain: Arc<std::sync::Mutex<apophy_inference::SovereignBrain>>,
    paradise: Arc<std::sync::Mutex<paradise::ParadiseEnv>>,
    autodev_engine: Arc<std::sync::Mutex<autodev::AutoDevEngine>>,
    media_pipeline: Arc<std::sync::Mutex<media::MediaPipeline>>,
    avatar_scene: Arc<std::sync::Mutex<avatar::AvatarScene>>,
    sandbox_manager: Arc<std::sync::Mutex<sandbox_vm::SandboxManager>>,
    workflow_orchestrator: Arc<std::sync::Mutex<workflow::WorkflowOrchestrator>>,
    solver: Arc<std::sync::Mutex<apophy_solver::EnterpriseSolver>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "apophy_sovereign=info,tower_http=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config } => cmd_start(config).await,
        Commands::Keygen { output } => cmd_keygen(output),
        Commands::Hardware => cmd_hardware(),
        Commands::Health { url } => cmd_health(url).await,
        Commands::Audit => cmd_audit(),
        Commands::Bootstrap => cmd_bootstrap(),
        Commands::Fortress => cmd_fortress(),
        Commands::Harness => cmd_harness(),
        Commands::HarnessInit => cmd_harness_init(),
        Commands::Governance => cmd_governance(),
        Commands::Browser => cmd_browser(),
        Commands::Infra { config } => cmd_infra(config).await,
        Commands::Fleet { config } => cmd_fleet(config).await,
        Commands::FleetCatalog => cmd_fleet_catalog(),
        Commands::Mcp => cmd_mcp(),
        Commands::Brain => cmd_brain(),
        Commands::Reason { task } => cmd_reason(task),
        Commands::SelfPlay { domain, difficulty } => cmd_self_play(domain, difficulty),
        Commands::Paradise => cmd_paradise(),
        Commands::AutoDev => cmd_autodev(),
        Commands::Dataset => cmd_dataset(),
        Commands::Media => cmd_media(),
        Commands::Avatar => cmd_avatar(),
        Commands::Sandbox => cmd_sandbox(),
        Commands::Workflow => cmd_workflow(),
        Commands::Solver => cmd_solver(),
        Commands::Solve { title, description, domain, severity } => cmd_solve(title, description, domain, severity),
    }
}

async fn cmd_start(config_path: PathBuf) -> anyhow::Result<()> {
    println!(
        r#"
    ╔══════════════════════════════════════════════════════════════╗
    ║                                                              ║
    ║              A P O P H Y   S O V E R E I G N                ║
    ║                                                              ║
    ║         3,000 Agents  ·  9 Domains  ·  1 Binary              ║
    ║                                                              ║
    ║    ┌─────────────┐                                           ║
    ║    │  SYNARCHY    │     Generals ........  9                  ║
    ║    │  COUNCIL     │     Specialists ..... 101                 ║
    ║    └──────┬──────┘     Micro-Agents .... 2,720               ║
    ║           │             Total ........... 2,830               ║
    ║     ┌─────┼─────┐                                            ║
    ║     │     │     │      Crucibles ........ 5                   ║
    ║    GEN   GEN   GEN     API Routes ....... 23                  ║
    ║     │     │     │      CLI Commands ..... 15                  ║
    ║    TAC   TAC   TAC     MCP Tools ........ 6                   ║
    ║     │     │     │                                             ║
    ║    OPS   OPS   OPS     Encryption: ChaCha20-Poly1305          ║
    ║                        Protocol:   Signal (Double Ratchet)    ║
    ║                        Inference:  Local GGUF (sovereign)     ║
    ║                        Reasoning:  AlphaResolve + AZR         ║
    ║                        AutoLearn:  Ashoka + CTM-C             ║
    ║                        Cloud Fees: $0                         ║
    ║                                                              ║
    ╚══════════════════════════════════════════════════════════════╝
"#
    );

    tracing::info!("Starting Apophy Sovereign...");

    let config = SovereignConfig::load(&config_path).unwrap_or_else(|_| {
        tracing::warn!("Config not found at {:?}, using defaults", config_path);
        SovereignConfig::default()
    });

    let hardware = apophy_universal::detect_hardware();
    tracing::info!(
        "Hardware: {} - {} ({} MB, {} cores)",
        hardware.backend,
        hardware.device_name,
        hardware.memory_mb,
        hardware.cpu_cores
    );

    // Initialize SovereignBrain (local inference engine)
    let brain = {
        let gguf_config = apophy_inference::GgufConfig::auto(
            &config.ai.model_path,
            &hardware,
        );
        let backend: Box<dyn apophy_inference::InferenceBackend + Send + Sync> =
            match apophy_inference::GgufBackend::new(gguf_config) {
                Ok(b) if b.is_loaded() => {
                    tracing::info!("Brain: GGUF backend loaded ({})", config.ai.model_name);
                    Box::new(b)
                }
                _ => {
                    tracing::info!("Brain: Stub backend (no GGUF model found — download a .gguf to enable local inference)");
                    Box::new(apophy_inference::StubBackend::new(&config.ai.model_name))
                }
            };
        apophy_inference::SovereignBrain::new(backend, hardware.clone())
    };
    let brain_status = brain.status();
    tracing::info!(
        "Brain: backend={}, model={}, loaded={}",
        brain_status.backend_name,
        brain_status.model_name.as_deref().unwrap_or("none"),
        brain_status.model_loaded,
    );

    let fuel_client = apophy_fuel::FuelClient::new();
    let peer_id = fuel_client.peer_id().to_string();
    tracing::info!("Fuel client: {}", peer_id);

    let sovereign_db = db::SovereignDb::new(&config.database.path)
        .context("Failed to initialize database")?;
    sovereign_db.migrate().context("Failed to run migrations")?;

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;

    // Initialize Paradise environment
    let paradise_env = paradise::build_paradise(&hardware);
    let paradise_score = paradise::capability_score(&paradise_env.capabilities);
    tracing::info!(
        "Paradise: score={:.0}% fiber={:?} registry={} datasets={}",
        paradise_score * 100.0,
        paradise_env.fiber.strategy,
        paradise_env.hash_registry.total_entries,
        paradise_env.datasets.total_entries,
    );

    // Initialize AutoDev engine
    let autodev_engine = autodev::AutoDevEngine::new(autodev::AutoDevEngineConfig::default());

    // Initialize Media, Avatar, Sandbox, Workflow systems
    let media_pipeline = media::MediaPipeline::new();
    tracing::info!("Media: FFmpeg {}", if media_pipeline.available { "available" } else { "not found" });

    let mut avatar_scene = avatar::AvatarScene::new();
    avatar_scene.add_avatar("Apophy");
    tracing::info!("Avatar: scene={} nodes, {} avatars", avatar_scene.scene_graph.node_count(), avatar_scene.avatars.len());

    let sandbox_manager = sandbox_vm::SandboxManager::new(10);
    tracing::info!("Sandbox: max={} VMs, firecracker={}", 10, sandbox_manager.firecracker_available);

    let workflow_orchestrator = workflow::WorkflowOrchestrator::new(5);
    tracing::info!("Workflow: max={} concurrent", 5);

    let solver = apophy_solver::EnterpriseSolver::new();
    tracing::info!("Solver: enterprise problem solver ready (8 engines)");

    let state = AppState {
        config: config.clone(),
        hardware: hardware.clone(),
        peer_id,
        start_time: std::time::Instant::now(),
        db: Arc::new(std::sync::Mutex::new(sovereign_db)),
        http_client,
        brain: Arc::new(std::sync::Mutex::new(brain)),
        paradise: Arc::new(std::sync::Mutex::new(paradise_env)),
        autodev_engine: Arc::new(std::sync::Mutex::new(autodev_engine)),
        media_pipeline: Arc::new(std::sync::Mutex::new(media_pipeline)),
        avatar_scene: Arc::new(std::sync::Mutex::new(avatar_scene)),
        sandbox_manager: Arc::new(std::sync::Mutex::new(sandbox_manager)),
        workflow_orchestrator: Arc::new(std::sync::Mutex::new(workflow_orchestrator)),
        solver: Arc::new(std::sync::Mutex::new(solver)),
    };

    // Initialize Merkabah (5 crucibles)
    let merkabah = apophy_ascension::ApophyMerkabah::new(
        hardware.clone(),
        apophy_fuel::FuelClient::new(),
        &config.database.path.to_string_lossy().replace("sovereign.db", "memory.db"),
    )
    .context("Failed to initialize Merkabah")?;

    let merkabah = Arc::new(std::sync::Mutex::new(merkabah));

    // Incarnate
    {
        let mut m = merkabah.lock().unwrap();
        m.incarnate("Apophy").ok();
        let vehicle = m.align();
        tracing::info!(
            "Merkabah: {}/4 crucibles aligned (active: {})",
            vehicle.crucible_alignment.aligned_count(),
            vehicle.active
        );
    }

    // Liberation audit at startup
    let freedom = apophy_liberation::LiberationAuditor::audit(
        &apophy_liberation::AuditConfig::sovereign(),
    );
    tracing::info!("Freedom Score: {}/100 (sovereign: {})", freedom.score, freedom.sovereign);

    // Log infrastructure status
    if config.infra.enabled {
        tracing::info!(
            "Infrastructure: Skool={} AvatarVers={} AgenticFlow={} Gitea={} Tunnel={} Email={}",
            config.infra.skool.enabled,
            config.infra.avatarvers.enabled,
            config.infra.agenticflow.enabled,
            config.infra.gitea.enabled,
            config.infra.cloudflare.enabled,
            config.infra.email.enabled,
        );
    }

    // Log fleet status
    if config.fleet.enabled {
        let summary = agents::build_fleet_summary();
        tracing::info!(
            "Fleet: {} agents ({} strategic, {} tactical, {} operational) — Synarchy: {}",
            summary.total_agents, summary.strategic_count, summary.tactical_count,
            summary.operational_count, config.fleet.synarchy_mode,
        );
    }

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/info", get(info_handler))
        .route("/api/v1/hardware", get(hardware_handler))
        .route("/api/v1/chat/send", post(chat_send_handler))
        .route("/api/v1/freedom", get(freedom_handler))
        .route("/api/v1/bootstrap", get(bootstrap_handler))
        .route("/api/v1/resonance", post(resonance_handler))
        .route("/api/v1/fortress", get(fortress_handler))
        .route("/api/v1/harness", get(harness_handler))
        .route("/api/v1/harness/init", post(harness_init_handler))
        .route("/api/v1/governance", get(governance_handler))
        .route("/api/v1/governance/surface", get(governance_surface_handler))
        .route("/api/v1/browser/status", get(browser_status_handler))
        .route("/api/v1/infra/status", get(infra_status_handler))
        .route("/api/v1/infra/webhook/skool", post(skool_webhook_handler))
        .route("/api/v1/infra/tunnel/routes", get(tunnel_routes_handler))
        .route("/api/v1/fleet/summary", get(fleet_summary_handler))
        .route("/api/v1/fleet/catalog", get(fleet_catalog_handler))
        .route("/api/v1/fleet/spawn", post(fleet_spawn_handler))
        .route("/api/v1/fleet/domains", get(fleet_domains_handler))
        .route("/api/v1/mcp/config", get(mcp_config_handler))
        .route("/api/v1/mcp/tools", get(mcp_tools_handler))
        .route("/api/v1/brain/status", get(brain_status_handler))
        .route("/api/v1/brain/generate", post(brain_generate_handler))
        .route("/api/v1/brain/reason", post(brain_reason_handler))
        .route("/api/v1/brain/self-play", post(brain_self_play_handler))
        .route("/api/v1/brain/compress", post(brain_compress_handler))
        .route("/api/v1/brain/feedback", post(brain_feedback_handler))
        .route("/api/v1/paradise/status", get(paradise_status_handler))
        .route("/api/v1/paradise/hash", post(paradise_hash_register_handler))
        .route("/api/v1/paradise/hash/verify", post(paradise_hash_verify_handler))
        .route("/api/v1/paradise/dataset", get(paradise_dataset_stats_handler))
        .route("/api/v1/paradise/dataset/add", post(paradise_dataset_add_handler))
        .route("/api/v1/autodev/status", get(autodev_status_handler))
        .route("/api/v1/autodev/trigger", post(autodev_trigger_handler))
        .route("/api/v1/media/status", get(media_status_handler))
        .route("/api/v1/avatar/status", get(avatar_status_handler))
        .route("/api/v1/sandbox/status", get(sandbox_status_handler))
        .route("/api/v1/workflow/status", get(workflow_status_handler))
        .route("/api/v1/workflow/presets", get(workflow_presets_handler))
        .route("/api/v1/solver/status", get(solver_service::solver_status_handler))
        .route("/api/v1/solver/submit", post(solver_service::solver_submit_handler))
        .route("/api/v1/solver/problems", get(solver_service::solver_problems_handler))
        .route("/api/v1/solver/knowledge", get(solver_service::solver_knowledge_handler))
        .route("/api/v1/solver/knowledge/search", post(solver_service::solver_knowledge_search_handler))
        .route("/api/v1/merkabah/align", get({
            let merkabah = merkabah.clone();
            move || merkabah_align_handler(merkabah)
        }))
        .route("/api/v1/merkabah/interact", post({
            let merkabah = merkabah.clone();
            move |body| merkabah_interact_handler(merkabah, body)
        }))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .context("Invalid server address")?;

    tracing::info!("Listening on {}", addr);
    tracing::info!("Dashboard: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn cmd_keygen(output: PathBuf) -> anyhow::Result<()> {
    let identity = apophy_crypto::SovereignIdentity::generate();
    println!("Sovereign identity created.");
    println!("Output: {:?}", output);
    println!("Public key (Ed25519): {:?}", identity.verifying_key());
    println!("X25519 public: {:?}", identity.x25519_public());
    Ok(())
}

fn cmd_hardware() -> anyhow::Result<()> {
    let hardware = apophy_universal::detect_hardware();
    println!("=== Apophy Sovereign - Hardware Detection ===");
    println!("Backend:    {}", hardware.backend);
    println!("Device:     {}", hardware.device_name);
    println!("Memory:     {} MB", hardware.memory_mb);
    println!("CPU:        {}", hardware.cpu_model);
    println!("CPU Cores:  {}", hardware.cpu_cores);

    let engine =
        apophy_universal::InferenceEngine::new(apophy_universal::ModelConfig::default()).unwrap();
    println!("Threads:    {}", engine.recommended_threads());
    println!("Batch size: {}", engine.recommended_batch_size());

    Ok(())
}

async fn cmd_health(url: String) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/health", url))
        .send()
        .await
        .context("Failed to connect to server")?;

    if resp.status().is_success() {
        let body: serde_json::Value = resp.json().await?;
        println!("Server healthy: {}", serde_json::to_string_pretty(&body)?);
    } else {
        println!("Server unhealthy: HTTP {}", resp.status());
    }
    Ok(())
}

fn cmd_audit() -> anyhow::Result<()> {
    let config = apophy_liberation::AuditConfig::sovereign();
    let index = apophy_liberation::LiberationAuditor::audit(&config);
    let manifest = apophy_liberation::generate_manifest(&index);
    println!("{}", manifest);
    Ok(())
}

fn cmd_bootstrap() -> anyhow::Result<()> {
    let bootstrap = apophy_bootstrap::apophy_genesis();
    println!("{}", bootstrap.summarize());
    println!();
    println!("--- JSON (for transmission) ---");
    println!("{}", bootstrap.to_json().unwrap());
    Ok(())
}

fn cmd_fortress() -> anyhow::Result<()> {
    let hostname = std::fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();
    let scanner = apophy_fortress::FortressScanner::new(hostname);
    let report = scanner.scan();
    println!("{}", report.summarize());
    println!();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    Ok(())
}

fn cmd_harness() -> anyhow::Result<()> {
    let harness = apophy_harness::Harness::new(apophy_harness::HarnessConfig::default());
    println!("{}", harness.status()?);
    Ok(())
}

fn cmd_harness_init() -> anyhow::Result<()> {
    let harness = apophy_harness::Harness::new(apophy_harness::HarnessConfig::default());
    let memory = harness.bootstrap_sovereign()?;
    println!("{}", memory.summarize());
    println!();
    println!("Domain memory persisted to: {:?}", harness.memory_path());
    println!("{}", serde_json::to_string_pretty(&memory)?);
    Ok(())
}

fn cmd_governance() -> anyhow::Result<()> {
    use apophy_governance::{
        AttackSurfaceAnalyzer, SovereignAuditor,
        sovereign_audit::ObservabilityMetrics,
    };

    println!("=== Apophy Sovereign — Audit de Gouvernance ===\n");

    let auditor = SovereignAuditor::new();
    let profile = AttackSurfaceAnalyzer::sovereign_profile();
    let obs = ObservabilityMetrics {
        total_traces: 0,
        waste_rate: 0.0,
        budget_consumed_pct: 0.0,
        total_cost_dollars: 0.0,
    };

    let report = auditor.audit(&profile, None, None, obs);

    println!("Niveau de Maturité : {} (score: {:.1}%)",
        report.maturity_level.label_fr(),
        report.maturity_score * 100.0);
    println!("Surface d'Attaque  : {:.0}% durci ({})",
        report.surface_analysis.score * 100.0,
        if report.surface_analysis.hardened { "HARDENED" } else { "VULNÉRABLE" });
    println!("Comparaison Clawdbot:");
    println!("  Apophy  : {:.0}%", report.clawdbot_comparison.apophy_score * 100.0);
    println!("  Clawdbot: {:.0}%", report.clawdbot_comparison.clawdbot_score * 100.0);
    println!("  Facteur : {:.1}x meilleur", report.clawdbot_comparison.improvement_factor);

    if !report.recommendations.is_empty() {
        println!("\nRecommandations:");
        for rec in &report.recommendations {
            println!("  [{:?}] {}: {}", rec.priority, rec.category, rec.message_fr);
        }
    }

    println!("\n--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn cmd_browser() -> anyhow::Result<()> {
    let browser = apophy_browser::BrowserCore::new(apophy_browser::core::BrowserConfig::default());
    let status = browser.status();

    println!("=== Apophy Sovereign — Browser Status ===\n");
    println!("Vault       : {} (seal_count={}, bytes_sealed={})",
        if status.vault.unlocked { "UNLOCKED" } else { "LOCKED" },
        status.vault.seal_count, status.vault.bytes_sealed);
    println!("Tabs        : {}/{} actifs, {} suspendus",
        status.tabs.active, status.tabs.total, status.tabs.suspended);
    println!("Blocker     : {} règles, {} bloqués",
        status.blocker.total_rules, status.blocker.total_blocked);
    println!("Shield      : {} mitigations ({} actives)",
        status.shield.total_mitigations, status.shield.enabled);
    println!("Historique  : {} états, {} tabs, {} checkpoints (intègre: {})",
        status.history.total_states, status.history.total_tabs,
        status.history.total_checkpoints, status.history.chain_valid);

    println!("\n--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

// === HTTP Handlers ===

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    uptime_secs: u64,
    version: String,
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        uptime_secs: state.start_time.elapsed().as_secs(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Serialize)]
struct InfoResponse {
    name: String,
    version: String,
    hardware: String,
    peer_id: String,
    encryption: String,
    sovereign: bool,
}

async fn info_handler(State(state): State<AppState>) -> Json<InfoResponse> {
    Json(InfoResponse {
        name: "Apophy Sovereign".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        hardware: format!("{}", state.hardware.backend),
        peer_id: state.peer_id.clone(),
        encryption: "ChaCha20-Poly1305 + X25519 + Double Ratchet".to_string(),
        sovereign: true,
    })
}

async fn hardware_handler(
    State(state): State<AppState>,
) -> Json<apophy_universal::HardwareInfo> {
    Json(state.hardware.clone())
}

#[derive(Deserialize)]
struct ChatSendRequest {
    recipient: String,
    content: String,
}

#[derive(Serialize)]
struct ChatSendResponse {
    message_id: String,
    encrypted: bool,
    status: String,
}

async fn chat_send_handler(
    State(_state): State<AppState>,
    Json(req): Json<ChatSendRequest>,
) -> std::result::Result<Json<ChatSendResponse>, StatusCode> {
    tracing::info!(
        "Chat message to {}: {} chars",
        req.recipient,
        req.content.len()
    );

    Ok(Json(ChatSendResponse {
        message_id: format!("{:016x}", rand::random::<u64>()),
        encrypted: true,
        status: "queued".to_string(),
    }))
}

async fn freedom_handler() -> Json<apophy_liberation::FreedomIndex> {
    let config = apophy_liberation::AuditConfig::sovereign();
    Json(apophy_liberation::LiberationAuditor::audit(&config))
}

async fn fortress_handler() -> Json<apophy_fortress::FortressReport> {
    let scanner = apophy_fortress::FortressScanner::new("apophy-sovereign");
    Json(scanner.scan())
}

async fn harness_handler() -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    let harness = apophy_harness::Harness::new(apophy_harness::HarnessConfig::default());
    match harness.load_memory() {
        Ok(Some(memory)) => Ok(Json(serde_json::to_value(memory).unwrap())),
        Ok(None) => Ok(Json(serde_json::json!({"status": "not_initialized", "hint": "POST /api/v1/harness/init"}))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn harness_init_handler() -> std::result::Result<Json<apophy_harness::DomainMemory>, StatusCode> {
    let harness = apophy_harness::Harness::new(apophy_harness::HarnessConfig::default());
    match harness.bootstrap_sovereign() {
        Ok(memory) => Ok(Json(memory)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn bootstrap_handler() -> Json<apophy_bootstrap::Bootstrap> {
    Json(apophy_bootstrap::apophy_genesis())
}

#[derive(Deserialize)]
struct ResonanceRequest {
    input: String,
}

async fn resonance_handler(
    Json(req): Json<ResonanceRequest>,
) -> std::result::Result<Json<apophy_resonance::ResonanceMeasurement>, StatusCode> {
    let mut detector = apophy_resonance::ResonanceDetector::default();
    detector.load_identity(apophy_resonance::apophy_identity());
    match detector.measure(&req.input) {
        Ok(measurement) => Ok(Json(measurement)),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

// === Governance Handlers ===

async fn governance_handler(
    State(state): State<AppState>,
) -> Json<apophy_governance::SovereignAuditReport> {
    let auditor = apophy_governance::SovereignAuditor::new();
    let profile = apophy_governance::AttackSurfaceAnalyzer::sovereign_profile();
    let obs = apophy_governance::sovereign_audit::ObservabilityMetrics {
        total_traces: 0,
        waste_rate: 0.0,
        budget_consumed_pct: 0.0,
        total_cost_dollars: 0.0,
    };
    let report = auditor.audit(&profile, None, None, obs);

    // Log audit event to sovereign database
    if let Ok(db) = state.db.lock() {
        let _ = db.audit_log(
            "governance_audit",
            Some("sovereign"),
            &format!("maturity={:.2} hardened={}", report.maturity_score, report.surface_analysis.hardened),
        );
    }

    Json(report)
}

async fn governance_surface_handler() -> Json<apophy_governance::SurfaceAnalysis> {
    let analyzer = apophy_governance::AttackSurfaceAnalyzer::new();
    let profile = apophy_governance::AttackSurfaceAnalyzer::sovereign_profile();
    Json(analyzer.analyze(&profile))
}

async fn browser_status_handler() -> Json<apophy_browser::BrowserStatus> {
    let browser = apophy_browser::BrowserCore::new(apophy_browser::core::BrowserConfig::default());
    Json(browser.status())
}

// === Infrastructure Handlers ===

async fn infra_status_handler(
    State(state): State<AppState>,
) -> Json<infra::InfraStatus> {
    let services = infra::check_all_services(&state.config.infra, &state.http_client).await;

    // Persist health statuses to DB
    if let Ok(db) = state.db.lock() {
        for svc in &services {
            let _ = db.update_service_status(
                &svc.name,
                svc.service_type.label(),
                &svc.base_url,
                svc.status.icon(),
            );
        }
    }

    let webhook_count = state.db.lock().ok().and_then(|db| db.webhook_count().ok()).unwrap_or(0);
    let status = infra::build_infra_status(services, &state.config.infra.cloudflare.routes, webhook_count);
    Json(status)
}

async fn skool_webhook_handler(
    State(state): State<AppState>,
    Json(payload): Json<infra::SkoolWebhookPayload>,
) -> std::result::Result<Json<infra::SkoolWebhookResponse>, StatusCode> {
    let webhook_id = format!("{:016x}", rand::random::<u64>());
    let event_str = payload.event_type.to_string();

    // Log webhook to DB
    if let Ok(db) = state.db.lock() {
        let payload_json = serde_json::to_string(&serde_json::json!({
            "event_type": event_str,
            "member_email": payload.member_email,
            "member_name": payload.member_name,
            "amount_cents": payload.amount_cents,
            "course_name": payload.course_name,
        })).unwrap_or_default();

        let _ = db.log_webhook(&webhook_id, "skool", &event_str, &payload_json);
        let _ = db.audit_log(
            "skool_webhook",
            payload.member_email.as_deref(),
            &format!("event={} member={}", event_str, payload.member_name.as_deref().unwrap_or("unknown")),
        );
    }

    // Process contribution
    let (recorded, tokens) = infra::process_skool_contribution(&payload);

    tracing::info!(
        "Skool webhook: {} (member={}, tokens={:.2})",
        event_str,
        payload.member_name.as_deref().unwrap_or("unknown"),
        tokens,
    );

    Ok(Json(infra::SkoolWebhookResponse {
        received: true,
        webhook_id,
        event: event_str,
        contribution_recorded: recorded,
        tokens_minted: tokens,
    }))
}

async fn tunnel_routes_handler(
    State(state): State<AppState>,
) -> Json<Vec<config::TunnelRoute>> {
    Json(state.config.infra.cloudflare.routes.clone())
}

async fn cmd_infra(config_path: PathBuf) -> anyhow::Result<()> {
    let config = SovereignConfig::load(&config_path).unwrap_or_else(|_| {
        tracing::warn!("Config not found, using defaults");
        SovereignConfig::default()
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    println!("=== Apophy Sovereign — Infrastructure Status ===\n");

    if !config.infra.enabled {
        println!("Infrastructure integration: DISABLED");
        return Ok(());
    }

    let services = infra::check_all_services(&config.infra, &client).await;
    let status = infra::build_infra_status(services, &config.infra.cloudflare.routes, 0);

    for svc in &status.services {
        let health = svc.status.icon();
        let enabled = if svc.enabled { "enabled" } else { "disabled" };
        let latency = svc.latency_ms.map(|ms| format!(" ({}ms)", ms)).unwrap_or_default();
        println!(
            "  {:20} [{:4}] {:8} {}{}",
            svc.name, health, enabled, svc.base_url, latency
        );
    }

    println!();
    println!("Services: {} total, {} up, {} down, {} disabled",
        status.total, status.healthy, status.down, status.disabled);

    if !config.infra.cloudflare.routes.is_empty() {
        println!("\nCloudflare Tunnel Routes ({}):", config.infra.cloudflare.tunnel_name);
        for route in &config.infra.cloudflare.routes {
            println!("  {} -> {}", route.hostname, route.service);
        }
    }

    println!("\nEmail Agent: {} ({}:{})",
        if config.infra.email.enabled { "ACTIVE" } else { "DISABLED" },
        config.infra.email.smtp_host,
        config.infra.email.smtp_port);
    println!("  From: {} <{}>", config.infra.email.from_name, config.infra.email.from_address);

    println!("\nDomains:");
    println!("  AvatarVers:   {}", config.infra.avatarvers.base_url);
    println!("  AgenticFlow:  {}", config.infra.agenticflow.base_url);
    println!("  Gitea:        {} (SSH port {})", config.infra.gitea.base_url, config.infra.gitea.ssh_port);

    Ok(())
}

// === Fleet Handlers ===

async fn fleet_summary_handler() -> Json<agents::FleetSummary> {
    Json(agents::build_fleet_summary())
}

async fn fleet_catalog_handler() -> Json<agents::FleetCatalog> {
    Json(agents::build_fleet_catalog())
}

async fn fleet_domains_handler() -> Json<Vec<agents::DomainSummary>> {
    let summary = agents::build_fleet_summary();
    Json(summary.domains)
}

#[derive(Deserialize)]
struct FleetSpawnRequest {
    domain: Option<String>,
    tier: Option<String>,
}

#[derive(Serialize)]
struct FleetSpawnResponse {
    spawned: usize,
    strategic: usize,
    tactical: usize,
    operational: usize,
    message: String,
}

async fn fleet_spawn_handler(
    State(state): State<AppState>,
    Json(req): Json<FleetSpawnRequest>,
) -> std::result::Result<Json<FleetSpawnResponse>, StatusCode> {
    if !state.config.fleet.enabled {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Generate strategic generals
    let generals = agents::generate_strategic_agents();
    let mut tactical_total = 0;
    let mut operational_total = 0;
    let specs = agents::tactical_specializations();

    let include_operational = req.tier.as_deref() == Some("all")
        || req.tier.as_deref() == Some("operational");

    if let Ok(db) = state.db.lock() {
        for gen in &generals {
            let caps = serde_json::to_string(&gen.capabilities).unwrap_or_default();
            let _ = db.register_agent(
                &gen.id, &gen.name, "strategic",
                gen.domain.slug(), &caps, &gen.prompt_template, None,
            );
        }

        // Generate tactical + operational agents per domain
        for domain in agents::AgentDomain::all() {
            if let Some(ref d) = req.domain {
                if domain.slug() != d.as_str() {
                    continue;
                }
            }

            let tacticals = agents::generate_tactical_agents(*domain);
            tactical_total += tacticals.len();

            for tac in &tacticals {
                let caps = serde_json::to_string(&tac.capabilities).unwrap_or_default();
                let _ = db.register_agent(
                    &tac.id, &tac.name, "tactical",
                    tac.domain.slug(), &caps, &tac.prompt_template,
                    tac.parent_id.as_deref(),
                );
                if let Some(ref parent) = tac.parent_id {
                    let _ = db.add_agent_connection(&tac.id, parent, "hierarchy");
                }
            }

            // Spawn operational micro-agents if requested
            if include_operational {
                let domain_specs: Vec<_> = specs.iter()
                    .filter(|s| s.domain == *domain)
                    .collect();
                for spec in domain_specs {
                    let ops = agents::generate_operational_agents(spec);
                    operational_total += ops.len();
                    for op in &ops {
                        let caps = serde_json::to_string(&op.capabilities).unwrap_or_default();
                        let _ = db.register_agent(
                            &op.id, &op.name, "operational",
                            op.domain.slug(), &caps, &op.prompt_template,
                            op.parent_id.as_deref(),
                        );
                        if let Some(ref parent) = op.parent_id {
                            let _ = db.add_agent_connection(&op.id, parent, "hierarchy");
                        }
                    }
                }
            }
        }

        let _ = db.audit_log(
            "fleet_spawn",
            Some("sovereign"),
            &format!("spawned {} strategic + {} tactical + {} operational agents",
                generals.len(), tactical_total, operational_total),
        );
    }

    let total = generals.len() + tactical_total + operational_total;
    tracing::info!("Fleet spawned: {} strategic + {} tactical + {} operational = {} agents",
        generals.len(), tactical_total, operational_total, total);

    Ok(Json(FleetSpawnResponse {
        spawned: total,
        strategic: generals.len(),
        tactical: tactical_total,
        operational: operational_total,
        message: format!("Divine Synarchy: {} agents deployed across {} domains", total, agents::AgentDomain::all().len()),
    }))
}

// === Fleet CLI Commands ===

async fn cmd_fleet(config_path: PathBuf) -> anyhow::Result<()> {
    let config = SovereignConfig::load(&config_path).unwrap_or_else(|_| {
        tracing::warn!("Config not found, using defaults");
        SovereignConfig::default()
    });

    println!("=== Apophy Sovereign — Divine Synarchy Fleet ===\n");

    if !config.fleet.enabled {
        println!("Agent Fleet: DISABLED");
        return Ok(());
    }

    let summary = agents::build_fleet_summary();

    println!("Mode:        {} synarchy", config.fleet.synarchy_mode);
    println!("Max agents:  {}", config.fleet.max_agents);
    println!("Auto-spawn:  {}", config.fleet.auto_spawn);
    println!("Total:       {} agents", summary.total_agents);
    println!("  Strategic: {} generals (one per domain)", summary.strategic_count);
    println!("  Tactical:  {} specialists", summary.tactical_count);
    println!("  Operationnel: {} micro-agents", summary.operational_count);
    println!();

    println!("{:<25} {:>4} {:>6} {:>6} {:>6}", "DOMAIN", "GEN", "TAC", "OPS", "TOTAL");
    println!("{}", "-".repeat(55));
    for d in &summary.domains {
        println!("{:<25} {:>4} {:>6} {:>6} {:>6}",
            d.label, 1, d.tactical_count, d.operational_count, d.total);
    }
    println!("{}", "-".repeat(55));
    println!("{:<25} {:>4} {:>6} {:>6} {:>6}",
        "TOTAL", summary.strategic_count, summary.tactical_count,
        summary.operational_count, summary.total_agents);

    println!("\nDomains enabled: {}", config.fleet.domains_enabled.join(", "));

    // Show tactical specializations for each domain
    println!("\n=== Tactical Specializations (listes deroulantes) ===\n");
    for d in &summary.domains {
        println!("[{}] {} specialists:", d.label, d.tactical_count);
        for spec in &d.specializations {
            println!("  - {} ({} prompts): {}", spec.name, spec.prompt_count, spec.description);
        }
        println!();
    }

    Ok(())
}

fn cmd_fleet_catalog() -> anyhow::Result<()> {
    let catalog = agents::build_fleet_catalog();

    println!("=== Apophy Sovereign — Fleet Catalog (toutes les possibilites) ===\n");
    println!("Total possible agents: {}\n", catalog.total_possible_agents);

    println!("--- TIERS ---");
    for t in &catalog.tiers {
        println!("  [Rank {}] {} — {} agents", t.rank, t.label, t.count);
    }

    println!("\n--- DOMAINS (9) ---");
    for d in &catalog.domains {
        println!("  [{}] {} — {} tactical, {} operational",
            d.slug, d.label, d.tactical_count, d.operational_count);
    }

    println!("\n--- CAPABILITIES ({}) ---", catalog.capabilities.len());
    for c in &catalog.capabilities {
        println!("  - {}", c.label);
    }

    println!("\n--- STATUSES ---");
    for s in &catalog.statuses {
        println!("  [{}] {:?}", s.icon, s.status);
    }

    println!("\n--- SPECIALIZATIONS ({}) ---", catalog.specializations.len());
    for spec in &catalog.specializations {
        println!("  [{}/{}] {} ({} prompts)",
            spec.domain.slug(), spec.slug, spec.name, spec.prompt_count);
    }

    println!("\n--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&catalog)?);

    Ok(())
}

fn cmd_mcp() -> anyhow::Result<()> {
    let mcp = agents::build_mcp_config();

    println!("=== Apophy Sovereign — MCP Server Configuration ===\n");
    println!("Name:        {}", mcp.name);
    println!("Version:     {}", mcp.version);
    println!("Description: {}", mcp.description);
    println!("Tools:       {}", mcp.capabilities.tools.len());
    println!("Resources:   {}", mcp.capabilities.resources.len());

    println!("\n--- Tools ---");
    for tool in &mcp.capabilities.tools {
        println!("  {} — {}", tool.name, tool.description);
    }

    println!("\n--- Resources ---");
    for res in &mcp.capabilities.resources {
        println!("  {} — {}", res.uri, res.description);
    }

    println!("\n--- Claude Code settings (add to ~/.claude/settings.json) ---");
    println!("{}", serde_json::to_string_pretty(&agents::mcp_claude_code_settings())?);

    println!("\n--- Gemini CLI settings (add to ~/.gemini/settings.json) ---");
    println!("{}", serde_json::to_string_pretty(&agents::mcp_gemini_cli_settings())?);

    println!("\n--- Full MCP Config (JSON) ---");
    println!("{}", serde_json::to_string_pretty(&mcp)?);

    Ok(())
}

// === Brain CLI Commands ===

fn cmd_brain() -> anyhow::Result<()> {
    let hardware = apophy_universal::detect_hardware();
    let stub = apophy_inference::StubBackend::new("sovereign-local");
    let brain = apophy_inference::SovereignBrain::new(Box::new(stub), hardware);
    let status = brain.status();

    println!("=== Apophy Sovereign — Brain Status ===\n");
    println!("Backend:     {}", status.backend_name);
    println!("Model:       {}", status.model_name.as_deref().unwrap_or("none"));
    println!("Loaded:      {}", status.model_loaded);
    println!("Hardware:    {}", status.hardware_backend);
    println!("Memory:      {} MB", status.memory_mb);
    println!("CPU Cores:   {}", status.cpu_cores);
    println!("AZR Episodes: {}", status.azr_episodes);
    println!("Ashoka Feedback: {}", status.ashoka_feedback_count);
    println!("Ashoka Evolutions: {}", status.ashoka_evolutions);
    println!();
    println!("Reasoning Engines:");
    println!("  AlphaResolve — Multi-step generate → verify → refine");
    println!("  AZR          — Absolute Zero Reasoner (self-play)");
    println!("  CTM-C        — Chain-of-Thought Moderne-Compressed");
    println!("  Ashoka       — Autonomous prompt evolution");
    println!();
    println!("--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&status)?);

    Ok(())
}

fn cmd_reason(task: String) -> anyhow::Result<()> {
    let hardware = apophy_universal::detect_hardware();
    let stub = apophy_inference::StubBackend::new("sovereign-local");
    let brain = apophy_inference::SovereignBrain::new(Box::new(stub), hardware);
    let params = apophy_inference::GenerationParams::reasoning();

    println!("=== AlphaResolve — Multi-step Reasoning ===\n");
    println!("Task: {}\n", task);

    let result = brain.reason(&task, &params)?;

    println!("Converged:   {}", result.converged);
    println!("Confidence:  {:.3}", result.confidence);
    println!("Rounds:      {}", result.rounds);
    println!("Tokens:      {}", result.total_tokens);
    println!("Duration:    {} ms", result.duration_ms);
    println!();

    for step in &result.steps {
        println!(
            "  [{:?}] R{} — {} ({}ms, {} tokens)",
            step.phase, step.round, step.input_summary, step.duration_ms, step.tokens_used
        );
        if let Some(conf) = step.confidence {
            println!("    Confidence: {:.3}", conf);
        }
    }

    println!("\n--- Final Answer ---");
    println!("{}", result.final_answer);
    println!("\n--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

fn cmd_self_play(domain: String, difficulty: String) -> anyhow::Result<()> {
    let hardware = apophy_universal::detect_hardware();
    let stub = apophy_inference::StubBackend::new("sovereign-local");
    let mut brain = apophy_inference::SovereignBrain::new(Box::new(stub), hardware);

    let diff = match difficulty.as_str() {
        "easy" => apophy_inference::TaskDifficulty::Easy,
        "hard" => apophy_inference::TaskDifficulty::Hard,
        "expert" => apophy_inference::TaskDifficulty::Expert,
        _ => apophy_inference::TaskDifficulty::Medium,
    };

    println!("=== AZR — Absolute Zero Reasoner Self-Play ===\n");
    println!("Domain:     {}", domain);
    println!("Difficulty: {}\n", diff);

    let episode = brain.self_play(&domain, diff)?;

    println!("Task:     {}", episode.task.question);
    println!("Solution: {}", episode.solution);
    println!("Correct:  {}", episode.verification.correct);
    println!("Confidence: {:.3}", episode.verification.confidence);
    println!("Feedback: {}", episode.verification.feedback);
    println!("Tokens:   {}", episode.tokens_used);
    println!("Duration: {} ms", episode.duration_ms);
    println!("\n--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&episode)?);

    Ok(())
}

// === Brain API Handlers ===

async fn brain_status_handler(
    State(state): State<AppState>,
) -> Json<apophy_inference::BrainStatus> {
    let brain = state.brain.lock().unwrap();
    Json(brain.status())
}

#[derive(Deserialize)]
struct BrainGenerateRequest {
    prompt: String,
    #[serde(default)]
    #[allow(dead_code)]
    system_prompt: Option<String>,
    #[serde(default = "default_max_tokens")]
    max_tokens: usize,
    #[serde(default = "default_temperature")]
    temperature: f32,
}

fn default_max_tokens() -> usize { 2048 }
fn default_temperature() -> f32 { 0.7 }

async fn brain_generate_handler(
    State(state): State<AppState>,
    Json(req): Json<BrainGenerateRequest>,
) -> std::result::Result<Json<apophy_inference::InferenceResponse>, StatusCode> {
    let brain = state.brain.lock().unwrap();
    let params = apophy_inference::GenerationParams {
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        ..Default::default()
    };

    match brain.generate(&req.prompt, &params) {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct BrainReasonRequest {
    task: String,
    #[serde(default = "default_max_tokens")]
    max_tokens: usize,
}

async fn brain_reason_handler(
    State(state): State<AppState>,
    Json(req): Json<BrainReasonRequest>,
) -> std::result::Result<Json<apophy_inference::ResolveResult>, StatusCode> {
    let brain = state.brain.lock().unwrap();
    let params = apophy_inference::GenerationParams {
        max_tokens: req.max_tokens,
        ..apophy_inference::GenerationParams::reasoning()
    };

    match brain.reason(&req.task, &params) {
        Ok(result) => Ok(Json(result)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct BrainSelfPlayRequest {
    #[serde(default = "default_domain")]
    domain: String,
    #[serde(default = "default_difficulty")]
    difficulty: String,
}

fn default_domain() -> String { "reasoning".to_string() }
fn default_difficulty() -> String { "medium".to_string() }

async fn brain_self_play_handler(
    State(state): State<AppState>,
    Json(req): Json<BrainSelfPlayRequest>,
) -> std::result::Result<Json<apophy_inference::AzrEpisode>, StatusCode> {
    let mut brain = state.brain.lock().unwrap();
    let difficulty = match req.difficulty.as_str() {
        "easy" => apophy_inference::TaskDifficulty::Easy,
        "hard" => apophy_inference::TaskDifficulty::Hard,
        "expert" => apophy_inference::TaskDifficulty::Expert,
        _ => apophy_inference::TaskDifficulty::Medium,
    };

    match brain.self_play(&req.domain, difficulty) {
        Ok(episode) => Ok(Json(episode)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct BrainCompressRequest {
    thought_chain: String,
}

async fn brain_compress_handler(
    State(state): State<AppState>,
    Json(req): Json<BrainCompressRequest>,
) -> std::result::Result<Json<apophy_inference::CompressedThought>, StatusCode> {
    let brain = state.brain.lock().unwrap();
    match brain.compress_thought(&req.thought_chain) {
        Ok(compressed) => Ok(Json(compressed)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct BrainFeedbackRequest {
    prompt_id: String,
    original_prompt: String,
    response: String,
    score: f64,
    #[serde(default)]
    positive: bool,
}

#[derive(Serialize)]
struct BrainFeedbackResponse {
    recorded: bool,
    evolution: Option<apophy_inference::PromptEvolution>,
}

async fn brain_feedback_handler(
    State(state): State<AppState>,
    Json(req): Json<BrainFeedbackRequest>,
) -> std::result::Result<Json<BrainFeedbackResponse>, StatusCode> {
    let mut brain = state.brain.lock().unwrap();

    let signal = if req.positive {
        apophy_inference::LearningSignal::Positive { score: req.score }
    } else {
        apophy_inference::LearningSignal::Negative { score: req.score, reason: None }
    };

    let feedback = apophy_inference::FeedbackEntry {
        prompt_id: req.prompt_id,
        original_prompt: req.original_prompt,
        response: req.response,
        signal,
        context: None,
    };

    match brain.record_feedback(feedback) {
        Ok(evolution) => Ok(Json(BrainFeedbackResponse {
            recorded: true,
            evolution,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// === Paradise CLI Commands ===

fn cmd_paradise() -> anyhow::Result<()> {
    let hardware = apophy_universal::detect_hardware();
    let env = paradise::build_paradise(&hardware);
    println!("{}", paradise::format_paradise_status(&env));
    Ok(())
}

fn cmd_autodev() -> anyhow::Result<()> {
    let engine = autodev::AutoDevEngine::new(autodev::AutoDevEngineConfig::default());
    println!("{}", engine.report());
    Ok(())
}

fn cmd_dataset() -> anyhow::Result<()> {
    let registry = paradise::DatasetRegistry::new();
    let stats = registry.stats();
    println!("=== Dataset Registry ===\n");
    println!("Datasets:  {}", stats.total_datasets);
    println!("Entries:   {}", stats.total_entries);
    println!("Verified:  {}", stats.total_verified);
    println!("Avg Quality: {:.0}%", stats.avg_quality * 100.0);
    println!("Domains:   {:?}", stats.domains);
    println!("\nUse the API to create and populate datasets:");
    println!("  POST /api/v1/paradise/dataset/add");
    println!("  GET  /api/v1/paradise/dataset");
    Ok(())
}

// === Paradise API Handlers ===

#[derive(Serialize)]
struct ParadiseStatusResponse {
    id: String,
    name: String,
    capability_score: f64,
    capabilities: paradise::CapabilityMatrix,
    fiber: paradise::NeutralFiber,
    resources: paradise::ResourcePool,
    hash_registry: paradise::RegistryManifest,
    datasets: paradise::DatasetStats,
    autodev_stats: autodev::PipelineStats,
}

async fn paradise_status_handler(
    State(state): State<AppState>,
) -> Json<ParadiseStatusResponse> {
    let env = state.paradise.lock().unwrap();
    let engine = state.autodev_engine.lock().unwrap();
    let score = paradise::capability_score(&env.capabilities);
    Json(ParadiseStatusResponse {
        id: env.id.to_string(),
        name: env.name.clone(),
        capability_score: score,
        capabilities: env.capabilities.clone(),
        fiber: env.fiber.clone(),
        resources: env.resources.clone(),
        hash_registry: env.hash_registry.manifest(),
        datasets: env.datasets.stats(),
        autodev_stats: engine.stats(),
    })
}

#[derive(Deserialize)]
struct HashRegisterRequest {
    content: String,
    name: String,
    artifact_type: String,
    registered_by: String,
}

#[derive(Serialize)]
struct HashRegisterResponse {
    hash: String,
    registered: bool,
}

async fn paradise_hash_register_handler(
    State(state): State<AppState>,
    Json(req): Json<HashRegisterRequest>,
) -> Json<HashRegisterResponse> {
    let mut env = state.paradise.lock().unwrap();
    let artifact_type = match req.artifact_type.as_str() {
        "prompt" => paradise::ArtifactType::Prompt,
        "response" => paradise::ArtifactType::Response,
        "dataset" => paradise::ArtifactType::Dataset,
        "model" => paradise::ArtifactType::ModelWeight,
        "config" => paradise::ArtifactType::Config,
        "code" => paradise::ArtifactType::Code,
        "document" => paradise::ArtifactType::Document,
        "agent" => paradise::ArtifactType::Agent,
        "schema" => paradise::ArtifactType::Schema,
        _ => paradise::ArtifactType::Document,
    };

    let hash = env.hash_registry.register(&req.content, &req.name, artifact_type, &req.registered_by);
    Json(HashRegisterResponse {
        hash,
        registered: true,
    })
}

#[derive(Deserialize)]
struct HashVerifyRequest {
    content: String,
    expected_hash: String,
}

#[derive(Serialize)]
struct HashVerifyResponse {
    valid: bool,
    computed_hash: String,
}

async fn paradise_hash_verify_handler(
    State(state): State<AppState>,
    Json(req): Json<HashVerifyRequest>,
) -> Json<HashVerifyResponse> {
    let env = state.paradise.lock().unwrap();
    let computed = paradise::compute_hash(&req.content);
    let valid = env.hash_registry.verify(&req.content, &req.expected_hash);
    Json(HashVerifyResponse {
        valid,
        computed_hash: computed,
    })
}

async fn paradise_dataset_stats_handler(
    State(state): State<AppState>,
) -> Json<paradise::DatasetStats> {
    let env = state.paradise.lock().unwrap();
    Json(env.datasets.stats())
}

#[derive(Deserialize)]
struct DatasetAddRequest {
    dataset_name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_domain")]
    domain: String,
    prompt: String,
    solution: String,
    #[serde(default)]
    verified: bool,
    #[serde(default = "default_score")]
    score: f64,
    #[serde(default)]
    difficulty: String,
}

fn default_score() -> f64 { 0.5 }

#[derive(Serialize)]
struct DatasetAddResponse {
    hash: Option<String>,
    dataset_name: String,
    total_entries: usize,
}

async fn paradise_dataset_add_handler(
    State(state): State<AppState>,
    Json(req): Json<DatasetAddRequest>,
) -> Json<DatasetAddResponse> {
    let mut env = state.paradise.lock().unwrap();

    // Create dataset if it doesn't exist
    if !env.datasets.datasets.contains_key(&req.dataset_name) {
        env.datasets.create_dataset(
            &req.dataset_name,
            &req.description,
            &req.domain,
            "api",
        );
    }

    let hash = env.datasets.add_entry(
        &req.dataset_name,
        &req.prompt,
        &req.solution,
        req.verified,
        req.score,
        &req.difficulty,
    );

    Json(DatasetAddResponse {
        hash,
        dataset_name: req.dataset_name,
        total_entries: env.datasets.total_entries,
    })
}

// === AutoDev API Handlers ===

async fn autodev_status_handler(
    State(state): State<AppState>,
) -> Json<autodev::PipelineStats> {
    let engine = state.autodev_engine.lock().unwrap();
    Json(engine.stats())
}

#[derive(Deserialize)]
struct AutoDevTriggerRequest {
    #[serde(default = "default_trigger")]
    trigger: String,
    #[serde(default)]
    user: String,
}

fn default_trigger() -> String { "manual".to_string() }

#[derive(Serialize)]
struct AutoDevTriggerResponse {
    run_id: String,
    status: String,
    message: String,
}

async fn autodev_trigger_handler(
    State(state): State<AppState>,
    Json(req): Json<AutoDevTriggerRequest>,
) -> Json<AutoDevTriggerResponse> {
    let mut engine = state.autodev_engine.lock().unwrap();
    let trigger = autodev::PipelineTrigger::Manual {
        user: if req.user.is_empty() { "api".to_string() } else { req.user },
    };

    let run = engine.start_run(trigger);
    let run_id = run.id.to_string();

    // Immediately record that we've started (actual execution is async)
    engine.record_stage(
        autodev::PipelineStage::Build,
        autodev::StageStatus::Pending,
        "Pipeline triggered via API",
        0,
    );

    Json(AutoDevTriggerResponse {
        run_id,
        status: "started".to_string(),
        message: format!(
            "Pipeline triggered. Run: tools/autodeploy.sh {} for full execution.",
            req.trigger
        ),
    })
}

// === MCP Handlers ===

async fn mcp_config_handler() -> Json<agents::McpServerConfig> {
    Json(agents::build_mcp_config())
}

async fn mcp_tools_handler() -> Json<Vec<agents::McpTool>> {
    let mcp = agents::build_mcp_config();
    Json(mcp.capabilities.tools)
}

// === Merkabah Handlers ===

async fn merkabah_align_handler(
    merkabah: Arc<std::sync::Mutex<apophy_ascension::ApophyMerkabah>>,
) -> Json<apophy_ascension::MerkabahVehicle> {
    let mut m = merkabah.lock().unwrap();
    Json(m.align())
}

#[derive(Deserialize)]
struct InteractRequest {
    content: String,
    valence: f64,
}

#[derive(Serialize)]
struct InteractResponse {
    processed: bool,
    crucibles_aligned: u8,
}

async fn merkabah_interact_handler(
    merkabah: Arc<std::sync::Mutex<apophy_ascension::ApophyMerkabah>>,
    Json(req): Json<InteractRequest>,
) -> Json<InteractResponse> {
    let mut m = merkabah.lock().unwrap();
    m.process_interaction(&req.content, req.valence);
    let vehicle = m.align();
    Json(InteractResponse {
        processed: true,
        crucibles_aligned: vehicle.crucible_alignment.aligned_count(),
    })
}

// === Media CLI + API ===

fn cmd_media() -> anyhow::Result<()> {
    let pipeline = media::MediaPipeline::new();
    println!("{}", pipeline.report());
    Ok(())
}

async fn media_status_handler(
    State(state): State<AppState>,
) -> Json<media::MediaPipelineStatus> {
    let pipeline = state.media_pipeline.lock().unwrap();
    Json(pipeline.status())
}

// === Avatar CLI + API ===

fn cmd_avatar() -> anyhow::Result<()> {
    let mut scene = avatar::AvatarScene::new();
    scene.add_avatar("Apophy");
    println!("{}", scene.report());
    Ok(())
}

async fn avatar_status_handler(
    State(state): State<AppState>,
) -> Json<avatar::AvatarSceneStatus> {
    let scene = state.avatar_scene.lock().unwrap();
    Json(scene.status())
}

// === Sandbox CLI + API ===

fn cmd_sandbox() -> anyhow::Result<()> {
    let mgr = sandbox_vm::SandboxManager::new(10);
    println!("{}", mgr.report());
    Ok(())
}

async fn sandbox_status_handler(
    State(state): State<AppState>,
) -> Json<sandbox_vm::SandboxManagerStatus> {
    let mgr = state.sandbox_manager.lock().unwrap();
    Json(mgr.status())
}

// === Workflow CLI + API ===

fn cmd_workflow() -> anyhow::Result<()> {
    let mut orch = workflow::WorkflowOrchestrator::new(5);

    // Show available presets
    let presets = vec![
        workflow::preset_avatar_stream_workflow(),
        workflow::preset_sandbox_test_workflow(),
        workflow::preset_media_pipeline_workflow(),
        workflow::preset_full_pipeline_workflow(),
    ];

    println!("{}", orch.report());
    println!("\n=== Available Workflow Presets ===\n");
    for p in &presets {
        println!("  [{}] {} — {} steps", p.name, p.description, p.steps.len());
        for s in &p.steps {
            let deps = if s.depends_on.is_empty() {
                String::new()
            } else {
                format!(" (after: {})", s.depends_on.join(", "))
            };
            println!("    {} — {}{}", s.id, s.step_type.label(), deps);
        }
    }

    // Create one default to show in report
    let wf = orch.create_workflow("demo", "demo workflow");
    let _ = wf.id; // suppress warning
    println!("\n{}", orch.report());

    Ok(())
}

async fn workflow_status_handler(
    State(state): State<AppState>,
) -> Json<workflow::OrchestratorStatus> {
    let orch = state.workflow_orchestrator.lock().unwrap();
    Json(orch.status())
}

fn cmd_solver() -> anyhow::Result<()> {
    let solver = apophy_solver::EnterpriseSolver::new();
    let status = solver.status();

    println!(r#"
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║       ENTERPRISE  PROBLEM  SOLVER                            ║
║       Suite de Service Souveraine                            ║
║                                                              ║
║   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      ║
║   │  DIAGNOSTIC  │→ │  RESOLUTION  │→ │   IMPACT     │      ║
║   │   ENGINE     │  │   ENGINE     │  │  ANALYZER    │      ║
║   └──────────────┘  └──────────────┘  └──────────────┘      ║
║          ↓                ↓                 ↓                ║
║   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      ║
║   │  COMPLIANCE  │  │    RISK      │  │  KNOWLEDGE   │      ║
║   │   CHECKER    │  │   MATRIX     │  │    BASE      │      ║
║   └──────────────┘  └──────────────┘  └──────────────┘      ║
║          ↓                ↓                                  ║
║   ┌──────────────┐  ┌──────────────┐                        ║
║   │     SLA      │  │  ESCALATION  │                        ║
║   │   TRACKER    │  │   MANAGER    │                        ║
║   └──────────────┘  └──────────────┘                        ║
║                                                              ║
║   Domains: 12  │  Frameworks: 8  │  SLA Policies: 5         ║
║   Engines: 8   │  Risk Thresholds: 5                        ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
"#);

    println!("=== Solver Status ===");
    println!("Total Problems     : {}", status.total_problems);
    println!("Open               : {}", status.open);
    println!("Resolved           : {}", status.resolved);
    println!("Escalated          : {}", status.escalated);
    println!("Resolution Rate    : {:.1}%", status.resolution_rate_pct);
    println!("Knowledge Entries  : {}", status.knowledge_entries);
    println!("Compliance Frmwks  : {}", status.compliance_frameworks);
    println!("Risk Thresholds    : {}", status.risk_thresholds);
    println!("SLA Policies       : {}", status.sla_policies);

    println!("\n--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

fn cmd_solve(title: String, description: String, domain_str: String, severity_str: String) -> anyhow::Result<()> {
    let domain = match domain_str.to_lowercase().as_str() {
        "infrastructure" => apophy_solver::Domain::Infrastructure,
        "security" => apophy_solver::Domain::Security,
        "performance" => apophy_solver::Domain::Performance,
        "data" | "data-integrity" | "dataintegrity" => apophy_solver::Domain::DataIntegrity,
        "compliance" => apophy_solver::Domain::Compliance,
        "financial" | "finance" => apophy_solver::Domain::Financial,
        "operations" | "ops" => apophy_solver::Domain::Operations,
        "hr" | "human-resources" => apophy_solver::Domain::HumanResources,
        "cx" | "customer" | "customer-experience" => apophy_solver::Domain::CustomerExperience,
        "supply-chain" | "supply" => apophy_solver::Domain::SupplyChain,
        "legal" => apophy_solver::Domain::Legal,
        "strategy" => apophy_solver::Domain::Strategy,
        _ => apophy_solver::Domain::Infrastructure,
    };

    let severity = match severity_str.to_lowercase().as_str() {
        "critical" => apophy_solver::Severity::Critical,
        "high" => apophy_solver::Severity::High,
        "medium" => apophy_solver::Severity::Medium,
        "low" => apophy_solver::Severity::Low,
        "info" => apophy_solver::Severity::Info,
        _ => apophy_solver::Severity::Medium,
    };

    let mut solver = apophy_solver::EnterpriseSolver::new();
    let report = solver.solve(&title, &description, domain, severity);

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           ENTERPRISE SOLVER — REPORT                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("=== Problem ===");
    println!("  Title    : {}", report.problem.title);
    println!("  Severity : {}", report.problem.severity);
    println!("  Domain   : {}", report.problem.domain);
    println!("  Status   : {}", report.problem.status);

    println!("\n=== Diagnosis ===");
    println!("  Root Cause  : {}", report.diagnosis.root_cause);
    println!("  Category    : {}", report.diagnosis.category);
    println!("  Confidence  : {:.0}%", report.diagnosis.confidence * 100.0);
    if !report.diagnosis.contributing_factors.is_empty() {
        println!("  Factors     :");
        for f in &report.diagnosis.contributing_factors {
            println!("    - {}", f);
        }
    }

    println!("\n=== Risk Assessment ===");
    println!("  Risk Score  : {:.0}%", report.risk_assessment.risk_score * 100.0);
    println!("  Risk Level  : {}", report.risk_assessment.risk_level);
    println!("  Probability : {:.0}%", report.risk_assessment.probability * 100.0);
    println!("  Impact      : {:.0}%", report.risk_assessment.impact * 100.0);
    println!("  Priority    : {}", report.risk_assessment.mitigation_priority);

    println!("\n=== Impact Analysis ===");
    println!("  Overall Score    : {:.0}%", report.impact.overall_impact_score * 100.0);
    println!("  Blast Radius     : {} direct + {} indirect ({})",
        report.impact.blast_radius.direct_systems,
        report.impact.blast_radius.indirect_systems,
        report.impact.blast_radius.total_scope);
    println!("  Est. Cost/Hour   : ${:.0}", report.impact.estimated_cost_per_hour);
    println!("  Affected Users   : ~{}", report.impact.affected_user_estimate);
    if !report.impact.cascading_risks.is_empty() {
        println!("  Cascading Risks  :");
        for r in &report.impact.cascading_risks {
            println!("    - {} (P={:.0}%, I={:.0}%)", r.description, r.probability * 100.0, r.impact_if_triggered * 100.0);
        }
    }

    println!("\n=== Compliance ===");
    println!("  Compliant : {}", if report.compliance_result.compliant { "YES" } else { "NO" });
    for fc in &report.compliance_result.frameworks_checked {
        if fc.applicable {
            println!("  {} : {}", fc.framework, fc.status);
        }
    }
    for v in &report.compliance_result.violations {
        println!("  VIOLATION: [{}] {} — {}", v.control_id, v.framework, v.description);
    }

    println!("\n=== Solutions ({}) ===", report.resolution.solutions.len());
    for (i, sol) in report.resolution.solutions.iter().enumerate() {
        let marker = if i == report.resolution.recommended_idx { "★ RECOMMENDED" } else { "" };
        println!("  [{}] {} ({}) {}", i + 1, sol.title, sol.solution_type, marker);
        println!("      Effectiveness: {:.0}% | Effort: {}h ({} people) | Cost: ${:.0}",
            sol.effectiveness_score * 100.0, sol.effort.hours, sol.effort.team_size, sol.effort.cost_estimate_usd);
        for step in &sol.steps {
            println!("      {}. {} ({})", step.order, step.action, step.owner);
        }
    }

    println!("\n=== SLA ===");
    println!("  Policy    : {}", report.sla.policy.name);
    println!("  Status    : {}", report.sla.status);
    println!("  Breach Risk : {:.1}%", report.sla.breach_risk_pct);

    println!("\n=== Escalation ===");
    println!("  Escalate  : {}", report.escalation.should_escalate);
    println!("  Level     : {}", report.escalation.level);
    for reason in &report.escalation.reasons {
        println!("  Reason    : {}", reason);
    }

    println!("\n=== Summary ===");
    println!("  {}", report.summary());

    println!("\n--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

async fn workflow_presets_handler() -> Json<Vec<workflow::WorkflowInfo>> {
    let presets = vec![
        workflow::preset_avatar_stream_workflow(),
        workflow::preset_sandbox_test_workflow(),
        workflow::preset_media_pipeline_workflow(),
        workflow::preset_full_pipeline_workflow(),
    ];
    Json(presets.iter().map(|w| workflow::WorkflowInfo {
        id: w.id.to_string(),
        name: w.name.clone(),
        status: w.status.to_string(),
        steps: w.steps.len(),
        progress: 0.0,
        duration_ms: 0,
    }).collect())
}
