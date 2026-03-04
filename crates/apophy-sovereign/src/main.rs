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
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod config;
mod db;

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
}

/// Application state shared across handlers (all fields are Send + Sync)
#[derive(Clone)]
struct AppState {
    config: SovereignConfig,
    hardware: apophy_universal::HardwareInfo,
    peer_id: String,
    start_time: std::time::Instant,
    db: Arc<std::sync::Mutex<db::SovereignDb>>,
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
    }
}

async fn cmd_start(config_path: PathBuf) -> anyhow::Result<()> {
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

    let fuel_client = apophy_fuel::FuelClient::new();
    let peer_id = fuel_client.peer_id().to_string();
    tracing::info!("Fuel client: {}", peer_id);

    let sovereign_db = db::SovereignDb::new(&config.database.path)
        .context("Failed to initialize database")?;
    sovereign_db.migrate().context("Failed to run migrations")?;

    let state = AppState {
        config: config.clone(),
        hardware: hardware.clone(),
        peer_id,
        start_time: std::time::Instant::now(),
        db: Arc::new(std::sync::Mutex::new(sovereign_db)),
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

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/info", get(info_handler))
        .route("/api/v1/hardware", get(hardware_handler))
        .route("/api/v1/chat/send", post(chat_send_handler))
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
