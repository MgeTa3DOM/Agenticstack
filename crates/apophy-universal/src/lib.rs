//! # Apophy Universal
//!
//! Hardware-agnostic AI inference layer.
//! Auto-detects and optimizes for any hardware:
//! - NVIDIA CUDA (RTX, Tesla, Quadro)
//! - AMD ROCm (RX, Radeon Pro, MI)
//! - Intel oneAPI (Arc, Iris, Xeon)
//! - Apple Neural Engine (M1/M2/M3/M4)
//! - CPU fallback (AVX2, AVX-512, NEON)
//! - NPU (Ryzen AI, Lunar Lake, Snapdragon)
//!
//! Zero vendor lock-in. Works on any machine.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UniversalError {
    #[error("Hardware detection failed: {0}")]
    DetectionFailed(String),

    #[error("Model loading failed: {0}")]
    ModelLoadFailed(String),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("Unsupported model format: {0}")]
    UnsupportedFormat(String),
}

pub type Result<T> = std::result::Result<T, UniversalError>;

/// Detected hardware backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareBackend {
    /// NVIDIA CUDA
    NvidiaCuda,
    /// AMD ROCm
    AmdRocm,
    /// Intel oneAPI / SYCL
    IntelOneApi,
    /// Apple Neural Engine (CoreML)
    AppleNeural,
    /// CPU with AVX-512
    CpuAvx512,
    /// CPU with AVX2
    CpuAvx2,
    /// CPU with ARM NEON
    CpuNeon,
    /// CPU generic fallback
    CpuGeneric,
    /// Neural Processing Unit
    Npu,
}

impl std::fmt::Display for HardwareBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NvidiaCuda => write!(f, "NVIDIA CUDA"),
            Self::AmdRocm => write!(f, "AMD ROCm"),
            Self::IntelOneApi => write!(f, "Intel oneAPI"),
            Self::AppleNeural => write!(f, "Apple Neural Engine"),
            Self::CpuAvx512 => write!(f, "CPU (AVX-512)"),
            Self::CpuAvx2 => write!(f, "CPU (AVX2)"),
            Self::CpuNeon => write!(f, "CPU (NEON)"),
            Self::CpuGeneric => write!(f, "CPU (Generic)"),
            Self::Npu => write!(f, "NPU"),
        }
    }
}

/// Hardware capabilities detected on this system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub backend: HardwareBackend,
    pub device_name: String,
    pub memory_mb: u64,
    pub compute_capability: Option<String>,
    pub cpu_cores: usize,
    pub cpu_model: String,
}

/// Detect the best available hardware backend
pub fn detect_hardware() -> HardwareInfo {
    let cpu_cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);

    // Try GPU backends
    if let Some(info) = detect_nvidia() {
        return info;
    }
    if let Some(info) = detect_amd() {
        return info;
    }
    if let Some(info) = detect_intel() {
        return info;
    }

    #[cfg(target_os = "macos")]
    if let Some(info) = detect_apple() {
        return info;
    }

    // CPU fallback with SIMD detection
    let (backend, cpu_model) = detect_cpu_capabilities();

    HardwareInfo {
        backend,
        device_name: cpu_model.clone(),
        memory_mb: get_system_memory_mb(),
        compute_capability: None,
        cpu_cores,
        cpu_model,
    }
}

fn detect_nvidia() -> Option<HardwareInfo> {
    let output = std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=name,memory.total")
        .arg("--format=csv,noheader,nounits")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?;
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

    let device_name = parts.first().unwrap_or(&"NVIDIA GPU").to_string();
    let memory_mb = parts
        .get(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    Some(HardwareInfo {
        backend: HardwareBackend::NvidiaCuda,
        device_name,
        memory_mb,
        compute_capability: None,
        cpu_cores: std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1),
        cpu_model: get_cpu_model(),
    })
}

fn detect_amd() -> Option<HardwareInfo> {
    let output = std::process::Command::new("rocm-smi")
        .arg("--showproductname")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let device_name = stdout
        .lines()
        .find(|l| l.contains("Card"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "AMD GPU".to_string());

    Some(HardwareInfo {
        backend: HardwareBackend::AmdRocm,
        device_name,
        memory_mb: 0,
        compute_capability: None,
        cpu_cores: std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1),
        cpu_model: get_cpu_model(),
    })
}

fn detect_intel() -> Option<HardwareInfo> {
    let output = std::process::Command::new("xpu-smi")
        .arg("discovery")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(HardwareInfo {
        backend: HardwareBackend::IntelOneApi,
        device_name: "Intel GPU".to_string(),
        memory_mb: 0,
        compute_capability: None,
        cpu_cores: std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1),
        cpu_model: get_cpu_model(),
    })
}

#[cfg(target_os = "macos")]
fn detect_apple() -> Option<HardwareInfo> {
    let output = std::process::Command::new("sysctl")
        .arg("-n")
        .arg("machdep.cpu.brand_string")
        .output()
        .ok()?;

    let cpu = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if cpu.contains("Apple") {
        return Some(HardwareInfo {
            backend: HardwareBackend::AppleNeural,
            device_name: cpu.clone(),
            memory_mb: get_system_memory_mb(),
            compute_capability: None,
            cpu_cores: std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1),
            cpu_model: cpu,
        });
    }
    None
}

fn detect_cpu_capabilities() -> (HardwareBackend, String) {
    let cpu_model = get_cpu_model();

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") {
            return (HardwareBackend::CpuAvx512, cpu_model);
        }
        if is_x86_feature_detected!("avx2") {
            return (HardwareBackend::CpuAvx2, cpu_model);
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        return (HardwareBackend::CpuNeon, cpu_model);
    }

    #[allow(unreachable_code)]
    (HardwareBackend::CpuGeneric, cpu_model)
}

fn get_cpu_model() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in cpuinfo.lines() {
                if line.starts_with("model name") {
                    if let Some(name) = line.split(':').nth(1) {
                        return name.trim().to_string();
                    }
                }
            }
        }
    }
    "Unknown CPU".to_string()
}

fn get_system_memory_mb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb / 1024;
                        }
                    }
                }
            }
        }
    }
    0
}

/// Supported model formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// ONNX Runtime compatible
    Onnx,
    /// GGUF (llama.cpp compatible)
    Gguf,
    /// SafeTensors
    SafeTensors,
}

impl ModelFormat {
    pub fn from_path(path: &Path) -> Result<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("onnx") => Ok(Self::Onnx),
            Some("gguf") => Ok(Self::Gguf),
            Some("safetensors") => Ok(Self::SafeTensors),
            other => Err(UniversalError::UnsupportedFormat(
                other.unwrap_or("none").to_string(),
            )),
        }
    }
}

/// Model configuration for inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_path: PathBuf,
    pub context_length: usize,
    pub temperature: f32,
    pub max_tokens: usize,
    pub top_p: f32,
    pub threads: Option<usize>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/default.gguf"),
            context_length: 4096,
            temperature: 0.7,
            max_tokens: 2048,
            top_p: 0.9,
            threads: None,
        }
    }
}

/// Universal inference engine
pub struct InferenceEngine {
    pub hardware: HardwareInfo,
    pub config: ModelConfig,
}

impl InferenceEngine {
    /// Create engine with auto-detected hardware
    pub fn new(config: ModelConfig) -> Result<Self> {
        let hardware = detect_hardware();
        tracing::info!(
            "Inference engine initialized: {} ({}, {} MB RAM, {} cores)",
            hardware.backend,
            hardware.device_name,
            hardware.memory_mb,
            hardware.cpu_cores
        );

        Ok(Self { hardware, config })
    }

    /// Get recommended thread count based on hardware
    pub fn recommended_threads(&self) -> usize {
        self.config
            .threads
            .unwrap_or_else(|| (self.hardware.cpu_cores).max(1))
    }

    /// Get recommended batch size based on available memory
    pub fn recommended_batch_size(&self) -> usize {
        match self.hardware.backend {
            HardwareBackend::NvidiaCuda | HardwareBackend::AmdRocm => {
                // GPU: scale with VRAM
                (self.hardware.memory_mb / 1024).max(1) as usize
            }
            HardwareBackend::AppleNeural => {
                // Unified memory: use 50%
                (self.hardware.memory_mb / 2048).max(1) as usize
            }
            _ => {
                // CPU: conservative
                (self.hardware.memory_mb / 4096).max(1) as usize
            }
        }
    }

    /// Check if model fits in available memory
    pub fn can_load_model(&self, model_size_mb: u64) -> bool {
        let available = match self.hardware.backend {
            HardwareBackend::NvidiaCuda | HardwareBackend::AmdRocm => self.hardware.memory_mb,
            _ => self.hardware.memory_mb,
        };
        model_size_mb < available * 90 / 100 // Keep 10% headroom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detection() {
        let info = detect_hardware();
        assert!(!info.device_name.is_empty());
        assert!(info.cpu_cores > 0);
        println!("Detected: {} - {}", info.backend, info.device_name);
    }

    #[test]
    fn test_model_format_detection() {
        assert_eq!(
            ModelFormat::from_path(Path::new("model.onnx")).unwrap(),
            ModelFormat::Onnx
        );
        assert_eq!(
            ModelFormat::from_path(Path::new("model.gguf")).unwrap(),
            ModelFormat::Gguf
        );
        assert!(ModelFormat::from_path(Path::new("model.xyz")).is_err());
    }

    #[test]
    fn test_inference_engine_creation() {
        let engine = InferenceEngine::new(ModelConfig::default()).unwrap();
        assert!(engine.recommended_threads() > 0);
        assert!(engine.recommended_batch_size() > 0);
    }

    #[test]
    fn test_model_memory_check() {
        let engine = InferenceEngine::new(ModelConfig::default()).unwrap();
        // A tiny model should always fit
        assert!(engine.can_load_model(1));
        // A petabyte model should never fit
        assert!(!engine.can_load_model(1_000_000_000));
    }

    #[test]
    fn test_hardware_display() {
        let info = detect_hardware();
        let display = format!("{}", info.backend);
        assert!(!display.is_empty());
    }
}
